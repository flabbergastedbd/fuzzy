use log::{error, debug};
use std::error::Error;

use clap::ArgMatches;
use tokio::task;

use crate::executor::ExecutorConfig;

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

            // Local task set
            let task_set = task::LocalSet::new();

            task_set.await;
        },
        ("fuzz_driver", Some(_)) => {
            debug!("Testing fuzz driver profile");
        },
        // Listing all tasks
        _ => {},
    }

    Ok(())
}
