//! What search finds, by example: matching, every filter, the order of results,
//! snippets and highlights. Every search here is also held to the reference scanner.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

#[path = "support/mod.rs"]
mod search_support;

use search_support::{check_hit, Rng};
use wombat_core::construct::Construct;
use wombat_core::eval::Calc;
use wombat_core::summary::Summary;
use wombat_core::{
    CellId, CellKind, Color, Command, Document, Editor, ImageRef, Link, LinkTarget, SeededIds,
    Style, Tag,
};
use wombat_search::query::{parse, Filter, Has, Order, Query, Term};
use wombat_search::{naive, DocIndex, Field, Hit, Options, SearchIndex};

const NOW: i64 = 1_800_000_000_000;
const DAY: i64 = 86_400_000;

/// Gives `cell` a grid of one column with these texts.
fn column(doc: &mut Document, ids: &mut SeededIds, cell: CellId, texts: &[&str]) -> Vec<CellId> {
    doc.add_grid(cell, 1, texts.len() as u32, ids).unwrap();
    let cells = doc.cell(cell).unwrap().grid().unwrap().cells().to_vec();
    for (id, text) in cells.iter().zip(texts) {
        doc.cell_mut(*id).unwrap().text = text.to_string();
    }
    cells
}

fn link(doc: &mut Document, cell: CellId, start: u32, len: u32, target: LinkTarget) {
    doc.cell_mut(cell).unwrap().links.push(Link {
        start,
        len,
        target,
        extra: Default::default(),
    });
}

struct World {
    shop: Document,
    notes: Document,
    /// The results of the formula cells of the two, as after loading.
    calcs: [Calc; 2],
    fruit: Vec<CellId>,
    apple_kinds: Vec<CellId>,
    lines: Vec<CellId>,
}

/// Two documents. `shop.wombat`: fruit with prices and properties. `notes.wombat`: lines
/// of text in several scripts, with links into the shop.
fn world() -> World {
    let mut ids = SeededIds::from_u64(6);
    let mut shop = Document::new(&mut ids);
    let root = shop.root();
    shop.cell_mut(root).unwrap().text = "Shop".into();
    let fruit = column(
        &mut shop,
        &mut ids,
        root,
        &[
            "Apple",
            "Pineapple",
            "apple pie",
            "Crab-apple jelly",
            "Pear",
            "todo",
            "Café Zürich",
        ],
    );
    let apple_kinds = column(
        &mut shop,
        &mut ids,
        fruit[0],
        &["Bramley", "green apple", "todo"],
    );
    {
        let cell = shop.cell_mut(fruit[4]).unwrap();
        cell.note = "Ask about the apple harvest.\nSecond line.".into();
        cell.cell_color = Color::rgb(0xff, 0, 0);
        cell.last_edit = Some(NOW - 2 * DAY);
    }
    {
        let cell = shop.cell_mut(fruit[1]).unwrap();
        cell.image = Some(ImageRef::new("Apple-Photo.png"));
        cell.style = Style {
            bold: true,
            ..Style::default()
        };
        cell.text_color = Color::rgb(0, 0, 0xff);
        cell.last_edit = Some(NOW - 40 * DAY);
    }
    shop.cell_mut(fruit[2]).unwrap().kind = CellKind::Formula;
    shop.cell_mut(fruit[2]).unwrap().last_edit = Some(NOW - DAY / 2);
    shop.cell_mut(apple_kinds[0]).unwrap().last_edit = Some(NOW);
    shop.tags_mut().insert("todo".into(), Tag::default());
    link(
        &mut shop,
        fruit[4],
        0,
        4,
        LinkTarget::External("https://example.com/Apple-Orchard".into()),
    );

    let mut notes = Document::new(&mut ids);
    let root = notes.root();
    notes.cell_mut(root).unwrap().text = "Notes".into();
    let lines = column(
        &mut notes,
        &mut ids,
        root,
        &[
            "see the fruit here",
            "日本語のテキストを検索する",
            "한국어 텍스트 검색",
            "中文搜索没有空格",
            "ＦＵＬＬ ｗｉｄｔｈ ｶﾞｲﾄﾞ",
            "Straße nach İstanbul",
            "todo",
            "gone",
        ],
    );
    link(
        &mut notes,
        lines[0],
        8,
        5,
        LinkTarget::Cell {
            doc: Some(shop.id()),
            cell: fruit[0],
        },
    );
    link(
        &mut notes,
        lines[7],
        0,
        4,
        LinkTarget::Cell {
            doc: Some(shop.id()),
            cell: CellId::parse("00000000-0000-0000-0000-00000000dead").unwrap(),
        },
    );
    shop.validate().unwrap();
    notes.validate().unwrap();
    World {
        calcs: [Calc::new(&shop), Calc::new(&notes)],
        shop,
        notes,
        fruit,
        apple_kinds,
        lines,
    }
}

impl World {
    fn docs(&self) -> Vec<naive::Named<'_>> {
        vec![
            ("shop.wombat", &self.shop, &self.calcs[0]),
            ("notes.wombat", &self.notes, &self.calcs[1]),
        ]
    }

    fn index(&self) -> SearchIndex {
        let mut index = SearchIndex::new();
        for (name, doc, calc) in self.docs() {
            index.insert_document(doc, calc, name);
        }
        index
    }

    /// Searches with the text syntax; the result is also what the reference gives.
    fn find(&self, text: &str) -> Vec<Hit> {
        let parsed = parse(text, NOW).unwrap_or_else(|error| panic!("{text}: {error}"));
        self.query(&parsed.query, parsed.order.unwrap_or_default())
    }

    fn query(&self, query: &Query, order: Order) -> Vec<Hit> {
        let options = Options::all(order);
        let found = self.index().search(query, &options).unwrap();
        let expected = naive::search(&self.docs(), query, &options).unwrap();
        assert_eq!(found, expected, "{query:?}");
        for hit in &found.hits {
            check_hit(hit, &self.docs());
        }
        found.hits
    }

    fn texts(&self, text: &str) -> Vec<String> {
        self.find(text).into_iter().map(|hit| hit.text).collect()
    }
}

#[test]
fn words_match_anywhere_whatever_their_case_accents_or_width() {
    let w = world();
    // While typing: every prefix of a word finds it.
    for typed in [
        "a",
        "ap",
        "app",
        "appl",
        "apple",
        "APPLE",
        "ＡＰＰＬＥ",
        "Äpple",
    ] {
        let found = w.texts(typed);
        assert!(found.contains(&"Apple".to_string()), "{typed}: {found:?}");
        assert!(
            found.contains(&"Pineapple".to_string()),
            "{typed}: {found:?}"
        );
    }
    // The best first: the whole cell, a whole word, the start of a word, inside a word.
    assert_eq!(
        w.texts("apple"),
        [
            "Apple",
            "apple pie",
            "green apple",
            "Crab-apple jelly",
            "Pineapple",
            // By its note, by the cell its link leads to, and by its link's address.
            "Pear",
            "see the fruit here",
        ]
    );
    // Several words: all of them, in any field and any order.
    assert_eq!(w.texts("pie apple"), ["apple pie"]);
    assert_eq!(w.texts("harvest pear"), ["Pear"]);
    assert_eq!(w.texts("apple zebra"), [] as [&str; 0]);
    // A phrase: the words together.
    assert_eq!(w.texts("\"apple pie\""), ["apple pie"]);
    assert_eq!(w.texts("\"pie apple\""), [] as [&str; 0]);
    assert_eq!(w.texts("\"e p\""), ["apple pie"]);
    // Or, not, brackets.
    assert_eq!(w.texts("pear OR bramley"), ["Pear", "Bramley"]);
    assert_eq!(w.texts("pear | bramley"), ["Pear", "Bramley"]);
    assert_eq!(
        w.texts("apple -pie -green -jelly -pine"),
        ["Apple", "Pear", "see the fruit here"]
    );
    assert_eq!(w.texts("(pear OR bramley) -harvest"), ["Bramley"]);
    // Accents and case fold; `case:` asks for the exact characters.
    assert_eq!(w.texts("cafe zurich"), ["Café Zürich"]);
    assert_eq!(w.texts("CAFÉ"), ["Café Zürich"]);
    assert_eq!(w.texts("case:Café"), ["Café Zürich"]);
    assert_eq!(w.texts("case:cafe"), [] as [&str; 0]);
    assert_eq!(
        w.texts("case:apple"),
        [
            "apple pie",
            "green apple",
            "Crab-apple jelly",
            "Pineapple",
            "Pear"
        ]
    );
    assert_eq!(w.texts("strasse"), ["Straße nach İstanbul"]);
    assert_eq!(w.texts("STRASSE istanbul"), ["Straße nach İstanbul"]);
    assert_eq!(w.texts("ıstanbul"), ["Straße nach İstanbul"]);
    // Full-width and half-width forms.
    assert_eq!(w.texts("full width"), ["ＦＵＬＬ ｗｉｄｔｈ ｶﾞｲﾄﾞ"]);
    assert_eq!(w.texts("ガイド"), ["ＦＵＬＬ ｗｉｄｔｈ ｶﾞｲﾄﾞ"]);
    // Text without spaces: any part of it, one character included.
    assert_eq!(w.texts("テキスト"), ["日本語のテキストを検索する"]);
    assert_eq!(w.texts("本語"), ["日本語のテキストを検索する"]);
    assert_eq!(
        w.texts("索"),
        ["中文搜索没有空格", "日本語のテキストを検索する"]
    );
    assert_eq!(w.texts("搜索 空格"), ["中文搜索没有空格"]);
    assert_eq!(w.texts("텍스트"), ["한국어 텍스트 검색"]);
    assert_eq!(w.texts("국"), ["한국어 텍스트 검색"]);
    assert_eq!(
        w.texts("てきすと"),
        [] as [&str; 0],
        "hiragana is not katakana"
    );
    // A word with a colon that is no filter is a word.
    assert_eq!(w.texts("https://example.com"), ["Pear"]);
    // Nothing typed yet: everything.
    assert_eq!(w.find("").len(), w.shop.cell_count() + w.notes.cell_count());
}

#[test]
fn a_hit_says_where_and_what_to_highlight() {
    let w = world();
    let hits = w.find("apple");
    let by_text = |text: &str| hits.iter().find(|hit| hit.text == text).unwrap();
    let apple = by_text("Apple");
    assert_eq!((apple.doc, apple.cell), (w.shop.id(), w.fruit[0]));
    assert_eq!((apple.field, apple.score, apple.depth), (Field::Text, 0, 1));
    assert_eq!(apple.ranges, [(0, 5)]);
    assert_eq!(apple.trail, ["Shop"]);
    let green = by_text("green apple");
    assert_eq!(green.trail, ["Shop", "Apple"]);
    assert_eq!(
        (green.ranges.clone(), green.score, green.depth),
        (vec![(6, 5)], 1, 2)
    );
    assert_eq!(by_text("Pineapple").ranges, [(4, 5)]);
    assert_eq!(
        by_text("Pineapple").score,
        3,
        "inside a word; the image name counts less"
    );
    // By the note: the note is what matched, with a snippet of it.
    let pear = by_text("Pear");
    assert_eq!((pear.field, pear.score), (Field::Note, 2 * 4 + 1));
    assert_eq!(pear.matched, "Ask about the apple harvest.\nSecond line.");
    assert_eq!(pear.ranges, [(14, 5)]);
    assert_eq!(
        pear.snippet.text,
        "Ask about the apple harvest. Second line."
    );
    assert_eq!(pear.snippet.ranges, [(14, 5)]);
    // By the text of the cell that a link leads to.
    let linking = by_text("see the fruit here");
    assert_eq!((linking.field, linking.score), (Field::LinkedText, 3 * 4));
    assert_eq!(
        (linking.matched.as_str(), linking.ranges.clone()),
        ("Apple", vec![(0, 5)])
    );
    assert_eq!(linking.trail, ["Notes"]);
    // By a link's address, and by an image's name.
    let hits = w.find("orchard");
    assert_eq!(
        (hits.len(), hits[0].field, hits[0].score),
        (1, Field::Url, 5 * 4 + 1)
    );
    assert_eq!(hits[0].matched, "https://example.com/Apple-Orchard");
    assert_eq!(hits[0].ranges, [(26, 7)]);
    let hits = w.find("photo");
    assert_eq!(
        (hits.len(), hits[0].field, hits[0].score),
        (1, Field::Image, 4 * 4 + 1)
    );
    assert_eq!(
        (hits[0].matched.as_str(), hits[0].ranges.clone()),
        ("Apple-Photo.png", vec![(6, 5)])
    );
    // Several words: all their places in the best field, joined where they touch.
    let hits = w.find("ple app \"e p\"");
    assert_eq!(hits[0].text, "apple pie");
    assert_eq!(hits[0].ranges, [(0, 7)]);
    // Folded matches go back to whole characters of the text as it is.
    let hits = w.find("ss");
    assert_eq!(
        (hits[0].text.as_str(), hits[0].ranges.clone()),
        ("Straße nach İstanbul", vec![(4, 1)])
    );
    let hits = w.find("ガイ");
    assert_eq!(
        hits[0].ranges,
        [(11, 3)],
        "half-width ka with its voicing mark is one character"
    );
    // A filter alone marks nothing.
    let hits = w.find("has:note");
    assert_eq!(
        (
            hits.len(),
            hits[0].field,
            hits[0].ranges.len(),
            hits[0].score
        ),
        (1, Field::Text, 0, 0)
    );
    assert_eq!(hits[0].snippet.text, "Pear");
}

#[test]
fn every_filter() {
    let w = world();
    let id = |cell: CellId| cell.to_string();
    // Tags: the cells whose text is the tag, in documents that have the tag.
    assert_eq!(w.texts("tag:todo sort:document"), ["todo", "todo"]);
    assert_eq!(
        w.find("tag:todo")
            .iter()
            .filter(|hit| hit.doc == w.notes.id())
            .count(),
        0
    );
    assert_eq!(w.texts("has:tag"), ["todo", "todo"]);
    assert_eq!(
        w.texts("tag:apple"),
        [] as [&str; 0],
        "a text that is no tag"
    );
    assert_eq!(
        w.texts("todo -has:tag"),
        ["todo"],
        "the cell of the document without the tag"
    );
    // Having things.
    assert_eq!(w.texts("has:note"), ["Pear"]);
    assert_eq!(w.texts("has:image"), ["Pineapple"]);
    assert_eq!(
        w.texts("has:grid sort:document"),
        ["Notes", "Shop", "Apple"]
    );
    assert_eq!(
        w.texts("has:link sort:document"),
        ["see the fruit here", "gone", "Pear"]
    );
    assert_eq!(w.texts("has:broken-link"), ["gone"]);
    assert_eq!(w.find("-has:text").len(), 0);
    // Links.
    assert_eq!(
        w.texts(&format!("links-to:{}", id(w.fruit[0]))),
        ["see the fruit here"]
    );
    assert_eq!(
        w.texts(&format!("linked-from:{}", id(w.lines[0]))),
        ["Apple"]
    );
    assert_eq!(
        w.texts(&format!("links-to:{}", id(w.fruit[1]))),
        [] as [&str; 0]
    );
    // Within a branch: the cell and everything inside it.
    let branch = format!("in:{}", id(w.fruit[0]));
    assert_eq!(
        w.texts(&format!("{branch} sort:document")),
        ["Apple", "Bramley", "green apple", "todo"]
    );
    assert_eq!(
        w.texts(&format!("apple {branch}")),
        ["Apple", "green apple"]
    );
    assert_eq!(w.texts(&format!("apple -{branch}")).len(), 5);
    // By document, with or without the ending, in any case.
    assert_eq!(w.texts("todo doc:notes"), ["todo"]);
    assert_eq!(
        w.texts("todo doc:SHOP.wombat sort:document"),
        ["todo", "todo"]
    );
    assert_eq!(w.texts("todo doc:nothing"), [] as [&str; 0]);
    let by_id = Query::Filter(Filter::Doc(w.notes.id()));
    assert_eq!(w.query(&by_id, Order::Document).len(), w.notes.cell_count());
    // Kind, colours, style.
    assert_eq!(w.texts("kind:formula"), ["apple pie"]);
    assert_eq!(w.texts("bg:#ff0000"), ["Pear"]);
    assert_eq!(w.texts("fg:#0000ff"), ["Pineapple"]);
    assert_eq!(w.texts("style:bold"), ["Pineapple"]);
    assert_eq!(
        w.find("style:none").len(),
        w.shop.cell_count() + w.notes.cell_count() - 1
    );
    assert_eq!(
        w.texts("style:bold+italic"),
        [] as [&str; 0],
        "the style must be exactly this"
    );
    // The same text, the same image.
    assert_eq!(w.texts("text:todo sort:document"), ["todo", "todo", "todo"]);
    assert_eq!(w.texts("text:\"apple pie\""), ["apple pie"]);
    assert_eq!(w.texts("text:Todo"), [] as [&str; 0], "exactly the same");
    assert_eq!(w.texts("image:Apple-Photo.png"), ["Pineapple"]);
    // Edit times: the caller says what time it is.
    assert_eq!(w.texts("edited:1d sort:recent"), ["Bramley", "apple pie"]);
    assert_eq!(
        w.texts("edited:3d sort:recent"),
        ["Bramley", "apple pie", "Pear"]
    );
    assert_eq!(w.texts("edited:6w sort:recent").len(), 4);
    assert_eq!(w.texts("edited:36h -edited:1h"), ["apple pie"]);
    assert_eq!(
        w.texts("edited:2027-01-15"),
        ["Bramley"],
        "the day of NOW, in UTC"
    );
    assert_eq!(
        w.texts("edited:2027-01-13..2027-01-14"),
        ["Pear", "apple pie"]
    );
    assert_eq!(w.texts("edited:..2027-01-01"), ["Pineapple"]);
    assert_eq!(
        w.texts("edited:2027-01-14.. sort:recent"),
        ["Bramley", "apple pie"]
    );
    // TreeSheets' "show 20% of last edits": of each document, counted its way.
    assert_eq!(
        w.texts("recent:20% doc:shop sort:recent"),
        ["Bramley", "apple pie", "Pear"]
    );
    assert_eq!(w.texts("recent:1% doc:shop"), ["Bramley"]);
    // Filters with words, and everything at once.
    assert_eq!(w.texts("apple edited:1d"), ["apple pie"]);
    assert_eq!(
        w.texts("apple (kind:formula OR has:image) -style:bold"),
        ["apple pie"]
    );
}

#[test]
fn the_order_of_results_is_the_documented_one() {
    let w = world();
    // Points first; then the shorter text; then the shallower cell; then the document's
    // name and the order within the document.
    let hits = w.find("todo");
    let places: Vec<_> = hits.iter().map(|hit| (hit.doc, hit.depth)).collect();
    assert_eq!(
        places,
        [(w.notes.id(), 1), (w.shop.id(), 1), (w.shop.id(), 2)],
        "equal points and lengths: by depth, then notes.wombat before shop.wombat"
    );
    assert_eq!(
        w.texts("p"),
        [
            "Pear",
            "Pineapple",
            "apple pie",
            "Shop",
            "Apple",
            "green apple",
            "Crab-apple jelly",
            "see the fruit here"
        ]
    );
    // More recent first, when asked, among equally good matches or before all else.
    assert_eq!(
        w.texts("pie OR pear OR bramley"),
        ["Pear", "Bramley", "apple pie"]
    );
    assert_eq!(
        w.texts("pie OR pear OR bramley sort:recent"),
        ["Bramley", "apple pie", "Pear"]
    );
    let hits = w.find("e sort:relevance-recent");
    let pos = |text: &str| hits.iter().position(|hit| hit.text == text).unwrap();
    assert!(
        pos("apple pie") < pos("Apple"),
        "both end a word with e; one was edited"
    );
    assert_eq!(
        w.texts("pie OR pear OR bramley sort:document"),
        ["Bramley", "apple pie", "Pear"]
    );
    // An OR ranks the cells that have more of the words first.
    assert_eq!(w.texts("green OR apple")[..2], ["green apple", "Apple"]);
    // Pages are parts of the same list.
    let all = w.find("e");
    let index = w.index();
    let query = parse("e", NOW).unwrap().query;
    let mut paged = Vec::new();
    for offset in (0..all.len() + 3).step_by(3) {
        let options = Options {
            order: Order::Relevance,
            offset,
            limit: 3,
        };
        let page = index.search(&query, &options).unwrap();
        assert_eq!(page.total, all.len());
        paged.extend(page.hits);
    }
    assert_eq!(paged, all);
    let none = Options {
        order: Order::Relevance,
        offset: 0,
        limit: 0,
    };
    assert_eq!(index.search(&query, &none).unwrap().hits.len(), 0);
}

#[test]
fn a_snippet_is_a_part_around_the_first_match() {
    let long: String = (0..40).map(|n| format!("word{n} ")).collect();
    let text = format!("{long}NEEDLE👨\u{200d}👩\u{200d}👧 e\u{301}\ttail {long}");
    let at = text.chars().position(|c| c == 'N').unwrap() as u32;
    let snippet = wombat_search::engine::snippet(&text, &[(at, 6), (at + 6, 5)], 80);
    assert!(snippet.cut_start && snippet.cut_end);
    assert_eq!(snippet.text.graphemes_count(), 80);
    let chars: Vec<char> = snippet.text.chars().collect();
    let (start, len) = snippet.ranges[0];
    let marked: String = chars[start as usize..(start + len) as usize]
        .iter()
        .collect();
    assert_eq!(marked, "NEEDLE");
    assert_eq!(
        start, 20,
        "a quarter of the snippet comes before the first match"
    );
    assert_eq!(snippet.ranges[1].1, 5, "the family is five code points");
    assert!(!snippet.text.contains('\t'));
    // Short texts whole; a match at the very end; no match at all; an empty text.
    let whole = wombat_search::engine::snippet("short", &[(1, 2)], 80);
    assert_eq!(
        (whole.text.as_str(), whole.cut_start, whole.cut_end),
        ("short", false, false)
    );
    let tail = wombat_search::engine::snippet(&text, &[(text.chars().count() as u32 - 2, 2)], 80);
    assert!(tail.cut_start && !tail.cut_end && tail.text.ends_with("word39 "));
    assert_eq!(tail.ranges, [(tail.text.chars().count() as u32 - 2, 2)]);
    let head = wombat_search::engine::snippet(&text, &[], 80);
    assert!(!head.cut_start && head.cut_end && head.text.starts_with("word0 "));
    assert_eq!(
        wombat_search::engine::snippet("", &[], 80),
        Default::default()
    );
    // A range that the snippet shows half of is cut to what is shown.
    let cut = wombat_search::engine::snippet(&text, &[(0, 500)], 80);
    assert_eq!(cut.ranges, [(0, 80)]);
}

trait Graphemes {
    fn graphemes_count(&self) -> usize;
}

impl Graphemes for String {
    fn graphemes_count(&self) -> usize {
        (0..=self.chars().count() as u32)
            .filter(|at| wombat_core::text::is_boundary(self, *at) == Some(true))
            .count()
            - 1
    }
}

#[test]
fn the_text_syntax_reads_as_documented() {
    let term = |text: &str| Query::Term(Term::new(text));
    let read = |text: &str| parse(text, NOW).unwrap().query;
    assert_eq!(read(""), Query::All);
    assert_eq!(read("   "), Query::All);
    assert_eq!(read("apple"), term("apple"));
    assert_eq!(
        read("apple pie"),
        Query::And(vec![term("apple"), term("pie")])
    );
    assert_eq!(
        read("\"apple pie\" x"),
        Query::And(vec![term("apple pie"), term("x")])
    );
    assert_eq!(
        read("a OR b c"),
        Query::Or(vec![term("a"), Query::And(vec![term("b"), term("c")])])
    );
    assert_eq!(read("a | b"), Query::Or(vec![term("a"), term("b")]));
    assert_eq!(
        read("a or b"),
        Query::And(vec![term("a"), term("or"), term("b")])
    );
    assert_eq!(read("-a"), Query::Not(Box::new(term("a"))));
    assert_eq!(
        read("--a"),
        Query::Not(Box::new(Query::Not(Box::new(term("a")))))
    );
    assert_eq!(
        read("e-mail - x"),
        Query::And(vec![term("e-mail"), term("-"), term("x")])
    );
    assert_eq!(
        read("(a OR b) -(c d)"),
        Query::And(vec![
            Query::Or(vec![term("a"), term("b")]),
            Query::Not(Box::new(Query::And(vec![term("c"), term("d")]))),
        ])
    );
    assert_eq!(read("case:Apple"), Query::Term(Term::exact("Apple")));
    assert_eq!(
        read("case:\"Apple Pie\""),
        Query::Term(Term::exact("Apple Pie"))
    );
    assert_eq!(
        read("tag:\"two words\""),
        Query::Filter(Filter::Tag("two words".into()))
    );
    assert_eq!(read("TAG:x"), Query::Filter(Filter::Tag("x".into())));
    assert_eq!(read("has:Note"), Query::Filter(Filter::Has(Has::Note)));
    assert_eq!(
        read("\"has:note\""),
        term("has:note"),
        "in quotes it is a word"
    );
    assert_eq!(read("http://x.org/a?b=c"), term("http://x.org/a?b=c"));
    assert_eq!(read("12:30"), term("12:30"));
    assert_eq!(
        read("text:\"\""),
        Query::Filter(Filter::TextIs(String::new()))
    );
    // Left open while typing: closed at the end.
    assert_eq!(read("\"apple pi"), term("apple pi"));
    assert_eq!(read("(a OR b"), Query::Or(vec![term("a"), term("b")]));
    assert_eq!(read("a) b"), Query::And(vec![term("a"), term("b")]));
    assert_eq!(read("a OR"), term("a"));
    assert_eq!(read("OR a"), term("a"));
    assert_eq!(read("()"), Query::All);
    assert_eq!(read("-"), term("-"));
    assert_eq!(
        read("a\tb\nc"),
        Query::And(vec![term("a"), term("b"), term("c")])
    );
    // The order.
    assert_eq!(
        parse("a sort:recent", NOW).unwrap().order,
        Some(Order::Recent)
    );
    assert_eq!(parse("a", NOW).unwrap().order, None);
    // Times.
    let edited = |text: &str| match read(text) {
        Query::Filter(Filter::Edited { from, to }) => (from, to),
        other => panic!("{other:?}"),
    };
    assert_eq!(edited("edited:7d"), (Some(NOW - 7 * DAY), None));
    assert_eq!(edited("edited:90m"), (Some(NOW - 90 * 60_000), None));
    assert_eq!(edited("edited:1970-01-02"), (Some(DAY), Some(2 * DAY - 1)));
    assert_eq!(
        edited("edited:2024-02-29..2024-03-01").0,
        Some(19_782 * DAY)
    );
    assert_eq!(edited("edited:2000-01-01..").1, None);
    // A filter with a value that means nothing is an error that says what would.
    for bad in [
        "has:wings",
        "kind:banana",
        "bg:red",
        "style:loud",
        "in:here",
        "links-to:x",
        "tag:",
        "edited:soon",
        "edited:2023-02-29",
        "edited:2023-13-01",
        "edited:..",
        "recent:0%",
        "recent:100",
        "sort:sideways",
        "edited:99999999999999999999d",
    ] {
        let error = parse(bad, NOW).unwrap_err().to_string();
        assert!(
            error.contains(bad.split(':').next().unwrap()),
            "{bad}: {error}"
        );
    }
}

#[test]
fn a_query_too_large_is_refused_and_none_can_take_long() {
    use wombat_search::query::{MAX_NESTING, MAX_PARTS, MAX_QUERY_BYTES, MAX_TERM_BYTES};
    let w = world();
    let index = w.index();
    assert!(parse(&"a".repeat(MAX_QUERY_BYTES + 1), NOW).is_err());
    assert!(parse(&"a ".repeat(MAX_PARTS + 1), NOW).is_err());
    assert!(parse(&"a ".repeat(MAX_PARTS), NOW).is_ok());
    assert!(parse(&"(".repeat(MAX_NESTING + 2), NOW).is_err());
    assert!(parse(&"-".repeat(MAX_NESTING + 2), NOW).is_err());
    assert!(parse(&format!("{}a", "-".repeat(MAX_NESTING + 2)), NOW).is_err());
    assert!(parse(&format!("\"{}\"", "a".repeat(MAX_TERM_BYTES + 1)), NOW).is_err());
    // Built by hand, the same limits hold when the search starts.
    let options = Options::default();
    let long = Query::Term(Term::new("a".repeat(MAX_TERM_BYTES + 1)));
    assert!(index.search(&long, &options).is_err());
    let mut deep = Query::All;
    for _ in 0..MAX_NESTING + 2 {
        deep = Query::Not(Box::new(deep));
    }
    assert!(index.search(&deep, &options).is_err());
    assert!(naive::search(&w.docs(), &deep, &options).is_err());
    let many = Query::Or(vec![Query::Term(Term::new("a")); MAX_PARTS + 1]);
    assert!(index.search(&many, &options).is_err());
    assert!(index
        .search(&Query::Filter(Filter::Recent(0)), &options)
        .is_err());
    // The largest queries that are allowed, of the kinds that could be slow: they end.
    let started = std::time::Instant::now();
    let texts = [
        "a ".repeat(MAX_PARTS),
        format!("{}b", "a".repeat(MAX_TERM_BYTES - 1)),
        "a OR ".repeat(MAX_PARTS - 1) + "a",
        "\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaab\" ".repeat(MAX_PARTS),
        "(((((((((((((((a)))))))))))))))".to_owned(),
        "-a ".repeat(MAX_PARTS),
        ".* (a+)+$ [^x]* \\ %s {} '; --".to_owned(),
    ];
    for text in texts {
        let query = parse(&text, NOW).unwrap().query;
        let found = index
            .search(&query, &Options::all(Order::Relevance))
            .unwrap();
        let expected = naive::search(&w.docs(), &query, &Options::all(Order::Relevance)).unwrap();
        assert_eq!(found, expected);
    }
    assert!(started.elapsed() < std::time::Duration::from_secs(20));
}

#[test]
fn next_and_previous_match_go_round_the_document() {
    let w = world();
    let index = w.index();
    let query = parse("apple", NOW).unwrap().query;
    let shop = w.shop.id();
    let next = |from: Option<CellId>, reverse: bool| {
        let hit = index.next_match(&query, shop, from, reverse).unwrap();
        let expected = naive::next_match(&w.docs(), &query, shop, from, reverse).unwrap();
        assert_eq!(hit, expected);
        hit.map(|hit| hit.text)
    };
    // In the order of the document: Apple, (Bramley,) green apple, (todo,) Pineapple...
    assert_eq!(next(None, false).unwrap(), "Apple");
    assert_eq!(next(Some(w.fruit[0]), false).unwrap(), "green apple");
    assert_eq!(next(Some(w.apple_kinds[0]), false).unwrap(), "green apple");
    assert_eq!(next(Some(w.apple_kinds[1]), false).unwrap(), "Pineapple");
    assert_eq!(
        next(Some(w.fruit[4]), false).unwrap(),
        "Apple",
        "round to the start"
    );
    assert_eq!(
        next(Some(w.fruit[0]), true).unwrap(),
        "Pear",
        "round to the end"
    );
    assert_eq!(next(Some(w.fruit[1]), true).unwrap(), "green apple");
    assert_eq!(next(None, true).unwrap(), "Pear");
    // One match: that one again. None: none. Only the document asked about.
    let one = parse("bramley", NOW).unwrap().query;
    let hit = index
        .next_match(&one, shop, Some(w.apple_kinds[0]), false)
        .unwrap();
    assert_eq!(hit.unwrap().cell, w.apple_kinds[0]);
    let none = parse("zebra", NOW).unwrap().query;
    assert_eq!(index.next_match(&none, shop, None, false).unwrap(), None);
    let other = parse("fruit", NOW).unwrap().query;
    assert_eq!(index.next_match(&other, shop, None, false).unwrap(), None);
    // TreeSheets' "go to matching cell": the next cell with the same text, or image.
    let same = Query::Filter(Filter::TextIs("todo".into()));
    let hit = index
        .next_match(&same, shop, Some(w.apple_kinds[2]), false)
        .unwrap()
        .unwrap();
    assert_eq!(hit.cell, w.fruit[5]);
    let hit = index
        .next_match(&same, shop, Some(w.fruit[5]), false)
        .unwrap()
        .unwrap();
    assert_eq!(hit.cell, w.apple_kinds[2]);
    let image = Query::Filter(Filter::Image("Apple-Photo.png".into()));
    let hit = index.next_match(&image, shop, None, true).unwrap().unwrap();
    assert_eq!(hit.cell, w.fruit[1]);
}

#[test]
fn the_index_follows_edits_and_trails_follow_moved_cells() {
    let w = world();
    let mut index = w.index();
    let mut editor = Editor::new(w.shop.clone()).unwrap();
    let mut ids = SeededIds::from_u64(99);
    let mut apply = |editor: &mut Editor, index: &mut SearchIndex, command: Command| {
        let changes = editor.apply(command, Some(NOW), &mut ids).unwrap();
        index.apply(editor.document(), editor.calc(), &changes);
    };
    let green = parse("green", NOW).unwrap().query;
    let trail = |index: &SearchIndex| {
        let found = index.search(&green, &Options::default()).unwrap();
        found.hits[0].trail.clone()
    };
    assert_eq!(trail(&index), ["Shop", "Apple"]);
    // Renaming a cell above changes the trail of what is inside it, though the report
    // names only the renamed cell: trails are worked out when results are made.
    apply(
        &mut editor,
        &mut index,
        Command::SetText {
            cell: w.fruit[0],
            text: "Malus".into(),
        },
    );
    assert_eq!(trail(&index), ["Shop", "Malus"]);
    // Moving the cell with everything in it: the trail of what is inside follows, and
    // so do searches within a branch.
    apply(
        &mut editor,
        &mut index,
        Command::CreateGrid {
            cell: w.fruit[4],
            xs: 1,
            ys: 1,
        },
    );
    let hole = editor.document().child(w.fruit[4], 0, 0).unwrap();
    apply(
        &mut editor,
        &mut index,
        Command::MoveCell {
            from: w.fruit[0],
            to: hole,
        },
    );
    assert_eq!(trail(&index), ["Shop", "Pear", "Malus"]);
    let inside = Query::And(vec![
        green.clone(),
        Query::Filter(Filter::Within(w.fruit[4])),
    ]);
    assert_eq!(index.search(&inside, &Options::default()).unwrap().total, 1);
    // What it was can no longer be found; what it is can.
    let count = |index: &SearchIndex, text: &str| {
        let query = parse(text, NOW).unwrap().query;
        index.search(&query, &Options::default()).unwrap().total
    };
    assert_eq!(count(&index, "text:Apple"), 0);
    // (Twice: the cell itself, and the cell of the other document that links to it,
    // which was not edited and is found by the text its link now leads to.)
    assert_eq!(count(&index, "malus"), 2);
    assert_eq!(
        count(&index, "edited:1m"),
        3,
        "the renamed cell, the new cell, and one that was edited at that time already"
    );
    // Undo everything: as at the start, and equal to an index made afresh.
    while let Some(changes) = editor.undo().unwrap() {
        index.apply(editor.document(), editor.calc(), &changes);
    }
    assert_eq!(trail(&index), ["Shop", "Apple"]);
    assert_eq!(index.encoded(), w.index().encoded());
    index.check().unwrap();
    // A document that is closed leaves; links into it are then broken.
    assert!(index.remove(w.shop.id()));
    assert_eq!(count(&index, "apple"), 0);
    assert_eq!(count(&index, "has:broken-link"), 2);
}

/// A formula cell is searched by its formula (its text) and, as a field of its own, by
/// the result it shows, which is worked out for a document that nobody edits.
#[test]
fn a_displayed_result_is_searched_as_a_field_of_its_own() {
    let mut w = world();
    let pie = w.fruit[2];
    assert_eq!(w.shop.cell(pie).unwrap().kind, CellKind::Formula);
    w.shop.cell_mut(pie).unwrap().text = "='Total: ' & 6 * 7 & ' Äpfel'".into();
    w.calcs[0] = Calc::new(&w.shop);
    let summary = <DocIndex as Summary>::of(&w.shop);
    assert!(summary.encode() == DocIndex::with(&w.shop, &w.calcs[0]).encode());
    // It survives the cache.
    let bytes = summary.encode();
    let back = DocIndex::decode(&bytes).unwrap();
    assert_eq!(back.encode(), bytes);
    let mut index = w.index();
    index.insert(back, "shop.wombat");
    index.check().unwrap();
    assert_eq!(index.encoded(), w.index().encoded());
    let find = |text: &str| w.find(text);
    let hits = find("42");
    assert_eq!(hits.len(), 1);
    assert_eq!(
        (hits[0].text.as_str(), hits[0].field),
        ("='Total: ' & 6 * 7 & ' Äpfel'", Field::Shown)
    );
    assert_eq!(hits[0].shown.as_deref(), Some("Total: 42 Äpfel"));
    assert_eq!(hits[0].matched, "Total: 42 Äpfel");
    assert_eq!(hits[0].ranges, [(7, 2)]);
    assert_eq!(
        hits[0].score,
        4 + 1,
        "a displayed result counts just after the text"
    );
    // One word in the formula and one only in the result: both count, and the hit
    // shows the better field.
    let hits = find("apfel 42");
    assert_eq!(hits.len(), 1);
    assert_eq!((hits[0].field, hits[0].score), (Field::Text, 1 + (4 + 1)));
    // The formula is still found, and ranks before the result.
    assert_eq!(find("total")[0].field, Field::Text);
    assert_eq!(find("total")[0].ranges, [(2, 5)]);
    assert_eq!(find("case:Total").len(), 1);
    // What is only in the result is found there; what is only in the formula, there.
    assert_eq!(find("\"Total: 42\"")[0].field, Field::Shown);
    assert_eq!(find("\"6 * 7\"")[0].field, Field::Text);
    assert!(find("kind:formula")[0].shown.is_some());
}

#[test]
fn arbitrary_documents_agree_with_the_reference_for_handwritten_queries() {
    let w = world();
    let index = w.index();
    let docs = w.docs();
    let mut rng = Rng(0x5eed);
    let mut stats = search_support::Compared::default();
    let reference = naive::Reference::new(&docs);
    for _ in 0..1500 {
        let query = search_support::query(&mut rng, &docs, 0);
        search_support::compare(
            &[&index],
            &reference,
            &docs,
            &query,
            &mut rng,
            "nothing",
            &mut stats,
        );
    }
    assert!(
        stats.selective > 500 && stats.by_linked_text > 10,
        "{stats:?}"
    );
}

/// Found by the random tests: the fields are looked at in the order of their points, and
/// a first version had the image name and the link targets the wrong way round there.
#[test]
fn a_word_in_several_fields_counts_for_the_best_of_them() {
    let mut ids = SeededIds::from_u64(4);
    let mut doc = Document::new(&mut ids);
    let root = doc.root();
    let cells = column(
        &mut doc,
        &mut ids,
        root,
        &["plain", "report", "x", "y", "z"],
    );
    let url = |end: &str| LinkTarget::External(format!("https://example.org/{end}"));
    // Image and link: the image name counts (4 x 4 + a whole word).
    doc.cell_mut(cells[2]).unwrap().image = Some(ImageRef::new("report.png"));
    link(&mut doc, cells[2], 0, 1, url("a-report"));
    // Note, image and link: the note. Two links: the better of the two.
    doc.cell_mut(cells[3]).unwrap().note = "the reports".into();
    doc.cell_mut(cells[3]).unwrap().image = Some(ImageRef::new("report.png"));
    link(&mut doc, cells[3], 0, 1, url("report"));
    link(&mut doc, cells[4], 0, 1, url("unreported"));
    doc.cell_mut(cells[4]).unwrap().text = "zz".into();
    link(
        &mut doc,
        cells[4],
        1,
        1,
        LinkTarget::External("report".into()),
    );
    doc.validate().unwrap();
    let mut index = SearchIndex::new();
    index.insert_document(&doc, &Calc::new(&doc), "d.wombat");
    let query = parse("report", NOW).unwrap().query;
    let options = Options::all(Order::Document);
    let found = index.search(&query, &options).unwrap();
    assert_eq!(
        found,
        naive::search(&[("d.wombat", &doc, &Calc::new(&doc))], &query, &options).unwrap()
    );
    let seen: Vec<(Field, u32)> = found
        .hits
        .iter()
        .map(|hit| (hit.field, hit.score))
        .collect();
    assert_eq!(
        seen,
        [
            (Field::Text, 0),
            (Field::Image, 4 * 4 + 1),
            (Field::Note, 2 * 4 + 2),
            (Field::Url, 5 * 4),
        ]
    );
    // What is shown is the first link target that has the word; the points are for the
    // best one.
    assert_eq!(found.hits[3].matched, "https://example.org/unreported");
}
