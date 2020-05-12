use std::env;
use std::error::Error;

use tonic::transport::Channel;

use crate::xpc::orchestrator_client::OrchestratorClient;
use crate::xpc::collector_client::CollectorClient;
use crate::common::constants::WORKER_CONNECT_ADDR_ENV_KEY;

pub fn get_connect_url() -> Result<String, Box<dyn Error>> {
    let url = env::var(WORKER_CONNECT_ADDR_ENV_KEY)?;
    let url = url.to_owned();
    Ok(url)
}

pub async fn get_orchestrator_client() -> Result<OrchestratorClient<Channel>, Box<dyn Error>> {
    let url = get_connect_url()?;
    let client = OrchestratorClient::connect(url).await?;
    Ok(client)
}

pub async fn get_collector_client() -> Result<CollectorClient<Channel>, Box<dyn Error>> {
    let url = get_connect_url()?;
    let client = CollectorClient::connect(url).await?;
    Ok(client)
}
