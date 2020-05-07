use std::error::Error;

use log::{warn, info, error, debug};
use clap::ArgMatches;
use tokio::io::AsyncBufReadExt;

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
            executor.launch()?;

            /*
                let reader = executor.get_stdout_reader().unwrap();
                let mut lines = reader.lines();
                while let Some(l) = lines.next_line().await? {
                    info!("{}", l);
                }
            */

            while let Some(line) = executor.get_stdout_line().await {
                info!("{}", line);
            }

            info!("{}", executor.id())

        },
        ("fuzz_driver", Some(_)) => {
            debug!("Testing fuzz driver profile");
        },
        // Listing all tasks
        _ => {},
    }

    Ok(())
}
