use std::usize;
use clap::{Args, Parser, Subcommand};
mod p_system_fs;
use p_system_fs::AppleDisk;

/// A command-file tool for manipulating Apple Pascal disk images
#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct MainArgs {
    /// Name of disk image to use
    #[arg(short, long)]
    image: String,
    #[command(subcommand)]
    command: Commands
}

#[derive(Subcommand)]
enum Commands {
    List,
    Remove {name: String},
    Transfer(TransferArgs),
    Change {from: String, to: String},
    Krunch,
    Zero,
    Dump {from: usize, to: usize} 
}

#[derive(Args, Debug)]
struct TransferArgs {
    name: String,
    #[arg(long)]
    to_image: bool,
    #[arg(long)]
    text: bool
}

fn main() {
    let args = MainArgs::parse();
    let image = args.image;
    let d = AppleDisk::new(&image);
    match &args.command {
        Commands::List => d.list(),
        Commands::Remove { name } => d.remove(name),
        Commands::Transfer(args, ) => d.transfer(&args.name, args.to_image, args.text),
        Commands::Change { from, to } => d.change(from, to),
        Commands::Krunch => d.krunch(),
        Commands::Zero => d.zero(),
        Commands::Dump { from, to } => d.dump(*from, *to)
    }
}
