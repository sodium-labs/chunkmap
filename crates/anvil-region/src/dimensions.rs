#[derive(Debug, Clone, PartialEq)]
pub enum Dimension {
    Overworld,
    Nether,
    End,
}

pub fn get_dimension_height_offset(dimension: &Dimension) -> i32 {
    match dimension {
        Dimension::Overworld => -64,
        Dimension::Nether => 0,
        Dimension::End => 0,
    }
}

pub fn get_dimension_heights(dimension: &Dimension) -> (i32, i32) {
    match dimension {
        Dimension::Overworld => (-64, 320),
        Dimension::Nether => (0, 256),
        Dimension::End => (0, 256),
    }
}
