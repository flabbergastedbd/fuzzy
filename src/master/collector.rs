use diesel::prelude::*;
use log::{error, debug};
use tonic::{Request, Response, Status};

use crate::schema;
use crate::models;
use crate::db::DbBroker;
use crate::xpc::collector_server::Collector;
use crate::xpc::HeartbeatResponse;

#[derive(Clone)]
pub struct CollectorService {
    db_broker: DbBroker,
}

#[tonic::async_trait]
impl Collector for CollectorService {
    async fn heartbeat(
        &self,
        request: Request<models::Worker>, // Accept request of type HelloRequest
    ) -> Result<Response<HeartbeatResponse>, Status> {

        // First get inner type of tonic::Request & then use our From traits
        let new_worker: models::Worker = request.into_inner();
        debug!("Received a heartbeat request from {}", new_worker.uuid);

        debug!("Inserting agent into database");
        // Get connection from pool (r2d2)
        let conn = self.db_broker.get_conn();
        // Upsert the new agent
        let rows_inserted = diesel::insert_into(schema::workers::table)
            .values(&new_worker)
            .on_conflict(schema::workers::uuid)
            .do_update()
            .set(&new_worker)
            .execute(&conn);

        if let Err(e) = rows_inserted {
            error!("Unable to update db due to {}", e);
        }

        Ok(Response::new(HeartbeatResponse { status: true }))
    }

}

impl CollectorService {
    pub fn new(db_broker: DbBroker) -> Self {
        CollectorService { db_broker }
    }
}
