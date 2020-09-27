use std::error::Error;

use tokio::sync::mpsc::Sender;
use tracing_subscriber::{
    self,
    fmt,
    registry::Registry,
    layer::SubscriberExt,
    EnvFilter
};

use crate::models;

mod network_layer;

pub enum TraceEvent {
    NewEvent(models::NewTraceEvent)
}

pub struct Tracer {
    verbose_n: u64
}

/// Heavy code dedup, fix this
impl Tracer {
    pub fn new(verbose_n: u64) -> Self {
        Self { verbose_n }
    }

    pub fn set_global_with_network_layer(self, tx: Sender<TraceEvent>) -> Result<(), Box<dyn Error>> {
        let fmt_layer = fmt::layer()
            .with_target(true);

        let env_filter = EnvFilter::from_default_env()
            .add_directive(match self.verbose_n {
                1 => "fuzzy=info",
                2 => "fuzzy=debug",
                3 => "fuzzy=trace",
                _ => "fuzzy=warn",
            }.parse()?);

        let log_layer = network_layer::NetworkLoggingLayer::new(tx);

        let subscriber = Registry::default()
            .with(env_filter)
            .with(log_layer)
            .with(fmt_layer);

        tracing::subscriber::set_global_default(subscriber)?;
        Ok(())
    }

    pub fn set_global(self) -> Result<(), Box<dyn Error>> {
        let fmt_layer = fmt::layer()
            .with_target(true);

        let env_filter = EnvFilter::from_default_env()
            .add_directive(match self.verbose_n {
                1 => "fuzzy=info",
                2 => "fuzzy=debug",
                3 => "fuzzy=trace",
                _ => "fuzzy=warn",
            }.parse()?);

        let subscriber = Registry::default()
            .with(env_filter)
            .with(fmt_layer);

        tracing::subscriber::set_global_default(subscriber)?;
        Ok(())
    }
}
