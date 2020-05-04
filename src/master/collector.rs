use log::{debug, info};
use tonic::{Request, Response, Status};

use crate::models;
use crate::db::DbBroker;
use crate::xpc::collector_server::Collector;
use crate::xpc::{HeartbeatRequest, HeartbeatResponse};

#[derive(Clone)]
pub struct CollectorService {
    db_broker: DbBroker,
}

#[tonic::async_trait]
impl Collector for CollectorService {
    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>, // Accept request of type HelloRequest
    ) -> Result<Response<HeartbeatResponse>, Status> {
        debug!("Received heartbeat request");

        // UPSERT worker
        let req = request.into();
        let new_worker = models::Worker::default();
        new_worker.id = req.id;
        new_worker.name =  req.name;
        new_worker.cpus =  req.cpus;
        new_worker.active =  true;

        Ok(Response::new(HeartbeatResponse { status: true }))
    }

}

impl CollectorService {
pub fn new(db_broker: DbBroker) -> Self {
    CollectorService { db_broker }
}
}
