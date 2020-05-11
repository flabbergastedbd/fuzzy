use std::sync::Arc;

use log::{debug, error, info};
use tokio::sync::RwLock;

use crate::models::NewWorker;
use crate::xpc::collector_client::CollectorClient;

// Heartbeat interval in seconds

pub struct Dispatcher {
    connect_addr: String
}

impl Dispatcher {
    pub fn new(addr: String) -> Self {
        let mut connect_addr = "http://".to_owned();
        connect_addr.push_str(addr.as_str());
        Dispatcher { connect_addr }
    }

    pub async fn heartbeat(self, worker_lock: Arc<RwLock<NewWorker>>) { // -> Result<(), Box<dyn std::error::Error>> {
        let mut interval = tokio::time::interval(crate::common::intervals::WORKER_HEARTBEAT_INTERVAL);
        loop {
            debug!("Trying to send heartbeat to given address");
            if let Ok(mut client) = CollectorClient::connect(self.connect_addr.clone()).await {
                // Aquire read lock
                let worker = worker_lock.read().await;
                // Create new request
                let request = tonic::Request::new(worker.clone());
                if let Err(e) = client.heartbeat(request).await {
                    error!("Sending heartbeat failed: {}", e);
                } else {
                    info!("Heartbeat sending was successful!");
                }
            }
            interval.tick().await;
        }
    }
}
