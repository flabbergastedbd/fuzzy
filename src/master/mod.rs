use clap::ArgMatches;
use log::{info, debug};
use std::net::SocketAddr;
use tonic::transport::Server;

use crate::db::DbBroker;
use crate::xpc::collector_server::CollectorServer;

mod collector;

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

        let db_broker = DbBroker::new();
        // Initialize all grpc services with database handle
        let collector_service = collector::CollectorService::new(db_broker);

        debug!("Starting master event loop on {}", self.listen_addr);
        Server::builder()
            .add_service(CollectorServer::new(collector_service))
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
