// Builds the 1024-byte editor header every p-System text file starts with
// (the real Editor's page-mapping/margin/place-marker state, restored when
// the file is reopened). Field layout per
// https://markbessey.blog/2025/05/08/ucsd-pascal-in-depth-3-n/, which
// reverse-engineered it by hex-dumping real Editor-authored files -- this
// project has no other authoritative source for it. Only the first 512
// bytes carry fields; the second 512 bytes are reserved and always zero.
//
// A prior version of this function left the whole header zero-filled,
// on the (untested, and apparently wrong) assumption that
// `text_from_blocks` -- which really does ignore this region
// unconditionally -- meant the real Editor would too. It doesn't: opening
// a file written that way could leave the real Editor's screen stuck in
// reverse video with nothing displayed. `right_margin: 0` is the leading
// suspect (a word-wrap column of zero is a plausible way to break the
// Editor's line-layout math), so that's the one field this function is
// confident is actively dangerous to leave at zero; the others below are
// filled in with reasonable, low-risk defaults (matching what an unused
// feature's "off" state should look like) rather than confirmed-correct
// values, since nothing in this project has verified them against a real
// Editor session yet.
fn text_file_header() -> [u8; 1024] {
    let mut header = [0u8; 1024];
    header[0..2].copy_from_slice(&1u16.to_le_bytes()); // version -- observed to always be 1
    // marker_count (2..4), marker_labels (4..84), the 10 unused bytes
    // (84..94), and marker_positions (94..114) all stay zero: this tool
    // has no notion of Editor place-markers, so "zero markers defined" is
    // the correct state to write, not a guess.
    // auto_indent (114..116), fill (116..118), and token (118..120) are
    // boolean feature flags -- zero/off is a safe default for all three.
    // left_margin (120..122) and para_margin (124..126) at zero are
    // ordinary, unremarkable states (text starts at column 0, no extra
    // paragraph indent) -- ordinary defaults, not zero-is-dangerous cases.
    header[122..124].copy_from_slice(&80u16.to_le_bytes()); // right_margin -- 0 is the suspected freeze trigger; 80 matches this project's own line-length convention elsewhere
    header[126..128].copy_from_slice(&0x5eu16.to_le_bytes()); // command_char -- '^', the UCSD Pascal Editor's documented command-prefix character
    // date_created (128..130) and date_last_used (130..132) stay zero
    // (an unset/null p-System date); filler (132..512) and the entire
    // second 512-byte block (512..1024) are reserved and always zero.
    header
}

// Inverse of text_from_blocks: builds the 1024-byte editor header (see
// text_file_header) followed by the encoded text, zero-padded to a whole
// number of 512-byte blocks.
//
// 0x00, 0x0d, and 0x10 are reserved by the on-disk encoding (null padding,
// line terminator, and RLE-indent marker respectively) and can't be
// represented as literal content, so input containing any of them is
// rejected rather than silently written in a form that won't decode back
// to the original bytes.
//
// Returns just the padded block bytes ready to write to disk. Unlike a
// binary/data-file transfer, the caller doesn't need the real pre-padding
// content length: every text file on real Apple Pascal media (confirmed
// against tests/AppleDsks/blog.dsk and a real Editor-authored 0-byte file)
// reports a full block (512) in its directory entry's `bytes_in_last_block`
// regardless of actual content length. Text files find their own end by
// scanning for CR/RLE markers, not via that field, so the real Filer/Editor
// apparently never bothers computing a real value for it.
pub fn text_to_blocks(text: &[u8]) -> Result<Vec<u8>, crate::error::Error> {
    let mut result = text_file_header().to_vec();
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
                return Err(crate::error::Error::ReservedTextByte { byte });
            }
            _ => result.push(byte),
        }
        i += 1;
    }
    super::directory::pad_to_block_boundary(&mut result);
    // The real Apple Pascal Filer/Editor always allocates at least two
    // 512-byte content blocks for a text file, no matter how short --
    // confirmed by hex-dumping real Editor-created files: even a 0-byte
    // file gets a lone CR in its first content block and a second content
    // block that's entirely unused and zero-filled. A file this tool
    // wrote with only one content block (short enough that padding to a
    // block boundary above didn't reach a second one) reliably locked up
    // the real Editor's screen in reverse video with nothing displayed;
    // matching the real minimum block count fixes it. The header (1024
    // bytes) doesn't count as a content block, so the floor is
    // 1024 + 2*BLOCK_SIZE, not 2*BLOCK_SIZE.
    let min_len = 1024 + 2 * super::directory::BLOCK_SIZE;
    if result.len() < min_len {
        result.resize(min_len, 0);
    }
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
    fn header_sets_documented_fields_not_all_zero() {
        // Regression test: this header used to be all-zero, which is
        // believed to be why opening a tool-written file in the real
        // Editor could leave its screen stuck in reverse video. Field
        // offsets per
        // https://markbessey.blog/2025/05/08/ucsd-pascal-in-depth-3-n/.
        let header = text_file_header();
        assert_eq!(u16::from_le_bytes([header[0], header[1]]), 1); // version
        assert_eq!(u16::from_le_bytes([header[2], header[3]]), 0); // marker_count
        assert_eq!(&header[4..84], &[0u8; 80][..]); // marker_labels, unused
        assert_eq!(u16::from_le_bytes([header[122], header[123]]), 80); // right_margin
        assert_eq!(u16::from_le_bytes([header[126], header[127]]), 0x5e); // command_char, '^'
        assert_eq!(&header[132..512], &[0u8; 380][..]); // filler
        assert_eq!(&header[512..1024], &[0u8; 512][..]); // second block, reserved
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
    fn text_to_blocks_always_allocates_at_least_two_content_blocks() {
        // Regression test for a real, confirmed bug: this tool used to
        // allocate only as many content blocks as the encoded content
        // needed, which could be just one for a short file. Hex-dumping
        // real Editor-created files (including a literal 0-byte one)
        // showed the real Apple Pascal Filer/Editor always allocates at
        // least two 512-byte content blocks, no matter how little content
        // there is -- a tool-written file with only one reliably locked
        // up the real Editor's screen in reverse video. `text_to_blocks`
        // must never return fewer than 1024 (header) + 2*512 (content) =
        // 2048 bytes, even for empty input.
        let blocks = text_to_blocks(b"").unwrap();
        assert_eq!(blocks.len(), 2048);
        assert_eq!(&blocks[1024..], &[0u8; 1024][..]); // both content blocks empty

        let blocks = text_to_blocks(b"short\n").unwrap();
        assert_eq!(blocks.len(), 2048);

        // A file whose content genuinely needs more than 2 content blocks
        // must not be truncated or shrunk back down to the minimum.
        let long_text = "x\n".repeat(1000);
        let blocks = text_to_blocks(long_text.as_bytes()).unwrap();
        assert!(blocks.len() > 2048);
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
