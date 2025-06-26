use crate::chunks::{Chunk, ChunkPosition};
use fastnbt::from_bytes;
use fastnbt::Value;
use flate2::read::ZlibDecoder;
use std::io;
use std::io::Read;

pub const CHUNKS_PER_REGION: usize = 1024;

/// A region is a 32x32 chunks area
pub struct Region {
    pub chunks: Vec<Chunk>,
}

pub fn parse_region_bytes(data: &[u8]) -> std::io::Result<Region> {
    if data.len() < 8192 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Region file too small",
        ));
    }

    let mut chunks = Vec::with_capacity(CHUNKS_PER_REGION);
    let header = &data[..8192];

    for i in 0..CHUNKS_PER_REGION {
        let idx = i * 4;
        let offset = ((header[idx] as u32) << 16)
            | ((header[idx + 1] as u32) << 8)
            | (header[idx + 2] as u32);
        let sector_count = header[idx + 3];

        if offset == 0 || sector_count == 0 {
            continue;
        }

        let byte_offset = offset as usize * 4096;
        if byte_offset + 5 > data.len() {
            continue;
        }

        let length = u32::from_be_bytes(data[byte_offset..byte_offset + 4].try_into().unwrap());
        let compression_type = data[byte_offset + 4];

        let data_start = byte_offset + 5;
        let data_end = data_start + (length - 1) as usize;
        if data_end > data.len() {
            continue;
        }

        let compressed_data = &data[data_start..data_end];
        let chunk_data = match compression_type {
            2 => {
                let mut decoder = ZlibDecoder::new(compressed_data);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed)?;
                decompressed
            }
            _ => continue,
        };

        let chunk = match parse_chunk_from_bytes(i as i32, chunk_data) {
            Ok(Some(v)) => v,
            Ok(None) => continue,
            Err(e) => return Err(io::Error::new(io::ErrorKind::Unsupported, e)),
        };

        chunks.push(chunk);
    }

    Ok(Region { chunks })
}

pub fn parse_chunk_from_bytes(i: i32, bytes: Vec<u8>) -> Result<Option<Chunk>, String> {
    let nbt: Value = match from_bytes(&bytes) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };

    let Value::Compound(ref root) = nbt else {
        return Ok(None);
    };

    if let Some(Value::String(status)) = root.get("Status") {
        if status != "minecraft:full" {
            // This is a proto-chunk (not fully generated) so we ignore it
            return Ok(None);
        }
    }

    let data_version = match root.get("DataVersion") {
        Some(Value::Int(v)) => *v,
        v => return Err(format!("'DataVersion' not found or not an Int. Got {v:#?}").into()),
    };
    let last_update = match root.get("LastUpdate") {
        Some(Value::Long(v)) => *v,
        v => return Err(format!("'LastUpdate' not found or not a Long. Got {v:#?}").into()),
    };
    let inhabited_time = match root.get("InhabitedTime") {
        Some(Value::Long(v)) => *v,
        v => return Err(format!("'InhabitedTime' not found or not a Long. Got {v:#?}").into()),
    };

    let chunk_x = root
        .get("xPos")
        .and_then(|v| match v {
            Value::Int(x) => Some(*x),
            _ => None,
        })
        .unwrap_or(i % 32);

    let chunk_z = root
        .get("zPos")
        .and_then(|v| match v {
            Value::Int(z) => Some(*z),
            _ => None,
        })
        .unwrap_or(i / 32);

    Ok(Some(Chunk {
        position: ChunkPosition {
            x: chunk_x,
            z: chunk_z,
        },
        data_version,
        last_update,
        inhabited_time,
        nbt,
    }))
}
