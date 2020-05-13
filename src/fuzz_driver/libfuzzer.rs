use std::io::{BufRead, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::error::Error;

use log::{trace, error, info, debug, warn};
use regex::Regex;
use tokio::{fs, task, sync::oneshot};
use tonic::Request;

use super::FuzzConfig;
use crate::executor::{self, CrashConfig};
use crate::common::worker_tasks::{mark_worker_task_active, mark_worker_task_inactive};
use crate::models::NewFuzzStat;
use crate::common::xpc::get_orchestrator_client;

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
    async fn start(&self, kill_switch: oneshot::Receiver<u8>) -> Result<(), Box<dyn Error>> {
        info!("Starting libfuzzer driver for {:#?}", self.worker_task_id);

        let mut runner = executor::new(self.config.execution.clone(), self.worker_task_id);

        // let local = task::LocalSet::new();

        // Spawn off corpus sync
        let corpus_syncer = runner.get_corpus_syncer()?;
        corpus_syncer.setup_corpus().await?;
        let corpus_sync_handle = task::spawn(async move {
            if let Err(e) = corpus_syncer.sync_corpus().await {
                error!("Error in syncing corpus: {}", e);
            }
        });

        // Spawn off crash sync
        let crash_config = CrashConfig {
            label: self.config.execution.corpus.label.clone(),
            path: runner.get_cwd_path().into_boxed_path(),
            filter: Regex::new("crash-.*")?,
        };
        let crash_syncer = runner.get_crash_syncer(crash_config)?;
        let crash_sync_handle = task::spawn(async move {
            if let Err(e) = crash_syncer.upload_crashes().await {
                error!("Error in syncing crashes: {}", e);
            }
        });

        // Stat collector
        let log_path = runner.get_cwd_path();
        let stats_collector = LibFuzzerStatCollector::new(self.config.execution.cpus, self.worker_task_id, log_path)?;
        let stats_collector_handle = task::spawn(async move {
            if let Err(e) = stats_collector.start().await {
                error!("Error in syncing crashes: {}", e);
            }
        });

        // Start the actual process
        runner.spawn().await?;
        /*
        let runner_handle = tokio::spawn(async move {
            if let Ok(output) = runner.wait().await {
                info!("Command exited first with code: {:?}", output.status.code());
                info!("Stdout: {:?}", String::from_utf8(output.stdout));
                warn!("Stderr: {:?}", String::from_utf8(output.stderr));
            }
        });
        */

        mark_worker_task_active(self.worker_task_id).await?;
        // Listen and wait for all and kill switch
        tokio::select! {
            _ = corpus_sync_handle => {
                error!("Corpus sync exited first instead of kill switch");
            },
            _ = crash_sync_handle => {
                error!("Crash sync exited first instead of kill switch");
            },
            _ = stats_collector_handle => {
                error!("Stats collector exited first instead of kill switch");
            },
            _ = kill_switch => {
                info!("Received kill for lib fuzzer driver");
            },
            /*
            _ = runner_handle => {
                error!("Command exited first");
            },
            */
        }
        mark_worker_task_inactive(self.worker_task_id).await?;

        // local.await;
        // If we reached here means one of the watches failed or kill switch triggered
        info!("Kill fuzzer process for {:?}", self.worker_task_id);
        runner.close().await?;

        Ok(())
    }
}

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

    pub async fn start(self) -> Result<(), Box<dyn Error>> {
        debug!("Spawning lib fuzzer stat collector");

        let mut interval = tokio::time::interval(crate::common::intervals::WORKER_FUZZDRIVER_STAT_UPLOAD_INTERVAL);
        let client = &get_orchestrator_client().await?;

        loop {
            interval.tick().await;
            let mut client_clone = client.clone();

            // Iterate over logs and get stats
            let active_logs = self.get_active_logs().await?;
            let mut stats: Vec<NewFuzzStat> = Vec::new();
            for log in active_logs.into_iter() {
                let new_fuzz_stat = self.get_stat_from_log(log.as_path());
                if let Err(e) = new_fuzz_stat {
                    error!("Error during gathering stat from {:?}: {}", log, e);
                } else {
                    stats.push(new_fuzz_stat.unwrap());
                }
            }

            // Submit gathered stats
            for stat in stats.into_iter() {
                let request = Request::new(stat);
                if let Err(e) = client_clone.submit_fuzz_stat(request).await {
                    error!("Failed to submit a fuzz stat: {}", e);
                }
            }

        }
    }

    fn get_stat_from_log(&self, relative_path: &Path) -> Result<NewFuzzStat, Box<dyn Error>> {
        let file_path = self.path.join(relative_path); // Add path to directory path in config
        let mut file = std::fs::File::open(file_path.as_path())?;
        let length = file.metadata()?.len();

        // Always seek from start
        // debug!("File {:?} length found to be {}", file_path.as_path(), length);
        file.seek(SeekFrom::Start(length - 300))?;

        let reader = std::io::BufReader::new(file);
        let mut lines: Vec<String> = reader.lines().map(|line| { line.unwrap() }).collect();

        trace!("Collected lines from log file {:?}: {:?}", file_path.as_path(), lines);
        let line = lines.pop();
        if let Some(line) = line {
            let new_stat = self.get_stat(line.as_str())?;
            Ok(new_stat)
        } else {
            Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Unable to get stat from libfuzzer log: {:?}", line))))
        }
    }

    async fn get_active_logs(&self) -> Result<Vec<PathBuf>, Box<dyn Error>> {
        debug!("Trying to get active logs at {:?}", self.path);

        let mut logs = Vec::new();
        let mut entries = fs::read_dir(self.path.as_path()).await?;
        while let Some(entry) = entries.next_entry().await? {
            let name = entry.file_name().into_string().unwrap();
            if entry.file_type().await?.is_file() && self.log_filter.is_match(name.as_str()) {
                trace!("Matched {} for libfuzzer log file", name);
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
                files.push(e.path());
            }
            debug!("Found active logs: {:?}", files);
            Ok(files)
        } else {
            warn!("Unable to find necessary number of lib fuzzer log files");
            Ok(vec![])
        }
    }

    pub fn get_stat(&self, line: &str) -> Result<NewFuzzStat, Box<dyn Error>> {
        trace!("Trying to extract stat from libFuzzer line: {}", line);
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
                trace!("Found stat: {:?}", new_fuzz_stat);
                return Ok(new_fuzz_stat)
            }
        }
        Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData,
            format!("Unable to get stat from line: {}", line))))
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

        let stats_collector = LibFuzzerStatCollector::new(0, Some(0), PathBuf::new()).unwrap();
        for line in stats.lines().into_iter() {
            println!("{:?}", stats_collector.get_stat(line).unwrap());
        }
    }
}
