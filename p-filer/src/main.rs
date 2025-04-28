use core::num;
use std::{fs, usize};
use clap::{Args, Parser, Subcommand};

/// Simple program to greet a person
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
    Dump
}

#[derive(Args, Debug)]
struct TransferArgs {
    name: String,
    #[arg(short, long)]
    to_image: bool
}

fn main() {
    let args = MainArgs::parse();
    let image = args.image;
    let d = AppleDisk::new(&image);
    match &args.command {
        Commands::List => d.list(),
        Commands::Remove { name } => d.remove(name),
        Commands::Transfer(args, ) => d.transfer(&args.name, args.to_image),
        Commands::Change { from, to } => d.change(from, to),
        Commands::Krunch => d.krunch(),
        Commands::Zero => d.zero(),
        Commands::Dump => d.dump()
    }
}


struct AppleDisk {
    image: String,
    buffer: Vec<u8>,
}

impl AppleDisk {
    pub fn read_block(&self, index: usize) -> &[u8] {
        let start:usize = index * 512;
        let end:usize = (index + 1) * 512;
        return &self.buffer[start..end]
    }

    pub fn num_blocks(&self) -> usize {
        return self.buffer.len() / 512
    }

    pub fn new(name: &str) -> Self {
        let mut new_self = Self {
            image: name.to_string(),
            buffer: Vec::new(),
        };
        new_self.fill_buffer();
        return new_self;
    }

    fn fill_buffer(& mut self) {
        //TODO: Don't assume we have enough memory to read the whole file at once here (but we totally do on the desktop)
        let contents: Vec<u8> = fs::read(&self.image) .expect("couldn't read file");
        self.buffer = Vec::with_capacity(contents.len());
        // Apple II .dsk files have interleaved sectors, so un-shuffle them
        let sector_map: [usize; 16] = [
            0, 14, 13, 12, 11, 10, 9, 8,
            7, 6, 5, 4, 3, 2, 1, 15
        ];
        let total_sectors = contents.len() / 256;
        let num_tracks = total_sectors / 16;
        println!("{num_tracks} tracks of 16 sectors = {total_sectors} sectors, {0} blocks", total_sectors/2);
        for track in 0..num_tracks {
            let track_offset = track * 16 * 256;
            //println!("track {track}, offset {track_offset}");
            for sector in 0..16 as usize {
                let sector2 = sector_map[sector];
                //println!("track: {track}, sector {sector2} -> {sector}");
                //let target_sector_offset = sector * 256 + track_offset;
                let source_sector_offset = sector2 * 256 + track_offset;
                //println!("");
                for byte in 0..256 as usize {
                    self.buffer.push(contents[source_sector_offset+byte]);
                }
            }
        }
        //println!("file len: {}, buffer len: {}", contents.len(), self.buffer.len());
        assert!(contents.len() == self.buffer.len());
    }

    fn list(&self) {
        println!("Listing files on {0}", self.image);
    }
    
    fn remove(&self, name: &str) {
        println!("Removing {name} on {0}", self.image);
    }
    
    fn transfer(&self, name: &str, to_image: bool) {
        if to_image {
            println!("Copying {name} to {0}", self.image);
        } else {
            println!("Copying {name} from {0}", self.image);
        }
    }
    
    fn change(&self, from: &str, to: &str) {
        println!("Renaming {from} to {to} on {0}", self.image);
        
    }
    
    fn krunch(&self) {
        println!("Consolidating free space on {0}", self.image);
    }
    
    fn zero(&self) {
        println!("Clearing directory on {0}", self.image);
    }

    fn dump(&self) {
        println!("Dumping contexts of {0}", self.image);
        let line_len = 16;
        for line in 0..self.buffer.len() / line_len {
            if line % 32 == 0 {
                println!();
            }
            print!("{:06x}  ", line * line_len);
            for byte in 0..line_len {
                print!("{:02x} ", self.buffer[line*line_len+byte]);
            }
            print!("  |");
            for byte in 0..line_len {
                let mut c = self.buffer[line*line_len+byte];
                if c < 32 || c > 126 {
                    c = 46;
                }
                print!("{}", char::from(c));
            }
            println!("|");
        }
    }
}
