//! The text syntax of queries against anything that can be typed: it never panics, it
//! gives the same query every time, and whatever it accepts can be searched for.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use proptest::prelude::*;
use wombat_core::construct::Construct;
use wombat_core::eval::Calc;
use wombat_core::{Document, SeededIds};
use wombat_search::query::{parse, Query};
use wombat_search::{naive, Options, SearchIndex};

fn document() -> Document {
    let mut ids = SeededIds::from_u64(8);
    let mut doc = Document::new(&mut ids);
    let root = doc.root();
    doc.add_grid(root, 2, 3, &mut ids).unwrap();
    let texts = [
        "tag:a",
        "(a OR b)",
        "\"quoted\"",
        "-a",
        "日本 語",
        "has:note",
    ];
    let cells = doc.cell(root).unwrap().grid().unwrap().cells().to_vec();
    for (id, text) in cells.iter().zip(texts) {
        doc.cell_mut(*id).unwrap().text = text.into();
    }
    doc
}

/// Text made of the pieces that mean something to the parser, and of anything at all.
fn typed() -> impl Strategy<Value = String> {
    let piece = prop_oneof![
        Just("a".to_owned()),
        Just(" ".to_owned()),
        Just("\"".to_owned()),
        Just("(".to_owned()),
        Just(")".to_owned()),
        Just("-".to_owned()),
        Just("|".to_owned()),
        Just(" OR ".to_owned()),
        Just(" AND ".to_owned()),
        Just("NOT ".to_owned()),
        Just("\u{201c}".to_owned()),
        Just("\u{201d}".to_owned()),
        Just("-5".to_owned()),
        Just("is:".to_owned()),
        Just("in:".to_owned()),
        Just("is-tag:".to_owned()),
        Just("formula:".to_owned()),
        Just("\u{301}".to_owned()),
        Just(":".to_owned()),
        Just("tag:".to_owned()),
        Just("has:note".to_owned()),
        Just("case:".to_owned()),
        Just("edited:".to_owned()),
        Just("7d".to_owned()),
        Just("2024-02-29".to_owned()),
        Just("..".to_owned()),
        Just("recent:5%".to_owned()),
        Just("sort:recent".to_owned()),
        Just("in:00000000-0000-0000-0000-000000000001".to_owned()),
        Just("日本".to_owned()),
        Just("e\u{301}".to_owned()),
        any::<char>().prop_map(String::from),
        "\\PC{0,6}",
    ];
    proptest::collection::vec(piece, 0..24).prop_map(|pieces| pieces.concat())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(3000))]

    #[test]
    fn any_text_parses_the_same_every_time_and_can_be_searched(text in typed(), now in any::<i64>()) {
        let first = parse(&text, now);
        prop_assert_eq!(&first, &parse(&text, now));
        let Ok(parsed) = first else {
            // An error says something.
            prop_assert!(!first.unwrap_err().to_string().is_empty());
            return Ok(());
        };
        prop_assert!(parsed.query.check().is_ok());
        // Whatever was accepted is searched without trouble, and as the reference says.
        let doc = document();
        let mut index = SearchIndex::new();
        index.insert_document(&doc, &Calc::new(&doc), "d.wombat");
        let options = Options::default();
        let found = index.search(&parsed.query, &options).unwrap();
        let expected = naive::search(&[("d.wombat", &doc, &Calc::new(&doc))], &parsed.query, &options).unwrap();
        prop_assert_eq!(found, expected);
        // Nothing typed, or only spaces: everything.
        if text.trim().is_empty() {
            prop_assert_eq!(parsed.query, Query::All);
        }
    }
}
