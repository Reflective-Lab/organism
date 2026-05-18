//! Shared helpers for routing `draft_batch_id` through fact ids and
//! diagnostic markers. The critic and scorer both need to mint and
//! parse batch-scoped fact ids; this module is their single source of
//! truth so the round-driven and explicit-batch paths cannot drift.

/// Hex-encode a `draft_batch_id` for use in fact ids. Hex is used so
/// arbitrary string batch ids — including ones that contain characters
/// disallowed in Converge fact ids — round-trip safely.
#[must_use]
pub(crate) fn encode_batch_id(draft_batch_id: &str) -> String {
    let mut encoded = String::with_capacity(draft_batch_id.len() * 2);
    for byte in draft_batch_id.as_bytes() {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

/// Reverse [`encode_batch_id`]. Returns `None` if the input isn't a
/// valid hex-encoded UTF-8 string.
#[must_use]
pub(crate) fn decode_batch_id(encoded: &str) -> Option<String> {
    if !encoded.len().is_multiple_of(2) {
        return None;
    }
    let mut bytes = Vec::with_capacity(encoded.len() / 2);
    for chunk in encoded.as_bytes().chunks(2) {
        let s = std::str::from_utf8(chunk).ok()?;
        bytes.push(u8::from_str_radix(s, 16).ok()?);
    }
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_simple_ids() {
        let cases = ["design-round-1", "good-batch", "abc/def", ""];
        for case in cases {
            let encoded = encode_batch_id(case);
            assert_eq!(decode_batch_id(&encoded).as_deref(), Some(case));
        }
    }

    #[test]
    fn rejects_malformed_hex() {
        assert!(decode_batch_id("zz").is_none());
        assert!(decode_batch_id("abc").is_none()); // odd length
    }
}
