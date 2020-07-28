use serde_derive::Deserialize;
use std::fs;

#[derive(Deserialize)]
pub struct HalDefinition {
    pub version: String,
    pub svd_patch_path: String,
}

impl HalDefinition {
    pub fn read(config_filename: &str) -> HalDefinition {
        let contents = fs::read_to_string(config_filename).ok().unwrap();
        serde_yaml::from_str(contents.as_str()).expect("Error while parsing hal configuration file")
    }
}
