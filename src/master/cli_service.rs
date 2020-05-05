use diesel::prelude::*;
use log::{error, debug};
use tonic::{Request, Response, Status};

use crate::db::DbBroker;
use crate::schema::tasks;
use crate::models::{Task, NewTask};
use crate::xpc::user_interface_server::UserInterface;
pub use crate::xpc::user_interface_server::UserInterfaceServer as CliInterfaceServer;

#[derive(Clone)]
pub struct CliServer {
    db_broker: DbBroker,
}

#[tonic::async_trait]
impl UserInterface for CliServer {
    async fn submit_task(&self, request: Request<NewTask>) -> Result<Response<()>, Status> {

        // First get inner type of tonic::Request & then use our From traits
        let new_task: NewTask = request.into_inner();
        debug!("Received a task request");

        debug!("Inserting agent into database");
        // Get connection from pool (r2d2)
        let conn = self.db_broker.get_conn();
        // Upsert the new agent
        let rows_inserted = diesel::insert_into(tasks::table)
            .values(&new_task)
            .returning(tasks::name)
            .execute(&conn);

        if let Err(e) = rows_inserted {
            error!("Unable to update db due to {}", e);
            Err(Status::new(tonic::Code::InvalidArgument, format!("{}", e)))
        } else {
            Ok(Response::new({}))
        }
    }
}

impl CliServer {
    pub fn new(db_broker: DbBroker) -> Self {
        CliServer { db_broker }
    }
}
