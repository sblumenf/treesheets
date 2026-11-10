//! The limits of `limits.rs`, at and just beyond each one, and decision A22 near them:
//! anything that saving produces, loading accepts.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use proptest::prelude::*;
use serde_json::Value;
use wombat_core::construct::Construct;

use wombat_core::limits::{
    MAX_CELLS, MAX_DEPTH, MAX_FILE_BYTES, MAX_IMAGE_NAME_BYTES, MAX_JSON_DEPTH,
    MAX_TEXT_CODE_POINTS, MAX_UNKNOWN_DEPTH,
};
use wombat_core::model::{is_valid_image_name, Extra};
use wombat_core::{
    format, CellId, Document, Error, Id, ImageRef, Link, LinkTarget, Run, SeededIds, Selection,
    Style, Tag, ViewState,
};

/// Decision A22: a document either saves to a file that loads back equal, or is refused with
/// a limit or validity error. Returns whether it was saved.
fn saves_only_what_loads(doc: &Document) -> bool {
    match format::to_string(doc) {
        Ok(text) => {
            let back = match format::from_slice(text.as_bytes()) {
                Ok(back) => back,
                Err(e) => panic!("a saved file of {} bytes was refused: {e}", text.len()),
            };
            assert!(back == *doc, "the saved file loads as another document");
            true
        }
        Err(Error::Limit(_) | Error::Invalid(_)) => false,
        Err(e) => panic!("unexpected error: {e}"),
    }
}

/// A document whose root holds a chain of 1 by 1 grids; returns the cell at `depth`.
fn chain(depth: u32) -> (Document, CellId, SeededIds) {
    let mut ids = SeededIds::from_u64(u64::from(depth));
    let mut doc = Document::new(&mut ids);
    let mut cell = doc.root();
    for _ in 0..depth {
        doc.add_grid(cell, 1, 1, &mut ids).unwrap();
        cell = doc.child(cell, 0, 0).unwrap();
    }
    (doc, cell, ids)
}

/// A value nested `depth` arrays deep: `nested(2)` is `[[null]]`.
fn nested(depth: usize) -> Value {
    (0..depth).fold(Value::Null, |inner, _| Value::Array(vec![inner]))
}

#[test]
fn a_document_too_large_to_load_is_not_saved() {
    let mut ids = SeededIds::from_u64(1);
    let mut doc = Document::new(&mut ids);
    // Every text is well within the text limit; together they are over the file limit.
    let chunk = 4 * 1024 * 1024;
    let count = u32::try_from(MAX_FILE_BYTES / chunk as u64).unwrap() + 1;
    doc.add_grid(doc.root(), 1, count, &mut ids).unwrap();
    let text = "x".repeat(chunk);
    for y in 0..count {
        let cell = doc.child(doc.root(), 0, y).unwrap();
        doc.cell_mut(cell).unwrap().text = text.clone();
    }
    doc.validate().unwrap();
    assert!(!saves_only_what_loads(&doc));
    assert!(matches!(format::to_string(&doc), Err(Error::Limit(_))));
}

#[test]
fn deeply_nested_unknown_values_are_saved_only_if_they_load() {
    for depth in [1, 20, 24, 25, 26, 27, 28, 100, 125, 126, 127, 128, 200] {
        for grid_depth in [0, 1, 30, 31, 32] {
            let (mut doc, cell, _) = chain(grid_depth);
            doc.cell_mut(cell)
                .unwrap()
                .extra
                .insert("zz".into(), nested(depth));
            saves_only_what_loads(&doc);
        }
    }
}

fn file(root_fields: &str) -> String {
    let (doc, root) = (Id::from_u128(1), Id::from_u128(2));
    format!(
        r#"{{"format": "wombat", "version": 1, "id": "{doc}", "root": {{"id": "{root}"{root_fields}}}}}"#
    )
}

#[test]
fn a_repeated_key_is_refused_at_every_level() {
    format::from_slice(file(r#", "text": "a", "zz": {"a": 1, "b": [{"a": 2}]}"#).as_bytes())
        .unwrap();
    let other = Id::from_u128(3);
    for fields in [
        r#", "text": "first", "text": "second""#.to_string(),
        format!(r#", "id": "{other}""#),
        r#", "runs": [{"start": 0, "len": 1, "len": 1}], "text": "a""#.to_string(),
        r#", "image": {"file": "a.png", "file": "a.png"}"#.to_string(),
        // Also inside the value of an unknown field, which is kept but not interpreted.
        r#", "zz": {"a": 1, "a": 1}"#.to_string(),
        r#", "zz": [[{"b": {"a": 1, "c": 2, "a": 3}}]]"#.to_string(),
    ] {
        let result = format::from_slice(file(&fields).as_bytes());
        assert!(
            matches!(&result, Err(Error::Invalid(m)) if m.contains("repeated")),
            "{fields}: {result:?}"
        );
    }
    let top = file("").replacen('{', r#"{"version": 1, "#, 1);
    assert!(matches!(
        format::from_slice(top.as_bytes()),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn image_names_that_some_system_cannot_use_are_refused() {
    for good in [
        "a.png",
        "A-b_c.1.JPG",
        "console.png",
        "com.png",
        "com10.png",
        "nul_.png",
        "x",
    ] {
        assert!(is_valid_image_name(good), "{good}");
    }
    for bad in [
        "",
        ".hidden",
        "a/b.png",
        "a\\b.png",
        "..",
        "a b.png",
        "é.png",
        "a.png ",
        "a.",
        "a..",
        "CON",
        "con",
        "Con.png",
        "PRN",
        "AUX.txt",
        "NUL",
        "nul.tar.gz",
        "COM1",
        "com9.png",
        "LPT1",
        "lpt0.png",
        "COM0",
    ] {
        assert!(!is_valid_image_name(bad), "{bad:?}");
        let text = file(&format!(r#", "image": {{"file": {bad:?}}}"#));
        assert!(format::from_slice(text.as_bytes()).is_err(), "{bad:?}");
    }
}

fn is_limit<T: std::fmt::Debug>(result: Result<T, Error>) -> bool {
    matches!(result, Err(Error::Limit(_)))
}

/// The deepest nesting of arrays and objects in JSON text (strings here hold no brackets).
fn json_depth(text: &str) -> u32 {
    let (mut depth, mut deepest) = (0u32, 0u32);
    for b in text.bytes() {
        match b {
            b'[' | b'{' => {
                depth += 1;
                deepest = deepest.max(depth);
            }
            b']' | b'}' => depth -= 1,
            _ => {}
        }
    }
    deepest
}

#[test]
fn file_size_exactly_at_the_limit_and_one_byte_over() {
    let max = usize::try_from(MAX_FILE_BYTES).unwrap();
    let mut doc = Document::new(&mut SeededIds::from_u64(1));
    let root = doc.root();
    doc.cell_mut(root).unwrap().text = "x".repeat(MAX_TEXT_CODE_POINTS);
    doc.cell_mut(root).unwrap().note = "x".into();
    let small = format::to_string(&doc).unwrap().len();
    // Lengthen the note until the file is exactly as large as a file may be.
    doc.cell_mut(root).unwrap().note = "x".repeat(max - small + 1);
    let text = format::to_string(&doc).unwrap();
    assert_eq!(text.len(), max);
    assert!(format::from_slice(text.as_bytes()).unwrap() == doc);
    doc.cell_mut(root).unwrap().note.push('x');
    assert!(is_limit(format::to_string(&doc)));

    // The reader has the same limit, whatever the bytes are.
    let mut bytes = format::to_string(&Document::new(&mut SeededIds::from_u64(1)))
        .unwrap()
        .into_bytes();
    bytes.resize(max, b' ');
    format::from_slice(&bytes).unwrap();
    bytes.push(b' ');
    assert!(is_limit(format::from_slice(&bytes)));
    assert!(is_limit(ViewState::from_json(&bytes)));
}

#[test]
fn text_and_note_exactly_at_the_limit_and_one_code_point_over() {
    for note in [false, true] {
        let mut doc = Document::new(&mut SeededIds::from_u64(1));
        let root = doc.root();
        let set = |doc: &mut Document, text: String| {
            let cell = doc.cell_mut(root).unwrap();
            *(if note { &mut cell.note } else { &mut cell.text }) = text;
        };
        // Code points, not bytes: this text is twice as many bytes.
        set(&mut doc, "\u{e9}".repeat(MAX_TEXT_CODE_POINTS));
        doc.validate().unwrap();
        set(&mut doc, "\u{e9}".repeat(MAX_TEXT_CODE_POINTS + 1));
        assert!(is_limit(doc.validate()));

        set(&mut doc, "x".repeat(MAX_TEXT_CODE_POINTS));
        assert!(saves_only_what_loads(&doc));
        let at_limit = format::to_string(&doc).unwrap();
        set(&mut doc, "x".repeat(MAX_TEXT_CODE_POINTS + 1));
        assert!(is_limit(format::to_string(&doc)));
        // The same file with one more character, written by someone else.
        let over = at_limit.replacen("xx", "xxx", 1);
        assert_eq!(over.len(), at_limit.len() + 1);
        assert!(is_limit(format::from_slice(over.as_bytes())));
    }
}

#[test]
fn grids_nested_exactly_to_the_limit_and_one_deeper() {
    let (mut doc, deepest, mut ids) = chain(MAX_DEPTH);
    assert_eq!(doc.depth(deepest), Some(MAX_DEPTH));
    assert!(saves_only_what_loads(&doc));
    assert!(is_limit(doc.add_grid(deepest, 1, 1, &mut ids)));
    assert!(saves_only_what_loads(&doc));

    // A file written by someone else with one more level.
    let text = format::to_string(&doc).unwrap();
    let cell = format!("{{\"id\": \"{deepest}\"}}");
    assert_eq!(text.matches(&cell).count(), 1);
    let extra_id = Id::from_u128(7);
    let deeper = text.replace(
        &cell,
        &format!(
            "{{\"id\": \"{deepest}\", \"grid\": {{\"xs\": 1, \"ys\": 1, \"cells\": [{{\"id\": \"{extra_id}\"}}]}}}}"
        ),
    );
    assert!(is_limit(format::from_slice(deeper.as_bytes())));
    let (mut doc, deepest, mut ids) = chain(MAX_DEPTH - 1);
    doc.add_grid(deepest, 1, 1, &mut ids).unwrap();
}

#[test]
fn more_cells_than_the_limit_are_refused_before_they_are_made() {
    // Reaching the limit itself would take gigabytes; only "just beyond" is tested, and only
    // in memory, since a file of that many cells is over the file size limit anyway.
    let mut ids = SeededIds::from_u64(1);
    let mut doc = Document::new(&mut ids);
    let all = u32::try_from(MAX_CELLS).unwrap();
    assert!(is_limit(doc.add_grid(doc.root(), all, 1, &mut ids)));
    assert!(is_limit(doc.add_grid(doc.root(), 1, all, &mut ids)));
    assert!(is_limit(doc.add_grid(
        doc.root(),
        u32::MAX,
        u32::MAX,
        &mut ids
    )));
    assert_eq!(doc.cell_count(), 1);
    assert!(
        MAX_CELLS as u64 * 40 > MAX_FILE_BYTES,
        "a cell takes over 40 bytes"
    );
}

#[test]
fn image_name_exactly_at_the_limit_and_one_byte_over() {
    let mut doc = Document::new(&mut SeededIds::from_u64(1));
    let root = doc.root();
    doc.cell_mut(root).unwrap().image = Some(ImageRef::new("a".repeat(MAX_IMAGE_NAME_BYTES)));
    assert!(saves_only_what_loads(&doc));
    doc.cell_mut(root).unwrap().image = Some(ImageRef::new("a".repeat(MAX_IMAGE_NAME_BYTES + 1)));
    assert!(!saves_only_what_loads(&doc));
    let text = file(&format!(
        r#", "image": {{"file": "{}"}}"#,
        "a".repeat(MAX_IMAGE_NAME_BYTES + 1)
    ));
    assert!(format::from_slice(text.as_bytes()).is_err());
}

/// Every place of a document that keeps unknown fields.
const SLOTS: usize = 7;

/// Puts an unknown field in the `slot`-th kind of place, on the cell `cell`.
fn put_unknown(doc: &mut Document, cell: CellId, slot: usize, value: Value) {
    let extra: Extra = [("zz".to_string(), value)].into();
    let target = doc.cell_mut(cell).unwrap();
    target.text = "abc".into();
    match slot % SLOTS {
        0 => target.extra = extra,
        1 => {
            target.runs = vec![Run {
                start: 0,
                len: 1,
                style: Style::default(),
                color: None,
                extra,
            }]
        }
        2 => {
            target.links = vec![Link {
                start: 1,
                len: 2,
                target: LinkTarget::Unknown,
                extra,
            }]
        }
        3 => {
            target.image = Some(ImageRef {
                extra,
                ..ImageRef::new("a.png")
            })
        }
        // A cell at the deepest level cannot have a grid; its parent's grid is the deepest.
        4 => match target.grid_mut() {
            Some(grid) => grid.extra = extra,
            None => {
                let parent = target.parent().unwrap_or(cell);
                match doc.cell_mut(parent).unwrap().grid_mut() {
                    Some(grid) => grid.extra = extra,
                    None => *doc.extra_mut() = extra,
                }
            }
        },
        5 => {
            let tag = Tag {
                extra,
                ..Tag::default()
            };
            doc.tags_mut().insert("tag".into(), tag);
        }
        _ => *doc.extra_mut() = extra,
    }
}

#[test]
fn unknown_values_nested_exactly_to_the_limit_and_one_deeper() {
    let (at, over) = (MAX_UNKNOWN_DEPTH as usize, MAX_UNKNOWN_DEPTH as usize + 1);
    let mut deepest_file = 0;
    for slot in 0..SLOTS {
        let (mut doc, cell, _) = chain(MAX_DEPTH);
        put_unknown(&mut doc, cell, slot, nested(at));
        assert!(saves_only_what_loads(&doc), "slot {slot}");
        let text = format::to_string(&doc).unwrap();
        deepest_file = deepest_file.max(json_depth(&text));

        // One level more: refused when saving, and when someone else wrote it.
        let (mut doc, cell, _) = chain(MAX_DEPTH);
        put_unknown(&mut doc, cell, slot, nested(over));
        assert!(is_limit(format::to_string(&doc)), "slot {slot}");
        let inner = serde_json::to_string(&nested(at)).unwrap();
        assert_eq!(text.matches(&inner).count(), 1);
        let deeper = text.replace(&inner, &format!("[{inner}]"));
        assert!(
            is_limit(format::from_slice(deeper.as_bytes())),
            "slot {slot}"
        );
        // Objects count like arrays.
        let object = text.replace(&inner, &format!("{{\"a\": {inner}}}"));
        assert!(
            is_limit(format::from_slice(object.as_bytes())),
            "slot {slot}"
        );
    }
    // The limits fit together: the deepest file that can be written is one the parser reads.
    assert_eq!(deepest_file, 1 + 3 * MAX_DEPTH + 3 + MAX_UNKNOWN_DEPTH);
    assert!(deepest_file <= MAX_JSON_DEPTH);

    // The parser's own limit is where the constant says, so the sum above means something.
    let open = "[".repeat(MAX_JSON_DEPTH as usize);
    let close = "]".repeat(MAX_JSON_DEPTH as usize);
    let parses =
        |text: String| !matches!(format::from_slice(text.as_bytes()), Err(Error::Syntax(_)));
    assert!(parses(format!("{open}{close}")));
    assert!(!parses(format!("[{open}{close}]")));

    // View state has the same rule.
    let mut view = ViewState::new(wombat_core::DocId(Id::from_u128(1)));
    view.extra.insert("zz".into(), nested(at));
    view.selection = Some(Selection {
        cell: CellId(Id::from_u128(2)),
        xs: 1,
        ys: 1,
        cursor: None,
        extra: [("zz".to_string(), nested(at))].into(),
    });
    let text = view.to_json().unwrap();
    assert!(ViewState::from_json(text.as_bytes()).unwrap() == view);
    view.extra.insert("zz".into(), nested(over));
    assert!(is_limit(view.to_json()));
    let inner = serde_json::to_string(&nested(at)).unwrap();
    let deeper = text.replace(&inner, &format!("[{inner}]"));
    assert!(is_limit(ViewState::from_json(deeper.as_bytes())));
}

/// A value nested exactly `shape.len()` deep, as arrays and objects with other members
/// beside the nested one.
fn shaped(shape: &[u8]) -> Value {
    shape
        .iter()
        .fold(Value::from(1), |inner, kind| match kind % 4 {
            0 => Value::Array(vec![inner]),
            1 => Value::Array(vec![Value::Null, inner, Value::from("x")]),
            2 => serde_json::json!({ "a": inner }),
            _ => serde_json::json!({"a": 0, "b": inner, "c": "x"}),
        })
}

/// One of the characters that take 1, 2, 4, 2 (escaped) or 6 (escaped) bytes in a file.
fn filler() -> impl Strategy<Value = char> {
    proptest::sample::select(vec!['x', '\u{e9}', '\u{1F600}', '"', '\u{1}'])
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 96, ..ProptestConfig::default() })]

    /// Decision A22 around the nesting limits: grids at and next to the deepest level, each
    /// with an unknown value nested at and next to its limit, in every kind of place.
    #[test]
    fn near_the_nesting_limits_what_saves_loads(
        grid_depth in MAX_DEPTH - 2..=MAX_DEPTH,
        places in proptest::collection::vec(
            (0u32..=MAX_DEPTH, 0..SLOTS,
             proptest::collection::vec(any::<u8>(), 20..=28)),
            1..4),
    ) {
        let (mut doc, _, _) = chain(grid_depth);
        let cells = doc.cells_in_order();
        let mut too_deep = false;
        for (at, slot, shape) in &places {
            let cell = cells[(*at as usize).min(cells.len() - 1)];
            put_unknown(&mut doc, cell, *slot, shaped(shape));
            too_deep |= shape.len() > MAX_UNKNOWN_DEPTH as usize;
        }
        // A later value can replace an earlier one in the same place, so "too deep" is only
        // certain the other way round.
        let saved = saves_only_what_loads(&doc);
        prop_assert!(saved || too_deep);
        if saved {
            prop_assert!(json_depth(&format::to_string(&doc).unwrap()) <= MAX_JSON_DEPTH);
        }
    }
}

proptest! {
    // Each case writes and reads tens of megabytes, so there are few.
    #![proptest_config(ProptestConfig { cases: 6, ..ProptestConfig::default() })]

    /// Decision A22 around the size limits: texts at and next to the longest text, of
    /// characters of every width, in a file brought to within a few bytes of the largest.
    #[test]
    fn near_the_size_limits_what_saves_loads(
        texts in proptest::collection::vec((
            filler(),
            prop_oneof![
                MAX_TEXT_CODE_POINTS - 1..=MAX_TEXT_CODE_POINTS + 1,
                MAX_TEXT_CODE_POINTS / 4 - 1..=MAX_TEXT_CODE_POINTS / 4 + 1,
                0usize..100,
            ],
            any::<bool>(),
        ), 1..=2),
        over in -2i64..=2,
    ) {
        let mut ids = SeededIds::from_u64(1);
        let mut doc = Document::new(&mut ids);
        doc.add_grid(doc.root(), 4, 1, &mut ids).unwrap();
        for (n, (c, count, note)) in texts.iter().enumerate() {
            let cell = doc.child(doc.root(), n as u32, 0).unwrap();
            let cell = doc.cell_mut(cell).unwrap();
            let text = c.to_string().repeat(*count);
            *(if *note { &mut cell.note } else { &mut cell.text }) = text;
        }
        // The text and note of the last two cells are filled up so that the file is `over`
        // bytes from the limit; each starts as one character, which fixes the other bytes.
        let fill = |doc: &mut Document, mut bytes: usize| {
            for n in [2, 3] {
                let cell = doc.child(doc.root(), n, 0).unwrap();
                let cell = doc.cell_mut(cell).unwrap();
                for slot in [&mut cell.text, &mut cell.note] {
                    let here = bytes.min(MAX_TEXT_CODE_POINTS - 1);
                    *slot = "x".repeat(1 + here);
                    bytes -= here;
                }
            }
            bytes == 0
        };
        fill(&mut doc, 0);
        if let Ok(text) = format::to_string(&doc) {
            prop_assert!(saves_only_what_loads(&doc));
            let room = MAX_FILE_BYTES as i64 - text.len() as i64 + over;
            if room >= 0 {
                prop_assert!(fill(&mut doc, room as usize));
                prop_assert_eq!(saves_only_what_loads(&doc), over <= 0);
                if over <= 0 {
                    let size = format::to_string(&doc).unwrap().len() as i64;
                    prop_assert_eq!(size, MAX_FILE_BYTES as i64 + over);
                }
            }
        } else {
            prop_assert!(!saves_only_what_loads(&doc));
        }
    }
}
