use std::process::ExitCode;

use clap::Parser;

use complexipy_cli::args::CliArgs;
use complexipy_cli::run::run_at;

#[cfg(any(
    target_os = "macos",
    target_os = "windows",
    all(target_os = "linux", target_arch = "x86_64")
))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> ExitCode {
    let invocation_path = std::env::current_dir()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();
    run_at(CliArgs::parse(), &invocation_path)
}
