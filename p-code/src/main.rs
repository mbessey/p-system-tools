use clap::{Parser, Subcommand};
mod segment_dictionary;
mod commands;
mod disassembler;

/// A command-file tool for manipulating UCSD pascal object files
#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct MainArgs {
    /// Name of disk image to use
    #[arg(short, long)]
    code_file: String,
    #[command(subcommand)]
    command: Commands
}

#[derive(Subcommand)]
enum Commands {
    List,
    Disassemble,
}

fn main() {
    println!("size of SegmentDictionary is {}", std::mem::size_of::<segment_dictionary::SegmentDictionary>());
    let args = MainArgs::parse();
    let file_name = args.code_file;
    match &args.command {
        Commands::List => commands::list::run(file_name),
        Commands::Disassemble => commands::disassemble::run(file_name),
    }
}
