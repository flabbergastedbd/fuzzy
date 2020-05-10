use std::path::Path;
use std::error::Error;

use crate::fuzz_driver::FuzzConfig;
use crate::utils::fs::read_file;

pub fn construct_profile(json_string: &str) -> Result<FuzzConfig, Box<dyn Error>> {
    let profile: FuzzConfig = serde_json::from_str(json_string)?;
    Ok(profile)
}

pub async fn construct_profile_from_disk(path: &Path) -> Result<FuzzConfig, Box<dyn Error>> {
    let content = read_file(Path::new(path)).await?;
    let content_str = String::from_utf8(content)?;

    let config: FuzzConfig = construct_profile(content_str.as_str())?;
    Ok(config)
}
