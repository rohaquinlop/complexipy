use complexipy_core::config::{config_candidates, load_candidate_value};
use serde::Deserialize;

use crate::types::Config;

pub fn get_complexipy_toml_config(invocation_path: &str) -> Option<Config> {
    for candidate in config_candidates(invocation_path) {
        let Some(value) = load_candidate_value(&candidate) else {
            continue;
        };

        match Config::deserialize(value) {
            Ok(config) => return Some(config),
            Err(e) => eprintln!("Invalid config in {}: {}", candidate.path.display(), e),
        }
    }

    None
}
