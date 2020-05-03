// use diesel::Queryable;

#[derive(Queryable)]
pub struct Worker {
    pub id: uuid::Uuid,
    pub name: Option<String>,
    pub cpus: i32,
    pub active: bool,
    pub created: std::time::SystemTime,
    pub updated: std::time::SystemTime,
}
