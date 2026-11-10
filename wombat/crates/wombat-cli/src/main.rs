//! The `wombat` command-line tool: a thin layer over the core.

mod outline;

use std::io::{Read, Write};
use std::path::Path;
use std::process::ExitCode;

use wombat_core::limits::MAX_FILE_BYTES;
use wombat_core::{storage, CellId, CellKind, Document, SeededIds, Workspace};

const USAGE: &str = "\
wombat: nested-grid documents

Usage:
  wombat new <file.wombat>                  create an empty document
  wombat new <file.wombat> --from <outline> create a document from an indented text outline
                                            (\"-\" reads the outline from standard input)
  wombat show <file.wombat> [--ids]         print the document as an indented outline
  wombat check <file.wombat>                check that the file is a valid document
  wombat list [<folder>]                    list the documents of a workspace folder
  wombat help                               show this text

Outline format: one line per row of a grid, cells separated by \" | \"; lines indented
further than a row become the grid inside that row's first cell.
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
        ["list"] => list("."),
        ["list", folder] => list(folder),
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

fn new(file: &str, outline_file: Option<&str>) -> Result<String, Failure> {
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

fn show(file: &str, with_ids: bool) -> Result<String, Failure> {
    let doc = storage::load_document(Path::new(file))?;
    let mut out = format!("document {} ({} cells)\n", doc.id(), doc.cell_count());
    for tag in doc.tags.keys() {
        out.push_str(&format!("tag {tag:?}\n"));
    }
    show_cell(&doc, doc.root(), 0, "", with_ids, &mut out);
    Ok(out)
}

/// Prints a cell, then its grid: a "grid" line and one line per cell, marked [column,row].
fn show_cell(
    doc: &Document,
    id: CellId,
    indent: usize,
    label: &str,
    with_ids: bool,
    out: &mut String,
) {
    let Some(cell) = doc.cell(id) else { return };
    let pad = " ".repeat(indent);
    out.push_str(&format!("{pad}{label}{:?}", cell.text));
    if cell.kind != CellKind::Data {
        out.push_str(&format!(" ({})", cell.kind.as_str()));
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
        out.push_str(&format!("{pad}  grid {} x {}\n", grid.xs(), grid.ys()));
        for y in 0..grid.ys() {
            for x in 0..grid.xs() {
                if let Some(child) = grid.at(x, y) {
                    let label = format!("[{},{}] ", x.saturating_add(1), y.saturating_add(1));
                    show_cell(doc, child, inner, &label, with_ids, out);
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

fn list(folder: &str) -> Result<String, Failure> {
    let mut out = String::new();
    for entry in Workspace::open(folder)?.list()? {
        match entry.id {
            Ok(id) => out.push_str(&format!("{id}  {}\n", entry.name)),
            Err(error) => out.push_str(&format!("{:36}  {}  ({error})\n", "damaged", entry.name)),
        }
    }
    Ok(out)
}
