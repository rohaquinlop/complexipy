use std::io::{BufReader, BufWriter};

pub mod analysis;
pub mod documents;
pub mod server;
pub mod transport;
mod workspace;

pub use server::SERVER_NAME;

pub fn run_server() -> i32 {
    let streams = transport::connect(
        BufReader::new(std::io::stdin()),
        BufWriter::new(std::io::stdout()),
    );
    let (connection, writer) = match streams {
        Ok(streams) => streams,
        Err(error) => {
            eprintln!(
                "{}: cannot start the stdio transport: {}",
                SERVER_NAME, error
            );
            return server::EXIT_CODE_FAILURE;
        }
    };
    let exit_code = server::serve(connection);
    let _ = writer.join();

    exit_code
}
