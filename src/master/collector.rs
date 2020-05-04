use diesel::prelude::*;
use log::{error, debug, info};
use tonic::{Request, Response, Status};

use crate::schema;
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
        let req = request.into_inner();
        let new_worker = models::Worker {
            id: uuid::Uuid::parse_str(req.worker_id.as_str()).expect("Unable to parse uuid"),
            name: Some(req.name),
            cpus: req.cpus as i32,
            active: true,
        };

        info!("Inserting new agent into database");
        let conn = self.db_broker.get_conn();
        let rows_inserted = diesel::insert_into(schema::workers::table)
            .values(&new_worker)
            .on_conflict(schema::workers::id)
            .do_update()
            .set(&new_worker)
            .execute(&conn);

        if let Err(e) = rows_inserted {
            error!("{}", e);
        }

        Ok(Response::new(HeartbeatResponse { status: true }))
    }

}

impl CollectorService {
    pub fn new(db_broker: DbBroker) -> Self {
        CollectorService { db_broker }
    }
}
