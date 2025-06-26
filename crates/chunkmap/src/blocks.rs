use crate::{biomes::BiomeData, utils::u32_to_rgb};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
};

pub const UNKNOWN_BLOCK_COLOR: [u8; 3] = [233, 66, 245];

pub fn get_block_color(
    name: &str,
    snowy: bool,
    biome_data: &BiomeData,
    block_colors: &HashMap<String, [u8; 3]>,
    unknown_blocks: &mut HashSet<String>,
) -> [u8; 3] {
    if snowy {
        return [255, 255, 255];
    }

    let clean_name = name.strip_prefix("minecraft:").unwrap_or(name);

    match clean_name {
        "air" | "cave_air" => [0, 0, 0],
        "grass_block" => u32_to_rgb(biome_data.grass_color),
        "water" => u32_to_rgb(biome_data.water_color),
        "lava" => [255, 100, 0],
        _ => {
            if clean_name.contains("leaves") {
                match clean_name {
                    "birch_leaves" | "spruce_leaves" | "cherry_leaves" => {}
                    _ => return u32_to_rgb(biome_data.foliage_color),
                }
            }

            if let Some(&color) = block_colors.get(clean_name) {
                color
            } else {
                unknown_blocks.insert(clean_name.to_string());
                UNKNOWN_BLOCK_COLOR
            }
        }
    }
}

// NOTE: Cannot use fs because of WASM support
pub const BLOCKS_JSON: &str = include_str!("../../../blocks.json");

pub fn load_block_colors() -> Result<HashMap<String, [u8; 3]>, Box<dyn Error>> {
    let raw_colors: HashMap<String, String> = serde_json::from_str(&BLOCKS_JSON)?;
    let mut block_colors = HashMap::new();

    for (block_name, hex_color) in raw_colors {
        let hex = hex_color.trim_start_matches('#');
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16)?;
            let g = u8::from_str_radix(&hex[2..4], 16)?;
            let b = u8::from_str_radix(&hex[4..6], 16)?;
            block_colors.insert(block_name, [r, g, b]);
        } else {
            return Err(format!("Invalid color found in the JSON: {hex_color}").into());
        }
    }

    Ok(block_colors)
}
