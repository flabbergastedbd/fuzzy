use clap::ArgMatches;
use log::{info, debug};

pub fn main(arg_matches: &ArgMatches) {
    debug!("Master main function launched");

    match arg_matches.subcommand() {
        ("start", Some(_)) => {
            info!("Starting master agent");
        },
        _ => {}
    }
}
