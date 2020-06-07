use std::error::Error;
use std::path::Path;

use validator::{Validate, ValidationError};

use crate::fuzz_driver::FuzzConfig;
use crate::utils::fs::read_file;

pub fn construct_profile(yaml_string: &str) -> Result<FuzzConfig, Box<dyn Error>> {
    let profile: FuzzConfig = serde_yaml::from_str(yaml_string)?;
    profile.validate()?;
    Ok(profile)
}

pub async fn construct_profile_from_disk(path: &Path) -> Result<FuzzConfig, Box<dyn Error>> {
    let content = read_file(Path::new(path)).await?;
    let content_str = String::from_utf8(content)?;

    let config: FuzzConfig = construct_profile(content_str.as_str())?;
    Ok(config)
}

pub fn validate_fuzz_profile(config: &FuzzConfig) -> Result<(), ValidationError> {
    // Because for lcov collector we redownload corpus as of now
    if config.fuzz_stat.is_some() && config.corpus.upload == false {
        return Err(ValidationError::new(
            "Fuzz stat collectors are not supported when corpus upload is disabled.",
        ));
    }
    Ok(())
}

pub fn validate_relative_path(path: &Box<Path>) -> Result<(), ValidationError> {
    if path.is_absolute() {
        Err(ValidationError::new("Absolute path found instead of relative"))
    } else {
        Ok(())
    }
}

pub async fn write_profile_to_disk(path: &str, yaml_string: &str) -> Result<(), Box<dyn Error>> {
    tokio::fs::write(path, yaml_string).await?;
    Ok(())
}
