#[derive(Debug, Clone)]
pub struct BlockPosition {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub position: BlockPosition,
    pub name: String,
    pub depth: u16,
    pub snowy: bool,
}
