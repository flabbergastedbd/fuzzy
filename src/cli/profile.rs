use std::path::Path;
use std::error::Error;

use log::{warn, info, error, debug};
use clap::ArgMatches;
use tokio::task::LocalSet;

use crate::executor::{self, Executor, ExecutorConfig};

pub async fn cli(args: &ArgMatches, connect_addr: String) -> Result<(), Box<dyn Error>> {

    match args.subcommand() {
        // Adding a new task
        ("executor", Some(sub_matches)) => {
            debug!("Testing executor profile");
            // Get profile
            let profile = sub_matches.value_of("file_path").unwrap();

            // Read profile
            let content = crate::common::read_file(Path::new(profile)).await?;
            let content_str = String::from_utf8(content);
            assert!(content_str.is_ok());

            // Convert to json
            let config: ExecutorConfig = serde_json::from_str(content_str.unwrap().as_str())?;

            // Create Executor
            let mut executor = executor::new(config, None);

            executor.setup().await?;


            let local_set = LocalSet::new();

            // Spawn off corpus sync
            let corpus_syncer = executor.get_corpus_syncer()?;
            corpus_syncer.setup_corpus(connect_addr.clone()).await?;
            local_set.spawn_local(async move {
                if let Err(e) = corpus_syncer.sync_corpus(connect_addr.clone()).await {
                    error!("Unable to sync corpus: {}", e);
                }
            });

            executor.launch().await?;
            info!("Child PID: {:?}", executor.get_pid());

            // Spawn off stdout output
            let mut stdout_reader = executor.get_stdout_reader().unwrap();
            local_set.spawn_local(async move {
                while let Ok(Some(line)) = stdout_reader.next_line().await {
                    info!("Stdout: {}", line);
                }
            });

            // Spawn off stderr output
            let mut stderr_reader = executor.get_stderr_reader().unwrap();
            local_set.spawn_local(async move {
                while let Ok(Some(line)) = stderr_reader.next_line().await {
                    warn!("Stderr: {}", line);
                }
            });

            local_set.await;


        },
        ("fuzz_driver", Some(_)) => {
            debug!("Testing fuzz driver profile");
        },
        // Listing all tasks
        _ => {},
    }

    Ok(())
}
