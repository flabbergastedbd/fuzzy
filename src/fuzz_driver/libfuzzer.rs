use std::path::{Path, PathBuf};
use std::error::Error;

use log::{trace, error, info, debug, warn};
use regex::Regex;
use tokio::fs;

use super::{FuzzStatCollector, FuzzConfig};
use crate::executor::Executor;
use crate::models::NewFuzzStat;
use crate::utils::fs::tail_n;

pub struct LibFuzzerDriver {
    config: FuzzConfig,
    worker_task_id: Option<i32>,
}

impl LibFuzzerDriver {
    pub fn new(config: FuzzConfig, worker_task_id: Option<i32>) -> Self {
        info!("Creating new libFuzzer driver with config {:#?}", config);
        Self { config, worker_task_id }
    }
}

#[tonic::async_trait]
impl super::FuzzDriver for LibFuzzerDriver {
    fn get_fuzz_config(&self) -> FuzzConfig {
        self.config.clone()
    }

    fn set_fuzz_config(&mut self, config: FuzzConfig) {
        self.config = config;
    }

    fn get_worker_task_id(&self) -> Option<i32> {
        self.worker_task_id.clone()
    }

    fn get_stat_collector(&self, executor: &Box<dyn Executor>) -> Result<Box<dyn FuzzStatCollector>, Box<dyn Error>> {
        let log_path = executor.get_cwd_path();
        let stats_collector = LibFuzzerStatCollector::new(self.config.execution.cpus, self.worker_task_id, log_path)?;
        Ok(Box::new(stats_collector))
    }

    fn fix_args(&mut self) {
        self.config.execution.args.insert(0, "-reload=1".to_owned());
        self.config.execution.args.insert(0, format!("-workers={}", self.config.execution.cpus));
    }
}

pub struct LibFuzzerStatCollector {
    instances: i32,
    worker_task_id: i32,
    path: PathBuf,
    log_filter: Regex,
    stat_filter: Regex,
}

#[tonic::async_trait]
impl super::FuzzStatCollector for LibFuzzerStatCollector {
    async fn get_stat(&self) -> Result<Option<NewFuzzStat>, Box<dyn Error>> {
        // Iterate over logs and get stats
        let mut total_coverage = 0;
        let mut total_execs = 0;
        let mut total_memory = 0;
        let mut total_stats = 0;

        let active_logs = self.get_active_logs().await?;
        for log in active_logs.into_iter() {
            let new_fuzz_stat = self.get_stat_from_log(log.as_path());
            if let Err(e) = new_fuzz_stat {
                error!("Error during gathering stat from {:?}: {}", log, e);
            } else {
                let new_fuzz_stat = new_fuzz_stat.unwrap();
                total_coverage += new_fuzz_stat.coverage;
                total_execs += new_fuzz_stat.execs.unwrap_or(0);
                total_memory += new_fuzz_stat.memory.unwrap_or(0);
                total_stats += 1;
            }
        }

        // Submit gathered stats
        let average_stat = NewFuzzStat {
            coverage: total_coverage/total_stats,
            execs: Some(total_execs/total_stats),
            memory: Some(total_memory/total_stats),
            worker_task_id: self.worker_task_id,
        };
        Ok(Some(average_stat))
    }
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

    fn get_stat_from_log(&self, relative_path: &Path) -> Result<NewFuzzStat, Box<dyn Error>> {
        let file_path = self.path.join(relative_path); // Add path to directory path in config
        let mut lines: Vec<String> = tail_n(file_path.as_path(), 300)?;

        trace!("Collected lines from log file {:?}: {:?}", file_path.as_path(), lines);
        let line = lines.pop();
        if let Some(line) = line {
            let new_stat = self.parse_stat(line.as_str())?;
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

    pub fn parse_stat(&self, line: &str) -> Result<NewFuzzStat, Box<dyn Error>> {
        trace!("Trying to extract stat from libFuzzer line: {}", line);
        if let Some(captures) = self.stat_filter.captures(line) {
            let coverage = captures.name("coverage");
            if coverage.is_some() {
                let coverage = coverage.unwrap().as_str().parse::<i32>()?;
                let execs = captures.name("execs").unwrap().as_str().parse::<i32>()?;
                let memory = captures.name("memory").unwrap().as_str().parse::<i32>()?;

                let new_fuzz_stat = NewFuzzStat {
                    coverage,
                    execs: Some(execs),
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
            println!("{:?}", stats_collector.parse_stat(line).unwrap());
        }
    }
}
