use data_encoding::HEXLOWER;
use ring::digest;

pub mod fs;

pub fn checksum(bytes: &Vec<u8>) -> String {
    let actual = digest::digest(&digest::SHA256, bytes);
    HEXLOWER.encode(actual.as_ref())
}
