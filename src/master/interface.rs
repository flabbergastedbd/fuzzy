use std::time::{Duration, UNIX_EPOCH};

use diesel::prelude::*;
use log::{error, debug};
use tonic::{Request, Response, Status, Code};

use crate::db::DbBroker;
use crate::schema::{workers, tasks, corpora, crashes, worker_tasks, fuzz_stats, sys_stats};
use crate::models::{
    Task, NewTask,
    Corpus, NewCorpus,
    Crash, NewCrash, PatchCrash,
    NewFuzzStat
};
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
    async fn get_tasks(&self, request: Request<xpc::FilterTask>) -> Result<Response<xpc::Tasks>, Status> {
        debug!("Returning filtered tasks");

        let filter_task: xpc::FilterTask = request.into_inner();
        let mut query = tasks::table.into_boxed();

        let conn = self.db_broker.get_conn();

        // Filter by task id
        if let Some(task_id) = filter_task.id {
            query = query.filter(tasks::id.eq(task_id));
        }

        // Filter task active status
        if let Some(active) = filter_task.active {
            query = query.filter(tasks::active.eq(active));
        }

        let task_list = query
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

    async fn update_task(&self, request: Request<xpc::PatchTask>) -> Result<Response<()>, Status> {

        // First get inner type of tonic::Request & then use our From traits
        let patch_task: xpc::PatchTask = request.into_inner();

            // .into_boxed();

        if let Some(patch_profile) = patch_task.profile.clone() {
            if let Ok(_) = construct_profile(patch_profile.as_str()) {
                debug!("Valid profile submitted");
            } else {
                return Err(Status::new(Code::InvalidArgument, "Bad profile submitted"))
            }
        }

        // Check profile is valid
        debug!("Updating task into database");
        // Get connection from pool (r2d2)
        let conn = self.db_broker.get_conn();
        // Upsert the new agent
        let tasks = diesel::update(tasks::table)
            .filter(tasks::id.eq(patch_task.id))
            .set(&patch_task)
            .returning(tasks::all_columns)
            .load::<Task>(&conn);

        if let Err(e) = tasks {
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
        debug!("Filtering and sending corpus {:?}", filter_corpus);

        let conn = self.db_broker.get_conn();
        let created_after = UNIX_EPOCH + Duration::from_secs(filter_corpus.created_after.seconds as u64);

        let mut query = corpora::table
            .filter(
                corpora::label.ilike(filter_corpus.label).and(
                corpora::created_at.gt(created_after))
            ).into_boxed();

        // Only one of for_worker_task_id or not_worker_task_id is used. not_ variant is
        // prioritized.
        // If worker is asking for corpus, don't return the same corpus already found by it
        if let Some(worker_task_id) = filter_corpus.not_worker_task_id {
            query = query.filter(corpora::worker_task_id.ne(worker_task_id).or(corpora::worker_task_id.is_null()));
        } else if let Some(worker_task_id) = filter_corpus.for_worker_task_id {
            query = query.filter(corpora::worker_task_id.eq(worker_task_id));
        }

        // If limit is present, sort by latest
        if let Some(limit) = filter_corpus.latest {
            query = query.order(corpora::created_at.desc()).limit(limit);
        }

        let corpus_list = query.load::<Corpus>(&conn);

        if let Err(e) = corpus_list {
            error!("Unable to get corpus: {}", e);
            Err(Status::new(Code::NotFound, ""))
        } else {
            Ok(Response::new(xpc::Corpora { data: corpus_list.unwrap() }))
        }
    }

    async fn delete_corpus(&self, request: Request<xpc::FilterCorpus>) -> Result<Response<()>, Status> {

        let filter_corpus = request.into_inner();
        debug!("Filtering and deleting corpus: {:?}", filter_corpus);

        let conn = self.db_broker.get_conn();
        let created_after = UNIX_EPOCH + Duration::from_secs(filter_corpus.created_after.seconds as u64);

        let query = corpora::table
            .filter(
                corpora::label.ilike(filter_corpus.label).and(
                corpora::created_at.gt(created_after))
            );

        let result = diesel::delete(query).execute(&conn);

        if let Err(e) = result {
            error!("Unable to delete corpus: {}", e);
            Err(Status::new(Code::NotFound, ""))
        } else {
            Ok(Response::new({}))
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

    async fn update_crash(&self, request: Request<PatchCrash>) -> Result<Response<()>, Status> {
        let patch_crash: PatchCrash = request.into_inner();

        let conn = self.db_broker.get_conn();
        let rows_inserted = diesel::update(crashes::table)
            .filter(crashes::id.eq(patch_crash.id))
            .set(&patch_crash)
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

    async fn get_crashes(&self, request: Request<xpc::FilterCrash>) -> Result<Response<xpc::Crashes>, Status> {

        let filter_crash = request.into_inner();
        debug!("Filtering and sending crashes {:?}", filter_crash);

        let conn = self.db_broker.get_conn();
        let created_after = UNIX_EPOCH + Duration::from_secs(filter_crash.created_after.seconds as u64);

        let mut query = crashes::table.inner_join(worker_tasks::table)
            .filter(
                crashes::label.ilike(filter_crash.label).and(
                crashes::created_at.gt(created_after))
            ).into_boxed();

        // Check if verified provided
        if let Some(verified) = filter_crash.verified {
            query = query.filter(crashes::verified.eq(verified));
        }

        // If output filter on it
        if let Some(output) = filter_crash.output {
            query = query.filter(crashes::output.ilike(output));
        }

        // If task id filter on it
        if let Some(task_id) = filter_crash.task_id {
            query = query.filter(
                worker_tasks::task_id.eq(task_id)
                .and(crashes::worker_task_id.eq(worker_tasks::id.nullable()))
            );
        }

        // If limit is present, sort by latest
        if let Some(limit) = filter_crash.latest {
            query = query.order(crashes::created_at.desc()).limit(limit);
        }

        let crash_list = query.select(crashes::all_columns).load::<Crash>(&conn);

        if let Err(e) = crash_list {
            error!("Unable to get corpus: {}", e);
            Err(Status::new(Code::NotFound, ""))
        } else {
            Ok(Response::new(xpc::Crashes { data: crash_list.unwrap() }))
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
                .and(
                    worker_tasks::worker_id.eq(workers::id).and(tasks::active.eq(true)) // Active tasks
                    .or(worker_tasks::id.eq_any(filter_worker_task.worker_task_ids)) // Non active tasks that worker is already running
                )
            )
            .select((worker_tasks::id, tasks::all_columns, worker_tasks::cpus, worker_tasks::active))
            .load::<xpc::WorkerTaskFull>(&conn);

        // Failure of constraint will be logged here
        if let Err(e) = tasks {
            error!("Unable to fetch worker tasks : {}", e);
            Err(Status::new(Code::InvalidArgument, format!("{}", e)))
        } else {
            Ok(Response::new(xpc::WorkerTasks { data: tasks.unwrap() }))
        }
    }

    async fn update_worker_task(&self, request: Request<xpc::PatchWorkerTask>) -> Result<Response<()>, Status> {
        let patch_worker_task = request.into_inner();

        let conn = self.db_broker.get_conn();
        let worker_tasks = diesel::update(worker_tasks::table)
            .filter(worker_tasks::id.eq(patch_worker_task.id))
            .set(&patch_worker_task)
            .execute(&conn);

        if let Err(e) = worker_tasks {
            error!("Unable to fetch worker tasks : {}", e);
            Err(Status::new(Code::InvalidArgument, format!("{}", e)))
        } else {
            Ok(Response::new({}))
        }
    }

    // Fuzz stat related calls
    async fn submit_fuzz_stat(&self, request: Request<NewFuzzStat>) -> Result<Response<()>, Status> {
        let new_fuzz_stat = request.into_inner();

        let conn = self.db_broker.get_conn();

        let rows_inserted = diesel::insert_into(fuzz_stats::table)
            .values(&new_fuzz_stat)
            .execute(&conn);

        if let Err(e) = rows_inserted {
            error!("Unable to add crash : {}", e);
            Err(Status::new(Code::InvalidArgument, format!("{}", e)))
        } else {
            Ok(Response::new({}))
        }
    }

    // Fuzz stat related calls
    async fn submit_sys_stat(&self, request: Request<xpc::NewSysStat>) -> Result<Response<()>, Status> {
        let new_sys_stat = request.into_inner();

        let conn = self.db_broker.get_conn();

        let rows_inserted = diesel::insert_into(sys_stats::table)
            .values(&new_sys_stat)
            .execute(&conn);

        if let Err(e) = rows_inserted {
            error!("Unable to add crash : {}", e);
            Err(Status::new(Code::InvalidArgument, format!("{}", e)))
        } else {
            Ok(Response::new({}))
        }
    }
}

impl OrchestratorService {
    pub fn new(db_broker: DbBroker) -> Self {
        Self { db_broker }
    }
}
