use std::error::Error;

use clap::{load_yaml, App};
use tracing::{span, debug, Level};
use tracing_subscriber;

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

fn setup_logging(verbose: u64) -> Result<(), Box<dyn Error>> {
    let subscriber_builder = tracing_subscriber::fmt()
        .with_target(true);

    let builder = match verbose {
        1 => {
            subscriber_builder.with_env_filter("fuzzy=info")
        },
        2 => {
            subscriber_builder.with_env_filter("fuzzy=debug")
        },
        3 => {
            subscriber_builder.with_env_filter("fuzzy=trace")
        },
        _ => {
            subscriber_builder.with_env_filter("fuzzy=warn")
        }
    };
    let subscriber = builder.finish();

    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

fn main() {
    let yaml = load_yaml!("cli.yml");
    let arg_matches = App::from(yaml).get_matches();

    // Enable debug logging as per -vvv
    let verbose_count = arg_matches.occurrences_of("verbose");
    // let logfile_path = arg_matches.value_of("logfile").unwrap_or("fuzzy.log");
    let global_span = span!(Level::TRACE, "fuzzy");
    // This guard will only be dropped once application exits
    let _guard = global_span.enter();

    if let Err(e) = setup_logging(verbose_count) {
        panic!("Error while setting up logging: {}", e);
    }
    /*
    if let Err(e) = setup_logging(verbose_count, logfile_path) {
        panic!("Error while setting up logging: {}", e);
    }
    */

    debug!("Matching subcommand and will launch appropriate main()");
    match arg_matches.subcommand() {
        ("master", Some(sub_matches)) => {
            master::main(sub_matches);
        }
        ("worker", Some(sub_matches)) => {
            worker::main(sub_matches);
        }
        ("cli", Some(sub_matches)) => {
            cli::main(sub_matches);
        }
        _ => {}
    }
}
