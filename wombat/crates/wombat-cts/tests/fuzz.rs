//! Damaged input is harmless (decision A12): arbitrary bytes, and the example files cut short
//! or altered (outside and inside the compressed part), never panic or hang; they give an
//! error or a document that is sound.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use proptest::prelude::*;
use wombat_core::SeededIds;
use wombat_cts::import;

/// Imports, and if that works, holds the result to everything a valid document promises.
fn import_and_check(bytes: &[u8]) -> bool {
    let Ok(imported) = import(bytes, &mut SeededIds::from_u64(3)) else {
        return false;
    };
    let doc = &imported.document;
    doc.validate().unwrap();
    let text = wombat_core::format::to_string(doc).unwrap();
    assert_eq!(
        &wombat_core::format::from_slice(text.as_bytes()).unwrap(),
        doc
    );
    let mut view = imported.view.clone();
    view.retain_existing(doc);
    assert_eq!(view, imported.view, "the view state names cells that exist");
    for image in &imported.images {
        assert!(wombat_core::model::is_valid_image_name(&image.file));
    }
    true
}

/// An example: its bytes, where its compressed part starts, and that part decompressed.
struct Example {
    bytes: Vec<u8>,
    header: usize,
    stream: Vec<u8>,
}

fn cts_files(dir: &Path, found: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            cts_files(&path, found);
        } else if path.extension().is_some_and(|e| e == "cts") {
            found.push(path);
        }
    }
}

fn examples() -> &'static [Example] {
    static EXAMPLES: OnceLock<Vec<Example>> = OnceLock::new();
    EXAMPLES.get_or_init(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        cts_files(&root.join("../../../TS/examples"), &mut files);
        cts_files(&root.join("../../referee/fixtures"), &mut files);
        cts_files(&root.join("tests/quirks"), &mut files);
        assert!(files.len() >= 38);
        files
            .iter()
            .map(|path| {
                let bytes = std::fs::read(path).unwrap();
                // The compressed part starts after the first "D" from which the rest of the
                // file is one whole zlib stream (image data could hold a "D" too).
                let (header, stream) = (0..bytes.len())
                    .filter(|at| bytes[*at] == b'D')
                    .find_map(|at| {
                        let rest = &bytes[at + 1..];
                        let stream = miniz_oxide::inflate::decompress_to_vec_zlib(rest).ok()?;
                        Some((at + 1, stream))
                    })
                    .unwrap_or_else(|| panic!("{}: no compressed part", path.display()));
                assert!(import_and_check(&bytes), "{}", path.display());
                Example {
                    bytes,
                    header,
                    stream,
                }
            })
            .collect()
    })
}

/// A way to damage bytes.
#[derive(Clone, Debug)]
enum Damage {
    /// Cut off at this fraction of the length.
    Truncate(f64),
    /// Flip these bits at these fractions of the length.
    Flip(Vec<(f64, u8)>),
    /// Overwrite four bytes with a number that counts and lengths react badly to.
    Number(f64, u32),
    /// Remove this many bytes, or insert that many copies of a byte.
    Remove(f64, usize),
    Insert(f64, usize, u8),
}

fn damage() -> impl Strategy<Value = Damage> {
    let at = 0.0..1.0f64;
    let numbers = prop::sample::select(vec![
        0u32,
        1,
        2,
        0x7f,
        0x80,
        0xff,
        0xffff,
        0x10000,
        0x40_0000,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_ffff,
    ]);
    prop_oneof![
        at.clone().prop_map(Damage::Truncate),
        prop::collection::vec((at.clone(), 1u8..=255), 1..8).prop_map(Damage::Flip),
        (at.clone(), numbers).prop_map(|(at, n)| Damage::Number(at, n)),
        (at.clone(), 1usize..40).prop_map(|(at, n)| Damage::Remove(at, n)),
        (at, 1usize..40, any::<u8>()).prop_map(|(at, n, b)| Damage::Insert(at, n, b)),
    ]
}

fn apply(damage: &Damage, bytes: &[u8]) -> Vec<u8> {
    let mut out = bytes.to_vec();
    let place = |at: f64| ((bytes.len() as f64 * at) as usize).min(bytes.len().saturating_sub(1));
    match damage {
        Damage::Truncate(at) => out.truncate(place(*at)),
        Damage::Flip(flips) => {
            for (at, bits) in flips {
                if let Some(byte) = out.get_mut(place(*at)) {
                    *byte ^= bits;
                }
            }
        }
        Damage::Number(at, number) => {
            for (byte, new) in out.iter_mut().skip(place(*at)).zip(number.to_le_bytes()) {
                *byte = new;
            }
        }
        Damage::Remove(at, count) => {
            let from = place(*at);
            out.drain(from..(from + count).min(bytes.len()));
        }
        Damage::Insert(at, count, byte) => {
            let from = place(*at);
            out.splice(from..from, std::iter::repeat_n(*byte, *count));
        }
    }
    out
}

fn recompress(header: &[u8], stream: &[u8]) -> Vec<u8> {
    let mut out = header.to_vec();
    out.extend_from_slice(&miniz_oxide::deflate::compress_to_vec_zlib(stream, 1));
    out
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn arbitrary_bytes_never_panic(bytes in prop::collection::vec(any::<u8>(), 0..2048)) {
        import_and_check(&bytes);
    }

    #[test]
    fn arbitrary_bytes_after_the_signature_never_panic(
        version in 0u8..=33,
        bytes in prop::collection::vec(any::<u8>(), 0..2048),
    ) {
        let mut file = b"TSFF".to_vec();
        file.push(version);
        file.extend_from_slice(&bytes);
        import_and_check(&file);
    }

    /// The cell reader itself, on bytes it was not written for: an arbitrary stream,
    /// properly compressed, behind a proper header of any version.
    #[test]
    fn an_arbitrary_cell_stream_never_panics(
        version in 0u8..=31,
        stream in prop::collection::vec(
            prop_oneof![3 => 0u8..8, 1 => any::<u8>()], 0..4096),
    ) {
        let mut header = b"TSFF".to_vec();
        header.push(version);
        let count = usize::from(version >= 30) * 2
            + usize::from(version >= 21) * 2
            + usize::from(version >= 23);
        header.extend_from_slice(&[0, 1, 1, 0, 0][..count]);
        header.push(b'D');
        import_and_check(&recompress(&header, &stream));
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn a_damaged_example_file_never_panics(file in 0usize..1000, damage in damage()) {
        let examples = examples();
        let example = &examples[file % examples.len()];
        import_and_check(&apply(&damage, &example.bytes));
    }

    /// The same damage to the decompressed cells, compressed again, so that it reaches the
    /// cell reader instead of being caught by the checksum of the compressed part.
    #[test]
    fn a_damaged_cell_stream_never_panics(
        file in 0usize..1000,
        damages in prop::collection::vec(damage(), 1..4),
    ) {
        let examples = examples();
        let example = &examples[file % examples.len()];
        let mut stream = example.stream.clone();
        for damage in &damages {
            stream = apply(damage, &stream);
        }
        import_and_check(&recompress(&example.bytes[..example.header], &stream));
    }
}

/// Every example cut at every length up to the start of the cells, and at steps beyond: the
/// header and image readers see every possible early end.
#[test]
fn every_truncation_of_every_example_is_refused() {
    for example in examples() {
        let mut cuts: Vec<usize> = (0..example.header.min(600)).collect();
        cuts.extend((example.header.min(600)..example.bytes.len()).step_by(97));
        for cut in cuts {
            assert!(
                !import_and_check(&example.bytes[..cut]),
                "a file cut to {cut} bytes was accepted"
            );
        }
        // The cells cut short at every length (compressed again as a whole stream).
        for cut in (0..example.stream.len()).step_by(example.stream.len() / 150 + 1) {
            import_and_check(&recompress(
                &example.bytes[..example.header],
                &example.stream[..cut],
            ));
        }
    }
}

/// How much of the damage gets past the first checks: if nearly all damaged streams were
/// refused at once, the tests above would say little about the cell reader.
#[test]
fn damaged_cell_streams_are_often_still_documents() {
    let mut accepted = 0;
    let mut tried = 0;
    for example in examples() {
        for at in (0..example.stream.len()).step_by(example.stream.len() / 40 + 1) {
            let mut stream = example.stream.clone();
            stream[at] ^= 0x01;
            tried += 1;
            accepted += usize::from(import_and_check(&recompress(
                &example.bytes[..example.header],
                &stream,
            )));
        }
    }
    println!("{accepted} of {tried} one-bit changes to cell streams still import");
    assert!(accepted * 4 > tried, "{accepted} of {tried}");
}
