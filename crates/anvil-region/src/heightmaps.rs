pub type Heightmap = Vec<u16>;

pub const HM_LENGTH: usize = 256;
pub const HM_BITS_PER_VALUE: usize = 9;
pub const HM_VALUES_PER_LONG: usize = 64 / HM_BITS_PER_VALUE;

/// View the wiki to understand how heightmaps are compacted
pub fn decode_heightmap(packed_data: &[i64]) -> Result<Heightmap, String> {
    let mut heights = Vec::with_capacity(HM_LENGTH);

    for i in 0..HM_LENGTH {
        let long_index = i / HM_VALUES_PER_LONG;
        let value_index = i % HM_VALUES_PER_LONG;

        let long_value = packed_data[long_index] as u64;
        let bit_offset = value_index * HM_BITS_PER_VALUE;

        let mask = (1u64 << HM_BITS_PER_VALUE) - 1; // 0b111111111
        let height = ((long_value >> bit_offset) & mask) as u16;

        heights.push(height);
    }

    Ok(heights)
}
