use tonic::transport::Server;
use std::net::SocketAddr;
use clap::ArgMatches;
use log::{info, debug};

use crate::db::DbBroker;
use crate::xpc::stats_server::StatsServer;

mod stats;

#[derive(Debug)]
pub struct Master {
    listen_addr: SocketAddr,
}

impl Master {
    fn new(listen_address: &str) -> Master {
        debug!("Initializing new master");
        // Initialize Master struct which exists for full lifetime
        let master = Master {
            listen_addr: listen_address.parse().expect("Invalid listen address provided"),
        };
        master
    }

    #[tokio::main]
    async fn main_loop(&self) -> Result<(), Box<dyn std::error::Error>> {

        // Initialize all grpc services
        let db_broker = DbBroker::new().await?;
        let stats_service = stats::StatsService::new(db_broker);

        debug!("Starting master event loop on {}", self.listen_addr);
        Server::builder()
            .add_service(StatsServer::new(stats_service))
            .serve(self.listen_addr)
            .await?;

        Ok(())
    }
}

pub fn main(arg_matches: &ArgMatches) {
    debug!("Master main function launched");

    match arg_matches.subcommand() {
        ("start", Some(sub_matches)) => {
            info!("Starting master agent");
            let master = Master::new(sub_matches.value_of("listen_addr").unwrap());

            if let Err(e) = master.main_loop() {
                panic!("{}", e);
            }
        },
        _ => {}
    }
}
