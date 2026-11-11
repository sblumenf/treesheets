//! Between the serialised form (`api/wire.rs`) and the types of the core. On the way in
//! everything is checked: an ID must be in its canonical text, a number finite, unknown
//! fields a JSON object, and a clip or a table goes through the core's own constructors,
//! which check what they are given. On the way out nothing can fail but a clip that the
//! core would not write as a document.

use std::path::Path;

use wombat_core as core;
use wombat_core::convert::{Entry, Table};
use wombat_core::model::Extra;
use wombat_core::{CellId, Clip, DocId};

use crate::api::wire::{
    BridgeError, CellInfo, CellKind, ClipData, Command, DrawStyle, EntryData, ErrorKind, GridInfo,
    HorizAlign, ImageRef, LinkTarget, Prop, Report, Rgb, Style, TableData, VertAlign,
};

impl BridgeError {
    /// An error of the bridge itself: what came over it cannot be used.
    pub(crate) fn invalid(message: impl Into<String>) -> BridgeError {
        BridgeError {
            kind: ErrorKind::Invalid,
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
    CellId::parse(text).ok_or_else(|| BridgeError::invalid(format!("{text:?} is not an ID")))
}

fn doc_id(text: &str) -> In<DocId> {
    DocId::parse(text).ok_or_else(|| BridgeError::invalid(format!("{text:?} is not an ID")))
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

fn finite(number: f64) -> In<f64> {
    if number.is_finite() {
        Ok(number)
    } else {
        Err(BridgeError::invalid("a scale that is not a finite number"))
    }
}

impl From<Rgb> for core::Color {
    fn from(c: Rgb) -> core::Color {
        core::Color::rgb(c.r, c.g, c.b)
    }
}

impl From<core::Color> for Rgb {
    fn from(c: core::Color) -> Rgb {
        Rgb {
            r: c.r,
            g: c.g,
            b: c.b,
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
    serde_json::from_str(text)
        .map_err(|e| BridgeError::invalid(format!("unknown fields that are no JSON object: {e}")))
}

fn extra_out(extra: &Extra) -> In<String> {
    if extra.is_empty() {
        return Ok(String::new());
    }
    serde_json::to_string(extra).map_err(|e| BridgeError::invalid(e.to_string()))
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
        Prop::CellColor(c) => core::Prop::CellColor(c.into()),
        Prop::TextColor(c) => core::Prop::TextColor(c.into()),
        Prop::RelSize(n) => core::Prop::RelSize(n),
        Prop::TextAlign(a) => core::Prop::TextAlign(a.into()),
        Prop::VertAlign(a) => core::Prop::VertAlign(a.into()),
        Prop::Kind(k) => core::Prop::Kind(k.into()),
        Prop::Note(text) => core::Prop::Note(text),
        Prop::DrawStyle(d) => core::Prop::DrawStyle(d.into()),
        Prop::Image(image) => core::Prop::Image(image.map(image_in).transpose()?),
        Prop::ImageScale(scale) => core::Prop::ImageScale(finite(scale)?),
        Prop::LastEdit(time) => core::Prop::LastEdit(time),
        Prop::BorderColor(c) => core::Prop::BorderColor(c.into()),
        Prop::OuterSpacing(n) => core::Prop::OuterSpacing(n),
        Prop::Vertical(on) => core::Prop::Vertical(on),
    })
}

fn prop_out(prop: &core::Prop) -> In<Prop> {
    Ok(match prop {
        core::Prop::CellColor(c) => Prop::CellColor((*c).into()),
        core::Prop::TextColor(c) => Prop::TextColor((*c).into()),
        core::Prop::RelSize(n) => Prop::RelSize(*n),
        core::Prop::TextAlign(a) => Prop::TextAlign((*a).into()),
        core::Prop::VertAlign(a) => Prop::VertAlign((*a).into()),
        core::Prop::Kind(k) => Prop::Kind((*k).into()),
        core::Prop::Note(text) => Prop::Note(text.clone()),
        core::Prop::DrawStyle(d) => Prop::DrawStyle((*d).into()),
        core::Prop::Image(image) => Prop::Image(image.as_ref().map(image_out).transpose()?),
        core::Prop::ImageScale(scale) => Prop::ImageScale(finite(*scale)?),
        core::Prop::LastEdit(time) => Prop::LastEdit(*time),
        core::Prop::BorderColor(c) => Prop::BorderColor((*c).into()),
        core::Prop::OuterSpacing(n) => Prop::OuterSpacing(*n),
        core::Prop::Vertical(on) => Prop::Vertical(*on),
    })
}

/// A clip from what was carried: the core reads the document as it reads a file, and
/// makes the clip of it as it makes one of any document.
fn clip_in(clip: ClipData) -> In<Clip> {
    let doc = core::format::from_slice(clip.document.as_bytes())?;
    let parent = clip.parent.as_deref().map(cell_id).transpose()?;
    Ok(Clip::from_document(&doc, doc_id(&clip.source)?, parent)?)
}

/// A clip to carry.
pub fn clip_out(clip: &Clip) -> In<ClipData> {
    Ok(ClipData {
        source: clip.source().to_string(),
        parent: clip.parent().map(|p| p.to_string()),
        document: core::format::to_string(&clip.as_document()?)?,
    })
}

fn table_in(table: TableData) -> In<Table> {
    let mut cells = Vec::with_capacity(table.cells.len());
    for entry in table.cells {
        let mut cell = Entry::new(&entry.text);
        if entry.formula {
            cell = cell.as_formula();
        }
        if let Some(grid) = entry.grid {
            cell = cell.with_grid(table_in(grid)?);
        }
        cells.push(cell);
    }
    Ok(Table::new(table.xs, cells)?)
}

fn table_out(table: &Table) -> TableData {
    let entry = |entry: &Entry| EntryData {
        text: entry.text().to_string(),
        formula: entry.is_formula(),
        grid: entry.grid().map(table_out),
    };
    TableData {
        xs: table.xs(),
        cells: table.cells().iter().map(entry).collect(),
    }
}

/// The command of the core that a carried command stands for.
pub fn command_in(command: Command) -> In<core::Command> {
    use core::Command as C;
    Ok(match command {
        Command::InsertText { cell, at, text } => C::InsertText {
            cell: cell_id(&cell)?,
            at,
            text,
        },
        Command::DeleteText { cell, start, len } => C::DeleteText {
            cell: cell_id(&cell)?,
            start,
            len,
        },
        Command::ReplaceText {
            cell,
            start,
            len,
            text,
        } => C::ReplaceText {
            cell: cell_id(&cell)?,
            start,
            len,
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
            start,
            len,
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
            start,
            len,
            color: color.map(Into::into),
        },
        Command::SetLink {
            cell,
            start,
            len,
            target,
        } => C::SetLink {
            cell: cell_id(&cell)?,
            start,
            len,
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
            column,
            width,
        },
        Command::CreateGrid { cell, xs, ys } => C::CreateGrid {
            cell: cell_id(&cell)?,
            xs,
            ys,
        },
        Command::RemoveGrid { cell } => C::RemoveGrid {
            cell: cell_id(&cell)?,
        },
        Command::InsertRows { cell, at, count } => C::InsertRows {
            cell: cell_id(&cell)?,
            at,
            count,
        },
        Command::InsertColumns { cell, at, count } => C::InsertColumns {
            cell: cell_id(&cell)?,
            at,
            count,
        },
        Command::DeleteRows { cell, at, count } => C::DeleteRows {
            cell: cell_id(&cell)?,
            at,
            count,
        },
        Command::DeleteColumns { cell, at, count } => C::DeleteColumns {
            cell: cell_id(&cell)?,
            at,
            count,
        },
        Command::ClearCells { cells } => C::ClearCells {
            cells: cell_ids(&cells)?,
        },
        Command::DeleteCells { cell, x, y, xs, ys } => C::DeleteCells {
            cell: cell_id(&cell)?,
            x,
            y,
            xs,
            ys,
        },
        Command::MoveRows {
            cell,
            from,
            count,
            to,
        } => C::MoveRows {
            cell: cell_id(&cell)?,
            from,
            count,
            to,
        },
        Command::MoveColumns {
            cell,
            from,
            count,
            to,
        } => C::MoveColumns {
            cell: cell_id(&cell)?,
            from,
            count,
            to,
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
            cell_color: cell_color.into(),
            text_color: text_color.into(),
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
            column,
            row,
            count,
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
            x,
            y,
            xs,
            ys,
        },
        Command::Collapse { cell, x, y, xs, ys } => C::Collapse {
            cell: cell_id(&cell)?,
            x,
            y,
            xs,
            ys,
        },
        Command::SplitCell { cell, at } => C::SplitCell {
            cell: cell_id(&cell)?,
            at,
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
            delta,
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
            column,
            count,
            delta,
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
            at: *at,
            text: text.clone(),
        },
        C::DeleteText { cell, start, len } => Command::DeleteText {
            cell: id(cell),
            start: *start,
            len: *len,
        },
        C::ReplaceText {
            cell,
            start,
            len,
            text,
        } => Command::ReplaceText {
            cell: id(cell),
            start: *start,
            len: *len,
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
            start: *start,
            len: *len,
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
            start: *start,
            len: *len,
            color: color.map(Into::into),
        },
        C::SetLink {
            cell,
            start,
            len,
            target,
        } => Command::SetLink {
            cell: id(cell),
            start: *start,
            len: *len,
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
            column: *column,
            width: *width,
        },
        C::CreateGrid { cell, xs, ys } => Command::CreateGrid {
            cell: id(cell),
            xs: *xs,
            ys: *ys,
        },
        C::RemoveGrid { cell } => Command::RemoveGrid { cell: id(cell) },
        C::InsertRows { cell, at, count } => Command::InsertRows {
            cell: id(cell),
            at: *at,
            count: *count,
        },
        C::InsertColumns { cell, at, count } => Command::InsertColumns {
            cell: id(cell),
            at: *at,
            count: *count,
        },
        C::DeleteRows { cell, at, count } => Command::DeleteRows {
            cell: id(cell),
            at: *at,
            count: *count,
        },
        C::DeleteColumns { cell, at, count } => Command::DeleteColumns {
            cell: id(cell),
            at: *at,
            count: *count,
        },
        C::ClearCells { cells } => Command::ClearCells {
            cells: texts(cells),
        },
        C::DeleteCells { cell, x, y, xs, ys } => Command::DeleteCells {
            cell: id(cell),
            x: *x,
            y: *y,
            xs: *xs,
            ys: *ys,
        },
        C::MoveRows {
            cell,
            from,
            count,
            to,
        } => Command::MoveRows {
            cell: id(cell),
            from: *from,
            count: *count,
            to: *to,
        },
        C::MoveColumns {
            cell,
            from,
            count,
            to,
        } => Command::MoveColumns {
            cell: id(cell),
            from: *from,
            count: *count,
            to: *to,
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
            column: *column,
            row: *row,
            count: *count,
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
            x: *x,
            y: *y,
            xs: *xs,
            ys: *ys,
        },
        C::Collapse { cell, x, y, xs, ys } => Command::Collapse {
            cell: id(cell),
            x: *x,
            y: *y,
            xs: *xs,
            ys: *ys,
        },
        C::SplitCell { cell, at } => Command::SplitCell {
            cell: id(cell),
            at: *at,
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
            delta: *delta,
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
            column: *column,
            count: *count,
            delta: *delta,
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
