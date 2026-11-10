//! Upper bounds on what the core will read or build (decision A12).
//!
//! They are part of the file format: see `docs/FILE_FORMAT.md`.

/// Largest document or view-state file that is read, in bytes.
pub const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;
/// Largest number of cells in one document.
pub const MAX_CELLS: usize = 5_000_000;
/// Deepest nesting of grids. The root cell is at depth 0, the cells of its grid at depth 1.
pub const MAX_DEPTH: u32 = 32;
/// Longest text or note of one cell, in Unicode code points.
pub const MAX_TEXT_CODE_POINTS: usize = 16 * 1024 * 1024;
/// Longest image file name, in bytes.
pub const MAX_IMAGE_NAME_BYTES: usize = 255;
