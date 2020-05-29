use std::sync::Arc;
use std::error::Error;
use std::collections::HashMap;

use log::{warn, trace, debug, error, info};
use tokio::{sync::RwLock, sync::oneshot::{self, error::TryRecvError}, task::JoinHandle};

use crate::xpc;
use crate::worker::NewWorker;
use crate::fuzz_driver::{self, FuzzConfig};
use crate::xpc::orchestrator_client::OrchestratorClient;
use crate::common::intervals::WORKER_TASK_REFRESH_INTERVAL;

struct TaskManagerTask {
    task_updated_at: prost_types::Timestamp,
    driver_handle: JoinHandle<()>,
    kill_switch: oneshot::Sender<u8>,
    dead_switch: oneshot::Receiver<u8>,
}

pub struct TaskManager {
    tasks: HashMap<i32, TaskManagerTask>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
        }
    }

    async fn remove_worker_task(&mut self, worker_task_id: &i32) -> Result<(), Box<dyn Error>> {
        debug!("Removing worker task: {:?}", worker_task_id);
        let wtask = self.tasks.remove(worker_task_id).unwrap();

        debug!("Sending kill command");
        let _ = wtask.kill_switch.send(0u8);
        debug!("Joining driver task");
        wtask.driver_handle.await?;
        Ok(())
    }

    async fn check_worker_task(&mut self, worker_task_id: &i32) -> Result<(), Box<dyn Error>> {
        debug!("Checking worker task: {:?}", worker_task_id);
        if let Some(wtask) = self.tasks.get_mut(worker_task_id) {
            match wtask.dead_switch.try_recv() {
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
        let (dead_tx, dead_rx) = oneshot::channel::<u8>();
        let mut driver = fuzz_driver::new(profile, Some(wtask.id));

        info!("Spawning new task: {:#?}", wtask);

        let driver_handle = tokio::spawn(async move {
            if let Err(e) = driver.start(rx, dead_tx).await {
                error!("Driver exited with error: {}", e);
            }
        });

        self.tasks.insert(wtask.id, TaskManagerTask {
            driver_handle,
            kill_switch: tx,
            dead_switch: dead_rx,
            task_updated_at: wtask.task.updated_at,
        });

        Ok(())
    }

    /// Order has to be kept intact
    /// 1. Remove if we are running an older version or an inactive task.
    /// 2. Start an active task if we are not running. (Stale tasks are handled above)
    async fn handle_tasks_update(&mut self, worker_tasks: Vec<xpc::WorkerTaskFull>) -> Result<(), Box<dyn Error>> {
        trace!("Handling task updates, iterating over {} tasks", worker_tasks.len());
        for worker_task in worker_tasks.into_iter() {
            let global_task_active = worker_task.active;

            debug!("Looping on task: {:#?}", worker_task);

            // Delete any inactive or stale tasks
            if self.tasks.contains_key(&worker_task.id) == true && global_task_active == false {
                // Stop this as this shouldn't be active
                self.remove_worker_task(&worker_task.id).await?;
            }

            if self.tasks.contains_key(&worker_task.id) == true && global_task_active == true {
                if let Some(tm_task) = self.tasks.get(&worker_task.id) {
                    if tm_task.task_updated_at != worker_task.task.updated_at {
                        debug!("Older version of task seems to be running, so removing");
                        self.remove_worker_task(&worker_task.id).await?;
                    } else {
                        // Check if we are still running it
                        self.check_worker_task(&worker_task.id).await?;
                    }
                }
            }

            if self.tasks.contains_key(&worker_task.id) == false && global_task_active == true {
                // Start this as we should be running it
                self.add_worker_task(worker_task).await?;
            }
        }
        Ok(())
    }

    pub async fn spawn(&mut self, worker_lock: Arc<RwLock<NewWorker>>) -> Result<(), Box<dyn std::error::Error>> {
        let mut interval = tokio::time::interval(WORKER_TASK_REFRESH_INTERVAL);
        loop {
            trace!("Trying to get tasks and update");
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
                    worker_task_ids: self.tasks.keys().cloned().collect(),
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
