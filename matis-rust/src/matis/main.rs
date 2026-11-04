fn main() {
    match matis::cli::commands::run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("Error: {error}");
            std::process::exit(1);
        }
    }
}
