//! Between the serialised form (`api/wire.rs`) and the types of the core. On the way in
//! everything is checked: an ID must be in its canonical text, a whole number in the range
//! the core takes, a scale finite, unknown
//! fields a JSON object, a table a list of whole rows, and a clip or a table goes through
//! the core's own constructors, which check what they are given: a clip as a file, a
//! table against the limits of a document. On the way out nothing can fail but a clip that
//! the core would not write as a document.

use std::path::Path;

use wombat_core as core;
use wombat_core::convert::{Entry, Table};
use wombat_core::model::Extra;
use wombat_core::{CellId, Clip, DocId};

use crate::api::wire::{
    BridgeError, CellInfo, CellKind, CellLook, ClipData, Command, CutText, DrawStyle, EntryData,
    ErrorKind, GridInfo, GridPart, GridSize, GridSummary, HorizAlign, ImageRef, LinkInfo,
    LinkTarget, Prop, Report, Rgb, RunInfo, StageAsk, StageInfo, Style, TableData, TrailStep,
    VertAlign,
};

impl BridgeError {
    /// An error of the bridge itself: what came over it is no request that the core
    /// could be asked.
    pub(crate) fn bad_request(message: impl Into<String>) -> BridgeError {
        BridgeError {
            kind: ErrorKind::BadRequest,
            message: message.into(),
            path: None,
        }
    }
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for BridgeError {}

fn shown(path: &Path) -> Option<String> {
    Some(path.display().to_string())
}

impl From<core::Error> for BridgeError {
    fn from(error: core::Error) -> BridgeError {
        let message = error.to_string();
        let (kind, path) = match &error {
            core::Error::Io { path, .. } => (ErrorKind::Io, shown(path)),
            core::Error::Syntax(_) => (ErrorKind::Syntax, None),
            core::Error::WrongFormat(_) => (ErrorKind::WrongFormat, None),
            core::Error::NewerVersion { .. } => (ErrorKind::NewerVersion, None),
            core::Error::Invalid(_) => (ErrorKind::Invalid, None),
            core::Error::Refused(_) => (ErrorKind::Refused, None),
            core::Error::Limit(_) => (ErrorKind::Limit, None),
            core::Error::NotFound(_) => (ErrorKind::NotFound, None),
            core::Error::AlreadyExists(path) => (ErrorKind::AlreadyExists, shown(path)),
            core::Error::HalfSaved { not_saved, .. } => (ErrorKind::HalfSaved, shown(not_saved)),
        };
        BridgeError {
            kind,
            message,
            path,
        }
    }
}

type In<T> = Result<T, BridgeError>;

/// Reads the ID of a cell.
pub(crate) fn cell_id(text: &str) -> In<CellId> {
    CellId::parse(text).ok_or_else(|| BridgeError::bad_request(format!("{text:?} is not an ID")))
}

fn doc_id(text: &str) -> In<DocId> {
    DocId::parse(text).ok_or_else(|| BridgeError::bad_request(format!("{text:?} is not an ID")))
}

fn cell_ids(texts: &[String]) -> In<Vec<CellId>> {
    texts.iter().map(|text| cell_id(text)).collect()
}

fn opt_doc(text: &Option<String>) -> In<Option<DocId>> {
    text.as_deref().map(doc_id).transpose()
}

fn texts<T: ToString>(ids: &[T]) -> Vec<String> {
    ids.iter().map(T::to_string).collect()
}

/// A whole number crosses in 64 bits, which is what the other side has, and is checked
/// here against what the core takes. Were it carried in fewer, its high bits would be
/// dropped on the way and row 4294967296 would arrive as row 0.
fn out_of_range(number: i64, least: i64, most: i64) -> BridgeError {
    BridgeError::bad_request(format!("the number {number} is not from {least} to {most}"))
}

/// A position, a count or a size.
fn unsigned(number: i64) -> In<u32> {
    u32::try_from(number).map_err(|_| out_of_range(number, 0, u32::MAX.into()))
}

/// A difference.
fn signed(number: i64) -> In<i32> {
    i32::try_from(number).map_err(|_| out_of_range(number, i32::MIN.into(), i32::MAX.into()))
}

/// One part of a colour.
fn byte(number: i64) -> In<u8> {
    u8::try_from(number).map_err(|_| out_of_range(number, 0, u8::MAX.into()))
}

fn finite(number: f64) -> In<f64> {
    if number.is_finite() {
        Ok(number)
    } else {
        Err(BridgeError::bad_request(
            "a scale that is not a finite number",
        ))
    }
}

fn color_in(c: Rgb) -> In<core::Color> {
    Ok(core::Color::rgb(byte(c.r)?, byte(c.g)?, byte(c.b)?))
}

impl From<core::Color> for Rgb {
    fn from(c: core::Color) -> Rgb {
        Rgb {
            r: c.r.into(),
            g: c.g.into(),
            b: c.b.into(),
        }
    }
}

impl From<Style> for core::Style {
    fn from(s: Style) -> core::Style {
        core::Style {
            bold: s.bold,
            italic: s.italic,
            fixed: s.fixed,
            underline: s.underline,
            strike: s.strike,
        }
    }
}

impl From<core::Style> for Style {
    fn from(s: core::Style) -> Style {
        Style {
            bold: s.bold,
            italic: s.italic,
            fixed: s.fixed,
            underline: s.underline,
            strike: s.strike,
        }
    }
}

/// The two directions between an enum of the core and its mirror with the same variants.
macro_rules! mirror {
    ($wire:ident, $core:ty, [$($variant:ident),+]) => {
        impl From<$wire> for $core {
            fn from(value: $wire) -> $core {
                match value { $($wire::$variant => <$core>::$variant,)+ }
            }
        }

        impl From<$core> for $wire {
            fn from(value: $core) -> $wire {
                match value { $(<$core>::$variant => $wire::$variant,)+ }
            }
        }
    };
}

mirror!(
    CellKind,
    core::CellKind,
    [Data, Code, VarAssign, VarRead, ViewH, ViewV, Formula]
);
mirror!(DrawStyle, core::DrawStyle, [Grid, Bubbles, Lines]);
mirror!(HorizAlign, core::TextAlign, [Auto, Left, Center, Right]);
mirror!(VertAlign, core::VertAlign, [Auto, Top, Middle, Bottom]);

fn extra_in(text: &str) -> In<Extra> {
    if text.is_empty() {
        return Ok(Extra::new());
    }
    serde_json::from_str(text).map_err(|e| {
        BridgeError::bad_request(format!("unknown fields that are no JSON object: {e}"))
    })
}

fn extra_out(extra: &Extra) -> In<String> {
    if extra.is_empty() {
        return Ok(String::new());
    }
    serde_json::to_string(extra).map_err(|e| BridgeError::bad_request(e.to_string()))
}

fn image_in(image: ImageRef) -> In<core::ImageRef> {
    Ok(core::ImageRef {
        file: image.file,
        scale: finite(image.scale)?,
        extra: extra_in(&image.extra)?,
    })
}

fn image_out(image: &core::ImageRef) -> In<ImageRef> {
    Ok(ImageRef {
        file: image.file.clone(),
        scale: finite(image.scale)?,
        extra: extra_out(&image.extra)?,
    })
}

fn target_in(target: LinkTarget) -> In<core::LinkTarget> {
    Ok(match target {
        LinkTarget::External { target } => core::LinkTarget::External(target),
        LinkTarget::Cell { doc, cell } => core::LinkTarget::Cell {
            doc: opt_doc(&doc)?,
            cell: cell_id(&cell)?,
        },
        LinkTarget::Unknown => core::LinkTarget::Unknown,
    })
}

fn target_out(target: &core::LinkTarget) -> LinkTarget {
    match target {
        core::LinkTarget::External(target) => LinkTarget::External {
            target: target.clone(),
        },
        core::LinkTarget::Cell { doc, cell } => LinkTarget::Cell {
            doc: doc.map(|d| d.to_string()),
            cell: cell.to_string(),
        },
        core::LinkTarget::Unknown => LinkTarget::Unknown,
    }
}

fn prop_in(prop: Prop) -> In<core::Prop> {
    Ok(match prop {
        Prop::CellColor(c) => core::Prop::CellColor(color_in(c)?),
        Prop::TextColor(c) => core::Prop::TextColor(color_in(c)?),
        Prop::RelSize(n) => core::Prop::RelSize(signed(n)?),
        Prop::TextAlign(a) => core::Prop::TextAlign(a.into()),
        Prop::VertAlign(a) => core::Prop::VertAlign(a.into()),
        Prop::Kind(k) => core::Prop::Kind(k.into()),
        Prop::Note(text) => core::Prop::Note(text),
        Prop::DrawStyle(d) => core::Prop::DrawStyle(d.into()),
        Prop::Image(image) => core::Prop::Image(image.map(image_in).transpose()?),
        Prop::ImageScale(scale) => core::Prop::ImageScale(finite(scale)?),
        Prop::LastEdit(time) => core::Prop::LastEdit(time),
        Prop::BorderColor(c) => core::Prop::BorderColor(color_in(c)?),
        Prop::OuterSpacing(n) => core::Prop::OuterSpacing(unsigned(n)?),
        Prop::Vertical(on) => core::Prop::Vertical(on),
    })
}

fn prop_out(prop: &core::Prop) -> In<Prop> {
    Ok(match prop {
        core::Prop::CellColor(c) => Prop::CellColor((*c).into()),
        core::Prop::TextColor(c) => Prop::TextColor((*c).into()),
        core::Prop::RelSize(n) => Prop::RelSize((*n).into()),
        core::Prop::TextAlign(a) => Prop::TextAlign((*a).into()),
        core::Prop::VertAlign(a) => Prop::VertAlign((*a).into()),
        core::Prop::Kind(k) => Prop::Kind((*k).into()),
        core::Prop::Note(text) => Prop::Note(text.clone()),
        core::Prop::DrawStyle(d) => Prop::DrawStyle((*d).into()),
        core::Prop::Image(image) => Prop::Image(image.as_ref().map(image_out).transpose()?),
        core::Prop::ImageScale(scale) => Prop::ImageScale(finite(*scale)?),
        core::Prop::LastEdit(time) => Prop::LastEdit(*time),
        core::Prop::BorderColor(c) => Prop::BorderColor((*c).into()),
        core::Prop::OuterSpacing(n) => Prop::OuterSpacing((*n).into()),
        core::Prop::Vertical(on) => Prop::Vertical(*on),
    })
}

/// A clip from what was carried: the core reads the document as it reads a file, and
/// makes the clip of it as it makes one of any document.
fn clip_in(clip: ClipData) -> In<Clip> {
    let doc = core::format::from_slice(clip.document.as_bytes())?;
    let parent = clip.parent.as_deref().map(cell_id).transpose()?;
    Ok(Clip::from_document(&doc, parent)?)
}

/// A clip to carry.
pub fn clip_out(clip: &Clip) -> In<ClipData> {
    Ok(ClipData {
        parent: clip.parent().map(|p| p.to_string()),
        document: core::format::to_string(&clip.as_document()?)?,
    })
}

/// Refuses a grid of a table that has no size: it needs a column, a cell, and only whole
/// rows. The core refuses the same when a table is made, but with its word for a file
/// that breaks a rule of the format (`Invalid`); a table is something the caller sends,
/// so the bridge looks first and says that the request is bad. What is left for the core
/// to refuse is a table past a limit of documents.
fn whole_rows(xs: u32, cells: usize) -> In<()> {
    if xs == 0 || cells == 0 || !cells.is_multiple_of(xs as usize) {
        return Err(BridgeError::bad_request(format!(
            "a grid of {cells} cells in {xs} columns: a table has at least one cell and \
             only whole rows"
        )));
    }
    Ok(())
}

/// A grid of a table that is being put together from the flat list.
struct Open {
    xs: u32,
    cells: Vec<Entry>,
    /// The cell it is the grid of, and how many cells are still to come; `None` for the
    /// outermost grid, which takes whatever is in no other.
    inside: Option<(Entry, u32)>,
}

impl Open {
    fn add(&mut self, cell: Entry) {
        self.cells.push(cell);
        if let Some((_, left)) = &mut self.inside {
            *left = left.saturating_sub(1);
        }
    }
}

/// The table that a flat list stands for. It is put together with a list of the grids
/// that are open, not by a function that calls itself: the list comes from outside and
/// may say that it is nested deeper than the stack is. The core refuses a table that is
/// too deep the moment one is made of it, so no more than that is ever built.
fn table_in(table: TableData) -> In<Table> {
    let mut outer: Vec<Open> = Vec::new();
    let mut now = Open {
        xs: unsigned(table.xs)?,
        cells: Vec::new(),
        inside: None,
    };
    for entry in table.cells {
        let mut cell = Entry::new(&entry.text);
        if entry.formula {
            cell = cell.as_formula();
        }
        match entry.grid {
            Some(grid) => {
                let (xs, cells) = (unsigned(grid.xs)?, unsigned(grid.cells)?);
                // Before any of its cells is read: it will be exactly this size or short.
                whole_rows(xs, cells as usize)?;
                let inner = Open {
                    xs,
                    cells: Vec::new(),
                    inside: Some((cell, cells)),
                };
                outer.push(std::mem::replace(&mut now, inner));
            }
            None => now.add(cell),
        }
        // A grid that has all its cells goes into its cell, which may complete the next.
        while matches!(now.inside, Some((_, 0))) {
            let Some(around) = outer.pop() else { break };
            let done = std::mem::replace(&mut now, around);
            if let Some((cell, _)) = done.inside {
                now.add(cell.with_grid(Table::new(done.xs, done.cells)?));
            }
        }
    }
    if let Some((_, left)) = now.inside {
        return Err(BridgeError::bad_request(format!(
            "a grid inside the table is {left} cells short"
        )));
    }
    whole_rows(now.xs, now.cells.len())?;
    Ok(Table::new(now.xs, now.cells)?)
}

/// The flat list for a table. This one may call itself: a table of the core is never
/// nested deeper than a document may be.
fn table_out(table: &Table) -> TableData {
    fn list(table: &Table, cells: &mut Vec<EntryData>) {
        for entry in table.cells() {
            let size = |grid: &Table| GridSize {
                xs: grid.xs().into(),
                cells: i64::try_from(grid.cells().len()).unwrap_or(i64::MAX),
            };
            cells.push(EntryData {
                text: entry.text().to_string(),
                formula: entry.is_formula(),
                grid: entry.grid().map(size),
            });
            if let Some(grid) = entry.grid() {
                list(grid, cells);
            }
        }
    }
    let mut cells = Vec::new();
    list(table, &mut cells);
    TableData {
        xs: table.xs().into(),
        cells,
    }
}

/// The command of the core that a carried command stands for.
pub fn command_in(command: Command) -> In<core::Command> {
    use core::Command as C;
    Ok(match command {
        Command::InsertText { cell, at, text } => C::InsertText {
            cell: cell_id(&cell)?,
            at: unsigned(at)?,
            text,
        },
        Command::DeleteText { cell, start, len } => C::DeleteText {
            cell: cell_id(&cell)?,
            start: unsigned(start)?,
            len: unsigned(len)?,
        },
        Command::ReplaceText {
            cell,
            start,
            len,
            text,
        } => C::ReplaceText {
            cell: cell_id(&cell)?,
            start: unsigned(start)?,
            len: unsigned(len)?,
            text,
        },
        Command::SetText { cell, text } => C::SetText {
            cell: cell_id(&cell)?,
            text,
        },
        Command::SetRangeStyle {
            cell,
            start,
            len,
            set,
            clear,
        } => C::SetRangeStyle {
            cell: cell_id(&cell)?,
            start: unsigned(start)?,
            len: unsigned(len)?,
            set: set.into(),
            clear: clear.into(),
        },
        Command::SetRangeColor {
            cell,
            start,
            len,
            color,
        } => C::SetRangeColor {
            cell: cell_id(&cell)?,
            start: unsigned(start)?,
            len: unsigned(len)?,
            color: color.map(color_in).transpose()?,
        },
        Command::SetLink {
            cell,
            start,
            len,
            target,
        } => C::SetLink {
            cell: cell_id(&cell)?,
            start: unsigned(start)?,
            len: unsigned(len)?,
            target: target.map(target_in).transpose()?,
        },
        Command::SetProps { cells, props } => C::SetProps {
            cells: cell_ids(&cells)?,
            props: props.into_iter().map(prop_in).collect::<In<_>>()?,
        },
        Command::SetStyle { cells, set, clear } => C::SetStyle {
            cells: cell_ids(&cells)?,
            set: set.into(),
            clear: clear.into(),
        },
        Command::SetColumnWidth {
            cell,
            column,
            width,
        } => C::SetColumnWidth {
            cell: cell_id(&cell)?,
            column: unsigned(column)?,
            width: unsigned(width)?,
        },
        Command::CreateGrid { cell, xs, ys } => C::CreateGrid {
            cell: cell_id(&cell)?,
            xs: unsigned(xs)?,
            ys: unsigned(ys)?,
        },
        Command::RemoveGrid { cell } => C::RemoveGrid {
            cell: cell_id(&cell)?,
        },
        Command::InsertRows { cell, at, count } => C::InsertRows {
            cell: cell_id(&cell)?,
            at: unsigned(at)?,
            count: unsigned(count)?,
        },
        Command::InsertColumns { cell, at, count } => C::InsertColumns {
            cell: cell_id(&cell)?,
            at: unsigned(at)?,
            count: unsigned(count)?,
        },
        Command::DeleteRows { cell, at, count } => C::DeleteRows {
            cell: cell_id(&cell)?,
            at: unsigned(at)?,
            count: unsigned(count)?,
        },
        Command::DeleteColumns { cell, at, count } => C::DeleteColumns {
            cell: cell_id(&cell)?,
            at: unsigned(at)?,
            count: unsigned(count)?,
        },
        Command::ClearCells { cells } => C::ClearCells {
            cells: cell_ids(&cells)?,
        },
        Command::DeleteCells { cell, x, y, xs, ys } => C::DeleteCells {
            cell: cell_id(&cell)?,
            x: unsigned(x)?,
            y: unsigned(y)?,
            xs: unsigned(xs)?,
            ys: unsigned(ys)?,
        },
        Command::MoveRows {
            cell,
            from,
            count,
            to,
        } => C::MoveRows {
            cell: cell_id(&cell)?,
            from: unsigned(from)?,
            count: unsigned(count)?,
            to: unsigned(to)?,
        },
        Command::MoveColumns {
            cell,
            from,
            count,
            to,
        } => C::MoveColumns {
            cell: cell_id(&cell)?,
            from: unsigned(from)?,
            count: unsigned(count)?,
            to: unsigned(to)?,
        },
        Command::SwapCells { a, b } => C::SwapCells {
            a: cell_id(&a)?,
            b: cell_id(&b)?,
        },
        Command::MoveCell { from, to } => C::MoveCell {
            from: cell_id(&from)?,
            to: cell_id(&to)?,
        },
        Command::CopyCell { from, to } => C::CopyCell {
            from: cell_id(&from)?,
            to: cell_id(&to)?,
        },
        Command::SetTag {
            name,
            cell_color,
            text_color,
        } => C::SetTag {
            name,
            cell_color: color_in(cell_color)?,
            text_color: color_in(text_color)?,
        },
        Command::RemoveTag { name } => C::RemoveTag { name },
        Command::SortRows {
            cell,
            column,
            row,
            count,
            descending,
        } => C::SortRows {
            cell: cell_id(&cell)?,
            column: unsigned(column)?,
            row: unsigned(row)?,
            count: unsigned(count)?,
            descending,
        },
        Command::Transpose { cells } => C::Transpose {
            cells: cell_ids(&cells)?,
        },
        Command::HierarchySwap { cell } => C::HierarchySwap {
            cell: cell_id(&cell)?,
        },
        Command::Hierarchify { cell } => C::Hierarchify {
            cell: cell_id(&cell)?,
        },
        Command::Flatten { cell } => C::Flatten {
            cell: cell_id(&cell)?,
        },
        Command::Wrap { cell, x, y, xs, ys } => C::Wrap {
            cell: cell_id(&cell)?,
            x: unsigned(x)?,
            y: unsigned(y)?,
            xs: unsigned(xs)?,
            ys: unsigned(ys)?,
        },
        Command::Collapse { cell, x, y, xs, ys } => C::Collapse {
            cell: cell_id(&cell)?,
            x: unsigned(x)?,
            y: unsigned(y)?,
            xs: unsigned(xs)?,
            ys: unsigned(ys)?,
        },
        Command::SplitCell { cell, at } => C::SplitCell {
            cell: cell_id(&cell)?,
            at: unsigned(at)?,
        },
        Command::TagCells { cells } => C::TagCells {
            cells: cell_ids(&cells)?,
        },
        Command::UntagCells { cells } => C::UntagCells {
            cells: cell_ids(&cells)?,
        },
        Command::AdjustRelSize {
            cells,
            delta,
            recursive,
        } => C::AdjustRelSize {
            cells: cell_ids(&cells)?,
            delta: signed(delta)?,
            recursive,
        },
        Command::AdjustColumnWidths {
            cell,
            column,
            count,
            delta,
            recursive,
        } => C::AdjustColumnWidths {
            cell: cell_id(&cell)?,
            column: unsigned(column)?,
            count: unsigned(count)?,
            delta: signed(delta)?,
            recursive,
        },
        Command::Run => C::Run,
        Command::EnterText { cell, text } => C::EnterText {
            cell: cell_id(&cell)?,
            text,
        },
        Command::ToggleCheck { cells } => C::ToggleCheck {
            cells: cell_ids(&cells)?,
        },
        Command::PasteCells { to, clip } => C::PasteCells {
            to: cell_id(&to)?,
            clip: clip_in(clip)?,
        },
        Command::AdoptCells { to, clip } => C::AdoptCells {
            to: cell_id(&to)?,
            clip: clip_in(clip)?,
        },
        Command::ReleaseCell { cell } => C::ReleaseCell {
            cell: cell_id(&cell)?,
        },
        Command::Import {
            cell,
            table,
            replace,
        } => C::Import {
            cell: cell_id(&cell)?,
            table: table_in(table)?,
            replace,
        },
        Command::RetargetLinks { cells, from, to } => C::RetargetLinks {
            cells: cell_ids(&cells)?,
            from: opt_doc(&from)?,
            to: opt_doc(&to)?,
        },
    })
}

/// A command of the core, to carry.
pub fn command_out(command: &core::Command) -> In<Command> {
    use core::Command as C;
    let id = |cell: &CellId| cell.to_string();
    Ok(match command {
        C::InsertText { cell, at, text } => Command::InsertText {
            cell: id(cell),
            at: i64::from(*at),
            text: text.clone(),
        },
        C::DeleteText { cell, start, len } => Command::DeleteText {
            cell: id(cell),
            start: i64::from(*start),
            len: i64::from(*len),
        },
        C::ReplaceText {
            cell,
            start,
            len,
            text,
        } => Command::ReplaceText {
            cell: id(cell),
            start: i64::from(*start),
            len: i64::from(*len),
            text: text.clone(),
        },
        C::SetText { cell, text } => Command::SetText {
            cell: id(cell),
            text: text.clone(),
        },
        C::SetRangeStyle {
            cell,
            start,
            len,
            set,
            clear,
        } => Command::SetRangeStyle {
            cell: id(cell),
            start: i64::from(*start),
            len: i64::from(*len),
            set: (*set).into(),
            clear: (*clear).into(),
        },
        C::SetRangeColor {
            cell,
            start,
            len,
            color,
        } => Command::SetRangeColor {
            cell: id(cell),
            start: i64::from(*start),
            len: i64::from(*len),
            color: color.map(Into::into),
        },
        C::SetLink {
            cell,
            start,
            len,
            target,
        } => Command::SetLink {
            cell: id(cell),
            start: i64::from(*start),
            len: i64::from(*len),
            target: target.as_ref().map(target_out),
        },
        C::SetProps { cells, props } => Command::SetProps {
            cells: texts(cells),
            props: props.iter().map(prop_out).collect::<In<_>>()?,
        },
        C::SetStyle { cells, set, clear } => Command::SetStyle {
            cells: texts(cells),
            set: (*set).into(),
            clear: (*clear).into(),
        },
        C::SetColumnWidth {
            cell,
            column,
            width,
        } => Command::SetColumnWidth {
            cell: id(cell),
            column: i64::from(*column),
            width: i64::from(*width),
        },
        C::CreateGrid { cell, xs, ys } => Command::CreateGrid {
            cell: id(cell),
            xs: i64::from(*xs),
            ys: i64::from(*ys),
        },
        C::RemoveGrid { cell } => Command::RemoveGrid { cell: id(cell) },
        C::InsertRows { cell, at, count } => Command::InsertRows {
            cell: id(cell),
            at: i64::from(*at),
            count: i64::from(*count),
        },
        C::InsertColumns { cell, at, count } => Command::InsertColumns {
            cell: id(cell),
            at: i64::from(*at),
            count: i64::from(*count),
        },
        C::DeleteRows { cell, at, count } => Command::DeleteRows {
            cell: id(cell),
            at: i64::from(*at),
            count: i64::from(*count),
        },
        C::DeleteColumns { cell, at, count } => Command::DeleteColumns {
            cell: id(cell),
            at: i64::from(*at),
            count: i64::from(*count),
        },
        C::ClearCells { cells } => Command::ClearCells {
            cells: texts(cells),
        },
        C::DeleteCells { cell, x, y, xs, ys } => Command::DeleteCells {
            cell: id(cell),
            x: i64::from(*x),
            y: i64::from(*y),
            xs: i64::from(*xs),
            ys: i64::from(*ys),
        },
        C::MoveRows {
            cell,
            from,
            count,
            to,
        } => Command::MoveRows {
            cell: id(cell),
            from: i64::from(*from),
            count: i64::from(*count),
            to: i64::from(*to),
        },
        C::MoveColumns {
            cell,
            from,
            count,
            to,
        } => Command::MoveColumns {
            cell: id(cell),
            from: i64::from(*from),
            count: i64::from(*count),
            to: i64::from(*to),
        },
        C::SwapCells { a, b } => Command::SwapCells { a: id(a), b: id(b) },
        C::MoveCell { from, to } => Command::MoveCell {
            from: id(from),
            to: id(to),
        },
        C::CopyCell { from, to } => Command::CopyCell {
            from: id(from),
            to: id(to),
        },
        C::SetTag {
            name,
            cell_color,
            text_color,
        } => Command::SetTag {
            name: name.clone(),
            cell_color: (*cell_color).into(),
            text_color: (*text_color).into(),
        },
        C::RemoveTag { name } => Command::RemoveTag { name: name.clone() },
        C::SortRows {
            cell,
            column,
            row,
            count,
            descending,
        } => Command::SortRows {
            cell: id(cell),
            column: i64::from(*column),
            row: i64::from(*row),
            count: i64::from(*count),
            descending: *descending,
        },
        C::Transpose { cells } => Command::Transpose {
            cells: texts(cells),
        },
        C::HierarchySwap { cell } => Command::HierarchySwap { cell: id(cell) },
        C::Hierarchify { cell } => Command::Hierarchify { cell: id(cell) },
        C::Flatten { cell } => Command::Flatten { cell: id(cell) },
        C::Wrap { cell, x, y, xs, ys } => Command::Wrap {
            cell: id(cell),
            x: i64::from(*x),
            y: i64::from(*y),
            xs: i64::from(*xs),
            ys: i64::from(*ys),
        },
        C::Collapse { cell, x, y, xs, ys } => Command::Collapse {
            cell: id(cell),
            x: i64::from(*x),
            y: i64::from(*y),
            xs: i64::from(*xs),
            ys: i64::from(*ys),
        },
        C::SplitCell { cell, at } => Command::SplitCell {
            cell: id(cell),
            at: i64::from(*at),
        },
        C::TagCells { cells } => Command::TagCells {
            cells: texts(cells),
        },
        C::UntagCells { cells } => Command::UntagCells {
            cells: texts(cells),
        },
        C::AdjustRelSize {
            cells,
            delta,
            recursive,
        } => Command::AdjustRelSize {
            cells: texts(cells),
            delta: i64::from(*delta),
            recursive: *recursive,
        },
        C::AdjustColumnWidths {
            cell,
            column,
            count,
            delta,
            recursive,
        } => Command::AdjustColumnWidths {
            cell: id(cell),
            column: i64::from(*column),
            count: i64::from(*count),
            delta: i64::from(*delta),
            recursive: *recursive,
        },
        C::Run => Command::Run,
        C::EnterText { cell, text } => Command::EnterText {
            cell: id(cell),
            text: text.clone(),
        },
        C::ToggleCheck { cells } => Command::ToggleCheck {
            cells: texts(cells),
        },
        C::PasteCells { to, clip } => Command::PasteCells {
            to: id(to),
            clip: clip_out(clip)?,
        },
        C::AdoptCells { to, clip } => Command::AdoptCells {
            to: id(to),
            clip: clip_out(clip)?,
        },
        C::ReleaseCell { cell } => Command::ReleaseCell { cell: id(cell) },
        C::Import {
            cell,
            table,
            replace,
        } => Command::Import {
            cell: id(cell),
            table: table_out(table),
            replace: *replace,
        },
        C::RetargetLinks { cells, from, to } => Command::RetargetLinks {
            cells: texts(cells),
            from: from.map(|d| d.to_string()),
            to: to.map(|d| d.to_string()),
        },
    })
}

/// A change report to carry, with the formula cells that show something else now.
pub fn report_out(changes: &core::Changes, recalculated: &[CellId]) -> Report {
    Report {
        changed: texts(&changes.changed),
        created: texts(&changes.created),
        removed: texts(&changes.removed),
        grids: texts(&changes.grids),
        tags: changes.tags.clone(),
        recalculated: texts(recalculated),
    }
}

fn cell_out(cell: core::CellView) -> CellInfo {
    CellInfo {
        id: cell.id.to_string(),
        text: cell.text,
        kind: cell.kind.into(),
        result: cell.result,
        failed: cell.failed,
        has_grid: cell.has_grid,
    }
}

/// A view of a grid to carry.
pub(crate) fn grid_out(view: core::GridView) -> GridInfo {
    GridInfo {
        cell: cell_out(view.cell),
        xs: view.xs,
        ys: view.ys,
        cells: view.cells.into_iter().map(cell_out).collect(),
    }
}

/// The request of a stage read, each number checked against what the core takes.
pub fn stage_ask_in(ask: StageAsk) -> In<core::StageAsk> {
    Ok(core::StageAsk {
        x0: unsigned(ask.x0)?,
        y0: unsigned(ask.y0)?,
        cols: unsigned(ask.cols)?,
        rows: unsigned(ask.rows)?,
        full_depth: unsigned(ask.full_depth)?,
        full_cells: unsigned(ask.full_cells)?,
        preview_cols: unsigned(ask.preview_cols)?,
        preview_rows: unsigned(ask.preview_rows)?,
        shape_cols: unsigned(ask.shape_cols)?,
        shape_rows: unsigned(ask.shape_rows)?,
        text_cut: unsigned(ask.text_cut)?,
        preview_cut: unsigned(ask.preview_cut)?,
    })
}

fn cut_out(text: core::CutText) -> CutText {
    CutText {
        text: text.text,
        cut: text.cut,
    }
}

/// One cell with all that is needed to draw it, to carry. What the core holds in the
/// unknown fields of a run or a link is not carried: the core gives none here.
pub fn look_out(look: core::CellLook) -> CellLook {
    let (result, result_cut) = match look.result {
        Some(result) => (Some(result.text), result.cut),
        None => (None, false),
    };
    let run = |run: &core::Run| RunInfo {
        start: run.start,
        len: run.len,
        style: run.style.into(),
        color: run.color.map(Rgb::from),
    };
    let link = |link: &core::Link| LinkInfo {
        start: link.start,
        len: link.len,
        target: target_out(&link.target),
    };
    CellLook {
        id: look.id.to_string(),
        text: look.text.text,
        text_cut: look.text.cut,
        kind: look.kind.into(),
        result,
        result_cut,
        failed: look.failed,
        cell_color: look.cell_color.into(),
        text_color: look.text_color.into(),
        is_tag: look.is_tag,
        style: look.style.into(),
        rel_size: look.rel_size,
        text_align: look.text_align.into(),
        vert_align: look.vert_align.into(),
        runs: look.runs.iter().map(run).collect(),
        links: look.links.iter().map(link).collect(),
        image: look.image,
        has_note: look.has_note,
        has_grid: look.has_grid,
        summary: look.summary,
        full: look.full,
    }
}

/// The stage and what is nested below it, to carry.
pub fn stage_out(view: core::StageView) -> StageInfo {
    let step = |step: core::TrailStep| TrailStep {
        id: step.id.to_string(),
        shown: cut_out(step.shown),
        color: step.color.into(),
    };
    let grid = |grid: core::GridPart| GridPart {
        owner: grid.owner.to_string(),
        level: grid.level,
        xs: grid.xs,
        ys: grid.ys,
        x0: grid.x0,
        y0: grid.y0,
        cols: grid.cols,
        rows: grid.rows,
        first: grid.first,
        col_widths: grid.col_widths,
        border_color: grid.border_color.into(),
        outer_spacing: grid.outer_spacing,
        vertical: grid.vertical,
        draw_style: grid.draw_style.into(),
    };
    let summary = |summary: core::GridSummary| GridSummary {
        owner: summary.owner.to_string(),
        xs: summary.xs,
        ys: summary.ys,
        border_color: summary.border_color.into(),
        preview_cols: summary.preview_cols,
        preview_rows: summary.preview_rows,
        preview: summary.preview.into_iter().map(cut_out).collect(),
        shape_cols: summary.shape_cols,
        shape_rows: summary.shape_rows,
        shape: summary.shape,
    };
    StageInfo {
        trail: view.trail.into_iter().map(step).collect(),
        grids: view.grids.into_iter().map(grid).collect(),
        cells: view.cells.into_iter().map(look_out).collect(),
        summaries: view.summaries.into_iter().map(summary).collect(),
    }
}
