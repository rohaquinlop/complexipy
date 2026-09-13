pub mod analysis;
pub mod documents;
pub mod server;

pub use server::SERVER_NAME;

pub fn run_server() -> i32 {
    let (connection, io_threads) = lsp_server::Connection::stdio();
    let exit_code = server::serve(connection);
    let _ = io_threads.join();

    exit_code
}
