//! Upper bounds on what the core will read or build (decision A12).
//!
//! They are part of the file format: see `docs/FILE_FORMAT.md`.

/// Largest document or view-state file that is read or written, in bytes. Loading needs
/// memory of up to about 17 times the file size (decision A26), so this is what bounds memory.
pub const MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;
/// Largest number of cells in one document.
pub const MAX_CELLS: usize = 5_000_000;
/// Deepest nesting of grids. The root cell is at depth 0, the cells of its grid at depth 1.
pub const MAX_DEPTH: u32 = 32;
/// Longest text or note of one cell, in Unicode code points.
pub const MAX_TEXT_CODE_POINTS: usize = 16 * 1024 * 1024;
/// Deepest nesting of arrays and objects inside the value of one unknown field: `1` and `"a"`
/// are 0 deep, `[]` is 1, `[{"a": []}]` is 3. With [`MAX_DEPTH`] it keeps every file within
/// [`MAX_JSON_DEPTH`], wherever in the document the field is.
pub const MAX_UNKNOWN_DEPTH: u32 = 24;
/// Deepest nesting of JSON arrays and objects the parser reads (serde_json refuses the 128th
/// level). The deepest place for an unknown field is a run or link of a cell at
/// [`MAX_DEPTH`]: the document object, three levels per grid, the cell, the list, the run.
pub const MAX_JSON_DEPTH: u32 = 127;
const _: () = assert!(1 + 3 * MAX_DEPTH + 3 + MAX_UNKNOWN_DEPTH <= MAX_JSON_DEPTH);
/// Longest image file name, in bytes.
pub const MAX_IMAGE_NAME_BYTES: usize = 255;
