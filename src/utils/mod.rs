use chrono::{offset::Local, DateTime};
use data_encoding::HEXLOWER;
use tracing::{info, warn};
use ring::digest;
use std::time::SystemTime;

pub mod fs;

pub fn checksum(bytes: &Vec<u8>) -> String {
    let actual = digest::digest(&digest::SHA256, bytes);
    HEXLOWER.encode(actual.as_ref())
}

pub fn get_human_dt(time: SystemTime) -> String {
    let datetime: DateTime<Local> = DateTime::from(time);
    format!("{}", datetime)
}

pub fn err_output(output: std::process::Output) {
    if output.status.success() == false {
        info!("Stdout: {:?}", String::from_utf8(output.stdout.clone()));
        warn!("Stderr: {:?}", String::from_utf8(output.stderr.clone()));
    }
}
