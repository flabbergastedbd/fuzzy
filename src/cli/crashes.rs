use std::error::Error;
use std::path::Path;
use std::time::SystemTime;

use clap::ArgMatches;
use log::{debug, info};

use crate::common::{
    cli::parse_volume_map_settings,
    crashes::{download_crashes, download_crashes_to_disk, update_crash},
    profiles::construct_profile,
    tasks::get_task,
    xpc::get_orchestrator_client,
};
use crate::executor::{crash_deduplicator::CrashDeduplicator, crash_validator::CrashValidator};

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

            let verified = if sub_matches.is_present("verified") {
                Some(true)
            } else {
                None
            };

            let output = sub_matches.value_of("output").map(|s| s.to_owned());

            let task_id = sub_matches
                .value_of("task_id")
                .expect("Task id not provided")
                .parse::<i32>()?;

            let crashes = download_crashes_to_disk(
                None,
                verified,
                output,
                Some(task_id),
                latest,
                SystemTime::UNIX_EPOCH,
                sub_matches.is_present("duplicate"),
                Path::new(path),
                &mut client,
            )
            .await?;

            info!("Successfully downloaded {} crashes to {}", crashes, path);
        }
        ("revalidate", Some(sub_matches)) => {
            parse_volume_map_settings(sub_matches);
            debug!("Revalidating crashes");
            let crash_path = Path::new("crash.fuzzy");
            let task_id = sub_matches
                .value_of("task_id")
                .expect("Task id not provided")
                .parse::<i32>()?;
            let verified = match sub_matches.is_present("all") {
                true => None,
                false => Some(false),
            };

            let task = get_task(task_id, &mut client).await?;
            let config = construct_profile(&task.profile)?;
            let validator = CrashValidator::new(config.crash.clone(), None)?;

            let crashes = download_crashes(
                Some(config.crash.label),
                verified,
                None,
                Some(task.id),
                None,
                std::time::UNIX_EPOCH,
                sub_matches.is_present("duplicate"),
                &mut client,
            )
            .await?;

            for crash in crashes.iter() {
                debug!("Validating crash {:?}", crash);
                tokio::fs::write(crash_path, &crash.content).await?;
                let (output, verified) = validator.validate_crash(crash_path).await?;
                // Set duplicate to None as you need to revalidate
                update_crash(crash.id, verified, output, None, &mut client).await?;
                tokio::fs::remove_file(crash_path).await?;
            }
        }
        ("deduplicate", Some(sub_matches)) => {
            parse_volume_map_settings(sub_matches);
            debug!("Deduplicating crashes");
            let task_id = sub_matches
                .value_of("task_id")
                .expect("Task id not provided")
                .parse::<i32>()?;

            let task = get_task(task_id, &mut client).await?;
            let config = construct_profile(&task.profile)?;

            let mut crashes = download_crashes(
                Some(config.crash.label.clone()),
                Some(true), // Only verified crashes
                None,
                Some(task.id),
                None,
                std::time::UNIX_EPOCH,
                sub_matches.is_present("all"),
                &mut client,
            )
            .await?;
            let _ = crashes.remove(0); // Remove one crash atleast to avoid looping on deduplication

            for crash in crashes.iter() {
                if let Some(output) = crash.output.as_ref() {
                    let deduplicator = CrashDeduplicator::new(config.crash.clone(), crash.worker_task_id)?;
                    info!("Deduplicating crash {:?}", crash.id);
                    let mut dup_crash_id = deduplicator.dedup_crash(output).await?;
                    // Ignore if we detect a duplicate crash with same or greater id or else we
                    // will go in loops
                    if let Some(duplicate_id) = dup_crash_id {
                        if duplicate_id >= crash.id {
                            dup_crash_id = None;
                        }
                    }
                    info!("Updating it with duplicate: {:?}", dup_crash_id);
                    update_crash(
                        crash.id,
                        crash.verified,
                        crash.output.clone(),
                        dup_crash_id,
                        &mut client,
                    )
                    .await?;
                }
            }
        }
        // Listing all tasks
        _ => {}
    }

    Ok(())
}
