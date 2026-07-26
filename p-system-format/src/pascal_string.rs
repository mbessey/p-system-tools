// Decodes (and encodes) the two Pascal-string encodings used by UCSD
// p-System on-disk formats: a length byte followed by that many characters
// (used for names in disk directory entries), and a fixed-width buffer
// space-padded on the right (used for fields like code-segment names).

use crate::error::FormatError;

pub fn from_length_prefixed(pstring: &[u8]) -> String {
    let len = pstring[0] as usize;
    pstring[1..=len].iter().map(|&b| b as char).collect()
}

pub fn from_space_padded(bytes: &[u8]) -> String {
    let mut result = String::new();
    for c in bytes {
        if *c == 0x20 {
            break;
        }
        result.push(*c as char);
    }
    result
}

pub fn to_length_prefixed<const N: usize>(s: &str) -> Result<[u8; N], FormatError> {
    if s.len() > N - 1 || s.len() > 255 {
        return Err(FormatError::InvalidValue {
            field: "length_prefixed string",
            value: s.len() as u32,
        });
    }
    let mut buf = [0u8; N];
    buf[0] = s.len() as u8;
    buf[1..=s.len()].copy_from_slice(s.as_bytes());
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_length_prefixed_round_trip() {
        let buf = to_length_prefixed::<16>("HELLO.TEXT").unwrap();
        assert_eq!(from_length_prefixed(&buf), "HELLO.TEXT");
    }

    #[test]
    fn to_length_prefixed_rejects_too_long() {
        assert!(to_length_prefixed::<4>("TOOLONG").is_err());
    }

    #[test]
    fn to_length_prefixed_boundary_fits_exactly() {
        // N=4 means at most 3 chars (byte 0 is the length prefix).
        let buf = to_length_prefixed::<4>("ABC").unwrap();
        assert_eq!(from_length_prefixed(&buf), "ABC");
        assert!(to_length_prefixed::<4>("ABCD").is_err());
    }
}
