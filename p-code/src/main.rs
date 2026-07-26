use clap::{Parser, Subcommand};
mod commands;
mod disassembler;
mod segment_dictionary;

/// A command-file tool for manipulating UCSD pascal object files
#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct MainArgs {
    /// Name of disk image to use
    #[arg(short, long)]
    code_file: String,
    /// Print extra diagnostic information
    #[arg(short, long)]
    verbose: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    List,
    Disassemble {
        /// Show each instruction's offset within the segment
        #[arg(long)]
        offsets: bool,
        /// Show each instruction's raw hex bytes
        #[arg(long)]
        bytes: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let args = MainArgs::parse();
    if args.verbose {
        println!(
            "size of SegmentDictionary is {}",
            std::mem::size_of::<segment_dictionary::SegmentDictionary>()
        );
    }
    let file_name = args.code_file;
    match &args.command {
        Commands::List => commands::list::run(file_name)?,
        Commands::Disassemble { offsets, bytes } => {
            commands::disassemble::run(file_name, *offsets, *bytes)?
        }
    }
    Ok(())
}
