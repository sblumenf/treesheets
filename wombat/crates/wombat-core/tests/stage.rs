//! What the first screen of an interface reads: `Editor::stage_view` (the stage grid, the
//! trail, and the grids nested below in full and in reduced form) and `Editor::cell_look`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use std::time::{Duration, Instant};

use proptest::prelude::*;
use support::{command, no_such_cell, op, Op};
use wombat_core::construct::Construct;
use wombat_core::model::Extra;
use wombat_core::text::{floor_boundary, is_boundary};
use wombat_core::{
    CellId, CellKind, CellLook, Color, Command, CutText, Document, DrawStyle, Editor, Error,
    GridSummary, ImageRef, Link, LinkTarget, Run, SeededIds, StageAsk, StageView, Style, Tag,
    TextAlign, VertAlign,
};

/// The whole stage grid and nothing nested cell by cell; texts whole, previews of 3 by 4.
const ALL: StageAsk = StageAsk {
    x0: 0,
    y0: 0,
    cols: u32::MAX,
    rows: u32::MAX,
    full_depth: 0,
    full_cells: u32::MAX,
    preview_cols: 3,
    preview_rows: 4,
    shape_cols: 12,
    shape_rows: 20,
    text_cut: u32::MAX,
    preview_cut: 40,
};

const RED: Color = Color::rgb(200, 0, 0);
const GREEN: Color = Color::rgb(0, 200, 0);
const BLUE: Color = Color::rgb(0, 0, 200);
const BOLD: Style = Style {
    bold: true,
    italic: false,
    fixed: false,
    underline: false,
    strike: false,
};

fn whole(text: &str) -> CutText {
    CutText {
        text: text.into(),
        cut: false,
    }
}

fn part(text: &str) -> CutText {
    CutText {
        text: text.into(),
        cut: true,
    }
}

fn run(start: u32, len: u32, style: Style, color: Option<Color>) -> Run {
    Run {
        start,
        len,
        style,
        color,
        extra: Extra::new(),
    }
}

fn link(start: u32, len: u32, to: &str) -> Link {
    Link {
        start,
        len,
        target: LinkTarget::External(to.into()),
        extra: Extra::new(),
    }
}

/// A document under construction, with the IDs it takes new cells from.
struct Build {
    doc: Document,
    ids: SeededIds,
}

impl Build {
    fn new(seed: u64) -> Build {
        let mut ids = SeededIds::from_u64(seed);
        Build {
            doc: Document::new(&mut ids),
            ids,
        }
    }

    /// Gives `cell` a grid whose cells have the texts `text(x, y)`, and returns them.
    fn grid(&mut self, cell: CellId, xs: u32, ys: u32, text: impl Fn(u32, u32) -> String) {
        self.doc.add_grid(cell, xs, ys, &mut self.ids).unwrap();
        for y in 0..ys {
            for x in 0..xs {
                self.at(cell, x, y).text = text(x, y);
            }
        }
    }

    fn id(&self, cell: CellId, x: u32, y: u32) -> CellId {
        self.doc.child(cell, x, y).unwrap()
    }

    fn at(&mut self, cell: CellId, x: u32, y: u32) -> &mut wombat_core::Cell {
        let id = self.id(cell, x, y);
        self.doc.cell_mut(id).unwrap()
    }

    fn editor(self) -> Editor {
        Editor::new(self.doc).unwrap()
    }
}

fn texts(view: &StageView) -> Vec<&str> {
    view.cells.iter().map(|c| c.text.text.as_str()).collect()
}

fn previews(summary: &GridSummary) -> Vec<&str> {
    summary.preview.iter().map(|t| t.text.as_str()).collect()
}

#[test]
fn a_stage_of_plain_cells_gives_each_cell_as_it_is_drawn() {
    let mut b = Build::new(1);
    let root = b.doc.root();
    b.grid(root, 3, 2, |x, y| format!("c{x}{y}"));
    {
        let top = b.doc.cell_mut(root).unwrap();
        top.text = "Home".into();
        top.cell_color = GREEN;
        top.draw_style = DrawStyle::Lines;
        let grid = top.grid_mut().unwrap();
        grid.col_widths = vec![80, 40, 120];
        grid.border_color = BLUE;
        grid.outer_spacing = 5;
        grid.vertical = false;
    }
    {
        let cell = b.at(root, 1, 0);
        cell.cell_color = RED;
        cell.text_color = BLUE;
        cell.style = BOLD;
        cell.rel_size = -2;
        cell.text_align = TextAlign::Center;
        cell.vert_align = VertAlign::Bottom;
        cell.note = "a note".into();
        cell.image = Some(ImageRef::new("photo.png"));
    }
    let editor = b.editor();
    let doc = editor.document();
    let view = editor.stage_view(root, &ALL).unwrap();

    assert_eq!(view.trail.len(), 1);
    assert_eq!(view.trail[0].id, root);
    assert_eq!(view.trail[0].shown, whole("Home"));
    assert_eq!(view.trail[0].color, GREEN);

    assert_eq!(view.grids.len(), 1);
    let grid = &view.grids[0];
    assert_eq!((grid.owner, grid.level), (root, 0));
    assert_eq!((grid.xs, grid.ys), (3, 2));
    assert_eq!((grid.x0, grid.y0, grid.cols, grid.rows), (0, 0, 3, 2));
    assert_eq!(grid.first, 0);
    assert_eq!(grid.col_widths, [80, 40, 120]);
    assert_eq!(grid.border_color, BLUE);
    assert_eq!(grid.outer_spacing, 5);
    assert!(!grid.vertical);
    assert_eq!(grid.draw_style, DrawStyle::Lines);

    // Row by row.
    assert_eq!(texts(&view), ["c00", "c10", "c20", "c01", "c11", "c21"]);
    let ids: Vec<_> = view.cells.iter().map(|c| c.id).collect();
    assert_eq!(ids, doc.cell(root).unwrap().grid().unwrap().cells());
    assert!(view.summaries.is_empty());

    let plain = &view.cells[0];
    assert_eq!(plain.text, whole("c00"));
    assert_eq!(plain.kind, CellKind::Data);
    assert_eq!((plain.result.clone(), plain.failed), (None, false));
    assert_eq!(
        (plain.cell_color, plain.text_color),
        (Color::WHITE, Color::BLACK)
    );
    assert!(!plain.is_tag && !plain.has_note && !plain.has_grid);
    assert_eq!((plain.style, plain.rel_size), (Style::default(), 0));
    assert_eq!(
        (plain.text_align, plain.vert_align),
        (TextAlign::Auto, VertAlign::Auto)
    );
    assert!(plain.runs.is_empty() && plain.links.is_empty());
    assert_eq!(
        (plain.image.clone(), plain.summary, plain.full),
        (None, None, None)
    );

    let dressed = &view.cells[1];
    assert_eq!((dressed.cell_color, dressed.text_color), (RED, BLUE));
    assert_eq!((dressed.style, dressed.rel_size), (BOLD, -2));
    assert_eq!(
        (dressed.text_align, dressed.vert_align),
        (TextAlign::Center, VertAlign::Bottom)
    );
    assert!(dressed.has_note);
    assert_eq!(dressed.image.as_deref(), Some("photo.png"));

    // The same request gives the same answer.
    assert_eq!(editor.stage_view(root, &ALL).unwrap(), view);
}

/// An editor on a root of 3 by 3 made by commands, with these texts entered in its first
/// cells row by row.
fn entered(texts: &[&str]) -> Editor {
    let mut ids = SeededIds::from_u64(2);
    let doc = Document::new(&mut ids);
    let root = doc.root();
    let mut editor = Editor::new(doc).unwrap();
    let grid = Command::CreateGrid {
        cell: root,
        xs: 3,
        ys: 3,
    };
    editor.apply(grid, None, &mut ids).unwrap();
    for (n, text) in texts.iter().enumerate() {
        let cell = editor.document().child(root, n as u32 % 3, n as u32 / 3);
        let enter = Command::EnterText {
            cell: cell.unwrap(),
            text: text.to_string(),
        };
        editor.apply(enter, None, &mut ids).unwrap();
    }
    editor
}

#[test]
fn a_formula_cell_gives_its_formula_and_what_it_shows() {
    let editor = entered(&["2", "=20+22", "=nosuchfunction(1)", "=100000*100000"]);
    let root = editor.document().root();
    let view = editor.stage_view(root, &ALL).unwrap();

    let data = &view.cells[0];
    assert_eq!((data.kind, data.result.clone()), (CellKind::Data, None));

    let formula = &view.cells[1];
    assert_eq!(formula.kind, CellKind::Formula);
    assert_eq!(formula.text, whole("=20+22"));
    assert_eq!(formula.result, Some(whole("42")));
    assert!(!formula.failed);

    let broken = &view.cells[2];
    assert_eq!(broken.text, whole("=nosuchfunction(1)"));
    assert!(broken.failed);
    let shown = editor.shown(broken.id).unwrap();
    assert_eq!(broken.result, Some(whole(shown)));
    assert!(shown.starts_with('#'), "{shown}");

    // The result is cut like the text.
    let ask = StageAsk { text_cut: 4, ..ALL };
    let view = editor.stage_view(root, &ask).unwrap();
    assert_eq!(view.cells[1].text, part("=20+"));
    assert_eq!(view.cells[1].result, Some(whole("42")));
    assert_eq!(view.cells[3].text, part("=100"));
    assert_eq!(view.cells[3].result, Some(part("1000")));
    assert_eq!(editor.shown(view.cells[3].id), Some("10000000000"));

    // The trail and the editor's look of a formula cell.
    let view = editor.stage_view(view.cells[1].id, &ALL).unwrap();
    assert_eq!(view.trail[1].shown, whole("42"));
    let look = editor.cell_look(view.trail[1].id).unwrap();
    assert_eq!(look.text, whole("=20+22"));
    assert_eq!(look.result, Some(whole("42")));
}

#[test]
fn runs_and_links_end_where_the_text_is_cut() {
    let mut b = Build::new(3);
    let root = b.doc.root();
    b.grid(root, 2, 1, |_, _| String::new());
    {
        let cell = b.at(root, 0, 0);
        cell.text = "abcdefghij".into();
        cell.runs = vec![run(2, 6, BOLD, Some(RED))];
        cell.links = vec![link(4, 5, "https://example.org")];
    }
    {
        // b, a family of five code points, c: a cut inside the family moves before it.
        let cell = b.at(root, 1, 0);
        cell.text = "ab\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}cd".into();
        cell.runs = vec![run(1, 7, BOLD, None)];
        cell.links = vec![link(2, 5, "x"), link(8, 1, "y")];
    }
    let editor = b.editor();
    let at = |text_cut: u32| {
        let ask = StageAsk { text_cut, ..ALL };
        editor.stage_view(root, &ask).unwrap().cells
    };

    let cells = at(100);
    assert_eq!(cells[0].text, whole("abcdefghij"));
    assert_eq!(cells[0].runs, [run(2, 6, BOLD, Some(RED))]);
    assert_eq!(cells[0].links, [link(4, 5, "https://example.org")]);
    assert_eq!(cells[1].links, [link(2, 5, "x"), link(8, 1, "y")]);

    // In the middle of the run and of the link.
    let cells = at(6);
    assert_eq!(cells[0].text, part("abcdef"));
    assert_eq!(cells[0].runs, [run(2, 4, BOLD, Some(RED))]);
    assert_eq!(cells[0].links, [link(4, 2, "https://example.org")]);
    // Inside the family: the text ends before it, and so do the run and the links.
    assert_eq!(cells[1].text, part("ab"));
    assert_eq!(cells[1].runs, [run(1, 1, BOLD, None)]);
    assert_eq!(cells[1].links, []);

    // Where the link starts: nothing of it is left.
    let cells = at(4);
    assert_eq!(cells[0].runs, [run(2, 2, BOLD, Some(RED))]);
    assert_eq!(cells[0].links, []);
    let cells = at(2);
    assert_eq!(cells[0].text, part("ab"));
    assert!(cells[0].runs.is_empty() && cells[0].links.is_empty());
    // Just past the family.
    let cells = at(7);
    assert_eq!(cells[1].text.text.chars().count(), 7);
    assert_eq!(cells[1].runs, [run(1, 6, BOLD, None)]);
    assert_eq!(cells[1].links, [link(2, 5, "x")]);

    let cells = at(0);
    assert_eq!(cells[0].text, part(""));
    assert!(cells[0].runs.is_empty() && cells[0].links.is_empty());

    // The editor's look cuts nothing.
    let look = editor.cell_look(cells[1].id).unwrap();
    assert_eq!(look.text.text.chars().count(), 9);
    assert!(!look.text.cut);
    assert_eq!(look.runs, [run(1, 7, BOLD, None)]);
    assert_eq!(look.links.len(), 2);
}

#[test]
fn the_look_an_editor_opens_on_has_the_whole_text() {
    let text = "\u{e9}".repeat(70_000);
    let mut b = Build::new(13);
    let root = b.doc.root();
    b.grid(root, 1, 1, |_, _| "\u{e9}".repeat(70_000));
    let id = b.id(root, 0, 0);
    b.at(root, 0, 0).runs = vec![run(69_990, 10, BOLD, None)];
    b.at(root, 0, 0).links = vec![link(69_999, 1, "end")];
    b.grid(id, 1, 1, |_, _| String::new());
    let editor = b.editor();
    let look = editor.cell_look(id).unwrap();
    assert_eq!(look.text, whole(&text));
    assert_eq!(look.runs, [run(69_990, 10, BOLD, None)]);
    assert_eq!(look.links, [link(69_999, 1, "end")]);
    // It says that there is a grid, and nothing of where a stage view would have it.
    assert!(look.has_grid);
    assert_eq!((look.summary, look.full), (None, None));

    // The stage has the beginning.
    let view = editor.stage_view(root, &StageAsk::default()).unwrap();
    let cell = &view.cells[0];
    assert_eq!(cell.text, part(&text[..4000]));
    assert!(cell.runs.is_empty() && cell.links.is_empty());
    assert_eq!(
        (cell.has_grid, cell.summary, cell.full),
        (true, Some(0), Some(1))
    );
    let below = editor.stage_view(id, &ALL).unwrap();
    assert_eq!(below.trail[1].shown, part(&text[..80]));
}

#[test]
fn a_cell_whose_text_is_a_tag_is_drawn_in_the_tags_colours() {
    let mut b = Build::new(4);
    let root = b.doc.root();
    b.grid(root, 3, 1, |_, _| String::new());
    let tag = Tag {
        cell_color: GREEN,
        text_color: BLUE,
        extra: Extra::new(),
    };
    b.doc.tags_mut().insert("todo".into(), tag);
    for (x, text) in ["todo", "todo ", "Todo"].iter().enumerate() {
        let cell = b.at(root, x as u32, 0);
        cell.text = text.to_string();
        cell.cell_color = RED;
        cell.text_color = Color::rgb(1, 2, 3);
        // A coloured plain range, then a bold coloured one, then a bold one.
        cell.runs = vec![
            run(0, 1, Style::default(), Some(RED)),
            run(1, 1, BOLD, Some(RED)),
            run(2, 1, BOLD, None),
        ];
    }
    let tagged = b.id(root, 0, 0);
    b.grid(tagged, 1, 1, |_, _| "inside".into());
    let editor = b.editor();
    let view = editor.stage_view(root, &ALL).unwrap();

    let cell = &view.cells[0];
    assert!(cell.is_tag);
    assert_eq!((cell.cell_color, cell.text_color), (GREEN, BLUE));
    // The tag's text colour is the only one: the runs keep their styles alone.
    assert_eq!(cell.runs, [run(1, 2, BOLD, None)]);
    assert_eq!(editor.cell_look(cell.id).unwrap().runs, cell.runs);
    assert_eq!(editor.cell_look(cell.id).unwrap().cell_color, GREEN);

    // Only the whole text is a tag, exactly.
    for other in &view.cells[1..] {
        assert!(!other.is_tag);
        assert_eq!(other.cell_color, RED);
        assert_eq!(other.text_color, Color::rgb(1, 2, 3));
        assert_eq!(other.runs.len(), 3);
        assert_eq!(other.runs[0].color, Some(RED));
    }

    // A cut text is still the tag that its whole text names.
    let ask = StageAsk { text_cut: 2, ..ALL };
    let cut = editor.stage_view(root, &ask).unwrap();
    assert_eq!(cut.cells[0].text, part("to"));
    assert!(cut.cells[0].is_tag);
    assert_eq!(cut.cells[0].cell_color, GREEN);

    // The trail and a shape show the colour the cell is drawn in.
    let below = editor.stage_view(tagged, &ALL).unwrap();
    assert_eq!(below.trail[1].color, GREEN);
    let mut b = Build::new(6);
    let root = b.doc.root();
    b.grid(root, 1, 1, |_, _| String::new());
    let holder = b.id(root, 0, 0);
    b.grid(holder, 2, 1, |x, _| ["todo", "to"][x as usize].into());
    b.at(holder, 0, 0).cell_color = RED;
    b.at(holder, 1, 0).cell_color = RED;
    let tag = Tag {
        cell_color: GREEN,
        text_color: BLUE,
        extra: Extra::new(),
    };
    b.doc.tags_mut().insert("todo".into(), tag);
    let editor = b.editor();
    let view = editor.stage_view(root, &ALL).unwrap();
    assert_eq!(view.summaries[0].shape, [0, 200, 0, 4, 200, 0, 0, 2]);
}

/// Root 2 by 2. Its first cell A holds 3 by 2; A's cell (1, 0), B, holds 2 by 2; B's cell
/// (0, 1), C, holds 1 by 5. The root's last cell D holds 3 by 3. Returns (A, B, C, D).
fn nested() -> (Editor, [CellId; 4]) {
    let mut b = Build::new(7);
    let root = b.doc.root();
    b.grid(root, 2, 2, |x, y| format!("r{x}{y}"));
    let a = b.id(root, 0, 0);
    b.grid(a, 3, 2, |x, y| format!("a{x}{y}"));
    let bb = b.id(a, 1, 0);
    b.grid(bb, 2, 2, |x, y| format!("b{x}{y}"));
    let c = b.id(bb, 0, 1);
    b.grid(c, 1, 5, |x, y| format!("c{x}{y}"));
    let d = b.id(root, 1, 1);
    b.grid(d, 3, 3, |x, y| format!("d{x}{y}"));
    (b.editor(), [a, bb, c, d])
}

/// The owners of the grids, and of the summaries.
fn owners(view: &StageView) -> (Vec<CellId>, Vec<CellId>) {
    (
        view.grids.iter().map(|g| g.owner).collect(),
        view.summaries.iter().map(|s| s.owner).collect(),
    )
}

#[test]
fn nested_grids_are_given_cell_by_cell_as_deep_and_as_large_as_asked() {
    let (editor, [a, b, c, d]) = nested();
    let root = editor.document().root();
    let ask = |full_depth: u32, full_cells: u32| {
        let ask = StageAsk {
            full_depth,
            full_cells,
            ..ALL
        };
        editor.stage_view(root, &ask).unwrap()
    };

    // Nothing cell by cell: the stage's cells, and a summary of each grid among them.
    let view = ask(0, u32::MAX);
    assert_eq!(owners(&view), (vec![root], vec![a, d]));
    assert_eq!(view.cells.len(), 4);
    assert_eq!((view.cells[0].summary, view.cells[0].full), (Some(0), None));
    assert_eq!((view.cells[3].summary, view.cells[3].full), (Some(1), None));
    assert!(view.cells[0].has_grid && !view.cells[1].has_grid);
    assert_eq!((view.cells[1].summary, view.cells[1].full), (None, None));

    // One level: A and D, in the order of their cells; B is met and summarised, not given.
    let view = ask(1, u32::MAX);
    assert_eq!(owners(&view), (vec![root, a, d], vec![a, d, b]));
    assert_eq!(view.cells.len(), 4 + 6 + 9);
    assert_eq!(
        (view.cells[0].summary, view.cells[0].full),
        (Some(0), Some(1))
    );
    assert_eq!(
        (view.cells[3].summary, view.cells[3].full),
        (Some(1), Some(2))
    );
    let (ga, gd) = (&view.grids[1], &view.grids[2]);
    assert_eq!((ga.level, ga.first, ga.xs, ga.ys), (1, 4, 3, 2));
    assert_eq!((ga.x0, ga.y0, ga.cols, ga.rows), (0, 0, 3, 2));
    assert_eq!(ga.col_widths, [80, 80, 80]);
    assert_eq!((gd.level, gd.first, gd.cols, gd.rows), (1, 10, 3, 3));
    assert_eq!(
        texts(&view)[4..10],
        ["a00", "a10", "a20", "a01", "a11", "a21"]
    );
    assert_eq!(view.cells[10].text.text, "d00");
    let inner = &view.cells[5];
    assert_eq!(inner.id, b);
    assert_eq!((inner.summary, inner.full), (Some(2), None));

    // Two levels: B too, after every grid of the first level; C is summarised.
    let view = ask(2, u32::MAX);
    assert_eq!(owners(&view), (vec![root, a, d, b], vec![a, d, b, c]));
    assert_eq!(view.cells[5].full, Some(3));
    assert_eq!((view.grids[3].level, view.grids[3].first), (2, 19));
    assert_eq!(texts(&view)[19..], ["b00", "b10", "b01", "b11"]);
    assert_eq!(
        (view.cells[21].summary, view.cells[21].full),
        (Some(3), None)
    );

    // Three levels, and more than there are.
    let view = ask(3, u32::MAX);
    assert_eq!(owners(&view), (vec![root, a, d, b, c], vec![a, d, b, c]));
    assert_eq!((view.grids[4].level, view.grids[4].first), (3, 23));
    assert_eq!(view.cells.len(), 4 + 6 + 9 + 4 + 5);
    assert_eq!(ask(u32::MAX, u32::MAX), view);

    // The size: A has 6 cells, D 9, B 4, C 5.
    assert_eq!(owners(&ask(3, 9)).0, [root, a, d, b, c]);
    assert_eq!(owners(&ask(3, 8)).0, [root, a, b, c]);
    assert_eq!(owners(&ask(3, 6)), (vec![root, a, b, c], vec![a, d, b, c]));
    // A grid that is too large hides what is in it, however small.
    assert_eq!(owners(&ask(3, 5)), (vec![root], vec![a, d]));
    assert_eq!(owners(&ask(3, 0)), (vec![root], vec![a, d]));
    // B fits and C, one cell larger, does not.
    let mut editor = editor;
    let narrow = Command::DeleteColumns {
        cell: a,
        at: 2,
        count: 1,
    };
    editor
        .apply(narrow, None, &mut SeededIds::from_u64(8))
        .unwrap();
    let ask = StageAsk {
        full_depth: 3,
        full_cells: 4,
        ..ALL
    };
    let view = editor.stage_view(root, &ask).unwrap();
    assert_eq!(owners(&view), (vec![root, a, b], vec![a, d, b, c]));

    // A stage further down counts its levels from itself.
    let ask = StageAsk {
        full_depth: 1,
        ..ALL
    };
    let view = editor.stage_view(b, &ask).unwrap();
    assert_eq!(owners(&view), (vec![b, c], vec![c]));
    let trail: Vec<_> = view.trail.iter().map(|t| t.id).collect();
    assert_eq!(trail, [root, a, b]);
    let shown: Vec<_> = view.trail.iter().map(|t| t.shown.text.as_str()).collect();
    assert_eq!(shown, ["", "r00", "a10"]);
}

#[test]
fn a_window_gives_those_rows_and_columns_of_the_stage_and_no_others() {
    let mut b = Build::new(9);
    let root = b.doc.root();
    b.grid(root, 7, 50, |x, y| format!("{x},{y}"));
    let widths: Vec<u32> = (0..7).map(|x| 10 + x).collect();
    b.doc.cell_mut(root).unwrap().grid_mut().unwrap().col_widths = widths;
    // A nested grid inside the window and one outside it.
    let inside = b.id(root, 3, 21);
    b.grid(inside, 2, 2, |_, _| "in".into());
    let outside = b.id(root, 0, 0);
    b.grid(outside, 2, 2, |_, _| "out".into());
    let editor = b.editor();
    let window = |x0: u32, y0: u32, cols: u32, rows: u32| {
        let ask = StageAsk {
            x0,
            y0,
            cols,
            rows,
            full_depth: 1,
            ..ALL
        };
        editor.stage_view(root, &ask).unwrap()
    };
    let shape = |view: &StageView| {
        let g = &view.grids[0];
        assert_eq!((g.xs, g.ys, g.first), (7, 50, 0));
        (g.x0, g.y0, g.cols, g.rows)
    };

    // In the middle.
    let view = window(2, 20, 3, 4);
    assert_eq!(shape(&view), (2, 20, 3, 4));
    assert_eq!(view.grids[0].col_widths, [12, 13, 14]);
    assert_eq!(
        texts(&view)[..12],
        [
            "2,20", "3,20", "4,20", "2,21", "3,21", "4,21", "2,22", "3,22", "4,22", "2,23", "3,23",
            "4,23"
        ]
    );
    // The cell in column x0 + i of row y0 + j is at first + j * cols + i.
    assert_eq!(view.cells[3 + 1].id, inside);
    // What is nested in the window is given; what is outside it is not looked at.
    assert_eq!(owners(&view), (vec![root, inside], vec![inside]));
    assert_eq!((view.grids[1].first, view.cells.len()), (12, 16));

    // Partly outside: what there is.
    let view = window(5, 48, 10, 5);
    assert_eq!(shape(&view), (5, 48, 2, 2));
    assert_eq!(view.grids[0].col_widths, [15, 16]);
    assert_eq!(texts(&view), ["5,48", "6,48", "5,49", "6,49"]);
    let view = window(6, 0, u32::MAX, 1);
    assert_eq!(shape(&view), (6, 0, 1, 1));
    assert_eq!(texts(&view), ["6,0"]);

    // Wholly outside, to the right, below, and far away: the grid and no cells.
    for (x0, y0) in [(7, 0), (0, 50), (u32::MAX, u32::MAX), (8, 3)] {
        let view = window(x0, y0, 3, 3);
        assert_eq!(view.grids.len(), 1, "{x0} {y0}");
        assert!(view.cells.is_empty() && view.summaries.is_empty());
        let (gx, gy, cols, rows) = shape(&view);
        assert_eq!((gx, gy), (x0.min(7), y0.min(50)));
        assert_eq!(cols * rows, 0);
    }
    assert_eq!(shape(&window(7, 0, 3, 3)), (7, 0, 0, 3));
    assert!(window(7, 0, 3, 3).grids[0].col_widths.is_empty());
    // Below the grid, the columns asked for still have their widths.
    assert_eq!(window(1, 50, 2, 3).grids[0].col_widths, [11, 12]);

    // Nothing asked for.
    let view = window(1, 1, 0, 5);
    assert_eq!(shape(&view), (1, 1, 0, 5));
    assert!(view.cells.is_empty());
    let view = window(1, 1, 5, 0);
    assert_eq!(shape(&view), (1, 1, 5, 0));
    assert!(view.cells.is_empty());

    // Everything.
    let view = window(0, 0, u32::MAX, u32::MAX);
    assert_eq!(shape(&view), (0, 0, 7, 50));
    assert_eq!(view.cells.len(), 350 + 4 + 4);
    assert_eq!(view.grids[0].col_widths.len(), 7);
    assert_eq!(owners(&view).0, [root, outside, inside]);
}

#[test]
fn a_stage_cell_without_a_grid_gives_its_trail_and_no_grids() {
    let (editor, [a, _, _, _]) = nested();
    let doc = editor.document();
    let root = doc.root();
    let leaf = doc.child(a, 2, 1).unwrap();
    let view = editor.stage_view(leaf, &ALL).unwrap();
    let trail: Vec<_> = view.trail.iter().map(|t| t.id).collect();
    assert_eq!(trail, [root, a, leaf]);
    assert_eq!(view.trail[2].shown, whole("a21"));
    assert!(view.grids.is_empty() && view.cells.is_empty() && view.summaries.is_empty());

    // A new document: the root alone.
    let mut ids = SeededIds::from_u64(10);
    let editor = Editor::new(Document::new(&mut ids)).unwrap();
    let root = editor.document().root();
    let view = editor.stage_view(root, &StageAsk::default()).unwrap();
    assert_eq!(view.trail.len(), 1);
    assert_eq!((view.trail[0].id, &view.trail[0].shown), (root, &whole("")));
    assert_eq!(view.trail[0].color, Color::WHITE);
    assert!(view.grids.is_empty() && view.cells.is_empty() && view.summaries.is_empty());
    let look = editor.cell_look(root).unwrap();
    assert_eq!(look.id, root);
    assert!(!look.has_grid);
}

#[test]
fn a_cell_that_does_not_exist_is_not_found() {
    let (editor, _) = nested();
    for ask in [ALL, StageAsk::default()] {
        assert!(matches!(
            editor.stage_view(no_such_cell(), &ask),
            Err(Error::NotFound(_))
        ));
    }
    assert!(matches!(
        editor.cell_look(no_such_cell()),
        Err(Error::NotFound(_))
    ));
}

/// A root of one cell holding a grid of 4 by 3 with one of everything a summary tells
/// apart. Returns the editor and the cell that holds the grid.
fn summarised() -> (Editor, CellId) {
    let mut b = Build::new(11);
    let root = b.doc.root();
    b.grid(root, 1, 1, |_, _| "Trip".into());
    let trip = b.id(root, 0, 0);
    b.grid(trip, 4, 3, |x, y| format!("t{x}{y}"));
    b.doc
        .cell_mut(trip)
        .unwrap()
        .grid_mut()
        .unwrap()
        .border_color = BLUE;
    b.at(trip, 0, 0).text = "Day 1".into();
    b.at(trip, 0, 0).cell_color = RED;
    b.at(trip, 1, 0).text = String::new();
    b.at(trip, 2, 0).text = "x".repeat(254);
    b.at(trip, 3, 0).text = "y".repeat(300);
    {
        let formula = b.at(trip, 0, 1);
        formula.text = "=1000+234".into();
        formula.kind = CellKind::Formula;
        formula.cell_color = GREEN;
    }
    b.at(trip, 1, 1).text = "z".repeat(253);
    // A cell with a grid, with and without text of its own.
    let holder = b.id(trip, 2, 1);
    b.grid(holder, 1, 1, |_, _| String::new());
    let empty = b.id(trip, 3, 1);
    b.doc.cell_mut(empty).unwrap().text = String::new();
    b.grid(empty, 1, 1, |_, _| String::new());
    // Five code points in two characters.
    b.at(trip, 0, 2).text = "e\u{301}\u{1F44D}\u{1F3FD}!".into();
    (b.editor(), trip)
}

#[test]
fn a_summary_gives_the_size_the_first_texts_and_the_shape() {
    let (editor, trip) = summarised();
    let root = editor.document().root();
    let view = editor.stage_view(root, &ALL).unwrap();
    assert_eq!(view.summaries.len(), 1);
    let summary = &view.summaries[0];
    assert_eq!(summary.owner, trip);
    // The count of a badge is these two multiplied: the cells directly in the grid, not
    // the two more that are further down.
    assert_eq!((summary.xs, summary.ys), (4, 3));
    assert_eq!(summary.border_color, BLUE);

    // The preview block is cut to the grid, and row by row.
    assert_eq!((summary.preview_cols, summary.preview_rows), (3, 3));
    let long = "x".repeat(40);
    assert_eq!(
        previews(summary),
        [
            "Day 1",
            "",
            long.as_str(),
            // A formula cell shows its result.
            "1234",
            &"z".repeat(40),
            "t21",
            "e\u{301}\u{1F44D}\u{1F3FD}!",
            "t12",
            "t22"
        ]
    );
    let cut: Vec<_> = summary.preview.iter().map(|t| t.cut).collect();
    assert_eq!(
        cut,
        [false, false, true, false, true, false, false, false, false]
    );

    // The shape: the colour drawn, then nothing, a count that stops at 254, or a grid.
    assert_eq!((summary.shape_cols, summary.shape_rows), (4, 3));
    let cells: Vec<_> = summary.shape.chunks(4).collect();
    assert_eq!(cells.len(), 12);
    assert_eq!(cells[0], [200, 0, 0, 5]);
    assert_eq!(cells[1], [255, 255, 255, 0]);
    assert_eq!(cells[2], [255, 255, 255, 254]);
    assert_eq!(cells[3], [255, 255, 255, 254]);
    // What it shows is counted: "1234", not the formula.
    assert_eq!(cells[4], [0, 200, 0, 4]);
    assert_eq!(cells[5], [255, 255, 255, 253]);
    assert_eq!(cells[6], [255, 255, 255, 255]);
    assert_eq!(cells[7], [255, 255, 255, 255]);
    // Code points, as everywhere.
    assert_eq!(cells[8], [255, 255, 255, 5]);
    assert_eq!(cells[9], [255, 255, 255, 3]);

    // Smaller blocks, each its own size.
    let ask = StageAsk {
        preview_cols: 2,
        preview_rows: 1,
        shape_cols: 1,
        shape_rows: 2,
        preview_cut: 3,
        ..ALL
    };
    let view = editor.stage_view(root, &ask).unwrap();
    let summary = &view.summaries[0];
    assert_eq!((summary.xs, summary.ys), (4, 3));
    assert_eq!((summary.preview_cols, summary.preview_rows), (2, 1));
    assert_eq!(summary.preview, [part("Day"), whole("")]);
    assert_eq!((summary.shape_cols, summary.shape_rows), (1, 2));
    assert_eq!(summary.shape, [200, 0, 0, 5, 0, 200, 0, 4]);

    // A preview text is cut between characters too.
    let ask = StageAsk {
        preview_cols: 1,
        preview_rows: 3,
        preview_cut: 3,
        ..ALL
    };
    let view = editor.stage_view(root, &ask).unwrap();
    assert_eq!(
        view.summaries[0].preview,
        [part("Day"), part("123"), part("e\u{301}")]
    );

    // None of either.
    for (cols, rows) in [(0, 5), (5, 0), (0, 0)] {
        let ask = StageAsk {
            preview_cols: cols,
            preview_rows: rows,
            shape_cols: rows,
            shape_rows: cols,
            ..ALL
        };
        let view = editor.stage_view(root, &ask).unwrap();
        let summary = &view.summaries[0];
        assert!(summary.preview.is_empty() && summary.shape.is_empty());
        assert_eq!(summary.preview_cols * summary.preview_rows, 0);
        assert_eq!(summary.shape_cols * summary.shape_rows, 0);
        assert_eq!((summary.xs, summary.ys), (4, 3));
    }

    // A grid given cell by cell has its summary all the same.
    let ask = StageAsk {
        full_depth: 1,
        ..ALL
    };
    let full = editor.stage_view(root, &ask).unwrap();
    assert_eq!(full.cells[0].full, Some(1));
    assert_eq!(
        full.summaries[0],
        editor.stage_view(root, &ALL).unwrap().summaries[0]
    );
    assert_eq!(full.summaries.len(), 3);
}

/// Whatever becomes of a line break in a cell's text, a preview and the trail are one
/// line each.
#[test]
fn a_preview_and_the_trail_have_no_line_breaks() {
    let mut editor = entered(&[]);
    let root = editor.document().root();
    let mut ids = SeededIds::from_u64(12);
    let first = editor.document().child(root, 0, 0).unwrap();
    let grid = Command::CreateGrid {
        cell: first,
        xs: 1,
        ys: 1,
    };
    editor.apply(grid, None, &mut ids).unwrap();
    let inner = editor.document().child(first, 0, 0).unwrap();
    for cell in [first, inner] {
        let text = "one\ntwo\r\nthree".to_string();
        let enter = Command::EnterText { cell, text };
        editor.apply(enter, None, &mut ids).unwrap();
    }
    let view = editor.stage_view(first, &ALL).unwrap();
    for line in [&view.trail[1].shown.text, &view.cells[0].text.text] {
        assert!(line.starts_with("one"), "{line:?}");
    }
    let view = editor.stage_view(root, &ALL).unwrap();
    let lines = [
        &view.trail[0].shown.text,
        &editor.stage_view(first, &ALL).unwrap().trail[1].shown.text,
        &view.summaries[0].preview[0].text,
    ];
    for line in lines {
        assert!(line.chars().all(|c| c >= ' '), "{line:?}");
    }
    let preview = &view.summaries[0].preview[0].text;
    assert!(preview.starts_with("one two"), "{preview:?}");
    // As long as the text it stands for.
    let text = &editor.document().cell(inner).unwrap().text;
    assert_eq!(preview.chars().count(), text.chars().count());
}

/// Numbers no interface means: nothing breaks, nothing is reserved for them, and they
/// mean "all" or "none".
#[test]
fn any_numbers_may_be_asked() {
    let (editor, _) = nested();
    let root = editor.document().root();
    let most = StageAsk {
        x0: 0,
        y0: 0,
        cols: u32::MAX,
        rows: u32::MAX,
        full_depth: u32::MAX,
        full_cells: u32::MAX,
        preview_cols: u32::MAX,
        preview_rows: u32::MAX,
        shape_cols: u32::MAX,
        shape_rows: u32::MAX,
        text_cut: u32::MAX,
        preview_cut: u32::MAX,
    };
    let exact = StageAsk {
        cols: 2,
        rows: 2,
        full_depth: 3,
        full_cells: 9,
        preview_cols: 3,
        preview_rows: 5,
        shape_cols: 3,
        shape_rows: 5,
        text_cut: 3,
        preview_cut: 3,
        ..most
    };
    let view = editor.stage_view(root, &most).unwrap();
    assert_eq!(view, editor.stage_view(root, &exact).unwrap());
    assert_eq!(view.cells.len(), editor.document().cell_count() - 1);
    assert_eq!(view.summaries[3].preview.len(), 5);

    let none = StageAsk {
        x0: u32::MAX,
        y0: u32::MAX,
        ..most
    };
    assert!(editor.stage_view(root, &none).unwrap().cells.is_empty());
    let zero = StageAsk {
        x0: 0,
        y0: 0,
        cols: u32::MAX,
        rows: u32::MAX,
        full_depth: 0,
        full_cells: 0,
        preview_cols: 0,
        preview_rows: 0,
        shape_cols: 0,
        shape_rows: 0,
        text_cut: 0,
        preview_cut: 0,
    };
    let view = editor.stage_view(root, &zero).unwrap();
    assert_eq!(view.cells.len(), 4);
    assert!(view.cells.iter().all(|c| c.text == part("")));
    assert_eq!(view.trail[0].shown, whole(""));
    assert_eq!(view.summaries.len(), 2);
    assert!(view.summaries.iter().all(|s| s.preview.is_empty()));
}

#[test]
fn the_values_to_start_from_are_those_of_the_design() {
    let ask = StageAsk::default();
    assert_eq!(
        (ask.x0, ask.y0, ask.cols, ask.rows),
        (0, 0, u32::MAX, u32::MAX)
    );
    assert_eq!((ask.full_depth, ask.full_cells), (1, 24));
    assert_eq!((ask.preview_cols, ask.preview_rows), (3, 4));
    assert_eq!((ask.shape_cols, ask.shape_rows), (12, 20));
    assert_eq!((ask.text_cut, ask.preview_cut), (2000, 40));
}

// ---- Any document, any request ----

/// An editor on a document that random commands made of a 3 by 3 grid.
fn edited(seed: u64, ops: &[Op]) -> Editor {
    let mut ids = SeededIds::from_u64(seed);
    let doc = Document::new(&mut ids);
    let root = doc.root();
    let mut editor = Editor::new(doc).unwrap();
    let grid = Command::CreateGrid {
        cell: root,
        xs: 3,
        ys: 3,
    };
    editor.apply(grid, None, &mut ids).unwrap();
    for op in ops {
        // Commands that cannot be carried out are refused and change nothing.
        let _ = editor.apply(command(editor.document(), op), Some(1), &mut ids);
    }
    editor
}

/// A number of a request: small ones, where the rules are, and now and then anything.
fn number() -> impl Strategy<Value = u32> {
    prop_oneof![6 => 0u32..8, 2 => Just(u32::MAX), 1 => any::<u32>()]
}

fn ask() -> impl Strategy<Value = StageAsk> {
    (
        (number(), number(), number(), number()),
        (0u32..5, number()),
        (number(), number(), number(), number()),
        (number(), number()),
        any::<bool>(),
    )
        .prop_map(|(window, full, blocks, cuts, all)| StageAsk {
            x0: if all { 0 } else { window.0 % 4 },
            y0: if all { 0 } else { window.1 % 4 },
            cols: if all { u32::MAX } else { window.2 },
            rows: if all { u32::MAX } else { window.3 },
            full_depth: full.0,
            full_cells: full.1.saturating_mul(3),
            preview_cols: blocks.0,
            preview_rows: blocks.1,
            shape_cols: blocks.2,
            shape_rows: blocks.3,
            text_cut: cuts.0,
            preview_cut: cuts.1,
        })
}

/// What `stage_view` must say of a text cut to `max`: found the slow way.
fn expected_cut(text: &str, max: u32) -> CutText {
    let kept = floor_boundary(text, max) as usize;
    CutText {
        text: text.chars().take(kept).collect(),
        cut: kept < text.chars().count(),
    }
}

fn expected_line(text: &str, max: u32) -> CutText {
    let cut = expected_cut(text, max);
    let space = |c: char| if c < ' ' { ' ' } else { c };
    CutText {
        text: cut.text.chars().map(space).collect(),
        cut: cut.cut,
    }
}

/// The style and colour of each of the first `n` code points.
fn painted(runs: &[Run], base: Style, n: usize) -> Vec<(Style, Option<Color>)> {
    let mut out = vec![(base, None); n];
    for run in runs {
        for at in run.start..run.start + run.len {
            if let Some(slot) = out.get_mut(at as usize) {
                *slot = (run.style, run.color);
            }
        }
    }
    out
}

/// The link each of the first `n` code points belongs to: its start and target.
fn linked(links: &[Link], n: usize) -> Vec<Option<(u32, LinkTarget)>> {
    let mut out = vec![None; n];
    for link in links {
        for at in link.start..link.start + link.len {
            if let Some(slot) = out.get_mut(at as usize) {
                *slot = Some((link.start, link.target.clone()));
            }
        }
    }
    out
}

/// Sorted, apart, not empty, inside `n` code points.
fn in_order(ranges: impl Iterator<Item = (u32, u32)>, n: u32) -> bool {
    let mut end = 0;
    for (start, len) in ranges {
        if len == 0 || start < end || start + len > n {
            return false;
        }
        end = start + len;
    }
    true
}

fn check_look(editor: &Editor, look: &CellLook, max: u32) -> Result<(), TestCaseError> {
    let doc = editor.document();
    let cell = doc.cell(look.id).unwrap();
    // The text is a beginning of the cell's, ends between two characters, and is the
    // longest such within the cut.
    prop_assert!(cell.text.starts_with(&look.text.text));
    let kept = look.text.text.chars().count() as u32;
    prop_assert!(kept <= max);
    prop_assert_eq!(is_boundary(&cell.text, kept), Some(true));
    prop_assert_eq!(&look.text, &expected_cut(&cell.text, max));
    prop_assert_eq!(look.kind, cell.kind);
    let result = editor.calc().result(look.id);
    prop_assert_eq!(result.is_some(), cell.kind == CellKind::Formula);
    prop_assert_eq!(
        look.result.clone(),
        result.map(|(text, _)| expected_cut(text, max))
    );
    prop_assert_eq!(look.failed, result.is_some_and(|(_, failed)| failed));
    let tag = doc.tags().get(&cell.text);
    prop_assert_eq!(look.is_tag, tag.is_some());
    let colors = tag.map_or((cell.cell_color, cell.text_color), |tag| {
        (tag.cell_color, tag.text_color)
    });
    prop_assert_eq!((look.cell_color, look.text_color), colors);
    prop_assert_eq!(look.style, cell.style);
    prop_assert_eq!(look.rel_size, cell.rel_size);
    prop_assert_eq!(look.text_align, cell.text_align);
    prop_assert_eq!(look.vert_align, cell.vert_align);
    prop_assert_eq!(look.image.as_ref(), cell.image.as_ref().map(|i| &i.file));
    prop_assert_eq!(look.has_note, !cell.note.is_empty());
    prop_assert_eq!(look.has_grid, cell.grid().is_some());
    // Runs and links: each code point that is given looks as it does in the document
    // (a tag's without the colours), and nothing reaches past the text.
    let n = kept as usize;
    let mut want = painted(&cell.runs, cell.style, n);
    if look.is_tag {
        want.iter_mut().for_each(|(_, color)| *color = None);
    }
    prop_assert_eq!(painted(&look.runs, look.style, n), want);
    prop_assert!(in_order(look.runs.iter().map(|r| (r.start, r.len)), kept));
    prop_assert!(look.runs.iter().all(|r| r.extra.is_empty()));
    prop_assert!(look
        .runs
        .iter()
        .all(|r| r.style != look.style || r.color.is_some()));
    prop_assert_eq!(linked(&look.links, n), linked(&cell.links, n));
    prop_assert!(in_order(look.links.iter().map(|l| (l.start, l.len)), kept));
    prop_assert!(look.links.iter().all(|l| l.extra.is_empty()));
    Ok(())
}

fn check_summary(
    editor: &Editor,
    summary: &GridSummary,
    ask: &StageAsk,
) -> Result<(), TestCaseError> {
    let doc = editor.document();
    let grid = doc.cell(summary.owner).unwrap().grid().unwrap();
    prop_assert_eq!((summary.xs, summary.ys), (grid.xs(), grid.ys()));
    prop_assert_eq!(summary.border_color, grid.border_color);
    prop_assert_eq!(summary.preview_cols, ask.preview_cols.min(grid.xs()));
    prop_assert_eq!(summary.preview_rows, ask.preview_rows.min(grid.ys()));
    let mut preview = Vec::new();
    for y in 0..summary.preview_rows {
        for x in 0..summary.preview_cols {
            let shown = editor.shown(grid.at(x, y).unwrap()).unwrap();
            preview.push(expected_line(shown, ask.preview_cut));
        }
    }
    prop_assert_eq!(&summary.preview, &preview);
    prop_assert_eq!(summary.shape_cols, ask.shape_cols.min(grid.xs()));
    prop_assert_eq!(summary.shape_rows, ask.shape_rows.min(grid.ys()));
    let mut shape = Vec::new();
    for y in 0..summary.shape_rows {
        for x in 0..summary.shape_cols {
            let cell = doc.cell(grid.at(x, y).unwrap()).unwrap();
            let color = doc
                .tags()
                .get(&cell.text)
                .map_or(cell.cell_color, |tag| tag.cell_color);
            let holds = match cell.grid() {
                Some(_) => 255,
                None => editor.shown(cell.id()).unwrap().chars().count().min(254) as u8,
            };
            shape.extend([color.r, color.g, color.b, holds]);
        }
    }
    prop_assert_eq!(&summary.shape, &shape);
    Ok(())
}

fn check_view(editor: &Editor, stage: CellId, ask: &StageAsk) -> Result<(), TestCaseError> {
    let doc = editor.document();
    let view = editor.stage_view(stage, ask).unwrap();
    prop_assert_eq!(&view, &editor.stage_view(stage, ask).unwrap());

    // The trail is the path from the root.
    let mut path = vec![stage];
    while let Some(parent) = doc.cell(path[path.len() - 1]).unwrap().parent() {
        path.push(parent);
    }
    path.reverse();
    prop_assert_eq!(path[0], doc.root());
    prop_assert_eq!(view.trail.len(), path.len());
    for (step, id) in view.trail.iter().zip(&path) {
        prop_assert_eq!(step.id, *id);
        let line = expected_line(editor.shown(*id).unwrap(), ask.preview_cut);
        prop_assert_eq!(&step.shown, &line);
        let cell = doc.cell(*id).unwrap();
        let color = doc
            .tags()
            .get(&cell.text)
            .map_or(cell.cell_color, |tag| tag.cell_color);
        prop_assert_eq!(step.color, color);
    }

    // The grids: the stage's first, if it has one.
    let has_grid = doc.cell(stage).unwrap().grid().is_some();
    prop_assert_eq!(view.grids.is_empty(), !has_grid);
    let (mut next_first, mut next_summary, mut next_grid) = (0u32, 0u32, 1u32);
    for (n, part) in view.grids.iter().enumerate() {
        let owner = doc.cell(part.owner).unwrap();
        let grid = owner.grid().unwrap();
        prop_assert_eq!((part.xs, part.ys), (grid.xs(), grid.ys()));
        if n == 0 {
            prop_assert_eq!((part.owner, part.level), (stage, 0));
            prop_assert_eq!(
                (part.x0, part.y0),
                (ask.x0.min(part.xs), ask.y0.min(part.ys))
            );
            prop_assert_eq!(part.cols, ask.cols.min(part.xs - part.x0));
            prop_assert_eq!(part.rows, ask.rows.min(part.ys - part.y0));
        } else {
            let whole = (0, 0, part.xs, part.ys);
            prop_assert_eq!((part.x0, part.y0, part.cols, part.rows), whole);
            prop_assert!(part.level >= 1 && part.level <= ask.full_depth);
            prop_assert!(u64::from(part.xs) * u64::from(part.ys) <= u64::from(ask.full_cells));
        }
        prop_assert_eq!(part.border_color, grid.border_color);
        prop_assert_eq!(part.outer_spacing, grid.outer_spacing);
        prop_assert_eq!(part.vertical, grid.vertical);
        prop_assert_eq!(part.draw_style, owner.draw_style);
        let (from, to) = (part.x0 as usize, (part.x0 + part.cols) as usize);
        prop_assert_eq!(&part.col_widths[..], &grid.col_widths[from..to]);
        // Its cells are exactly its window, row by row, right after the grid before.
        prop_assert_eq!(part.first, next_first);
        next_first += part.cols * part.rows;
        prop_assert!(next_first as usize <= view.cells.len());
        for j in 0..part.rows {
            for i in 0..part.cols {
                let look = &view.cells[(part.first + j * part.cols + i) as usize];
                let id = grid.at(part.x0 + i, part.y0 + j).unwrap();
                prop_assert_eq!(look.id, id);
                check_look(editor, look, ask.text_cut)?;
                // A summary exactly for a cell with a grid, in the order of the cells.
                let inner = doc.cell(id).unwrap().grid();
                prop_assert_eq!(look.summary.is_some(), inner.is_some());
                let Some(inner) = inner else {
                    prop_assert_eq!(look.full, None);
                    continue;
                };
                prop_assert_eq!(look.summary, Some(next_summary));
                next_summary += 1;
                let summary = &view.summaries[look.summary.unwrap() as usize];
                prop_assert_eq!(summary.owner, id);
                // Cell by cell exactly if it is near enough and small enough; then its
                // grid is the next one not yet pointed at, one level down.
                let count = u64::from(inner.xs()) * u64::from(inner.ys());
                let full = part.level < ask.full_depth && count <= u64::from(ask.full_cells);
                prop_assert_eq!(look.full.is_some(), full);
                if full {
                    prop_assert_eq!(look.full, Some(next_grid));
                    next_grid += 1;
                    let below = &view.grids[look.full.unwrap() as usize];
                    prop_assert_eq!((below.owner, below.level), (id, part.level + 1));
                }
            }
        }
    }
    prop_assert_eq!(next_first as usize, view.cells.len());
    prop_assert_eq!(next_summary as usize, view.summaries.len());
    prop_assert_eq!(next_grid as usize, view.grids.len().max(1));
    for summary in &view.summaries {
        check_summary(editor, summary, ask)?;
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    /// Whatever the document, the stage and the request: every index is in range, the
    /// cells of a grid are its window in order, texts are cut where the rules of text
    /// allow, and everything said is what the document and the results hold.
    #[test]
    fn a_stage_view_is_the_document_and_nothing_else(
        seed: u64,
        ops in proptest::collection::vec(op(), 0..40),
        asks in proptest::collection::vec((ask(), any::<prop::sample::Index>()), 1..6),
    ) {
        let editor = edited(seed, &ops);
        let order = editor.document().cells_in_order();
        check_view(&editor, editor.document().root(), &asks[0].0)?;
        for (ask, at) in &asks {
            check_view(&editor, *at.get(&order), ask)?;
        }
    }

    /// With every row and nothing nested, the stage view says what `grid_view` says, and
    /// the editor's look of a cell is the stage's look of it when nothing was cut.
    #[test]
    fn a_stage_view_agrees_with_the_grid_view(
        seed: u64,
        ops in proptest::collection::vec(op(), 0..40),
    ) {
        let editor = edited(seed, &ops);
        for id in editor.document().cells_in_order() {
            let view = editor.stage_view(id, &ALL).unwrap();
            let plain = editor.grid_view(id).unwrap();
            prop_assert_eq!(view.grids.len(), usize::from(plain.cell.has_grid));
            if let Some(grid) = view.grids.first() {
                prop_assert_eq!((grid.xs, grid.ys), (plain.xs, plain.ys));
                prop_assert_eq!((grid.cols, grid.rows), (plain.xs, plain.ys));
            }
            prop_assert_eq!(view.cells.len(), plain.cells.len());
            for (look, cell) in view.cells.iter().zip(&plain.cells) {
                prop_assert_eq!(look.id, cell.id);
                prop_assert_eq!(&look.text, &whole(&cell.text));
                prop_assert_eq!(look.kind, cell.kind);
                prop_assert_eq!(look.result.clone(), cell.result.as_deref().map(whole));
                prop_assert_eq!(look.failed, cell.failed);
                prop_assert_eq!(look.has_grid, cell.has_grid);
                let mut alone = editor.cell_look(look.id).unwrap();
                prop_assert_eq!((alone.summary, alone.full), (None, None));
                alone.summary = look.summary;
                prop_assert_eq!(&alone, look);
            }
            check_look(&editor, &editor.cell_look(id).unwrap(), u32::MAX)?;
        }
    }
}

// ---- What it costs ----

/// The 100,000-cell document of `large.rs`: 10 by 100 cells, each holding 10 by 10.
fn hundred_thousand() -> Editor {
    let mut b = Build::new(100);
    let root = b.doc.root();
    b.grid(root, 10, 100, |x, y| format!("Section {}", y * 10 + x));
    for n in 0..1000 {
        let cell = b.id(root, n % 10, n / 10);
        b.grid(cell, 10, 10, |x, y| {
            format!("Item {n}.{} with some ordinary text in it", y * 10 + x)
        });
        for y in 0..10 {
            let leaf = b.at(cell, 0, y);
            leaf.runs.push(run(0, 4, BOLD, None));
            leaf.links.push(link(5, 3, "https://example.org"));
        }
    }
    assert_eq!(b.doc.cell_count(), 101_001);
    b.editor()
}

/// A document like the generated test document of the interface specification: six levels
/// of grids below the root, the root's grid and one grid on each level 12 by 20, a third
/// of the cells of every grid holding a grid, smaller ones further down.
fn six_levels() -> (Editor, CellId) {
    let mut b = Build::new(60);
    let root = b.doc.root();
    b.doc.cell_mut(root).unwrap().text = "Home".into();
    let sizes = [(12, 20), (4, 5), (3, 4), (2, 3), (2, 2), (1, 2)];
    let mut level = vec![root];
    let mut big = root;
    for (depth, (xs, ys)) in sizes.iter().enumerate() {
        let mut next = Vec::new();
        for (n, cell) in level.iter().enumerate() {
            // The first grid of every level is as large as grids get.
            let (xs, ys) = if n == 0 { (12, 20) } else { (*xs, *ys) };
            b.grid(*cell, xs, ys, |x, y| {
                format!("Level {depth} cell {x}.{y} text")
            });
            for m in (0..xs * ys).step_by(3) {
                let id = b.id(*cell, m % xs, m / xs);
                next.push(id);
            }
        }
        if depth == 3 {
            big = level[0];
        }
        level = next;
    }
    assert_eq!(b.doc.depth(level[0]), Some(6));
    (b.editor(), big)
}

/// A stage of 12 by 20 in which every cell holds a grid of 12 by 20: the most a stage of
/// the specification's test document can ask of a summary.
fn all_grids() -> Editor {
    let mut b = Build::new(61);
    let root = b.doc.root();
    b.grid(root, 12, 20, |x, y| format!("Cell {x}.{y}"));
    for n in 0..240 {
        let cell = b.id(root, n % 12, n / 12);
        b.grid(cell, 12, 20, |x, y| format!("Inner {n} cell {x}.{y} text"));
    }
    b.editor()
}

/// The middle of five timings of `stage_view`, with the size of the answer.
fn timed(what: &str, editor: &Editor, stage: CellId, ask: &StageAsk) -> Duration {
    let mut times = Vec::new();
    let mut size = (0, 0, 0);
    for _ in 0..5 {
        let start = Instant::now();
        let view = editor.stage_view(stage, ask).unwrap();
        times.push(start.elapsed());
        let shapes: usize = view.summaries.iter().map(|s| s.shape.len() / 4).sum();
        size = (view.cells.len(), view.summaries.len(), shapes);
    }
    times.sort();
    println!(
        "stage_view: {what}: {:?} ({} cells, {} summaries, {} shape cells)",
        times[2], size.0, size.1, size.2
    );
    times[2]
}

/// Measures `stage_view` on large documents and prints the times (`--nocapture`); the
/// numbers are in `docs/milestones`. The limits are at least ten times what the
/// development machine takes, optimised or not: a slow or busy machine passes, a reading
/// that has become many times slower does not. That a reading does no work for cells it
/// does not give is held exactly by a count, in `edit/stage.rs`, not by these times.
#[test]
fn large_stages_are_read_within_a_set_time() {
    let phone = StageAsk::default();
    let limit = |fast: u64| {
        Duration::from_millis(if cfg!(debug_assertions) {
            fast * 10
        } else {
            fast
        })
    };

    let editor = hundred_thousand();
    let root = editor.document().root();
    let all = timed("100,000 cells, the whole root", &editor, root, &phone);
    assert!(all < limit(400), "{all:?}");
    let window = StageAsk {
        y0: 30,
        rows: 40,
        ..phone
    };
    let rows = timed("100,000 cells, 40 rows of the root", &editor, root, &window);
    assert!(rows < limit(160), "{rows:?}");
    let section = editor.document().child(root, 5, 50).unwrap();
    let one = timed(
        "100,000 cells, one section as the stage",
        &editor,
        section,
        &phone,
    );
    assert!(one < limit(40), "{one:?}");

    let (editor, big) = six_levels();
    let root = editor.document().root();
    println!("six levels: {} cells", editor.document().cell_count());
    let top = timed("six levels, the root", &editor, root, &phone);
    assert!(top < limit(120), "{top:?}");
    let deep = timed(
        "six levels, a 12 by 20 grid three levels down",
        &editor,
        big,
        &phone,
    );
    assert!(deep < limit(120), "{deep:?}");
    let two = StageAsk {
        full_depth: 2,
        ..phone
    };
    let more = timed(
        "six levels, the root, two levels cell by cell",
        &editor,
        root,
        &two,
    );
    assert!(more < limit(200), "{more:?}");

    let editor = all_grids();
    let root = editor.document().root();
    let worst = timed(
        "every cell of 12 by 20 holds 12 by 20",
        &editor,
        root,
        &phone,
    );
    assert!(worst < limit(200), "{worst:?}");
}
