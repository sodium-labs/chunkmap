use clap::{command, Parser};

#[derive(Parser, Debug)]
#[command(name = "chunkmap")]
#[command(about = "A tool to render Minecraft chunks", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Parser, Debug)]
pub enum Commands {
    /// Merge chunk images into a single file
    Merge {
        /// Input folder containing chunk images
        folder: String,
        /// Output file (PNG)
        #[arg(short, long)]
        o: String,
    },
    /// Render chunk data into images
    Render {
        /// Input folder containing chunk data
        folder: String,
        /// Output directory
        #[arg(short, long)]
        o: String,
        /// Dimension to render
        #[arg(short, long, value_parser = clap::builder::PossibleValuesParser::new(["overworld", "nether", "end"]))]
        d: String,
        /// Render mode
        #[arg(short, long, value_parser = clap::builder::PossibleValuesParser::new([
            "textures",
            "texturesnowater",
            "heightmap",
            "biomes",
            "temperature",
            "downfall",
            "inhabited",
            "lastupdated"
        ]))]
        r: String,
    },
}
