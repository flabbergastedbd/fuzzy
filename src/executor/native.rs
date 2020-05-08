use std::process::Stdio;
use std::error::Error;
use std::path::Path;

use log::{trace, debug};
use super::ExecutorConfig;
use tokio::{
    fs,
    process::{Command, Child, ChildStdout, ChildStderr},
    io::{BufReader, AsyncBufReadExt, Lines},
};

use super::corpus_syncer::CorpusSyncer;

pub struct NativeExecutor {
    config: ExecutorConfig,
    child: Option<Child>,
    worker_task_id: Option<i32>
}

#[tonic::async_trait]
impl super::Executor for NativeExecutor {
    fn new(config: ExecutorConfig, worker_task_id: Option<i32>) -> Self {
        debug!("Creating new native executor with config: {:#?}", config);
        Self {
            config, child: None, worker_task_id
        }
    }

    async fn setup(&self) -> Result<(), Box<dyn Error>> {
        debug!("Setting up execution environment");

        // Check if cwd exists, if not create
        Self::mkdir_p(&self.config.cwd).await?;
        Self::mkdir_p(&self.config.corpus.path).await?;

        Ok(())
    }

    async fn launch(&mut self) -> Result<(), Box<dyn Error>> {
        debug!("Launching child process");
        let mut cmd = Command::new(self.config.executable.clone());
        cmd
            .args(self.config.args.clone())
            .envs(self.config.envs.clone())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(self.config.cwd.clone());
            // .kill_on_drop(true);

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
        let corpus_config = self.config.corpus.clone();
        Ok(CorpusSyncer::new(
                corpus_config,
                self.worker_task_id
        )?)
    }

    fn get_pid(&self) -> u32 {
        self.child.as_ref().map(|c| c.id()).unwrap()
    }
}

impl NativeExecutor {
    async fn mkdir_p(path: &Path) -> std::io::Result<()> {
        debug!("Creating directory tree");
        fs::create_dir_all(path).await?;
        Ok(())
    }
}
