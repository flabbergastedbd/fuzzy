use std::sync::Arc;
use std::error::Error;
use std::collections::HashMap;

use log::{warn, debug, error, info};
use tokio::{sync::RwLock, sync::oneshot::{self, error::TryRecvError}, task::JoinHandle};

use crate::xpc;
use crate::worker::NewWorker;
use crate::fuzz_driver::{self, FuzzConfig};
use crate::xpc::orchestrator_client::OrchestratorClient;
use crate::common::intervals::WORKER_TASK_REFRESH_INTERVAL;

pub struct TaskManager {
    driver_handles: HashMap<i32, JoinHandle<()>>,
    kill_switches: HashMap<i32, oneshot::Sender<u8>>,
    death_switches: HashMap<i32, oneshot::Receiver<u8>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            driver_handles: HashMap::new(),
            kill_switches: HashMap::new(),
            death_switches: HashMap::new(),
        }
    }

    async fn remove_worker_task(&mut self, worker_task_id: &i32) -> Result<(), Box<dyn Error>> {
        debug!("Removing worker task: {:?}", worker_task_id);
        let driver_handle = self.driver_handles.remove(worker_task_id).unwrap();
        let kill_switch = self.kill_switches.remove(worker_task_id).unwrap();
        // Don't unwrap as we can remove this in check_worker_task
        let _ = self.death_switches.remove(worker_task_id);

        debug!("Sending kill command");
        let _ = kill_switch.send(0u8);
        debug!("Joining driver task");
        driver_handle.await?;
        Ok(())
    }

    async fn check_worker_task(&mut self, worker_task_id: &i32) -> Result<(), Box<dyn Error>> {
        debug!("Checking worker task: {:?}", worker_task_id);
        if let Some(dead_switch) = self.death_switches.get_mut(worker_task_id) {
            match dead_switch.try_recv() {
                Err(TryRecvError::Empty) => {
                    debug!("Doing nothing as worker task seems active");
                }
                _ => {
                    debug!("Recv failed with success or other end closed which means fuzz driver exited as reference dropped");
                    self.remove_worker_task(worker_task_id).await?;
                }
            }
        }
        Ok(())
    }

    async fn add_worker_task(&mut self, wtask: xpc::WorkerTaskFull) -> Result<(), Box<dyn Error>> {
        debug!("Adding worker task: {:?}", wtask);
        let profile: FuzzConfig = serde_yaml::from_str(wtask.task.profile.as_str())?;
        let (tx, rx) = oneshot::channel::<u8>();
        let (death_tx, death_rx) = oneshot::channel::<u8>();
        let mut driver = fuzz_driver::new(profile, Some(wtask.id));

        // self.drivers.insert(wtask.id,Box::new(driver));
        self.kill_switches.insert(wtask.id, tx);
        self.death_switches.insert(wtask.id, death_rx);

        info!("Spawning new task: {:#?}", wtask);

        let driver_handle = tokio::spawn(async move {
            if let Err(e) = driver.start(rx, death_tx).await {
                error!("Driver exited with error: {}", e);
            }
        });

        self.driver_handles.insert(wtask.id, driver_handle);

        Ok(())
    }

    async fn handle_tasks_update(&mut self, worker_tasks: Vec<xpc::WorkerTaskFull>) -> Result<(), Box<dyn Error>> {
        debug!("Handling task updates, iterating over {} tasks", worker_tasks.len());
        for worker_task in worker_tasks.into_iter() {
            let local_worker_task_active = self.driver_handles.contains_key(&worker_task.id);
            let global_task_active = worker_task.task.active;

            debug!("Looping on task: {:#?}", worker_task);
            debug!("Is task active already?: {}", local_worker_task_active);

            if local_worker_task_active == true && global_task_active == true {
                // Check if we are still running it
                self.check_worker_task(&worker_task.id).await?;
            } else if local_worker_task_active == false && global_task_active == true {
                // Start this as we should be running it
                self.add_worker_task(worker_task).await?;
            } else if local_worker_task_active == true && global_task_active == false {
                // Stop this as this shouldn't be active
                self.remove_worker_task(&worker_task.id).await?;
            } else {
                // Global task not active, so are we
                debug!("Not doing anything for this task as it is in desired state: {}", local_worker_task_active);
            }
        }
        Ok(())
    }

    pub async fn spawn(&mut self, worker_lock: Arc<RwLock<NewWorker>>) -> Result<(), Box<dyn std::error::Error>> {
        let mut interval = tokio::time::interval(WORKER_TASK_REFRESH_INTERVAL);
        loop {
            debug!("Trying to get tasks and update");
            // TODO: Fix this later, unable to send future error
            let endpoint = crate::common::xpc::get_server_endpoint().await?;
            if let Ok(channel) = endpoint.connect().await {
                let mut client = OrchestratorClient::new(channel);
                // Aquire read lock
                let worker = worker_lock.read().await;

                // Create new filter request
                let worker_clone = worker.clone();
                let filter_worker_task = xpc::FilterWorkerTask {
                    worker_uuid: worker_clone.uuid,
                    worker_task_ids: self.driver_handles.keys().cloned().collect(),
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
