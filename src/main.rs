use std::error::Error;

use clap::{load_yaml, App};
use tracing::debug;
use tokio::sync::mpsc::Receiver;
use tracing_subscriber::{
    self,
    fmt,
    registry::Registry,
    layer::SubscriberExt,
    EnvFilter
};

mod cli;
mod common;
mod db;
mod executor;
mod fuzz_driver;
mod master;
mod utils;
mod worker;
mod xpc;

#[macro_use]
extern crate validator_derive;
// TODO https://github.com/diesel-rs/diesel/issues/2155
#[macro_use]
extern crate diesel;
pub mod models;
pub mod schema;

pub enum TraceEvent {
    NewEvent(models::NewTraceEvent)
}

fn setup_logging(verbose: u64) -> Result<Receiver<TraceEvent>, Box<dyn Error>> {
    let fmt_layer = fmt::layer()
        .with_target(true);

    let env_filter = EnvFilter::from_default_env()
        .add_directive(match verbose {
            1 => "fuzzy=info",
            2 => "fuzzy=debug",
            3 => "fuzzy=trace",
            _ => "fuzzy=warn",
        }.parse()?);

    let (tx, rx) = tokio::sync::mpsc::channel::<TraceEvent>(50);
    // Worker related network logging layer
    let worker_network_layer = crate::worker::log_layer::NetworkLoggingLayer::new(tx);

    let subscriber = Registry::default()
        .with(env_filter)
        .with(worker_network_layer)
        .with(fmt_layer);
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(rx)
}

fn main() {
    let yaml = load_yaml!("cli.yml");
    let arg_matches = App::from(yaml).get_matches();

    // Enable debug logging as per -vvv
    let verbose_count = arg_matches.occurrences_of("verbose");
    // let global_span = span!(Level::TRACE, "fuzzy");
    // This guard will only be dropped once application exits
    // let _guard = global_span.enter();

    let trace_rx = setup_logging(verbose_count).expect("Error while setting up logging");

    debug!("Matching subcommand and will launch appropriate main()");
    match arg_matches.subcommand() {
        ("master", Some(sub_matches)) => {
            master::main(sub_matches);
        }
        ("worker", Some(sub_matches)) => {
            worker::main(sub_matches,trace_rx);
        }
        ("cli", Some(sub_matches)) => {
            cli::main(sub_matches);
        }
        _ => {}
    }
}
