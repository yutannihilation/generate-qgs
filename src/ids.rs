//! Unique id generation.
//!
//! QGIS sprinkles UUIDs over the project file (layer ids, symbol layer ids,
//! graduated range ids, ...). They only need to be unique within the
//! document, so a cheap non-cryptographic source is enough: we mix the
//! current time, the process id and an atomic counter.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Returns a random-looking UUID string, e.g. `"{0b31b699-73f7-4f89-bb3b-2ddb939863a3}"`
/// without the braces.
pub(crate) fn uuid() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = u64::from(std::process::id());

    let mut state = nanos ^ count.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (pid << 32);
    let mut bytes = [0u8; 16];
    for chunk in bytes.chunks_exact_mut(8) {
        // murmur3 fmix64
        state ^= state >> 33;
        state = state.wrapping_mul(0xff51_afd7_ed55_8ccd);
        state ^= state >> 33;
        chunk.copy_from_slice(&state.to_le_bytes());
        state = state.rotate_left(17) ^ nanos ^ count;
    }
    // Set the version/variant bits for a UUIDv4 look-alike (cosmetic only).
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    let mut s = String::with_capacity(36);
    for (i, b) in bytes.iter().enumerate() {
        if matches!(i, 4 | 6 | 8 | 10) {
            s.push('-');
        }
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Layer id as QGIS generates it: the layer name stripped to ASCII
/// alphanumerics, followed by `_` and a UUID with dashes turned into
/// underscores, e.g. `nc_b1079259_f0a1_4ebf_8df3_a22a440d836b`. If the name
/// has no ASCII alphanumerics (e.g. Japanese), the prefix is empty and the
/// id starts with `_`.
pub(crate) fn layer_id(name: &str) -> String {
    let prefix: String = name.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    format!("{}_{}", prefix, uuid().replace('-', "_"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn uuids_are_unique() {
        let ids: HashSet<String> = (0..1000).map(|_| uuid()).collect();
        assert_eq!(ids.len(), 1000);
    }

    #[test]
    fn uuid_format() {
        let id = uuid();
        assert_eq!(id.len(), 36);
        assert_eq!(id.chars().filter(|&c| c == '-').count(), 4);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit() || c == '-'));
    }

    #[test]
    fn layer_id_sanitizes_name() {
        let id = layer_id("nc");
        assert!(id.starts_with("nc_"));
        assert!(!id.contains('-'));

        let id = layer_id("地理院タイル（標準地図）");
        assert!(id.starts_with('_'));
    }
}
