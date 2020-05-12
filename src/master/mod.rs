use std::path::Path;
use std::error::Error;
use std::net::SocketAddr;

use clap::ArgMatches;
use log::{error, info, debug};
use tonic::transport::{Server, ServerTlsConfig};
use tokio::signal::unix::{signal, SignalKind};

use crate::db::DbBroker;
use crate::xpc::collector_server::CollectorServer;
use crate::utils::fs::read_file;
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
    async fn main_loop(&self, server_pem_path: &str, ca_cert_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let tls_config = create_server_tls_config(server_pem_path, ca_cert_path).await?;

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
            .tls_config(tls_config)
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

async fn create_server_tls_config(server_pem: &str, ca_cert: &str) -> Result<ServerTlsConfig, Box<dyn Error>> {
    let server_pem = read_file(Path::new(server_pem)).await;
    let ca_cert = read_file(Path::new(ca_cert)).await;
    if server_pem.is_err() || ca_cert.is_err() {
        error!("Unable to find either server pem or ca cert");
    }
    let server_pem = server_pem?;
    let ca_cert = ca_cert?;

    let config = ServerTlsConfig::new()
        .identity(tonic::transport::Identity::from_pem(server_pem.clone(), server_pem))
        .client_ca_root(tonic::transport::Certificate::from_pem(ca_cert));
    Ok(config)
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

            let server_pem_path = sub_matches.value_of("server_pem").unwrap_or("server.pem");
            let ca_cert_path = sub_matches.value_of("ca").unwrap_or("ca.crt");

            if let Err(e) = master.main_loop(server_pem_path, ca_cert_path) {
                error!("Master exited with error: {}", e);
                std::process::exit(1);
            }
        },
        _ => {}
    }
}
