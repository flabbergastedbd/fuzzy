use std::fmt;

use clap::ArgMatches;
use log::{info, debug};
use num_cpus;
use uuid::Uuid;

#[derive(Debug)]
pub struct Worker {
    id: Uuid,
    name: String,
    cpus: u8,
    connect_addr: String,
}

impl Worker {
    pub fn new() -> Worker {
        debug!("Creating new worker object");
        let worker = Worker {
            id: Uuid::new_v4(),
            name: String::new(),
            cpus: num_cpus::get() as u8,
            connect_addr: String::new(),
        };
        worker
    }

    pub fn name(mut self, name: Option<&str>) -> Worker {
        if let Some(custom_name) = name {
            self.name = String::from(custom_name);
        }
        self
    }

    pub fn connect_addr(mut self, connect_addr: Option<&str>) -> Worker {
        if let Some(addr) = connect_addr {
            self.name = String::from(addr);
        }
        self
    }
}

impl fmt::Display for Worker {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // This ugly thing has to done for proper string formatting
        writeln!(f, "Worker Info")?;
        writeln!(f, "ID  : {}", self.id)?;
        writeln!(f, "Name: {}", self.name)?;
        writeln!(f, "CPUs: {}", self.cpus)
    }
}

pub fn main(arg_matches: &ArgMatches) {
    debug!("Worker main function launched");

    match arg_matches.subcommand() {
        ("start", Some(sub_matches)) => {
            info!("Starting worker agent");
            let w = Worker::new()
                        .name(sub_matches.value_of("id"))
                        .connect_addr(sub_matches.value_of("connect_addr"));
            // Set name if provided
            info!("{}", w);
        },
        _ => {}
    }
}
