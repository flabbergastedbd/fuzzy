use std::error::Error;

use log::{debug, info};
use clap::ArgMatches;
use tonic::{Request, Response, Status, Code};
use prettytable::{Table, Row, Cell};

use crate::xpc::user_interface_client::UserInterfaceClient;
use crate::models::NewTask;

mod formatter;

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

async fn tasks(args: &ArgMatches, connect_addr: String) -> Result<(), Box<dyn Error>> {
    debug!("Creating interface client");
    let mut client = UserInterfaceClient::connect(connect_addr).await?;

    match args.subcommand() {
        // Adding a new task
        ("add", Some(sub_matches)) => {
            debug!("Adding a new task");
            let new_task = NewTask {
                name: sub_matches.value_of("name").unwrap().to_owned(),
                executor: sub_matches.value_of("executor").unwrap().to_owned(),
                fuzz_driver: sub_matches.value_of("fuzz_driver").unwrap().to_owned(),
                active: false,
            };
            let response = client.submit_task(Request::new(new_task)).await?;
            // TODO: Error handling
            println!("{:?}", response);
        },
        // Listing all tasks
        ("list", Some(_)) => {
            debug!("Listing all tasks");

            let response = client.get_tasks(Request::new({})).await?;
            let tasks = response.into_inner().data;

            let tasks_heading = vec!["ID", "Name", "Executor", "Fuzz Driver", "Active"];
            let mut tasks_vec = Vec::new();
            for t in tasks.iter() {
                tasks_vec.push(formatter::format_task(t));
            }

            print_results(tasks_heading, tasks_vec);
        },
        _ => {},
    }

    Ok(())
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
            tasks(sub_matches, connect_addr).await?;
        },
        _ => {}
    }
    Ok(())
}

pub fn main(args: &ArgMatches) {
    debug!("Cli launched");
    // All errors are propagated up till here
    if let Err(e) = main_loop(args) {
        panic!("Error encountered: {}", e);
    }
}
