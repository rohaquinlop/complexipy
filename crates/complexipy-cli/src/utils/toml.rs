use std::{fs, path::Path};

use toml;

use crate::types::Config;

pub fn get_complexipy_toml_config(invocation_path: &str) -> Option<Config> {
    let invocation_path = Path::new(invocation_path);

    if let Some(toml) = load_toml_config(invocation_path, "complexipy.toml") {
        return Some(toml);
    } else if let Some(toml) = load_toml_config(invocation_path, ".complexipy.toml") {
        return Some(toml);
    }
    load_pyproject_config(invocation_path)
}

fn load_toml_config(invocation_path: &Path, file_name: &str) -> Option<Config> {
    let config_file_path = invocation_path.join(file_name);

    if !config_file_path.exists() {
        return None;
    }

    let content = fs::read_to_string(&config_file_path).ok()?;

    match toml::from_str(&content) {
        Ok(config) => Some(config),
        Err(e) => {
            eprintln!("Failed to parse {}: {}", config_file_path.display(), e);
            None
        }
    }
}

fn load_pyproject_config(invocation_path: &Path) -> Option<Config> {
    let config_file_path = invocation_path.join("pyproject.toml");

    if !config_file_path.exists() {
        return None;
    }

    let content = fs::read_to_string(&config_file_path).ok()?;

    let value: toml::Value = match toml::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to parse {}: {}", config_file_path.display(), e);
            return None;
        }
    };

    let section = value.get("tool")?.get("complexipy")?;

    match section.clone().try_into() {
        Ok(config) => Some(config),
        Err(e) => {
            eprintln!("Invalid config in {}: {}", config_file_path.display(), e);
            None
        }
    }
}
