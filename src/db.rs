use std::sync::Arc;
use log::{debug, info, error};

use diesel::prelude::*;
use diesel::pg::PgConnection;

#[derive(Debug, Clone)]
pub struct DbBroker {
}

impl DbBroker {
    pub fn new() {
        info!("Creating a new connection");
        PgConnection::establish("postgres://fuzzy:fuzzy@127.0.0.1:5432/fuzzy");
    }
}
