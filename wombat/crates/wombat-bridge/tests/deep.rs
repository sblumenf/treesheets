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
    // What the core does not take for a table: no cells, no columns, rows that are not
    // whole, in the outermost grid and in one inside.
    assert_eq!(kind(1, vec![]), ErrorKind::Invalid);
    assert_eq!(kind(0, vec![plain()]), ErrorKind::Invalid);
    assert_eq!(kind(2, vec![plain(); 3]), ErrorKind::Invalid);
    assert_eq!(
        kind(1, vec![entry("a", Some((1, 0))), plain()]),
        ErrorKind::Invalid
    );
    assert_eq!(
        kind(1, vec![entry("a", Some((0, 1))), plain()]),
        ErrorKind::Invalid
    );
    let ragged = vec![entry("a", Some((2, 3))), plain(), plain(), plain()];
    assert_eq!(kind(1, ragged), ErrorKind::Invalid);
    // The cells after a grid that is full belong to the grid around it.
    let after = vec![entry("a", Some((1, 1))), plain(), plain()];
    assert_eq!(table(import(1, after)).unwrap().cells().len(), 2);
}
