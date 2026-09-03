#![forbid(unsafe_code)]

use blossom_model_gateway::{GatewayProcessError, run_production};

fn main() {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let result = match arguments.as_slice() {
        [] => run_production(),
        #[cfg(all(debug_assertions, unix))]
        [argument] if argument == "--synthetic-fixture" => {
            blossom_model_gateway::run_synthetic_fixture_from_environment()
        }
        _ => Err(GatewayProcessError::InvalidInvocation),
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(error.exit_code());
    }
}
