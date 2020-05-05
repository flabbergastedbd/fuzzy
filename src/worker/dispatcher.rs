use std::sync::Arc;
use std::time::Duration;

use log::{debug, error, info};
use tokio::sync::RwLock;
use tarpc::{client, context};
use tokio_serde::formats::Json;

use crate::models::Worker;
use crate::xpc::CollectorClient;

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
            let transport = tarpc::serde_transport::tcp::connect(self.connect_addr.clone(), Json::default()).await
                .expect("Unable to create transport to server");
            if let Ok(mut client) = CollectorClient::new(client::Config::default(), transport).spawn() {
                let worker = worker_lock.read().await;
                if let Err(e) = client.heartbeat(
                        context::current(), worker.clone()).await {

                    error!("Sending heartbeat failed: {}", e);
                } else {
                    info!("Heartbeat sending was successful!");
                }
            }
            interval.tick().await;
        }
    }
}
