// Inverse of text_from_blocks. Produces a 1024-byte zero-filled header (the
// real Apple Pascal editor stores page-mapping metadata there, but
// text_from_blocks skips those bytes unconditionally without interpreting
// them, so files this tool writes read back correctly through this tool and
// open in the real editor, just without fast paging) followed by the
// encoded text, zero-padded to a whole number of 512-byte blocks.
//
// 0x00, 0x0d, and 0x10 are reserved by the on-disk encoding (null padding,
// line terminator, and RLE-indent marker respectively) and can't be
// represented as literal content, so input containing any of them is
// rejected rather than silently written in a form that won't decode back
// to the original bytes.
pub fn text_to_blocks(text: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut result = vec![0u8; 1024];
    let mut i = 0;
    let mut at_line_start = true;
    while i < text.len() {
        if at_line_start {
            let mut count = 0;
            while i + count < text.len() && text[i + count] == 0x20 {
                count += 1;
            }
            at_line_start = false;
            if count > 0 {
                let mut remaining = count;
                while remaining > 0 {
                    // A single count byte tops out at 255-32=223.
                    let chunk = remaining.min(223);
                    result.push(0x10);
                    result.push((chunk + 32) as u8);
                    remaining -= chunk;
                }
                i += count;
                continue;
            }
        }
        let byte = text[i];
        match byte {
            0x0a => {
                result.push(0x0d); // convert LF to CR
                at_line_start = true;
            }
            0x00 | 0x0d | 0x10 => {
                anyhow::bail!(
                    "input contains byte {byte:#04x}, which is reserved by the \
                     p-System text encoding and can't be represented in a \
                     text-mode transfer (retry without --text)"
                );
            }
            _ => result.push(byte),
        }
        i += 1;
    }
    super::directory::pad_to_block_boundary(&mut result);
    Ok(result)
}

pub fn text_from_blocks(buffer: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut skip_next = false;
    for i in 1024..buffer.len() {
        let byte = buffer[i];
        if skip_next {
            skip_next = false;
            continue;
        }
        if byte == 0x0d {
            result.push(0x0a); // convert CR to LF
        } else if byte == 0x10 {
            // A well-formed marker's count byte is always present and >= 32;
            // treat anything else (truncated buffer, or a stray 0x10 that
            // was never actually a marker) as literal content rather than
            // indexing out of bounds or underflowing the subtraction.
            match buffer
                .get(i + 1)
                .and_then(|b| (*b as usize).checked_sub(32))
            {
                Some(space_count) => {
                    result.resize(result.len() + space_count, 0x20); // emit spaces for indent
                    skip_next = true; // skip the next byte
                }
                None => result.push(byte),
            }
        } else if byte == 0 {
            continue; // skip null bytes
        } else {
            result.push(byte);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apple_disk::AppleDisk;
    use crate::disk_image::DiskImage;

    fn fixture_path(name: &str) -> String {
        format!("{}/../tests/AppleDsks/{name}", env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn blog_disk_text_simple() {
        let disk = AppleDisk::from_file(&fixture_path("blog.dsk"), false).unwrap();
        // SHORT.TEXT: first_block=148, first_after_block=152
        let file_buffer = disk.read_blocks(148, 4);
        let text = text_from_blocks(file_buffer);
        assert_eq!(
            String::from_utf8(text).unwrap(),
            "This is about as simple as it gets.\nA couple of lines,\n\nAnd two paragraphs.\n"
        );
    }

    #[test]
    fn blog_disk_text_indented() {
        let disk = AppleDisk::from_file(&fixture_path("blog.dsk"), false).unwrap();
        // INDENTS.TEXT: first_block=156, first_after_block=160. Exercises the
        // run-length-encoded indentation (0x10 marker) decode path.
        let file_buffer = disk.read_blocks(156, 4);
        let text = text_from_blocks(file_buffer);
        let expected = "This is about as simple as it gets.\n\
             \u{20}A couple of lines.\n\
             \u{20}\u{20}Each further indented,\n\
             \u{20}\u{20}\u{20}Slowly,\n\
             \u{20}\u{20}\u{20}\u{20}Inexorably,\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}Approaching the right margin\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}I guess at the limit, we'd hit 'p'\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}But I don't have the patience...\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}Eight\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}Nine\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}Ten\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}Eleven\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}Twelve\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}Thirteen\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}Fourteen\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}Fifteen\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}Sixteen\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\n";
        assert_eq!(String::from_utf8(text).unwrap(), expected);
    }

    #[test]
    fn text_round_trip_simple() {
        let text =
            "This is about as simple as it gets.\nA couple of lines,\n\nAnd two paragraphs.\n";
        let blocks = text_to_blocks(text.as_bytes()).unwrap();
        assert_eq!(blocks.len() % 512, 0);
        assert_eq!(String::from_utf8(text_from_blocks(&blocks)).unwrap(), text);
    }

    #[test]
    fn text_round_trip_indented() {
        let text = "This is about as simple as it gets.\n\
             \u{20}A couple of lines.\n\
             \u{20}\u{20}Each further indented,\n\
             \u{20}\u{20}\u{20}Slowly,\n";
        let blocks = text_to_blocks(text.as_bytes()).unwrap();
        assert_eq!(blocks.len() % 512, 0);
        assert_eq!(String::from_utf8(text_from_blocks(&blocks)).unwrap(), text);
    }

    #[test]
    fn text_round_trip_no_marker_when_no_leading_space() {
        // First line has no leading spaces (no marker emitted); second line
        // has exactly one leading space (marker emitted even for count==1).
        let text = "no indent\n\u{20}one space\n";
        let blocks = text_to_blocks(text.as_bytes()).unwrap();
        let content = &blocks[1024..];
        assert_eq!(&content[..9], b"no indent");
        assert_eq!(content[9], 0x0d); // CR
        assert_eq!(content[10], 0x10); // RLE marker
        assert_eq!(content[11], 0x20 + 1); // count == 1
        assert_eq!(String::from_utf8(text_from_blocks(&blocks)).unwrap(), text);
    }

    #[test]
    fn text_to_blocks_rejects_reserved_bytes() {
        assert!(text_to_blocks(b"has a null \x00 byte").is_err());
        assert!(text_to_blocks(b"has a bare CR \x0d byte").is_err());
        assert!(text_to_blocks(b"has a marker \x10 byte").is_err());
    }

    #[test]
    fn text_from_blocks_handles_truncated_marker_without_panicking() {
        // A 0x10 as the very last byte in the buffer: no count byte follows.
        let mut buffer = vec![0u8; 1024];
        buffer.push(0x10);
        let result = text_from_blocks(&buffer);
        assert_eq!(result, vec![0x10]);
    }

    #[test]
    fn text_from_blocks_handles_undersized_count_byte_without_underflowing() {
        // A 0x10 followed by a byte < 32: not a well-formed count byte.
        let mut buffer = vec![0u8; 1024];
        buffer.push(0x10);
        buffer.push(0x05);
        let result = text_from_blocks(&buffer);
        assert_eq!(result, vec![0x10, 0x05]);
    }
}
