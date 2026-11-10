//! View state: which grids are folded, the selection and the zoom (decision A8).
//!
//! View state is not content. It is kept in its own small JSON file, apart from the document
//! (see [`crate::storage`] for where), and refers to cells by ID. It may be out of date with
//! the document, so it can name cells that no longer exist; [`ViewState::retain_existing`]
//! drops those.

use std::collections::BTreeSet;

use crate::error::{invalid, Error};
use crate::id::{CellId, DocId};
use crate::json::{
    into_extra, into_object, parse, take_array, take_header, take_id, take_u32, Fields,
};
use crate::limits::MAX_CELLS;
use crate::model::{Document, Extra};

/// The value of the `format` field of a view-state file.
pub const VIEW_FORMAT_NAME: &str = "wombat-view";
/// The major version of the view-state format this core writes and reads.
pub const VIEW_FORMAT_VERSION: u32 = 1;
/// The minor version of the view-state format this core writes.
pub const VIEW_FORMAT_MINOR: u32 = 0;

/// A selection: a rectangle of cells in one grid, or a text cursor in one cell.
#[derive(Clone, PartialEq, Debug)]
pub struct Selection {
    /// The top-left cell of the selection.
    pub cell: CellId,
    /// The number of columns selected, from `cell` rightwards. As in TreeSheets, 0 means an
    /// insertion point before the cell rather than the cell itself.
    pub xs: u32,
    /// The number of rows selected, from `cell` downwards; 0 as for `xs`.
    pub ys: u32,
    /// The text cursor inside `cell` when its text is being edited: start and end of the
    /// selected text in code points (equal for a plain cursor).
    pub cursor: Option<(u32, u32)>,
    /// Unrecognised fields.
    pub extra: Extra,
}

/// The view state of one document on one device.
#[derive(Clone, PartialEq, Debug)]
pub struct ViewState {
    /// The document this belongs to.
    pub doc: DocId,
    /// The cells whose grid is folded (hidden).
    pub folded: BTreeSet<CellId>,
    /// The selection.
    pub selection: Option<Selection>,
    /// The cell that is zoomed into and shown as if it were the root.
    pub zoom: Option<CellId>,
    /// Unrecognised fields.
    pub extra: Extra,
    minor: u32,
}

const VIEW_KEYS: &[&str] = &[
    "format",
    "version",
    "minor",
    "doc",
    "selection",
    "zoom",
    "folded",
];
const SELECTION_KEYS: &[&str] = &["cell", "xs", "ys", "cursor"];

impl ViewState {
    /// An empty view state: nothing folded, no selection, not zoomed.
    pub fn new(doc: DocId) -> ViewState {
        ViewState {
            doc,
            folded: BTreeSet::new(),
            selection: None,
            zoom: None,
            extra: Extra::new(),
            minor: 0,
        }
    }

    /// Forgets everything that refers to a cell that is not in `doc`.
    pub fn retain_existing(&mut self, doc: &Document) {
        self.folded.retain(|id| doc.cell(*id).is_some());
        if self
            .selection
            .as_ref()
            .is_some_and(|s| doc.cell(s.cell).is_none())
        {
            self.selection = None;
        }
        if self.zoom.is_some_and(|id| doc.cell(id).is_none()) {
            self.zoom = None;
        }
    }

    /// Writes the view state as text; the same state always gives the same bytes.
    pub fn to_json(&self) -> Result<String, Error> {
        let known = |extra: &Extra, keys: &[&str]| extra.keys().any(|k| keys.contains(&k.as_str()));
        if known(&self.extra, VIEW_KEYS)
            || self
                .selection
                .as_ref()
                .is_some_and(|s| known(&s.extra, SELECTION_KEYS))
        {
            return invalid("a known view-state field is among the unknown fields");
        }
        let mut out = String::from("{\n");
        let mut head = Fields::new(&mut out);
        head.string("format", VIEW_FORMAT_NAME);
        head.display("version", VIEW_FORMAT_VERSION);
        head.display(
            "minor",
            crate::json::minor_to_write(self.minor, VIEW_FORMAT_MINOR),
        );
        head.quoted("doc", self.doc);
        if let Some(selection) = &self.selection {
            out.push_str(",\n\"selection\": {");
            let mut f = Fields::new(&mut out);
            f.quoted("cell", selection.cell);
            f.display("xs", selection.xs);
            f.display("ys", selection.ys);
            if let Some((start, end)) = selection.cursor {
                f.display("cursor", format_args!("[{start}, {end}]"));
            }
            f.extra(&selection.extra)?;
            out.push('}');
        }
        if let Some(zoom) = self.zoom {
            out.push_str(&format!(",\n\"zoom\": \"{zoom}\""));
        }
        for (key, value) in &self.extra {
            out.push_str(",\n");
            crate::json::write_string(&mut out, key);
            out.push_str(": ");
            crate::json::write_value(&mut out, value)?;
        }
        if !self.folded.is_empty() {
            out.push_str(",\n\"folded\": [");
            for (i, id) in self.folded.iter().enumerate() {
                out.push_str(if i == 0 { "\n " } else { ",\n " });
                out.push_str(&format!("\"{id}\""));
            }
            out.push_str("\n]");
        }
        out.push_str("\n}\n");
        Ok(out)
    }

    /// Reads view-state text. Any input gives either a view state or an error.
    pub fn from_json(bytes: &[u8]) -> Result<ViewState, Error> {
        let place = "the view state";
        let mut map = into_object(parse(bytes)?, place)?;
        let minor = take_header(&mut map, VIEW_FORMAT_NAME, VIEW_FORMAT_VERSION)?;
        let Some(doc) = take_id(&mut map, "doc", place)?.map(DocId) else {
            return invalid("the view state has no \"doc\"");
        };
        let mut view = ViewState::new(doc);
        view.minor = minor;
        if let Some(value) = map.remove("selection") {
            let place = "the selection";
            let mut sel = into_object(value, place)?;
            let (Some(cell), Some(xs), Some(ys)) = (
                take_id(&mut sel, "cell", place)?.map(CellId),
                take_u32(&mut sel, "xs", place)?,
                take_u32(&mut sel, "ys", place)?,
            ) else {
                return invalid("the selection needs \"cell\", \"xs\" and \"ys\"");
            };
            let cursor = match take_array(&mut sel, "cursor", place)? {
                None => None,
                Some(pair) => {
                    let position =
                        |i: usize| pair.get(i)?.as_u64().and_then(|n| u32::try_from(n).ok());
                    match (pair.len(), position(0), position(1)) {
                        (2, Some(start), Some(end)) if start <= end => Some((start, end)),
                        _ => return invalid("the cursor is not [start, end]"),
                    }
                }
            };
            view.selection = Some(Selection {
                cell,
                xs,
                ys,
                cursor,
                extra: into_extra(sel),
            });
        }
        view.zoom = take_id(&mut map, "zoom", place)?.map(CellId);
        if let Some(folded) = take_array(&mut map, "folded", place)? {
            if folded.len() > MAX_CELLS {
                return Err(Error::Limit(format!("more than {MAX_CELLS} folded cells")));
            }
            for value in folded {
                match value.as_str().and_then(CellId::parse) {
                    Some(id) => view.folded.insert(id),
                    None => return invalid("\"folded\" has an entry that is not an ID"),
                };
            }
        }
        view.extra = into_extra(map);
        Ok(view)
    }
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
    use crate::id::{IdSource, SeededIds};
    use serde_json::json;

    fn sample() -> (Document, ViewState) {
        let mut ids = SeededIds::from_u64(3);
        let mut doc = Document::new(&mut ids);
        doc.add_grid(doc.root(), 2, 2, &mut ids).unwrap();
        let a = doc.child(doc.root(), 0, 0).unwrap();
        let b = doc.child(doc.root(), 1, 1).unwrap();
        let mut view = ViewState::new(doc.id());
        view.folded.insert(doc.root());
        view.folded.insert(b);
        view.selection = Some(Selection {
            cell: a,
            xs: 2,
            ys: 1,
            cursor: Some((1, 4)),
            extra: Extra::new(),
        });
        view.zoom = Some(b);
        (doc, view)
    }

    #[test]
    fn view_state_round_trips_and_is_deterministic() {
        let (_, mut view) = sample();
        view.extra.insert("scale".into(), json!(1.5));
        view.selection
            .as_mut()
            .unwrap()
            .extra
            .insert("later".into(), json!({"a": [1]}));
        let text = view.to_json().unwrap();
        let back = ViewState::from_json(text.as_bytes()).unwrap();
        assert_eq!(back, view);
        assert_eq!(back.to_json().unwrap(), text);
        assert!(text.starts_with("{\n\"format\": \"wombat-view\", \"version\": 1, \"minor\": 0, "));
    }

    #[test]
    fn empty_view_state_is_small() {
        let view = ViewState::new(DocId(crate::Id::from_u128(1)));
        let text = view.to_json().unwrap();
        assert_eq!(
            text,
            "{\n\"format\": \"wombat-view\", \"version\": 1, \"minor\": 0, \
             \"doc\": \"00000000-0000-0000-0000-000000000001\"\n}\n"
        );
        assert_eq!(ViewState::from_json(text.as_bytes()).unwrap(), view);
    }

    #[test]
    fn stale_references_are_dropped() {
        let (doc, mut view) = sample();
        let mut ids = SeededIds::from_u64(99);
        let gone = CellId(ids.next_id());
        view.folded.insert(gone);
        view.retain_existing(&doc);
        assert_eq!(view.folded.len(), 2);
        assert!(view.selection.is_some() && view.zoom.is_some());
        view.zoom = Some(gone);
        view.selection.as_mut().unwrap().cell = gone;
        view.retain_existing(&doc);
        assert!(view.selection.is_none() && view.zoom.is_none());
    }

    #[test]
    fn bad_view_state_is_refused() {
        let (_, view) = sample();
        let good: serde_json::Value = serde_json::from_str(&view.to_json().unwrap()).unwrap();
        let with = |key: &str, value: serde_json::Value| {
            let mut v = good.clone();
            v[key] = value;
            ViewState::from_json(v.to_string().as_bytes())
        };
        assert!(matches!(
            with("format", json!("wombat")),
            Err(Error::WrongFormat(_))
        ));
        assert!(matches!(
            with("version", json!(2)),
            Err(Error::NewerVersion { found: 2, .. })
        ));
        assert!(matches!(with("doc", json!(5)), Err(Error::Invalid(_))));
        assert!(matches!(
            with("folded", json!(["x"])),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(with("zoom", json!("x")), Err(Error::Invalid(_))));
        assert!(matches!(
            with("selection", json!({"cell": 1})),
            Err(Error::Invalid(_))
        ));
        let mut bad_cursor = good.clone();
        bad_cursor["selection"]["cursor"] = json!([4, 1]);
        assert!(ViewState::from_json(bad_cursor.to_string().as_bytes()).is_err());
        assert!(with("minor", json!(7)).is_ok());
        assert!(matches!(ViewState::from_json(b"[1"), Err(Error::Syntax(_))));
        let mut view = view;
        view.extra.insert("doc".into(), json!(1));
        assert!(view.to_json().is_err());
    }
}
