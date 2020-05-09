use std::time::{Duration, UNIX_EPOCH};

use diesel::prelude::*;
use log::{error, debug};
use tonic::{Request, Response, Status, Code};

use crate::db::DbBroker;
use crate::schema::{tasks, corpora, worker_tasks};
use crate::models::{Task, NewTask, Corpus, NewCorpus};
use crate::xpc;
use crate::xpc::orchestrator_server::Orchestrator;
pub use crate::xpc::orchestrator_server::OrchestratorServer as OrchestratorServer;

#[derive(Clone)]
pub struct OrchestratorService {
    db_broker: DbBroker,
}

#[tonic::async_trait]
impl Orchestrator for OrchestratorService {

    // Task related calls
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

    // Corpus related calls
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

    async fn get_corpus(&self, request: Request<xpc::FilterCorpus>) -> Result<Response<xpc::Corpora>, Status> {
        debug!("Filtering and sending corpus");

        let filter_corpus = request.into_inner();
        let conn = self.db_broker.get_conn();
        let created_after = UNIX_EPOCH + Duration::from_secs(filter_corpus.created_after.seconds as u64);

        let mut query = corpora::table
            .filter(
                corpora::label.ilike(filter_corpus.label).and(
                corpora::created_at.gt(created_after))
            ).into_boxed();

        // If worker is asking for corpus, don't return the same corpus already found by it
        if let Some(worker_task_id) = filter_corpus.worker_task_id {
            query = query.filter(corpora::worker_task_id.ne(worker_task_id));
        }

        let corpus_list = query.load::<Corpus>(&conn);

        if let Err(e) = corpus_list {
            error!("Unable to get task: {}", e);
            Err(Status::new(Code::NotFound, ""))
        } else {
            Ok(Response::new(xpc::Corpora { data: corpus_list.unwrap() }))
        }
    }
}

impl OrchestratorService {
    pub fn new(db_broker: DbBroker) -> Self {
        Self { db_broker }
    }
}
