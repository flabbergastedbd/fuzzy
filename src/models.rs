use diesel::{Queryable, Insertable};

use super::schema::worker_tasks;
pub use crate::xpc::{
    NewWorker, Worker,
    NewTask, Task,
    NewCorpus, Corpus,
};

#[derive(Clone, Queryable, Insertable, Associations)]
#[table_name = "worker_tasks"]
#[belongs_to(Task)]
#[belongs_to(Worker)]
pub struct WorkerTask {
    pub worker_id: i32,
    pub task_id: i32
}
