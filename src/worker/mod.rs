use std::env;
use std::sync::Arc;
use std::error::Error;
use std::fmt;

use clap::ArgMatches;
use log::{error, info, debug};
use uuid::Uuid;
use tokio::{sync::RwLock, signal::unix::{signal, SignalKind}};
use heim::units::information;

use crate::models::NewWorker;
use crate::common::constants::WORKER_CONNECT_ADDR_ENV_KEY;

mod dispatcher;
mod tasks;

impl NewWorker {
    pub fn new() -> Self {
        debug!("Creating new worker object");
        let worker = NewWorker {
            uuid: Uuid::new_v4().to_string(),
            name: None,
            cpus: 0,
            memory: 0,
            active: true,
        };
        worker
    }

    // Assign given name to this worker
    pub fn with_name(mut self, name: Option<&str>) -> Self {
        if let Some(custom_name) = name {
            self.name = Some(custom_name.to_owned());
        } self
    }

    // Assign given name to this worker
    pub fn with_uuid(mut self, id: Option<&str>) -> Self {
        if let Some(custom_id) = id {
            debug!("Parsing for valid uuid");
            self.uuid = Uuid::parse_str(custom_id).unwrap().to_string();
        }
        self
    }

    pub async fn update_self(&mut self) {
        // Update CPU
        let cpus = heim::cpu::logical_count().await;
        if let Err(e) = cpus {
            panic!("Failed to get cpu count: {}", e);
        } else {
            self.cpus = cpus.unwrap() as i32;
        }

        // Update Memory
        let memory = heim::memory::memory().await;
        if let Err(e) = memory {
            panic!("Failed to get memory: {}", e);
        } else {
            self.memory = memory.unwrap().total().get::<information::megabyte>() as i32;
        }
    }
}

impl fmt::Display for NewWorker {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // This ugly thing has to done for proper string formatting
        writeln!(f, "Worker Info")?;
        writeln!(f, "UUID  : {}", self.uuid)?;
        writeln!(f, "Name  : {:?}", self.name)?;
        writeln!(f, "CPUs  : {}", self.cpus)
    }
}

#[tokio::main]
pub async fn main_loop(worker: Arc<RwLock<NewWorker>>) -> Result<(), Box<dyn Error>> {
    // Launch a cpu update task, because of well `heim` and async only
    let worker_clone = worker.clone();
    tokio::task::spawn(async move {
        let mut worker_writable = worker_clone.write().await;
        worker_writable.update_self().await
    });


    // Launch periodic heartbeat dispatcher
    info!("Launching heartbeat task");
    let worker_clone = worker.clone();
    let heartbeat_handle = tokio::spawn(async move {
        // let d = dispatcher::Dispatcher::new();
        dispatcher::heartbeat(worker_clone).await;
    });

    // Launch task manager
    let mut task_manager = tasks::TaskManager::new();
    let worker_clone = worker.clone();
    info!("Launching task manager task");
    let task_manager_handle = tokio::spawn(async move {
        if let Err(e) = task_manager.spawn(worker_clone).await {
            error!("Task manager exited with error: {}", e);
        }
    });

    let mut stream = signal(SignalKind::interrupt())?;
    tokio::select! {
        result = heartbeat_handle => {
            if let Err(e) = result {
                error!("Heartbeat handle exited first: {}", e);
            }
        },
        result = task_manager_handle => {
            if let Err(e) = result {
                error!("Task manager exited first: {}", e);
            }
        },
        _ = stream.recv() => {
            info!("Keyboard interrput received");
        },
    }

    Ok(())
}

// Called from main if woker subcommand found, parameters can be seen in src/cli.yml
pub fn main(arg_matches: &ArgMatches) {
    debug!("Worker main function launched");

    match arg_matches.subcommand() {
        ("start", Some(sub_matches)) => {
            info!("Starting worker agent");
            let w = NewWorker::new()
                .with_uuid(sub_matches.value_of("uuid"))
                .with_name(sub_matches.value_of("name"));

            // Set up connect addr environment variable
            if let Some(connect_addr) = sub_matches.value_of("connect_addr") {
                env::set_var(WORKER_CONNECT_ADDR_ENV_KEY, connect_addr);
            } else {
                error!("Connect address not provided");
                panic!("Exiting");
            }

            // Start main loop
            if let Err(e) = main_loop(Arc::new(RwLock::new(w))) {
                error!("{}", e);
                panic!("Failed to start main loop")
            }
        },
        _ => {}
    }
}
