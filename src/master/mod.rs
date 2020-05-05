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
    db_connect_str: String,
}

impl Master {
    fn new(
            listen_address: &str,
            db_connect_str: &str
            ) -> Master {
        debug!("Initializing new master");
        let master = Master {
            listen_addr: listen_address.parse().expect("Invalid listen address provided"),
            db_connect_str: String::from(db_connect_str),
        };
        master
    }

    #[tokio::main]
    async fn main_loop(&self) -> Result<(), Box<dyn std::error::Error>> {

        let db_broker = DbBroker::new(self.db_connect_str.clone());
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
    debug!("Master main launched!");

    match arg_matches.subcommand() {
        ("start", Some(sub_matches)) => {
            info!("Starting master agent");
            let master = Master::new(
                sub_matches.value_of("listen_addr").unwrap_or("127.0.0.1:12700"),
                sub_matches.value_of("db_connect_str").unwrap_or("postgres://fuzzy:fuzzy@127.0.0.1:5432/fuzzy"),
            );

            if let Err(e) = master.main_loop() {
                panic!("{}", e);
            }
        },
        _ => {}
    }
}
