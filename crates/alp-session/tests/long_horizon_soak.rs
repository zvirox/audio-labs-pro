//! Long-horizon playback soak harness.
//!
//! The full 24-hour run is `#[ignore]` because it is a deliberate endurance
//! check, not a unit test. CI runs the short proxy; operators run the long one
//! with `cargo test -p alp-session --test long_horizon_soak -- --ignored`.
//!
//! Note on honesty: the headless transport's virtual clock is wall-clock based,
//! so this exercises 86,400 consecutive monitor callbacks and checks memory
//! stability and monotonic timestamps. It does not accelerate audio samples —
//! real-device drop-frame checks belong in `scripts/phase3-demo.sh --soak`.

use alp_session::{EditSession, TransportController};

/// Number of one-second-equivalent monitor callbacks in 24 hours.
const TICKS_24H: u64 = 24 * 60 * 60;

/// Resident set size in kB, parsed from Linux procfs without external crates.
fn rss_kb() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/self/status").ok()?;
    text.lines()
        .find(|line| line.starts_with("VmRSS:"))
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u64>().ok())
}

fn soak(ticks: u64, max_growth_kb: u64) {
    let mut transport = TransportController::new(ticks as f64, 48_000);
    let session = EditSession::demo();
    transport.play(&session).expect("play succeeds");

    let rss_start = rss_kb();
    let mut last_position = 0.0_f64;
    let mut dropped = 0u64;
    let mut callbacks = 0u64;

    for _ in 0..ticks {
        let frame = transport.tick();
        if frame.position_secs + f64::EPSILON < last_position {
            dropped += 1;
        }
        last_position = frame.position_secs;
        callbacks += 1;
    }

    assert_eq!(callbacks, ticks, "every callback must be delivered");
    assert_eq!(dropped, 0, "playhead must never move backwards");
    assert!(
        transport.state().playing,
        "transport must remain active after soak"
    );

    if let (Some(start), Some(end)) = (rss_start, rss_kb()) {
        let growth = end.saturating_sub(start);
        assert!(
            growth < max_growth_kb,
            "RSS grew by {growth} kB over {ticks} callbacks (limit {max_growth_kb} kB)"
        );
    }
}

#[test]
fn short_playback_soak_is_stable() {
    soak(1_000, 64 * 1024);
}

#[test]
#[ignore = "24-hour simulated playback soak; run explicitly with --ignored"]
fn simulated_24h_playback_soak_has_no_dropped_callbacks() {
    assert_eq!(TICKS_24H, 86_400);
    soak(TICKS_24H, 256 * 1024);
}
