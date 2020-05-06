use ring::digest;
use data_encoding::HEXUPPER;

pub fn checksum(bytes: &Vec<u8>) -> String {
    let actual = digest::digest(&digest::SHA256, bytes);
    HEXUPPER.encode(actual.as_ref())
}
