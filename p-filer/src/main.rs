use clap::{Args, Parser, Subcommand};
mod apple_disk;
mod disk_image;
mod error;
mod p_system_fs;
use apple_disk::AppleDisk;
use error::Error;
use p_system_fs::Volume;

/// A command-file tool for manipulating Apple Pascal disk images
#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct MainArgs {
    /// Name of disk image to use
    #[arg(short, long)]
    image: String,
    /// Print extra diagnostic information
    #[arg(short, long)]
    verbose: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    List,
    Remove { name: String },
    Transfer(TransferArgs),
    Change { from: String, to: String },
    Krunch,
    Zero { new_name: Option<String> },
    Dump { from: usize, to: usize },
}

#[derive(Args, Debug)]
struct TransferArgs {
    name: String,
    /// Destination, like `cp`'s second argument: on --to-image, overrides
    /// the volume filename (still uppercased and 15-character-limited)
    /// instead of deriving it from `name`'s basename; when extracting, an
    /// existing directory copies in under the volume entry's own name,
    /// otherwise it's used as the exact host path to write
    dest: Option<String>,
    #[arg(long)]
    to_image: bool,
    #[arg(long)]
    text: bool,
    #[arg(long, short)]
    preserve_date: bool,
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
    let verbose = args.verbose;
    let image = args.image;
    let mut d = Volume::new(AppleDisk::from_file(&image, verbose)?, image)?;
    match &args.command {
        Commands::List => d.list()?,
        Commands::Remove { name } => d.remove(name)?,
        Commands::Transfer(args) => d.transfer(
            &args.name,
            args.dest.as_deref(),
            args.to_image,
            args.text,
            args.preserve_date,
        )?,
        Commands::Change { from, to } => d.change(from, to)?,
        Commands::Krunch => d.krunch()?,
        Commands::Zero { new_name } => d.zero(new_name.as_deref())?,
        Commands::Dump { from, to } => d.dump(*from, *to)?,
    }
    Ok(())
}
