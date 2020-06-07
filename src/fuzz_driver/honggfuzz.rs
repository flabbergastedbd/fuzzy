use std::error::Error;
use std::path::PathBuf;

use log::{error, info, trace};
use regex::Regex;

use super::{FuzzConfig, FuzzStatCollector};
use crate::executor::Executor;
use crate::models::NewFuzzStat;
use crate::utils::fs::tail_n;

const HONGGFUZZ_LOG: &str = "honggfuzz.log";

pub struct HonggfuzzDriver {
    config: FuzzConfig,
    worker_task_id: Option<i32>,
}

impl HonggfuzzDriver {
    pub fn new(config: FuzzConfig, worker_task_id: Option<i32>) -> Self {
        info!("Creating new libFuzzer driver with config {:#?}", config);
        Self { config, worker_task_id }
    }
}

#[tonic::async_trait]
impl super::FuzzDriver for HonggfuzzDriver {
    fn get_fuzz_config(&self) -> FuzzConfig {
        self.config.clone()
    }

    fn set_fuzz_config(&mut self, config: FuzzConfig) {
        self.config = config;
    }

    fn get_worker_task_id(&self) -> Option<i32> {
        self.worker_task_id.clone()
    }

    fn get_custom_stat_collector(
        &self,
        executor: &Box<dyn Executor>,
    ) -> Result<Option<Box<dyn FuzzStatCollector>>, Box<dyn Error>> {
        let log_path = executor.get_cwd_path().join(HONGGFUZZ_LOG);
        let stats_collector = HonggfuzzStatCollector::new(self.worker_task_id, log_path)?;
        Ok(Some(Box::new(stats_collector)))
    }

    fn fix_args(&mut self) {
        self.config.execution.args.insert(0, "--threads".to_owned());
        self.config
            .execution
            .args
            .insert(1, format!("{}", self.config.execution.cpus));

        self.config.execution.args.insert(0, "--logfile".to_owned());
        self.config.execution.args.insert(1, HONGGFUZZ_LOG.to_owned());
    }
}

pub struct HonggfuzzStatCollector {
    worker_task_id: i32,
    path: PathBuf,
    stat_filter: Regex,
}

#[tonic::async_trait]
impl super::FuzzStatCollector for HonggfuzzStatCollector {
    async fn get_stat(&self) -> Result<Option<NewFuzzStat>, Box<dyn Error>> {
        let mut lines = tail_n(self.path.as_path(), 100)?;
        let line = lines.pop();
        if let Some(line) = line {
            let new_stat = self.parse_stat(line.as_str())?;
            Ok(Some(new_stat))
        } else {
            error!("Unable to get stat from honggfuzz log line: {:?}", line);
            Ok(None)
        }
    }
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
                        branch_coverage: Some(coverage),
                        line_coverage: None,
                        function_coverage: None,
                        worker_task_id: self.worker_task_id,
                        execs: None,
                        memory: None,
                    };
                    trace!("Found stat: {:?}", new_fuzz_stat);
                    return Ok(new_fuzz_stat);
                }
            }
        }
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Unable to get stat from line: {}", line),
        )))
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
