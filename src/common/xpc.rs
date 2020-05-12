use std::env;
use std::path::Path;
use std::error::Error;

use log::{error, debug};
use tonic::transport::{ClientTlsConfig, Channel, Endpoint, Identity, Certificate};

use crate::xpc::orchestrator_client::OrchestratorClient;
use crate::xpc::collector_client::CollectorClient;
use crate::common::constants::{
    WORKER_CONNECT_CACERT_ENV_KEY,
    WORKER_CONNECT_WORKERPEM_ENV_KEY,
    WORKER_CONNECT_ADDR_ENV_KEY};
use crate::utils::fs::read_file;

// Client pem utils
pub fn set_worker_pem(path: &str) {
    debug!("Setting worker pem path to {}", path);
    env::set_var(WORKER_CONNECT_WORKERPEM_ENV_KEY, path);
}

pub fn get_worker_pem() -> Result<String, Box<dyn Error>> {
    let path = env::var(WORKER_CONNECT_WORKERPEM_ENV_KEY);
    if path.is_err() {
        error!("Environment variable worker pem not defined");
    }
    let path = path?.to_owned();
    Ok(path)
}

// CA Cert related utils
pub fn set_ca_crt(path: &str) {
    debug!("Setting ca cert path to {}", path);
    env::set_var(WORKER_CONNECT_CACERT_ENV_KEY, path);
}

pub fn get_ca_crt() -> Result<String, Box<dyn Error>> {
    let path = env::var(WORKER_CONNECT_CACERT_ENV_KEY);
    if path.is_err() {
        error!("Environment variable path not defined");
    }
    let path = path?.to_owned();
    Ok(path)
}

// Connect url things
pub fn set_connect_url(url: &str) {
    debug!("Setting url to {}", url);
    env::set_var(WORKER_CONNECT_ADDR_ENV_KEY, url);
}

pub fn get_connect_url() -> Result<String, Box<dyn Error>> {
    let url = env::var(WORKER_CONNECT_ADDR_ENV_KEY);
    if url.is_err() {
        error!("Environment variable url not defined");
    }
    let url = url?.to_owned();
    Ok(url)
}

pub async fn get_server_endpoint() -> Result<Endpoint, Box<dyn Error>> {
    let url = get_connect_url()?;
    let ca_cert_path = get_ca_crt()?;
    let worker_pem_path = get_worker_pem()?;

    let ca_cert_bytes = read_file(Path::new(ca_cert_path.as_str())).await?;
    let worker_pem_bytes = read_file(Path::new(worker_pem_path.as_str())).await?;

    let tls_config = ClientTlsConfig::new()
        .ca_certificate(Certificate::from_pem(ca_cert_bytes))
        .identity(Identity::from_pem(&worker_pem_bytes, &worker_pem_bytes));

    let endpoint = tonic::transport::Channel::from_shared(url)?
        .tls_config(tls_config);

    Ok(endpoint)
}

pub async fn get_orchestrator_client() -> Result<OrchestratorClient<Channel>, Box<dyn Error>> {
    let endpoint = get_server_endpoint().await?;
    let channel = endpoint.connect().await?;
    let client = OrchestratorClient::new(channel);
    Ok(client)
}
