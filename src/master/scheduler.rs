use std::error::Error;
use std::time::{Duration, SystemTime};

use diesel::dsl::sum;
use diesel::prelude::*;
use tracing::{debug, error, trace, warn};

use crate::common::profiles::construct_profile;
use crate::db::DbBroker;
use crate::models::{Task, Worker, WorkerTask};
use crate::schema::{trace_events, sys_stats, tasks, worker_tasks, workers};

#[derive(Clone)]
pub struct Scheduler {
    db_broker: DbBroker,
}

impl Scheduler {
    pub fn new(db_broker: DbBroker) -> Self {
        Self { db_broker }
    }

    fn activate_worker_task(&self, worker_task: &WorkerTask) -> Result<(), Box<dyn Error>> {
        let conn = self.db_broker.get_conn();

        diesel::update(worker_tasks::table.find(worker_task.id))
            .set(worker_tasks::active.eq(true))
            .execute(&conn)?;

        Ok(())
    }

    fn disable_worker_tasks_for_inactive_tasks(&self) -> Result<(), Box<dyn Error>> {
        let conn = self.db_broker.get_conn();

        // Get tasks which are inactive by themselves but have active worker tasks
        let stale_tasks = worker_tasks::table
            .inner_join(tasks::table)
            .filter(tasks::active.eq(false).and(worker_tasks::active.eq(true)))
            .select(tasks::all_columns)
            .load::<Task>(&conn)?;

        // Deactivate worker tasks
        for task in stale_tasks.iter() {
            self.disable_worker_tasks_for_task(&task)?;
        }

        Ok(())
    }

    fn is_worker_active(&self, worker: &Worker) -> Result<bool, Box<dyn Error>> {
        let conn = self.db_broker.get_conn();

        let activity_window = SystemTime::now() - (25 * crate::common::intervals::WORKER_HEARTBEAT_INTERVAL);

        let sys_stats: i64 = sys_stats::table
            .filter(
                sys_stats::worker_id
                    .eq(worker.id)
                    .and(sys_stats::created_at.gt(activity_window)),
            )
            .count()
            .first(&conn)?;

        Ok(sys_stats > 0)
    }

    fn disable_inactive_workers(&self) -> Result<(), Box<dyn Error>> {
        let conn = self.db_broker.get_conn();

        let active_workers = workers::table.filter(workers::active.eq(true)).load::<Worker>(&conn)?;

        for worker in active_workers {
            if self.is_worker_active(&worker)? == false {
                debug!("Worker {} found inactive", worker.id);
                // Remove active on worker
                diesel::update(workers::table)
                    .filter(workers::id.eq(worker.id))
                    .set(workers::active.eq(false))
                    .execute(&conn)?;

                // Deactivate all associated worker tasks
                self.disable_worker_tasks_for_worker(&worker)?;
            }
        }

        Ok(())
    }

    fn disable_worker_tasks_for_task(&self, task: &Task) -> Result<(), Box<dyn Error>> {
        let conn = self.db_broker.get_conn();

        diesel::update(WorkerTask::belonging_to(task))
            .set(worker_tasks::active.eq(false))
            .execute(&conn)?;
        Ok(())
    }

    fn disable_worker_tasks_for_worker(&self, worker: &Worker) -> Result<(), Box<dyn Error>> {
        let conn = self.db_broker.get_conn();

        diesel::update(WorkerTask::belonging_to(worker))
            .set(worker_tasks::active.eq(false))
            .execute(&conn)?;
        Ok(())
    }

    fn get_free_workers(&self) -> Result<Vec<(Worker, i32)>, Box<dyn Error>> {
        let conn = self.db_broker.get_conn();

        let workers = workers::table.filter(workers::active.eq(true)).load::<Worker>(&conn)?;
        let mut workers_free: Vec<(Worker, i32)> = vec![];

        for worker in workers {
            let free = self.get_free_cpus(&worker)?;
            if free > 0 {
                workers_free.push((worker, free))
            }
        }

        Ok(workers_free)
    }

    fn get_free_cpus(&self, worker: &Worker) -> Result<i32, Box<dyn Error>> {
        let conn = self.db_broker.get_conn();

        let allocated: Option<i64> = WorkerTask::belonging_to(worker)
            .filter(worker_tasks::active.eq(true))
            .select(sum(worker_tasks::cpus))
            .first(&conn)?;

        let free = worker.cpus - allocated.unwrap_or(0) as i32;
        Ok(free)
    }

    fn get_new_requirement(&self, task: &Task, task_requirement: i32) -> Result<i32, Box<dyn Error>> {
        let conn = self.db_broker.get_conn();

        let allocated: Option<i64> = WorkerTask::belonging_to(task)
            .filter(
                worker_tasks::active.eq(true), // and that are active
            )
            .select(sum(worker_tasks::cpus))
            .first(&conn)?;
        let allocated = allocated.unwrap_or(0) as i32;
        Ok(task_requirement - allocated)
    }

    fn add_worker_task(&self, task: &Task, worker: &Worker, cpus: i32) -> Result<(), Box<dyn Error>> {
        let conn = self.db_broker.get_conn();

        diesel::insert_into(worker_tasks::table)
            .values((
                worker_tasks::task_id.eq(task.id),
                worker_tasks::worker_id.eq(worker.id),
                worker_tasks::cpus.eq(cpus),
            ))
            .execute(&conn)?;

        Ok(())
    }

    // Ensure that worker task is activatable by following conditions
    //
    // 1. Relevant worker is still active
    // 2. There are free cpus left
    //
    fn get_activatable_worker_task(&self, task: &Task, requirement: i32) -> Result<Option<WorkerTask>, Box<dyn Error>> {
        let conn = self.db_broker.get_conn();

        let worker_tasks = WorkerTask::belonging_to(task)
            .inner_join(workers::table)
            .filter(
                worker_tasks::active
                    .eq(false)
                    .and(workers::active.eq(true))
                    .and(worker_tasks::cpus.eq(requirement)),
            )
            .select((worker_tasks::all_columns, workers::all_columns))
            .load::<(WorkerTask, Worker)>(&conn)?;

        for (worker_task, worker) in worker_tasks {
            if self.get_free_cpus(&worker)? >= requirement {
                return Ok(Some(worker_task));
            }
        }

        Ok(None)
    }

    fn allocate_tasks(&self) -> Result<(), Box<dyn Error>> {
        let conn = self.db_broker.get_conn();

        // Get active tasks, loop over them allocating
        let active_tasks = tasks::table.filter(tasks::active.eq(true)).load::<Task>(&conn)?;

        for task in active_tasks {
            // Construct profile
            let profile = construct_profile(task.profile.as_str())?;

            // 1. Check if new allocation needs to be done
            let new_requirement = self.get_new_requirement(&task, profile.execution.cpus)?;

            // 2. We need to allocate
            if new_requirement > 0 {
                // Always get free workers in loop as allocations might have happened
                let free_workers = self.get_free_workers()?;
                debug!("Got {} free workers", free_workers.len());

                // 3. Check if existing inactive worker task can satisfy this
                if let Some(worker_task) = self.get_activatable_worker_task(&task, new_requirement)? {
                    self.activate_worker_task(&worker_task)?;
                    continue;
                }

                // 4. If not create a new worker task
                let worker = free_workers.iter().find_map(|w_f| {
                    let (worker, free_cpus) = w_f;
                    if free_cpus >= &new_requirement {
                        Some(worker)
                    } else {
                        None
                    }
                });
                if let Some(worker) = worker {
                    self.add_worker_task(&task, &worker, new_requirement)?;
                } else {
                    warn!("Couldn't find free worker for task: {}", task.id);
                }
            } else {
                trace!("Task already seems to be fully allocated: {:#?}\n", task);
            }
        }

        Ok(())
    }

    fn schedule(&self) -> Result<(), Box<dyn Error>> {
        // Disable stale worker tasks first, should free up resources
        self.disable_worker_tasks_for_inactive_tasks()?;

        // Disable inactive workers
        self.disable_inactive_workers()?;

        // Allocate tasks
        self.allocate_tasks()?;

        // Do cleaning at last ot prevent errors in here spoiling scheduling
        self.clean_db()?;

        Ok(())
    }

    /// Use this to clean up any old data, like super old sys stat, error logs data
    fn clean_db(&self) -> Result<(), Box<dyn Error>> {
        let conn = self.db_broker.get_conn();

        let now_7 = SystemTime::now() - Duration::from_secs(7 * 24 * 60 * 60);

        let query = sys_stats::table
            .filter(sys_stats::created_at.le(now_7));
        diesel::delete(query).execute(&conn)?;

        let query = trace_events::table
            .filter(trace_events::created_at.le(now_7));
        diesel::delete(query).execute(&conn)?;
        Ok(())
    }

    pub async fn spawn(&self) -> Result<(), Box<dyn Error>> {
        debug!("Spawning scheduler");

        let mut interval = tokio::time::interval(crate::common::intervals::MASTER_SCHEDULER_INTERVAL);
        loop {
            interval.tick().await;
            if let Err(e) = self.schedule() {
                error!("Failed to schedule tasks: {}", e);
                warn!(
                    "Will try again in {:?}",
                    crate::common::intervals::MASTER_SCHEDULER_INTERVAL
                );
            }
        }
    }
}
