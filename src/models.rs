use diesel::{Queryable, Insertable};
use diesel::query_builder::AsChangeset;

use super::schema::workers;

#[derive(Clone, Queryable, Insertable, AsChangeset)]
pub struct Worker {
    pub id: String,
    pub name: Option<String>,
    pub cpus: i32,
    pub active: bool,
}
