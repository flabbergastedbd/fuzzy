use std::error::Error;

use log::{error, debug};
use clap::ArgMatches;
use prettytable::{Table, Row};

mod formatter;
mod tasks;
mod corpora;
mod profile;

fn print_results<T>(headings: Vec<&str>, entries: Vec<Vec<T>>)
    where T: std::fmt::Display
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
    let mut connect_addr = "http://".to_owned();
    connect_addr.push_str(arg_matches.value_of("connect_addr").unwrap_or("127.0.0.1:12700"));

    // Start matching
    match arg_matches.subcommand() {
        ("tasks", Some(sub_matches)) => {
            debug!("Launched tasks subcommand");
            tasks::cli(sub_matches, connect_addr).await?;
        },
        ("corpora", Some(sub_matches)) => {
            debug!("Launched tasks subcommand");
            corpora::cli(sub_matches, connect_addr).await?;
        },
        ("profile", Some(sub_matches)) => {
            debug!("Launched profile subcommand");
            profile::cli(sub_matches, connect_addr).await?;
        },
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
