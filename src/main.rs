use clap::{load_yaml, App};

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
pub mod trace;

fn main() {
    let yaml = load_yaml!("cli.yml");
    let arg_matches = App::from(yaml).get_matches();

    // Enable debug logging as per -vvv
    let verbose_count = arg_matches.occurrences_of("verbose");

    let tracer = trace::Tracer::new(verbose_count);

    match arg_matches.subcommand() {
        ("master", Some(sub_matches)) => {
            tracer.set_global().expect("Failed to setup logging");
            master::main(sub_matches);
        }
        ("worker", Some(sub_matches)) => {
            worker::main(sub_matches, tracer);
        }
        ("cli", Some(sub_matches)) => {
            tracer.set_global().expect("Failed to setup logging");
            cli::main(sub_matches);
        }
        _ => {}
    }
}
