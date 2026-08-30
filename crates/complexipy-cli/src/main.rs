use std::process::ExitCode;

use clap::Parser;

use complexipy_cli::args::CliArgs;
use complexipy_cli::run::run_at;

fn main() -> ExitCode {
    let invocation_path = std::env::current_dir()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();
    run_at(CliArgs::parse(), &invocation_path)
}
