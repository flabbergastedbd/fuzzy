use std::error::Error;

use log::{error, warn, debug};
use diesel::prelude::*;
use diesel::dsl::sum;

use crate::db::DbBroker;
use crate::models::{Task, Worker};
use crate::schema::{tasks, worker_tasks, workers};
use crate::common::profiles::construct_profile;

#[derive(Clone)]
pub struct Scheduler {
    db_broker: DbBroker,
}

impl Scheduler {
    pub fn new(db_broker: DbBroker) -> Self {
        Self { db_broker }
    }

    fn get_free_worker(&self, cores: i32) -> Result<Option<Worker>, Box<dyn Error>> {
        let conn = self.db_broker.get_conn();

        let active_workers = workers::table.load::<Worker>(&conn)?;

        for worker in active_workers {
            let actual_cpus = worker.cpus;
            let used_cpus: Option<i64> = worker_tasks::table.inner_join(tasks::table).inner_join(workers::table)
            .filter(
                workers::id.eq(worker.id)
                .and(worker_tasks::worker_id.eq(worker.id))
                .and(tasks::active.eq(true))
            )
            .select(sum(worker_tasks::cpus))
            .first(&conn)?;
            let used_cpus = used_cpus.unwrap_or(0) as i32;
            if actual_cpus - used_cpus >= cores {
                return Ok(Some(worker))
            }
        }
        Ok(None)
    }

    fn get_task_allocation_requirement(&self, task: &Task, task_requirement: i32) -> Result<i32, Box<dyn Error>> {
        let conn = self.db_broker.get_conn();

        let allocated: Option<i64> = worker_tasks::table.inner_join(tasks::table)
            .filter(
                worker_tasks::task_id.eq(task.id)
            )
            .select(sum(worker_tasks::cpus))
            .first(&conn)?;
        let allocated = allocated.unwrap_or(0) as i32;
        Ok(task_requirement - allocated)
    }

    fn update_worker_tasks(&self) -> Result<(), Box<dyn Error>> {
        let conn = self.db_broker.get_conn();

        let active_tasks = tasks::table
            .filter(tasks::active.eq(true))
            .load::<Task>(&conn)?;

        for task in active_tasks {
            // Construct profile
            let profile = construct_profile(task.profile.as_str())?;

            // Check if new allocation needs to be done
            let new_requirement = self.get_task_allocation_requirement(&task, profile.execution.cpus)?;
            if new_requirement > 0 {
                let worker = self.get_free_worker(new_requirement)?;
                if worker.is_none() {
                    warn!("Couldn't find a free worker slot for task: {:#?}", task);
                    continue
                }
                let worker = worker.unwrap();
                diesel::insert_into(worker_tasks::table)
                    .values((
                        worker_tasks::task_id.eq(task.id),
                        worker_tasks::worker_id.eq(worker.id),
                        worker_tasks::cpus.eq(new_requirement),
                    ))
                    .execute(&conn)?;

            } else {
                debug!("Task already seems to be fully allocated: {:#?}\nRequirement: {}", task, new_requirement);
            }
        }

        Ok(())
    }

    pub async fn spawn(&self) -> Result<(), Box<dyn Error>> {
        debug!("Spawning scheduler");

        let mut interval = tokio::time::interval(crate::common::intervals::MASTER_SCHEDULER_INTERVAL);
        loop {
            interval.tick().await;
            if let Err(e) = self.update_worker_tasks() {
                error!("Failed to schedule tasks: {}", e);
                warn!("Will try again in {:?}", crate::common::intervals::MASTER_SCHEDULER_INTERVAL);
            }
        }
    }
}
