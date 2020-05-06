use log::info;

use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};

pub mod enums;

#[derive(Clone)]
pub struct DbBroker {
    pool: Pool<ConnectionManager<PgConnection>>
}

impl DbBroker {
    pub fn new(connection_string: String) -> Self {
        info!("Creating a new db connection");
        let manager = ConnectionManager::<PgConnection>::new(connection_string);
        let pool = Pool::builder()
            .max_size(5)
            .build(manager).expect("Failed to create db connection pool");
        DbBroker { pool }

    }

    pub fn get_conn(&self) -> PooledConnection<ConnectionManager<PgConnection>> {
        self.pool.get().expect("Failed to get connection from pool")
    }
}
