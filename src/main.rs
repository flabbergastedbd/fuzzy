use std::env;

use log::debug;
use clap::{App, load_yaml};
use pretty_env_logger;

mod cli;
mod db;
mod xpc;
mod master;
mod worker;
mod utils;
mod common;
mod executor;
mod fuzz_driver;

// TODO https://github.com/diesel-rs/diesel/issues/2155
#[macro_use] extern crate diesel;
pub mod schema;
pub mod models;

fn main() {
    let yaml = load_yaml!("cli.yml");
    let arg_matches = App::from(yaml).get_matches();

    // Enable debug logging as per -vvv
    let verbose_count = arg_matches.occurrences_of("verbose");
    if verbose_count > 1 {
        env::set_var("RUST_LOG", "debug");
    } else if verbose_count == 1 {
        env::set_var("RUST_LOG", "fuzzy=debug");
    } else {
        env::set_var("RUST_LOG", "info");
    }

    // Logger initialization is first
    pretty_env_logger::init();
    debug!("Log initialization complete");


    debug!("Matching subcommand and will launch appropriate main()");
    match arg_matches.subcommand() {
        ("master", Some(sub_matches)) => {
            master::main(sub_matches);
        },
        ("worker", Some(sub_matches)) => {
            worker::main(sub_matches);
        },
        ("cli", Some(sub_matches)) => {
            cli::main(sub_matches);
        },
        _ => {}
    }
}
