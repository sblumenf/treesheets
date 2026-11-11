//! The `wombat` command-line tool: a thin layer over the core.

mod edit;
mod outline;

use std::io::{Read, Write};
use std::path::Path;
use std::process::ExitCode;

use wombat_core::eval::Calc;
use wombat_core::limits::MAX_FILE_BYTES;
use wombat_core::{storage, CellId, CellKind, Document, SeededIds, ViewState, Workspace};

const USAGE: &str = "\
wombat: nested-grid documents

Usage:
  wombat new <file.wombat>                  create an empty document
  wombat new <file.wombat> --from <outline> create a document from an indented text outline
                                            (\"-\" reads the outline from standard input)
  wombat show <file> [--ids]                print a document (.wombat, or a TreeSheets .cts
                                            read on the spot) as an indented outline; a
                                            formula cell as its formula = what it shows
  wombat import <file.cts> <file.wombat>    convert a TreeSheets file: writes the document,
                                            its images and its view state (folds, selection)
  wombat copy <file.wombat> <new.wombat>    save a copy that has its own document and cell
                                            IDs, with its images and view state (copying
                                            the file itself would duplicate the IDs)
  wombat edit <file.wombat> --script <file> run editing commands, one per line (\"-\" reads
                                            them from standard input); `wombat edit --help`
                                            lists the commands
  wombat check <file.wombat>                check that the file is a valid document
  wombat list [<folder>] [--cells]          list the documents of a workspace folder and warn
                                            of files with the same document ID; --cells also
                                            loads every document to find shared cell IDs
  wombat help                               show this text

Outline format: one line per row of a grid, cells separated by \" | \"; lines indented
further than a row become the grid inside that row's first cell. A cell that starts with =
is a formula cell, for example =SUM(Costs) * 2 (see docs/milestones/M5.md).
";

/// What went wrong: a mistake in the command line (exit code 2) or a failed command (1).
enum Failure {
    Usage(String),
    Failed(String),
}

impl From<wombat_core::Error> for Failure {
    fn from(error: wombat_core::Error) -> Failure {
        Failure::Failed(error.to_string())
    }
}

impl From<wombat_cts::CtsError> for Failure {
    fn from(error: wombat_cts::CtsError) -> Failure {
        Failure::Failed(error.to_string())
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match run(&args) {
        Ok(output) => {
            // A closed pipe (for example `wombat show | head`) is not an error worth a panic.
            let _ = std::io::stdout().lock().write_all(output.as_bytes());
            ExitCode::SUCCESS
        }
        Err(Failure::Usage(message)) => {
            eprintln!("wombat: {message}\n\n{USAGE}");
            ExitCode::from(2)
        }
        Err(Failure::Failed(message)) => {
            eprintln!("wombat: {message}");
            ExitCode::from(1)
        }
    }
}

fn run(args: &[&str]) -> Result<String, Failure> {
    match args {
        [] | ["help" | "--help" | "-h"] => Ok(USAGE.to_string()),
        ["--version"] => Ok(format!("wombat {}\n", wombat_core::VERSION)),
        ["new", file] => new(file, None),
        ["new", file, "--from", outline] | ["new", "--from", outline, file] => {
            new(file, Some(outline))
        }
        ["show", file] => show(file, false),
        ["show", file, "--ids"] | ["show", "--ids", file] => show(file, true),
        ["check", file] => check(file),
        ["edit", "--help" | "-h" | "help"] => Ok(edit::HELP.to_string()),
        ["edit", file, "--script", script] | ["edit", "--script", script, file] => {
            edit(file, script)
        }
        ["import", source, file] => import(source, file),
        ["copy", source, file] => copy(source, file),
        ["list"] => list(".", false),
        ["list", "--cells"] => list(".", true),
        ["list", folder, "--cells"] | ["list", "--cells", folder] => list(folder, true),
        ["list", folder] => list(folder, false),
        _ => Err(Failure::Usage("unknown command or wrong arguments".into())),
    }
}

/// An ID source seeded from the operating system, so that IDs are unique across documents.
fn os_ids() -> Result<SeededIds, Failure> {
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed)
        .map_err(|e| Failure::Failed(format!("no randomness from the operating system: {e}")))?;
    Ok(SeededIds::from_seed(seed))
}

/// The path of a document that is about to be created: named `.wombat` and not there yet.
fn new_path(file: &str) -> Result<&Path, Failure> {
    let path = Path::new(file);
    if path.extension().is_none_or(|e| e != storage::EXTENSION) {
        return Err(Failure::Usage(format!(
            "{file}: a document's name ends in .wombat"
        )));
    }
    if path.exists() {
        return Err(Failure::Failed(format!(
            "{file} already exists; it was not changed"
        )));
    }
    Ok(path)
}

fn new(file: &str, outline_file: Option<&str>) -> Result<String, Failure> {
    let path = new_path(file)?;
    let mut ids = os_ids()?;
    let doc = match outline_file {
        None => Document::new(&mut ids),
        Some(source) => {
            let text = read_text(source)?;
            let rows =
                outline::parse(&text).map_err(|e| Failure::Failed(format!("{source}: {e}")))?;
            outline::to_document(&rows, &mut ids)?
        }
    };
    storage::save_document(&doc, path)?;
    Ok(format!(
        "created {file}: document {}, {} cells\n",
        doc.id(),
        doc.cell_count()
    ))
}

fn read_text(source: &str) -> Result<String, Failure> {
    let bytes = if source == "-" {
        let mut bytes = Vec::new();
        std::io::stdin()
            .lock()
            .take(MAX_FILE_BYTES)
            .read_to_end(&mut bytes)
            .map_err(|e| Failure::Failed(format!("standard input: {e}")))?;
        bytes
    } else {
        storage::read_bounded(Path::new(source))?
    };
    String::from_utf8(bytes).map_err(|_| Failure::Failed(format!("{source}: not UTF-8 text")))
}

/// Converts a TreeSheets file into a new Wombat document with its images and view state.
fn import(source: &str, file: &str) -> Result<String, Failure> {
    let path = new_path(file)?;
    let imported = wombat_cts::import_file(Path::new(source), &mut os_ids()?)
        .map_err(|e| Failure::Failed(format!("{source}: {e}")))?;
    imported.save(path)?;
    let doc = &imported.document;
    let mut out = format!(
        "imported {source} (TreeSheets format version {}) as {file}: document {}, {} cells, \
         nested {} deep\n",
        imported.version,
        doc.id(),
        doc.cell_count(),
        imported.max_depth
    );
    let images = imported.images.iter().filter(|image| image.used).count();
    if images > 0 {
        let folder = storage::images_dir(path);
        out.push_str(&format!("{images} images in {}\n", folder.display()));
    }
    let unused = imported.images.len().saturating_sub(images);
    if unused > 0 {
        out.push_str(&format!(
            "note: {unused} images that no cell shows were left out\n"
        ));
    }
    if !imported.view.folded.is_empty() {
        out.push_str(&format!(
            "{} folded grids (kept in the view state, not the document)\n",
            imported.view.folded.len()
        ));
    }
    for link in &imported.unresolved_links {
        out.push_str(&format!(
            "note: cell {} links to a cell with the text {:?}, which does not exist; the link \
             keeps that text\n",
            link.cell, link.target
        ));
    }
    if imported.undecodable_strings > 0 {
        out.push_str(&format!(
            "note: {} texts were not valid UTF-8 and are empty, as in TreeSheets\n",
            imported.undecodable_strings
        ));
    }
    if imported.autoexport_html != 0 || imported.autoexport_pdf {
        out.push_str("note: the file's \"export on every save\" setting was not kept\n");
    }
    Ok(out)
}

/// Loads what `show` prints: a Wombat document with its view state if it has one, or a
/// TreeSheets file, which is imported in memory and not written anywhere.
fn load_for_show(file: &str) -> Result<(Document, Option<ViewState>), Failure> {
    let path = Path::new(file);
    // By name, not content, so that a damaged .wombat file is always reported as one.
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("cts"))
    {
        let imported = wombat_cts::import_file(path, &mut os_ids()?)
            .map_err(|e| Failure::Failed(format!("{file}: {e}")))?;
        return Ok((imported.document, Some(imported.view)));
    }
    let doc = storage::load_document(path)?;
    // View state is a convenience: a damaged one must not stop the document being shown.
    let folder = path.parent().unwrap_or(Path::new(""));
    let view = storage::load_view_state(folder, doc.id()).ok().flatten();
    Ok((doc, view))
}

fn show(file: &str, with_ids: bool) -> Result<String, Failure> {
    let (doc, view) = load_for_show(file)?;
    Ok(show_document(
        &doc,
        &Calc::new(&doc),
        view.as_ref(),
        with_ids,
    ))
}

/// A document as an indented outline.
fn show_document(doc: &Document, calc: &Calc, view: Option<&ViewState>, with_ids: bool) -> String {
    let mut out = format!("document {} ({} cells)\n", doc.id(), doc.cell_count());
    for tag in doc.tags().keys() {
        out.push_str(&format!("tag {tag:?}\n"));
    }
    let showing = Showing {
        doc,
        calc,
        view,
        with_ids,
    };
    show_cell(&showing, doc.root(), 0, "", &mut out);
    out
}

/// Runs a script of editing commands on a document. Output so far is printed even if a
/// later line fails.
fn edit(file: &str, script: &str) -> Result<String, Failure> {
    let text = read_text(script)?;
    let mut session = edit::Session::open(Path::new(file), os_ids()?)
        .map_err(|e| Failure::Failed(format!("{file}: {e}")))?;
    let mut out = String::new();
    for (n, line) in text.lines().enumerate() {
        match session.line(line) {
            Ok(printed) => out.push_str(&printed),
            Err(error) => {
                let _ = std::io::stdout().lock().write_all(out.as_bytes());
                let lost = if session.saved() {
                    ""
                } else {
                    "; changes since the last save were not written"
                };
                return Err(Failure::Failed(format!(
                    "{script} line {}: {error}{lost}",
                    n.saturating_add(1)
                )));
            }
        }
    }
    if !session.saved() {
        out.push_str("not saved: the script changed the document after its last `save`\n");
    }
    Ok(out)
}

/// What `show_cell` prints from.
struct Showing<'a> {
    doc: &'a Document,
    calc: &'a Calc,
    view: Option<&'a ViewState>,
    with_ids: bool,
}

/// Prints a cell, then its grid: a "grid" line and one line per cell, marked [column,row].
/// A formula cell is printed as its formula, then `=` and what it shows.
fn show_cell(showing: &Showing, id: CellId, indent: usize, label: &str, out: &mut String) {
    let Showing {
        doc,
        calc,
        view,
        with_ids,
    } = *showing;
    let Some(cell) = doc.cell(id) else { return };
    let pad = " ".repeat(indent);
    out.push_str(&format!("{pad}{label}{:?}", cell.text));
    if cell.kind != CellKind::Data {
        out.push_str(&format!(" ({})", cell.kind.as_str()));
    }
    if let Some((result, _)) = calc.result(id) {
        out.push_str(&format!(" = {result:?}"));
    }
    if !cell.note.is_empty() {
        out.push_str(&format!(" note: {:?}", cell.note));
    }
    if let Some(image) = &cell.image {
        out.push_str(&format!(" image: {}", image.file));
    }
    if !cell.links.is_empty() {
        out.push_str(&format!(" links: {}", cell.links.len()));
    }
    if with_ids {
        out.push_str(&format!("  #{id}"));
    }
    out.push('\n');
    if let Some(grid) = cell.grid() {
        let inner = indent.saturating_add(2);
        let folded = view.is_some_and(|view| view.folded.contains(&id));
        let folded = if folded { " (folded)" } else { "" };
        out.push_str(&format!(
            "{pad}  grid {} x {}{folded}\n",
            grid.xs(),
            grid.ys()
        ));
        for y in 0..grid.ys() {
            for x in 0..grid.xs() {
                if let Some(child) = grid.at(x, y) {
                    let label = format!("[{},{}] ", x.saturating_add(1), y.saturating_add(1));
                    show_cell(showing, child, inner, &label, out);
                }
            }
        }
    }
}

fn check(file: &str) -> Result<String, Failure> {
    let path = Path::new(file);
    let doc = storage::load_document(path)?;
    let depth = doc
        .cells_in_order()
        .into_iter()
        .filter_map(|id| doc.depth(id))
        .max()
        .unwrap_or(0);
    let mut out = format!(
        "{file}: ok. document {}, {} cells, nested {depth} deep\n",
        doc.id(),
        doc.cell_count()
    );
    for image in storage::missing_images(&doc, path) {
        out.push_str(&format!("{file}: note: image file {image} is missing\n"));
    }
    Ok(out)
}

/// Saves a copy of a document that shares no ID with it.
fn copy(source: &str, file: &str) -> Result<String, Failure> {
    let path = new_path(file)?;
    let doc = storage::copy_document(Path::new(source), path, &mut os_ids()?)?;
    Ok(format!(
        "copied {source} as {file}: document {}, {} cells, all with new IDs\n",
        doc.id(),
        doc.cell_count()
    ))
}

fn list(folder: &str, cells: bool) -> Result<String, Failure> {
    const ADVICE: &str = "one is a copy of the file; make copies with \"wombat copy\"";
    let workspace = Workspace::open(folder)?;
    let mut out = String::new();
    let mut warnings = String::new();
    for entry in workspace.list()? {
        match entry.id {
            Ok(id) => out.push_str(&format!("{id}  {}\n", entry.name)),
            Err(error) => out.push_str(&format!("{:36}  {}  ({error})\n", "damaged", entry.name)),
        }
        // Each pair once, at the first of the two.
        for other in entry.same_id.iter().filter(|other| **other > entry.name) {
            warnings.push_str(&format!(
                "warning: {} and {other} have the same document ID: {ADVICE}\n",
                entry.name
            ));
        }
    }
    if cells {
        for found in workspace.id_collisions()? {
            warnings.push_str(&format!(
                "warning: {} and {} share {} IDs{}: {ADVICE}\n",
                found.first,
                found.second,
                found.shared_ids,
                if found.same_document {
                    ", the document ID among them"
                } else {
                    ""
                }
            ));
        }
    }
    Ok(out + &warnings)
}
