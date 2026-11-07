//! Percent-encoding for the query strings an OAuth flow sends and receives.

use std::borrow::Cow;

fn is_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"-._~".contains(&byte)
}

pub fn encode(value: &str) -> Cow<'_, str> {
    if value.bytes().all(is_unreserved) {
        return Cow::Borrowed(value);
    }
    Cow::Owned(value.bytes().fold(String::new(), |mut encoded, byte| {
        if is_unreserved(byte) {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
        encoded
    }))
}

/// `pairs` as `key=value&…`.
pub fn build(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(key, value)| format!("{key}={}", encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

fn decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let hex = bytes
            .get(index + 1..index + 3)
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match (bytes[index], hex) {
            (b'%', Some(byte)) => {
                decoded.push(byte);
                index += 3;
            }
            (b'+', _) => {
                decoded.push(b' ');
                index += 1;
            }
            (byte, _) => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// The value of `key` in `query`, decoded.
pub fn find(query: &str, key: &str) -> Option<String> {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(name, _)| *name == key)
        .map(|(_, value)| decode(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_survives_encoding_and_decoding() {
        let scope = "user-read playback/state&x=1";
        let query = build(&[("scope", scope), ("code", "abc")]);
        assert_eq!(find(&query, "scope").as_deref(), Some(scope));
        assert_eq!(find(&query, "code").as_deref(), Some("abc"));
        assert_eq!(find(&query, "state"), None);
    }

    #[test]
    fn plain_values_are_not_copied() {
        assert!(matches!(encode("plain-value_1.~"), Cow::Borrowed(_)));
    }
}
