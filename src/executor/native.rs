use std::process::Stdio;
use std::error::Error;
use std::path::Path;

use log::debug;
use super::ExecutorConfig;
use tokio::{fs, process::{self, Command, Child}};

#[derive(Debug)]
pub struct NativeExecutor {
    config: ExecutorConfig,
    child: Option<Child>,
}

#[tonic::async_trait]
impl super::Executor for NativeExecutor {
    fn new(config: ExecutorConfig) -> Self {
        debug!("Creating new native executor with config: {:#?}", config);
        Self {
            config,
            child: None
        }
    }

    async fn setup(self) -> Result<(), Box<dyn Error>> {
        debug!("Setting up execution environment");

        // Check if cwd exists, if not create
        Self::mkdir_p(&self.config.cwd).await?;

        Ok(())
    }

    fn launch(mut self) -> Result<(), Box<dyn Error>> {
        debug!("Launching child process");
        let mut cmd = Command::new(self.config.executable);
        cmd
            .args(self.config.args)
            .envs(self.config.envs)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(self.config.cwd);

        self.child = Some(cmd.spawn()?);

        Ok(())
    }

    async fn grab_stdout(self) -> Result<Vec<u8>, Box<dyn Error>> {
        debug!("Grabbing stdout of child process");

        Ok(vec![])
    }
}

impl NativeExecutor {
    async fn mkdir_p(path: &Path) -> std::io::Result<()> {
        debug!("Creating directory tree");
        fs::create_dir_all(path).await?;
        Ok(())
    }
}
