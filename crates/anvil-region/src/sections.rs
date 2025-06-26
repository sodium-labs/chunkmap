use crate::nbt::Compound;
use fastnbt::Value;
use std::collections::HashMap;

pub fn get_biome_at_position(
    section: &Compound,
    x: usize,
    y: usize,
    z: usize,
) -> Result<String, String> {
    let biomes = match section.get("biomes") {
        Some(Value::Compound(v)) => v,
        v => {
            return Err(
                format!("'biomes' not found in section or not a Compound. Got {v:?}").into(),
            )
        }
    };

    let palette = match biomes.get("palette") {
        Some(Value::List(v)) => v,
        v => {
            return Err(
                format!("'biomes.palette' not found in section or not a List. Got {v:?}").into(),
            )
        }
    };

    // If the palette contains only one element, it is always this biome

    if palette.len() == 1 {
        return match &palette[0] {
            Value::String(name) => Ok(name.clone()),
            v => {
                return Err(
                    format!("The biome name in the palette is not a String. Got {v:?}").into(),
                )
            }
        };
    }

    // Otherwise we calculate the palette index

    let data = match biomes.get("data") {
        Some(Value::LongArray(v)) => v,
        v => return Err(format!("'biomes.data' not found or not a LongArray. Got {v:?}").into()),
    };

    let cell_x = x / 4;
    let cell_y = y / 4;
    let cell_z = z / 4;
    let biome_index = (cell_y * 4 + cell_z) * 4 + cell_x; // Y-Z-X, 4x4x4 = 64
    let bits_per_entry = (palette.len() as f64).log2().ceil() as usize;
    let palette_index = extract_palette_index(data, biome_index, bits_per_entry)?;

    if palette_index >= palette.len() {
        return Err(format!(
            "Invalid biome palette index: got {}, palette size is {}, bits_per_entry is {}, biome_index is {}",
            palette_index, palette.len(), bits_per_entry, biome_index
        ).into());
    }

    match &palette[palette_index] {
        Value::String(name) => Ok(name.clone()),
        v => {
            return Err(format!(
                "The biome name at index {palette_index} in the palette is not a String. Got {v:?}"
            )
            .into())
        }
    }
}

pub fn get_block_at_position(
    section: &Compound,
    x: usize,
    y: usize,
    z: usize,
) -> Result<(String, Option<HashMap<String, String>>), String> {
    let block_states = match section.get("block_states") {
        Some(Value::Compound(v)) => v,
        v => {
            return Err(
                format!("'block_states' not found in section or not a Compound. Got {v:?}").into(),
            )
        }
    };

    let palette = match block_states.get("palette") {
        Some(Value::List(v)) => v,
        v => {
            return Err(format!(
                "'block_states.palette' not found in section or not a List. Got {v:?}"
            )
            .into())
        }
    };

    // If the palette contains only one element, it is always this biome

    if palette.len() == 1 {
        let block = match &palette[0] {
            Value::Compound(v) => v,
            v => return Err(format!("'palette[0]' is not a Compound. Got {v:?}").into()),
        };

        return Ok(extract_block_data(block)?);
    }

    // Otherwise we calculate the palette index

    let data = match block_states.get("data") {
        Some(Value::LongArray(array)) => array,
        _ => return Err("block data not found or not a long array".into()),
    };

    let block_index = (y * 16 + z) * 16 + x;
    let bits_per_entry = calculate_bits_per_entry(palette.len());
    let palette_index = extract_palette_index(data, block_index, bits_per_entry)?;

    if palette_index >= palette.len() {
        return Err(format!(
            "Invalid palette index: got {}, palette size is {}, bits_per_entry is {}, block_index is {}", 
            palette_index, palette.len(), bits_per_entry, block_index
        ).into());
    }

    let block = match &palette[palette_index] {
        Value::Compound(v) => v,
        v => return Err(format!("'palette[palette_index]' is not a Compound. Got {v:?}").into()),
    };

    Ok(extract_block_data(block)?)
}

pub fn extract_block_data(
    block: &Compound,
) -> Result<(String, Option<HashMap<String, String>>), String> {
    let properties = match block.get("Properties") {
        Some(Value::Compound(props)) => Some(props),
        None => None,
        v => {
            return Err(
                format!("'block.Properties' was found but is not Compound. Got {v:?}").into(),
            )
        }
    };

    // To reduce RAM usage, only some useful props are kept
    let mut block_props = None;

    if let Some(c) = properties {
        if c.contains_key("snowy") {
            match c.get("snowy") {
                Some(Value::String(snowy)) => {
                    let mut map = HashMap::new();

                    map.insert(String::from("snowy"), snowy.clone());
                    block_props = Some(map);
                }
                _ => {}
            };
        }
    }

    let block_name = match block.get("Name") {
        Some(Value::String(name)) => name.clone(),
        v => return Err(format!("'block.Name' not found or not a String. Got {v:?}").into()),
    };

    Ok((block_name, block_props))
}

/// Calculate the bits per entry in the palette array.
/// Check the wiki for more info
pub fn calculate_bits_per_entry(palette_size: usize) -> usize {
    if palette_size <= 1 {
        return 0;
    }

    let min_bits = (palette_size as f64).log2().ceil() as usize;

    match min_bits {
        0..=4 => 4,
        5..=8 => min_bits,
        _ => min_bits.min(15),
    }
}

pub fn extract_palette_index(
    data: &[i64],
    block_index: usize,
    bits_per_entry: usize,
) -> Result<usize, String> {
    if bits_per_entry == 0 {
        return Ok(0);
    }

    let entries_per_long = 64 / bits_per_entry;
    let long_index = block_index / entries_per_long;
    let entry_index = block_index % entries_per_long;

    if long_index >= data.len() {
        return Err(format!(
            "Long index {} out of bounds (data length: {})",
            long_index,
            data.len()
        )
        .into());
    }

    let long_value = data[long_index] as u64;
    let shift = entry_index * bits_per_entry;
    let mask = (1u64 << bits_per_entry) - 1;
    let palette_index = (long_value >> shift) & mask;

    Ok(palette_index as usize)
}
