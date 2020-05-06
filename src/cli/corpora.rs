use std::error::Error;

use log::{error, info, debug};
use clap::ArgMatches;
use tonic::{Request, transport::channel::Channel};
use tokio::{task, fs::File};
use tokio::prelude::*;

use crate::models::NewCorpus;
use crate::xpc::user_interface_client::UserInterfaceClient;

pub async fn upload_corpus(file_path: String, label: String, client: &mut UserInterfaceClient<Channel>) {
    let mut content = vec![];
    let file = File::open(file_path.clone()).await;

    if let Err(e) = file {
        error!("Unable to process file {}: {}", file_path, e);
        return
    } else {
        let mut file = file.unwrap();
        if let Err(e) = file.read_to_end(&mut content).await {
            error!("Unable to process file {}: {}", file_path, e);
            return
        }

        // Generate checksum
        let checksum = crate::common::checksum(&content);

        // Send request
        let new_corpus = NewCorpus {
            content,
            checksum,
            label,
        };
        let response = client.submit_corpus(Request::new(new_corpus)).await;
        if let Err(e) = response {
            error!("Failed to add {}: {:?}", file_path, e);
        } else {
            info!("Successfully added: {}", file_path);
        }
    }
}

pub async fn cli(args: &ArgMatches, connect_addr: String) -> Result<(), Box<dyn Error>> {
    debug!("Creating interface client");
    let client = UserInterfaceClient::connect(connect_addr).await?;

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
                    upload_corpus(file_path, label, &mut local_client).await
                });
            }

            task_set.await;
        },
        // Listing all tasks
        _ => {},
    }

    Ok(())
}
