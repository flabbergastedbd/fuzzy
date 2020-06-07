use diesel::prelude::*;
use log::{debug, error};
use tonic::{Code, Request, Response, Status};

use crate::db::DbBroker;
use crate::models::{NewWorker, Worker};
use crate::schema::workers;
use crate::xpc::collector_server::Collector;

#[derive(Clone)]
pub struct CollectorService {
    db_broker: DbBroker,
}

#[tonic::async_trait]
impl Collector for CollectorService {
    async fn heartbeat(&self, request: Request<NewWorker>) -> Result<Response<Worker>, Status> {
        // First get inner type of tonic::Request & then use our From traits
        let new_worker: NewWorker = request.into_inner();
        debug!("Received a heartbeat request from {}", new_worker.uuid);

        debug!("Inserting agent into database");
        // Get connection from pool (r2d2)
        let conn = self.db_broker.get_conn();
        // Upsert the new agent
        let worker = diesel::insert_into(workers::table)
            .values(&new_worker)
            .on_conflict(workers::uuid)
            .do_update()
            .set(&new_worker)
            .load::<Worker>(&conn);
        // .execute(&conn);

        if let Err(e) = worker {
            error!("Unable to update db due to {}", e);
            Err(Status::new(Code::Internal, format!("{}", e)))
        } else {
            let worker = worker.unwrap().pop().unwrap();
            Ok(Response::new(worker))
        }
    }
}

impl CollectorService {
    pub fn new(db_broker: DbBroker) -> Self {
        CollectorService { db_broker }
    }
}
