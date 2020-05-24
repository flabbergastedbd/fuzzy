use std::path::Path;
use std::time::SystemTime;
use std::error::Error;

use log::{info, debug};
use clap::ArgMatches;
use tokio::task;

use crate::common::corpora::{delete_corpus, upload_corpus_from_disk, download_corpus_to_disk};
use crate::common::xpc::get_orchestrator_client;

pub async fn cli(args: &ArgMatches) -> Result<(), Box<dyn Error>> {
    debug!("Creating interface client");
    let mut client = get_orchestrator_client().await?;

    match args.subcommand() {
        // Adding a new task
        ("add", Some(sub_matches)) => {
            debug!("Adding a new corpus");
            // Get all files
            let files = sub_matches.values_of("file_path").unwrap();
            // Local task set
            let task_set = task::LocalSet::new();

            for file_path in files.into_iter() {
                let file_path        = file_path.to_owned();
                let label            = sub_matches.value_of("label").unwrap().to_owned();
                let mut local_client = client.clone(); // Create new client clones to pass
                task_set.spawn_local(async move {
                    upload_corpus_from_disk(Path::new(file_path.as_str()), label, None, &mut local_client).await
                });
            }
            task_set.await;
        },
        ("download", Some(sub_matches)) => {
            debug!("Downloading corpus");
            let path = sub_matches.value_of("path").expect("Path to save corpus not provided");

            let corpora = download_corpus_to_disk(
                sub_matches.value_of("label").expect("Label not provided").to_owned(),
                None,
                None,
                None,
                SystemTime::UNIX_EPOCH,
                Path::new(path),
                &mut client
            ).await?;

            info!("Successfully downloaded {} corpus to {}", corpora, path);
        },
        ("delete", Some(sub_matches)) => {
            let label = sub_matches.value_of("label").expect("Label to delete not provided").to_owned();

            let _ = delete_corpus(
                label,
                None,
                None,
                None,
                SystemTime::UNIX_EPOCH,
                &mut client
            ).await?;
        },
        // Listing all tasks
        _ => {},
    }

    Ok(())
}
