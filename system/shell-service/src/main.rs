#![forbid(unsafe_code)]

fn main() {
    if let Err(error) = blossom_shell_service::run_production() {
        eprintln!("{error}");
        std::process::exit(error.exit_code());
    }
}
