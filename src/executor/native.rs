use std::process::Stdio;
use std::error::Error;
use std::path::{Path, PathBuf};

use log::{info, debug};
use tokio::{
    fs,
    process::{Command, Child, ChildStdout, ChildStderr},
    io::{BufReader, AsyncBufReadExt, Lines},
};

use super::{CrashConfig, ExecutorConfig};
use super::corpus_syncer::CorpusSyncer;
use super::crash_syncer::CrashSyncer;
use crate::utils::fs::mkdir_p;

pub struct NativeExecutor {
    config: ExecutorConfig,
    child: Option<Child>,
    worker_task_id: Option<i32>,
}

#[tonic::async_trait]
impl super::Executor for NativeExecutor {
    async fn setup(&self) -> Result<(), Box<dyn Error>> {
        debug!("Setting up execution environment");

        // Check if cwd exists, if not create
        mkdir_p(&self.config.cwd).await?;

        // Check if corpus dir exists, if not create it
        let absolute_corpus_path = self.config.cwd.join(&self.config.corpus.path);
        mkdir_p(absolute_corpus_path.as_path()).await?;

        Ok(())
    }

    async fn spawn(&mut self) -> Result<(), Box<dyn Error>> {
        debug!("Launching child process");
        let mut cmd = Command::new(self.config.executable.clone());
        cmd
            .args(self.config.args.clone())
            .envs(self.config.envs.clone())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(self.config.cwd.clone());
            // .kill_on_drop(true);
        debug!("Command: {:#?}", cmd);

        let child = cmd.spawn()?;
        self.child = Some(child);

        Ok(())
    }

    fn get_stdout_reader(&mut self) -> Option<Lines<BufReader<ChildStdout>>> {
        let out = self.child.as_mut().map(|c| { c.stdout.take() })??;
        let reader = BufReader::new(out).lines();
        Some(reader)
    }

    fn get_stderr_reader(&mut self) -> Option<Lines<BufReader<ChildStderr>>> {
        let out = self.child.as_mut().map(|c| { c.stderr.take() })??;
        let reader = BufReader::new(out).lines();
        Some(reader)
    }

    fn get_corpus_syncer(&self) -> Result<CorpusSyncer, Box<dyn Error>> {
        let mut corpus_config = self.config.corpus.clone();
        corpus_config.path = self.config.cwd.join(corpus_config.path).into_boxed_path();
        Ok(CorpusSyncer::new(
                corpus_config,
                self.worker_task_id
        )?)
    }

    fn get_crash_syncer(&self, config: CrashConfig) -> Result<CrashSyncer, Box<dyn Error>> {
        Ok(CrashSyncer::new(
                config,
                self.worker_task_id
        )?)
    }

    fn get_cwd_path(&self) -> PathBuf {
        self.config.cwd.clone().to_path_buf()
    }

    fn close(&mut self) -> Result<(), Box<dyn Error>> {
        debug!("Closing out executor");
        Ok(self.child.as_mut().map(|c| c.kill()).unwrap()?)
    }
}

impl NativeExecutor {
    pub fn new(config: ExecutorConfig, worker_task_id: Option<i32>) -> Self {
        debug!("Creating new native executor with config: {:#?}", config);
        Self {
            config, child: None, worker_task_id
        }
    }
}
