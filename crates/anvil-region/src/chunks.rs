use crate::{
    blocks::{Block, BlockPosition},
    dimensions::{get_dimension_height_offset, get_dimension_heights, Dimension},
    heightmaps::{decode_heightmap, Heightmap},
    nbt::Compound,
    sections::{get_biome_at_position, get_block_at_position},
};
use fastnbt::Value;
use std::{collections::HashMap, error::Error};

#[derive(Debug, Clone)]
pub struct ChunkPosition {
    pub x: i32,
    pub z: i32,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub data_version: i32,
    pub last_update: i64,
    pub inhabited_time: i64,
    pub position: ChunkPosition,
    pub nbt: Value,
}

#[derive(Debug, Clone)]
pub struct ChunkSurface {
    /// Highest 256 blocks (16x16) stored in YZX order
    pub blocks: Vec<Block>,
    /// 16 biomes (4x4) stored in YZX order
    pub biomes: Vec<String>,
}

/// Get the world surface and ocean floor heightmaps
pub fn parse_chunk_heightmaps(root: &Compound) -> Result<(Heightmap, Heightmap), String> {
    let heightmaps = match root.get("Heightmaps") {
        Some(Value::Compound(v)) => v,
        _ => return Err("'Heightmaps' not found or not a Compound".into()),
    };

    let motion_blocking_raw = match heightmaps.get("MOTION_BLOCKING") {
        Some(Value::LongArray(array)) => array,
        _ => return Err("'MOTION_BLOCKING' not found or not a LongArray".into()),
    };

    let ocean_floor_raw = match heightmaps.get("OCEAN_FLOOR") {
        Some(Value::LongArray(array)) => array,
        _ => return Err("'OCEAN_FLOOR' not found or not a LongArray".into()),
    };

    let motion_blocking_heightmap = decode_heightmap(motion_blocking_raw)?;
    let ocean_floor_heightmap = decode_heightmap(ocean_floor_raw)?;

    Ok((motion_blocking_heightmap, ocean_floor_heightmap))
}

pub fn parse_chunk_sections(root: &Compound) -> Result<HashMap<i32, &Compound>, String> {
    let sections = match root.get("sections") {
        Some(Value::List(v)) => v,
        _ => return Err("'sections' not found or not a list".into()),
    };

    let mut sorted_sections = HashMap::new();

    for section in sections {
        match section {
            Value::Compound(section_compound) => {
                let y_pos = match section_compound.get("Y") {
                    Some(Value::Byte(y)) => *y as i32,
                    Some(Value::Int(y)) => *y,
                    v => return Err(format!("'section' has invalid 'Y' value. Got {v:#?}")),
                };

                sorted_sections.insert(y_pos, section_compound);
            }
            v => return Err(format!("'section' is not a Compound. Got {v:#?}")),
        }
    }

    Ok(sorted_sections)
}

/// Parse the highest blocks of the chunk and their biomes
pub fn parse_chunk_surface(
    chunk: &Chunk,
    dimension: &Dimension,
) -> Result<ChunkSurface, Box<dyn Error>> {
    let mut highest_blocks = Vec::new();
    let mut highest_biomes = Vec::new();

    let root = match &chunk.nbt {
        Value::Compound(v) => v,
        _ => return Err("Root NBT is not a compound".into()),
    };

    let (mb_heightmap, of_heightmap) = parse_chunk_heightmaps(root)?;
    let sections = parse_chunk_sections(root)?;
    let dimension_offset = get_dimension_height_offset(dimension);

    let (min_y, max_y) = if dimension == &Dimension::Nether {
        // In the nether we only care about the blocks between lava level (31) and the bedrock roof
        (31, 127)
    } else {
        get_dimension_heights(dimension)
    };

    let mut nether_columns: Option<Vec<Vec<Option<String>>>> = None;
    // None = dynamic height (much slower)
    // Some = take only blocks at this level (essentially a slice of the nether)
    // TODO: make this configurable
    let nether_fixed_height: Option<i32> = None;

    // TODO: move that
    let min_nether = 31i32;
    let max_nether = 127i32;

    // If we are in the nether, precompute the block columns for later
    if dimension == &Dimension::Nether && nether_fixed_height.is_none() {
        let height_span = (max_nether - min_nether + 1) as usize;
        let mut columns = vec![vec![None; height_span]; 16 * 16];

        let min_section = min_nether.div_euclid(16);
        let max_section = max_nether.div_euclid(16);

        for section_y in min_section..=max_section {
            if let Some(section) = sections.get(&section_y) {
                for local_y in 0..16 {
                    let global_y = section_y * 16 + local_y;
                    if global_y < min_nether || global_y > max_nether {
                        continue;
                    }
                    let idx_y = (global_y - min_nether) as usize;
                    for local_z_iter in 0..16 {
                        for local_x_iter in 0..16 {
                            let col_idx = (local_z_iter * 16 + local_x_iter) as usize;
                            let (block_name, _) = get_block_at_position(
                                section,
                                local_x_iter as usize,
                                local_y as usize,
                                local_z_iter as usize,
                            )?;
                            columns[col_idx][idx_y] = Some(block_name);
                        }
                    }
                }
            }
        }

        nether_columns = Some(columns);
    }

    // Iterate in a chunk square and find each surface block
    for local_z in 0..16i32 {
        for local_x in 0..16i32 {
            let height_index = (local_z * 16 + local_x) as usize;

            let world_x = chunk.position.x * 16 + local_x;
            let world_z = chunk.position.z * 16 + local_z;

            let mut surface_y = mb_heightmap[height_index] as i32 - 1 + dimension_offset;
            let mut ocean_y = of_heightmap[height_index] as i32 - 1 + dimension_offset;

            surface_y = surface_y.max(min_y).min(max_y);
            ocean_y = ocean_y.max(min_y).min(max_y);

            if dimension != &Dimension::Overworld {
                // TODO: There can be ocean in the End too?
                ocean_y = surface_y;
            }

            // Nether: start at bedrock roof (max_nether), search downward for first air block;
            // continue downward until a non-air block; If no air was found first,
            // or no non-air block was found in the second step, use block at level 31 (min_nether)
            // TODO: performance
            if dimension == &Dimension::Nether {
                if let Some(height) = nether_fixed_height {
                    surface_y = height;
                    ocean_y = height;
                } else {
                    let columns = nether_columns.as_ref().expect("nether columns built");
                    let col_index = (local_z * 16 + local_x) as usize;
                    let col = &columns[col_index];

                    let mut chosen_y: Option<i32> = None;

                    for y_check in (min_nether..=max_nether).rev() {
                        let idx = (y_check - min_nether) as usize;
                        if let Some(name) = &col[idx] {
                            let is_air =
                                name == "minecraft:air" || name == "air" || name.ends_with("_air");
                            if is_air {
                                let mut found_block_y: Option<i32> = None;
                                for y_block in (min_nether..=(y_check - 1)).rev() {
                                    let idx_block = (y_block - min_nether) as usize;
                                    if let Some(name_block) = &col[idx_block] {
                                        let is_air_block = name_block == "minecraft:air"
                                            || name_block == "air"
                                            || name_block.ends_with("_air");
                                        if !is_air_block {
                                            found_block_y = Some(y_block);
                                            break;
                                        }
                                    }
                                }
                                chosen_y = Some(found_block_y.unwrap_or(min_nether));
                                break;
                            }
                        }
                    }

                    let selected = chosen_y.unwrap_or(min_nether);
                    surface_y = selected;
                    ocean_y = selected;
                }
            }

            // TODO: ocean_y should be renamed.
            // `surface_y` can be higher if there is an ocean (`ocean_y` being the ocean floor y).

            // Section y in the chunk
            let section_y = ocean_y.div_euclid(16);
            // Local Y position in the section (0-15)
            let local_y = ((ocean_y % 16) + 16) % 16;

            match sections.get(&section_y) {
                Some(section) => {
                    // Biomes are divided in 4x4 cells
                    if local_x % 4 == 0 && local_z % 4 == 0 {
                        let biome_name = get_biome_at_position(
                            section,
                            local_x as usize,
                            local_y as usize,
                            local_z as usize,
                        )?;

                        highest_biomes.push(biome_name);
                    }

                    let (block_name, block_props) = get_block_at_position(
                        section,
                        local_x as usize,
                        local_y as usize,
                        local_z as usize,
                    )?;

                    let depth = if surface_y > ocean_y {
                        (surface_y - ocean_y) as u16
                    } else {
                        0
                    };

                    let snowy = if let Some(props) = block_props {
                        let v = props.get("snowy").map_or("false", |v| v);
                        v == "true"
                    } else {
                        false
                    };

                    highest_blocks.push(Block {
                        position: BlockPosition {
                            x: world_x,
                            y: ocean_y,
                            z: world_z,
                        },
                        name: block_name,
                        depth,
                        snowy,
                    });
                }
                None => {
                    return Err(format!("Section Y={section_y} missing").into());
                }
            }
        }
    }

    Ok(ChunkSurface {
        blocks: highest_blocks,
        biomes: highest_biomes,
    })
}
