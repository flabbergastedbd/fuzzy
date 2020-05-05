use diesel::{Queryable, Insertable};

use super::schema::{executors, tasks};

pub use crate::xpc::Worker;

#[derive(Clone, Queryable, Insertable, AsChangeset)]
#[table_name = "executors"]
pub struct Executor {
    pub name: String,
}

#[derive(Clone, Queryable, AsChangeset)]
#[table_name = "tasks"]
pub struct Task {
    pub id: String,
    pub name: String,
    pub active: bool,
    pub executor_id: i32
}
