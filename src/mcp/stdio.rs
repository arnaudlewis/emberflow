use crate::mcp::server::{start_stdio_server, StdioTransportConfig};
use std::io::{self, BufReader};

pub fn serve_from_env() -> Result<(), String> {
    let session =
        start_stdio_server(StdioTransportConfig::from_env()).map_err(|error| error.to_string())?;
    let stdin = io::stdin();
    let stdout = io::stdout();
    let stderr = io::stderr();
    let mut stdout = stdout.lock();
    let mut stderr = stderr.lock();
    let input = BufReader::new(stdin.lock());
    session
        .serve_stdio(input, &mut stdout, &mut stderr)
        .map_err(|error| error.to_string())
}
