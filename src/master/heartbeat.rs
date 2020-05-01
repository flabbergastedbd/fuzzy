use std::net::SocketAddr;
use log::{info, debug};
use tonic::{Request, Response, Status};

// Protobuf
use comms::master_server::Master;
use comms::{PingRequest, PingResponse};

pub mod comms {
    tonic::include_proto!("comms"); // The string specified here must match the proto package name
}
// Protobuf

#[derive(Debug, Clone, Copy)]
pub struct CommsService {
    listen_addr: SocketAddr,
}

#[tonic::async_trait]
impl Master for CommsService {
    async fn ping(
        &self,
        request: Request<PingRequest>,
    ) -> Result<Response<PingResponse>, Status> {
        debug!("Recieved a new pulse");

        info!("Got a new pulse from {:?}", request);

        Ok(Response::new(PingResponse { status: true }))
    }
}

impl CommsService {
    pub fn new(addr: SocketAddr) -> CommsService {
        CommsService { listen_addr: addr }
    }

    pub fn get_listen_addr(&self) -> SocketAddr {
        self.listen_addr
    }
}

