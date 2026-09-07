#![forbid(unsafe_code)]

use blossom_core::{
    ContextValue, NetworkConnectivity, NetworkConnectivityProvider,
    NetworkManagerConnectivityProvider, validate_network_connectivity_observation,
};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let expectation = std::env::args().nth(1).unwrap_or_default();
    match expectation.as_str() {
        "expect-offline" | "expect-no-internet" | "expect-online" => {}
        _ => {
            eprintln!(
                "usage: installed_network_probe expect-offline|expect-no-internet|expect-online"
            );
            std::process::exit(64);
        }
    }

    let mut provider = NetworkManagerConnectivityProvider;
    let observation = provider
        .read_network_connectivity()
        .unwrap_or_else(|error| {
            eprintln!("network evidence unavailable: {error}");
            std::process::exit(1);
        });
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            duration.as_millis().try_into().unwrap_or(u64::MAX)
        });
    validate_network_connectivity_observation(&observation, now_ms).unwrap_or_else(|error| {
        eprintln!("network evidence invalid: {error}");
        std::process::exit(1);
    });

    let ContextValue::Present(actual) = observation.observation else {
        eprintln!("network connectivity was unexpectedly absent");
        std::process::exit(1);
    };
    let matched = match expectation.as_str() {
        "expect-offline" => actual == NetworkConnectivity::Offline,
        "expect-no-internet" => matches!(
            actual,
            NetworkConnectivity::Offline
                | NetworkConnectivity::Limited
                | NetworkConnectivity::Local
        ),
        "expect-online" => actual == NetworkConnectivity::Online,
        _ => false,
    };
    if !matched {
        eprintln!("network evidence did not match the required class");
        std::process::exit(1);
    }
    let class = match actual {
        NetworkConnectivity::Offline => "offline",
        NetworkConnectivity::Online => "online",
        NetworkConnectivity::Local => "local",
        NetworkConnectivity::Limited => "limited",
        NetworkConnectivity::Unknown => "unknown",
    };
    println!("network-evidence={class} schema=1 source=system.network.connectivity");
}
