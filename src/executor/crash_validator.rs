use std::path::Path;
use std::error::Error;

use log::info;

use crate::executor;
use crate::fuzz_driver::CrashConfig;
use crate::utils::fs::rm_r;

/// A file system corpus syncer. Need to convert this into trait when implementing docker
pub struct CrashValidator {
    config: CrashConfig,
    worker_task_id: Option<i32>,
}

impl CrashValidator {
    pub fn new(config: CrashConfig, worker_task_id: Option<i32>) -> Result<Self, Box<dyn Error>> {
        Ok(Self { config, worker_task_id })
    }

    pub async fn validate_crash(&self, crash: &Path) -> Result<(Option<String>, bool), Box<dyn Error>> {
        if let Some(mut exec_config) = self.config.validate.clone() {
            // Create a temporary name that will be passed as argv[-1]
            let relative_file_name = "crash.fuzzy";
            exec_config.args.push(relative_file_name.to_owned());

            // Create executor
            let mut executor = executor::new(exec_config, self.worker_task_id);
            executor.setup().await?;

            // Copy crash file into cwd of validate
            let cwd = executor.get_cwd_path();
            let temp_path = cwd.join(relative_file_name);
            // Copy file into cwd
            tokio::fs::copy(crash, temp_path.as_path()).await?;

            let output = executor.spawn_blocking().await?;

            // Any non zero exit code, we mark crash as verified
            let verified = output.status.success() == false;

            let stdout = std::str::from_utf8(&output.stdout)?;
            let stderr = std::str::from_utf8(&output.stderr)?;

            // Join stdout/stderr for now as output
            let output = format!("STDOUT\n\
                                  ------\n\
                                  {}\n\
                                  STDERR\n\
                                  ------\n\
                                  {}\n", stdout, stderr
            );

            // Remove cwd
            rm_r(&cwd).await?;
            Ok((Some(output), verified))
        } else {
            info!("Not validating crash {:?} as no validate in profile", crash);
            Ok((None, false))
        }
    }
}
