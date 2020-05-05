use std::sync::Arc;
use std::error::Error;
use std::fmt;

use clap::ArgMatches;
use log::{error, info, debug};
use num_cpus;
use uuid::Uuid;
use tokio::sync::RwLock;

use crate::xpc::Worker;

mod dispatcher;

impl Worker {
    pub fn new() -> Self {
        debug!("Creating new worker object");
        let worker = Worker {
            uuid: Uuid::new_v4().to_string(),
            name: None,
            cpus: num_cpus::get() as i32,
            active: true,
            // connect_addr: None,
        };
        worker
    }

    // Assign given name to this worker
    pub fn with_name(mut self, name: Option<&str>) -> Self {
        if let Some(custom_name) = name {
            self.name = Some(String::from(custom_name));
        }
        self
    }

    // Assign given name to this worker
    pub fn with_uuid(mut self, id: Option<&str>) -> Self {
        if let Some(custom_id) = id {
            debug!("Parsing for valid uuid");
            self.uuid = Uuid::parse_str(custom_id).unwrap().to_string();
        }
        self
    }
}

impl fmt::Display for Worker {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // This ugly thing has to done for proper string formatting
        writeln!(f, "Worker Info")?;
        writeln!(f, "UUID  : {}", self.uuid)?;

        if self.name.is_some() {
            writeln!(f, "Name: {:?}", self.name)?;
        }

        writeln!(f, "CPUs: {}", self.cpus)
    }
}

#[tokio::main]
pub async fn main_loop(worker: Arc<RwLock<Worker>>, connect_addr: &str) -> Result<(), Box<dyn Error>> {
    let d = dispatcher::Dispatcher::new(String::from(connect_addr));
    // Launch periodic heartbeat dispatcher
    info!("Launching heartbeat task");
    tokio::spawn(d.heartbeat(worker)).await?;

    Ok(())
}

// Called from main if woker subcommand found, parameters can be seen in src/cli.yml
pub fn main(arg_matches: &ArgMatches) {
    debug!("Worker main function launched");

    match arg_matches.subcommand() {
        ("start", Some(sub_matches)) => {
            info!("Starting worker agent");
            let w = Worker::new()
                .with_uuid(sub_matches.value_of("uuid"))
                .with_name(sub_matches.value_of("name"));

            // Start main loop
            if let Err(e) = main_loop(Arc::new(RwLock::new(w)), sub_matches.value_of("connect_addr").unwrap()) {
                error!("{}", e);
                panic!("Failed to start main loop")
            }
        },
        _ => {}
    }
}
