use std::fmt;
use std::path::Path;

use complexipy_core::fs_guard::is_within;

use crate::args::CliArgs;
use crate::types::{Color, Config, ConfigGateChange, RunConfig, Sort};

#[derive(Debug, PartialEq)]
pub enum ConfigError {
    MissingPaths,
    CacheDirNotAString,
    CacheDirEmpty,
    OptionLikeDiffRef(String),
    OutsideProject { key: &'static str, value: String },
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
            Self::OptionLikeDiffRef(git_ref) => write!(
                f,
                "The diff reference {:?} starts with '-', so git would read it as an option",
                git_ref
            ),
            Self::OutsideProject { key, value } => write!(
                f,
                "The '{}' value {:?} in the config file points outside the project directory; \
                 pass it on the command line to use a path outside the project",
                key, value
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

fn resolve_cache_dir(
    cli_value: Option<String>,
    toml_value: Option<&toml::Value>,
    root: &Path,
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
                inside_project(root, "cache-dir", value).map(Some)
            }
        }
        Some(_) => Err(ConfigError::CacheDirNotAString),
        None => Ok(None),
    }
}

fn inside_project(root: &Path, key: &'static str, value: &str) -> Result<String, ConfigError> {
    if is_within(root, Path::new(value)) {
        Ok(value.to_string())
    } else {
        Err(ConfigError::OutsideProject {
            key,
            value: value.to_string(),
        })
    }
}

fn reject_option_like(reference: &str) -> Result<(), ConfigError> {
    if reference.starts_with('-') {
        Err(ConfigError::OptionLikeDiffRef(reference.to_string()))
    } else {
        Ok(())
    }
}

/// Merges the CLI arguments over the config file. Values that come from the
/// config file are untrusted: scan and write paths must stay inside the
/// project directory, and no diff reference may start with `-`.
pub fn resolve_config(
    toml_config: Option<Config>,
    cli: CliArgs,
    invocation_path: &str,
) -> Result<RunConfig, ConfigError> {
    let toml = toml_config.as_ref();
    let root = Path::new(invocation_path);

    let paths = if !cli.paths.is_empty() {
        cli.paths
    } else if let Some(toml) = toml {
        toml.paths
            .clone()
            .into_vec()
            .iter()
            .map(|path| inside_project(root, "paths", path))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        return Err(ConfigError::MissingPaths);
    };
    if paths.is_empty() {
        return Err(ConfigError::MissingPaths);
    }

    let max_complexity_allowed = cli
        .max_complexity_allowed
        .or_else(|| toml.map(|toml| toml.max_complexity_allowed))
        .unwrap_or(15);

    let mut gate_changes_from_config = Vec::new();
    let snapshot_create = match cli.snapshot_create {
        Some(value) => value,
        None => {
            let from_config = toml.is_some_and(|toml| toml.snapshot_create);
            if from_config {
                gate_changes_from_config.push(ConfigGateChange::SnapshotCreate);
            }
            from_config
        }
    };

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

    let output = match cli.output {
        Some(output) => Some(output),
        None => toml
            .and_then(|toml| toml.output.as_deref())
            .map(|output| inside_project(root, "output", output))
            .transpose()?,
    };

    let exclude = if !cli.exclude.is_empty() {
        cli.exclude
    } else {
        toml.map(|toml| toml.exclude.clone().into_vec())
            .unwrap_or_default()
    };

    let select = if !cli.select.is_empty() {
        cli.select
    } else {
        toml.map(|toml| toml.select.clone().into_vec())
            .unwrap_or_default()
    };

    let ignore = if !cli.ignore.is_empty() {
        cli.ignore
    } else {
        toml.map(|toml| toml.ignore.clone().into_vec())
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
    let fix = cli.fix.unwrap_or(false);
    let dry_run = cli.dry_run.unwrap_or(false);
    let suggest_refactors = cli.suggest_refactors.unwrap_or(false) || fix || dry_run;
    let top = cli.top;
    let cache_dir = resolve_cache_dir(
        cli.cache_dir,
        toml.and_then(|toml| toml.cache_dir.as_ref()),
        root,
    )?;

    let mut diff = cli.diff;
    let diff_only = cli.diff_only;
    let mut staged = cli.staged.unwrap_or(false);
    let section = toml
        .and_then(|toml| toml.diff.as_ref())
        .filter(|section| section.branch.as_deref() != Some(""));
    if let Some(section) = section {
        if diff.is_none()
            && diff_only.is_none()
            && let Some(branch) = &section.branch
        {
            diff = Some(branch.clone());
            gate_changes_from_config.push(ConfigGateChange::DiffRef(branch.clone()));
        }
        if cli.staged.is_none()
            && let Some(section_staged) = section.staged
        {
            staged = section_staged;
            if section_staged && diff_only.is_none() {
                gate_changes_from_config.push(ConfigGateChange::StagedDiff);
            }
        }
    }
    for reference in diff.iter().chain(diff_only.iter()) {
        reject_option_like(reference)?;
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
        select,
        ignore,
        check_script,
        no_ignore,
        report_ignored,
        plain,
        suggest_refactors,
        fix,
        dry_run,
        top,
        cache_dir,
        diff,
        diff_only,
        staged,
        gate_changes_from_config,
    })
}

#[cfg(test)]
mod tests;
