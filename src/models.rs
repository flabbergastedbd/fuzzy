use diesel::{Queryable, Insertable};

use super::schema::{tasks, worker_tasks};

pub use crate::xpc::Worker;

#[derive(Clone, Queryable, Insertable, Associations)]
#[table_name = "tasks"]
pub struct Task {
    pub name: String,
    pub active: bool,
    pub executor: String,
    pub fuzz_driver: String
}

#[derive(Clone, Queryable, Insertable, Associations)]
#[table_name = "worker_tasks"]
#[belongs_to(Task)]
#[belongs_to(Worker)]
pub struct WorkerTask {
    pub worker_id: i32,
    pub task_id: i32
}
