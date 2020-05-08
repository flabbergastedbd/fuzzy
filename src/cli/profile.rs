use std::path::Path;
use std::error::Error;

use log::{warn, info, error, debug};
use clap::ArgMatches;

use crate::executor::{self, Executor, ExecutorConfig};

pub async fn cli(args: &ArgMatches, _: String) -> Result<(), Box<dyn Error>> {

    match args.subcommand() {
        // Adding a new task
        ("executor", Some(sub_matches)) => {
            debug!("Testing executor profile");
            // Get profile
            let profile = sub_matches.value_of("file_path").unwrap().to_owned();

            // Read profile
            let content = crate::common::read_file(profile).await?;
            let content_str = String::from_utf8(content);
            assert!(content_str.is_ok());

            // Convert to json
            let config: ExecutorConfig = serde_json::from_str(content_str.unwrap().as_str())?;

            // Create Executor
            let mut executor = executor::new(config);

            executor.setup().await?;
            executor.launch().await?;

            info!("Child PID: {}", executor.get_pid());

            if let Some(path) = sub_matches.value_of("watch") {
                debug!("Watching for files in {}", path);
                let path = Path::new(path);
                let mut fw = executor.get_file_watcher(path)?;
                while let Some(file) = fw.get_new_file().await {
                    info!("New file created: {}", file);
                }
            }

            if sub_matches.is_present("stdout") {
                let mut stdout_reader = executor.get_stdout_reader().unwrap();
                let mut stderr_reader = executor.get_stderr_reader().unwrap();
                while let Some(line) = stdout_reader.next_line().await? {
                    info!("Stdout: {}", line);
                }

                while let Some(line) = stderr_reader.next_line().await? {
                    warn!("Stderr: {}", line);
                }
            }

        },
        ("fuzz_driver", Some(_)) => {
            debug!("Testing fuzz driver profile");
        },
        // Listing all tasks
        _ => {},
    }

    Ok(())
}
