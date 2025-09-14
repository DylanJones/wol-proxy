// Simple UDP packet recognition logic we can unit test off-device.
// Current strategy: any non-empty payload triggers wake; optionally accept a
// fixed ASCII token to avoid accidental wakes on noisy networks.

pub const DEFAULT_TOKEN: &[u8] = b"WAKE";

pub fn packet_means_wake(payload: &[u8]) -> bool {
    if payload.is_empty() {
        return false;
    }
    if payload == DEFAULT_TOKEN {
        return true;
    }
    // Accept also lowercase token or any non-empty payload as a fallback
    if payload.eq_ignore_ascii_case(DEFAULT_TOKEN) {
        return true;
    }
    // Minimalistic: any non-empty packet
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_token() {
        assert!(packet_means_wake(b"WAKE"));
        assert!(packet_means_wake(b"wake"));
    }

    #[test]
    fn non_empty_is_true() {
        assert!(packet_means_wake(b"x"));
    }

    #[test]
    fn empty_is_false() {
        assert!(!packet_means_wake(b""));
    }
}
