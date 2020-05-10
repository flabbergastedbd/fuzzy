use std::time::{Duration, UNIX_EPOCH};

use diesel::prelude::*;
use log::{error, debug};
use tonic::{Request, Response, Status, Code};

use crate::db::DbBroker;
use crate::schema::{workers, tasks, corpora, crashes, worker_tasks};
use crate::models::{Task, NewTask, Corpus, NewCorpus, NewCrash};
use crate::xpc;
use crate::xpc::orchestrator_server::Orchestrator;
use crate::common::profiles::construct_profile;
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

        if let Err(e) = construct_profile(new_task.profile.as_str()) {
            error!("Bad profile: {}", e);
            return Err(Status::new(Code::InvalidArgument, format!("{}", e)))
        }

        // Check profile is valid
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

        let filter_corpus = request.into_inner();
        debug!("Filtering and sending corpus for worker task {:?}", filter_corpus.worker_task_id);

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

    // Crash related calls
    async fn submit_crash(&self, request: Request<NewCrash>) -> Result<Response<()>, Status> {
        debug!("Received new crash");

        let new_crash: NewCrash = request.into_inner();

        let conn = self.db_broker.get_conn();
        let rows_inserted = diesel::insert_into(crashes::table)
            .values(&new_crash)
            .returning(crashes::id)
            .execute(&conn);

        // Failure of constraint will be logged here
        if let Err(e) = rows_inserted {
            error!("Unable to add crash : {}", e);
            Err(Status::new(Code::InvalidArgument, format!("{}", e)))
        } else {
            Ok(Response::new({}))
        }
    }

    // Worker task related calls
    async fn get_worker_task(&self, request: Request<xpc::FilterWorkerTask>) -> Result<Response<xpc::WorkerTasks>, Status> {
        let filter_worker_task = request.into_inner();
        debug!("Filtering worker tasks with {:#?}", filter_worker_task);

        let conn = self.db_broker.get_conn();
        let tasks = worker_tasks::table.inner_join(tasks::table).inner_join(workers::table)
            .filter(
                workers::uuid.eq(filter_worker_task.worker_uuid)
                .and(worker_tasks::worker_id.eq(workers::id))
                // Not active tasks only, as previous ones need to be stopped
                // .and(tasks::active.eq(true)) // Active tasks only
                // .and(worker_tasks::task_id.eq(tasks::id))
            )
            .select((worker_tasks::id, tasks::all_columns, worker_tasks::cpus))
            .load::<xpc::WorkerTaskFull>(&conn);

        // Failure of constraint will be logged here
        if let Err(e) = tasks {
            error!("Unable to fetch worker tasks : {}", e);
            Err(Status::new(Code::InvalidArgument, format!("{}", e)))
        } else {
            Ok(Response::new(xpc::WorkerTasks { data: tasks.unwrap() }))
        }
    }
}

impl OrchestratorService {
    pub fn new(db_broker: DbBroker) -> Self {
        Self { db_broker }
    }
}
