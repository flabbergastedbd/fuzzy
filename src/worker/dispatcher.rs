use std::sync::Arc;

use log::{warn, debug, error, info};
use tokio::sync::RwLock;

use crate::models::NewWorker;
use crate::xpc::collector_client::CollectorClient;
use crate::common::intervals::WORKER_HEARTBEAT_INTERVAL;

// Heartbeat interval in seconds

pub async fn heartbeat(worker_lock: Arc<RwLock<NewWorker>>) -> Result<(), Box<dyn std::error::Error>> {
    let mut interval = tokio::time::interval(WORKER_HEARTBEAT_INTERVAL);
    loop {
        let endpoint = crate::common::xpc::get_server_endpoint().await?;
        let channel_or_error = endpoint.connect().await;
        if let Ok(channel) = channel_or_error {
            let mut client = CollectorClient::new(channel);

            debug!("Trying to send heartbeat to given address");
            // Aquire read lock
            let worker = worker_lock.read().await;
            // Create new request
            let request = tonic::Request::new(worker.clone());
            if let Err(e) = client.heartbeat(request).await {
                error!("Sending heartbeat failed: {}", e);
            } else {
                info!("Heartbeat sending was successful!");
            }
        } else {
            warn!("Failed to send a heartbeat, will try after {:?}: {:?}", WORKER_HEARTBEAT_INTERVAL, channel_or_error);
        }
        interval.tick().await;
    }
}

