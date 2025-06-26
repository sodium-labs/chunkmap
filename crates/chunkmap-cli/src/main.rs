use crate::{
    cli::{Cli, Commands},
    render::render_regions,
};
use anvilregion::dimensions::Dimension;
use chunkmap::images::{create_map_image, ImageRenderType};
use clap::Parser;

mod cli;
mod render;

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Merge { folder, o } => {
            if !o.ends_with(".png") {
                panic!("Output must be a PNG file");
            }

            let image = match create_map_image(&folder) {
                Ok(v) => v,
                Err(e) => {
                    panic!("Failed to merge regions: {e:?}");
                }
            };

            image.save(&o).expect("Failed to save final image");

            println!("Merged regions in {o}");
        }
        Commands::Render { folder, o, d, r } => {
            let dimension = match d.as_str() {
                "overworld" => Dimension::Overworld,
                "nether" => Dimension::Nether,
                "end" => Dimension::End,
                _ => panic!("Invalid dimension. Allowed: overworld | nether | end"),
            };

            let render_type = match r.as_str() {
                "textures" => ImageRenderType::Textures,
                "texturesnowater" => ImageRenderType::TexturesWithoutWater,
                "heightmap" => ImageRenderType::Heightmap,
                "biomes" => ImageRenderType::Biomes,
                "temperature" => ImageRenderType::Temperature,
                "downfall" => ImageRenderType::Downfall,
                "inhabited" => ImageRenderType::Inhabited,
                "lastupdated" => ImageRenderType::LastUpdated,
                _ => panic!("Invalid render type. Allowed: textures | texturesnowater | heightmap | biomes | temperature | downfall | inhabited | lastupdated")
            };

            render_regions(&folder, &o, render_type, dimension);

            println!("All regions rendered");
        }
    }
}
