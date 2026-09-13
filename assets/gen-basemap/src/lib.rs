#![forbid(unsafe_code)]

use std::{
    io,
    path::{Path, PathBuf},
};

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

/// The repository checkout this crate is built from.
///
/// Both binaries read and write paths relative to the repository rather than to
/// the working directory, so `cargo run` behaves the same from any directory.
/// The crate lives at `assets/gen-basemap`, two levels below the root.
///
/// # Errors
/// Returns an [`io::Error`] if the crate's manifest directory has fewer than two
/// ancestors, which only a relocated crate can cause.
pub fn repository_root() -> io::Result<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .ok_or_else(|| io::Error::other("crate is not nested under the repository root"))
}
