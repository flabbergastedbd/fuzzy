use std::error::Error;

use log::debug;
use clap::ArgMatches;
use tonic::Request;
use tokio::fs::File;
use tokio::prelude::*;

use crate::models::NewCorpus;
use crate::xpc::user_interface_client::UserInterfaceClient;

pub async fn cli(args: &ArgMatches, connect_addr: String) -> Result<(), Box<dyn Error>> {
    debug!("Creating interface client");
    let mut client = UserInterfaceClient::connect(connect_addr).await?;

    match args.subcommand() {
        // Adding a new task
        ("add", Some(sub_matches)) => {
            debug!("Adding a new corpus");
            // Read new file
            let mut file = File::open(sub_matches.value_of("file_path").unwrap()).await?;
            let mut content = vec![];
            file.read_to_end(&mut content).await?;

            // Generate checksum
            let checksum = crate::common::checksum(&content);

            // Send request
            let new_corpus = NewCorpus {
                content,
                checksum,
                task_id: sub_matches.value_of("task").unwrap().parse::<i32>().unwrap(),
            };
            let response = client.submit_corpus(Request::new(new_corpus)).await?;
            // TODO: Error handling
            println!("{:?}", response);
        },
        // Listing all tasks
        _ => {},
    }

    Ok(())
}
