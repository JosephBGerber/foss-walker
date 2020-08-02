use alloc::vec::Vec;

pub const WIDTH: usize = 144;
pub const HEIGHT: usize = 168;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Object {
    pub sprite: &'static [u8],
    pub width: u16,
    pub height: u16,
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OAM {
    pub objects: Vec<Object>
}