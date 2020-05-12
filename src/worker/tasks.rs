use std::sync::Arc;
use std::error::Error;
use std::collections::HashMap;

use log::{warn, debug, error, info};
use tokio::{sync::RwLock, sync::oneshot, task::JoinHandle};

use crate::xpc;
use crate::worker::NewWorker;
use crate::fuzz_driver::{self, FuzzConfig, FuzzDriver};
use crate::common::xpc::{get_connect_url, get_orchestrator_client};
use crate::xpc::orchestrator_client::OrchestratorClient;
use crate::common::intervals::WORKER_TASK_REFRESH_INTERVAL;

pub struct TaskManager {
    driver_handles: HashMap<i32, JoinHandle<()>>,
    kill_switches: HashMap<i32, oneshot::Sender<u8>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            driver_handles: HashMap::new(),
            kill_switches: HashMap::new(),
        }
    }

    async fn remove_worker_task(&mut self, worker_task_id: &i32) -> Result<(), Box<dyn Error>> {
        debug!("Removing worker task: {:?}", worker_task_id);
        let driver_handle = self.driver_handles.remove(worker_task_id).unwrap();
        let kill_switch = self.kill_switches.remove(worker_task_id).unwrap();

        debug!("Sending kill command");
        let _ = kill_switch.send(0u8);
        debug!("Joining driver task");
        driver_handle.await?;
        Ok(())
    }

    async fn add_worker_task(&mut self, wtask: xpc::WorkerTaskFull) -> Result<(), Box<dyn Error>> {
        debug!("Adding worker task: {:?}", wtask);
        let profile: FuzzConfig = serde_json::from_str(wtask.task.profile.as_str())?;
        let (tx, rx) = oneshot::channel::<u8>();
        let driver = fuzz_driver::new(profile, Some(wtask.id));

        // self.drivers.insert(wtask.id,Box::new(driver));
        self.kill_switches.insert(wtask.id, tx);

        info!("Spawning new task: {:#?}", wtask);

        let driver_handle = tokio::spawn(async move {
            if let Err(e) = driver.start(rx).await {
                error!("Driver exited with error: {}", e);
            }
        });

        self.driver_handles.insert(wtask.id, driver_handle);

        Ok(())
    }

    async fn handle_tasks_update(&mut self, worker_tasks: Vec<xpc::WorkerTaskFull>) -> Result<(), Box<dyn Error>> {
        debug!("Handling task updates, iterating over {} tasks", worker_tasks.len());
        for worker_task in worker_tasks.into_iter() {
            debug!("Looping on task: {:?}", worker_task);
            // Remove if we run the worker_task but active is false
            if self.driver_handles.contains_key(&worker_task.id) && worker_task.task.active == false {
                self.remove_worker_task(&worker_task.id).await?;
            } else if self.driver_handles.contains_key(&worker_task.id) == false && worker_task.task.active == true {
                self.add_worker_task(worker_task).await?;
            } else { // We contain key & task is active
                debug!("Doing nothing, as we seem to be running the task already");
            }
        }
        Ok(())
    }

    pub async fn spawn(&mut self, worker_lock: Arc<RwLock<NewWorker>>) -> Result<(), Box<dyn std::error::Error>> {
        let mut interval = tokio::time::interval(WORKER_TASK_REFRESH_INTERVAL);
        loop {
            debug!("Trying to get tasks and update");
            // TODO: Fix this later, unable to send future error
            let url = get_connect_url()?;
            if let Ok(mut client) = OrchestratorClient::connect(url).await {
                // Aquire read lock
                let worker = worker_lock.read().await;

                // Create new filter request
                let worker_clone = worker.clone();
                let filter_worker_task = xpc::FilterWorkerTask {
                    worker_uuid: worker_clone.uuid
                };

                let response = client.get_worker_task(filter_worker_task).await;
                if let Err(e) =  response {
                    error!("Getting worker task failed: {}", e);
                } else {
                    let worker_tasks = response.unwrap().into_inner();
                    if let Err(e) = self.handle_tasks_update(worker_tasks.data).await {
                        error!("Error while handling task updates: {}", e);
                    }
                }
            } else {
                warn!("Failed to get tasks, will try after {:?}", WORKER_TASK_REFRESH_INTERVAL);
            }
            interval.tick().await;
        }
    }
}
