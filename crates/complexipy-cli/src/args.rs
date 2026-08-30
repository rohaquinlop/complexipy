use clap::Parser;

use crate::types::{Color, OutputFormat, Sort};

#[derive(Parser, Debug, Clone, PartialEq)]
#[command(name = "complexipy", version)]
pub struct CliArgs {
    pub paths: Vec<String>,

    #[arg(short, long, value_delimiter = ',')]
    pub exclude: Vec<String>,

    #[arg(long)]
    pub max_complexity_allowed: Option<u64>,

    #[arg(long, num_args = 0..=1, default_missing_value = "true", require_equals = true)]
    pub snapshot_create: Option<bool>,

    #[arg(long, num_args = 0..=1, default_missing_value = "true", require_equals = true)]
    pub snapshot_ignore: Option<bool>,

    #[arg(short, long, num_args = 0..=1, default_missing_value = "true", require_equals = true)]
    pub quiet: Option<bool>,

    #[arg(short, long, num_args = 0..=1, default_missing_value = "true", require_equals = true)]
    pub ignore_complexity: Option<bool>,

    #[arg(short, long, num_args = 0..=1, default_missing_value = "true", require_equals = true)]
    pub failed: Option<bool>,

    #[arg(short = 'C', long)]
    pub color: Option<Color>,

    #[arg(short, long)]
    pub sort: Option<Sort>,

    #[arg(long)]
    pub output: Option<String>,

    #[arg(long)]
    pub cache_dir: Option<String>,

    #[arg(long, value_delimiter = ',')]
    pub output_format: Option<Vec<OutputFormat>>,

    #[arg(short, long)]
    pub diff: Option<String>,

    #[arg(long)]
    pub diff_only: Option<String>,

    #[arg(long, num_args = 0..=1, default_missing_value = "true", require_equals = true)]
    pub staged: Option<bool>,

    #[arg(short, long, value_parser = clap::value_parser!(u64).range(1..))]
    pub top: Option<u64>,

    #[arg(long, conflicts_with = "quiet", num_args = 0..=1, default_missing_value = "true", require_equals = true)]
    pub plain: Option<bool>,

    #[arg(long, num_args = 0..=1, default_missing_value = "true", require_equals = true)]
    pub suggest_refactors: Option<bool>,

    #[arg(long, num_args = 0..=1, default_missing_value = "true", require_equals = true)]
    pub check_script: Option<bool>,

    #[arg(long, num_args = 0..=1, default_missing_value = "true", require_equals = true)]
    pub no_ignore: Option<bool>,

    #[arg(long, num_args = 0..=1, default_missing_value = "true", require_equals = true)]
    pub report_ignored: Option<bool>,
}

#[cfg(test)]
mod tests;
