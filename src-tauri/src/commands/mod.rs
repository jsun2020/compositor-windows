pub mod files;
pub mod package;
pub mod recent;

/// Decodes `%XX` percent-escapes in an IPC header value. An escape that is
/// missing a full pair of hex digits, or whose digits don't parse, is copied
/// through verbatim rather than dropped or panicking.
pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_decode_handles_common_escapes_and_a_trailing_lone_percent() {
        assert_eq!(percent_decode("%20"), " ");
        assert_eq!(percent_decode("%2F"), "/");
        assert_eq!(percent_decode("C%3A/p/A%20B%2FC"), "C:/p/A B/C");
        assert_eq!(percent_decode("trailing%"), "trailing%");
    }
}
pub mod clipboard;
