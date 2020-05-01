use clap::ArgMatches;
use log::{info, debug};
use tonic::transport::server::Server;

use crate::worker::Worker;

mod heartbeat;
use heartbeat::comms::master_server::MasterServer as MasterCommsServer;

#[derive(Debug)]
pub struct Master {
    workers: Vec<Worker>,
    comms: heartbeat::CommsService,
}

impl Master {
    fn new(listen_address: &str) -> Master {
        debug!("Initializing new master");

        // Create sub services
        let comms = heartbeat::CommsService::new(listen_address.parse()
            .expect("Invalid listen address provided"));

        // Initialize Master struct
        let master = Master {
            workers: Vec::new(),
            comms,
        };
        master
    }

    #[tokio::main]
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let comms = MasterCommsServer::new(self.comms);

        Server::builder()
            .add_service(comms)
            .serve(self.comms.get_listen_addr())
            .await?;

        Ok(())
    }
}


#[allow(unused_must_use)]
pub fn main(arg_matches: &ArgMatches) {
    debug!("Master main function launched");

    match arg_matches.subcommand() {
        ("start", Some(sub_matches)) => {
            info!("Starting master agent");
            let master = Master::new(sub_matches.value_of("listen_addr").unwrap());

            debug!("Starting main tokio loop");
            master.start();
        },
        _ => {}
    }
}
