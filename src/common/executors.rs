use std::env;
use std::error::Error;
use std::io::{self, ErrorKind};
use std::path::PathBuf;

use log::{debug, error};

use crate::common::constants::WORKER_CONTAINER_VOLUME_MAP;

// Client pem utils
pub fn set_container_volume_map(volume_map: &str) {
    debug!("Setting container volume map to {}", volume_map);
    env::set_var(WORKER_CONTAINER_VOLUME_MAP, volume_map);
}

pub fn validate_container_volume_map(volume_map: &str) -> Result<(), Box<dyn Error>> {
    let (_, mapped_path) = extract_contraint_volume_map(volume_map);
    if mapped_path.exists() == false || mapped_path.is_dir() == false {
        let err = format!("Mapped path {:?} doesn't seem to exist", mapped_path);
        Err(Box::new(io::Error::new(ErrorKind::InvalidInput, err.as_str())))
    } else {
        Ok(())
    }
}

pub fn extract_contraint_volume_map(volume_map: &str) -> (PathBuf, PathBuf) {
    let map: Vec<&str> = volume_map.splitn(2, ":").collect();
    (PathBuf::from(map[0]), PathBuf::from(map[1]))
}

pub fn get_container_volume_map() -> Result<String, Box<dyn Error>> {
    let volume_map = env::var(WORKER_CONTAINER_VOLUME_MAP);
    if volume_map.is_err() {
        error!("Environment variable {} is not defined", WORKER_CONTAINER_VOLUME_MAP);
    }
    let volume_map = volume_map?.to_owned();
    Ok(volume_map)
}
