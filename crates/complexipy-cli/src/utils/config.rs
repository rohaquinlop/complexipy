use std::fmt;

use crate::args::CliArgs;
use crate::types::{Color, Config, RunConfig, Sort};

#[derive(Debug, PartialEq)]
pub enum ConfigError {
    MissingPaths,
    CacheDirNotAString,
    CacheDirEmpty,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPaths => write!(
                f,
                "You need to define paths in the CLI call arguments or in complexipy.toml file"
            ),
            Self::CacheDirNotAString => write!(
                f,
                "The 'cache-dir' option must be a string in the TOML config file"
            ),
            Self::CacheDirEmpty => write!(
                f,
                "The 'cache-dir' option cannot be an empty string in the TOML config file"
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

fn resolve_cache_dir(
    cli_value: Option<String>,
    toml_value: Option<&toml::Value>,
) -> Result<Option<String>, ConfigError> {
    if let Some(value) = cli_value {
        if value.trim().is_empty() {
            return Err(ConfigError::CacheDirEmpty);
        }
        return Ok(Some(value));
    }
    match toml_value {
        Some(toml::Value::String(value)) => {
            if value.trim().is_empty() {
                Err(ConfigError::CacheDirEmpty)
            } else {
                Ok(Some(value.clone()))
            }
        }
        Some(_) => Err(ConfigError::CacheDirNotAString),
        None => Ok(None),
    }
}

pub fn resolve_config(toml_config: Option<Config>, cli: CliArgs) -> Result<RunConfig, ConfigError> {
    let toml = toml_config.as_ref();

    let paths = if !cli.paths.is_empty() {
        cli.paths
    } else if let Some(toml) = toml {
        toml.paths.clone().into_vec()
    } else {
        return Err(ConfigError::MissingPaths);
    };

    let max_complexity_allowed = cli
        .max_complexity_allowed
        .or_else(|| toml.map(|toml| toml.max_complexity_allowed))
        .unwrap_or(15);

    let snapshot_create = cli
        .snapshot_create
        .or_else(|| toml.map(|toml| toml.snapshot_create))
        .unwrap_or(false);

    let snapshot_ignore = cli
        .snapshot_ignore
        .or_else(|| toml.map(|toml| toml.snapshot_ignore))
        .unwrap_or(false);

    let quiet = cli
        .quiet
        .or_else(|| toml.map(|toml| toml.quiet))
        .unwrap_or(false);

    let ignore_complexity = cli
        .ignore_complexity
        .or_else(|| toml.map(|toml| toml.ignore_complexity))
        .unwrap_or(false);

    let failed = cli
        .failed
        .or_else(|| toml.map(|toml| toml.failed))
        .unwrap_or(false);

    let color = cli.color.or_else(|| toml.map(|toml| toml.color.clone()));
    let color = color.unwrap_or(Color::Auto);

    let sort = cli.sort.or_else(|| toml.map(|toml| toml.sort.clone()));
    let sort = sort.unwrap_or(Sort::Asc);

    let output_format = match cli.output_format {
        Some(formats) => formats,
        None => toml
            .map(|toml| toml.output_format.clone().into_vec())
            .unwrap_or_default(),
    };

    let output = cli
        .output
        .or_else(|| toml.and_then(|toml| toml.output.clone()));

    let exclude = if !cli.exclude.is_empty() {
        cli.exclude
    } else {
        toml.map(|toml| toml.exclude.clone().into_vec())
            .unwrap_or_default()
    };

    let check_script = cli
        .check_script
        .or_else(|| toml.map(|toml| toml.check_script))
        .unwrap_or(false);

    let no_ignore = cli
        .no_ignore
        .or_else(|| toml.map(|toml| toml.no_ignore))
        .unwrap_or(false);

    let report_ignored = cli
        .report_ignored
        .or_else(|| toml.map(|toml| toml.report_ignored))
        .unwrap_or(false);

    let plain = cli.plain.unwrap_or(false);
    let suggest_refactors = cli.suggest_refactors.unwrap_or(false);
    let top = cli.top;
    let cache_dir =
        resolve_cache_dir(cli.cache_dir, toml.and_then(|toml| toml.cache_dir.as_ref()))?;

    let mut diff = cli.diff;
    let diff_only = cli.diff_only;
    let mut staged = cli.staged.unwrap_or(false);
    if let Some(section) = toml.and_then(|toml| toml.diff.as_ref()) {
        if diff.is_none()
            && diff_only.is_none()
            && let Some(branch) = &section.branch
        {
            diff = Some(branch.clone());
        }
        if cli.staged.is_none()
            && let Some(section_staged) = section.staged
        {
            staged = section_staged;
        }
    }

    Ok(RunConfig {
        paths,
        max_complexity_allowed,
        snapshot_create,
        snapshot_ignore,
        quiet,
        ignore_complexity,
        failed,
        color,
        sort,
        output_format,
        output,
        exclude,
        check_script,
        no_ignore,
        report_ignored,
        plain,
        suggest_refactors,
        top,
        cache_dir,
        diff,
        diff_only,
        staged,
    })
}

#[cfg(test)]
mod tests;
