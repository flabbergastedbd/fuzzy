use std::path::PathBuf;
use std::error::Error;

use log::{trace, error, info, debug, warn};
use regex::Regex;
use tokio::{sync::oneshot, sync::broadcast};
use tonic::Request;

use super::FuzzConfig;
use crate::executor;
use crate::common::worker_tasks::{mark_worker_task_active, mark_worker_task_inactive};
use crate::models::NewFuzzStat;
use crate::common::xpc::get_orchestrator_client;
use crate::utils::fs::tail_n;

const HONGGFUZZ_LOG: &str = "honggfuzz.log";

pub struct HonggfuzzDriver {
    config: FuzzConfig,
    worker_task_id: Option<i32>,
}

#[tonic::async_trait]
impl super::FuzzDriver for HonggfuzzDriver {
    fn new(config: FuzzConfig, worker_task_id: Option<i32>) -> Self {
        info!("Creating new libFuzzer driver with config {:#?}", config);
        Self { config, worker_task_id }
    }

    async fn start(&mut self, kill_switch: oneshot::Receiver<u8>, death_switch: oneshot::Sender<u8>) -> Result<(), Box<dyn Error>> {
        self.fix_args();
        info!("Starting libfuzzer driver for {:#?}", self.worker_task_id);

        let mut runner = executor::new(self.config.execution.clone(), self.worker_task_id);
        runner.setup().await?;

        // let local = task::LocalSet::new();

        // Spawn off corpus sync
        let mut corpus_syncer = runner.get_corpus_syncer()?;
        corpus_syncer.setup_corpus().await?;

        // Spawn off crash sync
        let crash_syncer = runner.get_crash_syncer()?;

        // Stat collector
        let log_path = runner.get_cwd_path().join(HONGGFUZZ_LOG);
        let stats_collector = HonggfuzzStatCollector::new(self.worker_task_id, log_path)?;

        // Start the actual process
        runner.spawn().await?;

        mark_worker_task_active(self.worker_task_id).await?;
        // Listen and wait for all and kill switch
        let (longshot, longshot_recv) = broadcast::channel(5);
        let crash_longshot_recv = longshot.subscribe();
        let stat_longshot_recv = longshot.subscribe();
        let runner_longshot_recv = longshot.subscribe();
        tokio::select! {
            result = corpus_syncer.sync_corpus(longshot_recv) => {
                error!("Error in syncing corpus: {:?}", result);
            },
            result = crash_syncer.upload_crashes(crash_longshot_recv) => {
                error!("Error in syncing crashes: {:?}", result);
            },
            result = stats_collector.start(stat_longshot_recv) => {
                error!("Error in collecting stats : {:?}", result);
            },
            _ = kill_switch => {
                warn!("Received kill for lib fuzzer driver");
            },
            result = runner.wait(runner_longshot_recv) => {
                error!("Error in executor: {:?}", result);
            },
        }
        let close_time = std::time::SystemTime::now();
        // If we are here it means select wrapped up from above
        // Close the fuzz process
        if let Err(e) = longshot.send(0) {
            error!("Error in sending longshot: {:?}", e);
        }
        if let Err(e) = death_switch.send(0) {
            error!("Error in sending death switch: {:?}", e);
        }
        info!("Sending kill signal for execturo {:?} as select! ended", self.worker_task_id);
        runner.close().await?;
        corpus_syncer.close(close_time).await?;

        mark_worker_task_inactive(self.worker_task_id).await?;

        // local.await;
        // If we reached here means one of the watches failed or kill switch triggered
        info!("Kill fuzzer process for {:?}", self.worker_task_id);
        // runner.close().await?;

        Ok(())
    }
}

impl HonggfuzzDriver {
    fn fix_args(&mut self) {
        self.config.execution.args.insert(0, "--threads".to_owned());
        self.config.execution.args.insert(1, format!("{}", self.config.execution.cpus));

        self.config.execution.args.insert(0, "--logfile".to_owned());
        self.config.execution.args.insert(1, HONGGFUZZ_LOG.to_owned());
    }
}

pub struct HonggfuzzStatCollector {
    worker_task_id: i32,
    path: PathBuf,
    stat_filter: Regex,
}

impl HonggfuzzStatCollector {
    pub fn new(worker_task_id: Option<i32>, path: PathBuf) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            path,
            worker_task_id: worker_task_id.unwrap_or(0),
            stat_filter: Regex::new(r"^.* Tot:(?P<total_stats>[0-9/]+)")?,
        })
    }

    fn parse_stat(&self, line: &str) -> Result<NewFuzzStat, Box<dyn Error>> {
        if let Some(captures) = self.stat_filter.captures(line) {
            let total_stats = captures.name("total_stats");
            if let Some(total_stats) = total_stats {
                let stats: Vec<&str> = total_stats.as_str().split("/").collect();
                if stats.len() == 6 && stats.get(3).is_some() {
                    let coverage = stats.get(3).unwrap().parse::<i32>()?;
                    let new_fuzz_stat = NewFuzzStat {
                        coverage,
                        worker_task_id: self.worker_task_id,
                        execs: None,
                        memory: None,
                    };
                    trace!("Found stat: {:?}", new_fuzz_stat);
                    return Ok(new_fuzz_stat)
                }
            }
        }
        Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData,
            format!("Unable to get stat from line: {}", line))))
    }

    async fn get_stat(&self) -> Result<NewFuzzStat, Box<dyn Error>> {
        let mut lines = tail_n(self.path.as_path(), 100)?;
        let line = lines.pop();
        if let Some(line) = line {
            let new_stat = self.parse_stat(line.as_str())?;
            Ok(new_stat)
        } else {
            Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Unable to get stat from honggfuzz log line: {:?}", line))))
        }
    }

    async fn main_loop(self) -> Result<(), Box<dyn Error>> {
        let mut interval = tokio::time::interval(crate::common::intervals::WORKER_FUZZDRIVER_STAT_UPLOAD_INTERVAL);
        let client = &get_orchestrator_client().await?;
        loop {
            interval.tick().await;
            let mut client = client.clone();
            // Iterate over logs and get stats
            let stat: Option<NewFuzzStat> = match self.get_stat().await {
                Ok(stat) => Some(stat),
                Err(e) => {
                    warn!("Unable to get last lines from log file {:?}: {}", self.path.as_path(), e);
                    None
                },
            };

            if let Some(stat) = stat {
                if let Err(e) = client.submit_fuzz_stat(Request::new(stat)).await {
                    error!("Failed to submit a fuzz stat: {}", e);
                }
            }

        }
    }

    pub async fn start(self, mut kill_switch: broadcast::Receiver<u8>) -> Result<(), Box<dyn Error>> {
        debug!("Started honggfuzz stat collection for {}", self.worker_task_id);

        tokio::select! {
            result = self.main_loop() => {
                if let Err(e) = result {
                    error!("Honggfuzz stat collection exited with error: {}", e);
                }
            },
            _ = kill_switch.recv() => {},
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_stat_regex() {
        let stats = r"Size:27 (i,b,hw,ed,ip,cmp): 0/0/0/1/0/0, Tot:0/0/0/144/11/2408
Size:19 (i,b,hw,ed,ip,cmp): 0/0/0/0/0/1, Tot:0/0/0/144/11/2409
Size:63 (i,b,hw,ed,ip,cmp): 0/0/0/0/0/1, Tot:0/0/0/144/11/2410
Size:4328 (i,b,hw,ed,ip,cmp): 0/0/0/2/0/0, Tot:0/0/0/146/11/2410
Size:27 (i,b,hw,ed,ip,cmp): 0/0/0/1/0/0, Tot:0/0/0/147/11/2410
Size:63 (i,b,hw,ed,ip,cmp): 0/0/0/2/0/0, Tot:0/0/0/149/11/2410
Size:9 (i,b,hw,ed,ip,cmp): 0/0/0/0/0/2, Tot:0/0/0/149/11/2412
Size:3 (i,b,hw,ed,ip,cmp): 0/0/0/0/0/1, Tot:0/0/0/149/11/2413";

        let stats_collector = HonggfuzzStatCollector::new(Some(0), PathBuf::new()).unwrap();
        for line in stats.lines().into_iter() {
            println!("{:?}", stats_collector.parse_stat(line).unwrap());
        }
    }
}
