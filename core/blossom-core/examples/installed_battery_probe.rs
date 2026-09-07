#![forbid(unsafe_code)]

use blossom_core::{
    BatteryState, BatterySummaryProvider, ContextValue, UpowerBatterySummaryProvider,
    validate_battery_observation,
};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let expected = std::env::args().nth(1).unwrap_or_default();
    if !matches!(expected.as_str(), "expect-present" | "expect-absent") {
        eprintln!("usage: installed_battery_probe expect-present|expect-absent");
        std::process::exit(64);
    }

    let mut provider = UpowerBatterySummaryProvider;
    let observation = provider.read_battery_summary().unwrap_or_else(|error| {
        eprintln!("battery evidence unavailable: {error}");
        std::process::exit(1);
    });
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            duration.as_millis().try_into().unwrap_or(u64::MAX)
        });
    validate_battery_observation(&observation, now_ms).unwrap_or_else(|error| {
        eprintln!("battery evidence invalid: {error}");
        std::process::exit(1);
    });

    match (expected.as_str(), observation.observation) {
        ("expect-present", ContextValue::Present(summary)) => {
            let state = match summary.state {
                BatteryState::Charging => "charging",
                BatteryState::Discharging => "discharging",
                BatteryState::Full => "full",
                BatteryState::NotCharging => "not_charging",
                BatteryState::Unknown => "unknown",
            };
            println!(
                "battery-evidence=present state={state} schema=1 source=system.battery.summary"
            );
        }
        ("expect-absent", ContextValue::Absent) => {
            println!("battery-evidence=absent schema=1 source=system.battery.summary");
        }
        ("expect-present", ContextValue::Absent) => {
            eprintln!("expected a present battery but UPower reported absent");
            std::process::exit(1);
        }
        ("expect-absent", ContextValue::Present(_)) => {
            eprintln!("expected no battery but UPower reported present");
            std::process::exit(1);
        }
        _ => unreachable!(),
    }
}
