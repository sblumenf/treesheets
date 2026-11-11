//! `wombat export` and the text formats of `wombat import`: reads the options and the
//! files; the conversions are `wombat_core::convert`, the writing `export_files`.

use std::path::Path;

use wombat_core::convert::lookup::{self, LookupOptions};
use wombat_core::convert::{self, Delimiter, ExportOptions, Quoting, Source, Table};
use wombat_core::eval::Calc;
use wombat_core::limits::MAX_DEPTH;
use wombat_core::{export_files, storage};

use crate::Failure;

/// The text of `wombat export --help`.
pub const EXPORT_HELP: &str = "\
wombat export <file> --format FORMAT [--cell CELL] [options] <output>

Exports the grid of a cell (default: the root cell, the whole document) of a .wombat
document, or of a TreeSheets .cts file read on the spot. The document is not changed.
CELL is a place such as /2,1 or an ID, as in `wombat edit`. An <output> of - prints to
standard output (not for lookup).

Formats
  csv      the grid as one table, a line per row; of a cell with a grid inside only its
           text is exported (TreeSheets' \"CSV (flat)\")
  tsv      the same, separated by tabs
  flat     the grid and every grid nested in it as one table: a line per innermost row,
           after the rows it is inside of (project, task, ...)
  lookup   <output> is a folder: one CSV file per grid, index.csv and README.txt. Every
           table has ID columns, so nested tables can be joined to their parents with
           XLOOKUP; README.txt has the formula for the exported files
  text     TreeSheets' indented text: a line per cell, two spaces per level. Lines end
           in CR LF, as TreeSheets writes them on Linux and Windows, or with --lf in LF,
           as on macOS (of the options below only --lf and --overwrite apply)

Options (csv, tsv, flat, lookup)
  --delimiter comma|semicolon|tab   what separates fields (default comma; tsv: tab)
  --formulas     export formula cells as their formulas, not as what they show
  --notes        add a column with the notes after every column
  --raw          do not protect against formula injection. By default a text that starts
                 with = + - or @ (and is not just a number) gets a ' in front, so that
                 no spreadsheet runs it as a formula when it opens the file
  --no-bom       do not start the file with a UTF-8 byte-order mark (Excel needs the
                 mark to show non-ASCII text right; some other programs show it as junk)
  --lf           end lines with LF instead of CR LF
  --quote-all    put every field in double quotes
  --treesheets   the bytes TreeSheets itself writes: --raw --no-bom --lf, and for csv
                 --quote-all
  --header-row   (lookup) name the columns after the first row of each grid instead of
                 A, B, C, ...
Options (all)
  --overwrite    replace an existing file, or write into a folder that is not empty.
                 A symbolic link is never written through
";

/// The text of `wombat import --help`.
pub const IMPORT_HELP: &str = "\
wombat import <file> <new.wombat> [--format FORMAT] [--formulas]

Makes a new document from another file. The format is taken from the file's name unless
--format says it:

  .cts            a TreeSheets file, with its images and view state
  .csv            csv: comma separated values
  .tsv .tab       tsv: tab separated values
  (--format semicolon)  values separated by semicolons, as spreadsheets write CSV where
                  the comma is the decimal sign
  .txt            text: indented text; a line indented further than the one before it
                  starts a grid inside that one

Delimited text is read as RFC 4180 says (quoted fields may hold the delimiter, doubled
quotes and line breaks; a line break inside a field becomes a space, since cell text has
none); rows are padded with empty cells to a rectangle. UTF-8, UTF-16 with a byte-order
mark and Windows-1252 are understood.

  --formulas      make cells whose text starts with = formula cells. By default they stay
                  plain text: a file from somewhere else should not start calculating

To import into a cell of an existing document, use `import` in `wombat edit`.
";

/// The options of an export as given on the command line.
struct Flags {
    options: ExportOptions,
    lookup: LookupOptions,
    overwrite: bool,
    /// `--lf` itself, for the indented text, which `--treesheets` does not change.
    lf: bool,
    cell: Option<String>,
    format: Option<String>,
    rest: Vec<String>,
}

fn usage(message: impl Into<String>) -> Failure {
    Failure::Usage(message.into())
}

fn delimiter(word: &str) -> Result<Delimiter, Failure> {
    match word {
        "comma" | "," => Ok(Delimiter::Comma),
        "semicolon" | ";" => Ok(Delimiter::Semicolon),
        "tab" => Ok(Delimiter::Tab),
        _ => Err(usage(format!(
            "{word:?} is not a delimiter: comma, semicolon or tab"
        ))),
    }
}

fn flags(args: &[&str]) -> Result<Flags, Failure> {
    let mut flags = Flags {
        options: ExportOptions::default(),
        lookup: LookupOptions::default(),
        overwrite: false,
        lf: false,
        cell: None,
        format: None,
        rest: Vec::new(),
    };
    let mut chosen = None;
    let mut treesheets = false;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let mut value = |what: &str| {
            args.next()
                .map(|word| word.to_string())
                .ok_or_else(|| usage(format!("{arg} needs {what}")))
        };
        match *arg {
            "--format" => flags.format = Some(value("a format")?),
            "--cell" => flags.cell = Some(value("a cell")?),
            "--delimiter" => chosen = Some(delimiter(&value("a delimiter")?)?),
            "--formulas" => flags.options.formulas = true,
            "--notes" => flags.options.notes = true,
            "--raw" => flags.options.guard = false,
            "--no-bom" => flags.options.bom = false,
            "--lf" => {
                flags.options.crlf = false;
                flags.lf = true;
            }
            "--quote-all" => flags.options.quoting = Quoting::All,
            "--treesheets" => treesheets = true,
            "--header-row" => flags.lookup.header_row = true,
            "--overwrite" => flags.overwrite = true,
            word if word.starts_with("--") => {
                return Err(usage(format!("unknown option {word}")));
            }
            word => flags.rest.push(word.to_string()),
        }
    }
    let tsv = flags.format.as_deref() == Some("tsv");
    flags.options.delimiter = chosen.unwrap_or(if tsv {
        Delimiter::Tab
    } else {
        Delimiter::Comma
    });
    if treesheets {
        let like = ExportOptions::treesheets(flags.options.delimiter);
        flags.options = ExportOptions {
            formulas: flags.options.formulas,
            notes: flags.options.notes,
            ..like
        };
    }
    Ok(flags)
}

/// `wombat export`.
pub fn export(args: &[&str]) -> Result<String, Failure> {
    let flags = flags(args)?;
    let [file, output] = flags.rest.as_slice() else {
        return Err(usage("export needs a document and an output"));
    };
    let Some(format) = flags.format.as_deref() else {
        return Err(usage(
            "export needs --format csv, tsv, flat, lookup or text",
        ));
    };
    let (doc, _) = crate::load_for_show(file)?;
    let cell = match &flags.cell {
        Some(word) => crate::links::find_cell(&doc, word).map_err(Failure::Failed)?,
        None => doc.root(),
    };
    let calc = Calc::new(&doc);
    let source = Source {
        doc: &doc,
        calc: &calc,
    };
    let options = &flags.options;
    let text = match format {
        "csv" | "tsv" => convert::grid_csv(source, cell, options)?,
        "flat" => convert::flat_csv(source, cell, options)?,
        "text" => convert::outline(source, cell, true, !flags.lf)?,
        "lookup" => {
            if output == "-" {
                return Err(usage("a lookup export is a folder, not standard output"));
            }
            let files = lookup::export(source, cell, options, flags.lookup)?;
            let parts: Vec<(&str, &[u8])> = files
                .iter()
                .map(|f| (f.name.as_str(), f.text.as_bytes()))
                .collect();
            export_files::write_folder(Path::new(output), &parts, flags.overwrite)?;
            let mut out = format!(
                "exported {file} to the folder {output}: {} tables\n",
                files.len().saturating_sub(2)
            );
            for f in &files {
                out.push_str(&format!("  {}\n", f.name));
            }
            return Ok(out);
        }
        other => {
            return Err(usage(format!(
                "{other:?} is not a format: csv, tsv, flat, lookup or text"
            )))
        }
    };
    if output == "-" {
        return Ok(text);
    }
    export_files::write_file(Path::new(output), text.as_bytes(), flags.overwrite)?;
    Ok(format!(
        "exported {file} as {output}: {format}, {} lines\n",
        text.lines().count()
    ))
}

/// What a file to import is, by `--format` or else by the end of its name.
fn kind<'a>(file: &'a str, format: Option<&'a str>) -> Result<&'a str, Failure> {
    let extension = Path::new(file)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    match (format, extension.as_deref()) {
        (Some(format @ ("csv" | "tsv" | "semicolon" | "text" | "cts")), _) => Ok(format),
        (Some(other), _) => Err(usage(format!(
            "{other:?} is not a format: csv, tsv, semicolon, text or cts"
        ))),
        (None, Some("cts")) => Ok("cts"),
        (None, Some("csv")) => Ok("csv"),
        (None, Some("tsv" | "tab")) => Ok("tsv"),
        (None, Some("txt")) => Ok("text"),
        (None, _) => Err(usage(format!(
            "{file}: say what it is with --format csv, tsv, semicolon or text"
        ))),
    }
}

/// Reads a file to import as a table. `levels` is how deep its grids may nest.
pub fn read_table(
    file: &str,
    format: Option<&str>,
    formulas: bool,
    levels: u32,
) -> Result<Table, Failure> {
    let kind = kind(file, format)?;
    let text = convert::decode(&storage::read_bounded(Path::new(file))?);
    let table = match kind {
        "csv" => convert::parse_table(&text, Delimiter::Comma, formulas),
        "semicolon" => convert::parse_table(&text, Delimiter::Semicolon, formulas),
        "tsv" => convert::parse_table(&text, Delimiter::Tab, formulas),
        "text" => convert::parse_outline(&text, levels, formulas),
        _ => {
            return Err(Failure::Failed(format!(
                "{file}: a TreeSheets file is a whole document, not a table"
            )))
        }
    };
    table.map_err(|e| Failure::Failed(format!("{file}: {e}")))
}

/// `wombat import`: a TreeSheets file, or delimited or indented text, as a new document.
pub fn import(args: &[&str]) -> Result<String, Failure> {
    let (mut format, mut formulas, mut rest) = (None, false, Vec::new());
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match *arg {
            "--format" => {
                format = Some(
                    *args
                        .next()
                        .ok_or_else(|| usage("--format needs a format"))?,
                );
            }
            "--formulas" => formulas = true,
            word if word.starts_with("--") => {
                return Err(usage(format!("unknown option {word}")));
            }
            word => rest.push(word),
        }
    }
    let [source, file] = rest.as_slice() else {
        return Err(usage("import needs a file and the name of a new document"));
    };
    if kind(source, format)? == "cts" {
        if formulas {
            return Err(usage(
                "--formulas is for text files; a .cts file has its own",
            ));
        }
        return crate::import(source, file);
    }
    let path = crate::new_path(file)?;
    let table = read_table(source, format, formulas, MAX_DEPTH)?;
    let doc = convert::to_document(&table, &mut crate::os_ids()?)?;
    storage::save_document(&doc, path)?;
    Ok(format!(
        "imported {source} as {file}: document {}, {} cells, {} columns by {} rows at the top\n",
        doc.id(),
        doc.cell_count(),
        table.xs(),
        table.ys()
    ))
}
