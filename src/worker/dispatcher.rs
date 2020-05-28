use std::error::Error;
use std::sync::Arc;

use log::{warn, trace, error};
use tokio::sync::RwLock;
use heim::{
    memory,
    cpu,
    units::information
};

use crate::models::{NewWorker, NewSysStat};
use crate::xpc::collector_client::CollectorClient;
use crate::common::intervals::WORKER_HEARTBEAT_INTERVAL;
use crate::common::xpc::get_orchestrator_client;

// Heartbeat interval in seconds

pub async fn heartbeat(worker_lock: Arc<RwLock<NewWorker>>) -> Result<(), Box<dyn std::error::Error>> {
    let mut interval = tokio::time::interval(WORKER_HEARTBEAT_INTERVAL);
    loop {
        let endpoint = crate::common::xpc::get_server_endpoint().await?;
        let channel_or_error = endpoint.connect().await;
        if let Ok(channel) = channel_or_error {
            // Send heartbeat
            let mut client = CollectorClient::new(channel);

            trace!("Trying to send heartbeat to given address");
            // Aquire read lock
            let worker = worker_lock.read().await;
            // Create new request
            let request = tonic::Request::new(worker.clone());
            let response = client.heartbeat(request).await;
            if let Ok(response) = response {
                trace!("Heartbeat sending was successful!");
                // Send stats
                let worker = response.into_inner();
                if let Err(e) = send_sys_stats(worker.id).await {
                    error!("system stat collection failed: {}", e);
                }
            } else {
                error!("Sending heartbeat failed");
            }
        } else {
            warn!("Failed to send a heartbeat, will try after {:?}: {:?}", WORKER_HEARTBEAT_INTERVAL, channel_or_error);
        }
        interval.tick().await;
    }
}

// TODO: Shittiest collection, fix this
pub async fn send_sys_stats(worker_id: i32) -> Result<(), Box<dyn Error>> {
    trace!("Collecting stats");
    let memory = memory::memory().await?;
    let swap = memory::swap().await?;

    let cpu_time = cpu::time().await?;

    let new_stat = NewSysStat {
        cpu_system_time: cpu_time.system().get::<heim::units::time::second>(),
        cpu_user_time: cpu_time.user().get::<heim::units::time::second>(),
        cpu_idle_time: cpu_time.idle().get::<heim::units::time::second>(),

        memory_total: memory.total().get::<information::megabyte>() as i32,
        memory_used: get_used_memory().await?,

        swap_total: swap.total().get::<information::megabyte>() as i32,
        swap_used: swap.used().get::<information::megabyte>() as i32,

        worker_id,
    };

    let mut client = get_orchestrator_client().await?;

    client.submit_sys_stat(tonic::Request::new(new_stat)).await?;

    Ok(())
}

#[cfg(target_os = "linux")]
async fn get_used_memory() -> Result<i32, Box<dyn Error>> {
    use heim::memory::os::linux::MemoryExt;

    let memory = memory::memory().await?;
    Ok(memory.used().get::<information::megabyte>() as i32)
}

#[cfg(not(target_os = "linux"))]
async fn get_used_memory() -> Result<i32, Box<dyn Error>> {
    Ok(0)
}

