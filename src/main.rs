pub mod coordinate_stats;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Analyze coordinate statistics in the MOJ XML file
    CoordinateStats {
        /// Path to the directory containing zip files
        #[arg(short, long, value_name = "DIR")]
        path: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::CoordinateStats { path } => {
            println!("CoordinateStats command called for path: {:?}", path);
            // Call the function from the new module
            if let Err(e) = coordinate_stats::run_coordinate_stats(path) {
                eprintln!("Error processing coordinate stats: {}", e);
            }
        }
    }
}
