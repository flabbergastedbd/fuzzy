use diesel::{Queryable, Insertable};
use diesel::query_builder::AsChangeset;

use super::schema::workers;

#[derive(Queryable, Insertable, AsChangeset)]
pub struct Worker {
    pub id: uuid::Uuid,
    pub name: Option<String>,
    pub cpus: i32,
    pub active: bool,
}
