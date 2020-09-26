use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;

use tokio::sync::mpsc::Receiver;
use clap::ArgMatches;
use heim::units::information;
use tracing::{trace_span, debug, error, info, warn};
use tokio::signal::unix::{signal, SignalKind};
use tracing_futures::Instrument;
use uuid::Uuid;

use crate::TraceEvent;
use crate::common::cli::{parse_global_settings, parse_volume_map_settings};
use crate::models::{NewWorker, Worker};
use crate::xpc::collector_client::CollectorClient;

mod dispatcher;
mod tasks;
pub mod log_layer;

const METADATA_PATH: &str = ".fuzzy_worker.yaml";

impl NewWorker {
    pub fn new() -> Self {
        let worker = NewWorker::load_from_cwd();
        if let Ok(worker) = worker {
            worker
        } else {
            warn!("Unable to load from cwd, may be due to first run as well: {:?}", worker);
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
    }

    pub fn load_from_cwd() -> Result<Self, Box<dyn Error>> {
        let path = Path::new(METADATA_PATH);
        let metadata = fs::read_to_string(path)?;
        let worker: NewWorker = serde_yaml::from_str(&metadata)?;
        Ok(worker)
    }

    pub fn save_to_cwd(&self) -> Result<(), Box<dyn Error>> {
        let worker = serde_yaml::to_string(self)?;
        fs::write(METADATA_PATH, worker)?;
        Ok(())
    }

    // Assign given name to this worker
    pub fn with_name(mut self, name: Option<&str>) -> Self {
        if let Some(custom_name) = name {
            self.name = Some(custom_name.to_owned());
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

    pub async fn get_worker_info(&self) -> Result<Worker, Box<dyn Error>> {
        let endpoint = crate::common::xpc::get_server_endpoint().await?;
        let channel = endpoint.connect().await?;
        // Send heartbeat
        let mut client = CollectorClient::new(channel);
        let request = tonic::Request::new(self.clone());
        let response = client.heartbeat(request).await?;
        let worker = response.into_inner();
        Ok(worker)
    }
}

impl fmt::Display for NewWorker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // This ugly thing has to done for proper string formatting
        writeln!(f, "Worker Info")?;
        writeln!(f, "UUID  : {}", self.uuid)?;
        writeln!(f, "Name  : {:?}", self.name)?;
        writeln!(f, "CPUs  : {}", self.cpus)
    }
}

#[tokio::main]
pub async fn main_loop(mut new_worker: NewWorker, trace_rx: Receiver<TraceEvent>) -> Result<(), Box<dyn Error>> {
    // Launch a cpu update task, because of well `heim` and async only
    // Then get worker info struct
    new_worker.update_self().await;
    let worker = new_worker.get_worker_info().await?;

    // Launch periodic heartbeat dispatcher
    info!("Launching heartbeat task");
    let worker_clone = worker.clone();
    let heartbeat_handle = tokio::spawn(async move {
        if let Err(e) = dispatcher::heartbeat(worker_clone, trace_rx).await {
            error!("Dispatcher exited with error: {}", e);
        }
    });

    // Set worker.id
    let span = trace_span!("worker", worker_id=worker.id);
    let _guard = span.enter();

    // Launch task manager
    let mut task_manager = tasks::TaskManager::new();
    info!("Launching task manager task");
    let task_manager_handle = tokio::spawn(async move {
        if let Err(e) = task_manager.spawn(worker).in_current_span().await {
            error!("Task manager exited with error: {}", e);
        }
    });

    // Listen for SIGINT
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
pub fn main(arg_matches: &ArgMatches, trace_rx: Receiver<TraceEvent>) {
    debug!("Worker main function launched");

    match arg_matches.subcommand() {
        ("start", Some(sub_matches)) => {
            info!("Starting worker agent");
            let w = NewWorker::new()
                .with_uuid(sub_matches.value_of("uuid"))
                .with_name(sub_matches.value_of("name"));

            if let Err(e) = w.save_to_cwd() {
                error!("Failed to save metadata to cwd: {}", e);
            }

            parse_global_settings(sub_matches);
            parse_volume_map_settings(sub_matches);

            // Start main loop
            if let Err(e) = main_loop(w, trace_rx) {
                error!("{}", e);
                panic!("Failed to start main loop")
            }
        }
        _ => {}
    }
}
