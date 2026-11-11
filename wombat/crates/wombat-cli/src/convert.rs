//! `wombat export` and the text formats of `wombat import`: reads the options and the
//! files; the conversions are `wombat_core::convert`, the writing `export_files`.

use std::path::Path;

use wombat_core::convert::lookup::{self, LookupOptions};
use wombat_core::convert::{self, Delimiter, Encoding, ExportOptions, Quoting, Source, Table};
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
           after the rows it is inside of (project, task, ...). Where a row has grids
           in several of its cells, every line ends with columns that name the column
           of the cell its inner rows are in (--parent-columns: always; --no-parent-columns:
           never)
  lookup   <output> is a folder: one CSV file per grid, index.csv and README.txt. Every
           table has ID columns, so nested tables can be joined to their parents with
           XLOOKUP or INDEX/MATCH; README.txt has the formulas for the exported files,
           for Excel, Google Sheets and LibreOffice
  text     TreeSheets' indented text: a line per cell, two spaces per level. Lines end
           in CR LF, as TreeSheets writes them on Linux and Windows, or with --lf in LF,
           as on macOS (of the options below only --lf and --overwrite apply)

Options (csv, tsv, flat, lookup)
  --delimiter comma|semicolon|tab   what separates fields (default comma; tsv: tab).
                 Excel opens a comma separated file in one column where its list
                 separator is the semicolon (where the comma is the decimal sign): use
                 semicolon for such a system
  --formulas     export formula cells as their formulas, not as what they show
  --notes        add a column with the notes after every column
  --raw          do not protect against formula injection. By default a text that a
                 spreadsheet could run as a formula gets a ' in front, which is then
                 visible in the cell: one that starts with = , or with + - or @ and has
                 a parenthesis, a | or a reference to another sheet in it, or goes on
                 like a calculation or a name (+1+1, -A1). Left alone: -5, - an item,
                 +44 20 7946 0958, @name
  --no-bom       do not start the file with a UTF-8 byte-order mark (Excel needs the
                 mark to show non-ASCII text right; some other programs show it as junk)
  --lf           end lines with LF instead of CR LF
  --quote-all    put every field in double quotes (by default: those with a comma, a
                 semicolon, a tab, a quote or a line break in them)
  --treesheets   the bytes TreeSheets itself writes: --raw --no-bom --lf, and for csv
                 --quote-all
  --header-row   (lookup) name the columns after the first row of each grid instead of
                 A, B, C, ...
Options (all)
  --overwrite    replace an existing regular file, or write again into the folder of an
                 earlier lookup export, whose tables are removed first (other files in
                 it stay). A symbolic link is never written through, and nothing but a
                 regular file is ever replaced

A count of the cells that were exported as error values (#LIMIT!, #NAME? and the like)
is printed if there are any. A spreadsheet that opens a CSV file makes numbers and dates
of what looks like them (00123 becomes 123): import the columns as text where that matters.
";

/// The text of `wombat import --help`.
pub const IMPORT_HELP: &str = "\
wombat import <file> <new.wombat> [--format FORMAT] [options]

Makes a new document from another file. The format is taken from the file's name unless
--format says it:

  .cts            a TreeSheets file, with its images and view state
  .csv            csv: comma separated values. What separates the fields is looked for
                  in the file: semicolons (as spreadsheets write CSV where the comma is
                  the decimal sign) and tabs are found if the lines agree on them;
                  otherwise it is the comma. --delimiter says it outright
  .tsv .tab       tsv: tab separated values
  (--format semicolon)  values separated by semicolons
  .txt            text: indented text; a line indented further than the one before it
                  starts a grid inside that one

Delimited text is read as RFC 4180 says (quoted fields may hold the delimiter, doubled
quotes and line breaks; a line break inside a field becomes a space, since cell text has
none); rows are padded with empty cells to a rectangle.

  --delimiter comma|semicolon|tab
                  what separates the fields, whatever the name or the content say
  --encoding utf-8|utf-16le|utf-16be|windows-1252
                  what the bytes are. By default UTF-8 (with or without a byte-order
                  mark), or UTF-16 if the file starts with its byte-order mark. A file
                  that is neither is refused, with the place of the first wrong byte:
                  nothing is guessed. Excel's plain \"CSV\" on Windows is often
                  windows-1252
  --formulas      make cells whose text starts with = formula cells. By default they stay
                  plain text: a file from somewhere else should not start calculating
  --unguard       take away the ' that `wombat export` puts in front of texts a
                  spreadsheet could run. Only for files that Wombat exported: in other
                  files a text such as '=1+1 may be what its author wrote

The import says what it read the file as. To import into a cell of an existing document,
use `import` in `wombat edit`.
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
            "--parent-columns" => flags.options.parent_columns = Some(true),
            "--no-parent-columns" => flags.options.parent_columns = Some(false),
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
            parent_columns: flags.options.parent_columns,
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
    // What went out as an error value: said after the export, apart from the table.
    let errors = |nested: bool| {
        let count = if options.formulas && format != "text" {
            0
        } else {
            convert::error_cells(source, cell, nested)
        };
        match count {
            0 => String::new(),
            1 => "warning: 1 cell was exported as an error value (such as #NAME? or #LIMIT!)\n"
                .to_string(),
            n => format!(
                "warning: {n} cells were exported as error values (such as #NAME? or #LIMIT!)\n"
            ),
        }
    };
    let warning = errors(!matches!(format, "csv" | "tsv"));
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
            return Ok(out + &warning);
        }
        other => {
            return Err(usage(format!(
                "{other:?} is not a format: csv, tsv, flat, lookup or text"
            )))
        }
    };
    if output == "-" {
        // Standard output is the table and nothing else.
        eprint!("{warning}");
        return Ok(text);
    }
    export_files::write_file(Path::new(output), text.as_bytes(), flags.overwrite)?;
    Ok(format!(
        "exported {file} as {output}: {format}, {} lines\n{warning}",
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

/// How a file to import is read, beyond what its name says.
#[derive(Default)]
pub struct ReadOptions<'a> {
    /// `--format`.
    pub format: Option<&'a str>,
    /// What separates the fields, if said.
    pub delimiter: Option<Delimiter>,
    /// What the bytes are, if said.
    pub encoding: Option<Encoding>,
    /// Whether texts that start with `=` become formula cells.
    pub formulas: bool,
    /// Whether the guard of an export is taken off.
    pub unguard: bool,
}

/// The encoding that a word names.
fn encoding(word: &str) -> Option<Encoding> {
    match word {
        "utf-8" | "utf8" => Some(Encoding::Utf8),
        "utf-16le" => Some(Encoding::Utf16Le),
        "utf-16be" => Some(Encoding::Utf16Be),
        "windows-1252" | "cp1252" => Some(Encoding::Windows1252),
        _ => None,
    }
}

impl ReadOptions<'_> {
    /// Takes `word` as an option of `import` in `wombat edit` if it is a delimiter or an
    /// encoding.
    pub fn word(&mut self, word: &str) -> bool {
        match (word, encoding(word)) {
            ("comma", _) => self.delimiter = Some(Delimiter::Comma),
            ("tab", _) => self.delimiter = Some(Delimiter::Tab),
            (_, Some(encoding)) => self.encoding = Some(encoding),
            _ => return false,
        }
        true
    }
}

/// A file read as a table, and a line that says what it was read as.
pub struct Read {
    /// The table.
    pub table: Table,
    /// "read as UTF-8, fields separated by semicolons (found in the file)".
    pub how: String,
}

/// Reads a file to import as a table. `levels` is how deep its grids may nest.
pub fn read_table(file: &str, options: &ReadOptions, levels: u32) -> Result<Read, Failure> {
    let kind = kind(file, options.format)?;
    if kind == "cts" {
        return Err(Failure::Failed(format!(
            "{file}: a TreeSheets file is a whole document, not a table"
        )));
    }
    let failed = |e: wombat_core::Error| Failure::Failed(format!("{file}: {e}"));
    let bytes = storage::read_bounded(Path::new(file))?;
    let (text, encoding) = convert::decode(&bytes, options.encoding).map_err(|e| {
        // The message alone: what is wrong is the file's encoding, not a document.
        let message = match e {
            wombat_core::Error::Invalid(message) => message,
            other => other.to_string(),
        };
        Failure::Failed(format!(
            "{file}: {message} (--encoding, or the word after the file in `wombat edit`)"
        ))
    })?;
    let mut how = format!("read as {}", encoding.name());
    let table = if kind == "text" {
        how.push_str(", indented text");
        convert::parse_outline(&text, levels, options.formulas)
    } else {
        let by_name = match kind {
            "semicolon" => Delimiter::Semicolon,
            "tsv" => Delimiter::Tab,
            _ => Delimiter::Comma,
        };
        // Only a file called CSV is looked into: that name is used for all three.
        let found = match (options.delimiter, kind) {
            (None, "csv") => convert::delimited::detect(&text),
            _ => None,
        };
        let (delimiter, why) = match (options.delimiter, found) {
            (Some(delimiter), _) => (delimiter, "as asked"),
            (None, Some(delimiter)) => (delimiter, "found in the file"),
            (None, None) if kind == "csv" => (by_name, "nothing else was found in the file"),
            (None, None) => (by_name, "as its format says"),
        };
        let name = match delimiter {
            Delimiter::Comma => "commas",
            Delimiter::Semicolon => "semicolons",
            Delimiter::Tab => "tabs",
        };
        how.push_str(&format!(", fields separated by {name} ({why})"));
        convert::parse_table(&text, delimiter, options.formulas)
    };
    let mut table = table.map_err(failed)?;
    if options.unguard {
        table = table.unguarded(options.formulas);
        how.push_str(", the quotes of the export's guard taken off");
    }
    Ok(Read { table, how })
}

/// `wombat import`: a TreeSheets file, or delimited or indented text, as a new document.
pub fn import(args: &[&str]) -> Result<String, Failure> {
    let mut options = ReadOptions::default();
    let mut rest = Vec::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let mut value = |what: &str| {
            args.next()
                .copied()
                .ok_or_else(|| usage(format!("{arg} needs {what}")))
        };
        match *arg {
            "--format" => options.format = Some(value("a format")?),
            "--delimiter" => options.delimiter = Some(delimiter(value("a delimiter")?)?),
            "--encoding" => {
                let word = value("an encoding")?;
                options.encoding = Some(encoding(word).ok_or_else(|| {
                    usage(format!(
                        "{word:?} is not an encoding: utf-8, utf-16le, utf-16be or windows-1252"
                    ))
                })?);
            }
            "--formulas" => options.formulas = true,
            "--unguard" => options.unguard = true,
            word if word.starts_with("--") => {
                return Err(usage(format!("unknown option {word}")));
            }
            word => rest.push(word),
        }
    }
    let [source, file] = rest.as_slice() else {
        return Err(usage("import needs a file and the name of a new document"));
    };
    if kind(source, options.format)? == "cts" {
        if options.formulas || options.unguard || options.delimiter.is_some() {
            return Err(usage(
                "--formulas, --unguard and --delimiter are for text files; a .cts file needs none",
            ));
        }
        return crate::import(source, file);
    }
    let path = crate::new_path(file)?;
    let read = read_table(source, &options, MAX_DEPTH)?;
    let table = &read.table;
    let doc = convert::to_document(table, &mut crate::os_ids()?)?;
    storage::save_document(&doc, path)?;
    Ok(format!(
        "imported {source} as {file}: document {}, {} cells, {} columns by {} rows at the top\n  {}\n",
        doc.id(),
        doc.cell_count(),
        table.xs(),
        table.ys(),
        read.how
    ))
}
