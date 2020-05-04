use std::sync::Arc;
use std::time::Duration;

use log::{error, debug};
use tokio::{task, time, sync::RwLock};

use crate::models::Worker;
use crate::xpc::collector_client::CollectorClient;
use crate::xpc::HeartbeatRequest;

// Heartbeat interval in seconds
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

pub async fn periodic_heartbeat(address: Arc<String>, worker_lock: Arc<RwLock<Worker>>) -> Result<(), Box<dyn std::error::Error>> {
    task::spawn(async move {
        let mut interval = time::interval(HEARTBEAT_INTERVAL);

        loop {
            if let Ok(mut client) = CollectorClient::connect(address.as_str()).await {
                let worker = worker_lock.read().await;
                let request = tonic::Request::new(HeartbeatRequest {
                    worker_id: worker.id.to_string(),
                    cpus: worker.cpus,
                    name: worker.name.clone().unwrap_or_default()
                });

                if let Err(e) = client.heartbeat(request).await {
                    error!("Sending heartbeat failed: {}", e);
                }
            }
            interval.tick().await;
        }

    });

    Ok(())
}

