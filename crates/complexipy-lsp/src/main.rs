#[cfg(any(
    target_os = "macos",
    target_os = "windows",
    all(target_os = "linux", target_arch = "x86_64"),
    all(target_os = "linux", target_arch = "aarch64", target_env = "musl")
))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    std::process::exit(complexipy_lsp::run_server());
}
