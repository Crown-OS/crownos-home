//! Percent-encoding for query strings, and `file://` URIs back to paths.

use std::path::PathBuf;

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

/// The path a `file://` URI names, its percent-escapes undone.
pub fn file_path(uri: &str) -> Option<PathBuf> {
    let escaped = uri.strip_prefix("file://")?.as_bytes();
    let mut bytes = Vec::with_capacity(escaped.len());
    let mut rest = escaped;
    while let Some((&byte, after)) = rest.split_first() {
        let decoded = (byte == b'%')
            .then(|| std::str::from_utf8(after.get(..2)?).ok())
            .flatten()
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match decoded {
            Some(value) => {
                bytes.push(value);
                rest = &after[2..];
            }
            None => {
                bytes.push(byte);
                rest = after;
            }
        }
    }
    Some(PathBuf::from(String::from_utf8(bytes).ok()?))
}

/// The local files a `text/uri-list` names, skipping comments and other schemes.
pub fn file_paths(list: &str) -> Vec<PathBuf> {
    list.lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .filter_map(file_path)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_uri_reads_back_as_its_path() {
        assert_eq!(
            file_path("file:///home/me/My%20Photo%2B1.jpg"),
            Some(PathBuf::from("/home/me/My Photo+1.jpg"))
        );
        assert_eq!(file_path("https://example.com"), None);
    }

    #[test]
    fn a_uri_list_reads_back_as_its_files() {
        let list =
            "# dragged\r\nfile:///tmp/a.png\r\nhttps://example.com/b\r\nfile:///tmp/c%20d.pdf\r\n";
        assert_eq!(
            file_paths(list),
            [PathBuf::from("/tmp/a.png"), PathBuf::from("/tmp/c d.pdf")]
        );
    }

    #[test]
    fn a_query_value_escapes_everything_but_the_unreserved_set() {
        assert_eq!(encode_component("São Paulo"), "S%C3%A3o%20Paulo");
        assert_eq!(encode_component("a&b=c"), "a%26b%3Dc");
    }
}
