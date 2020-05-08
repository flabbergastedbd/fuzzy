use std::process::Stdio;
use std::error::Error;
use std::path::Path;

use tokio::stream::StreamExt;
use inotify::{Inotify, WatchMask};
use log::{trace, debug};
use super::ExecutorConfig;
use tokio::{
    fs,
    process::{self, Command, Child, ChildStdout, ChildStderr},
    io::{BufReader, AsyncBufReadExt, Lines},
};

pub struct NativeExecutor {
    config: ExecutorConfig,
    child: Option<Child>,
    // Related use index as tying point
    watchers: Vec<Inotify>,
    streams: Vec<inotify::EventStream<[u8; 32]>>,
}

#[tonic::async_trait]
impl super::Executor for NativeExecutor {
    fn new(config: ExecutorConfig) -> Self {
        debug!("Creating new native executor with config: {:#?}", config);
        Self {
            config,
            child: None,
            // We use one inotify per watch,
            // TODO: Need to improve this
            watchers: Vec::new(),
            streams: Vec::new(),
        }
    }

    async fn setup(&self) -> Result<(), Box<dyn Error>> {
        debug!("Setting up execution environment");

        // Check if cwd exists, if not create
        Self::mkdir_p(&self.config.cwd).await?;

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

    fn add_watch(&mut self, path: &Path) -> Result<usize, Box<dyn Error>> {
        debug!("Adding watch at {:?}", path);
        let index = self.watchers.len();

        let mut i = Inotify::init()?;
        i.add_watch(path, WatchMask::CREATE)?;
        let buffer = [0; 32];
        let stream = i.event_stream(buffer)?;

        self.watchers.push(i);
        self.streams.push(stream);

        Ok(index)
    }

    // async fn read_event(&mut self, watch_index: usize) -> Result<Option<String>, Box<dyn Error>> {
    async fn get_watched_files(&mut self, watch_index: usize) -> Option<String> {
        let event_or_error = self.streams[watch_index].next().await?;
        trace!("Received inotify event: {:?}", event_or_error);
        if let Ok(event) = event_or_error {
            Some(event.name?.into_string().unwrap())
        } else {
            None
        }
    }

    fn rm_watch(&mut self, watch_index: usize) -> Result<bool, Box<dyn Error>> {
        debug!("Removing watch at index: {}", watch_index);

        self.watchers.remove(watch_index).close()?;
        self.streams.remove(watch_index);
        Ok(true)
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
