use std::process::Stdio;
use std::error::Error;
use std::path::PathBuf;
use std::os::unix::fs::MetadataExt;

use log::debug;
use tokio::{
    fs,
    process::{Command, Child, ChildStdout, ChildStderr},
    io::{BufReader, AsyncBufReadExt, Lines},
};

use super::{CrashConfig, ExecutorConfig};
use super::corpus_syncer::CorpusSyncer;
use super::crash_syncer::CrashSyncer;
use crate::common::executors::{extract_contraint_volume_map, get_container_volume_map};
use crate::utils::fs::mkdir_p;

/// config.cwd is used only to mount a volume at that path & run command
/// from there when starting docker container
pub struct DockerExecutor {
    config: ExecutorConfig,
    child: Option<Child>,
    worker_task_id: Option<i32>,

    identifier: String,
    /// mapped_path (On fuzzy's container) == host_path (On Host) == config.cwd (On target
    // Path accessible to fuzzy as cwd, that is also cwd for fuzzer
    // inside docker but with a different name
    mapped_cwd: PathBuf,
    host_cwd: PathBuf,
}

#[tonic::async_trait]
impl super::Executor for DockerExecutor {

    async fn setup(&self) -> Result<(), Box<dyn Error>> {
        debug!("Setting up docker execution environment");

        // Current working directory is where we mount a volume
        // cwd: Is used to mount a volume at that

        // Create a new working directory just for this task at mapped_path
        mkdir_p(&self.mapped_cwd).await?;

        let mapped_corpus_path = self.mapped_cwd.join(&self.config.corpus.path);
        mkdir_p(mapped_corpus_path.as_path()).await?;

        Ok(())
    }

    async fn spawn(&mut self) -> Result<(), Box<dyn Error>> {
        // Since we created this folder this should be our uid
        let cwd_metadata = fs::metadata(&self.mapped_cwd).await?;
        let uid = cwd_metadata.uid();
        let gid = cwd_metadata.gid();

        debug!("Constructing args for docker process");
        let mut cmd = Command::new("docker");
        cmd
            .arg("run")
            .arg("--attach=STDOUT")
            .arg("--attach=STDERR")
            /*
            .arg("--net=host")
            .arg("--ipc=host")
            .arg("--uts=host")
            .arg("--pid=host")
            .arg("--userns=host")
            */
            .arg("--privileged") // TODO: Rather just mount devices required for fuzzers like shm or kvm
            .arg(format!("--user={}:{}", uid, gid))
            .arg(format!("--name={}", self.identifier))
            .arg(format!("--entrypoint={}", self.config.executable));

        // Set cwd volume
        let target_container_cwd = self.config.cwd.to_str();
        let host_cwd = self.host_cwd.to_str();

        // Set working directory inside target container
        if target_container_cwd.is_some() {
            cmd.arg(format!("--workdir={}", target_container_cwd.unwrap()));

            if host_cwd.is_some() {
                cmd.arg(format!("--volume={}:{}", host_cwd.unwrap(), target_container_cwd.unwrap()));
            }
        }

        // Iterate over envs and set keys, docker will take them from
        // launch environment, which we set using cmd.envs()
        for (key, _) in self.config.envs.iter() {
            cmd.arg(format!("-e={}", key));
        }

        cmd
            .arg(self.config.image.clone())
            .args(self.config.args.clone())
            .envs(self.config.envs.clone())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(self.mapped_cwd.as_path());
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
        corpus_config.path = self.mapped_cwd.join(corpus_config.path).into_boxed_path();
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
        self.mapped_cwd.clone()
    }

    fn close(&mut self) -> Result<(), Box<dyn Error>> {
        debug!("Closing out executor");
        Ok(self.child.as_mut().map(|c| c.kill()).unwrap()?)
    }
}

impl DockerExecutor {
    pub fn new(config: ExecutorConfig, worker_task_id: Option<i32>) -> Self {
        debug!("Creating new docker executor with config: {:#?}", config);
        let volume_path = get_container_volume_map();
        if volume_path.is_err() {
            panic!("This is bad, volume path doesn't seem to be set!");
        }
        let (host_path, mapped_path) = extract_contraint_volume_map(volume_path.unwrap().as_ref());

        let mut identifier = uuid::Uuid::new_v4().to_string();
        identifier.push_str("-");
        identifier.push_str(worker_task_id.as_ref().unwrap_or(&0).to_string().as_ref());
        debug!("Created new identifier for docker executor: {}", identifier);
        // Append a folder to both host & mapped path so that we don't collide different docker
        // executor instances
        let mapped_cwd = mapped_path.as_path().join(&identifier);
        let host_cwd = host_path.as_path().join(&identifier);

        Self {
            config,
            child: None,
            worker_task_id,
            mapped_cwd,
            identifier,
            host_cwd,
        }
    }
}
