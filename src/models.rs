use diesel::{Queryable, Insertable};
use diesel::query_builder::AsChangeset;
use serde::{Serialize, Deserialize};

use super::schema::{workers, executors, tasks};

#[derive(Clone, Queryable, Insertable, AsChangeset, Serialize, Deserialize)]
#[table_name = "workers"]
pub struct Worker {
    pub uuid: String,
    pub name: Option<String>,
    pub cpus: i32,
    pub active: bool,
}

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
