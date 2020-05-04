use diesel::Queryable;
use std::time::SystemTime;

#[derive(Queryable)]
pub struct Worker {
    pub id: uuid::Uuid,
    pub name: Option<String>,
    pub cpus: i32,
    pub active: bool,
    pub created: Option<SystemTime>,
    pub updated: Option<SystemTime>,
}
