//! `text/uri-list` parsing, per RFC 2483.
//!
//! crowndock carries a copy of this that skips percent-decoding; a dropped file
//! whose name contains a space or a `#` breaks there. This one decodes.

use std::{
    ffi::OsString,
    os::unix::ffi::OsStringExt,
    path::{Path, PathBuf},
};

pub const URI_LIST: &str = "text/uri-list";

/// The paths in a `text/uri-list` payload, in order, skipping blank lines and
/// the `#` comments the format allows.
pub fn parse_uri_list(payload: &[u8]) -> impl Iterator<Item = PathBuf> + '_ {
    payload
        .split(|&b| b == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line))
        .filter(|line| !line.is_empty() && !line.starts_with(b"#"))
        .filter_map(uri_to_path)
}

/// The path a `file://` URI names, or `None` for any other scheme.
///
/// A path is bytes, not text, so the percent-decoded body becomes an `OsString`
/// directly rather than going through `String::from_utf8_lossy` — a filename
/// that is not valid UTF-8 still round-trips.
pub fn uri_to_path(uri: &[u8]) -> Option<PathBuf> {
    let rest = uri.strip_prefix(b"file://")?;
    // An authority component, when present, is the host the file lives on.
    // Only an empty one or `localhost` names this machine.
    let path = match rest.iter().position(|&b| b == b'/') {
        Some(0) => rest,
        Some(slash) if &rest[..slash] == b"localhost" => &rest[slash..],
        _ => return None,
    };
    Some(PathBuf::from(OsString::from_vec(percent_decode(path))))
}

/// The `file://` URI naming `path`, for a drag this process originates.
pub fn path_to_uri(path: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;

    let mut uri = String::from("file://");
    for &byte in path.as_os_str().as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                uri.push(byte as char)
            }
            _ => uri.push_str(&format!("%{byte:02X}")),
        }
    }
    uri
}

/// `text` percent-encoded for use as one query-string value.
pub fn encode_component(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len());
    for &byte in text.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char)
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn percent_decode(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    let mut rest = input;
    while let Some((&byte, tail)) = rest.split_first() {
        match (byte, tail) {
            (b'%', [high, low, ..]) => match (nibble(*high), nibble(*low)) {
                (Some(high), Some(low)) => {
                    out.push((high << 4) | low);
                    rest = &tail[2..];
                }
                _ => {
                    out.push(byte);
                    rest = tail;
                }
            },
            _ => {
                out.push(byte);
                rest = tail;
            }
        }
    }
    out
}

fn nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(input: &str) -> String {
        String::from_utf8(percent_decode(input.as_bytes())).unwrap()
    }

    #[test]
    fn percent_escapes_decode() {
        assert_eq!(decode("/tmp/my%20file.txt"), "/tmp/my file.txt");
        assert_eq!(decode("/tmp/a%23b"), "/tmp/a#b");
        assert_eq!(decode("/tmp/plain"), "/tmp/plain");
    }

    #[test]
    fn a_stray_percent_survives_untouched() {
        assert_eq!(decode("100%"), "100%");
        assert_eq!(decode("%zz"), "%zz");
        assert_eq!(decode("%4"), "%4");
    }

    #[test]
    fn a_file_uri_becomes_its_path() {
        assert_eq!(
            uri_to_path(b"file:///home/u/Desktop/note.txt"),
            Some(PathBuf::from("/home/u/Desktop/note.txt"))
        );
        assert_eq!(
            uri_to_path(b"file://localhost/tmp/x"),
            Some(PathBuf::from("/tmp/x"))
        );
    }

    #[test]
    fn a_uri_this_machine_cannot_serve_is_rejected() {
        assert_eq!(uri_to_path(b"http://example.com/x"), None);
        assert_eq!(uri_to_path(b"file://otherhost/tmp/x"), None);
    }

    #[test]
    fn a_uri_list_skips_blanks_and_comments() {
        let payload = b"# comment\r\nfile:///a\r\n\r\nfile:///b%20c\r\n";
        let paths: Vec<_> = parse_uri_list(payload).collect();
        assert_eq!(paths, [PathBuf::from("/a"), PathBuf::from("/b c")]);
    }

    #[test]
    fn a_query_value_escapes_everything_but_the_unreserved_set() {
        assert_eq!(encode_component("São Paulo"), "S%C3%A3o%20Paulo");
        assert_eq!(encode_component("a&b=c"), "a%26b%3Dc");
    }

    #[test]
    fn a_path_round_trips_through_its_uri() {
        for path in ["/tmp/my file.txt", "/tmp/a#b", "/tmp/héllo", "/tmp/100%"] {
            let uri = path_to_uri(Path::new(path));
            assert_eq!(uri_to_path(uri.as_bytes()), Some(PathBuf::from(path)));
        }
    }
}
