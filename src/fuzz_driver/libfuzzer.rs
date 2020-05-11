use std::io::{BufRead, Seek, SeekFrom};
use std::time::Duration;
use std::path::{Path, PathBuf};
use std::error::Error;

use log::{error, info, debug, warn};
use regex::Regex;
use tokio::{fs, task, sync::oneshot};
use tonic::Request;

use super::FuzzConfig;
use crate::executor::{self, CrashConfig, Executor};
use crate::xpc::orchestrator_client::OrchestratorClient;
use crate::models::NewFuzzStat;

pub struct LibFuzzerDriver {
    config: FuzzConfig,
    worker_task_id: Option<i32>,
}

#[tonic::async_trait]
impl super::FuzzDriver for LibFuzzerDriver {
    fn new(config: FuzzConfig, worker_task_id: Option<i32>) -> Self {
        info!("Creating new libFuzzer driver with config {:#?}", config);
        Self { config, worker_task_id }
    }

    /// LibFuzzer driver needs to do couple of things
    /// 1. Setup corpus
    /// 2. Start corpus sync
    /// 3. Collect metrics from log files
    async fn start(&self, connect_addr: String, kill_switch: oneshot::Receiver<u8>) -> Result<(), Box<dyn Error>> {
        info!("Starting libfuzzer driver for {:#?}", self.worker_task_id);

        let mut runner = executor::new(self.config.execution.clone(), self.worker_task_id);

        // let local = task::LocalSet::new();

        // Spawn off corpus sync
        let corpus_syncer = runner.get_corpus_syncer().await?;
        corpus_syncer.setup_corpus(connect_addr.clone()).await?;
        let connect_addr_clone = connect_addr.clone();
        let corpus_sync_handle = task::spawn(async move {
            if let Err(e) = corpus_syncer.sync_corpus(connect_addr_clone).await {
                error!("Error in syncing corpus: {}", e);
            }
        });

        // Spawn off crash sync
        let crash_config = CrashConfig {
            label: self.config.execution.corpus.label.clone(),
            path: self.config.execution.cwd.clone(),
            filter: Regex::new("crash-.*")?,
        };
        let crash_syncer = runner.get_crash_syncer(crash_config).await?;
        let connect_addr_clone = connect_addr.clone();
        let crash_sync_handle = task::spawn(async move {
            if let Err(e) = crash_syncer.upload_crashes(connect_addr_clone).await {
                error!("Error in syncing crashes: {}", e);
            }
        });

        // Stat collector
        let log_path = runner.get_abs_path(Path::new("."));
        let stats_collector = LibFuzzerStatCollector::new(self.config.execution.cpus, self.worker_task_id, log_path)?;
        let connect_addr_clone = connect_addr.clone();
        let stats_collector_handle = task::spawn(async move {
            if let Err(e) = stats_collector.start(connect_addr_clone).await {
                error!("Error in syncing crashes: {}", e);
            }
        });

        // Start the actual process
        runner.spawn().await?;

        // Listen and wait for all and kill switch
        tokio::select! {
            _ = corpus_sync_handle => {
                error!("Corpus sync exited first instead of kill switch");
            },
            _ = crash_sync_handle => {
                error!("Corpus sync exited first instead of kill switch");
            },
            _ = stats_collector_handle => {
                error!("Stats collector exited first instead of kill switch");
            },
            _ = kill_switch => {
                info!("Received kill for lib fuzzer driver");
            },
        }

        // local.await;
        // If we reached here means one of the watches failed or kill switch triggered
        info!("Kill fuzzer process for {:?}", self.worker_task_id);
        runner.close()?;

        Ok(())
    }
}

const STAT_UPLOAD_INTERVAL: Duration = Duration::from_secs(60);

pub struct LibFuzzerStatCollector {
    instances: i32,
    worker_task_id: i32,
    path: PathBuf,
    log_filter: Regex,
    stat_filter: Regex,
}

impl LibFuzzerStatCollector {
    pub fn new(instances: i32, worker_task_id: Option<i32>, path: PathBuf) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            instances,
            path,
            log_filter: Regex::new(r"fuzz-\d+.log")?,
            stat_filter: Regex::new(r"^.* cov: (?P<coverage>\d+) .* exec/s: (?P<execs>\d+) rss: (?P<memory>\d+)Mb .*$")?,
            worker_task_id: worker_task_id.unwrap_or(0),
        })
    }

    pub async fn start(self, connect_addr: String) -> Result<(), Box<dyn Error>> {
        debug!("Spawning lib fuzzer stat collector");

        let mut interval = tokio::time::interval(STAT_UPLOAD_INTERVAL);
        let client = &OrchestratorClient::connect(connect_addr).await?;

        loop {
            interval.tick().await;
            let mut client_clone = client.clone();
            // Iterate over logs
            let active_logs = self.get_active_logs().await?;
            for log in active_logs.into_iter() {
                let log = log.into_std().await;
                if let Ok(new_fuzz_stat) = self.get_stat_from_log(log) {
                    if new_fuzz_stat.is_some() {
                        let request = Request::new(new_fuzz_stat.unwrap());
                        let response = client_clone.submit_fuzz_stat(request).await;
                        /*
                        if let Err(e) = client_clone.submit_fuzz_stat(request).await {
                            error!("Failed to submit a fuzz stat: {}", e);
                        }
                        */
                    }
                }
            }
        }
    }

    fn get_stat_from_log(&self, file: std::fs::File) -> Result<Option<NewFuzzStat>, Box<dyn Error>> {
        file.seek(SeekFrom::End(400))?;

        let reader = std::io::BufReader::new(file);
        let mut lines: Vec<_> = reader.lines().map(|line| { line.unwrap() }).collect();

        if let Some(line) = lines.pop() {
            let new_stat = self.get_stat(line.as_str())?;
            Ok(new_stat)
        } else {
            Ok(None)
        }
    }

    async fn get_active_logs(&self) -> Result<Vec<fs::File>, Box<dyn Error>> {
        debug!("Trying to get active logs at {:?}", self.path);

        let mut logs = Vec::new();
        let mut entries = fs::read_dir(self.path.as_path()).await?;
        while let Some(entry) = entries.next_entry().await? {
            let name = entry.file_name().into_string().unwrap();
            if entry.file_type().await?.is_file() && self.log_filter.is_match(name.as_str()) {
                debug!("Matched {} for libfuzzer log file", name);
                logs.push(entry);
            }
        }
        // Sort by filenames, because always fuzz-0, fuzz-1 etc..
        logs.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

        // It has to be, otherwise something is wrong
        if logs.len() as i32 >= self.instances {
            let (logs, _) = logs.split_at(self.instances as usize);
            let mut files = Vec::new();
            for e in logs {
                let file_path = self.path.join(e.file_name());
                files.push(fs::File::open(file_path).await?);
            }
            debug!("Found active logs: {:?}", files);
            Ok(files)
        } else {
            Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData,
                "Unable to find necessary number of lib fuzzer log files")))
        }
    }

    pub fn get_stat(&self, line: &str) -> Result<Option<NewFuzzStat>, Box<dyn Error>> {
        debug!("Trying to extract stat from libFuzzer line: {}", line);
        if let Some(captures) = self.stat_filter.captures(line) {
            let coverage = captures.name("coverage");
            if coverage.is_some() {
                let coverage = coverage.unwrap().as_str().parse::<i32>()?;
                let execs = captures.name("execs").unwrap().as_str().parse::<i32>()?;
                let memory = captures.name("memory").unwrap().as_str().parse::<i32>()?;

                let new_fuzz_stat = NewFuzzStat {
                    coverage,
                    execs,
                    memory: Some(memory),
                    worker_task_id: self.worker_task_id,
                };
                debug!("Found stat: {:?}", new_fuzz_stat);
                return Ok(Some(new_fuzz_stat))
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_stat_regex() {
        let stats = r"#3811 NEW    cov: 4 ft: 3 corp: 2/2b exec/s: 0 rss: 25Mb L: 1 MS: 5 ChangeBit-ChangeByte-ChangeBit-ShuffleBytes-ChangeByte-
#3827 NEW    cov: 5 ft: 4 corp: 3/4b exec/s: 0 rss: 25Mb L: 2 MS: 1 CopyPart-
#3963 NEW    cov: 6 ft: 5 corp: 4/6b exec/s: 0 rss: 25Mb L: 2 MS: 2 ShuffleBytes-ChangeBit-
#4167 NEW    cov: 7 ft: 6 corp: 5/9b exec/s: 0 rss: 25Mb L: 3 MS: 1 InsertByte-";

        let stats_collector = LibFuzzerStatCollector::new(0, PathBuf::new()).unwrap();
        for line in stats.lines().into_iter() {
            println!("{:?}", stats_collector.get_stat(line).unwrap());
        }
    }
}