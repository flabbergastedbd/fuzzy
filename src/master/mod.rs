use std::net::SocketAddr;

use clap::ArgMatches;
use log::{error, info, debug};
use tonic::transport::Server;
use tokio::{sync::RwLock, signal::unix::{signal, SignalKind}};

use crate::db::DbBroker;
use crate::xpc::collector_server::CollectorServer;
use interface::{OrchestratorService, OrchestratorServer};

mod collector;
mod interface;
mod scheduler;

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
        let collector_service = collector::CollectorService::new(db_broker.clone());
        let orchestrator_service = OrchestratorService::new(db_broker.clone());

        // Spawn off scheduler service
        let scheduler_service = scheduler::Scheduler::new(db_broker.clone());
        let scheduler_handle = tokio::spawn(async move {
            if let Err(e) = scheduler_service.spawn().await {
                error!("Scheduler exited early with error: {}", e);
            }
        });

        debug!("Starting master event loop on {}", self.listen_addr);
        let interface_server = Server::builder()
            .add_service(CollectorServer::new(collector_service))
            .add_service(OrchestratorServer::new(orchestrator_service))
            .serve(self.listen_addr);

        let mut stream = signal(SignalKind::interrupt())?;
        tokio::select! {
            _ = scheduler_handle => {},
            _ = interface_server => {},
            _ = stream.recv() => {
                info!("Keyboard interrput received");
            },
        }

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
