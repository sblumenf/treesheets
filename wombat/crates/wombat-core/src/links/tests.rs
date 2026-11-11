//! Unit tests of links: resolution, backlinks, deletion, ambiguity, copying and moving
//! between documents, and what TreeSheets' link actions became.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use super::*;
use crate::edit::{Clip, Editor};
use crate::error::Error;
use crate::id::{Id, SeededIds};
use crate::model::Extra;

/// A cell to build: its text, the number of columns of its grid, the cells of the grid.
struct N(&'static str, u32, Vec<N>);

fn n(text: &'static str) -> N {
    N(text, 0, Vec::new())
}

fn g(text: &'static str, xs: u32, kids: Vec<N>) -> N {
    N(text, xs, kids)
}

fn fill(doc: &mut Document, ids: &mut SeededIds, cell: CellId, spec: &N) {
    doc.cell_mut(cell).unwrap().text = spec.0.to_string();
    if spec.1 == 0 {
        return;
    }
    let ys = spec.2.len() as u32 / spec.1;
    doc.add_grid(cell, spec.1, ys, ids).unwrap();
    let kids = doc.cell(cell).unwrap().grid().unwrap().cells().to_vec();
    for (kid, spec) in kids.iter().zip(&spec.2) {
        fill(doc, ids, *kid, spec);
    }
}

fn build(seed: u64, spec: &N) -> (Document, SeededIds) {
    let mut ids = SeededIds::from_u64(seed);
    let mut doc = Document::new(&mut ids);
    let root = doc.root();
    fill(&mut doc, &mut ids, root, spec);
    doc.validate().unwrap();
    (doc, ids)
}

/// The cell at a path of (column, row) steps from the root.
fn at(doc: &Document, path: &[(u32, u32)]) -> CellId {
    path.iter()
        .fold(doc.root(), |cell, (x, y)| doc.child(cell, *x, *y).unwrap())
}

fn link(start: u32, len: u32, doc: Option<DocId>, cell: CellId) -> Link {
    Link {
        start,
        len,
        target: LinkTarget::Cell { doc, cell },
        extra: Extra::new(),
    }
}

fn to_cell(doc: Option<DocId>, cell: CellId) -> LinkTarget {
    LinkTarget::Cell { doc, cell }
}

fn rebuilt(docs: &[&Document]) -> LinkIndex {
    let mut index = LinkIndex::new();
    for doc in docs {
        index.insert_document(doc);
    }
    index
}

fn found(doc: DocId, cell: CellId) -> Resolved {
    Resolved::Cell {
        doc,
        cell,
        moved: false,
    }
}

fn shop_spec() -> N {
    g(
        "Shop",
        2,
        vec![
            n("Fruit"),
            n("Price"),
            g("Apple", 1, vec![n("red"), n("green")]),
            n("1.20"),
            n("Pear"),
            n(""),
        ],
    )
}

fn notes_spec() -> N {
    g(
        "Notes",
        1,
        vec![n("see the apple here"), n("and green"), n("")],
    )
}

/// Two documents: `notes` links to `Apple` and `green` of `shop`; `shop` links to itself.
fn pair() -> (Document, Document, SeededIds) {
    let (mut shop, _) = build(1, &shop_spec());
    let (mut notes, ids) = build(2, &notes_spec());
    let apple = at(&shop, &[(0, 1)]);
    let green = at(&shop, &[(0, 1), (0, 1)]);
    let first = at(&notes, &[(0, 0)]);
    let second = at(&notes, &[(0, 1)]);
    notes.cell_mut(first).unwrap().links = vec![link(8, 5, Some(shop.id()), apple)];
    notes.cell_mut(second).unwrap().links = vec![link(4, 5, Some(shop.id()), green)];
    let pear = at(&shop, &[(0, 2)]);
    shop.cell_mut(pear).unwrap().links = vec![link(0, 4, None, apple)];
    shop.validate().unwrap();
    notes.validate().unwrap();
    (shop, notes, ids)
}

#[test]
fn a_link_resolves_to_a_cell_here_a_cell_elsewhere_an_external_target_or_nothing() {
    let (shop, notes, _) = pair();
    let index = rebuilt(&[&shop, &notes]);
    let apple = at(&shop, &[(0, 1)]);
    let green = at(&shop, &[(0, 1), (0, 1)]);
    // Within the document, and from another document found by its ID.
    assert_eq!(
        index.resolve(shop.id(), &to_cell(None, apple)),
        found(shop.id(), apple)
    );
    assert_eq!(
        index.resolve(notes.id(), &to_cell(Some(shop.id()), green)),
        found(shop.id(), green)
    );
    // Enough to show it: the target's text and where it is.
    let place = place(&shop, green).unwrap();
    assert_eq!(place.text, "green");
    assert_eq!(place.trail, ["Shop", "Apple"]);
    assert_eq!(super::place(&shop, shop.root()).unwrap().trail.len(), 0);
    assert_eq!(super::place(&shop, at(&notes, &[(0, 0)])), None);

    let url = LinkTarget::External("https://example.org".into());
    assert_eq!(
        index.resolve(shop.id(), &url),
        Resolved::External("https://example.org".into())
    );
    assert_eq!(
        index.resolve(shop.id(), &LinkTarget::Unknown),
        Resolved::Broken(Broken::Unknown)
    );
    // A cell that no document has, in a known and in an unknown document.
    let nowhere = CellId(Id::from_u128(77));
    let no_doc = DocId(Id::from_u128(78));
    assert_eq!(
        index.resolve(shop.id(), &to_cell(None, nowhere)),
        Resolved::Broken(Broken::NoCell(shop.id()))
    );
    assert_eq!(
        index.resolve(shop.id(), &to_cell(Some(notes.id()), nowhere)),
        Resolved::Broken(Broken::NoCell(notes.id()))
    );
    assert_eq!(
        index.resolve(shop.id(), &to_cell(Some(no_doc), nowhere)),
        Resolved::Broken(Broken::NoDocument(no_doc))
    );
    // A link from a document that is not in the index still resolves into those that are.
    assert_eq!(
        index.resolve(no_doc, &to_cell(Some(shop.id()), apple)),
        found(shop.id(), apple)
    );
}

#[test]
fn backlinks_list_the_links_to_a_cell_from_all_documents_in_a_fixed_order() {
    let (shop, notes, _) = pair();
    let apple = at(&shop, &[(0, 1)]);
    let green = at(&shop, &[(0, 1), (0, 1)]);
    let pear = at(&shop, &[(0, 2)]);
    let first = at(&notes, &[(0, 0)]);
    for index in [rebuilt(&[&shop, &notes]), rebuilt(&[&notes, &shop])] {
        let links = index.backlinks(shop.id(), apple);
        let mut expected = vec![
            (shop.id(), pear, 0, 4, "Pear"),
            (notes.id(), first, 8, 5, "apple"),
        ];
        expected.sort();
        let got: Vec<_> = links
            .iter()
            .map(|l| (l.doc, l.cell, l.start, l.len, l.text.as_str()))
            .collect();
        assert_eq!(got, expected);
        assert!(links.iter().all(|l| l.target == apple && !l.moved));
        assert_eq!(index.backlinks(shop.id(), green).len(), 1);
        assert_eq!(index.backlinks(shop.id(), pear), []);
        // The cell is in `shop`, not in `notes`.
        assert_eq!(index.backlinks(notes.id(), apple), []);

        let out = index.outgoing(notes.id());
        assert_eq!(out.len(), 2);
        assert!(out
            .iter()
            .all(|o| matches!(o.resolved, Resolved::Cell { .. })));
        assert_eq!(index.broken(), []);
        assert_eq!(index.owners(apple), [shop.id()]);
    }
}

#[test]
fn several_links_in_one_cell_keep_their_texts_also_after_characters_of_several_bytes() {
    let (mut doc, _) = build(3, &g("r", 1, vec![n("é😀 one two"), n("x")]));
    let (a, b) = (at(&doc, &[(0, 0)]), at(&doc, &[(0, 1)]));
    doc.cell_mut(a).unwrap().links = vec![
        link(0, 2, None, b),
        Link {
            start: 3,
            len: 3,
            target: LinkTarget::External("u".into()),
            extra: Extra::new(),
        },
        link(7, 3, None, b),
    ];
    let texts: Vec<String> = entries(doc.cell(a).unwrap())
        .into_iter()
        .map(|e| e.text)
        .collect();
    assert_eq!(texts, ["é😀", "one", "two"]);
    let index = rebuilt(&[&doc]);
    // Two links from one cell to one cell are two backlinks.
    assert_eq!(index.backlinks(doc.id(), b).len(), 2);
}

#[test]
fn deleting_a_target_breaks_the_link_and_undo_mends_it() {
    let (shop, notes, mut ids) = pair();
    let apple = at(&shop, &[(0, 1)]);
    let green = at(&shop, &[(0, 1), (0, 1)]);
    let pear = at(&shop, &[(0, 2)]);
    let mut index = rebuilt(&[&shop, &notes]);
    let before = index.clone();
    let notes_text = crate::format::to_string(&notes).unwrap();

    // The warning to show first: what would break.
    let warn = index.would_break(&shop, &[apple]);
    let mut sources: Vec<_> = warn.iter().map(|l| (l.doc, l.target)).collect();
    sources.sort();
    let mut expected = vec![(shop.id(), apple), (notes.id(), apple), (notes.id(), green)];
    expected.sort();
    assert_eq!(sources, expected);
    // A link from inside what is deleted is not a link that breaks.
    assert_eq!(index.would_break(&shop, &[shop.root()]).len(), 2);
    assert_eq!(index.would_break(&shop, &[pear]), []);
    assert_eq!(index.would_break_document(shop.id()).len(), 2);
    assert_eq!(index.would_break_document(notes.id()), []);

    let mut editor = Editor::new(shop).unwrap();
    let delete = Command::DeleteRows {
        cell: editor.document().root(),
        at: 1,
        count: 1,
    };
    let changes = editor.apply(delete, None, &mut ids).unwrap();
    index.apply(editor.document(), &changes);
    assert_eq!(index, rebuilt(&[editor.document(), &notes]));
    // The links are still in the texts, in this and in the other document, untouched.
    assert_eq!(crate::format::to_string(&notes).unwrap(), notes_text);
    assert_eq!(editor.document().cell(pear).unwrap().links.len(), 1);
    let gone = Resolved::Broken(Broken::NoCell(editor.document().id()));
    let broken = index.broken();
    assert_eq!(broken.len(), 3);
    assert!(broken.iter().all(|link| link.resolved == gone));
    assert_eq!(index.backlinks(editor.document().id(), apple), []);

    // Undo brings the cells back with their IDs, and the links resolve again.
    let changes = editor.undo().unwrap().unwrap();
    index.apply(editor.document(), &changes);
    assert_eq!(index, before);
    assert_eq!(index.broken(), []);
    assert_eq!(index.backlinks(editor.document().id(), apple).len(), 2);

    // Deleting a document: its cells are gone for the others.
    let shop_id = editor.document().id();
    assert!(index.remove(shop_id));
    assert!(!index.remove(shop_id));
    assert_eq!(index, rebuilt(&[&notes]));
    assert_eq!(
        index.outgoing(notes.id())[0].resolved,
        Resolved::Broken(Broken::NoDocument(shop_id))
    );
}

#[test]
fn an_ambiguous_target_is_reported_and_not_guessed() {
    let (shop, notes, _) = pair();
    let apple = at(&shop, &[(0, 1)]);
    let mut index = rebuilt(&[&shop, &notes]);
    // Two files have the ID of `shop`: links into it from elsewhere say so.
    index.mark_ambiguous(shop.id());
    let ambiguous = Resolved::Broken(Broken::AmbiguousDocument(shop.id()));
    assert_eq!(
        index.resolve(notes.id(), &to_cell(Some(shop.id()), apple)),
        ambiguous
    );
    assert_eq!(index.backlinks(shop.id(), apple).len(), 1);
    // The document's own links stay within it.
    assert_eq!(
        index.resolve(shop.id(), &to_cell(None, apple)),
        found(shop.id(), apple)
    );
    assert_eq!(index.broken().len(), 2);

    // A cell that is not where the link says and is in two other documents.
    let (mut a, _) = build(1, &shop_spec());
    let (b, _) = build(1, &shop_spec());
    // Same seed: the same cell IDs. Give one of them another document ID.
    let (copy, fresh) = a.with_fresh_ids(&mut SeededIds::from_u64(50)).unwrap();
    let _ = fresh;
    a.id = copy.id();
    let index = rebuilt(&[&a, &b, &notes]);
    let cell = at(&a, &[(0, 0)]);
    assert_eq!(index.owners(cell).len(), 2);
    let mut owners = vec![a.id(), b.id()];
    owners.sort();
    assert_eq!(
        index.resolve(notes.id(), &to_cell(None, cell)),
        Resolved::Broken(Broken::AmbiguousCell(owners))
    );
    // Named exactly, each is found.
    assert_eq!(
        index.resolve(notes.id(), &to_cell(Some(a.id()), cell)),
        found(a.id(), cell)
    );
    assert_eq!(
        index.resolve(b.id(), &to_cell(None, cell)),
        found(b.id(), cell)
    );
}

/// A document with a link to `target` from its root cell and one from another document;
/// carries out `command`, and checks that both links resolve to the very cell if it is
/// still there and are broken if not, that the index followed, and that undo restores
/// everything. `alive` is what decision A41 says about the target, `None` if it says
/// nothing.
fn survives(
    spec: &N,
    target: &[(u32, u32)],
    alive: Option<bool>,
    command: impl Fn(&Document) -> Command,
) {
    let (mut doc, mut ids) = build(7, spec);
    let target = at(&doc, target);
    let root = doc.root();
    doc.cell_mut(root).unwrap().links = vec![link(0, 1, None, target)];
    let (mut other, _) = build(8, &g("other", 1, vec![n("link")]));
    let from = at(&other, &[(0, 0)]);
    other.cell_mut(from).unwrap().links = vec![link(0, 4, Some(doc.id()), target)];
    let id = doc.id();
    let text = doc.cell(target).unwrap().text.clone();
    let command = command(&doc);
    let what = format!("{command:?}");

    let mut index = rebuilt(&[&doc, &other]);
    let before = index.clone();
    assert_eq!(index.backlinks(id, target).len(), 2, "{what}");
    let mut editor = Editor::new(doc).unwrap();
    let changes = editor.apply(command, Some(5), &mut ids).unwrap();
    assert!(!changes.is_empty(), "nothing happened: {what}");
    index.apply(editor.document(), &changes);
    assert_eq!(index, rebuilt(&[editor.document(), &other]), "{what}");
    let there = editor.document().cell(target).is_some();
    if let Some(alive) = alive {
        assert_eq!(there, alive, "{what}");
    }
    let own = index.resolve(id, &to_cell(None, target));
    let across = index.resolve(other.id(), &to_cell(Some(id), target));
    if there {
        assert_eq!(own, found(id, target), "{what}");
        assert_eq!(across, found(id, target), "{what}");
        assert_eq!(index.backlinks(id, target).len(), 2, "{what}");
        assert!(place(editor.document(), target).is_some());
    } else {
        let gone = Resolved::Broken(Broken::NoCell(id));
        assert_eq!((own, across), (gone.clone(), gone), "{what}");
        assert_eq!(index.backlinks(id, target), [], "{what}");
        assert_eq!(index.broken().len(), 2, "{what}");
    }
    let changes = editor.undo().unwrap().unwrap();
    index.apply(editor.document(), &changes);
    assert_eq!(index, before, "{what}");
    assert_eq!(index.backlinks(id, target).len(), 2, "{what}");
    assert_eq!(editor.document().cell(target).unwrap().text, text);
}

fn table() -> N {
    g("T", 2, vec![n("b"), n("2"), n("c"), n("d"), n("a"), n("")])
}

fn tree() -> N {
    g(
        "T",
        1,
        vec![
            g("P", 1, vec![n("x"), n("y")]),
            g("Q", 1, vec![n("x"), n("z")]),
        ],
    )
}

fn repeats() -> N {
    g("T", 2, vec![n("A"), n("1"), n("A"), n("2"), n("B"), n("3")])
}

#[test]
fn a_link_survives_renaming_moving_and_restructuring_its_target() {
    const D: &[(u32, u32)] = &[(1, 1)];
    let root = |doc: &Document| doc.root();
    // Renaming.
    survives(&table(), D, Some(true), |doc| Command::SetText {
        cell: at(doc, D),
        text: "renamed".into(),
    });
    survives(&table(), D, Some(true), |doc| Command::InsertText {
        cell: at(doc, D),
        at: 1,
        text: "more".into(),
    });
    // Moving.
    survives(&table(), D, Some(true), |doc| Command::MoveCell {
        from: at(doc, D),
        to: at(doc, &[(1, 2)]),
    });
    survives(&table(), D, Some(true), |doc| Command::SwapCells {
        a: at(doc, D),
        b: at(doc, &[(0, 0)]),
    });
    survives(&table(), D, Some(true), |doc| Command::MoveRows {
        cell: root(doc),
        from: 1,
        count: 1,
        to: 0,
    });
    survives(&table(), D, Some(true), |doc| Command::MoveColumns {
        cell: root(doc),
        from: 1,
        count: 1,
        to: 0,
    });
    survives(&tree(), &[(0, 0), (0, 0)], Some(true), |doc| {
        Command::SwapCells {
            a: at(doc, &[(0, 0), (0, 0)]),
            b: at(doc, &[(0, 1), (0, 1)]),
        }
    });
    // Rows and columns around it.
    survives(&table(), D, Some(true), |doc| Command::InsertRows {
        cell: root(doc),
        at: 0,
        count: 2,
    });
    survives(&table(), D, Some(true), |doc| Command::InsertColumns {
        cell: root(doc),
        at: 1,
        count: 1,
    });
    survives(&table(), D, Some(true), |doc| Command::DeleteRows {
        cell: root(doc),
        at: 0,
        count: 1,
    });
    // Restructuring (decision A41).
    for descending in [false, true] {
        survives(&table(), D, Some(true), |doc| Command::SortRows {
            cell: root(doc),
            column: 0,
            row: 0,
            count: 3,
            descending,
        });
    }
    survives(&table(), D, Some(true), |doc| Command::Transpose {
        cells: vec![root(doc)],
    });
    survives(&table(), D, Some(true), |doc| Command::Wrap {
        cell: root(doc),
        x: 1,
        y: 1,
        xs: 1,
        ys: 1,
    });
    survives(&table(), D, Some(true), |doc| Command::Wrap {
        cell: root(doc),
        x: 0,
        y: 0,
        xs: 2,
        ys: 3,
    });
    survives(&table(), D, Some(true), |doc| Command::SplitCell {
        cell: at(doc, D),
        at: 1,
    });
    // Collapse: the first cell of the block keeps its ID.
    survives(&table(), &[(0, 1)], Some(true), |doc| Command::Collapse {
        cell: root(doc),
        x: 0,
        y: 1,
        xs: 2,
        ys: 1,
    });
    survives(&table(), D, None, |doc| Command::Collapse {
        cell: root(doc),
        x: 0,
        y: 1,
        xs: 2,
        ys: 1,
    });
    // Hierarchify: the first cell of a repeated row goes, the earlier one and the rest stay.
    let hierarchify = |doc: &Document| Command::Hierarchify { cell: root(doc) };
    survives(&repeats(), &[(0, 0)], Some(true), hierarchify);
    survives(&repeats(), &[(1, 1)], Some(true), hierarchify);
    survives(&repeats(), &[(0, 1)], Some(false), hierarchify);
    // Flatten: a parent and a leaf both keep their IDs.
    let flatten = |doc: &Document| Command::Flatten { cell: root(doc) };
    survives(&tree(), &[(0, 0)], Some(true), flatten);
    survives(&tree(), &[(0, 1), (0, 1)], Some(true), flatten);
    // Hierarchy swap: the swapped cell keeps its ID; of two that merge, the first stays.
    let swap = |doc: &Document| Command::HierarchySwap {
        cell: at(doc, &[(0, 0), (0, 0)]),
    };
    survives(&tree(), &[(0, 0), (0, 0)], Some(true), swap);
    survives(&tree(), &[(0, 1), (0, 0)], Some(false), swap);
    survives(&tree(), &[(0, 0), (0, 1)], Some(true), swap);
    survives(&tree(), &[(0, 0)], None, swap);
    // Emptying keeps the cell; pasting over it keeps it too.
    survives(&table(), D, Some(true), |doc| Command::ClearCells {
        cells: vec![at(doc, D)],
    });
    survives(&table(), D, Some(true), |doc| Command::DeleteCells {
        cell: root(doc),
        x: 1,
        y: 1,
        xs: 1,
        ys: 1,
    });
    survives(&table(), D, Some(true), |doc| Command::CopyCell {
        from: at(doc, &[(0, 0)]),
        to: at(doc, D),
    });
}

#[test]
fn a_link_is_broken_when_its_target_is_deleted_by_any_command() {
    const D: &[(u32, u32)] = &[(1, 1)];
    const X: &[(u32, u32)] = &[(0, 0), (0, 0)];
    survives(&table(), D, Some(false), |doc| Command::DeleteRows {
        cell: doc.root(),
        at: 1,
        count: 1,
    });
    survives(&table(), D, Some(false), |doc| Command::DeleteColumns {
        cell: doc.root(),
        at: 1,
        count: 1,
    });
    survives(&table(), D, Some(false), |doc| Command::DeleteCells {
        cell: doc.root(),
        x: 0,
        y: 1,
        xs: 2,
        ys: 1,
    });
    survives(&tree(), X, Some(false), |doc| Command::RemoveGrid {
        cell: at(doc, &[(0, 0)]),
    });
    survives(&tree(), X, Some(false), |doc| Command::ClearCells {
        cells: vec![at(doc, &[(0, 0)])],
    });
    survives(&tree(), X, Some(false), |doc| Command::CopyCell {
        from: at(doc, &[(0, 1)]),
        to: at(doc, &[(0, 0)]),
    });
}

#[test]
fn the_summary_of_a_document_reads_back_from_its_cache_form() {
    let (shop, mut notes, _) = pair();
    let first = at(&notes, &[(0, 0)]);
    let cell = notes.cell_mut(first).unwrap();
    cell.links.insert(
        0,
        Link {
            start: 0,
            len: 3,
            target: LinkTarget::External("https://example.org/\"x\"".into()),
            extra: Extra::new(),
        },
    );
    let mut extra = Extra::new();
    extra.insert(CTS_TARGET.into(), "gone".into());
    cell.links.push(Link {
        start: 14,
        len: 2,
        target: LinkTarget::Unknown,
        extra,
    });
    cell.links.push(Link {
        start: 16,
        len: 2,
        target: LinkTarget::Unknown,
        extra: Extra::new(),
    });
    notes.validate().unwrap();
    for doc in [&shop, &notes] {
        let summary = DocLinks::of(doc);
        let value = summary.encode();
        assert_eq!(DocLinks::decode(&value), Some(summary.clone()));
        assert_eq!(value, DocLinks::of(&doc.clone()).encode());
        assert_eq!(summary.cell_count(), doc.cell_count());
        // Through text, as the cache file holds it.
        let text = serde_json::to_string(&value).unwrap();
        let back: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(DocLinks::decode(&back), Some(summary));
    }
    let hints: Vec<_> = DocLinks::of(&notes)
        .links_of(first)
        .iter()
        .map(|e| e.hint.clone())
        .collect();
    assert_eq!(hints, [None, None, Some("gone".to_string()), None]);

    // Anything else is not a summary.
    let good = DocLinks::of(&notes).encode();
    let spoil = |change: &dyn Fn(&mut Value)| {
        let mut value = good.clone();
        change(&mut value);
        DocLinks::decode(&value)
    };
    assert_eq!(DocLinks::decode(&Value::Null), None);
    assert_eq!(spoil(&|v| v["id"] = "12".into()), None);
    assert_eq!(spoil(&|v| v["extra"] = 1.into()), None);
    assert_eq!(spoil(&|v| v["cells"] = "abc".into()), None);
    assert_eq!(spoil(&|v| v["cells"] = "G".repeat(32).into()), None);
    assert_eq!(spoil(&|v| v["links"][0][0] = "0".repeat(32).into()), None);
    assert_eq!(spoil(&|v| v["links"][0][1] = (-1).into()), None);
    assert_eq!(spoil(&|v| v["links"][0][4] = "x".into()), None);
    assert_eq!(spoil(&|v| v["links"][2][1] = 0.into()), None);
    assert_eq!(
        spoil(&|v| v["links"][0].as_array_mut().unwrap().push(1.into())),
        None
    );
    assert_eq!(
        spoil(&|v| {
            let cells = v["cells"].as_str().unwrap().to_string();
            v["cells"] = format!("{cells}{}", &cells[..32]).into();
        }),
        None
    );
}

#[test]
fn copying_between_documents_keeps_every_link_pointing_at_the_same_cell() {
    let (mut shop, notes, mut ids) = pair();
    let apple = at(&shop, &[(0, 1)]);
    let (red, green) = (at(&shop, &[(0, 1), (0, 0)]), at(&shop, &[(0, 1), (0, 1)]));
    let fruit = at(&shop, &[(0, 0)]);
    let note = at(&notes, &[(0, 0)]);
    // In the subtree to copy: a link inside it, one to the rest of its document, one to
    // the document it will be pasted into, one to a third document, one external.
    let third = DocId(Id::from_u128(3));
    let elsewhere = CellId(Id::from_u128(4));
    shop.cell_mut(red).unwrap().text = "abcdefgh".into();
    shop.cell_mut(red).unwrap().links = vec![
        link(0, 1, None, green),
        link(1, 1, None, fruit),
        link(2, 1, Some(notes.id()), note),
        link(3, 1, Some(third), elsewhere),
        link(4, 1, None, apple),
        Link {
            start: 5,
            len: 1,
            target: LinkTarget::External("x".into()),
            extra: Extra::new(),
        },
    ];
    shop.validate().unwrap();
    let to = at(&notes, &[(0, 2)]);
    let mut index = rebuilt(&[&shop, &notes]);
    let mut editor = Editor::new(notes).unwrap();
    let before = editor.document().clone();
    let changes = copy_between(&shop, apple, &mut editor, to, &mut ids).unwrap();
    index.apply(editor.document(), &changes);
    let notes = editor.document();
    notes.validate().unwrap();
    assert_eq!(index, rebuilt(&[&shop, notes]));
    assert_eq!(changes.changed, [to]);
    assert_eq!(changes.created.len(), 2);
    // The pasted cell kept its own ID and place, and has copies of the two cells inside.
    assert_eq!(notes.cell(to).unwrap().text, "Apple");
    let (new_red, new_green) = (at(notes, &[(0, 2), (0, 0)]), at(notes, &[(0, 2), (0, 1)]));
    assert!(shop.cell(new_red).is_none() && shop.cell(new_green).is_none());
    let targets: Vec<_> = notes
        .cell(new_red)
        .unwrap()
        .links
        .iter()
        .map(|l| l.target.clone())
        .collect();
    assert_eq!(
        targets,
        [
            to_cell(None, new_green),
            to_cell(Some(shop.id()), fruit),
            to_cell(None, note),
            to_cell(Some(third), elsewhere),
            to_cell(None, to),
            LinkTarget::External("x".into()),
        ]
    );
    // The original cells have a new backlink each, from the copy.
    assert_eq!(index.backlinks(shop.id(), fruit).len(), 2);
    assert_eq!(index.backlinks(notes.id(), note).len(), 2);
    assert_eq!(index.backlinks(notes.id(), new_green).len(), 1);
    // One unit of undo, which gives the document back exactly.
    let changes = editor.undo().unwrap().unwrap();
    index.apply(editor.document(), &changes);
    assert_eq!(*editor.document(), before);
    assert_eq!(index, rebuilt(&[&shop, &before]));

    // Pasting within the same document is `CopyCell`.
    let mut one = Editor::new(shop.clone()).unwrap();
    let mut two = Editor::new(shop.clone()).unwrap();
    let pear = at(&shop, &[(0, 2)]);
    let clip = Clip::of(&shop, apple).unwrap();
    assert_eq!((clip.source(), clip.top()), (shop.id(), Some(apple)));
    assert_eq!(clip.cell_count(), 3);
    let (mut a, mut b) = (SeededIds::from_u64(9), SeededIds::from_u64(9));
    let paste = Command::PasteCells { to: pear, clip };
    one.apply(paste, None, &mut a).unwrap();
    let copy = Command::CopyCell {
        from: apple,
        to: pear,
    };
    two.apply(copy, None, &mut b).unwrap();
    assert_eq!(one.document(), two.document());
}

#[test]
fn pasting_is_refused_where_copying_is() {
    let (shop, notes, mut ids) = pair();
    let apple = at(&shop, &[(0, 1)]);
    let mut editor = Editor::new(notes).unwrap();
    let before = editor.document().clone();
    let root = before.root();
    let refused = |result: Result<Changes, Error>| matches!(result, Err(Error::Refused(_)));
    assert!(matches!(Clip::of(&shop, root), Err(Error::NotFound(_))));
    let clip = Clip::of(&shop, apple).unwrap();
    let paste = |to| Command::PasteCells {
        to,
        clip: clip.clone(),
    };
    assert!(refused(editor.apply(paste(root), None, &mut ids)));
    assert!(matches!(
        editor.apply(paste(apple), None, &mut ids),
        Err(Error::NotFound(_))
    ));
    // Too deep: a chain of grids that just fits, and a clip with one level more.
    let mut deep = Editor::new(shop.clone()).unwrap();
    let mut cell = at(&shop, &[(0, 2)]);
    let levels = crate::limits::MAX_DEPTH - 1;
    for _ in 0..levels {
        let grid = Command::CreateGrid { cell, xs: 1, ys: 1 };
        deep.apply(grid, None, &mut ids).unwrap();
        cell = deep.document().child(cell, 0, 0).unwrap();
    }
    assert!(matches!(
        deep.apply(paste(cell), None, &mut ids),
        Err(Error::Limit(_))
    ));
    assert_eq!(*editor.document(), before);
    assert_eq!(editor.undo_len(), 0);
}

/// `shop`, `notes` and a third document that links into `shop`.
fn three() -> (Document, Document, Document, SeededIds) {
    let (shop, mut notes, ids) = pair();
    let apple = at(&shop, &[(0, 1)]);
    let (red, green) = (at(&shop, &[(0, 1), (0, 0)]), at(&shop, &[(0, 1), (0, 1)]));
    let mut shop = shop;
    // The cells that will move link to each other, to a cell that stays, and to `notes`.
    let fruit = at(&shop, &[(0, 0)]);
    let note = at(&notes, &[(0, 0)]);
    shop.cell_mut(red).unwrap().links = vec![
        link(0, 1, None, green),
        link(1, 1, None, fruit),
        link(2, 1, Some(notes.id()), note),
    ];
    // `notes` has a link without a document to a cell it does not have: left alone.
    let second = at(&notes, &[(0, 1)]);
    notes
        .cell_mut(second)
        .unwrap()
        .links
        .insert(0, link(0, 1, None, apple));
    let (mut third, _) = build(3, &g("Third", 1, vec![n("apple and green")]));
    let cell = at(&third, &[(0, 0)]);
    third.cell_mut(cell).unwrap().links = vec![
        link(0, 5, Some(shop.id()), apple),
        link(10, 5, Some(shop.id()), green),
    ];
    for doc in [&shop, &notes, &third] {
        doc.validate().unwrap();
    }
    (shop, notes, third, ids)
}

#[test]
fn moving_between_documents_keeps_the_ids_so_that_links_survive() {
    let (shop, notes, third, mut ids) = three();
    let apple = at(&shop, &[(0, 1)]);
    let (red, green) = (at(&shop, &[(0, 1), (0, 0)]), at(&shop, &[(0, 1), (0, 1)]));
    let (fruit, pear) = (at(&shop, &[(0, 0)]), at(&shop, &[(0, 2)]));
    let (note, second, to) = (
        at(&notes, &[(0, 0)]),
        at(&notes, &[(0, 1)]),
        at(&notes, &[(0, 2)]),
    );
    let (shop_id, notes_id) = (shop.id(), notes.id());
    let mut index = rebuilt(&[&shop, &notes, &third]);
    let start = index.clone();
    let (shop_before, notes_before) = (shop.clone(), notes.clone());
    let mut source = Editor::new(shop).unwrap();
    let mut target = Editor::new(notes).unwrap();

    let moved = move_between(&mut source, apple, &mut target, to, None, &mut ids).unwrap();
    index.apply(source.document(), &moved.source);
    index.apply(target.document(), &moved.target);
    let (shop, notes) = (source.document(), target.document());
    shop.validate().unwrap();
    notes.validate().unwrap();
    assert_eq!(index, rebuilt(&[shop, notes, &third]));
    assert_eq!((source.undo_len(), target.undo_len()), (1, 1));

    // The cells are in `notes` with their IDs, texts and grid, where the empty cell was.
    assert_eq!(moved.target.created, {
        let mut ids = vec![apple, red, green];
        ids.sort();
        ids
    });
    assert_eq!(moved.target.removed, [to]);
    assert_eq!(notes.child(notes.root(), 0, 2), Some(apple));
    assert_eq!(place(notes, green).unwrap().trail, ["Notes", "Apple"]);
    // In `shop` they are gone and a new empty cell stands there.
    assert!([apple, red, green].iter().all(|c| shop.cell(*c).is_none()));
    let left = shop.child(shop.root(), 0, 1).unwrap();
    assert_eq!(moved.source.created, [left]);
    assert_eq!(shop.cell(left).unwrap().text, "");
    assert_eq!(shop.cell_count(), shop_before.cell_count() - 2);

    // Links of the moved cells: to each other as before, to a cell that stayed across
    // documents now, to a cell of the document they arrived in without a document.
    let targets: Vec<_> = notes
        .cell(red)
        .unwrap()
        .links
        .iter()
        .map(|l| l.target.clone())
        .collect();
    assert_eq!(
        targets,
        [
            to_cell(None, green),
            to_cell(Some(shop_id), fruit),
            to_cell(None, note)
        ]
    );
    // Links to the moved cells: from `shop` they now name `notes`, in `notes` they name
    // no document, and the one in `notes` that named none already is as it was.
    assert_eq!(
        shop.cell(pear).unwrap().links[0].target,
        to_cell(Some(notes_id), apple)
    );
    assert_eq!(
        notes.cell(note).unwrap().links[0].target,
        to_cell(None, apple)
    );
    let seconds: Vec<_> = notes
        .cell(second)
        .unwrap()
        .links
        .iter()
        .map(|l| l.target.clone())
        .collect();
    assert_eq!(seconds, [to_cell(None, apple), to_cell(None, green)]);
    // Nothing is broken anywhere; every backlink of before is a backlink now.
    assert_eq!(index.broken(), []);
    assert_eq!(index.backlinks(notes_id, apple).len(), 4);
    assert_eq!(
        index.backlinks(notes_id, green).len(),
        start.backlinks(shop_id, green).len()
    );
    assert_eq!(index.backlinks(shop_id, apple), []);
    // The third document was not touched: its links still name `shop`, and resolve to
    // where the cells are now, marked as moved, until they are corrected.
    let from_third = index.outgoing(third.id());
    assert!(from_third.iter().all(|link| matches!(
        link.resolved,
        Resolved::Cell { doc, moved: true, .. } if doc == notes_id
    )));
    assert_eq!(index.repairs(shop), []);
    let repairs = index.repairs(&third);
    assert_eq!(repairs.len(), 2);
    let mut other = Editor::new(third.clone()).unwrap();
    let mut with_third = index.clone();
    for command in repairs {
        let changes = other.apply(command, None, &mut ids).unwrap();
        with_third.apply(other.document(), &changes);
    }
    assert_eq!(with_third, rebuilt(&[shop, notes, other.document()]));
    assert!(with_third
        .outgoing(third.id())
        .iter()
        .all(|link| link.resolved == found(notes_id, link_cell(&link.link))));

    // Undone in both documents, both are exactly as they were.
    let changes = target.undo().unwrap().unwrap();
    index.apply(target.document(), &changes);
    // In between the cells are in neither document, and the links say so.
    assert_eq!(index.broken().len(), 6);
    let changes = source.undo().unwrap().unwrap();
    index.apply(source.document(), &changes);
    assert_eq!(*source.document(), shop_before);
    assert_eq!(*target.document(), notes_before);
    assert_eq!(index, start);
    // Redone in the source only, and then in the target: moved again.
    let changes = source.redo().unwrap().unwrap();
    index.apply(source.document(), &changes);
    let changes = target.redo().unwrap().unwrap();
    index.apply(target.document(), &changes);
    assert_eq!(index.broken(), []);
    assert_eq!(index.backlinks(notes_id, apple).len(), 4);
    // Undone in the source only: the cells are in both documents. Each document finds
    // its own, and nothing is guessed for a link that names neither.
    let changes = source.undo().unwrap().unwrap();
    index.apply(source.document(), &changes);
    assert_eq!(index.owners(apple).len(), 2);
    assert_eq!(
        index,
        rebuilt(&[source.document(), target.document(), &third])
    );
    assert_eq!(
        index.resolve(third.id(), &to_cell(Some(shop_id), apple)),
        found(shop_id, apple)
    );
    let stranger = DocId(Id::from_u128(99));
    assert!(matches!(
        index.resolve(stranger, &to_cell(None, apple)),
        Resolved::Broken(Broken::AmbiguousCell(_))
    ));
}

fn link_cell(entry: &LinkEntry) -> CellId {
    match entry.target {
        LinkTarget::Cell { cell, .. } => cell,
        _ => panic!("not a link to a cell"),
    }
}

#[test]
fn a_move_between_documents_that_cannot_be_done_leaves_both_as_they_were() {
    let (shop, notes, _, mut ids) = three();
    let apple = at(&shop, &[(0, 1)]);
    let (note, to) = (at(&notes, &[(0, 0)]), at(&notes, &[(0, 2)]));
    let (shop_before, notes_before) = (shop.clone(), notes.clone());
    let mut source = Editor::new(shop).unwrap();
    let mut target = Editor::new(notes).unwrap();
    let same = |source: &Editor, target: &Editor| {
        assert_eq!(*source.document(), shop_before);
        assert_eq!(*target.document(), notes_before);
        assert_eq!((source.undo_len(), source.redo_len()), (0, 0));
        assert_eq!((target.undo_len(), target.redo_len()), (0, 0));
    };
    let refused = |result: Result<Moved, Error>| {
        assert!(matches!(result, Err(Error::Refused(_))), "{result:?}");
    };
    // Onto a cell that is not empty, onto the root, of the root, of a missing cell.
    refused(move_between(
        &mut source,
        apple,
        &mut target,
        note,
        None,
        &mut ids,
    ));
    same(&source, &target);
    let root = notes_before.root();
    refused(move_between(
        &mut source,
        apple,
        &mut target,
        root,
        None,
        &mut ids,
    ));
    let root = shop_before.root();
    refused(move_between(
        &mut source,
        root,
        &mut target,
        to,
        None,
        &mut ids,
    ));
    assert!(matches!(
        move_between(&mut source, to, &mut target, to, None, &mut ids),
        Err(Error::NotFound(_))
    ));
    same(&source, &target);
    // While a group is open.
    target.begin_group();
    refused(move_between(
        &mut source,
        apple,
        &mut target,
        to,
        None,
        &mut ids,
    ));
    target.end_group();
    same(&source, &target);
    // Within one document.
    let mut twin = Editor::new(shop_before.clone()).unwrap();
    let pear = at(&shop_before, &[(1, 2)]);
    refused(move_between(
        &mut source,
        apple,
        &mut twin,
        pear,
        None,
        &mut ids,
    ));
    // The second half fails (the source of IDs gives one that is taken, for the empty
    // cell that is left behind): the first half is taken back.
    struct Taken(Id);
    impl crate::id::IdSource for Taken {
        fn next_id(&mut self) -> Id {
            self.0
        }
    }
    let mut taken = Taken(shop_before.root().0);
    refused(move_between(
        &mut source,
        apple,
        &mut target,
        to,
        None,
        &mut taken,
    ));
    same(&source, &target);

    // An ID that the target has, or that its history can bring back, is refused.
    let moved = move_between(&mut source, apple, &mut target, to, None, &mut ids).unwrap();
    assert!(!moved.source.is_empty());
    source.undo().unwrap().unwrap();
    // Now both have the cells. Moving them again cannot be done:
    let empty = at(target.document(), &[(0, 1)]);
    let clear = Command::ClearCells { cells: vec![empty] };
    target.apply(clear, None, &mut ids).unwrap();
    refused(move_between(
        &mut source,
        apple,
        &mut target,
        empty,
        None,
        &mut ids,
    ));
    // ... nor after they were deleted from the target, while its undo can bring them back.
    let delete = Command::ClearCells { cells: vec![apple] };
    let mut probe = target.clone();
    let grid = Command::CreateGrid {
        cell: apple,
        xs: 1,
        ys: 1,
    };
    assert!(probe.apply(grid, None, &mut ids).is_err());
    target.apply(delete, None, &mut ids).unwrap();
    let red = at(&shop_before, &[(0, 1), (0, 0)]);
    assert!(target.document().cell(red).is_none());
    let clip = Clip::of(source.document(), red).unwrap();
    let adopt = Command::AdoptCells { to: empty, clip };
    assert!(matches!(
        target.apply(adopt, None, &mut ids),
        Err(Error::Refused(_))
    ));
    target.document().validate().unwrap();
}

#[test]
fn releasing_a_cell_takes_its_id_out_of_the_document() {
    let (shop, _, mut ids) = pair();
    let apple = at(&shop, &[(0, 1)]);
    let mut editor = Editor::new(shop.clone()).unwrap();
    let root = shop.root();
    assert!(editor
        .apply(Command::ReleaseCell { cell: root }, None, &mut ids)
        .is_err());
    let color = crate::model::Color::rgb(1, 2, 3);
    let paint = Command::SetProps {
        cells: vec![apple],
        props: vec![crate::edit::Prop::CellColor(color)],
    };
    editor.apply(paint, None, &mut ids).unwrap();
    let release = Command::ReleaseCell { cell: apple };
    let changes = editor.apply(release, Some(9), &mut ids).unwrap();
    let doc = editor.document();
    doc.validate().unwrap();
    assert_eq!(changes.removed.len(), 3);
    assert_eq!(changes.grids, [root]);
    let new = doc.child(root, 0, 1).unwrap();
    assert_ne!(new, apple);
    let cell = doc.cell(new).unwrap();
    // Empty, with the looks of the cell that left.
    assert_eq!((cell.text.as_str(), cell.cell_color), ("", color));
    assert!(cell.grid().is_none());
    editor.undo().unwrap().unwrap();
    editor.undo().unwrap().unwrap();
    assert_eq!(*editor.document(), shop);
}

#[test]
fn the_link_to_follow_is_the_one_at_the_cursor_or_the_first() {
    let (shop, notes, _) = pair();
    let first = at(&notes, &[(0, 0)]);
    // "see the apple here": the link is at 8..13.
    let start = |cursor| link_at(&notes, first, cursor).map(|l| l.start);
    assert_eq!(start(None), Some(8));
    assert_eq!(start(Some(8)), Some(8));
    assert_eq!(start(Some(12)), Some(8));
    // Just after the link, as TreeSheets: the link before the cursor.
    assert_eq!(start(Some(13)), Some(8));
    assert_eq!(start(Some(14)), None);
    assert_eq!(start(Some(0)), None);
    assert_eq!(start(Some(7)), None);
    assert!(link_at(&notes, at(&notes, &[(0, 2)]), None).is_none());
    assert!(link_at(&notes, at(&shop, &[(0, 0)]), None).is_none());
}

#[test]
fn an_external_target_is_a_url_or_a_file_path_by_treesheets_rule() {
    for url in [
        "https://example.org",
        "mailto:a@b.c",
        "x-app+v1.2:thing",
        "ab:",
    ] {
        assert_eq!(external(url), External::Url, "{url}");
    }
    for path in [
        "notes.txt",
        "C:\\files\\a.txt",
        "c:/a",
        "../up/file.cts",
        "/abs/path",
        "1ab:c",
        "a b:c",
        ":x",
        "",
        "folder/with:colon",
    ] {
        assert_eq!(external(path), External::Path, "{path}");
    }
}

#[test]
fn an_imported_link_without_a_target_gets_one_when_exactly_one_cell_has_its_text() {
    let (mut doc, mut ids) = build(
        5,
        &g(
            "r",
            1,
            vec![
                n("to one, to two, to none, to self"),
                n("one"),
                n("two"),
                n("two"),
                n("=x"),
            ],
        ),
    );
    let from = at(&doc, &[(0, 0)]);
    let unknown = |start: u32, len: u32, text: &str| {
        let mut extra = Extra::new();
        extra.insert(CTS_TARGET.into(), text.into());
        extra.insert("x_other".into(), 1.into());
        Link {
            start,
            len,
            target: LinkTarget::Unknown,
            extra,
        }
    };
    doc.cell_mut(from).unwrap().links = vec![
        unknown(3, 3, "one"),
        unknown(11, 3, "two"),
        unknown(19, 4, "nothing"),
        unknown(28, 4, "to one, to two, to none, to self"),
        Link {
            start: 0,
            len: 2,
            target: LinkTarget::Unknown,
            extra: Extra::new(),
        },
    ];
    doc.cell_mut(from).unwrap().links.sort_by_key(|l| l.start);
    // A formula cell is never a target (decision B15), nor is the root.
    let formula = at(&doc, &[(0, 4)]);
    doc.cell_mut(formula).unwrap().kind = CellKind::Formula;
    doc.cell_mut(formula).unwrap().text = "nothing".into();
    doc.validate().unwrap();

    let found = unknown_links(&doc);
    let shape: Vec<_> = found
        .iter()
        .map(|l| (l.start, l.hint.as_str(), l.candidates.len()))
        .collect();
    assert_eq!(
        shape,
        [
            (3, "one", 1),
            (11, "two", 2),
            (19, "nothing", 0),
            (28, "to one, to two, to none, to self", 0)
        ]
    );
    let commands = resolve_unknown(&doc);
    assert_eq!(commands.len(), 1);
    let mut index = rebuilt(&[&doc]);
    assert_eq!(index.broken().len(), 5);
    let mut editor = Editor::new(doc).unwrap();
    for command in commands {
        let changes = editor.apply(command, None, &mut ids).unwrap();
        index.apply(editor.document(), &changes);
    }
    let doc = editor.document();
    let one = at(doc, &[(0, 1)]);
    let link = &doc.cell(from).unwrap().links[1];
    assert_eq!(link.target, to_cell(None, one));
    // The text it kept is dropped with the rest of its unknown fields.
    assert!(link.extra.is_empty());
    assert_eq!(index.broken().len(), 4);
    assert_eq!(index.backlinks(doc.id(), one).len(), 1);
    assert_eq!(unknown_links(doc).len(), 3);
    assert_eq!(resolve_unknown(doc), []);
}
