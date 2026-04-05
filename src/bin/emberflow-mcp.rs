fn main() {
    if let Err(error) = emberflow::mcp::stdio::serve_from_env() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
