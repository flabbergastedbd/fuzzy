use std::error::Error;

use clap::ArgMatches;
use tracing::{debug, error};
use prettytable::{Row, Table};

use crate::common::cli::parse_global_settings;

mod corpora;
mod crashes;
mod formatter;
mod profile;
mod tasks;

fn print_results<T>(headings: Vec<&str>, entries: Vec<Vec<T>>)
where
    T: std::fmt::Display,
{
    let mut table = Table::new();
    table.add_row(Row::from(headings));

    for r in entries.iter() {
        table.add_row(Row::from(r));
    }

    table.printstd();
}

#[tokio::main]
async fn main_loop(arg_matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    // Create url for server & create a client
    parse_global_settings(arg_matches);

    // Start matching
    match arg_matches.subcommand() {
        ("tasks", Some(sub_matches)) => {
            tasks::cli(sub_matches).await?;
        }
        ("corpora", Some(sub_matches)) => {
            corpora::cli(sub_matches).await?;
        }
        ("crashes", Some(sub_matches)) => {
            crashes::cli(sub_matches).await?;
        }
        ("profile", Some(sub_matches)) => {
            profile::cli(sub_matches).await?;
        }
        _ => {}
    }
    Ok(())
}

pub fn main(args: &ArgMatches) {
    debug!("Cli launched");
    // All errors are propagated up till here
    if let Err(e) = main_loop(args) {
        error!("Error encountered: {}", e);
        std::process::exit(1)
    }
}
