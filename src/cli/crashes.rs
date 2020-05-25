use std::path::Path;
use std::time::SystemTime;
use std::error::Error;

use log::{info, debug};
use clap::ArgMatches;
use tokio::task;

use crate::common::crashes::download_crashes_to_disk;
use crate::common::xpc::get_orchestrator_client;

pub async fn cli(args: &ArgMatches) -> Result<(), Box<dyn Error>> {
    debug!("Creating interface client");
    let mut client = get_orchestrator_client().await?;

    match args.subcommand() {
        ("download", Some(sub_matches)) => {
            debug!("Downloading crashes");
            let path = sub_matches.value_of("path").expect("Path to save corpus not provided");

            let mut latest = None;
            if let Some(l) = sub_matches.value_of("latest") {
                latest = Some(l.parse::<i64>()?);
            }

            let verified = if sub_matches.is_present("verified") { Some(true) } else { None };

            let output = sub_matches.value_of("output").map(|s| s.to_owned());

            let task_id = match sub_matches.value_of("task_id") {
                Some(task_id) => Some(task_id.parse::<i32>()?),
                None => None,
            };

            let crashes = download_crashes_to_disk(
                sub_matches.value_of("label").expect("Label not provided").to_owned(),
                verified,
                output,
                task_id,
                latest,
                SystemTime::UNIX_EPOCH,
                Path::new(path),
                &mut client
            ).await?;

            info!("Successfully downloaded {} crashes to {}", crashes, path);
        },
        // Listing all tasks
        _ => {},
    }

    Ok(())
}
