use std::error::Error;

use log::error;
use serde::{Serialize, Deserialize};
use tokio::sync::broadcast;
use tonic::Request;

use super::executor::ExecutorConfig;
use crate::models::NewFuzzStat;
use crate::common::xpc::get_orchestrator_client;
use crate::common::intervals::WORKER_FUZZDRIVER_STAT_UPLOAD_INTERVAL;

mod lcov;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum StatCollectorEnum {
    LCov
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FuzzStatConfig {
    pub collector: StatCollectorEnum,
    pub execution: ExecutorConfig,
}

#[tonic::async_trait]
pub trait FuzzStatCollector: Send + Sync {
    async fn start(self: Box<Self>, mut _kill_switch: broadcast::Receiver<u8>) -> Result<(), Box<dyn Error>> {
        self.main_loop().await?;
        /* TODO: Stat collection kill switch disabled as we don't spawn as of now. Should be fine
         * https://users.rust-lang.org/t/explanation-on-fn-self-box-self-for-trait-objects/34024/3
        tokio::select! {
            result = self.main_loop() => {
                if let Err(e) = result {
                    error!("Stat collection exited with error: {}", e);
                }
            },
            _ = kill_switch.recv() => {},
        }
        */

        Ok(())
    }

    async fn main_loop(self: Box<Self>) -> Result<(), Box<dyn Error>> {
        let mut interval = tokio::time::interval(WORKER_FUZZDRIVER_STAT_UPLOAD_INTERVAL);
        let client = &get_orchestrator_client().await?;
        loop {
            interval.tick().await;
            let mut client = client.clone();
            // Iterate over logs and get stats
            let stat: Option<NewFuzzStat> = match self.get_stat().await {
                Ok(stat) => stat,
                Err(e) => {
                    error!("Failed to collect stat: {}", e);
                    None
                },
            };

            if let Some(stat) = stat {
                if let Err(e) = client.submit_fuzz_stat(Request::new(stat)).await {
                    error!("Failed to submit a fuzz stat: {}", e);
                }
            }
        }
    }

    async fn get_stat(&self) -> Result<Option<NewFuzzStat>, Box<dyn Error>>;
}
