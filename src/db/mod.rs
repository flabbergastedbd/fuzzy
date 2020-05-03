use std::sync::Arc;
use log::{debug, info, error};
use tokio_postgres::{Client, NoTls, Error};

#[derive(Debug, Clone)]
pub struct DbBroker {
    pub client: Arc<Client>
}

impl DbBroker {
    pub async fn new() -> Result<Arc<Self>, Error> {
        info!("Creating a new connection");
        let (client, connection) =
            tokio_postgres::connect("postgres://fuzzy:fuzzy@127.0.0.1:5432/fuzzy", NoTls).await?;

        // The connection object performs the actual communication with the database,
        // so spawn it off to run on its own.
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("connection error: {}", e);
            } else {
                debug!("Connection successful");
            }
        });

        debug!("Initializing database if not already done");
        client.batch_execute(include_str!("setup.sql")).await?;

        Ok(Arc::new(DbBroker {
            client: Arc::new(client)
        }))
    }
}
