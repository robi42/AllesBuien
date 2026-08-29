#![forbid(unsafe_code)]

pub mod geojson;
pub mod raster;
mod sha256;
mod transform;

pub use sha256::sha256_hex;
pub use transform::{MapProjection, ScreenPoint};

pub const WIDTH: u32 = 1058;
pub const HEIGHT: u32 = 915;
pub const WATER: [u8; 3] = [0xAA, 0xB2, 0xBB];
pub const LAND: [u8; 3] = [0xC8, 0xCD, 0xD2];
pub const BOUNDARY: [u8; 3] = [0x8A, 0x93, 0x9E];
