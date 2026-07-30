use clap::{Parser, Subcommand};
mod commands;
mod disassembler;
mod error;
mod segment_dictionary;
use error::Error;

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
        /// Turn off jump-target labels and resolved-target operand text
        /// (shown by default)
        #[arg(long)]
        no_labels: bool,
    },
}

fn main() -> std::process::ExitCode {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::ExitCode::FAILURE
    } else {
        std::process::ExitCode::SUCCESS
    }
}

fn run() -> Result<(), Error> {
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
        Commands::Disassemble {
            offsets,
            bytes,
            no_labels,
        } => commands::disassemble::run(file_name, *offsets, *bytes, !*no_labels)?,
    }
    Ok(())
}
