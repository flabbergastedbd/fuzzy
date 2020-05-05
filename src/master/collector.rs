use diesel::prelude::*;
use log::{error, debug, info};
use futures::future::{self, Ready};
use tarpc::context;

use crate::schema;
use crate::models;
use crate::db::DbBroker;
use crate::xpc::Collector;

#[derive(Clone)]
pub struct CollectorService {
    db_broker: DbBroker,
}

impl Collector for CollectorService {
    type HeartbeatFut = Ready<bool>;

    fn heartbeat(self, _: context::Context, new_worker: models::Worker) -> Self::HeartbeatFut {

        // First get inner type of tonic::Request & then use our From traits
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

        future::ready(true)
    }
}

impl CollectorService {
    pub fn new(db_broker: DbBroker) -> Self {
        CollectorService { db_broker }
    }
}
