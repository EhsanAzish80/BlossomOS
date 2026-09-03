#![forbid(unsafe_code)]

#[cfg(debug_assertions)]
use blossom_core::{GatewayProfile, fixed_synthetic_provider_package};
#[cfg(debug_assertions)]
use std::fs::OpenOptions;
#[cfg(debug_assertions)]
use std::io::Write;
#[cfg(debug_assertions)]
use std::path::PathBuf;

#[cfg(debug_assertions)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let directory = PathBuf::from(arguments.next().ok_or("output directory is required")?);
    if arguments.next().is_some() || !directory.is_absolute() || !directory.is_dir() {
        return Err("output directory is invalid".into());
    }
    for (profile, name) in [
        (GatewayProfile::OllamaCpuV1, "blossom-model-ollama.service"),
        (
            GatewayProfile::LlamaCppCpuV1,
            "blossom-model-llama-cpp.service",
        ),
    ] {
        let package = fixed_synthetic_provider_package(profile)
            .map_err(|_| "synthetic provider package is invalid")?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join(name))?;
        output.write_all(package.rendered_unit())?;
        output.sync_all()?;
    }
    Ok(())
}

#[cfg(not(debug_assertions))]
fn main() {
    eprintln!("synthetic provider renderer is unavailable in release builds");
    std::process::exit(64);
}
