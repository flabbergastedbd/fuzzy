use log::debug;
use clap::{App, load_yaml};
use pretty_env_logger;

mod db;
mod xpc;
mod master;
mod worker;

// TODO://github.com/diesel-rs/diesel/issues/2155
#[macro_use] extern crate diesel;
pub mod schema;
pub mod models;

fn main() {
    // Logger initialization is first
    pretty_env_logger::init();

    let yaml = load_yaml!("cli.yml");
    let arg_matches = App::from(yaml).get_matches();

    debug!("Matching subcommand and will launch appropriate main()");
    match arg_matches.subcommand() {
        ("master", Some(sub_matches)) => {
            master::main(sub_matches);
        },
        ("worker", Some(sub_matches)) => {
            worker::main(sub_matches);
        },
        _ => {}
    }
}
