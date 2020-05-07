use std::error::Error;

use log::debug;
use clap::ArgMatches;
use tokio::task;

use crate::common::{upload_corpus, get_corpus};
use crate::xpc::orchestrator_client::OrchestratorClient;

pub async fn cli(args: &ArgMatches, connect_addr: String) -> Result<(), Box<dyn Error>> {
    debug!("Creating interface client");
    let mut client = OrchestratorClient::connect(connect_addr).await?;

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
        ("list", Some(sub_matches)) => {
            debug!("Listing corpus");

            let corpora = get_corpus(
                sub_matches.value_of("label").unwrap().to_owned(),
                &mut client
            ).await;

            super::formatter::print_corpora(corpora);
        },
        // Listing all tasks
        _ => {},
    }

    Ok(())
}
