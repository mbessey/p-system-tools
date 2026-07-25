// Decodes the two Pascal-string encodings used by UCSD p-System on-disk
// formats: a length byte followed by that many characters (used for names in
// disk directory entries), and a fixed-width buffer space-padded on the
// right (used for fields like code-segment names).

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
