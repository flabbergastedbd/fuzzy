use std::str;
use std::error::Error;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use log::{debug, error};
use tokio::{
    fs,
    io::{AsyncBufReadExt, BufReader, Lines},
    process::{Child, ChildStderr, ChildStdout, Command},
    sync::broadcast,
};

use super::corpus_syncer::CorpusSyncer;
use super::crash_syncer::CrashSyncer;
use super::ExecutorConfig;
use crate::common::executors::{extract_contraint_volume_map, get_container_volume_map};
use crate::fuzz_driver::{CorpusConfig, CrashConfig};
use crate::utils::fs::{mkdir_p, rm_r};
use crate::utils::{checksum, err_output};

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

        // Force pull the image in a blocking fashion
        force_pull_image(self.config.image.clone()).await?;

        // Current working directory is where we mount a volume
        // cwd: Is used to mount a volume at that

        // Create a new working directory just for this task at mapped_path
        mkdir_p(&self.mapped_cwd).await?;
        Ok(())
    }

    async fn create_relative_dirp(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        let mapped_path = self.mapped_cwd.join(path);
        mkdir_p(mapped_path.as_path()).await?;
        Ok(())
    }

    async fn rm_relative_dirp(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        let mapped_path = self.mapped_cwd.join(path);
        rm_r(mapped_path.as_path()).await?;
        Ok(())
    }

    async fn wait(&self, mut kill_switch: broadcast::Receiver<u8>) -> Result<(), Box<dyn Error>> {
        let identifier = self.identifier.clone();
        tokio::select! {
            result = check_if_container_alive(identifier) => {
                result?;
            },
            _ = kill_switch.recv() => {
                debug!("Kill received for docker executor, hope the command dies");
            },
        };
        Ok(())
    }

    async fn spawn(&mut self) -> Result<(), Box<dyn Error>> {
        let mut cmd = self.create_cmd(false).await?;
        let child = cmd.spawn()?;
        self.child = Some(child);
        Ok(())
    }

    async fn spawn_blocking(&mut self) -> Result<std::process::Output, Box<dyn Error>> {
        let mut cmd = self.create_cmd(true).await?;
        let child = cmd.spawn()?;
        Ok(child.wait_with_output().await?)
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
        config.path = self.mapped_cwd.join(config.path).into_boxed_path();
        Ok(CorpusSyncer::new(config, self.worker_task_id)?)
    }

    fn get_crash_syncer(&self, mut config: CrashConfig) -> Result<CrashSyncer, Box<dyn Error>> {
        config.path = self.mapped_cwd.join(config.path).into_boxed_path();
        Ok(CrashSyncer::new(config, self.worker_task_id)?)
    }

    fn get_cwd_path(&self) -> PathBuf {
        self.mapped_cwd.clone()
    }

    async fn close(mut self: Box<Self>) -> Result<(), Box<dyn Error>> {
        // We are here means we need to stop now
        let mut cmd = Command::new("docker");
        cmd.arg("stop").arg(self.identifier.clone()).kill_on_drop(true);

        let output = cmd.output().await?;
        if output.status.success() == false {
            error!("Unable to stop container: {}", self.identifier);
            err_output(output);
        }

        // Remove the container
        let mut cmd = Command::new("docker");
        cmd.arg("rm").arg(self.identifier.clone()).kill_on_drop(true);

        let output = cmd.output().await?;
        if output.status.success() == false {
            error!("Unable to remove container: {}", self.identifier);
            err_output(output);
        }

        Ok(())
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

        // Create unique identifier
        let mut unique_string = format!("{}", &worker_task_id.as_ref().unwrap_or(&0));
        unique_string.push_str(config.executable.as_str());
        for arg in &config.args {
            unique_string.push_str(arg.as_str());
        }
        let identifier = checksum(&unique_string.into_bytes());
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

    async fn create_cmd(&self, blocking: bool) -> Result<Command, Box<dyn Error>> {
        // Since we created this folder this should be our uid
        let cwd_metadata = fs::metadata(&self.mapped_cwd).await?;
        let uid = cwd_metadata.uid();
        let gid = cwd_metadata.gid();

        debug!("Constructing args for docker process");
        let mut cmd = Command::new("docker");
        cmd.arg("run")
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

        if blocking == false {
            cmd.arg("-d");
        } else {
            cmd.arg("--rm") // Auto removal of container if blocking
                .arg("--attach=STDOUT")
                .arg("--attach=STDERR");
        }

        // Set cwd volume
        let target_container_cwd = self.config.cwd.to_str();
        let host_cwd = self.host_cwd.to_str();

        // Set working directory inside target container
        if target_container_cwd.is_some() {
            cmd.arg(format!("--workdir={}", target_container_cwd.unwrap()));

            if host_cwd.is_some() {
                cmd.arg(format!(
                    "--volume={}:{}",
                    host_cwd.unwrap(),
                    target_container_cwd.unwrap()
                ));
            }
        }

        // Iterate over envs and set keys, docker will take them from
        // launch environment, which we set using cmd.envs()
        for (key, _) in self.config.envs.iter() {
            cmd.arg(format!("-e={}", key));
        }

        cmd.arg(self.config.image.clone())
            .args(self.config.args.clone())
            .envs(self.config.envs.clone())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(self.mapped_cwd.as_path())
            .kill_on_drop(true);
        debug!("Command: {:#?}", cmd);
        Ok(cmd)
    }
}

async fn force_pull_image(image: String) -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::new("docker");
    cmd.arg("pull")
        .arg(image.clone())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let child = cmd.spawn()?;
    let output = child.wait_with_output().await?;

    if output.status.success() == false {
        error!("Image pull exited with status: {:?}", output.status.code());
        error!("Stdout: {:?}", str::from_utf8(&output.stdout).unwrap_or("Couldn't UTF-8 decode stdout"));
        error!("Stderr: {:?}", str::from_utf8(&output.stderr).unwrap_or("Couldn't UTF-8 decode stderr"));
        Err(
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unable to pull image: {}", &image)
            ))
        )
    } else {
        Ok(())
    }
}

// This check should be very liberal
async fn check_if_container_alive(identitifer: String) -> Result<(), Box<dyn Error>> {
    let mut interval = tokio::time::interval(crate::common::intervals::WORKER_PROCESS_CHECK_INTERVAL);
    let name_filter = format!("name={}", identitifer);
    let mut fail_count = 0;
    loop {
        interval.tick().await;
        let mut cmd = Command::new("docker");
        cmd.arg("ps")
            .arg("-f")
            .arg(name_filter.as_str())
            .arg("--format={{.ID}}")
            .kill_on_drop(true);
        let output = cmd.output().await?;
        if output.stdout.len() == 0 {
            if fail_count > 4 {
                break;
            } else {
                fail_count = fail_count + 1;
            }
        }
    }
    Ok(())
}
