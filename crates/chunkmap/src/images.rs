use crate::{
    biomes::{get_biome_data, load_biomes_data},
    blocks::{get_block_color, load_block_colors},
    utils::{
        apply_blue_tint, chunk_to_region_coords, downfall_color, get_biome_index, height_color,
        linear_color, temperature_color, u32_to_rgb,
    },
};
use anvilregion::{
    chunks::{parse_chunk_surface, Chunk},
    dimensions::Dimension,
};
use image::{GenericImage, ImageBuffer, Rgba, RgbaImage};
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fs::{self},
    path::Path,
    time::SystemTime,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ImageRenderType {
    Textures,
    TexturesWithoutWater,
    Heightmap,
    Biomes,
    Temperature,
    Downfall,
    Inhabited,
    LastUpdated,
}

/// Create a region image from its chunks.
/// Can create multiple images if all the chunks are not in the same region
pub fn create_region_images(
    chunks: &Vec<Chunk>,
    dimension: &Dimension,
    render_type: &ImageRenderType,
) -> Result<Vec<(i32, i32, ImageBuffer<Rgba<u8>, Vec<u8>>)>, Box<dyn Error>> {
    let block_colors = load_block_colors()?;
    let biomes_data = load_biomes_data()?;

    // Group by region

    let mut regions: HashMap<(i32, i32), Vec<&Chunk>> = HashMap::new();
    for chunk in chunks {
        let region_coords = chunk_to_region_coords(chunk.position.x, chunk.position.z);
        regions.entry(region_coords).or_default().push(chunk);
    }

    // Generate images

    let mut images = Vec::new();

    for ((rx, rz), region_chunks) in &regions {
        let min_x = rx * 32;
        let min_z = rz * 32;
        let width = 32 * 16;
        let height = 32 * 16;

        let mut unknown_blocks = HashSet::new();
        let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(width as u32, height as u32, Rgba([0, 0, 0, 0]));

        for chunk in region_chunks {
            let surface = match parse_chunk_surface(chunk, dimension) {
                Ok(surface) => surface,
                Err(e) => return Err(e),
            };

            let mut block_map = HashMap::new();
            for b in &surface.blocks {
                block_map.insert((b.position.x, b.position.z), b.position.y);
            }

            for block in surface.blocks {
                let local_x = block.position.x - min_x * 16;
                let local_z = block.position.z - min_z * 16;
                if local_x < 0 || local_x >= width || local_z < 0 || local_z >= height {
                    continue;
                }

                let pixel_x = local_x as u32;
                let pixel_y = local_z as u32;
                let chunk_local_x = block.position.x & 0xF;
                let chunk_local_z = block.position.z & 0xF;

                let color = if render_type == &ImageRenderType::Heightmap {
                    height_color(block.position.y, dimension)
                } else {
                    let biome_index = get_biome_index(chunk_local_x, chunk_local_z);
                    let biome_name = &surface.biomes[biome_index]
                        .strip_prefix("minecraft:")
                        .unwrap();
                    let biome_data = get_biome_data(&biomes_data, biome_name);

                    match render_type {
                        ImageRenderType::Textures | ImageRenderType::TexturesWithoutWater => {
                            let mut color = get_block_color(
                                &block.name,
                                block.snowy,
                                biome_data,
                                &block_colors,
                                &mut unknown_blocks,
                            );

                            if render_type == &ImageRenderType::Textures {
                                if block.depth > 0 {
                                    color = apply_blue_tint(
                                        color,
                                        block.depth,
                                        u32_to_rgb(biome_data.water_color),
                                    );
                                }
                            }

                            if block.depth == 0 {
                                // 3d effect with black and white shadows

                                let above_y =
                                    block_map.get(&(block.position.x, block.position.z - 1));
                                let below_y =
                                    block_map.get(&(block.position.x, block.position.z + 1));

                                if let (Some(&above), Some(&below)) = (above_y, below_y) {
                                    if above > block.position.y {
                                        let v = (above - block.position.y).min(3);
                                        // Black tint
                                        for _ in 0..v {
                                            for c in color.iter_mut() {
                                                *c = (*c as f32 * 0.8) as u8;
                                            }
                                        }
                                    } else if below > block.position.y {
                                        let v = (below - block.position.y).min(3);
                                        // White tint
                                        for _ in 0..v {
                                            for c in color.iter_mut() {
                                                *c = ((*c as f32) * 0.9 + 255.0 * 0.1) as u8;
                                            }
                                        }
                                    }
                                }
                            }

                            color
                        }
                        ImageRenderType::Temperature => temperature_color(biome_data.temperature),
                        ImageRenderType::Downfall => downfall_color(biome_data.downfall),
                        ImageRenderType::Biomes => u32_to_rgb(biome_data.grass_color),
                        ImageRenderType::Inhabited => {
                            linear_color(chunk.inhabited_time as f32, 0.0, 1_600_000.0)
                        }
                        ImageRenderType::LastUpdated => {
                            let now = SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis() as f32;
                            linear_color(
                                chunk.last_update as f32,
                                now - 365.0 * 24.0 * 60.0 * 60.0 * 1000.0,
                                now,
                            )
                        }
                        ImageRenderType::Heightmap => unreachable!("Heightmap render type"),
                    }
                };

                img.put_pixel(pixel_x, pixel_y, Rgba([color[0], color[1], color[2], 255]));
            }
        }

        if !unknown_blocks.is_empty() {
            eprintln!("Unknown blocks found: {unknown_blocks:?}");
            eprintln!("");
        }

        images.push((*rx, *rz, img));
    }

    Ok(images)
}

/// Merge all regions images from the folder
pub fn create_map_image(folder: &str) -> Result<RgbaImage, Box<dyn Error>> {
    // Validate folder
    let folder_path = Path::new(folder);
    if !folder_path.is_dir() {
        return Err(format!("Provided path '{}' is not a valid directory.", folder).into());
    }

    // Regex for r.x.z.png
    let re = Regex::new(r"^r\.(-?\d+)\.(-?\d+)\.png$")?;

    // Collect region images and their coordinates
    let mut regions = Vec::new();
    for entry in fs::read_dir(folder_path)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let fname = match path.file_name().and_then(|n| n.to_str()) {
            Some(f) => f,
            None => continue,
        };
        if !fname.ends_with(".png") {
            continue;
        }
        let caps = match re.captures(fname) {
            Some(c) => c,
            None => continue,
        };
        let x: i32 = caps[1].parse()?;
        let z: i32 = caps[2].parse()?;
        let metadata = fs::metadata(&path)?;
        if metadata.len() == 0 {
            continue;
        }
        let img = image::open(&path)?.to_rgba8();
        regions.push(((x, z), img));
    }

    if regions.is_empty() {
        return Err("No valid region images found.".into());
    }

    // Find bounds
    let (min_x, max_x, min_z, max_z) = regions.iter().fold(
        (i32::MAX, i32::MIN, i32::MAX, i32::MIN),
        |(min_x, max_x, min_z, max_z), ((x, z), _)| {
            (min_x.min(*x), max_x.max(*x), min_z.min(*z), max_z.max(*z))
        },
    );

    // Assume all regions are the same size
    let region_width = regions[0].1.width();
    let region_height = regions[0].1.height();

    let map_width = ((max_x - min_x + 1) as u32) * region_width;
    let map_height = ((max_z - min_z + 1) as u32) * region_height;

    let mut map_img = RgbaImage::from_pixel(map_width, map_height, image::Rgba([0, 0, 0, 0]));

    for ((x, z), img) in regions {
        let offset_x = ((x - min_x) as u32) * region_width;
        let offset_y = ((z - min_z) as u32) * region_height;
        map_img.copy_from(&img, offset_x, offset_y)?;
    }

    Ok(map_img)
}
