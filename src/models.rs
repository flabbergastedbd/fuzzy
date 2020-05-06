use diesel::prelude::*;
use std::time::SystemTime;
use diesel::{Queryable, Insertable};

use super::schema::worker_tasks;
pub use crate::xpc::{NewWorker, Worker, NewTask, Task};

#[derive(Clone, Queryable, Insertable, Associations)]
#[table_name = "worker_tasks"]
#[belongs_to(Task)]
#[belongs_to(Worker)]
pub struct WorkerTask {
    pub worker_id: i32,
    pub task_id: i32
}

// Models and traits related to tasks
#[derive(Clone, Queryable)]
pub struct DieselTaskModel {
    pub id: i32,
    pub name: String,
    pub active: bool,
    pub executor: Option<String>,
    pub fuzz_driver: Option<String>,
    pub created_at: SystemTime,
    pub updated_at: Option<SystemTime>
}

impl From<DieselTaskModel> for Task {
    fn from(ft: DieselTaskModel) -> Self {
        Self {
            id          : ft.id,
            name        : ft.name,
            active      : ft.active,
            executor    : ft.executor,
            fuzz_driver : ft.fuzz_driver,
            created_at  : prost_types::Timestamp::from(ft.created_at),
            updated_at  : match ft.updated_at {
                Some(t) => Some(prost_types::Timestamp::from(t)),
                None => None
            }
        }
    }
}
