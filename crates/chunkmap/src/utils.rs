use anvilregion::dimensions::{get_dimension_heights, Dimension};

pub fn u32_to_rgb(color: u32) -> [u8; 3] {
    [
        ((color >> 16) & 0xFF) as u8, // Red
        ((color >> 8) & 0xFF) as u8,  // Green
        (color & 0xFF) as u8,         // Blue
    ]
}

pub fn chunk_to_region_coords(x: i32, z: i32) -> (i32, i32) {
    (x.div_euclid(32), z.div_euclid(32))
}

pub fn get_biome_index(local_x: i32, local_z: i32) -> usize {
    let cell_x = local_x / 4;
    let cell_z = local_z / 4;
    let biome_index = cell_z * 4 + cell_x;
    biome_index as usize
}

pub fn apply_blue_tint(rgb: [u8; 3], depth: u16, water_color: [u8; 3]) -> [u8; 3] {
    let alpha = depth_to_alpha(depth);
    let mut tinted_color = [0u8; 3];

    for i in 0..3 {
        let orig = rgb[i] as f32;
        let blue_chan = water_color[i] as f32;
        let val = (1.0 - alpha) * orig + alpha * blue_chan;
        tinted_color[i] = val.round().clamp(0.0, 255.0) as u8;
    }

    tinted_color
}

/// Returns a number in the range [0, 0.7]
pub fn depth_to_alpha(depth: u16) -> f32 {
    match depth {
        0 => 0.0,
        1..=9 => 0.4 + 0.3 * (depth as f32 / 10.0),
        _ => 0.7,
    }
}

pub fn height_color(value: i32, dimension: &Dimension) -> [u8; 3] {
    let (min, max) = get_dimension_heights(dimension);
    let clamped_value = value.max(min).min(max);
    let normalized_value = (clamped_value + min.abs()) as f32 / (min.abs() + max) as f32;

    [(normalized_value * 255.0) as u8; 3]
}

/// value range: [-1;2]
pub fn temperature_color(value: f32) -> [u8; 3] {
    let cold = [64.0, 125.0, 237.0];
    let warm = [250.0, 118.0, 77.0];

    let clamped_value = value.max(-1.0).min(2.0);
    let t = (clamped_value + 1.0) / (1.0 + 2.0);

    let r = (cold[0] * (1.0 - t) + warm[0] * t).round() as u8;
    let g = (cold[1] * (1.0 - t) + warm[1] * t).round() as u8;
    let b = (cold[2] * (1.0 - t) + warm[2] * t).round() as u8;

    [r, g, b]
}

pub fn downfall_color(value: f32) -> [u8; 3] {
    let cold = [0.0, 0.0, 0.0];
    let warm = [255.0, 255.0, 255.0];

    let t = value.max(0.0).min(1.0);

    let r = (cold[0] * (1.0 - t) + warm[0] * t).round() as u8;
    let g = (cold[1] * (1.0 - t) + warm[1] * t).round() as u8;
    let b = (cold[2] * (1.0 - t) + warm[2] * t).round() as u8;

    [r, g, b]
}

pub fn linear_color(value: f32, min: f32, max: f32) -> [u8; 3] {
    let cold = [0.0, 0.0, 0.0];
    let warm = [255.0, 255.0, 255.0];

    let clamped_value = value.max(min).min(max);
    let t = (clamped_value + min.abs()) / (min.abs() + max.abs());

    let r = (cold[0] * (1.0 - t) + warm[0] * t).round() as u8;
    let g = (cold[1] * (1.0 - t) + warm[1] * t).round() as u8;
    let b = (cold[2] * (1.0 - t) + warm[2] * t).round() as u8;

    [r, g, b]
}
