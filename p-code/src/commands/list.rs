use crate::segment_dictionary::{SegmentDictionary, string_from_segment_info};

pub fn run(file_name: String) -> anyhow::Result<()> {
    println!("Listing code file {file_name}");
    let contents = std::fs::read(file_name)?;
    let segment_dictionary = SegmentDictionary::parse(&contents)?;
    println!("File length: {}", contents.len());
    let copyright = String::from_utf8(segment_dictionary.copyright_string.to_vec())?;
    println!("Copyright: {}", copyright);
    println!("Segments:");
    for (s, code_info, seg_name) in segment_dictionary.active_segments() {
        let seg_kind = segment_dictionary.segment_kind(s)?;
        let text_addr = segment_dictionary.text_addr[s];
        let seg_info = segment_dictionary.seg_info[s];

        println!(
            "Segment {:#x?}, name: {}, address: {:#x?}, length: {:#x?},",
            s,
            seg_name,
            code_info.address * 512,
            code_info.length
        );
        println!(
            "\t kind: {:?}, text_addr: {:#x?}, seg_info: {:#x?}",
            seg_kind,
            text_addr,
            string_from_segment_info(seg_info)
        );
    }
    println!();
    Ok(())
}
