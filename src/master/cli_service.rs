use diesel::prelude::*;
use log::{error, debug};
use tonic::{Request, Response, Status, Code};

use crate::db::DbBroker;
use crate::schema::{tasks, corpora};
use crate::models::{Task, NewTask, Corpus, NewCorpus};
use crate::xpc;
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

        debug!("Inserting new task into database");
        // Get connection from pool (r2d2)
        let conn = self.db_broker.get_conn();
        // Upsert the new agent
        let rows_inserted = diesel::insert_into(tasks::table)
            .values(&new_task)
            .returning(tasks::name)
            .execute(&conn);

        if let Err(e) = rows_inserted {
            error!("Unable to update db due to {}", e);
            Err(Status::new(Code::InvalidArgument, format!("{}", e)))
        } else {
            Ok(Response::new({}))
        }
    }

    async fn get_tasks(&self, _: Request<()>) -> Result<Response<xpc::Tasks>, Status> {
        debug!("Returning all tasks");

        let conn = self.db_broker.get_conn();
        let task_list = tasks::table
            .load::<Task>(&conn);

        if let Err(e) = task_list {
            error!("Unable to get task: {}", e);
            Err(Status::new(Code::NotFound, ""))
        } else {
            Ok(Response::new(xpc::Tasks { data: task_list.unwrap() }))
        }
    }

    async fn submit_corpus(&self, request: Request<NewCorpus>) -> Result<Response<()>, Status> {
        debug!("Received new corpus");

        let new_corpus: NewCorpus = request.into_inner();

        let conn = self.db_broker.get_conn();
        let rows_inserted = diesel::insert_into(corpora::table)
            .values(&new_corpus)
            .returning(corpora::id)
            .execute(&conn);

        if let Err(e) = rows_inserted {
            error!("Unable to update db due to {}", e);
            Err(Status::new(Code::InvalidArgument, format!("{}", e)))
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
