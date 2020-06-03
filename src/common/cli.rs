use std::env;

use log::error;
use clap::ArgMatches;

use log::debug;
use crate::common::executors::{validate_container_volume_map, set_container_volume_map};
use crate::common::xpc::{
    set_connect_url,
    set_ca_crt,
    set_worker_pem
};
use crate::common::constants::{
    FUZZY_CONNECT_URL,
    FUZZY_CA_CERT,
    FUZZY_CLIENT_PEM,
};

pub fn parse_volume_map_settings(sub_matches: &ArgMatches) {
    // Set up volume map after verifying
    if let Some(container_volume) = sub_matches.value_of("container_volume") {
        if let Err(e) = validate_container_volume_map(container_volume) {
            error!("Invalid volume map provided: {}", e);
            panic!("Exiting");
        } else {
            set_container_volume_map(container_volume);
        }
    } else {
        error!("Volume map is not provided");
        panic!("Exiting");
    }
}

// In order of provided args, environment variable & default
fn get_arg(sub_matches: &ArgMatches, arg_name: &str, env_key: &str, default: &str) -> String {
    let value: String;
    if let Some(argument) = sub_matches.value_of(arg_name) {
        value = argument.to_owned();
    } else {
        debug!("Getting {} from environment variable: {}", arg_name, env_key);
        value = match env::var(env_key) {
            Ok(value) => value,
            Err(_) => default.to_owned(),
        };
    }
    value
}

pub fn parse_global_settings(sub_matches: &ArgMatches) {
    // Set up connect addr environment variable
    let connect_addr = get_arg(sub_matches, "connect_addr", FUZZY_CONNECT_URL, "https://localhost:12700/");
    set_connect_url(&connect_addr);

    // Set up connect addr environment variable
    let ca_cert_path = get_arg(sub_matches, "ca", FUZZY_CA_CERT, "ca.crt");
    set_ca_crt(&ca_cert_path);

    // Set up connect addr environment variable
    let worker_pem_path = get_arg(sub_matches, "worker_pem", FUZZY_CLIENT_PEM, "worker.pem");
    set_worker_pem(&worker_pem_path);
}
