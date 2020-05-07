use std::process::Stdio;
use std::error::Error;
use std::path::Path;

use log::debug;
use super::ExecutorConfig;
use tokio::{
    fs,
    process::{self, Command, Child, ChildStdout, ChildStderr},
    io::{BufReader, AsyncBufReadExt, Lines},
};

#[derive(Debug)]
pub struct NativeExecutor {
    config: ExecutorConfig,
    child: Option<Child>,
    stdout_reader: Option<Lines<BufReader<ChildStdout>>>,
    // stdout_reader: Option<BufReader<ChildStdout>>,
}

#[tonic::async_trait]
impl super::Executor for NativeExecutor {
    fn new(config: ExecutorConfig) -> Self {
        debug!("Creating new native executor with config: {:#?}", config);
        Self {
            config,
            child: None,
            stdout_reader: None,
        }
    }

    async fn setup(&self) -> Result<(), Box<dyn Error>> {
        debug!("Setting up execution environment");

        // Check if cwd exists, if not create
        // Self::mkdir_p(Path::new(self.config.cwd.as_str())).await?;
        Self::mkdir_p(&self.config.cwd).await?;

        Ok(())
    }

    fn launch(&mut self) -> Result<(), Box<dyn Error>> {
        debug!("Launching child process");
        let mut cmd = Command::new(self.config.executable.clone());
        cmd
            .args(self.config.args.clone())
            .envs(self.config.envs.clone())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(self.config.cwd.clone());
            // .kill_on_drop(true);

        let mut child = cmd.spawn()?;

        let stdout = child.stdout.take();
        if let Some(out) = stdout {
            self.stdout_reader = Some(BufReader::new(out).lines());
        }
        /*
        let stderr = child.stderr.take();
        if let Some(err) = stderr {
            self.stderr_reader = Some(BufReader::new(err).lines());
        }
        */

        self.child = Some(child);

        Ok(())
    }

    async fn get_stdout_line(&mut self) -> Option<String> {
        match self.stdout_reader {
            Some(ref mut reader) => {
                if let Ok(maybe_line) = reader.next_line().await {
                    maybe_line
                } else {
                    None
                }
            },
            _ => None,
        }
    }

    fn get_stderr_reader(self) -> Option<BufReader<ChildStderr>>
    {
        debug!("Taking out stderr reader");
        // Create async line reader for stdout and stderr
        let stdout = self.child.unwrap().stderr.take();
        if let Some(out) = stdout {
            return Some(BufReader::new(out))
        } else {
            return None
        }
    }

    fn id(&self) -> u32 {
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
