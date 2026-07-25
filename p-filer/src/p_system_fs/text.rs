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
            let space_count = buffer[i+1] as usize - 32;
            for _ in 0..space_count {
                result.push(0x20); // emit spaces for indent
            }
            skip_next = true; // skip the next byte
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
        let disk = AppleDisk::from_file(&fixture_path("blog.dsk"));
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
        let disk = AppleDisk::from_file(&fixture_path("blog.dsk"));
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
}
