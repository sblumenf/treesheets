//! The referee comparison: every example `.cts` and the fixture is imported and compared,
//! cell by cell and property by property, with what the real TreeSheets read from the same
//! file (`referee/golden/<name>.json`). Needs no TreeSheets at test time.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use wombat_core::{
    storage, Cell, CellId, CellKind, Document, DrawStyle, LinkTarget, SeededIds, Style, TextAlign,
    VertAlign,
};
use wombat_cts::{import, Imported, UNRESOLVED_TARGET_FIELD};

fn referee_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../referee")
}

/// A set of `.cts` files with the golden data the real TreeSheets produced for them.
#[derive(Clone, Copy, PartialEq)]
enum Set {
    /// The 37 examples of TreeSheets and the referee's fixture (`referee/golden`).
    Examples,
    /// Hand-built files for what no example exercises (`tests/quirks`, see its README).
    Quirks,
}

fn quirks_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/quirks")
}

/// The `.cts` file a manifest name stands for.
fn cts_path(set: Set, name: &str) -> PathBuf {
    match (set, name.strip_prefix("fixtures/")) {
        (Set::Quirks, _) => quirks_dir().join(format!("{name}.cts")),
        (Set::Examples, Some(fixture)) => referee_dir().join(format!("fixtures/{fixture}.cts")),
        (Set::Examples, None) => referee_dir().join(format!("../../TS/examples/{name}.cts")),
    }
}

fn golden_dir(set: Set) -> PathBuf {
    match set {
        Set::Examples => referee_dir().join("golden"),
        Set::Quirks => quirks_dir().join("golden"),
    }
}

/// The files of a set by name, with what the referee's manifest says about each. The quirk
/// files have no manifest (the referee writes one only for its own golden folder): their
/// names are those of the golden files, and the entry is null.
fn manifest(set: Set) -> BTreeMap<String, Value> {
    if set == Set::Quirks {
        return fs::read_dir(golden_dir(set))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|e| e == "json"))
            .map(|path| {
                (
                    path.file_stem().unwrap().to_str().unwrap().to_string(),
                    Value::Null,
                )
            })
            .collect();
    }
    let text = fs::read_to_string(golden_dir(set).join("MANIFEST.json")).unwrap();
    let json: Value = serde_json::from_str(&text).unwrap();
    json["files"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

fn golden(set: Set, name: &str) -> Value {
    let text = fs::read_to_string(golden_dir(set).join(format!("{name}.json"))).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// Counts, per property, how many values were compared and collects the differences.
#[derive(Default)]
struct Report {
    compared: BTreeMap<&'static str, usize>,
    mismatches: Vec<String>,
}

impl Report {
    fn check<T: PartialEq + std::fmt::Debug>(
        &mut self,
        property: &'static str,
        place: &str,
        ours: T,
        theirs: T,
    ) {
        *self.compared.entry(property).or_default() += 1;
        if ours != theirs {
            self.mismatches.push(format!(
                "{place}: {property}: Wombat {ours:?}, TreeSheets {theirs:?}"
            ));
        }
    }
}

fn style_bits(style: Style) -> u64 {
    u64::from(style.bold)
        | u64::from(style.italic) << 1
        | u64::from(style.fixed) << 2
        | u64::from(style.underline) << 3
        | u64::from(style.strike) << 4
}

fn kind_name(kind: CellKind) -> &'static str {
    match kind {
        CellKind::Data => "data",
        CellKind::Code => "operation",
        CellKind::VarAssign => "variable_assign",
        CellKind::ViewH => "view_horizontal",
        CellKind::VarRead => "variable_read",
        CellKind::ViewV => "view_vertical",
        CellKind::Formula => "formula",
    }
}

fn draw_name(style: DrawStyle) -> &'static str {
    match style {
        DrawStyle::Grid => "grid",
        DrawStyle::Bubbles => "bubble",
        DrawStyle::Lines => "line",
    }
}

fn align_name(align: TextAlign) -> &'static str {
    align.as_str()
}

fn valign_name(align: VertAlign) -> &'static str {
    align.as_str()
}

/// The golden cell at a path, for the independent link search.
fn golden_at<'a>(root: &'a Value, path: &[(u64, u64)]) -> &'a Value {
    let mut cell = root;
    for (x, y) in path {
        let grid = &cell["grid"];
        let index = y * grid["cols"].as_u64().unwrap() + x;
        cell = &grid["cells"][index as usize];
    }
    cell
}

/// TreeSheets' rule for the target of an internal link, written out the slow and obvious way
/// from `FormulaEvaluator::LinkTarget` / `ForNearest` over the *golden* tree: this is what
/// the importer's indexed search is checked against. Returns the path of the target.
fn golden_link_target(root: &Value, from: &[(u64, u64)], name: &str) -> Option<Vec<(u64, u64)>> {
    let mut scope = from.to_vec();
    while !scope.is_empty() {
        scope.pop();
        let mut level = vec![scope.clone()];
        while !level.is_empty() {
            let mut next = Vec::new();
            for path in &level {
                let cell = golden_at(root, path);
                if path.as_slice() != from && !path.is_empty() && cell["text"] == name {
                    return Some(path.clone());
                }
                if let Some(cells) = cell["grid"]["cells"].as_array() {
                    let cols = cell["grid"]["cols"].as_u64().unwrap();
                    for i in 0..cells.len() as u64 {
                        let mut child = path.clone();
                        child.push((i % cols, i / cols));
                        next.push(child);
                    }
                }
            }
            level = next;
        }
    }
    None
}

struct Compare<'a> {
    name: &'a str,
    imported: &'a Imported,
    golden_root: &'a Value,
    paths: HashMap<CellId, Vec<(u64, u64)>>,
    report: &'a mut Report,
    cells: usize,
    unresolved: Vec<String>,
}

impl Compare<'_> {
    fn doc(&self) -> &Document {
        &self.imported.document
    }

    /// The style and colour of every code point, runs applied.
    fn our_styles(&self, cell: &Cell) -> Vec<(u64, String)> {
        let mut styles =
            vec![(style_bits(cell.style), cell.text_color.to_string()); cell.text.chars().count()];
        for run in &cell.runs {
            for slot in &mut styles[run.start as usize..(run.start + run.len) as usize] {
                *slot = (
                    style_bits(run.style),
                    run.color.unwrap_or(cell.text_color).to_string(),
                );
            }
        }
        styles
    }

    fn their_styles(&self, theirs: &Value) -> Vec<(u64, String)> {
        let fg = theirs["fg"].as_str().unwrap().to_lowercase();
        let count = theirs["text"].as_str().unwrap().chars().count();
        let mut styles = vec![(theirs["stylebits"].as_u64().unwrap(), fg); count];
        for run in theirs["runs"].as_array().unwrap() {
            let start = run["start"].as_u64().unwrap() as usize;
            let len = run["len"].as_u64().unwrap() as usize;
            for slot in &mut styles[start..start + len] {
                *slot = (
                    run["stylebits"].as_u64().unwrap(),
                    run["fg"].as_str().unwrap().to_lowercase(),
                );
            }
        }
        styles
    }

    /// Per code point: is it in a link, is the link internal, and the target (for an internal
    /// link: the text of the cell it was resolved to, or the kept text if there is none).
    fn our_links(&mut self, id: CellId, cell: &Cell, place: &str) -> Vec<Option<(bool, String)>> {
        let mut links = vec![None; cell.text.chars().count()];
        for link in &cell.links {
            let value = match &link.target {
                LinkTarget::External(url) => (false, url.clone()),
                LinkTarget::Cell { doc, cell: to } => {
                    assert!(doc.is_none());
                    let target = self.doc().cell(*to).expect("a link goes to a missing cell");
                    (true, target.text.clone())
                }
                LinkTarget::Unknown => {
                    let text = link.extra[UNRESOLVED_TARGET_FIELD].as_str().unwrap();
                    (true, text.to_string())
                }
            };
            // Independently of the importer: where does TreeSheets' rule lead in the golden
            // tree? It must be the same cell, or nowhere for both.
            if value.0 {
                let from = self.paths[&id].clone();
                let expected = golden_link_target(self.golden_root, &from, &value.1);
                let ours = match &link.target {
                    LinkTarget::Cell { cell: to, .. } => Some(self.paths[to].clone()),
                    _ => None,
                };
                self.report
                    .check("link target cell", place, ours.clone(), expected);
                if ours.is_none() {
                    self.unresolved.push(format!("{place} -> {:?}", value.1));
                }
            }
            for slot in &mut links[link.start as usize..(link.start + link.len) as usize] {
                *slot = Some(value.clone());
            }
        }
        links
    }

    fn their_links(&self, theirs: &Value) -> Vec<Option<(bool, String)>> {
        let mut links = vec![None; theirs["text"].as_str().unwrap().chars().count()];
        for link in theirs["links"].as_array().unwrap() {
            let start = link["start"].as_u64().unwrap() as usize;
            let len = link["len"].as_u64().unwrap() as usize;
            for slot in &mut links[start..start + len] {
                *slot = Some((
                    link["internal"].as_bool().unwrap(),
                    link["target"].as_str().unwrap().to_string(),
                ));
            }
        }
        links
    }

    fn cell(&mut self, id: CellId, theirs: &Value) {
        self.cells += 1;
        let path = self.paths[&id].clone();
        let place = format!("{} {:?}", self.name, path);
        let place = place.as_str();
        let doc = &self.imported.document;
        let cell = doc.cell(id).unwrap();
        let str_of = |key: &str| theirs[key].as_str().unwrap();
        let golden_path: Vec<(u64, u64)> = theirs["path"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| (p[0].as_u64().unwrap(), p[1].as_u64().unwrap()))
            .collect();
        assert_eq!(path, golden_path, "{place}: the walk is out of step");

        let formula = cell.kind == CellKind::Formula;
        self.report
            .check("type", place, kind_name(cell.kind), str_of("type"));
        if formula {
            // The golden text of a formula cell is its calculated result (milestone 5).
            self.report.check(
                "formula",
                place,
                Some(cell.text.as_str()),
                theirs["formula"].as_str(),
            );
        } else {
            self.report
                .check("text", place, cell.text.as_str(), str_of("text"));
            self.report
                .check("formula", place, None, theirs["formula"].as_str());
        }
        self.report
            .check("note", place, cell.note.as_str(), str_of("note"));
        self.report.check(
            "bg",
            place,
            cell.cell_color.to_string(),
            str_of("bg").to_lowercase(),
        );
        self.report.check(
            "fg",
            place,
            cell.text_color.to_string(),
            str_of("fg").to_lowercase(),
        );
        // The dump has the opposite sign.
        self.report.check(
            "relsize",
            place,
            -i64::from(cell.rel_size),
            theirs["relsize"].as_i64().unwrap(),
        );
        self.report.check(
            "stylebits",
            place,
            style_bits(cell.style),
            theirs["stylebits"].as_u64().unwrap(),
        );
        self.report
            .check("align", place, align_name(cell.text_align), str_of("align"));
        self.report.check(
            "valign",
            place,
            valign_name(cell.vert_align),
            str_of("valign"),
        );
        self.report.check(
            "render_style",
            place,
            draw_name(cell.draw_style),
            str_of("render_style"),
        );
        self.report.check(
            "has_image",
            place,
            cell.image.is_some(),
            theirs["has_image"].as_bool().unwrap(),
        );
        self.report.check(
            "last_edit",
            place,
            cell.last_edit,
            theirs["last_edit"].as_i64(),
        );
        self.report.check(
            "is_tag",
            place,
            doc.tags().contains_key(&cell.text),
            theirs["is_tag"].as_bool().unwrap(),
        );
        assert!(!theirs["filtered"].as_bool().unwrap());
        if formula {
            // TreeSheets has no runs or links on the result that it shows for a formula.
            self.report.check("runs", place, cell.runs.len(), 0);
            self.report.check("links", place, cell.links.len(), 0);
        } else {
            let (ours, golden_styles) = (self.our_styles(cell), self.their_styles(theirs));
            self.report.check("runs", place, ours, golden_styles);
            let ours = self.our_links(id, cell, place);
            let golden_links = self.their_links(theirs);
            self.report.check("links", place, ours, golden_links);
        }

        let their_grid = &theirs["grid"];
        self.report.check(
            "has grid",
            place,
            cell.grid().is_some(),
            !their_grid.is_null(),
        );
        let (Some(grid), false) = (cell.grid(), their_grid.is_null()) else {
            return;
        };
        self.report.check(
            "grid size",
            place,
            (u64::from(grid.xs()), u64::from(grid.ys())),
            (
                their_grid["cols"].as_u64().unwrap(),
                their_grid["rows"].as_u64().unwrap(),
            ),
        );
        // Wombat keeps this on the grid (decision B10), so only cells with a grid have it.
        self.report.check(
            "vertical_layout",
            place,
            grid.vertical,
            theirs["vertical_layout"].as_bool().unwrap(),
        );
        self.report.check(
            "folded",
            place,
            self.imported.view.folded.contains(&id),
            their_grid["folded"].as_bool().unwrap(),
        );
        self.report.check(
            "border_color",
            place,
            grid.border_color.to_string(),
            their_grid["border_color"].as_str().unwrap().to_lowercase(),
        );
        let widths: Vec<u64> = their_grid["col_widths"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| w.as_u64().unwrap())
            .collect();
        let ours: Vec<u64> = grid.col_widths.iter().map(|w| u64::from(*w)).collect();
        self.report.check("col_widths", place, ours, widths);
        let their_cells = their_grid["cells"].as_array().unwrap();
        if grid.cells().len() != their_cells.len() {
            return;
        }
        for (child, their_child) in grid.cells().to_vec().into_iter().zip(their_cells) {
            self.cell(child, their_child);
        }
    }
}

/// The path of every cell: column and row in each grid from the root down.
fn paths(doc: &Document) -> HashMap<CellId, Vec<(u64, u64)>> {
    let mut paths = HashMap::new();
    paths.insert(doc.root(), Vec::new());
    for id in doc.cells_in_order() {
        let path: Vec<(u64, u64)> = paths[&id].clone();
        if let Some(grid) = doc.cell(id).unwrap().grid() {
            for y in 0..grid.ys() {
                for x in 0..grid.xs() {
                    let mut child = path.clone();
                    child.push((u64::from(x), u64::from(y)));
                    paths.insert(grid.at(x, y).unwrap(), child);
                }
            }
        }
    }
    paths
}

#[test]
fn every_example_matches_what_treesheets_read() {
    let deepest = compare_set(Set::Examples, 38, 8198);
    // Wombat's nesting limit (decision A20) holds every example.
    assert_eq!(deepest, 15);
    assert!(deepest <= wombat_core::limits::MAX_DEPTH);
}

#[test]
fn every_quirk_file_matches_what_treesheets_read() {
    compare_set(Set::Quirks, 11, 83);
}

/// Imports every file of a set and compares it with its golden data; returns the deepest
/// nesting found.
fn compare_set(set: Set, files: usize, cells: usize) -> u32 {
    let manifest = manifest(set);
    assert_eq!(manifest.len(), files);
    let mut report = Report::default();
    let mut total_cells = 0;
    let mut deepest = (0, String::new());
    let mut versions = BTreeMap::new();
    let mut unresolved = Vec::new();
    let mut undecodable = Vec::new();
    for (name, entry) in &manifest {
        let bytes = fs::read(cts_path(set, name)).unwrap();
        if !entry.is_null() {
            assert_eq!(
                bytes.len() as u64,
                entry["bytes"].as_u64().unwrap(),
                "{name}: the example changed since the golden data was made"
            );
        }
        let imported = import(&bytes, &mut SeededIds::from_u64(2))
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let golden = golden(set, name);
        let doc = &imported.document;
        doc.validate().unwrap();
        *versions.entry(imported.version).or_insert(0) += 1;
        if imported.max_depth > deepest.0 {
            deepest = (imported.max_depth, name.clone());
        }
        if imported.undecodable_strings > 0 {
            undecodable.push(format!("{name}: {}", imported.undecodable_strings));
        }

        if !entry.is_null() {
            report.check(
                "format version",
                name,
                u64::from(imported.version),
                entry["format_version"].as_u64().unwrap(),
            );
            report.check(
                "cell count",
                name,
                doc.cell_count() as u64,
                entry["stats"]["cells"].as_u64().unwrap(),
            );
        }
        // Old files have no such setting; TreeSheets then shows the app's own (off).
        report.check(
            "autoexport",
            name,
            (u64::from(imported.autoexport_html), imported.autoexport_pdf),
            (
                golden["autoexport_html"].as_u64().unwrap(),
                golden["autoexport_pdf"].as_bool().unwrap(),
            ),
        );
        let paths = paths(doc);
        // The selection: where in its grid, and its size. The dump does not say which grid.
        let selection = imported.view.selection.as_ref().unwrap();
        let (x, y) = *paths[&selection.cell].last().unwrap();
        let theirs = &golden["selection"];
        report.check(
            "selection",
            name,
            (x, y, u64::from(selection.xs), u64::from(selection.ys)),
            (
                theirs["x"].as_u64().unwrap(),
                theirs["y"].as_u64().unwrap(),
                theirs["cols"].as_u64().unwrap(),
                theirs["rows"].as_u64().unwrap(),
            ),
        );

        let mut compare = Compare {
            name,
            imported: &imported,
            golden_root: &golden["root"],
            paths,
            report: &mut report,
            cells: 0,
            unresolved: Vec::new(),
        };
        compare.cell(doc.root(), &golden["root"]);
        total_cells += compare.cells;
        unresolved.append(&mut compare.unresolved);
        assert_eq!(
            imported.unresolved_links.len(),
            imported
                .document
                .cells_in_order()
                .iter()
                .flat_map(|id| &doc.cell(*id).unwrap().links)
                .filter(|link| link.target == LinkTarget::Unknown)
                .count()
        );
    }

    println!(
        "referee comparison: {} files, {total_cells} cells",
        manifest.len()
    );
    println!("format versions (version: files): {versions:?}");
    println!("deepest nesting: {} in {}", deepest.0, deepest.1);
    println!("values compared per property: {:#?}", report.compared);
    println!("links without a target (kept by text): {unresolved:#?}");
    println!("files with strings that are not UTF-8: {undecodable:#?}");
    assert!(
        report.mismatches.is_empty(),
        "{} mismatches, the first 40:\n{}",
        report.mismatches.len(),
        report
            .mismatches
            .iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert_eq!(total_cells, cells);
    deepest.0
}

#[test]
fn every_example_survives_saving_and_loading_as_wombat() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cts-referee");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let names = [Set::Examples, Set::Quirks]
        .into_iter()
        .flat_map(|set| manifest(set).into_keys().map(move |name| (set, name)));
    for (number, (set, name)) in names.enumerate() {
        let bytes = fs::read(cts_path(set, &name)).unwrap();
        let imported = import(&bytes, &mut SeededIds::from_u64(number as u64)).unwrap();
        // The same seed gives the same document, IDs included.
        let again = import(&bytes, &mut SeededIds::from_u64(number as u64)).unwrap();
        assert_eq!(imported, again, "{name}");

        let text = wombat_core::format::to_string(&imported.document).unwrap();
        let reloaded = wombat_core::format::from_slice(text.as_bytes()).unwrap();
        assert_eq!(reloaded, imported.document, "{name}");
        assert_eq!(wombat_core::format::to_string(&reloaded).unwrap(), text);

        let path = dir.join(format!("{number}.wombat"));
        imported.save(&path).unwrap();
        assert_eq!(storage::load_document(&path).unwrap(), imported.document);
        let view = storage::load_view_state(&dir, imported.document.id()).unwrap();
        assert_eq!(view.as_ref(), Some(&imported.view), "{name}");
        assert!(storage::missing_images(&imported.document, &path).is_empty());
        for image in &imported.images {
            let file = storage::image_path(&path, &image.file).unwrap();
            if image.used {
                assert_eq!(fs::read(file).unwrap(), image.data, "{name}");
                assert!(image.data.starts_with(b"\x89PNG"), "{name}");
                assert!(image.data.ends_with(b"IEND\xae\x42\x60\x82"), "{name}");
            } else {
                assert!(!file.exists());
            }
        }
    }
}
