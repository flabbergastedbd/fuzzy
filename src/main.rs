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

/*
fn setup_logging(verbose: u64, file_path: &str) -> Result<(), Box<dyn Error>> {
    let mut config = fern::Dispatch::new();

    config = match verbose {
        1 => config.level(LevelFilter::Info).level_for("fuzzy", LevelFilter::Debug),
        2 => config.level(LevelFilter::Debug),
        3 => config.level(LevelFilter::Info).level_for("fuzzy", LevelFilter::Trace),
        _ => config.level(LevelFilter::Info),
    };

    // Colors first
    let colors = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        .info(Color::White)
        .debug(Color::White)
        .trace(Color::BrightBlack);

    let file_config = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{color_line}[{date}][{target}][{level}{color_line}] {message}\x1B[0m",
                color_line = format_args!("\x1B[{}m", colors.get_color(&record.level()).to_fg_str()),
                date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                target = record.target(),
                level = colors.color(record.level()),
                message = message,
            ));
        })
        .chain(fern::log_file(file_path)?);

    // Stdout config
    let stdout_config = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{color_line}[{date}][{target}][{level}{color_line}] {message}\x1B[0m",
                color_line = format_args!("\x1B[{}m", colors.get_color(&record.level()).to_fg_str()),
                date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                target = record.target(),
                level = colors.color(record.level()),
                message = message,
            ));
        })
        .chain(std::io::stdout());

    config.chain(file_config).chain(stdout_config).apply()?;

    // Logging to log file.
    debug!("Log initialization complete");

    Ok(())
}
*/

fn setup_logging(verbose: u64) -> Result<(), Box<dyn Error>> {
    let level = match verbose {
        1 => Level::INFO,
        2 => Level::DEBUG,
        3 => Level::TRACE,
        _ => Level::WARN,
    };

    let subscriber = tracing_subscriber::fmt()
        .with_target(true)
        .with_max_level(level)
        .finish();

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
