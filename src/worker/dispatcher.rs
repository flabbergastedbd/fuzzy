use std::error::Error;
use std::io::{self, ErrorKind};

use heim::{cpu, memory, units::information};
use tracing::{error, trace, warn};
use tokio::sync::mpsc::Receiver;

use crate::TraceEvent;
use crate::common::intervals::WORKER_HEARTBEAT_INTERVAL;
use crate::common::xpc::get_orchestrator_client;
use crate::models::{NewSysStat, Worker};

pub async fn heartbeat(worker: Worker, mut tracing_rx: Receiver<TraceEvent>) -> Result<(), Box<dyn std::error::Error>> {
    let worker_id = worker.id;

    let mut interval = tokio::time::interval(WORKER_HEARTBEAT_INTERVAL);

    // We repeatedly iterate over here because we want to keep connecting back with server if we have
    // disconnections in the middle
    loop {
        tokio::select! {
            result = send_sys_stats(worker_id) => {
                if let Err(e) = result {
                    error!("System stat collection failed: {}", e);
                }
            },
            result = send_trace_events(worker_id, &mut tracing_rx) => {
                if let Err(e) = result {
                    warn!("Tracing event sender exited first, probably : {}", e);
                }
            }
        }
        interval.tick().await;
    }
}

async fn send_trace_events(worker_id: i32, tracing_rx: &mut Receiver<TraceEvent>) -> Result<(), Box<dyn Error>> {
    // This loop should exit for entire length of program
    let mut client = get_orchestrator_client().await?;
    while let Some(event) = tracing_rx.recv().await {
        match event {
            TraceEvent::NewEvent(mut e) => {
                // Unable to get worker_id from span data, need to set it here for now
                e.worker_id = worker_id;
                client.submit_trace_event(tonic::Request::new(e)).await?
            }
        };
    }
    Err(Box::new(io::Error::new(ErrorKind::InvalidData, "Trace logging channel closed on sender side")))
}

// TODO: Shittiest collection, fix this
async fn send_sys_stats(worker_id: i32) -> Result<(), Box<dyn Error>> {
    let mut interval = tokio::time::interval(WORKER_HEARTBEAT_INTERVAL);

    loop {
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
        interval.tick().await;
    }
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
