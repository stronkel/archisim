//! Base64 of the `data:` URIs of glTF.

pub fn decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut acc = 0u32;
    let mut bits = 0;
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' | b'\n' | b'\r' | b' ' => continue,
            _ => return None,
        } as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// The bytes of a `data:…;base64,…` URI.
pub fn data_uri(uri: &str) -> Option<Vec<u8>> {
    let rest = uri.strip_prefix("data:")?;
    let (head, data) = rest.split_once(',')?;
    if head.ends_with(";base64") {
        decode(data)
    } else {
        Some(crate::percent_decode(data).into_bytes())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn decodes() {
        assert_eq!(super::decode("TWFu").unwrap(), b"Man");
        assert_eq!(super::decode("TWE=").unwrap(), b"Ma");
        assert_eq!(
            super::data_uri("data:application/octet-stream;base64,TQ==").unwrap(),
            b"M"
        );
    }
}
