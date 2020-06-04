use std::path::Path;
use std::time::SystemTime;
use std::error::Error;

use log::{info, debug};
use clap::ArgMatches;

use crate::common::{
    crashes::{download_crashes, update_crash, download_crashes_to_disk},
    xpc::get_orchestrator_client,
    tasks::get_task,
    profiles::construct_profile,
    cli::parse_volume_map_settings,
};
use crate::executor::crash_validator::CrashValidator;

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
        ("revalidate", Some(sub_matches)) => {
            parse_volume_map_settings(sub_matches);
            debug!("Revalidating crashes");
            let crash_path = Path::new("crash.fuzzy");
            let task_id = sub_matches.value_of("task_id").expect("Task id not provided").parse::<i32>()?;
            let verified = match sub_matches.is_present("all") {
                true => None,
                false => Some(false),
            };

            let task = get_task(task_id, &mut client).await?;
            let config = construct_profile(&task.profile)?;
            let validator = CrashValidator::new(config.crash.clone(), None)?;

            let crashes = download_crashes(
                config.crash.label,
                verified,
                None,
                Some(task.id),
                None,
                std::time::UNIX_EPOCH,
                &mut client
            ).await?;

            for crash in crashes.iter() {
                debug!("Validating crash {:?}", crash);
                tokio::fs::write(crash_path, &crash.content).await?;
                let (output, verified) = validator.validate_crash(crash_path).await?;
                update_crash(crash.id, verified, output, &mut client).await?;
                tokio::fs::remove_file(crash_path).await?;
            }
        },
        // Listing all tasks
        _ => {},
    }

    Ok(())
}
