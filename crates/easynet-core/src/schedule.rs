//! Punch timers. The caller supplies `now_ms`.

/// Probe interval while a path is not established.
pub const PROBE_MS: u64 = 250;
/// Keepalive interval on an established path, and STUN refresh during pairing.
pub const KEEPALIVE_MS: u64 = 15_000;

/// True when `now_ms - last_ms` has reached `interval_ms`.
pub fn due(last_ms: u64, now_ms: u64, interval_ms: u64) -> bool {
    now_ms.saturating_sub(last_ms) >= interval_ms
}

#[cfg(test)]
mod tests {
    use super::{KEEPALIVE_MS, due};

    #[test]
    fn keepalive_is_due_at_fifteen_seconds() {
        let last = 1_000_000;
        assert!(!due(last, last + 14_999, KEEPALIVE_MS));
        assert!(due(last, last + 15_000, KEEPALIVE_MS));
    }
}
