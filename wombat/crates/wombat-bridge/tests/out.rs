//! What goes out over the bridge: each list of a change report, what a formula cell
//! shows, and every error of the core with its kind and its file. And one check on the
//! way in that the other tests passed by. Each of these fails if the conversion mixes up
//! or drops what it names.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::io;
use std::path::{Path, PathBuf};

use wombat_bridge::api::document::OpenDocument;
use wombat_bridge::api::wire::{self, BridgeError, CellKind, Command, ErrorKind, Report};
use wombat_bridge::convert::{command_in, report_out};
use wombat_core::{CellId, Changes, Error, Id, SeededIds};

fn id(n: u128) -> CellId {
    CellId(Id::from_u128(n))
}

#[test]
fn every_list_of_a_report_is_the_list_of_that_name() {
    let text = |n: u128| vec![id(n).to_string()];
    let changes = Changes {
        changed: vec![id(1)],
        created: vec![id(2)],
        removed: vec![id(3)],
        grids: vec![id(4)],
        tags: vec!["a tag".into(), "another".into()],
    };
    let expected = Report {
        changed: text(1),
        created: text(2),
        removed: text(3),
        grids: text(4),
        tags: vec!["a tag".into(), "another".into()],
        recalculated: text(5),
    };
    assert_eq!(report_out(&changes, &[id(5)]), expected);
    assert_eq!(report_out(&Changes::default(), &[]), Report::default());
}

/// The lists as an open document gives them: a command that removes cells, and one that
/// changes a tag.
#[test]
fn a_report_names_removed_cells_and_tags() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("out-report");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut doc = OpenDocument::create_with(&dir.join("a.wombat"), SeededIds::from_u64(1)).unwrap();
    let root = doc.root();
    let grid = Command::CreateGrid {
        cell: root.clone(),
        xs: 1,
        ys: 2,
    };
    let created = doc.apply(grid).unwrap().created;
    assert_eq!(created.len(), 2);
    let second = doc.grid(root.clone()).unwrap().cells[1].id.clone();
    let delete = Command::DeleteRows {
        cell: root.clone(),
        at: 1,
        count: 1,
    };
    let report = doc.apply(delete).unwrap();
    assert_eq!(report.removed, vec![second]);
    assert_eq!(report.grids, vec![root.clone()]);
    assert_eq!(report.created, Vec::<String>::new());

    let rgb = |r| wire::Rgb { r, g: 0, b: 0 };
    let tag = Command::SetTag {
        name: "urgent".into(),
        cell_color: rgb(1),
        text_color: rgb(2),
    };
    let expected = Report {
        tags: vec!["urgent".into()],
        ..Report::default()
    };
    assert_eq!(doc.apply(tag).unwrap(), expected);
}

#[test]
fn a_formula_that_fails_is_marked_as_failed() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("out-failed");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut doc = OpenDocument::create_with(&dir.join("a.wombat"), SeededIds::from_u64(2)).unwrap();
    let root = doc.root();
    let grid = Command::CreateGrid {
        cell: root.clone(),
        xs: 3,
        ys: 1,
    };
    doc.apply(grid).unwrap();
    let cells = doc.grid(root.clone()).unwrap().cells;
    for (cell, text) in cells.iter().zip(["=nosuchfunction(1)", "=1+2", "plain"]) {
        let enter = Command::EnterText {
            cell: cell.id.clone(),
            text: text.into(),
        };
        doc.apply(enter).unwrap();
    }
    let shown: Vec<_> = doc
        .grid(root)
        .unwrap()
        .cells
        .into_iter()
        .map(|c| (c.kind, c.result, c.failed))
        .collect();
    let (kind, result, failed) = &shown[0];
    assert_eq!(*kind, CellKind::Formula);
    assert!(
        result.as_deref().is_some_and(|r| r.starts_with('#')),
        "{result:?}"
    );
    assert!(failed);
    assert_eq!(shown[1], (CellKind::Formula, Some("3".into()), false));
    assert_eq!(shown[2], (CellKind::Data, None, false));
    // The cell itself is read as the cells of a grid are.
    let own = doc.grid(cells[0].id.clone()).unwrap().cell;
    assert!(own.failed && own.result.is_some());
}

/// One error of the core of every kind there is. The match does not compile when the
/// core gains one; then the list and the bridge's `ErrorKind` need it.
fn every_error() -> Vec<(Error, ErrorKind, Option<&'static str>)> {
    let io = || io::Error::other("the disk said no");
    let all = vec![
        (
            Error::Io {
                path: PathBuf::from("/a/io.wombat"),
                source: io(),
            },
            ErrorKind::Io,
            Some("/a/io.wombat"),
        ),
        (Error::Syntax("s".into()), ErrorKind::Syntax, None),
        (Error::WrongFormat("w".into()), ErrorKind::WrongFormat, None),
        (
            Error::NewerVersion {
                found: 9,
                supported: 1,
            },
            ErrorKind::NewerVersion,
            None,
        ),
        (Error::Invalid("i".into()), ErrorKind::Invalid, None),
        (Error::Refused("r".into()), ErrorKind::Refused, None),
        (Error::Limit("l".into()), ErrorKind::Limit, None),
        (Error::NotFound("n".into()), ErrorKind::NotFound, None),
        (
            Error::AlreadyExists(PathBuf::from("/a/there.wombat")),
            ErrorKind::AlreadyExists,
            Some("/a/there.wombat"),
        ),
        (
            Error::HalfSaved {
                saved: PathBuf::from("/a/saved.wombat"),
                not_saved: PathBuf::from("/a/not-saved.wombat"),
                backup: Some(PathBuf::from("/a/backup.wombat")),
                source: io(),
                restore: io(),
            },
            ErrorKind::HalfSaved,
            // The file that still needs saving is the one to tell the user about.
            Some("/a/not-saved.wombat"),
        ),
    ];
    for (error, ..) in &all {
        match error {
            Error::Io { .. }
            | Error::Syntax(_)
            | Error::WrongFormat(_)
            | Error::NewerVersion { .. }
            | Error::Invalid(_)
            | Error::Refused(_)
            | Error::Limit(_)
            | Error::NotFound(_)
            | Error::AlreadyExists(_)
            | Error::HalfSaved { .. } => {}
        }
    }
    all
}

#[test]
fn every_error_of_the_core_has_its_kind_its_message_and_its_file() {
    let all = every_error();
    let mut kinds = Vec::new();
    for (error, kind, path) in all {
        let message = error.to_string();
        let carried = BridgeError::from(error);
        assert_eq!(carried.kind, kind, "{message}");
        assert_eq!(carried.path.as_deref(), path, "{message}");
        assert_eq!(carried.message, message);
        assert!(!kinds.contains(&kind), "{kind:?} twice");
        kinds.push(kind);
    }
    // One of each variant of the core, each with a kind of its own; `BadRequest` is the
    // one kind that no error of the core has.
    assert_eq!(kinds.len(), 10);
    assert!(!kinds.contains(&ErrorKind::BadRequest));
}

/// The scale of an image that is set whole is checked like a scale set alone: JSON
/// cannot write a number that is not finite, so one must not get in.
#[test]
fn the_scale_of_an_image_must_be_finite() {
    let image = |scale| {
        command_in(Command::SetProps {
            cells: vec![id(1).to_string()],
            props: vec![wire::Prop::Image(Some(wire::ImageRef {
                file: "a.png".into(),
                scale,
                extra: String::new(),
            }))],
        })
    };
    image(1.5).unwrap();
    for scale in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let error = image(scale).unwrap_err();
        assert_eq!(error.kind, ErrorKind::BadRequest, "{error:?}");
    }
}
