//! The lookup-style export (decision B7): one table per grid, with ID columns that let a
//! spreadsheet join every table to the one it is nested in with `XLOOKUP`.
//!
//! [`export`] gives the files of a folder as values; writing them is
//! [`crate::export_files::write_folder`]. The layout:
//!
//! - **One CSV file per grid**: the grid of the exported cell and every grid nested in
//!   it. Each row of a grid is a row of its table.
//! - **Columns**, in this order. First the content, so that it is what a reader sees: one
//!   column per column of the grid, named `A`, `B`, `C`, ... like the columns of the
//!   spreadsheet they land in (or after the grid's first row, see [`LookupOptions`]);
//!   with the notes option each is followed by `<name>_note`. Then `row_id`, the key of
//!   the row. Then `<name>_id` per column: the permanent ID of each cell. Then
//!   `<name>_table` for each column that has nested grids in this table: the file of the
//!   table inside that cell, or empty. Then, in every table but the top one,
//!   `parent_table` (the file of the table it is nested in), `parent_row_id` (the `row_id`
//!   there of the row it is nested in) and `parent_cell_id` (the ID of the cell it is
//!   nested in, which is in one of the `<name>_id` columns of that row).
//! - **`row_id`** is the ID of the row's first cell. Cells have permanent IDs and rows
//!   have none, so the first cell stands for its row: the key stays the same from one
//!   export to the next for as long as that cell exists and stays first in its row.
//!   Sorting rows keeps it; swapping or moving columns, or transposing, changes which
//!   cell is first.
//! - **`index.csv`**: one row per table, in document order: `table` (file), `trail` (the
//!   texts on the way to the cell it sits in), `rows`, `columns`, `header_row`,
//!   `parent_table`, `parent_row_id`, `parent_column`, `cell_id` and `cell_text` (the
//!   cell it sits in).
//! - **`README.txt`**: the layout in a few lines, and lookup formulas written for two of
//!   the exported tables: `XLOOKUP` as Excel and Google Sheets write it and as LibreOffice
//!   does, and `INDEX` with `MATCH`, which every version of them has.
//!
//! Nothing is lost: the tree of texts and IDs can be rebuilt from the folder alone (a
//! test does). File names are made by [`super::names`].

use std::collections::{HashMap, HashSet};

use super::delimited::Writer;
use super::export::{cell_fields, grid_of};
use super::{names, too_large, ExportOptions, Source, MAX_EXPORT_BYTES};
use crate::error::Error;
use crate::id::CellId;
use crate::model::Cell;

/// The most tables one export may have. Each is a file; a document with more grids than
/// this is exported in parts, by cell.
pub const MAX_TABLES: usize = 10_000;

/// The file that lists the tables.
pub const INDEX_FILE: &str = "index.csv";
/// The file that explains the folder.
pub const README_FILE: &str = "README.txt";

/// Choices of the lookup export beyond [`ExportOptions`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct LookupOptions {
    /// Whether the first row of a grid names its columns. Grids have no header row of
    /// their own in Wombat, so by default (`false`) columns are named `A`, `B`, `C`, ...
    /// and every row is data. With `true`, a table takes its column names from the texts
    /// of the grid's first row and leaves that row out of the data; a grid with one row,
    /// or with a nested grid in its first row, keeps the letters and all its rows.
    /// `index.csv` says per table which it was.
    pub header_row: bool,
}

/// One file of an export.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ExportFile {
    /// The file name, without any folder: safe on every system (see [`super::names`]).
    pub name: String,
    /// The content.
    pub text: String,
}

/// The spreadsheet's name for column number `n` (from 0): `A` to `Z`, then `AA`, ...
pub fn column_letters(n: usize) -> String {
    let mut out = Vec::new();
    let mut n = n;
    loop {
        out.push(char::from(b'A'.saturating_add((n % 26) as u8)));
        n = match (n / 26).checked_sub(1) {
            Some(next) => next,
            None => break,
        };
    }
    out.iter().rev().collect()
}

/// A table before it is written.
struct Table {
    owner: CellId,
    /// The index of the table it is nested in, the `row_id` of the row and the column.
    parent: Option<(usize, CellId, usize)>,
    labels: Vec<String>,
    stem: String,
    /// Whether the first row gives the names.
    header: bool,
}

/// Where the columns of a written table are, counted from 0.
struct Layout {
    names: Vec<String>,
    row_id: usize,
    parent_row_id: usize,
    /// The columns of the grid that have a `_table` column.
    nested: Vec<usize>,
}

fn has_grid(source: Source, id: CellId) -> bool {
    source.doc.cell(id).is_some_and(|c| c.grid().is_some())
}

/// What a cell is called in a trail or a file name: what it shows, or for a cell that
/// shows nothing the first text of its row (the row's label).
fn label(source: Source, id: CellId, row: &[CellId]) -> String {
    std::iter::once(&id)
        .chain(row)
        .map(|c| source.text(*c, false))
        .find(|text| !text.is_empty())
        .unwrap_or("")
        .to_string()
}

/// The tables of the export in document order.
fn collect(source: Source, top: CellId) -> Result<Vec<Table>, Error> {
    let mut tables: Vec<Table> = Vec::new();
    let top_label = source.text(top, false).to_string();
    let mut stack = vec![(top, None, vec![top_label])];
    while let Some((owner, parent, labels)) = stack.pop() {
        let Some(grid) = source.doc.cell(owner).and_then(Cell::grid) else {
            continue;
        };
        if tables.len() >= MAX_TABLES {
            return Err(Error::Limit(format!(
                "more than {MAX_TABLES} tables; export a part of the document by its cell"
            )));
        }
        let index = tables.len();
        let mut found = Vec::new();
        for row in grid.cells().chunks(grid.xs().max(1) as usize) {
            let Some(first) = row.first() else { continue };
            for (x, id) in row.iter().enumerate() {
                if has_grid(source, *id) {
                    let mut inner = labels.clone();
                    inner.push(label(source, *id, row));
                    found.push((*id, Some((index, *first, x)), inner));
                }
            }
        }
        stack.extend(found.into_iter().rev());
        tables.push(Table {
            owner,
            parent,
            labels,
            stem: String::new(),
            header: false,
        });
    }
    Ok(tables)
}

/// `name`, or with a number after it if a column of that name (in any case) is there.
fn unique(name: String, used: &mut HashSet<String>) -> String {
    let mut candidate = name.clone();
    let mut n = 1usize;
    while !used.insert(candidate.to_lowercase()) {
        n = n.saturating_add(1);
        candidate = format!("{name}_{n}");
    }
    candidate
}

/// The column names of a table and where its key columns are.
fn layout(source: Source, table: &Table, options: &ExportOptions) -> Result<Layout, Error> {
    let grid = grid_of(source.doc, table.owner)?;
    let xs = grid.xs() as usize;
    let mut used = HashSet::new();
    let mut names = Vec::new();
    let mut content = Vec::with_capacity(xs);
    for x in 0..xs {
        let own = match grid.at(x as u32, 0) {
            Some(id) if table.header => source.text(id, options.formulas),
            _ => "",
        };
        let name = if own.is_empty() {
            column_letters(x)
        } else {
            own.to_string()
        };
        let name = unique(name, &mut used);
        names.push(name.clone());
        if options.notes {
            names.push(unique(format!("{name}_note"), &mut used));
        }
        content.push(name);
    }
    let row_id = names.len();
    names.push(unique("row_id".into(), &mut used));
    for name in &content {
        names.push(unique(format!("{name}_id"), &mut used));
    }
    let nested: Vec<usize> = (0..xs)
        .filter(|x| {
            grid.cells()
                .iter()
                .skip(*x)
                .step_by(xs.max(1))
                .any(|id| has_grid(source, *id))
        })
        .collect();
    for x in &nested {
        let name = content.get(*x).map_or("", String::as_str);
        names.push(unique(format!("{name}_table"), &mut used));
    }
    let parent_row_id = names.len().saturating_add(1);
    if table.parent.is_some() {
        for name in ["parent_table", "parent_row_id", "parent_cell_id"] {
            names.push(unique(name.into(), &mut used));
        }
    }
    Ok(Layout {
        names,
        row_id,
        parent_row_id,
        nested,
    })
}

fn file_name(stem: &str) -> String {
    format!("{stem}.csv")
}

/// Writes one table.
fn write_table(
    source: Source,
    (tables, by_owner): (&[Table], &HashMap<CellId, usize>),
    table: &Table,
    layout: &Layout,
    options: &ExportOptions,
    room: usize,
) -> Result<String, Error> {
    let grid = grid_of(source.doc, table.owner)?;
    let xs = grid.xs().max(1) as usize;
    let mut out = Writer::with_limit(options, room);
    for name in &layout.names {
        out.field(name)?;
    }
    out.end_row();
    // The file of the table inside a cell.
    let inside = |id: CellId| {
        let table = tables.get(*by_owner.get(&id)?)?;
        Some(file_name(&table.stem))
    };
    let parent = table.parent.and_then(|(index, row, _)| {
        let parent = tables.get(index)?;
        Some((file_name(&parent.stem), row.to_string()))
    });
    for row in grid
        .cells()
        .chunks(xs)
        .skip(if table.header { 1 } else { 0 })
    {
        for id in row {
            cell_fields(&mut out, source, *id, options)?;
        }
        out.raw(&row.first().map(CellId::to_string).unwrap_or_default())?;
        for id in row {
            out.raw(&id.to_string())?;
        }
        for x in &layout.nested {
            let file = row.get(*x).and_then(|id| inside(*id)).unwrap_or_default();
            out.raw(&file)?;
        }
        if let Some((file, row_id)) = &parent {
            out.raw(file)?;
            out.raw(row_id)?;
            out.raw(&table.owner.to_string())?;
        }
        out.end_row();
    }
    Ok(out.finish())
}

/// The explanation, with formulas for the first nested table if there is one.
fn readme(
    tables: &[Table],
    layouts: &[Layout],
    options: &ExportOptions,
    lookup: LookupOptions,
) -> String {
    let mut out = String::from(
        "Wombat lookup export\n\
         ====================\n\
         \n\
         This folder holds one table (a CSV file) for every grid of the exported document:\n\
         the top grid and every grid nested inside a cell. index.csv lists the tables.\n\
         \n\
         Columns of every table, from left to right:\n\
         \n",
    );
    out.push_str(if lookup.header_row {
        "  content         one column per column of the grid, named after the grid's first\n\
         \x20                 row where it has one to spare (index.csv, column header_row),\n\
         \x20                 otherwise A, B, C, ...\n"
    } else {
        "  A, B, C, ...    the cells of the grid, one column per column of the grid\n"
    });
    if options.notes {
        out.push_str("  <column>_note   after each of them: the notes of its cells\n");
    }
    out.push_str(
        "  row_id          the key of the row: the permanent ID of the row's first cell\n\
         \x20 <column>_id     the permanent ID of each cell\n\
         \x20 <column>_table  for a column with nested grids: the file of the table inside the cell\n\
         \x20 parent_table    the file of the table this one is nested in (not in the top table)\n\
         \x20 parent_row_id   the row_id, in that table, of the row this table is nested in\n\
         \x20 parent_cell_id  the ID of the cell this table is nested in\n\
         \n\
         Every row of a nested table carries the row_id of the row it belongs to, so the two\n\
         tables can be joined with a lookup formula. The key does not have to be the first\n\
         column, so the ID columns are last.\n\
         \n",
    );
    let nested = tables
        .iter()
        .zip(layouts)
        .enumerate()
        .find_map(|(n, (t, l))| {
            let (index, _, _) = t.parent?;
            Some((n, t, l, tables.get(index)?, layouts.get(index)?))
        });
    match nested {
        None => out.push_str("This export has one table: the document has no nested grids.\n"),
        Some((_, child, child_layout, parent, parent_layout)) => {
            let key = column_letters(child_layout.parent_row_id);
            let free = column_letters(child_layout.names.len());
            let row_id = column_letters(parent_layout.row_id);
            out.push_str(&format!(
                "Worked example\n\
                 --------------\n\
                 \n\
                 {child_file} is nested in {parent_file}.\n\
                 To show, beside every row of {child_file}, column A of the row it belongs to:\n\
                 \n\
                 1. Get both files into one workbook, each as a sheet named after its file\n\
                 \x20  without \".csv\": {child} and {parent}.\n\
                 \x20  Excel: open both files (each opens as a workbook with one sheet of that\n\
                 \x20  name), then right-click the sheet tab > Move or Copy, into the other.\n\
                 \x20  Google Sheets: File > Import > Upload > Insert new sheet(s), once per\n\
                 \x20  file; rename the sheet if it did not get the name above.\n\
                 \x20  LibreOffice: open one file (in the dialog that comes, keep \"Separated by\"\n\
                 \x20  comma; the sheet is named after the file), then Sheet > Insert Sheet from\n\
                 \x20  File for the other.\n\
                 2. In sheet {child}, the column parent_row_id is column {key} and the first free\n\
                 \x20  column is {free}. In cell {free}2 enter one of these and fill it down.\n\
                 \n\
                 \x20  Excel 2021 or 365, and Google Sheets:\n\
                 \n\
                 \x20      =XLOOKUP({key}2, '{parent}'!{row_id}:{row_id}, '{parent}'!A:A)\n\
                 \n\
                 \x20  LibreOffice 24.8 or later (it writes a sheet as $'name'. and not 'name'!):\n\
                 \n\
                 \x20      =XLOOKUP({key}2, $'{parent}'.{row_id}:{row_id}, $'{parent}'.A:A)\n\
                 \n\
                 \x20  Every version of all three, without XLOOKUP:\n\
                 \n\
                 \x20      =INDEX('{parent}'!A:A, MATCH({key}2, '{parent}'!{row_id}:{row_id}, 0))\n\
                 \x20      =INDEX($'{parent}'.A:A, MATCH({key}2, $'{parent}'.{row_id}:{row_id}, 0))    (LibreOffice)\n\
                 \n\
                 \x20  {row_id} is the column row_id of sheet {parent}; replace A:A by another column\n\
                 \x20  of that sheet to fetch something else.\n\
                 \x20  Where the comma is the decimal sign (much of Europe), formulas take ; in place\n\
                 \x20  of each comma between arguments: =XLOOKUP({key}2; ...; ...).\n\
                 3. The other way round, the number of rows of {child} under each row of\n\
                 \x20  {parent}: in sheet {parent}, in a free column of row 2,\n\
                 \n\
                 \x20      =COUNTIF('{child}'!{key}:{key}, {row_id}2)\n\
                 \x20      =COUNTIF($'{child}'.{key}:{key}, {row_id}2)    (LibreOffice)\n\
                 \n\
                 The same works between any table and the one named in its parent_table column;\n\
                 the letters of the key columns differ with the number of columns of each table.\n",
                child_file = file_name(&child.stem),
                parent_file = file_name(&parent.stem),
                child = child.stem,
                parent = parent.stem,
            ));
        }
    }
    out.push_str(
        "\nHow long the IDs stay the same\n\
         ------------------------------\n\
         \n\
         A cell's ID never changes. row_id is the ID of the row's first cell: it is the same\n\
         in every export as long as that cell exists and stays first in its row. Sorting\n\
         rows keeps it. Deleting that cell, swapping or moving columns, or transposing the\n\
         grid changes which cell is first, and so the row_id of the row.\n\
         \n\
         What a spreadsheet does to the values when it opens a CSV file\n\
         --------------------------------------------------------------\n\
         \n\
         A CSV file is text; the spreadsheet decides what each value is. 00123 becomes the\n\
         number 123, a number of more than 15 digits is rounded (12345678901234567890\n\
         becomes 1.23E+19), 2026-03-04 becomes a date, and Excel is known to do the same to\n\
         1/2 and to 1e5. The IDs stay text. Where the exact text matters, import the columns\n\
         as text: LibreOffice, in the dialog on opening, select the columns and choose the\n\
         column type Text; Excel, Data > From Text/CSV > Transform Data, or the Text Import\n\
         Wizard with the column format Text; Google Sheets, File > Import with \"Convert\n\
         text to numbers, dates and formulas\" switched off.\n",
    );
    if options.delimiter == super::Delimiter::Comma {
        out.push_str(
            "\nIf Excel shows a whole row in one column, its list separator is the semicolon\n\
             (as where the comma is the decimal sign): export again with the delimiter\n\
             semicolon (wombat export --delimiter semicolon), or import the file through\n\
             Data > From Text/CSV.\n",
        );
    }
    if options.guard {
        out.push_str(
            "\nSafety\n\
             ------\n\
             \n\
             A text that a spreadsheet could run as a formula was exported with a single quote '\n\
             in front: one that starts with = , or with + - or @ and has in it what a formula\n\
             needs to do something (a parenthesis, a | , a reference to another sheet), or that\n\
             continues like a calculation or a name (+1+1, -A1). The quote is visible in the\n\
             cell. Ordinary texts are left as they are: -5, +44 20 7946 0958, - a list item,\n\
             @name. Exporting with the raw option leaves every text as it is; importing such a\n\
             file back into Wombat with the option unguard takes the quotes away again.\n",
        );
    }
    out
}

/// The names of the columns of `index.csv`.
const INDEX_COLUMNS: [&str; 10] = [
    "table",
    "trail",
    "rows",
    "columns",
    "header_row",
    "parent_table",
    "parent_row_id",
    "parent_column",
    "cell_id",
    "cell_text",
];

/// The files of the tables that an `index.csv` of this export lists, for removing the
/// tables of an earlier export from a folder that is exported into again. `None` if the
/// text is not such an index (its first line is not the names of the columns, with any
/// of the three delimiters): then the folder is not one this export made.
pub fn index_tables(text: &str) -> Option<Vec<String>> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let limits = super::delimited::ParseLimits {
        cells: MAX_TABLES
            .saturating_add(1)
            .saturating_mul(INDEX_COLUMNS.len()),
        field: crate::limits::MAX_TEXT_CODE_POINTS,
    };
    use super::Delimiter::{Comma, Semicolon, Tab};
    [Comma, Semicolon, Tab].into_iter().find_map(|delimiter| {
        let (rows, _) = super::delimited::parse(text, delimiter, limits).ok()?;
        let (names, tables) = rows.split_first()?;
        if names != &INDEX_COLUMNS {
            return None;
        }
        Some(
            tables
                .iter()
                .filter_map(|row| row.first().cloned())
                .collect(),
        )
    })
}

/// The lookup-style export of the grid of `cell` and everything nested in it: the files
/// of one folder, the tables in document order, then `README.txt`, then `index.csv` last,
/// so that a folder written in this order has its index only when it is complete.
/// See the module documentation for the layout. The same document and options give the
/// same files, byte for byte; the document is not changed.
///
/// Refused for a cell without a grid, for more than [`MAX_TABLES`] grids, and if the
/// files together would be larger than [`MAX_EXPORT_BYTES`].
pub fn export(
    source: Source,
    cell: CellId,
    options: &ExportOptions,
    lookup: LookupOptions,
) -> Result<Vec<ExportFile>, Error> {
    grid_of(source.doc, cell)?;
    let mut tables = collect(source, cell)?;
    let stems = names::stems(
        &tables
            .iter()
            .map(|t| (t.labels.iter().map(String::as_str).collect(), t.owner))
            .collect::<Vec<_>>(),
    );
    for (table, stem) in tables.iter_mut().zip(stems) {
        table.stem = stem;
        let grid = grid_of(source.doc, table.owner)?;
        let first_row = grid.cells().iter().take(grid.xs() as usize);
        table.header = lookup.header_row
            && grid.ys() > 1
            && !first_row.clone().any(|id| has_grid(source, *id));
    }
    let layouts = tables
        .iter()
        .map(|t| layout(source, t, options))
        .collect::<Result<Vec<_>, _>>()?;

    let by_owner: HashMap<CellId, usize> = tables
        .iter()
        .enumerate()
        .map(|(n, t)| (t.owner, n))
        .collect();

    let mut index = Writer::new(options);
    for name in INDEX_COLUMNS {
        index.raw(name)?;
    }
    index.end_row();
    let mut files = Vec::with_capacity(tables.len().saturating_add(2));
    let mut total = 0usize;
    for (table, layout) in tables.iter().zip(&layouts) {
        let grid = grid_of(source.doc, table.owner)?;
        let room = MAX_EXPORT_BYTES.saturating_sub(total);
        let text = write_table(source, (&tables, &by_owner), table, layout, options, room)?;
        total = total.saturating_add(text.len());
        files.push(ExportFile {
            name: file_name(&table.stem),
            text,
        });

        index.raw(&file_name(&table.stem))?;
        let trail: Vec<&str> = table
            .labels
            .iter()
            .map(String::as_str)
            .filter(|l| !l.is_empty())
            .collect();
        index.field(&trail.join(" / "))?;
        let rows = grid.ys().saturating_sub(u32::from(table.header));
        index.raw(&rows.to_string())?;
        index.raw(&grid.xs().to_string())?;
        index.raw(if table.header { "yes" } else { "no" })?;
        let parent = table
            .parent
            .and_then(|(n, row, x)| Some((tables.get(n)?, layouts.get(n)?, row, x)));
        match parent {
            Some((parent, parent_layout, row, x)) => {
                index.raw(&file_name(&parent.stem))?;
                index.raw(&row.to_string())?;
                // The name of the content column: with notes, every second name.
                let step = if options.notes { 2 } else { 1 };
                let name = parent_layout.names.get(x.saturating_mul(step));
                index.field(name.map_or("", String::as_str))?;
            }
            None => {
                for _ in 0..3 {
                    index.raw("")?;
                }
            }
        }
        index.raw(&table.owner.to_string())?;
        index.field(source.text(table.owner, options.formulas))?;
        index.end_row();
        if total.saturating_add(index.len()) > MAX_EXPORT_BYTES {
            return Err(too_large());
        }
    }
    files.push(ExportFile {
        name: README_FILE.to_string(),
        text: readme(&tables, &layouts, options, lookup),
    });
    files.push(ExportFile {
        name: INDEX_FILE.to_string(),
        text: index.finish(),
    });
    Ok(files)
}
