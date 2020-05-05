use std::sync::Arc;
use std::time::Duration;

use log::{debug, error, info};
use tokio::sync::RwLock;

use crate::models::Worker;
use crate::xpc::collector_client::CollectorClient;
use crate::xpc::HeartbeatRequest;

// Heartbeat interval in seconds
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

pub struct Dispatcher {
    connect_addr: String
}

impl Dispatcher {
    pub fn new(connect_addr: String) -> Self {
        Dispatcher { connect_addr }
    }

    pub async fn heartbeat(self, worker_lock: Arc<RwLock<Worker>>) { // -> Result<(), Box<dyn std::error::Error>> {
        let mut interval = tokio::time::interval(HEARTBEAT_INTERVAL);
        loop {
            debug!("Trying to send heartbeat to given address");
            if let Ok(mut client) = CollectorClient::connect(self.connect_addr.clone()).await {
                let worker = worker_lock.read().await;
                let request = tonic::Request::new(HeartbeatRequest {
                    id: worker.id.clone(),
                    cpus: worker.cpus,
                    name: worker.name.clone().unwrap_or_default()
                });

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
