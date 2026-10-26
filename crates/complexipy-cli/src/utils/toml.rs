use std::path::Path;

use complexipy_core::config::{ConfigSource, read_complexipy_config, read_config_file};
use complexipy_core::sanitize::escape_controls;

use crate::args::CliArgs;
use crate::types::Config;

pub fn get_complexipy_toml_config(invocation_path: &str) -> Option<ConfigSource<Config>> {
    let (source, warnings) = read_complexipy_config(invocation_path);
    for warning in warnings {
        eprintln!("{}", escape_controls(&warning));
    }

    source
}

/// The config file a run uses: none under `--isolated`, the named file under
/// `--config`, and otherwise the first config file of the invocation directory.
pub fn load_run_config(
    cli: &CliArgs,
    invocation_path: &str,
) -> Result<Option<ConfigSource<Config>>, String> {
    if cli.isolated.unwrap_or(false) {
        return Ok(None);
    }
    match &cli.config {
        Some(path) => read_config_file(Path::new(path)).map(Some),
        None => Ok(get_complexipy_toml_config(invocation_path)),
    }
}
