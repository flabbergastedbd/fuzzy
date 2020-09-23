use std::error::Error;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::process::Stdio;

use tracing::{debug, error};
use tokio::{
    io::{AsyncBufReadExt, BufReader, Lines},
    process::{Child, ChildStderr, ChildStdout, Command},
    sync::broadcast,
};

use super::corpus_syncer::CorpusSyncer;
use super::crash_syncer::CrashSyncer;
use super::ExecutorConfig;
use crate::fuzz_driver::{CorpusConfig, CrashConfig};
use crate::utils::fs::{mkdir_p, rm_r};

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
        Ok(())
    }

    async fn create_relative_dirp(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        let mapped_path = self.config.cwd.join(path);
        mkdir_p(mapped_path.as_path()).await?;
        Ok(())
    }

    async fn rm_relative_dirp(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        let mapped_path = self.config.cwd.join(path);
        rm_r(mapped_path.as_path()).await?;
        Ok(())
    }

    async fn wait(&self, mut kill_switch: broadcast::Receiver<u8>) -> Result<(), Box<dyn Error>> {
        if self.child.is_some() {
            let pid = self.child.as_ref().map(|c| c.id());
            if let Some(pid) = pid {
                tokio::select! {
                    _ = pid_exists(pid) => {
                        error!("Process with pid {} exited first", pid);
                    },
                    _ = kill_switch.recv() => {
                        debug!("Kill received for native executor, hope the command dies");
                    },
                };
            }
            Ok(())
        } else {
            Err(Box::new(io::Error::new(
                ErrorKind::InvalidData,
                "wait() called on executor",
            )))
        }
    }

    async fn spawn(&mut self) -> Result<(), Box<dyn Error>> {
        debug!("Launching child process");
        let mut cmd = self.create_cmd();
        let child = cmd.spawn()?;
        self.child = Some(child);
        Ok(())
    }

    async fn spawn_blocking(&mut self) -> Result<std::process::Output, Box<dyn Error>> {
        let mut cmd = self.create_cmd();
        Ok(cmd.spawn()?.wait_with_output().await?)
    }

    fn get_stdout_reader(&mut self) -> Option<Lines<BufReader<ChildStdout>>> {
        let out = self.child.as_mut().map(|c| c.stdout.take())??;
        let reader = BufReader::new(out).lines();
        Some(reader)
    }

    fn get_stderr_reader(&mut self) -> Option<Lines<BufReader<ChildStderr>>> {
        let out = self.child.as_mut().map(|c| c.stderr.take())??;
        let reader = BufReader::new(out).lines();
        Some(reader)
    }

    fn get_corpus_syncer(&self, mut config: CorpusConfig) -> Result<CorpusSyncer, Box<dyn Error>> {
        config.path = self.config.cwd.join(config.path).into_boxed_path();
        Ok(CorpusSyncer::new(config, self.worker_task_id)?)
    }

    fn get_crash_syncer(&self, mut config: CrashConfig) -> Result<CrashSyncer, Box<dyn Error>> {
        config.path = self.config.cwd.join(config.path).into_boxed_path();
        Ok(CrashSyncer::new(config, self.worker_task_id)?)
    }

    fn get_cwd_path(&self) -> PathBuf {
        self.config.cwd.clone().to_path_buf()
    }

    async fn close(mut self: Box<Self>) -> Result<(), Box<dyn Error>> {
        if let Some(mut c) = self.child {
            c.kill()?;
            let output = c.wait_with_output().await?;
            debug!("Driver exited with status: {}", output.status);
        }
        Ok(())
    }
}

impl NativeExecutor {
    pub fn new(config: ExecutorConfig, worker_task_id: Option<i32>) -> Self {
        debug!("Creating new native executor with config: {:#?}", config);
        Self {
            config,
            child: None,
            worker_task_id,
        }
    }

    fn create_cmd(&self) -> Command {
        let mut cmd = Command::new(self.config.executable.clone());
        cmd.args(self.config.args.clone())
            .envs(self.config.envs.clone())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(self.config.cwd.clone())
            .kill_on_drop(true);
        debug!("Command: {:#?}", cmd);
        cmd
    }
}

// This check should be very liberal
async fn pid_exists(pid: u32) -> Result<(), Box<dyn Error>> {
    let mut interval = tokio::time::interval(crate::common::intervals::WORKER_PROCESS_CHECK_INTERVAL);
    let mut fail_count = 0;
    loop {
        interval.tick().await;
        if heim::process::pid_exists(pid as i32).await? == false {
            if fail_count > 4 {
                break;
            } else {
                fail_count = fail_count + 1;
            }
        }
    }
    Ok(())
}
