//! `wombat links`, `backlinks`, `broken`, `copy-cell` and `move-cell`: what links point at
//! and what points at a cell, across the documents of a folder. A thin layer over
//! `wombat_core::links` and `wombat_core::scan`.

use std::collections::HashMap;
use std::path::Path;

use wombat_core::eval::Calc;
use wombat_core::links::{self, Broken, LinkEntry, Resolved};
use wombat_core::scan::{self, WorkspaceLinks};
use wombat_core::{storage, CellId, DocId, Document, LinkTarget, Workspace};

use crate::Failure;

/// The cell that a word names in `doc`: a place (`/2,1/1,3`) or an ID.
pub fn find_cell(doc: &Document, word: &str) -> Result<CellId, String> {
    let Some(place) = word.strip_prefix('/') else {
        let id = CellId::parse(word.strip_prefix('#').unwrap_or(word))
            .ok_or_else(|| format!("{word:?} is neither a place like /2,1 nor an ID"))?;
        return match doc.cell(id) {
            Some(_) => Ok(id),
            None => Err(format!("there is no cell {id}")),
        };
    };
    let mut at = doc.root();
    for step in place.split('/').filter(|step| !step.is_empty()) {
        let numbers = step.split_once(',').and_then(|(x, y)| {
            let x: u32 = x.parse().ok()?;
            let y: u32 = y.parse().ok()?;
            Some((x.checked_sub(1)?, y.checked_sub(1)?))
        });
        let (x, y) =
            numbers.ok_or_else(|| format!("{word:?}: {step:?} is not column,row from 1"))?;
        at = doc
            .child(at, x, y)
            .ok_or_else(|| format!("{word:?}: there is no cell {step} there"))?;
    }
    Ok(at)
}

/// The place of a cell, as [`find_cell`] reads it.
pub fn place_of(doc: &Document, id: CellId) -> String {
    let mut steps = Vec::new();
    let mut at = id;
    while let Some(parent) = doc.cell(at).and_then(|c| c.parent()) {
        let grid = doc.cell(parent).and_then(|c| c.grid());
        let index = grid.and_then(|g| g.cells().iter().position(|c| *c == at));
        let (Some(grid), Some(index)) = (grid, index) else {
            break;
        };
        let xs = grid.xs().max(1) as usize;
        steps.push(format!(
            "/{},{}",
            (index % xs).saturating_add(1),
            (index / xs).saturating_add(1)
        ));
        at = parent;
    }
    if steps.is_empty() {
        return "/".into();
    }
    steps.reverse();
    steps.concat()
}

/// A workspace folder with its link index, and the documents loaded so far to show cells.
struct Folder {
    workspace: Workspace,
    links: WorkspaceLinks,
    /// With what their formula cells show.
    loaded: HashMap<DocId, Option<(Document, Calc)>>,
}

impl Folder {
    fn open(dir: &Path, cache: bool) -> Result<Folder, Failure> {
        let dir = if dir.as_os_str().is_empty() {
            Path::new(".")
        } else {
            dir
        };
        let workspace = Workspace::open(dir)?;
        let mode = if cache {
            scan::CacheMode::Fast
        } else {
            scan::CacheMode::Off
        };
        let links = scan::link_index(&workspace, mode)?;
        Ok(Folder {
            workspace,
            links,
            loaded: HashMap::new(),
        })
    }

    fn name(&self, doc: DocId) -> String {
        match self.links.names.get(&doc) {
            Some(name) => name.clone(),
            None => format!("document {doc}"),
        }
    }

    fn doc(&mut self, id: DocId) -> Option<&Document> {
        let (workspace, names) = (&self.workspace, &self.links.names);
        self.loaded
            .entry(id)
            .or_insert_with(|| {
                let doc = workspace.open_name(names.get(&id)?).ok()?;
                let calc = Calc::new(&doc);
                Some((doc, calc))
            })
            .as_ref()
            .map(|(doc, _)| doc)
    }

    /// A cell for showing: its place, its text and the texts of the cells around it.
    fn cell(&mut self, doc: DocId, cell: CellId) -> String {
        self.doc(doc);
        let Some(Some((found, calc))) = self.loaded.get(&doc) else {
            return format!("cell {cell}");
        };
        match links::place_shown(found, calc, cell) {
            Some(place) if place.trail.is_empty() => {
                format!("{} {:?}", place_of(found, cell), place.text)
            }
            Some(place) => {
                let trail: Vec<String> = place.trail.iter().map(|t| format!("{t:?}")).collect();
                format!(
                    "{} {:?} (in: {})",
                    place_of(found, cell),
                    place.text,
                    trail.join(" > ")
                )
            }
            None => format!("cell {cell}"),
        }
    }

    /// What a link points at, in words; the flag says whether it points at nothing.
    fn target(&mut self, from: DocId, link: &LinkEntry) -> (String, bool) {
        match self.links.index.resolve(from, &link.target) {
            Resolved::External(target) => {
                let kind = match links::external(&target) {
                    links::External::Url => "URL",
                    links::External::Path => "file",
                };
                (format!("{kind} {target}"), false)
            }
            Resolved::Cell { doc, cell, moved } => {
                let shown = self.cell(doc, cell);
                let note = if moved {
                    " (moved there; the link still names another document)"
                } else {
                    ""
                };
                (format!("{} {shown}{note}", self.name(doc)), false)
            }
            Resolved::Broken(why) => {
                let cell = match link.target {
                    LinkTarget::Cell { cell, .. } => cell.to_string(),
                    _ => String::new(),
                };
                let why = match why {
                    Broken::NoDocument(doc) => {
                        format!("BROKEN: cell {cell}; there is no document {doc} in the folder")
                    }
                    Broken::NoCell(doc) => {
                        format!("BROKEN: there is no cell {cell} in {}", self.name(doc))
                    }
                    Broken::AmbiguousDocument(doc) => {
                        format!("AMBIGUOUS: cell {cell}; several files have the document ID {doc}")
                    }
                    Broken::AmbiguousCell(docs) => {
                        let names: Vec<String> = docs.iter().map(|d| self.name(*d)).collect();
                        format!(
                            "AMBIGUOUS: cell {cell} is not where the link says and is in: {}",
                            names.join(", ")
                        )
                    }
                    Broken::Unknown => match &link.hint {
                        Some(text) => format!(
                            "NO TARGET: a TreeSheets link to a cell with the text {text:?}, \
                             which did not exist"
                        ),
                        None => "NO TARGET: a kind of link this version does not know".into(),
                    },
                };
                (why, true)
            }
        }
    }

    /// The warnings about the folder itself.
    fn warnings(&self) -> String {
        let mut out = String::new();
        for (doc, files) in &self.links.duplicates {
            out.push_str(&format!(
                "warning: {} have the same document ID {doc}; links into it are ambiguous \
                 (make copies with \"wombat copy\")\n",
                files.join(" and ")
            ));
        }
        for (name, error) in &self.links.failed {
            out.push_str(&format!("warning: {name} could not be read: {error}\n"));
        }
        out
    }
}

fn range(link: &LinkEntry) -> String {
    format!("[{}+{} {:?}]", link.start, link.len, link.text)
}

/// `wombat links`: the links of one document, in document order, with what they point at.
pub fn links(file: &str, cache: bool) -> Result<String, Failure> {
    let path = Path::new(file);
    let doc = storage::load_document(path)?;
    let mut folder = Folder::open(path.parent().unwrap_or(Path::new("")), cache)?;
    // The file itself counts as it is now, whatever the folder scan made of it.
    folder.links.index.insert_document(&doc);
    let id = doc.id();
    let mut out = String::new();
    let (mut count, mut broken) = (0usize, 0usize);
    for cell in doc.cells_in_order() {
        let list = match folder.links.index.document(id) {
            Some(summary) => summary.links_of(cell).to_vec(),
            None => Vec::new(),
        };
        for link in list {
            let (target, bad) = folder.target(id, &link);
            let text = doc.cell(cell).map(|c| c.text.as_str()).unwrap_or_default();
            out.push_str(&format!(
                "{} {text:?} {} -> {target}\n",
                place_of(&doc, cell),
                range(&link)
            ));
            count = count.saturating_add(1);
            broken = broken.saturating_add(usize::from(bad));
        }
    }
    out.push_str(&format!(
        "{count} links, {broken} of them pointing at nothing\n"
    ));
    Ok(out + &folder.warnings())
}

/// `wombat backlinks`: the links to one cell from all documents of a folder.
pub fn backlinks(dir: &str, document: &str, cell: &str, cache: bool) -> Result<String, Failure> {
    let mut folder = Folder::open(Path::new(dir), cache)?;
    let doc = folder.workspace.open_name(document)?;
    let id = doc.id();
    let cell = find_cell(&doc, cell).map_err(Failure::Failed)?;
    if folder.links.names.get(&id).map(String::as_str) != Some(document) {
        return Err(Failure::Failed(format!(
            "{document} has the document ID of {}; links to its cells cannot be told apart",
            folder.name(id)
        )));
    }
    let mut out = format!("{document} {}\n", folder.cell(id, cell));
    let found = folder.links.index.backlinks(id, cell);
    // By file name and then in the order of the document that has the link.
    let mut lines = Vec::new();
    for link in &found {
        let order = folder
            .doc(link.doc)
            .and_then(|d| d.cells_in_order().iter().position(|c| *c == link.cell));
        let shown = folder.cell(link.doc, link.cell);
        let moved = if link.moved {
            " (the link names another document; the cell moved)"
        } else {
            ""
        };
        lines.push((
            folder.name(link.doc),
            order,
            link.start,
            format!(
                "  {} {shown} [{}+{} {:?}]{moved}\n",
                folder.name(link.doc),
                link.start,
                link.len,
                link.text
            ),
        ));
    }
    lines.sort();
    out.push_str(&format!("{} links to it\n", lines.len()));
    for (_, _, _, line) in lines {
        out.push_str(&line);
    }
    Ok(out + &folder.warnings())
}

/// `wombat broken`: every link in a folder that points at nothing.
pub fn broken(dir: &str, cache: bool) -> Result<String, Failure> {
    let mut folder = Folder::open(Path::new(dir), cache)?;
    let mut lines = Vec::new();
    for link in folder.links.index.broken() {
        let (target, _) = folder.target(link.doc, &link.link);
        let shown = folder.cell(link.doc, link.cell);
        lines.push(format!(
            "{} {shown} {} -> {target}\n",
            folder.name(link.doc),
            range(&link.link)
        ));
    }
    lines.sort();
    let mut out = lines.concat();
    out.push_str(&format!(
        "{} links pointing at nothing in {} documents\n",
        lines.len(),
        folder.links.index.documents().count()
    ));
    Ok(out + &folder.warnings())
}

/// `wombat copy-cell` and `wombat move-cell`: a cell with all in it goes from one
/// document to another; both files are saved.
pub fn transfer(
    from: &str,
    cell: &str,
    to: &str,
    place: &str,
    moving: bool,
) -> Result<String, Failure> {
    let (from_path, to_path) = (Path::new(from), Path::new(to));
    let source = storage::load_document(from_path)?;
    let target = storage::load_document(to_path)?;
    let cell = find_cell(&source, cell).map_err(|e| Failure::Failed(format!("{from}: {e}")))?;
    let place = find_cell(&target, place).map_err(|e| Failure::Failed(format!("{to}: {e}")))?;
    let mut ids = crate::os_ids()?;
    // All of it is the library's: image files, tags, folds, and saving both or neither.
    let done = storage::transfer_cell(from_path, cell, to_path, place, moving, &mut ids)?;
    let copied = done.images.copied.len();
    let mut out = if moving {
        format!(
            "moved {} cells from {from} to {to}; they keep their IDs, so links to them still \
             lead to them\n",
            done.cells
        )
    } else {
        format!(
            "copied {} cells from {from} into {to}; the copies inside have new IDs\n",
            done.cells
        )
    };
    if copied > 0 || !done.images.reused.is_empty() {
        out.push_str(&format!(
            "{copied} image file{} copied into the image folder of {to}\n",
            if copied == 1 { "" } else { "s" }
        ));
    }
    for (old, new) in &done.images.renamed {
        out.push_str(&format!(
            "the image {old} is called {new} there: {to} has another image called {old}\n"
        ));
    }
    for name in &done.removed_images {
        out.push_str(&format!(
            "the image file {name} was removed from the image folder of {from}, where no \
             cell shows it any more\n"
        ));
    }
    Ok(out)
}
