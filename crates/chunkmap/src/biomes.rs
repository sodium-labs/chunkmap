use serde::Deserialize;
use std::{collections::HashMap, error::Error};

#[derive(Deserialize)]
pub struct BiomeData {
    pub name: String,
    pub temperature: f32,
    pub downfall: f32,
    pub foliage_color: u32,
    pub grass_color: u32,
    pub water_color: u32,
}

// NOTE: Cannot use fs because of WASM support
const BIOMES_JSON: &str = include_str!("../../../biomes.json");

pub fn load_biomes_data() -> Result<HashMap<String, BiomeData>, Box<dyn Error>> {
    let raw_biomes: Vec<BiomeData> = serde_json::from_str(&BIOMES_JSON)?;
    let mut biomes = HashMap::new();

    for biome in raw_biomes {
        biomes.insert(biome.name.clone(), biome);
    }

    Ok(biomes)
}

pub fn get_biome_data<'a>(data: &'a HashMap<String, BiomeData>, name: &str) -> &'a BiomeData {
    data.get(name).unwrap_or_else(|| {
        eprintln!("No biome data found for '{name}'");
        data.get("plains").expect("No 'plains' biome found")
    })
}
