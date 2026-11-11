//! Unit tests of the restructuring commands of milestone 3b: what each does, which cells
//! keep their IDs, and what is refused. What TreeSheets itself does with the same
//! operations is compared in `wombat-cli/tests/ops.rs`.

use super::*;

/// Gives `cell` a grid of `xs` columns with these texts, row by row; returns its cells.
fn fill(f: &mut Fixture, cell: CellId, xs: u32, texts: &[&str]) -> Vec<CellId> {
    let ys = texts.len() as u32 / xs;
    f.ok(Command::CreateGrid { cell, xs, ys });
    let cells = f.grid(cell).cells().to_vec();
    for (id, text) in cells.iter().zip(texts) {
        f.ok(Command::SetText {
            cell: *id,
            text: (*text).into(),
        });
    }
    cells
}

/// A document whose root has a grid of `xs` columns with these texts.
fn table(xs: u32, texts: &[&str]) -> (Fixture, Vec<CellId>) {
    // (The root starts without a grid: once it has one, no command takes it away, B33.)
    let mut ids = SeededIds::from_u64(7);
    let doc = Document::new(&mut ids);
    let mut f = Fixture {
        e: Editor::new(doc).unwrap(),
        ids,
        cells: Vec::new(),
    };
    let root = f.root();
    let cells = fill(&mut f, root, xs, texts);
    f.e.clear_history();
    (f, cells)
}

/// A cell and all inside it as text: `text[row / row]`, an empty text as `_`.
fn outline(f: &Fixture, id: CellId) -> String {
    let cell = f.cell(id);
    let mut out = if cell.text.is_empty() {
        "_".to_string()
    } else {
        cell.text.clone()
    };
    if let Some(grid) = cell.grid() {
        let rows: Vec<String> = grid
            .cells()
            .chunks(grid.xs() as usize)
            .map(|row| {
                let cells: Vec<String> = row.iter().map(|c| outline(f, *c)).collect();
                cells.join(" ")
            })
            .collect();
        out.push_str(&format!("[{}]", rows.join(" / ")));
    }
    out
}

fn sort(cell: CellId, column: u32, row: u32, count: u32, descending: bool) -> Command {
    Command::SortRows {
        cell,
        column,
        row,
        count,
        descending,
    }
}

#[test]
fn rows_are_sorted_as_treesheets_sorts_them() {
    let texts = [
        "pear", "10", "b", "10", "Apple", "9", "apple", "100", "", "2", "apple", "9",
    ];
    let (mut f, cells) = table(2, &texts);
    let root = f.root();
    // By text, without regard to case; equal first cells are told apart by the next column.
    f.ok(sort(root, 0, 0, 6, false));
    assert_eq!(f.texts(root), " 2/apple 100/Apple 9/apple 9/b 10/pear 10");
    // The same cells, with their IDs, in another order.
    let mut before = cells.clone();
    let mut after = f.grid(root).cells().to_vec();
    before.sort();
    after.sort();
    assert_eq!(before, after);
    assert_eq!(f.cell(cells[4]).text, "Apple");
    // Sorting again changes nothing and leaves no history.
    assert!(f.ok(sort(root, 0, 0, 6, false)).is_empty());
    // Numbers are compared as text, and the columns wrap round: by the second column,
    // then by the first.
    f.ok(sort(root, 1, 0, 6, false));
    assert_eq!(f.texts(root), "b 10/pear 10/apple 100/ 2/Apple 9/apple 9");
    // Descending keeps rows that are equal in every column in their order.
    f.ok(sort(root, 1, 0, 6, true));
    assert_eq!(f.texts(root), "Apple 9/apple 9/ 2/apple 100/pear 10/b 10");
    // Only the rows asked for.
    f.ok(sort(root, 0, 3, 3, false));
    assert_eq!(f.texts(root), "Apple 9/apple 9/ 2/apple 100/b 10/pear 10");
    // One row or none is nothing to sort, not an error.
    assert!(f.ok(sort(root, 0, 5, 1, false)).is_empty());
    assert!(f.ok(sort(root, 0, 6, 0, false)).is_empty());
    for bad in [
        sort(root, 2, 0, 6, false),
        sort(root, 0, 1, 6, false),
        sort(root, 0, u32::MAX, 2, false),
        sort(cells[0], 0, 0, 1, false),
    ] {
        assert!(matches!(f.bad(bad), Error::Refused(_)));
    }
    let missing = CellId(Id::from_u128(9));
    assert!(matches!(
        f.bad(sort(missing, 0, 0, 1, false)),
        Error::NotFound(_)
    ));
}

#[test]
fn sorting_folds_case_beyond_ascii_one_character_at_a_time() {
    let (mut f, _) = table(1, &["éclair", "Zebra", "Éclair", "zebra", "\u{1F600}", "e"]);
    let root = f.root();
    f.ok(sort(root, 0, 0, 6, false));
    assert_eq!(f.texts(root), "e/Zebra/zebra/éclair/Éclair/\u{1F600}");
}

#[test]
fn grids_are_transposed() {
    let (mut f, cells) = table(3, &["a", "b", "c", "d", "e", "f"]);
    let root = f.root();
    f.ok(Command::SetColumnWidth {
        cell: root,
        column: 1,
        width: 40,
    });
    f.ok(Command::SetColumnWidth {
        cell: root,
        column: 2,
        width: 30,
    });
    let changes = f.ok(Command::Transpose { cells: vec![root] });
    assert_eq!(f.texts(root), "a d/b e/c f");
    // The widths stay with their place; the third is gone.
    assert_eq!(f.grid(root).col_widths, [80, 40]);
    assert_eq!((changes.changed, changes.grids), (vec![root], vec![root]));
    // Back: the same cells in the same places; the new third column takes the last width.
    f.ok(Command::Transpose { cells: vec![root] });
    assert_eq!(f.texts(root), "a b c/d e f");
    assert_eq!(f.grid(root).cells(), cells);
    assert_eq!(f.grid(root).col_widths, [80, 40, 40]);
    // Several cells: those with a grid are transposed, each once; the others passed over.
    let inner = fill(&mut f, cells[0], 2, &["x", "y"]);
    f.ok(Command::Transpose {
        cells: vec![cells[0], cells[1], cells[0], inner[0]],
    });
    assert_eq!(outline(&f, cells[0]), "a[x / y]");
    // A grid of one cell is its own transpose.
    fill(&mut f, cells[1], 1, &["only"]);
    assert!(f
        .ok(Command::Transpose {
            cells: vec![cells[1], cells[2]]
        })
        .is_empty());
    let missing = CellId(Id::from_u128(9));
    assert!(matches!(
        f.bad(Command::Transpose {
            cells: vec![root, missing]
        }),
        Error::NotFound(_)
    ));
}

#[test]
fn a_table_becomes_a_hierarchy() {
    let texts = [
        "Fruit", "Apple", "red", "Fruit", "Apple", "green", "Veg", "Leek", "green", "Fruit",
        "Pear", "green", "Veg", "Leek", "white",
    ];
    let (mut f, cells) = table(3, &texts);
    let root = f.root();
    f.ok(Command::SetColumnWidth {
        cell: root,
        column: 2,
        width: 44,
    });
    f.ok(props(&[cells[3]], &[Prop::Note("gone".into())]));
    let changes = f.ok(Command::Hierarchify { cell: root });
    assert_eq!(
        outline(&f, root),
        "_[Fruit[Apple[red / green] / Pear[green]] / Veg[Leek[green / white]]]"
    );
    // The first cell of each group stays; the first cells of repeated rows are removed,
    // down the levels. Every other cell is the cell it was.
    let mut removed = vec![cells[3], cells[9], cells[12], cells[4], cells[13]];
    removed.sort();
    assert_eq!(changes.removed, removed);
    assert!(changes.created.is_empty());
    assert_eq!(f.grid(root).cells(), [cells[0], cells[6]]);
    assert_eq!(f.grid(cells[0]).cells(), [cells[1], cells[10]]);
    assert_eq!(f.grid(cells[1]).cells(), [cells[2], cells[5]]);
    assert_eq!(f.cell(cells[5]).parent(), Some(cells[1]));
    // The new grids have the widths of the columns their cells were in.
    assert_eq!(f.grid(root).col_widths, [80]);
    assert_eq!(f.grid(cells[0]).col_widths, [80]);
    assert_eq!(f.grid(cells[1]).col_widths, [44]);
    // What it made is no table any more.
    let error = f.bad(Command::Hierarchify { cell: root });
    assert!(matches!(error, Error::Refused(_)));
    assert!(matches!(
        f.bad(Command::Hierarchify { cell: cells[2] }),
        Error::Refused(_)
    ));
    // One column: rows with the same text become one. Nothing to merge: nothing changes.
    let (mut f, cells) = table(1, &["a", "b", "a", "", ""]);
    let root = f.root();
    let changes = f.ok(Command::Hierarchify { cell: root });
    assert_eq!(outline(&f, root), "_[a / b / _]");
    assert_eq!(changes.removed.len(), 2);
    assert!(!changes.removed.contains(&cells[0]));
    assert!(f.ok(Command::Hierarchify { cell: root }).is_empty());
}

#[test]
fn hierarchify_respects_the_nesting_limit() {
    let mut f = Fixture::new(1, 1);
    let mut at = f.cells[0];
    for _ in 1..MAX_DEPTH - 2 {
        f.ok(Command::CreateGrid {
            cell: at,
            xs: 1,
            ys: 1,
        });
        at = f.grid(at).cells()[0];
    }
    // Three columns would become three levels: one too many here; two fit.
    fill(&mut f, at, 3, &["a", "b", "c"]);
    assert!(matches!(
        f.bad(Command::Hierarchify { cell: at }),
        Error::Limit(_)
    ));
    f.ok(Command::DeleteColumns {
        cell: at,
        at: 2,
        count: 1,
    });
    f.ok(Command::Hierarchify { cell: at });
    assert_eq!(outline(&f, at), "_[a[b]]");
}

/// Fruit[Apple[red / green] / Pear] / Veg[Apple / Leek] / solo, in the root.
fn tree() -> (Fixture, Vec<CellId>, Vec<CellId>, Vec<CellId>, Vec<CellId>) {
    let (mut f, top) = table(1, &["Fruit", "Veg", "solo"]);
    let fruit = fill(&mut f, top[0], 1, &["Apple", "Pear"]);
    let apple = fill(&mut f, fruit[0], 1, &["red", "green"]);
    let veg = fill(&mut f, top[1], 1, &["Apple", "Leek"]);
    f.e.clear_history();
    (f, top, fruit, apple, veg)
}

#[test]
fn a_hierarchy_becomes_a_table() {
    let (mut f, top, fruit, apple, veg) = tree();
    let root = f.root();
    f.ok(props(
        &[fruit[0]],
        &[Prop::CellColor(RED), Prop::Note("n".into())],
    ));
    f.ok(styled(apple[0], 0, 2, "b"));
    f.ok(props(
        &[apple[0]],
        &[Prop::RelSize(2), Prop::TextColor(RED)],
    ));
    f.ok(Command::SetColumnWidth {
        cell: root,
        column: 0,
        width: 33,
    });
    let before = f.doc().cell_count();
    let changes = f.ok(Command::Flatten { cell: root });
    assert_eq!(
        f.texts(root),
        "Fruit Apple red/Fruit Apple green/Fruit Pear /Veg Apple /Veg Leek /solo  "
    );
    // Every cell is still there: each leaf on its row, each parent on the first row it
    // stands on. Their repeats and the empty cells are new.
    assert!(changes.removed.is_empty());
    assert_eq!(changes.created.len(), 18 - (before - 1));
    let rows: Vec<&[CellId]> = f.grid(root).cells().chunks(3).collect();
    assert_eq!(rows[0], [top[0], fruit[0], apple[0]]);
    assert_eq!(rows[1][2], apple[1]);
    assert_eq!(rows[2][1], fruit[1]);
    assert_eq!(rows[3][..2], [top[1], veg[0]]);
    assert_eq!(rows[4][1], veg[1]);
    assert_eq!(rows[5][0], top[2]);
    for new in [rows[1][0], rows[1][1], rows[2][0], rows[2][2], rows[5][1]] {
        assert!(changes.created.contains(&new));
    }
    // Only the text and what belongs to it is carried over: styles, size; not colours
    // or notes. The table has default widths.
    let red = f.cell(apple[0]);
    assert_eq!(f.runs(apple[0]), [(0, 2, "b".to_string(), None)]);
    assert_eq!((red.rel_size, red.text_color), (2, Color::BLACK));
    let was_red = f.cell(fruit[0]);
    assert_eq!(
        (was_red.cell_color, was_red.note.as_str()),
        (Color::WHITE, "")
    );
    assert_eq!(f.grid(root).col_widths, [80, 80, 80]);
    assert!(f.cell(fruit[0]).grid().is_none());
    // A table flattens to one column, a cell without a grid not at all.
    f.ok(Command::Flatten { cell: root });
    assert_eq!((f.grid(root).xs(), f.grid(root).ys()), (1, 18));
    assert!(f.ok(Command::Flatten { cell: root }).is_empty());
    assert!(matches!(
        f.bad(Command::Flatten { cell: top[0] }),
        Error::Refused(_)
    ));
}

/// Not a rule of TreeSheets, but what follows from the two: a hierarchy whose leaves are
/// all at the same level and whose texts differ under each parent comes back from flatten
/// and hierarchify with the same shape, texts and IDs.
#[test]
fn flatten_then_hierarchify_gives_back_an_even_hierarchy() {
    let (mut f, top) = table(1, &["A", "B"]);
    let a = fill(&mut f, top[0], 1, &["a1", "a2"]);
    let b = fill(&mut f, top[1], 1, &["b1"]);
    let leaves = [
        fill(&mut f, a[0], 1, &["x", "y"]),
        fill(&mut f, a[1], 1, &["z"]),
        fill(&mut f, b[0], 1, &["x"]),
    ];
    let root = f.root();
    let before = outline(&f, root);
    let ids = f.doc().cells_in_order();
    f.ok(Command::Flatten { cell: root });
    assert_eq!(f.grid(root).xs(), 3);
    f.ok(Command::Hierarchify { cell: root });
    assert_eq!(outline(&f, root), before);
    assert_eq!(f.doc().cells_in_order(), ids);
    assert_eq!(f.cell(leaves[2][0]).parent(), Some(b[0]));
    // Leaves at different levels do not come back: the empty cells stay.
    let (mut f, _, _, _, _) = tree();
    let root = f.root();
    f.ok(Command::Flatten { cell: root });
    f.ok(Command::Hierarchify { cell: root });
    assert_eq!(
        outline(&f, root),
        "_[Fruit[Apple[red / green] / Pear[_]] / Veg[Apple[_] / Leek[_]] / solo[_[_]]]"
    );
}

#[test]
fn a_hierarchy_is_swapped() {
    let (mut f, top, fruit, apple, veg) = tree();
    let root = f.root();
    f.ok(props(&[fruit[0]], &[Prop::Note("one".into())]));
    f.ok(props(&[veg[0]], &[Prop::Note("two\n\none".into())]));
    f.e.clear_history();
    let changes = f.ok(Command::HierarchySwap { cell: veg[0] });
    assert_eq!(
        outline(&f, root),
        "_[Fruit[Pear] / Veg[Leek] / solo / Apple[Fruit[red / green] / Veg]]"
    );
    // The first Apple found moved up with its ID and holds what it held, inside a copy of
    // its parent; the second was merged into it and is gone, its note added.
    assert_eq!(f.grid(root).cells()[3], fruit[0]);
    assert_eq!(f.cell(fruit[0]).note, "one\n\ntwo");
    assert_eq!(changes.removed, [veg[0]]);
    let copies = f.grid(fruit[0]).cells().to_vec();
    let mut created = copies.clone();
    created.sort();
    assert_eq!(changes.created, created);
    assert_eq!(f.grid(copies[0]).cells(), apple);
    // The parents stay, with their IDs, as they still hold other cells.
    assert_eq!(f.grid(root).cells()[..2], top[..2]);
    // Swapping back the Veg inside Apple: it moves up and merges into the Veg there,
    // which gets a copy of Apple; the Fruit inside Apple is another text and stays.
    f.ok(Command::HierarchySwap { cell: copies[1] });
    assert_eq!(
        outline(&f, root),
        "_[Fruit[Pear] / Veg[Leek / Apple] / solo / Apple[Fruit[red / green]]]"
    );
    assert_eq!(f.grid(root).cells()[..3], top[..]);
    assert_eq!(f.cell(apple[0]).text, "red");
}

#[test]
fn a_parent_left_empty_by_a_swap_moves_with_its_id() {
    // P[Q[T[t1 / t2]]] / R[T / S]
    let (mut f, top) = table(1, &["P", "R"]);
    let q = fill(&mut f, top[0], 1, &["Q"]);
    let t = fill(&mut f, q[0], 1, &["T"]);
    let leaves = fill(&mut f, t[0], 1, &["t1", "t2"]);
    let r = fill(&mut f, top[1], 1, &["T", "S"]);
    f.ok(props(&[q[0]], &[Prop::CellColor(RED)]));
    f.e.clear_history();
    let root = f.root();
    let changes = f.ok(Command::HierarchySwap { cell: r[0] });
    assert_eq!(outline(&f, root), "_[R[S] / T[P[Q[t1 / t2]] / R]]");
    // P and Q held nothing else, so they are gone from where they were, and their copies
    // inside T are they: same IDs, same looks. R stays, so its copy is a new cell.
    assert_eq!(f.grid(root).cells(), [top[1], t[0]]);
    assert_eq!(f.grid(t[0]).cells()[0], top[0]);
    assert_eq!(f.grid(top[0]).cells(), q);
    assert_eq!(f.cell(q[0]).cell_color, RED);
    assert_eq!(f.grid(q[0]).cells(), leaves);
    assert_eq!(changes.created, [f.grid(t[0]).cells()[1]]);
    assert_eq!(changes.removed, [r[0]]);
    // One level only: the grid above has one cell, which the swapped cell replaces.
    let (mut f, top) = table(1, &["P"]);
    let q = fill(&mut f, top[0], 1, &["Q"]);
    let t = fill(&mut f, q[0], 1, &["T"]);
    let changes = f.ok(Command::HierarchySwap { cell: t[0] });
    assert_eq!(outline(&f, top[0]), "P[T[Q]]");
    assert_eq!(f.grid(t[0]).cells(), q);
    assert!(changes.created.is_empty() && changes.removed.is_empty());
    // A parent with the same text is swapped once, not forever.
    f.ok(Command::SetText {
        cell: q[0],
        text: "T".into(),
    });
    f.ok(Command::HierarchySwap { cell: q[0] });
    assert_eq!(outline(&f, top[0]), "P[T[T]]");
    assert_eq!(f.grid(top[0]).cells(), q);
}

#[test]
fn a_swap_out_of_a_wide_grid_leaves_an_empty_cell_or_takes_the_row() {
    // The cell above holds a table; "x" is found in it though it was selected elsewhere.
    let (mut f, top) = table(1, &["A", "B"]);
    let a = fill(&mut f, top[0], 2, &["x", "k", "", "", "m", "x"]);
    let b = fill(&mut f, top[1], 1, &["x"]);
    f.ok(props(&[top[0]], &[Prop::RelSize(3)]));
    let root = f.root();
    f.ok(Command::HierarchySwap { cell: b[0] });
    // Each x of the table leaves an empty cell, as its row and its column hold other
    // cells; the second and third x merge into the first. B is left empty and goes; its
    // copy has its ID.
    assert_eq!(outline(&f, root), "_[A[_ k / _ _ / m _] / x[A / B]]");
    assert_eq!(f.grid(root).cells()[1], a[0]);
    let left = f.grid(top[0]).cells()[0];
    assert_ne!(left, a[0]);
    assert_eq!(f.cell(left).rel_size, 3);
    assert_eq!(f.grid(a[0]).cells()[1], top[1]);
    // An x alone in its row takes the row with it.
    let (mut f, top) = table(1, &["A", "B"]);
    fill(&mut f, top[0], 2, &["", "x", "k", "m"]);
    let b = fill(&mut f, top[1], 1, &["y"]);
    let x = f.grid(top[0]).cells()[1];
    f.ok(Command::SetText {
        cell: b[0],
        text: "x".into(),
    });
    let root = f.root();
    f.ok(Command::HierarchySwap { cell: b[0] });
    assert_eq!(outline(&f, root), "_[A[k m] / x[A / B]]");
    assert_eq!(f.grid(root).cells()[1], x);
}

#[test]
fn hierarchy_swaps_that_cannot_be_done_are_refused() {
    let (mut f, top, fruit, apple, _) = tree();
    let root = f.root();
    // No cell two levels up.
    for cell in [root, top[0]] {
        assert!(matches!(
            f.bad(Command::HierarchySwap { cell }),
            Error::Refused(_)
        ));
    }
    // No text.
    f.ok(Command::SetText {
        cell: apple[0],
        text: String::new(),
    });
    assert!(matches!(
        f.bad(Command::HierarchySwap { cell: apple[0] }),
        Error::Refused(_)
    ));
    // Out of, or into, a grid of several rows and columns.
    let wide = fill(&mut f, fruit[1], 2, &["a", "b", "c", "d"]);
    assert!(matches!(
        f.bad(Command::HierarchySwap { cell: wide[0] }),
        Error::Refused(_)
    ));
    let inner = fill(&mut f, wide[0], 1, &["deep"]);
    assert!(matches!(
        f.bad(Command::HierarchySwap { cell: inner[0] }),
        Error::Refused(_)
    ));
    let missing = CellId(Id::from_u128(9));
    assert!(matches!(
        f.bad(Command::HierarchySwap { cell: missing }),
        Error::NotFound(_)
    ));
}

fn wrap(cell: CellId, x: u32, y: u32, xs: u32, ys: u32) -> Command {
    Command::Wrap { cell, x, y, xs, ys }
}

#[test]
fn cells_are_wrapped_in_a_new_parent() {
    let mut f = Fixture::new(3, 3);
    let (root, cells) = (f.root(), f.cells.clone());
    f.ok(props(
        &[cells[4]],
        &[Prop::CellColor(RED), Prop::RelSize(1)],
    ));
    f.ok(props(&[cells[5]], &[Prop::Note("stays".into())]));
    f.ok(Command::SetColumnWidth {
        cell: root,
        column: 2,
        width: 50,
    });
    f.ok(props(&[root], &[Prop::BorderColor(RED)]));
    // A block in the middle: its cells move into a new cell in the place of the first;
    // the other places are left empty, with the looks and notes of what was there.
    let changes = f.ok(wrap(root, 1, 1, 2, 2));
    assert_eq!(outline(&f, root), "_[0 1 2 / 3 _[4 5 / 7 8] _ / 6 _ _]");
    let grid = f.grid(root).cells().to_vec();
    let wrapper = f.cell(grid[4]);
    assert_eq!((wrapper.cell_color, wrapper.rel_size), (RED, 1));
    let inner = wrapper.grid().unwrap();
    assert_eq!(inner.cells(), [cells[4], cells[5], cells[7], cells[8]]);
    assert_eq!(
        (inner.col_widths.as_slice(), inner.border_color),
        ([80, 50].as_slice(), RED)
    );
    assert_eq!(f.cell(grid[5]).note, "stays");
    assert_eq!(f.cell(cells[5]).note, "stays");
    assert_eq!(changes.created.len(), 4);
    assert!(changes.removed.is_empty());
    assert_eq!(f.e.undo_len(), 5);
    // The whole grid: one cell is left, holding everything.
    let mut f = Fixture::new(3, 2);
    let root = f.root();
    let changes = f.ok(wrap(root, 0, 0, 3, 2));
    assert_eq!(outline(&f, root), "_[_[0 1 2 / 3 4 5]]");
    assert_eq!(changes.created.len(), 1);
    // Whole columns or rows left empty are deleted; a single cell just moves down.
    let mut f = Fixture::new(3, 2);
    let root = f.root();
    f.ok(wrap(root, 1, 0, 2, 2));
    assert_eq!(outline(&f, root), "_[0 _[1 2 / 4 5] / 3 _]");
    f.ok(wrap(root, 0, 1, 1, 1));
    assert_eq!(outline(&f, root), "_[0 _[1 2 / 4 5] / _[3] _]");
    let mut f = Fixture::new(1, 3);
    let root = f.root();
    f.ok(wrap(root, 0, 1, 1, 2));
    assert_eq!(outline(&f, root), "_[0 / _[1 / 2]]");
    for bad in [
        wrap(root, 0, 0, 0, 1),
        wrap(root, 0, 0, 1, 3),
        wrap(root, 1, 0, 1, 1),
        wrap(f.cells[0], 0, 0, 1, 1),
    ] {
        assert!(matches!(f.bad(bad), Error::Refused(_)));
    }
}

#[test]
fn wrapping_respects_the_nesting_limit() {
    let mut f = Fixture::new(1, 1);
    let mut at = f.root();
    for _ in 1..MAX_DEPTH {
        at = f.grid(at).cells()[0];
        f.ok(Command::CreateGrid {
            cell: at,
            xs: 1,
            ys: 1,
        });
    }
    assert!(matches!(f.bad(wrap(at, 0, 0, 1, 1)), Error::Limit(_)));
    let above = f.cell(at).parent().unwrap();
    assert!(matches!(f.bad(wrap(above, 0, 0, 1, 1)), Error::Limit(_)));
}

fn collapse(cell: CellId, x: u32, y: u32, xs: u32, ys: u32) -> Command {
    Command::Collapse { cell, x, y, xs, ys }
}

#[test]
fn cells_are_collapsed_into_the_first() {
    let mut f = Fixture::new(3, 2);
    let (root, cells) = (f.root(), f.cells.clone());
    fill(&mut f, cells[1], 1, &["in", ""]);
    f.ok(styled(cells[0], 0, 1, "b"));
    f.ok(props(&[cells[0]], &[Prop::LastEdit(Some(5))]));
    // The texts of the block and of what is inside it, after the first cell's own.
    f.ok(collapse(root, 0, 0, 2, 2));
    // The column of the emptied cells below and beside the first is left empty, and goes.
    assert_eq!(outline(&f, root), "_[0 1 in 3 4 2 / _ 5]");
    assert_eq!(f.runs(cells[0]), [(0, 1, "b".to_string(), None)]);
    assert_eq!(f.cell(cells[0]).last_edit, Some(5));
    // A row to its first cell: the emptied columns go.
    let mut f = Fixture::new(3, 1);
    let root = f.root();
    f.ok(collapse(root, 0, 0, 3, 1));
    assert_eq!(outline(&f, root), "_[0 1 2]");
    assert_eq!(f.grid(root).xs(), 1);
    // The first cell keeps its grid, though the texts in it are appended too.
    let mut f = Fixture::new(1, 2);
    let (root, cells) = (f.root(), f.cells.clone());
    fill(&mut f, cells[0], 1, &["a", "b"]);
    f.ok(collapse(root, 0, 0, 1, 2));
    assert_eq!(outline(&f, root), "_[0 a b 1[a / b]]");
    // Refused: one cell, nothing in the block, no such block.
    assert!(matches!(
        f.bad(collapse(root, 0, 0, 1, 1)),
        Error::Refused(_)
    ));
    assert!(matches!(
        f.bad(collapse(root, 0, 0, 1, 2)),
        Error::Refused(_)
    ));
    let mut f = Fixture::new(2, 1);
    let root = f.root();
    f.ok(Command::ClearCells {
        cells: f.cells.clone(),
    });
    assert!(matches!(
        f.bad(collapse(root, 0, 0, 2, 1)),
        Error::Refused(_)
    ));
}

#[test]
fn a_cell_is_split_at_a_position() {
    let (mut f, id) = Fixture::text("one two");
    let root = f.root();
    f.ok(styled(id, 2, 3, "b"));
    f.ok(linked(id, 5, 2, "u"));
    f.ok(props(
        &[id],
        &[
            Prop::CellColor(RED),
            Prop::Note("n".into()),
            Prop::Image(Some(ImageRef::new("i.png"))),
            Prop::LastEdit(Some(7)),
        ],
    ));
    f.e.clear_history();
    let changes = f.ok(Command::SplitCell { cell: id, at: 4 });
    assert_eq!(outline(&f, root), "_[_[one  two]]");
    // The cell stays what and where it was, emptied; the parts are new cells with their
    // share of the styles and links; the image goes with the first.
    let whole = f.cell(id);
    assert_eq!((whole.cell_color, whole.note.as_str()), (RED, "n"));
    assert!(whole.image.is_none());
    let parts = f.grid(id).cells().to_vec();
    assert_eq!(changes.created.len(), 2);
    assert_eq!(f.runs(parts[0]), [(2, 2, "b".to_string(), None)]);
    assert_eq!(f.runs(parts[1]), [(0, 1, "b".to_string(), None)]);
    assert!(f.links(parts[0]).is_empty());
    assert_eq!(f.links(parts[1]), [(1, 2, "u".to_string())]);
    let (first, second) = (f.cell(parts[0]), f.cell(parts[1]));
    assert_eq!(first.image, Some(ImageRef::new("i.png")));
    assert!(second.image.is_none());
    assert_eq!((first.cell_color, second.cell_color), (RED, RED));
    assert_eq!((first.last_edit, second.last_edit), (Some(7), Some(7)));
    assert_eq!((first.note.as_str(), second.note.as_str()), ("", ""));
    // A cell that has a grid keeps it beside the parts.
    let old = f.grid(id).clone();
    f.ok(Command::SetText {
        cell: id,
        text: "ab".into(),
    });
    f.ok(Command::SplitCell { cell: id, at: 2 });
    assert_eq!(outline(&f, root), "_[_[_[ab _] _[one  two]]]");
    let holders = f.grid(id).cells().to_vec();
    assert_eq!(*f.grid(holders[1]), old);
    assert_eq!(f.cell(parts[0]).parent(), Some(holders[1]));
    assert_eq!(f.cell(holders[0]).cell_color, RED);
    // Not inside a character made of several code points, not beyond the text.
    let (mut f, id) = Fixture::text("e\u{301}x");
    for at in [1, 4] {
        assert!(matches!(
            f.bad(Command::SplitCell { cell: id, at }),
            Error::Refused(_)
        ));
    }
    f.ok(Command::SplitCell { cell: id, at: 2 });
    assert_eq!(outline(&f, id), "_[e\u{301} x]");
}

#[test]
fn cell_texts_become_tags_and_stop_being_tags() {
    let mut f = Fixture::new(2, 2);
    let cells = f.cells.clone();
    f.ok(props(&[cells[0]], &[Prop::CellColor(RED)]));
    f.ok(props(&[cells[1]], &[Prop::TextColor(RED)]));
    f.ok(Command::SetText {
        cell: cells[2],
        text: String::new(),
    });
    f.ok(Command::SetText {
        cell: cells[3],
        text: "0".into(),
    });
    f.e.clear_history();
    // Each text with its cell's colours; no tag for the empty text; a later cell with the
    // same text decides the colours.
    let changes = f.ok(Command::TagCells {
        cells: cells.clone(),
    });
    assert_eq!(changes.tags, ["0", "1"]);
    let tags = f.doc().tags();
    assert_eq!(tags.len(), 2);
    assert_eq!(tags["0"].cell_color, Color::WHITE);
    assert_eq!(tags["1"].text_color, RED);
    f.ok(Command::TagCells {
        cells: vec![cells[0]],
    });
    assert_eq!(f.doc().tags()["0"].cell_color, RED);
    assert!(f
        .ok(Command::TagCells {
            cells: vec![cells[0], cells[2]]
        })
        .is_empty());
    // Removing passes over texts that are no tags.
    let changes = f.ok(Command::UntagCells {
        cells: vec![cells[1], cells[2], cells[1]],
    });
    assert_eq!(changes.tags, ["1"]);
    assert!(f
        .ok(Command::UntagCells {
            cells: vec![cells[1]]
        })
        .is_empty());
    let missing = CellId(Id::from_u128(9));
    for bad in [
        Command::TagCells {
            cells: vec![cells[0], missing],
        },
        Command::UntagCells {
            cells: vec![missing],
        },
    ] {
        assert!(matches!(f.bad(bad), Error::NotFound(_)));
    }
    assert_eq!(f.doc().tags().len(), 1);
}

#[test]
fn sizes_and_widths_change_by_steps_within_their_limits() {
    let mut f = Fixture::new(2, 1);
    let (root, cells) = (f.root(), f.cells.clone());
    let inner = fill(&mut f, cells[0], 2, &["a", "b"]);
    let resize = |cells: &[CellId], delta, recursive| Command::AdjustRelSize {
        cells: cells.to_vec(),
        delta,
        recursive,
    };
    f.ok(props(&[inner[0]], &[Prop::RelSize(7)]));
    // With the cells inside, each once, though named twice.
    f.ok(resize(&[cells[0], inner[1], cells[0]], 1, true));
    let sizes = |f: &Fixture| {
        let size = |id: CellId| f.cell(id).rel_size;
        [
            size(cells[0]),
            size(cells[1]),
            size(inner[0]),
            size(inner[1]),
        ]
    };
    assert_eq!(sizes(&f), [1, 0, 8, 1]);
    // Without; and the limits hold.
    f.ok(resize(&[cells[0]], -3, false));
    assert_eq!(sizes(&f), [-2, 0, 8, 1]);
    f.ok(resize(&[cells[0]], 5, true));
    assert_eq!(sizes(&f), [3, 0, 8, 6]);
    f.ok(resize(&cells, -100, true));
    assert_eq!(sizes(&f), [-32, -32, -32, -32]);
    assert!(f.ok(resize(&cells, -1, true)).is_empty());
    assert!(f.ok(resize(&cells, 0, true)).is_empty());
    assert_eq!((MIN_REL_SIZE, MAX_REL_SIZE), (-32, 8));

    let widen = |cell, column, count, delta, recursive| Command::AdjustColumnWidths {
        cell,
        column,
        count,
        delta,
        recursive,
    };
    f.ok(widen(root, 0, 1, 5, false));
    assert_eq!(f.grid(root).col_widths, [85, 80]);
    assert_eq!(f.grid(cells[0]).col_widths, [80, 80]);
    // With the grids inside the cells of those columns: all their columns.
    f.ok(widen(root, 0, 2, -10, true));
    assert_eq!(f.grid(root).col_widths, [75, 70]);
    assert_eq!(f.grid(cells[0]).col_widths, [70, 70]);
    f.ok(widen(root, 1, 1, -10, true));
    assert_eq!(f.grid(cells[0]).col_widths, [70, 70]);
    f.ok(widen(root, 0, 1, -1000, true));
    assert_eq!(f.grid(root).col_widths, [MIN_COL_WIDTH, 60]);
    assert_eq!(f.grid(cells[0]).col_widths, [MIN_COL_WIDTH; 2]);
    assert!(f.ok(widen(root, 0, 1, -5, true)).is_empty());
    assert!(f.ok(widen(root, 0, 0, 5, true)).is_empty());
    for bad in [
        widen(root, 2, 1, 5, false),
        widen(root, 1, 2, 5, false),
        widen(cells[1], 0, 1, 5, false),
    ] {
        assert!(matches!(f.bad(bad), Error::Refused(_)));
    }
}

#[test]
fn checkboxes_are_added_ticked_and_removed() {
    let mut f = Fixture::new(3, 1);
    let cells = f.cells.clone();
    let toggle = |cells: &[CellId]| Command::ToggleCheck {
        cells: cells.to_vec(),
    };
    f.ok(styled(cells[0], 0, 1, "b"));
    f.ok(Command::SetText {
        cell: cells[1],
        text: "\u{2611}x".into(),
    });
    f.ok(props(&cells, &[Prop::LastEdit(Some(1))]));
    let root = f.root();
    // Not all have one: those without get an open one. The ticked one is left alone.
    f.ok(toggle(&cells));
    assert_eq!(f.texts(root), "\u{2610} 0 \u{2611}x \u{2610} 2");
    assert_eq!(f.runs(cells[0]), [(0, 3, "b".to_string(), None)]);
    let edits = |f: &Fixture| cells.map_or_times(f);
    assert_eq!(edits(&f), [NOW, Some(1), NOW]);
    // All have one: all ticked. All ticked: the boxes go, with the space after them.
    f.ok(toggle(&cells));
    assert_eq!(f.texts(root), "\u{2611} 0 \u{2611}x \u{2611} 2");
    assert_eq!(edits(&f), [NOW; 3]);
    f.ok(toggle(&cells));
    assert_eq!(f.texts(root), "0 x 2");
    // A checkbox that is half of a longer character cannot be cut off it.
    f.ok(Command::SetText {
        cell: cells[0],
        text: "\u{2611}\u{FE0F} y".into(),
    });
    assert!(matches!(f.bad(toggle(&cells[..1])), Error::Refused(_)));
}

trait Times {
    fn map_or_times(&self, f: &Fixture) -> Vec<Option<i64>>;
}

impl Times for Vec<CellId> {
    fn map_or_times(&self, f: &Fixture) -> Vec<Option<i64>> {
        self.iter().map(|c| f.cell(*c).last_edit).collect()
    }
}
