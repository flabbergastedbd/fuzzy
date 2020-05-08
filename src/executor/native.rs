use std::process::Stdio;
use std::error::Error;
use std::path::Path;

use inotify::{Inotify, WatchMask};
use log::{trace, debug};
use super::ExecutorConfig;
use tokio::{
    fs,
    process::{self, Command, Child, ChildStdout, ChildStderr},
    io::{BufReader, AsyncBufReadExt, Lines},
};

use super::file_watcher::InotifyFileWatcher;
use super::corpus_syncer::CorpusSyncer;

pub struct NativeExecutor {
    config: ExecutorConfig,
    child: Option<Child>,
}

#[tonic::async_trait]
impl super::Executor for NativeExecutor {
    fn new(config: ExecutorConfig) -> Self {
        debug!("Creating new native executor with config: {:#?}", config);
        Self {
            config, child: None,
        }
    }

    async fn setup(&self) -> Result<(), Box<dyn Error>> {
        debug!("Setting up execution environment");

        // Check if cwd exists, if not create
        Self::mkdir_p(&self.config.cwd).await?;
        Self::mkdir_p(&self.config.corpus).await?;

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

    fn get_file_watcher(&self, path: &Path) -> Result<InotifyFileWatcher, Box<dyn Error>> {
        Ok(InotifyFileWatcher::new(path)?)
    }

    fn get_corpus_syncer(&self) -> Result<CorpusSyncer, Box<dyn Error>> {
        Ok(CorpusSyncer::new(&self.config.corpus)?)
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
