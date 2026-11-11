//! The indented-text outline that `wombat new --from` reads.
//!
//! Each line is one row of a grid; cells in a row are separated by ` | `. Lines indented
//! further than a row become the grid of that row's first cell. Blank lines are skipped.
//! If the outline is a single line with a single cell at the top, with everything else
//! indented under it, that line is the root cell; otherwise the root cell is empty and
//! holds the top-level rows.

use wombat_core::construct::Construct;
use wombat_core::limits::MAX_DEPTH;
use wombat_core::{CellId, Document, Error, IdSource};

/// One row of a grid, and the rows of the grid inside its first cell.
#[derive(Debug, PartialEq)]
pub struct Row {
    /// The texts of the cells of the row.
    pub cells: Vec<String>,
    /// The rows of the grid inside the first cell.
    pub children: Vec<Row>,
}

struct Line {
    number: usize,
    indent: usize,
    cells: Vec<String>,
}

/// Reads an outline into rows.
pub fn parse(text: &str) -> Result<Vec<Row>, String> {
    let lines: Vec<Line> = text
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(n, line)| Line {
            number: n.saturating_add(1),
            indent: line.chars().take_while(|c| c.is_whitespace()).count(),
            cells: line
                .trim()
                .split(" | ")
                .map(|cell| cell.trim().to_string())
                .collect(),
        })
        .collect();
    let mut pos = 0;
    let indent = lines.first().map_or(0, |line| line.indent);
    let rows = block(&lines, &mut pos, indent, 1)?;
    match lines.get(pos) {
        None => Ok(rows),
        Some(line) => Err(format!(
            "line {}: indented less than the first line",
            line.number
        )),
    }
}

/// Reads the rows at exactly `indent`, starting at `pos`, with their children.
fn block(lines: &[Line], pos: &mut usize, indent: usize, depth: u32) -> Result<Vec<Row>, String> {
    let mut rows = Vec::new();
    while let Some(line) = lines.get(*pos) {
        if line.indent < indent {
            break;
        }
        if line.indent > indent {
            return Err(format!(
                "line {}: its indentation matches no line above it",
                line.number
            ));
        }
        *pos = pos.saturating_add(1);
        let mut row = Row {
            cells: line.cells.clone(),
            children: Vec::new(),
        };
        if let Some(next) = lines.get(*pos).filter(|next| next.indent > indent) {
            if depth >= MAX_DEPTH {
                return Err(format!(
                    "line {}: nested deeper than {MAX_DEPTH}",
                    next.number
                ));
            }
            row.children = block(lines, pos, next.indent, depth.saturating_add(1))?;
        }
        rows.push(row);
    }
    Ok(rows)
}

/// Makes a document from an outline.
pub fn to_document(rows: &[Row], ids: &mut dyn IdSource) -> Result<Document, Error> {
    let mut doc = Document::new(ids);
    let root = doc.root();
    match rows {
        [only] if only.cells.len() == 1 => {
            set_text(&mut doc, root, only.cells.first());
            fill(&mut doc, root, &only.children, ids)?;
        }
        _ => fill(&mut doc, root, rows, ids)?,
    }
    Ok(doc)
}

fn set_text(doc: &mut Document, cell: CellId, text: Option<&String>) {
    if let (Some(cell), Some(text)) = (doc.cell_mut(cell), text) {
        cell.text.clone_from(text);
        // As when typed into a cell: a text that starts with "=" is a formula.
        if wombat_core::eval::is_formula_text(text) {
            cell.kind = wombat_core::CellKind::Formula;
        }
    }
}

/// Gives `cell` a grid with one row per outline row, as wide as the widest row.
fn fill(
    doc: &mut Document,
    cell: CellId,
    rows: &[Row],
    ids: &mut dyn IdSource,
) -> Result<(), Error> {
    let xs = rows.iter().map(|row| row.cells.len()).max().unwrap_or(0);
    let (Ok(xs), Ok(ys)) = (u32::try_from(xs), u32::try_from(rows.len())) else {
        return Err(Error::Limit("the outline is too large".into()));
    };
    if ys == 0 {
        return Ok(());
    }
    doc.add_grid(cell, xs, ys, ids)?;
    for (y, row) in (0..ys).zip(rows) {
        for (x, text) in (0..xs).zip(&row.cells) {
            if let Some(child) = doc.child(cell, x, y) {
                set_text(doc, child, Some(text));
            }
        }
        if let Some(first) = doc.child(cell, 0, y) {
            fill(doc, first, &row.children, ids)?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use wombat_core::SeededIds;

    fn row(cells: &[&str], children: Vec<Row>) -> Row {
        Row {
            cells: cells.iter().map(|c| c.to_string()).collect(),
            children,
        }
    }

    #[test]
    fn rows_cells_and_nesting() {
        let rows =
            parse("Shopping\n  Fruit | Price\n  Apple | 1.20\n\n      red\n      green\n  Pear\n")
                .unwrap();
        assert_eq!(
            rows,
            vec![row(
                &["Shopping"],
                vec![
                    row(&["Fruit", "Price"], vec![]),
                    row(
                        &["Apple", "1.20"],
                        vec![row(&["red"], vec![]), row(&["green"], vec![])]
                    ),
                    row(&["Pear"], vec![]),
                ]
            )]
        );
        let doc = to_document(&rows, &mut SeededIds::from_u64(1)).unwrap();
        doc.validate().unwrap();
        assert_eq!(doc.cell_count(), 1 + 6 + 2);
        let root = doc.cell(doc.root()).unwrap();
        assert_eq!(root.text, "Shopping");
        assert_eq!(
            (root.grid().unwrap().xs(), root.grid().unwrap().ys()),
            (2, 3)
        );
        let apple = doc.child(doc.root(), 0, 1).unwrap();
        assert_eq!(doc.cell(apple).unwrap().text, "Apple");
        assert_eq!(
            doc.cell(doc.child(apple, 0, 1).unwrap()).unwrap().text,
            "green"
        );
        assert_eq!(
            doc.cell(doc.child(doc.root(), 1, 2).unwrap()).unwrap().text,
            ""
        );
    }

    #[test]
    fn several_top_level_rows_go_under_an_empty_root() {
        let doc = to_document(&parse("a\nb | c\n").unwrap(), &mut SeededIds::from_u64(1)).unwrap();
        assert_eq!(doc.cell(doc.root()).unwrap().text, "");
        assert_eq!(doc.cell_count(), 5);
        let empty = to_document(&parse("\n  \n").unwrap(), &mut SeededIds::from_u64(1)).unwrap();
        assert_eq!(empty.cell_count(), 1);
    }

    #[test]
    fn bad_indentation_is_an_error() {
        assert!(parse("a\n    b\n  c\n").unwrap_err().contains("line 3"));
        assert!(parse("  a\nb\n").unwrap_err().contains("line 2"));
        let mut deep = String::new();
        for n in 0..40 {
            deep.push_str(&" ".repeat(n));
            deep.push_str("x\n");
        }
        assert!(parse(&deep).unwrap_err().contains("nested deeper"));
    }
}
