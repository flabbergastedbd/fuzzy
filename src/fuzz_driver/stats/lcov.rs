use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use lcov_parser::{FromFile, LCOVRecord};
use tracing::{debug, error};
use tokio::fs::read_dir;
use tokio::stream::StreamExt;

use super::{FuzzStatCollector, FuzzStatConfig};
use crate::common::corpora::download_corpus_to_disk;
use crate::common::xpc::get_orchestrator_client;
use crate::executor;
use crate::fuzz_driver::FuzzConfig;
use crate::models::NewFuzzStat;
use crate::utils::err_output;
use crate::utils::fs::rm_r;

#[derive(Clone)]
pub struct LCovCollector {
    config: FuzzStatConfig,
    worker_task_id: Option<i32>,
    corpus_label: String,
    last_sync: SystemTime,
    refresh_interval: Duration,
}

impl LCovCollector {
    pub fn new(config: FuzzStatConfig, full_config: FuzzConfig, worker_task_id: Option<i32>) -> Self {
        Self {
            config,
            worker_task_id,
            corpus_label: full_config.corpus.label,
            last_sync: UNIX_EPOCH,
            refresh_interval: Duration::from_secs(full_config.corpus.refresh_interval),
        }
    }
}

#[tonic::async_trait]
impl FuzzStatCollector for LCovCollector {
    fn get_refresh_duration(&self) -> std::time::Duration {
        self.refresh_interval
    }

    async fn get_stat(&self) -> Result<Option<NewFuzzStat>, Box<dyn Error>> {
        debug!("Getting new stat using lcov collector");
        let mut client = get_orchestrator_client().await?;

        // Create an executor
        let mut executor = executor::new(self.config.execution.clone(), self.worker_task_id);

        // Get latest corpus to cwd
        executor.setup().await?;

        // Download latest corpus found by this worker
        let cwd = executor.get_cwd_path();
        let num_files = download_corpus_to_disk(
            self.corpus_label.clone(),
            None,
            self.worker_task_id,
            Some(10), // Get 10 latest samples
            self.last_sync,
            cwd.as_path(),
            &mut client,
        )
        .await?;

        let mut new_fuzz_stat: Option<NewFuzzStat> = None;
        if num_files > 0 {
            debug!("{} corpus downloaded for stat collection", num_files);

            let output = executor.spawn_blocking().await?;
            if output.status.success() == false {
                error!("Stat collection execution failed");
                err_output(output);
            }
            // TODO: Not necessary but let us keep it for now so that if we add volume cleanup it
            // should be done.
            // executor.close().await?;

            // We look for a .lcov file anyway
            let entries = read_dir(cwd.as_path()).await?;
            let lcov_files = entries.filter_map(|f| {
                if let Ok(file) = f {
                    let path = file.path();
                    let extension = path.extension();
                    if extension.is_some() && extension.unwrap() == "lcov" {
                        return Some(path);
                    }
                }
                None
            });
            let mut lcov_paths: Vec<PathBuf> = lcov_files.collect::<Vec<PathBuf>>().await;

            // TODO: Ugliest piece of code I ever wrote, fix this
            if let Some(lcov_path) = lcov_paths.pop() {
                let result = self.parse_lcov(&lcov_path);
                if let Ok(stat) = result {
                    new_fuzz_stat = Some(stat);
                } else {
                    error!("Failed to parse merged lcov: {:?}", result);
                }
            } else {
                error!("No .lcov file found, so exiting");
            }
        } else {
            debug!("No corpus could be downloaded, doing nothing");
        }

        rm_r(&cwd).await?;
        Ok(new_fuzz_stat)
    }
}

impl LCovCollector {
    fn parse_lcov(&self, path: &Path) -> Result<NewFuzzStat, Box<dyn Error>> {
        // https://docs.rs/lcov-parser/3.2.2/src/lcov_parser/record.rs.html#18
        // As of now we only deal with aggregates LinesHit, LinesFound, BranchesHit, BranchesFound
        debug!("Parsing lcov info file at {:?}", path);
        let mut parser = lcov_parser::LCOVParser::from_file(path)?;
        let records = parser.parse()?;

        let mut branches_hit = 0;
        let mut lines_hit = 0;
        let mut functions_hit = 0;
        /*
        let mut lines_found = 0;
        let mut branches_found = 0;
        let mut functions_found = 0;
        */

        for record in records.iter() {
            match record {
                LCOVRecord::LinesHit(n) => lines_hit += n,
                LCOVRecord::BranchesHit(n) => branches_hit += n,
                LCOVRecord::FunctionsHit(n) => functions_hit += n,
                /*
                LCOVRecord::LinesFound(n) => { lines_found += n },
                LCOVRecord::BranchesFound(n) => { branches_found += n },
                LCOVRecord::FunctionsFound(n) => { functions_found += n },
                */
                _ => continue,
            }
        }

        if branches_hit == 0 && lines_hit == 0 && functions_hit == 0 {
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unable to get parse lcov info from {:?}", path),
            )))
        } else {
            // We send percentages instead of raw count
            Ok(NewFuzzStat {
                /*
                branch_coverage: if branches_found >    0 { Some((branches_hit/branches_found) as i32) } else { None },
                line_coverage: if lines_found >         0 { Some((lines_hit/lines_found) as i32) } else { None },
                function_coverage: if functions_found > 0 { Some((functions_hit/functions_found) as i32) } else { None },
                */
                branch_coverage: Some(branches_hit as i32),
                line_coverage: Some(lines_hit as i32),
                function_coverage: Some(functions_hit as i32),
                execs: None,
                memory: None,
                worker_task_id: self.worker_task_id.unwrap_or(0),
            })
        }
    }
}
