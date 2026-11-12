//! Rules that hold for every document and every input (property tests).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeMap;
use wombat_core::construct::Construct;

use proptest::collection::{btree_map, vec};
use proptest::prelude::*;
use serde_json::Value;

use wombat_core::model::Extra;
use wombat_core::view::Level;
use wombat_core::{
    format, CellId, CellKind, Color, DocId, Document, DrawStyle, Id, ImageRef, Link, LinkTarget,
    Run, SeededIds, Selection, Style, Tag, TextAlign, VertAlign, ViewState,
};

// ---- Generators ----

/// Any JSON value serde_json reads back exactly: whole numbers and finite fractions.
fn json() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::from),
        any::<i64>().prop_map(Value::from),
        any::<u64>().prop_map(Value::from),
        any::<f64>()
            .prop_filter("finite", |f| f.is_finite())
            .prop_map(Value::from),
        text().prop_map(Value::from),
    ];
    leaf.prop_recursive(3, 12, 4, |inner| {
        prop_oneof![
            vec(inner.clone(), 0..4).prop_map(Value::from),
            btree_map(text(), inner, 0..4).prop_map(|m| Value::Object(m.into_iter().collect())),
        ]
    })
}

/// Unknown fields. The names start with "zz", which no known field does.
fn extra() -> impl Strategy<Value = Extra> {
    btree_map("zz[a-z]{0,3}", json(), 0..2)
}

/// Any text: control characters, quotes, characters outside the Basic Multilingual Plane.
fn text() -> impl Strategy<Value = String> {
    prop_oneof![
        vec(any::<char>(), 0..10).prop_map(|c| c.into_iter().collect()),
        "[a-z \"\\\\\n\u{1F600}é]{0,10}",
    ]
}

/// Any text a cell or a tag name may have: no control characters (decision A32).
fn cell_text() -> impl Strategy<Value = String> {
    text().prop_map(|t| wombat_core::text::clean(&t))
}

/// Any valid image file name; the pattern alone also gives names such as `a.` and `NUL`.
fn image_name() -> impl Strategy<Value = String> {
    "[A-Za-z0-9_-][A-Za-z0-9._-]{0,8}".prop_filter("a usable file name", |name| {
        wombat_core::model::is_valid_image_name(name)
    })
}

fn color() -> impl Strategy<Value = Color> {
    prop_oneof![
        Just(Color::WHITE),
        Just(Color::BLACK),
        Just(Color::BORDER),
        any::<(u8, u8, u8)>().prop_map(|(r, g, b)| Color::rgb(r, g, b)),
    ]
}

fn style() -> impl Strategy<Value = Style> {
    prop_oneof![
        Just(Style::default()),
        any::<[bool; 5]>().prop_map(|[bold, italic, fixed, underline, strike]| Style {
            bold,
            italic,
            fixed,
            underline,
            strike
        }),
    ]
}

fn id() -> impl Strategy<Value = Id> {
    any::<u128>().prop_map(Id::from_u128)
}

fn target() -> impl Strategy<Value = LinkTarget> {
    prop_oneof![
        "[a-z:/.]{1,12}".prop_map(LinkTarget::External),
        (proptest::option::of(id()), id()).prop_map(|(doc, cell)| LinkTarget::Cell {
            doc: doc.map(DocId),
            cell: CellId(cell)
        }),
        Just(LinkTarget::Unknown),
    ]
}

/// Ranges as (gap before, length); they are laid out in order and cut off at the text's end.
fn ranges<T: std::fmt::Debug>(
    payload: impl Strategy<Value = T>,
) -> impl Strategy<Value = Vec<(u32, u32, T)>> {
    vec((0u32..3, 1u32..4, payload), 0..4)
}

fn lay_out<T>(ranges: Vec<(u32, u32, T)>, len: usize) -> Vec<(u32, u32, T)> {
    let mut pos = 0u32;
    let mut out = Vec::new();
    for (gap, count, payload) in ranges {
        let start = pos + gap;
        if (start + count) as usize > len {
            break;
        }
        out.push((start, count, payload));
        pos = start + count;
    }
    out
}

#[derive(Clone, Debug)]
struct GridSpec {
    xs: u32,
    ys: u32,
    border: Color,
    spacing: u32,
    vertical: bool,
    widths: Vec<u32>,
    extra: Extra,
    cells: Vec<CellSpec>,
}

/// The style, colour and unknown fields of a run.
type RunSpec = (Style, Option<Color>, Extra);

#[derive(Clone, Debug)]
struct CellSpec {
    text: String,
    note: String,
    kind: CellKind,
    colors: (Color, Color),
    draw: DrawStyle,
    aligns: (TextAlign, VertAlign),
    size: i32,
    style: Style,
    runs: Vec<(u32, u32, RunSpec)>,
    links: Vec<(u32, u32, (LinkTarget, Extra))>,
    edited: Option<i64>,
    image: Option<(String, f64, Extra)>,
    extra: Extra,
    grid: Option<Box<GridSpec>>,
}

fn pick<T: Copy + std::fmt::Debug + 'static>(all: &'static [T]) -> impl Strategy<Value = T> {
    // Weighted towards the default, which is first, as in real documents.
    prop_oneof![3 => Just(all[0]), 1 => proptest::sample::select(all)]
}

fn cell_spec() -> impl Strategy<Value = CellSpec> {
    let leaf = (
        (
            cell_text(),
            prop_oneof![Just(String::new()), text()],
            pick(CellKind::ALL),
        ),
        (
            (color(), color()),
            pick(DrawStyle::ALL),
            (pick(TextAlign::ALL), pick(VertAlign::ALL)),
        ),
        (
            prop_oneof![Just(0), any::<i32>()],
            style(),
            proptest::option::of(any::<i64>()),
        ),
        ranges((style(), proptest::option::of(color()), extra())),
        ranges((target(), extra())),
        proptest::option::of((image_name(), 0.01f64..100.0, extra())),
        extra(),
    )
        .prop_map(
            |(
                (text, note, kind),
                (colors, draw, aligns),
                (size, style, edited),
                runs,
                links,
                image,
                extra,
            )| {
                CellSpec {
                    text,
                    note,
                    kind,
                    colors,
                    draw,
                    aligns,
                    size,
                    style,
                    runs,
                    links,
                    edited,
                    image,
                    extra,
                    grid: None,
                }
            },
        );
    leaf.prop_recursive(4, 48, 6, |inner| {
        (
            inner.clone(),
            (1u32..4, 1u32..4),
            vec(inner, 9),
            (
                color(),
                prop_oneof![Just(3u32), any::<u32>()],
                any::<bool>(),
            ),
            vec(prop_oneof![Just(80u32), any::<u32>()], 3),
            extra(),
        )
            .prop_map(
                |(
                    mut cell,
                    (xs, ys),
                    mut cells,
                    (border, spacing, vertical),
                    mut widths,
                    extra,
                )| {
                    cells.truncate((xs * ys) as usize);
                    widths.truncate(xs as usize);
                    cell.grid = Some(Box::new(GridSpec {
                        xs,
                        ys,
                        border,
                        spacing,
                        vertical,
                        widths,
                        extra,
                        cells,
                    }));
                    cell
                },
            )
    })
}

#[derive(Clone, Debug)]
struct DocSpec {
    seed: u64,
    root: CellSpec,
    tags: BTreeMap<String, (Color, Color, Extra)>,
    extra: Extra,
}

fn doc_spec() -> impl Strategy<Value = DocSpec> {
    (
        any::<u64>(),
        cell_spec(),
        btree_map(cell_text(), (color(), color(), extra()), 0..3),
        extra(),
    )
        .prop_map(|(seed, root, tags, extra)| DocSpec {
            seed,
            root,
            tags,
            extra,
        })
}

fn fill(doc: &mut Document, id: CellId, spec: &CellSpec, ids: &mut SeededIds) {
    let len = spec.text.chars().count();
    let cell = doc.cell_mut(id).unwrap();
    cell.text = spec.text.clone();
    cell.note = spec.note.clone();
    cell.kind = spec.kind;
    (cell.cell_color, cell.text_color) = spec.colors;
    cell.draw_style = spec.draw;
    (cell.text_align, cell.vert_align) = spec.aligns;
    cell.rel_size = spec.size;
    cell.style = spec.style;
    cell.last_edit = spec.edited;
    cell.runs = lay_out(spec.runs.clone(), len)
        .into_iter()
        .map(|(start, len, (style, color, extra))| Run {
            start,
            len,
            style,
            color,
            extra,
        })
        .collect();
    cell.links = lay_out(spec.links.clone(), len)
        .into_iter()
        .map(|(start, len, (target, extra))| Link {
            start,
            len,
            target,
            extra,
        })
        .collect();
    cell.image = spec
        .image
        .clone()
        .map(|(file, scale, extra)| ImageRef { file, scale, extra });
    cell.extra = spec.extra.clone();
    if let Some(grid_spec) = &spec.grid {
        doc.add_grid(id, grid_spec.xs, grid_spec.ys, ids).unwrap();
        let grid = doc.cell_mut(id).unwrap().grid_mut().unwrap();
        grid.border_color = grid_spec.border;
        grid.outer_spacing = grid_spec.spacing;
        grid.vertical = grid_spec.vertical;
        grid.col_widths = grid_spec.widths.clone();
        grid.extra = grid_spec.extra.clone();
        let children = grid.cells().to_vec();
        for (child, child_spec) in children.iter().zip(&grid_spec.cells) {
            fill(doc, *child, child_spec, ids);
        }
    }
}

fn build(spec: &DocSpec) -> Document {
    let mut ids = SeededIds::from_u64(spec.seed);
    let mut doc = Document::new(&mut ids);
    let root = doc.root();
    fill(&mut doc, root, &spec.root, &mut ids);
    for (name, (cell_color, text_color, extra)) in &spec.tags {
        let tag = Tag {
            cell_color: *cell_color,
            text_color: *text_color,
            extra: extra.clone(),
        };
        doc.tags_mut().insert(name.clone(), tag);
    }
    *doc.extra_mut() = spec.extra.clone();
    doc
}

// ---- Mutations of valid files ----

#[derive(Clone, Debug)]
enum ByteEdit {
    Replace(usize, u8),
    Insert(usize, Vec<u8>),
    Delete(usize, usize),
    Truncate(usize),
    Repeat(usize, usize),
}

fn byte_edit() -> impl Strategy<Value = ByteEdit> {
    prop_oneof![
        (any::<usize>(), any::<u8>()).prop_map(|(at, b)| ByteEdit::Replace(at, b)),
        (any::<usize>(), vec(any::<u8>(), 1..6)).prop_map(|(at, b)| ByteEdit::Insert(at, b)),
        (any::<usize>(), "[\\[\\]{}\",:0-9 ]{1,6}")
            .prop_map(|(at, s)| ByteEdit::Insert(at, s.into_bytes())),
        (any::<usize>(), 1usize..40).prop_map(|(at, n)| ByteEdit::Delete(at, n)),
        any::<usize>().prop_map(ByteEdit::Truncate),
        (any::<usize>(), 1usize..80).prop_map(|(at, n)| ByteEdit::Repeat(at, n)),
    ]
}

fn apply(bytes: &mut Vec<u8>, edit: &ByteEdit) {
    let len = bytes.len().max(1);
    match edit {
        ByteEdit::Replace(at, b) => {
            if let Some(slot) = bytes.get_mut(at % len) {
                *slot = *b;
            }
        }
        ByteEdit::Insert(at, new) => {
            let at = at % len;
            bytes.splice(at..at, new.iter().copied());
        }
        ByteEdit::Delete(at, n) => {
            let at = at % len;
            let end = (at + n).min(bytes.len());
            bytes.drain(at.min(end)..end);
        }
        ByteEdit::Truncate(at) => bytes.truncate(at % len),
        ByteEdit::Repeat(at, n) => {
            let at = at % len;
            let end = (at + n).min(bytes.len());
            let copy = bytes[at.min(end)..end].to_vec();
            bytes.splice(end..end, copy);
        }
    }
}

/// The value at the end of a path of choices: at each step, the n-th member or element.
fn node<'a>(value: &'a mut Value, path: &[usize]) -> &'a mut Value {
    let Some((first, rest)) = path.split_first() else {
        return value;
    };
    let descend = match value {
        Value::Object(map) => !map.is_empty(),
        Value::Array(items) => !items.is_empty(),
        _ => false,
    };
    if !descend {
        return value;
    }
    match value {
        Value::Object(map) => {
            let key = map.keys().nth(first % map.len()).unwrap().clone();
            node(map.get_mut(&key).unwrap(), rest)
        }
        Value::Array(items) => {
            let n = first % items.len();
            node(&mut items[n], rest)
        }
        _ => unreachable!(),
    }
}

#[derive(Clone, Debug)]
enum TreeEdit {
    /// Put an arbitrary value somewhere.
    Set(Vec<usize>, Value),
    /// Copy one part of the file over another: duplicates IDs, moves cells into runs, etc.
    Copy(Vec<usize>, Vec<usize>),
    /// Remove a member or element.
    Remove(Vec<usize>, usize),
}

fn tree_edit() -> impl Strategy<Value = TreeEdit> {
    let path = || vec(any::<usize>(), 0..10);
    let plausible = prop_oneof![
        json(),
        prop_oneof![
            Just(0u64),
            Just(1),
            Just(2),
            Just(u32::MAX as u64),
            Just(u64::MAX)
        ]
        .prop_map(Value::from),
        Just(Value::from(-1)),
        Just(Value::from("")),
        Just(Value::Array(vec![])),
        Just(Value::Object(Default::default())),
    ];
    prop_oneof![
        (path(), plausible).prop_map(|(p, v)| TreeEdit::Set(p, v)),
        (path(), path()).prop_map(|(a, b)| TreeEdit::Copy(a, b)),
        (path(), any::<usize>()).prop_map(|(p, n)| TreeEdit::Remove(p, n)),
    ]
}

fn apply_tree(value: &mut Value, edit: &TreeEdit) {
    match edit {
        TreeEdit::Set(path, new) => *node(value, path) = new.clone(),
        TreeEdit::Copy(from, to) => {
            let copy = node(value, from).clone();
            *node(value, to) = copy;
        }
        TreeEdit::Remove(path, n) => match node(value, path) {
            Value::Object(map) if !map.is_empty() => {
                let key = map.keys().nth(n % map.len()).unwrap().clone();
                map.remove(&key);
            }
            Value::Array(items) if !items.is_empty() => {
                items.remove(n % items.len());
            }
            _ => {}
        },
    }
}

/// The JSON tokens of a file Wombat wrote: strings, numbers and names, and punctuation.
fn tokens(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        match bytes[i] {
            b' ' | b'\n' => {
                i += 1;
                continue;
            }
            b'"' => {
                i += 1;
                while bytes[i] != b'"' {
                    i += if bytes[i] == b'\\' { 2 } else { 1 };
                }
                i += 1;
            }
            b'{' | b'}' | b'[' | b']' | b',' | b':' => i += 1,
            _ => {
                while i < bytes.len() && !b" \n{}[],:\"".contains(&bytes[i]) {
                    i += 1;
                }
            }
        }
        out.push(text[start..i].to_string());
    }
    out
}

/// Where the value that starts at token `at` ends (one past its last token).
fn value_end(tokens: &[String], at: usize) -> usize {
    let mut depth = 0usize;
    for (i, token) in tokens.iter().enumerate().skip(at) {
        match token.as_str() {
            "{" | "[" => depth += 1,
            "}" | "]" => depth -= 1,
            _ => {}
        }
        if depth == 0 {
            return i + 1;
        }
    }
    tokens.len()
}

/// The tokens that are the key of a member.
fn keys(tokens: &[String]) -> Vec<usize> {
    (0..tokens.len().saturating_sub(1))
        .filter(|i| tokens[i + 1] == ":")
        .collect()
}

/// The tokens at which a value starts.
fn values(tokens: &[String]) -> Vec<usize> {
    (0..tokens.len())
        .filter(|&i| {
            !matches!(tokens[i].as_str(), "}" | "]" | "," | ":")
                && tokens.get(i + 1).is_none_or(|next| next != ":")
        })
        .collect()
}

/// A change to a valid file made on its JSON tokens, so that the result is still JSON (or
/// nearly) and reaches the checks behind the parser.
#[derive(Clone, Debug)]
enum TokenEdit {
    /// Replace a value, whatever it is, by one of another type.
    SwapType(usize, &'static str),
    /// Replace a number by another number.
    Number(usize, &'static str),
    /// Remove a member with its key.
    DeleteMember(usize),
    /// Repeat a member, so that its object has the key twice.
    DuplicateMember(usize),
    /// Give a member the key of another member.
    RenameKey(usize, usize),
    /// Wrap a value in arrays or objects, this many levels.
    Nest(usize, usize, bool),
}

fn token_edit() -> impl Strategy<Value = TokenEdit> {
    let n = any::<usize>;
    let other = proptest::sample::select(vec![
        "0",
        "1",
        "-1",
        "1.5",
        "true",
        "false",
        "null",
        "\"\"",
        "\"x\"",
        "[]",
        "{}",
        "[0]",
        "\"00000000-0000-0000-0000-000000000001\"",
        "\"#12345\"",
    ]);
    let number = proptest::sample::select(vec![
        "0",
        "1",
        "2",
        "-1",
        "-0",
        "0.5",
        "1e2",
        "1E400",
        "4294967295",
        "4294967296",
        "9223372036854775807",
        "9223372036854775808",
        "18446744073709551616",
        "1.0",
        "01",
    ]);
    prop_oneof![
        (n(), other).prop_map(|(at, new)| TokenEdit::SwapType(at, new)),
        (n(), number).prop_map(|(at, new)| TokenEdit::Number(at, new)),
        n().prop_map(TokenEdit::DeleteMember),
        n().prop_map(TokenEdit::DuplicateMember),
        (n(), n()).prop_map(|(at, from)| TokenEdit::RenameKey(at, from)),
        (
            n(),
            prop_oneof![1usize..4, 20usize..40, 90usize..140],
            any::<bool>()
        )
            .prop_map(|(at, levels, object)| TokenEdit::Nest(at, levels, object)),
    ]
}

fn apply_token(tokens: &mut Vec<String>, edit: &TokenEdit) {
    let pick = |list: Vec<usize>, n: usize| (!list.is_empty()).then(|| list[n % list.len()]);
    match edit {
        TokenEdit::SwapType(n, new) => {
            if let Some(at) = pick(values(tokens), *n) {
                let end = value_end(tokens, at);
                tokens.splice(at..end, [new.to_string()]);
            }
        }
        TokenEdit::Number(n, new) => {
            let numbers = values(tokens)
                .into_iter()
                .filter(|&i| tokens[i].starts_with(|c: char| c == '-' || c.is_ascii_digit()))
                .collect();
            if let Some(at) = pick(numbers, *n) {
                tokens[at] = new.to_string();
            }
        }
        TokenEdit::DeleteMember(n) => {
            if let Some(at) = pick(keys(tokens), *n) {
                let end = value_end(tokens, at + 2);
                if tokens.get(end).is_some_and(|t| t == ",") {
                    tokens.drain(at..=end);
                } else {
                    let start = if tokens[at - 1] == "," { at - 1 } else { at };
                    tokens.drain(start..end);
                }
            }
        }
        TokenEdit::DuplicateMember(n) => {
            if let Some(at) = pick(keys(tokens), *n) {
                let end = value_end(tokens, at + 2);
                let mut copy = vec![",".to_string()];
                copy.extend_from_slice(&tokens[at..end]);
                tokens.splice(end..end, copy);
            }
        }
        TokenEdit::RenameKey(n, from) => {
            let keys = keys(tokens);
            if let (Some(at), Some(from)) = (pick(keys.clone(), *n), pick(keys, *from)) {
                tokens[at] = tokens[from].clone();
            }
        }
        TokenEdit::Nest(n, levels, object) => {
            if let Some(at) = pick(values(tokens), *n) {
                let end = value_end(tokens, at);
                let (open, close): (&[&str], &str) = if *object {
                    (&["{", "\"zz\"", ":"], "}")
                } else {
                    (&["["], "]")
                };
                tokens.splice(end..end, vec![close.to_string(); *levels]);
                let opens = open.repeat(*levels);
                tokens.splice(at..at, opens.iter().map(|t| t.to_string()));
            }
        }
    }
}

/// Whatever the loader accepts must be a sound document: valid, and stable under save/load.
fn check_loaded(bytes: &[u8]) -> Result<bool, TestCaseError> {
    match format::from_slice(bytes) {
        Err(_) => Ok(false),
        Ok(doc) => {
            prop_assert!(doc.validate().is_ok());
            prop_assert_eq!(doc.cells_in_order().len(), doc.cell_count());
            let text = format::to_string(&doc).unwrap();
            let again = format::from_slice(text.as_bytes()).unwrap();
            prop_assert_eq!(&again, &doc);
            prop_assert_eq!(format::to_string(&again).unwrap(), text);
            Ok(true)
        }
    }
}

/// Adds an unknown field to every object of the file that the format defines.
fn add_unknown_everywhere(value: &mut Value, unknown: &Value, count: &mut usize) {
    fn cell(value: &mut Value, unknown: &Value, count: &mut usize) {
        mark(value, unknown, count);
        for list in ["runs", "links"] {
            if let Some(Value::Array(items)) = value.get_mut(list) {
                items.iter_mut().for_each(|item| mark(item, unknown, count));
            }
        }
        if let Some(image) = value.get_mut("image") {
            mark(image, unknown, count);
        }
        if let Some(grid) = value.get_mut("grid") {
            mark(grid, unknown, count);
            if let Some(Value::Array(cells)) = grid.get_mut("cells") {
                cells.iter_mut().for_each(|c| cell(c, unknown, count));
            }
        }
    }
    fn mark(value: &mut Value, unknown: &Value, count: &mut usize) {
        value
            .as_object_mut()
            .unwrap()
            .insert("zzunknown".into(), unknown.clone());
        *count += 1;
    }
    mark(value, unknown, count);
    if let Some(Value::Object(tags)) = value.get_mut("tags") {
        tags.values_mut().for_each(|tag| mark(tag, unknown, count));
    }
    cell(&mut value["root"], unknown, count);
}

// ---- The rules ----

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    #[test]
    fn save_then_load_gives_an_equal_document(spec in doc_spec()) {
        let doc = build(&spec);
        prop_assert!(doc.validate().is_ok());
        let text = format::to_string(&doc).unwrap();
        let loaded = format::from_slice(text.as_bytes()).unwrap();
        prop_assert_eq!(&loaded, &doc);
        // And the file is plain JSON that names every cell exactly once.
        prop_assert!(serde_json::from_str::<Value>(&text).is_ok());
        for id in doc.cells_in_order() {
            prop_assert_eq!(text.matches(&format!("\"id\": \"{id}\"")).count(), 1);
        }
    }

    #[test]
    fn saving_is_deterministic(spec in doc_spec()) {
        let doc = build(&spec);
        let text = format::to_string(&doc).unwrap();
        prop_assert_eq!(&format::to_string(&doc.clone()).unwrap(), &text);
        // Built again from nothing (so with differently ordered hash maps): the same bytes.
        prop_assert_eq!(&format::to_string(&build(&spec)).unwrap(), &text);
        // Loaded and saved again: the same bytes.
        let loaded = format::from_slice(text.as_bytes()).unwrap();
        prop_assert_eq!(&format::to_string(&loaded).unwrap(), &text);
    }

    #[test]
    fn unknown_fields_survive_load_then_save(spec in doc_spec(), unknown in json()) {
        let text = format::to_string(&build(&spec)).unwrap();
        let mut file: Value = serde_json::from_str(&text).unwrap();
        let mut count = 0;
        add_unknown_everywhere(&mut file, &unknown, &mut count);
        file["minor"] = Value::from(7);
        file["zzfuture"] = unknown.clone();
        let doc = format::from_slice(file.to_string().as_bytes()).unwrap();
        prop_assert!(count >= 2 && count >= doc.cell_count());
        let saved: Value = serde_json::from_str(&format::to_string(&doc).unwrap()).unwrap();
        prop_assert_eq!(saved, file);
    }

    #[test]
    fn loading_arbitrary_bytes_never_panics(bytes in vec(any::<u8>(), 0..600)) {
        check_loaded(&bytes)?;
        let _ = ViewState::from_json(&bytes);
    }

    #[test]
    fn loading_arbitrary_json_never_panics(value in json(), id1 in id(), id2 in id()) {
        check_loaded(value.to_string().as_bytes())?;
        let _ = ViewState::from_json(value.to_string().as_bytes());
        // The same value where a document, a cell, a grid and a view state are expected.
        let wrapped = serde_json::json!({
            "format": "wombat", "version": 1, "id": id1.to_string(), "tags": value,
            "root": {"id": id2.to_string(), "text": "abc", "runs": value, "links": value,
                     "image": value, "grid": value, "zz": value},
        });
        check_loaded(wrapped.to_string().as_bytes())?;
        let grid = serde_json::json!({
            "format": "wombat", "version": 1, "id": id1.to_string(),
            "root": {"id": id2.to_string(), "grid": {"xs": 1, "ys": 1, "cells": [value]}},
        });
        check_loaded(grid.to_string().as_bytes())?;
        let view = serde_json::json!({
            "format": "wombat-view", "version": 1, "doc": id1.to_string(),
            "selection": value, "zoom": value, "folded": value, "levels": value,
        });
        let _ = ViewState::from_json(view.to_string().as_bytes());
        // And inside what each level remembers.
        let levels = serde_json::json!({
            "format": "wombat-view", "version": 1, "doc": id1.to_string(),
            "levels": [
                {"cell": id2.to_string(), "x": value, "y": value, "selected": value, "zz": value},
                {"cell": value},
                value,
            ],
        });
        let _ = ViewState::from_json(levels.to_string().as_bytes());
    }

    #[test]
    fn loading_a_file_with_damaged_bytes_never_panics(
        spec in doc_spec(),
        edits in vec(byte_edit(), 1..4),
    ) {
        let mut bytes = format::to_string(&build(&spec)).unwrap().into_bytes();
        for edit in &edits {
            apply(&mut bytes, edit);
        }
        check_loaded(&bytes)?;
    }

    #[test]
    fn loading_a_file_with_damaged_structure_never_panics(
        spec in doc_spec(),
        edits in vec(tree_edit(), 1..4),
    ) {
        let text = format::to_string(&build(&spec)).unwrap();
        let mut file: Value = serde_json::from_str(&text).unwrap();
        for edit in &edits {
            apply_tree(&mut file, edit);
        }
        check_loaded(file.to_string().as_bytes())?;
    }

    #[test]
    fn loading_a_file_with_changed_tokens_never_panics(
        spec in doc_spec(),
        edits in vec(token_edit(), 1..4),
    ) {
        let text = format::to_string(&build(&spec)).unwrap();
        let mut tokens = tokens(&text);
        // The tokens are the file: put together again they are the same document.
        prop_assert!(check_loaded(tokens.concat().as_bytes())?);
        for edit in &edits {
            apply_token(&mut tokens, edit);
        }
        check_loaded(tokens.concat().as_bytes())?;
    }

    #[test]
    fn view_state_round_trips(
        doc in id(),
        folded in vec(id(), 0..6),
        selection in proptest::option::of((id(), any::<u32>(), any::<u32>(),
            proptest::option::of((any::<u32>(), any::<u32>())), extra())),
        zoom in proptest::option::of(id()),
        levels in vec((id(), any::<u32>(), any::<u32>(), proptest::option::of(id()), extra()), 0..6),
        unknown in extra(),
    ) {
        let mut view = ViewState::new(DocId(doc));
        view.folded = folded.into_iter().map(CellId).collect();
        view.selection = selection.map(|(cell, xs, ys, cursor, extra)| Selection {
            cell: CellId(cell),
            xs,
            ys,
            cursor: cursor.map(|(a, b)| (a.min(b), a.max(b))),
            extra,
        });
        view.zoom = zoom.map(CellId);
        for (cell, x, y, selected, extra) in levels {
            // No level twice: that is not written.
            if view.level(CellId(cell)).is_none() {
                view.levels.push(Level {
                    cell: CellId(cell),
                    // Small numbers too, so that the defaults that are left out come up.
                    x: x % 3,
                    y: if y.is_multiple_of(2) { y } else { 0 },
                    selected: selected.map(CellId),
                    extra,
                });
            }
        }
        view.extra = unknown;
        let text = view.to_json().unwrap();
        let loaded = ViewState::from_json(text.as_bytes()).unwrap();
        prop_assert_eq!(&loaded, &view);
        prop_assert_eq!(loaded.to_json().unwrap(), text);
    }
}

/// The damage tests are only worth something if some damaged files still load and some do
/// not; this checks the generators on fixed examples of both.
#[test]
fn the_damage_generators_reach_both_outcomes() {
    let mut ids = SeededIds::from_u64(9);
    let mut doc = Document::new(&mut ids);
    doc.add_grid(doc.root(), 2, 1, &mut ids).unwrap();
    let text = format::to_string(&doc).unwrap();
    let mut file: Value = serde_json::from_str(&text).unwrap();

    // root -> grid -> cells -> second cell -> id, copied from the first: a duplicate ID.
    let root = file
        .as_object()
        .unwrap()
        .keys()
        .position(|k| k == "root")
        .unwrap();
    let first = [root, 0, 0, 0, 0];
    let second = [root, 0, 0, 1, 0];
    assert!(node(&mut file, &first).is_string(), "{file}");
    apply_tree(&mut file, &TreeEdit::Copy(first.to_vec(), second.to_vec()));
    assert!(!check_loaded(file.to_string().as_bytes()).unwrap());

    let mut harmless: Value = serde_json::from_str(&text).unwrap();
    let minor = harmless
        .as_object()
        .unwrap()
        .keys()
        .position(|k| k == "minor")
        .unwrap();
    apply_tree(&mut harmless, &TreeEdit::Remove(vec![], minor));
    assert!(harmless.get("minor").is_none());
    assert!(check_loaded(harmless.to_string().as_bytes()).unwrap());

    let mut bytes = text.clone().into_bytes();
    let half = bytes.len() / 2;
    apply(&mut bytes, &ByteEdit::Truncate(half));
    assert!(!check_loaded(&bytes).unwrap());
}

/// The same for the changes made on tokens: each kind does what it says, and the outcomes
/// that the format's rules require follow.
#[test]
fn the_token_edits_reach_both_outcomes() {
    let mut ids = SeededIds::from_u64(9);
    let mut doc = Document::new(&mut ids);
    doc.add_grid(doc.root(), 2, 1, &mut ids).unwrap();
    let root = doc.root();
    doc.cell_mut(root).unwrap().text = "abc".into();
    doc.cell_mut(root)
        .unwrap()
        .extra
        .insert("zz".into(), serde_json::json!([1]));
    let text = format::to_string(&doc).unwrap();
    let original = tokens(&text);
    assert_eq!(
        serde_json::from_str::<Value>(&original.concat()).unwrap(),
        serde_json::from_str::<Value>(&text).unwrap()
    );
    let key = |name: &str| {
        let quoted = format!("\"{name}\"");
        let keys = keys(&original);
        keys.iter().position(|&i| original[i] == quoted).unwrap()
    };
    let value = |name: &str| {
        let quoted = format!("\"{name}\"");
        let at = keys(&original)[key(name)] + 2;
        assert_eq!(original[at - 2], quoted);
        values(&original).iter().position(|&i| i == at).unwrap()
    };
    let outcome = |edit: TokenEdit| {
        let mut tokens = original.clone();
        apply_token(&mut tokens, &edit);
        let text = tokens.concat();
        assert_ne!(tokens, original, "{edit:?} changed nothing");
        (check_loaded(text.as_bytes()).unwrap(), text)
    };
    // A text that is a number, and a column count that is a text: refused.
    assert!(!outcome(TokenEdit::SwapType(value("text"), "1")).0);
    assert!(!outcome(TokenEdit::SwapType(value("xs"), "\"x\"")).0);
    // An unknown value may be anything.
    assert!(outcome(TokenEdit::SwapType(value("zz"), "{}")).0);
    // Numbers: a later minor version loads; fractions, exponents and overflows do not.
    let minor = 1; // The numbers in order: version, minor, the unknown field's, xs, ys.
    assert!(outcome(TokenEdit::Number(minor, "4294967295")).0);
    for bad in ["4294967296", "1.0", "1e2", "-1", "01"] {
        assert!(!outcome(TokenEdit::Number(minor, bad)).0, "{bad}");
    }
    assert!(!outcome(TokenEdit::Number(0, "2")).0);
    // Members: one that has a default can go, a required one cannot; none may be repeated.
    let (loads, without) = outcome(TokenEdit::DeleteMember(key("text")));
    assert!(loads && !without.contains("abc"));
    assert!(outcome(TokenEdit::DeleteMember(key("minor"))).0);
    assert!(!outcome(TokenEdit::DeleteMember(key("ys"))).0);
    assert!(!outcome(TokenEdit::DeleteMember(key("root"))).0);
    for name in ["format", "text", "zz", "xs", "root"] {
        let (loads, twice) = outcome(TokenEdit::DuplicateMember(key(name)));
        assert!(!loads, "{name}");
        assert_eq!(twice.matches(&format!("\"{name}\":")).count(), 2, "{name}");
    }
    assert!(!outcome(TokenEdit::RenameKey(key("text"), key("zz"))).0);
    assert!(!outcome(TokenEdit::RenameKey(key("ys"), key("xs"))).0);
    // Nesting: an unknown value may be deeper up to the limit; a known one may not be at all.
    let deep = wombat_core::limits::MAX_UNKNOWN_DEPTH as usize;
    assert!(outcome(TokenEdit::Nest(value("zz"), deep - 1, false)).0);
    assert!(outcome(TokenEdit::Nest(value("zz"), deep - 1, true)).0);
    assert!(!outcome(TokenEdit::Nest(value("zz"), deep, false)).0);
    assert!(!outcome(TokenEdit::Nest(value("zz"), 500, true)).0);
    assert!(!outcome(TokenEdit::Nest(value("root"), 1, false)).0);
    assert!(!outcome(TokenEdit::Nest(value("root"), 500, false)).0);
}
