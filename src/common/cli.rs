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
use crate::common::constants::FUZZY_CONNECT_URL;

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

fn parse_connect_url(sub_matches: &ArgMatches) -> String {
    let addr: String;
    if let Some(connect_addr) = sub_matches.value_of("connect_addr") {
        addr = connect_addr.to_owned();
    } else {
        debug!("Getting connect url from environment variable: {}", FUZZY_CONNECT_URL);
        addr = match env::var(FUZZY_CONNECT_URL) {
            Ok(connect_addr) => connect_addr,
            Err(_) => "https://localhost:12700/".to_owned(),
        };
    }
    addr
}

pub fn parse_global_settings(sub_matches: &ArgMatches) {
    // Set up connect addr environment variable
    let connect_addr = parse_connect_url(sub_matches);
    set_connect_url(&connect_addr);

    // Set up connect addr environment variable
    let ca_cert_path = sub_matches.value_of("ca").unwrap_or("ca.crt");
    set_ca_crt(ca_cert_path);

    // Set up connect addr environment variable
    let worker_pem_path = sub_matches.value_of("worker_pem").unwrap_or("worker.pem");
    set_worker_pem(worker_pem_path);
}
