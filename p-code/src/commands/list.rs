use crate::segment_dictionary::{string_from_segment_info, SegmentDictionary};

pub fn run(file_name: String) {
    println!("Listing code file {file_name}");
    let contents = std::fs::read(file_name).expect("Unable to read file");
    let segment_dictionary = SegmentDictionary::new(&contents);
    println!("File length: {}", contents.len());
    let copyright = match String::from_utf8(segment_dictionary.copyright_string.to_vec()) {
        Ok(v) => v,
        Err(e) => panic!("{}", e),
    };
    println!("Copyright: {}", copyright);
    println!("Segments:");
    for (s, code_info, seg_name) in segment_dictionary.active_segments() {
        let seg_kind = segment_dictionary.seg_kind[s];
        let text_addr = segment_dictionary.text_addr[s];
        let seg_info = segment_dictionary.seg_info[s];

        println!("Segment {:#x?}, name: {}, address: {:#x?}, length: {:#x?},", s, seg_name, code_info.address*512, code_info.length);
        println!("\t kind: {:?}, text_addr: {:#x?}, seg_info: {:#x?}", seg_kind, text_addr, string_from_segment_info(seg_info));
    }
    println!();
}
