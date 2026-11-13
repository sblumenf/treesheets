//! The serialised form of a table to import is one flat list (decision A135), so that a
//! table nested deeper than any stack is refused as a value: nothing that reads, converts
//! or drops it calls itself. As a table within a table it killed the process.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use wombat_bridge::api::wire::{BridgeError, Command, EntryData, ErrorKind, GridSize, TableData};
use wombat_bridge::convert::{command_in, command_out};
use wombat_bridge::record::{command_from_json, command_to_json};
use wombat_core::convert::{Entry, Table};

const CELL: &str = "00000000-0000-4000-8000-000000000000";

fn entry(text: &str, grid: Option<(i64, i64)>) -> EntryData {
    EntryData {
        text: text.into(),
        formula: false,
        grid: grid.map(|(xs, cells)| GridSize { xs, cells }),
    }
}

fn import(xs: i64, cells: Vec<EntryData>) -> Command {
    Command::Import {
        cell: CELL.into(),
        table: TableData { xs, cells },
        replace: true,
    }
}

fn table(command: Command) -> Result<Table, BridgeError> {
    match command_in(command)? {
        wombat_core::Command::Import { table, .. } => Ok(table),
        other => panic!("{other:?}"),
    }
}

/// A chain of grids of one cell, each inside the cell of the one before.
fn chain(depth: usize) -> Command {
    let mut cells = vec![entry("x", Some((1, 1))); depth];
    cells.push(entry("x", None));
    import(1, cells)
}

/// On a stack of 2 MiB, which is what the threads have that the bridge's calls run on.
fn on_a_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    let small = std::thread::Builder::new().stack_size(2 << 20);
    small.spawn(work).unwrap().join().unwrap()
}

#[test]
fn a_table_nested_100_000_deep_is_refused_as_the_core_refuses_it() {
    let error = on_a_small_stack(|| table(chain(100_000)).unwrap_err());
    assert_eq!(error.kind, ErrorKind::Limit, "{error:?}");
    assert!(error.message.contains("nested deeper"), "{error:?}");
    // As deep as a document may be is a table; one level more is not.
    let deepest = wombat_core::limits::MAX_DEPTH as usize - 1;
    table(chain(deepest)).unwrap();
    assert_eq!(
        table(chain(deepest + 1)).unwrap_err().kind,
        ErrorKind::Limit
    );
    // The same as JSON text, which is as flat.
    let error = on_a_small_stack(|| {
        let text = serde_json::to_string(&chain(100_000)).unwrap();
        command_from_json(&text).unwrap_err()
    });
    assert_eq!(error.kind, ErrorKind::Limit, "{error:?}");
    // And a chain that never ends: the grids are still open when the list is over.
    let error = on_a_small_stack(|| {
        let open = vec![entry("x", Some((1, 1))); 100_000];
        table(import(1, open)).unwrap_err()
    });
    assert_eq!(error.kind, ErrorKind::BadRequest, "{error:?}");
}

/// The limits of a document are the core's to keep, and are told apart from a list that
/// is no table: too deep is above; too many cells and a text too long are here.
#[test]
fn a_table_past_a_limit_is_refused_as_the_core_refuses_it() {
    let many = wombat_core::limits::MAX_CELLS;
    let error = table(import(1, vec![entry("", None); many])).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Limit, "{error:?}");
    // One too many inside, in whole rows: the shape is right, the size is not.
    let mut cells = vec![entry("a", Some((2, many as i64)))];
    cells.extend(vec![entry("", None); many]);
    let error = table(import(1, cells)).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Limit, "{error:?}");
    let long = "x".repeat(wombat_core::limits::MAX_TEXT_CODE_POINTS + 1);
    let error = table(import(1, vec![entry(&long, None)])).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Limit, "{error:?}");
}

#[test]
fn a_flat_list_is_the_table_it_describes() {
    // a b
    // c d      where a holds a column of e and f, f holds g, and d holds a row of h and i.
    let cells = vec![
        entry("a", Some((1, 2))),
        entry("e", None),
        entry("f", Some((1, 1))),
        entry("g", None),
        entry("b", None),
        entry("c", None),
        entry("d", Some((2, 2))),
        entry("h", None),
        entry("i", None),
    ];
    let g = Table::new(1, vec![Entry::new("g")]).unwrap();
    let a = Table::new(1, vec![Entry::new("e"), Entry::new("f").with_grid(g)]).unwrap();
    let d = Table::new(2, vec![Entry::new("h"), Entry::new("i")]).unwrap();
    let expected = Table::new(
        2,
        vec![
            Entry::new("a").with_grid(a),
            Entry::new("b"),
            Entry::new("c"),
            Entry::new("d").with_grid(d),
        ],
    )
    .unwrap();
    let carried = import(2, cells);
    assert_eq!(table(carried.clone()).unwrap(), expected);
    // And back: the list of a table is that list, also as JSON.
    let core = command_in(carried.clone()).unwrap();
    assert_eq!(command_out(&core).unwrap(), carried);
    let text = command_to_json(&core).unwrap();
    assert!(text.contains(r#"{"text":"f","formula":false,"grid":{"xs":1,"cells":1}}"#));
    assert_eq!(command_from_json(&text).unwrap(), core);
}

#[test]
fn a_list_that_is_no_table_is_refused() {
    let kind = |xs, cells| table(import(xs, cells)).unwrap_err().kind;
    let plain = || entry("x", None);
    // A grid that says it has more cells than follow, at the end and further out.
    assert_eq!(
        kind(1, vec![entry("a", Some((1, 2))), plain()]),
        ErrorKind::BadRequest
    );
    let short = vec![entry("a", Some((1, 2))), entry("b", Some((1, 1))), plain()];
    assert_eq!(kind(1, short), ErrorKind::BadRequest);
    let error = table(import(1, vec![entry("a", Some((1, 3))), plain()])).unwrap_err();
    assert!(error.message.contains("2 cells short"), "{error:?}");
    // Numbers that are none.
    for (xs, cells) in [(-1, 1), (1, -1), (1 << 32, 1), (1, 1 << 32)] {
        let list = vec![entry("a", Some((xs, cells))), plain()];
        assert_eq!(kind(1, list), ErrorKind::BadRequest);
    }
    // What is no table of any size: no cells, no columns, rows that are not whole, in
    // the outermost grid and in one inside. It is the caller's mistake like the rest, and
    // the bridge says so itself: the core's word for it, `Invalid`, is kept for a file.
    assert_eq!(kind(1, vec![]), ErrorKind::BadRequest);
    assert_eq!(kind(0, vec![plain()]), ErrorKind::BadRequest);
    assert_eq!(kind(0, vec![]), ErrorKind::BadRequest);
    assert_eq!(kind(2, vec![plain(); 3]), ErrorKind::BadRequest);
    let inside = |xs, cells, follow| {
        let mut list = vec![entry("a", Some((xs, cells)))];
        list.extend(vec![plain(); follow]);
        list
    };
    assert_eq!(kind(1, inside(1, 0, 1)), ErrorKind::BadRequest);
    assert_eq!(kind(1, inside(0, 1, 1)), ErrorKind::BadRequest);
    assert_eq!(kind(1, inside(0, 0, 0)), ErrorKind::BadRequest);
    assert_eq!(kind(1, inside(2, 3, 3)), ErrorKind::BadRequest);
    // Cells after a grid inside that leave the outermost grid with a row not whole.
    assert_eq!(kind(2, inside(1, 1, 3)), ErrorKind::BadRequest);
    let error = table(import(2, inside(3, 4, 5))).unwrap_err();
    assert!(error.message.contains("4 cells in 3 columns"), "{error:?}");
    // In a grid deep inside, and the same as JSON text.
    let mut deep = vec![entry("x", Some((1, 1))); 10];
    deep.extend(inside(2, 3, 3));
    assert_eq!(kind(1, deep.clone()), ErrorKind::BadRequest);
    let text = serde_json::to_string(&import(1, deep)).unwrap();
    let error = command_from_json(&text).unwrap_err();
    assert_eq!(error.kind, ErrorKind::BadRequest, "{error:?}");
    // The cells after a grid that is full belong to the grid around it.
    let after = vec![entry("a", Some((1, 1))), plain(), plain()];
    assert_eq!(table(import(1, after)).unwrap().cells().len(), 2);
}
