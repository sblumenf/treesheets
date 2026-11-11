//! `wombat edit`: reads editing commands, one per line, and hands them to the core.
//!
//! This file only turns text into [`Command`]s and prints results; what the commands do is
//! in `wombat_core::edit`.

use std::path::Path;

use wombat_core::{
    storage, CellId, CellKind, Changes, Color, Command, Document, DrawStyle, Editor, ImageRef,
    LinkTarget, Prop, SeededIds, Style, TextAlign, VertAlign, ViewState,
};

/// The text of `wombat edit --help`.
pub const HELP: &str = "\
wombat edit <file.wombat> --script <file>   run editing commands (\"-\": from standard input)

One command per line; empty lines and lines starting with # are skipped. Nothing is written
to the file until a `save` line. If a line fails, the run stops there with an error and
exit code 1, and what was not saved is lost.

Naming a cell (CELL): by its place, as `wombat show` prints it: / is the root cell, /2,1 the
cell in column 2, row 1 of the root's grid, /2,1/1,3 the cell in column 1, row 3 of the grid
inside that one. Or by its ID (as `show ids` prints it, with or without the #).

Numbers: columns and rows count from 1, as in the places above. Positions in text count the
characters before the place, from 0 (0 is the start of the text), and lengths are numbers
of characters; a character here is a Unicode code point, and a range that would cut an
emoji or an accented letter made of several code points is refused.
Text is written in double quotes; inside them \\\" \\\\ \\n \\t and \\u{1F600} are understood.
Colours are written #rrggbb.

Text
  text CELL \"text\"                    replace the whole text (drops styles and links)
  enter CELL \"text\"                   the same, as typed into the cell: a text that starts
                                      with = makes it a formula cell (=SUM(Costs) * 2),
                                      any other text a data cell again
  insert CELL POSITION \"text\"         insert text
  delete CELL POSITION LENGTH         delete text
  replace CELL POSITION LENGTH \"text\" replace a range of text
  style CELL POSITION LENGTH +bold -italic ...
                                      switch styles on (+) or off (-) for a range:
                                      bold italic fixed underline strike
  color CELL POSITION LENGTH #rrggbb  colour a range (`none`: the cell's text colour)
  link CELL POSITION LENGTH url \"target\"
  link CELL POSITION LENGTH cell CELL
  link CELL POSITION LENGTH cell-in FILE CELL
  link CELL POSITION LENGTH none      make a range a link (to a web page or file, to a
                                      cell, to a cell of the document in FILE), or remove
                                      its links
  replace-all \"text\" \"with\" [case] [CELL]
                                      replace the text wherever it is found in the texts
                                      of cells (of CELL and the cells inside it), as one
                                      unit of undo; case: only exactly these characters
  find \"query\" [next|prev CELL]       print the cells that match a search (the syntax of
                                      `wombat search`), or the next or previous one from CELL
  resolve-unknown                     give a target to the links imported from TreeSheets
                                      without one, where exactly one cell now has the
                                      text they named

Properties
  set CELL... name=value...           set properties of one or more cells:
        color=#rrggbb  text-color=#rrggbb  size=N (positive is smaller)
        align=auto|left|center|right  valign=auto|top|middle|bottom
        kind=data|code|var_assign|var_read|view_h|view_v|formula
        note=\"text\"  draw=grid|bubbles|lines  image=FILE|none  image-scale=NUMBER
        edited=MILLISECONDS|none
        of the cell's grid: border=#rrggbb  spacing=N  vertical=true|false
  cell-style CELL... +bold -italic... switch styles on or off for whole cells
  width CELL COLUMN WIDTH             set the width of a column of the cell's grid

Structure (CELL is the cell that has the grid)
  grid CELL COLUMNS ROWS              give a cell a grid of empty cells
  remove-grid CELL                    remove a cell's grid and all in it
  insert-rows CELL ROW COUNT          insert empty rows before ROW (rows + 1: at the end)
  insert-columns CELL COLUMN COUNT
  delete-rows CELL ROW COUNT          delete rows with all in them
  delete-columns CELL COLUMN COUNT
  clear CELL...                       empty cells (text, image, grid), keep their looks
  delete-cells CELL COLUMN ROW COLUMNS ROWS
                                      the Delete key of TreeSheets on a block of cells:
                                      empty them, and delete rows or columns left empty
  move-rows CELL ROW COUNT TO         move rows so that the first becomes row TO
  move-columns CELL COLUMN COUNT TO
  swap CELL CELL                      two cells trade places
  move CELL EMPTY-CELL                move a cell to where an empty cell is
  copy CELL CELL                      make the second cell a copy of the first (paste)
  import CELL FILE [csv|tsv|semicolon|text] [comma|tab] [ENCODING] [formulas] [unguard]
         [replace]                    read a file of comma, tab or semicolon separated
                                      values or indented text (by default told by its
                                      name, and for .csv by what its lines are separated
                                      by) as the grid of CELL; one unit of undo. A grid
                                      with content is replaced only with `replace`.
                                      ENCODING: utf-8 (the default; nothing is guessed),
                                      utf-16le, utf-16be, windows-1252. formulas: texts
                                      that start with = become formula cells. unguard:
                                      take away the ' of the guard of `wombat export`

Restructuring (as the TreeSheets menu actions of the same names)
  sort CELL COLUMN ROW COUNT [desc]   sort COUNT rows from ROW on by the text in COLUMN,
                                      then by the columns after it; text order without
                                      regard to case (\"10\" before \"9\"), equal rows keep
                                      their order
  transpose CELL...                   rows become columns in the grids of the cells
  hierarchify CELL                    turn a table into a hierarchy: rows that start with
                                      the same text are merged under one cell
  flatten CELL                        turn a hierarchy into a table: one row per innermost
                                      cell, with the cells it was inside of to its left
  hierarchy-swap CELL                 the cells with the text of CELL move up one level and
                                      take their parents inside them
  wrap CELL COLUMN ROW COLUMNS ROWS   move a block of cells into a new cell in its place
  collapse CELL COLUMN ROW COLUMNS ROWS
                                      join the texts of a block into its first cell
  split CELL POSITION                 cut the text in two cells inside the cell

Sizes and checkboxes
  size-by DELTA [deep] CELL...        add DELTA to the size (negative is bigger), within
                                      -32..8; deep: also the cells inside
  width-by CELL COLUMN COUNT DELTA [deep]
                                      add DELTA to the widths of COUNT columns; deep: also
                                      all columns of the grids inside them
  toggle-check CELL...                add to-do checkboxes, tick them, or remove them

Calculating
  run                                 run the program of the document, as Program > Run in
                                      TreeSheets: fills the view cells, assigns variables
  (formula cells are kept up to date after every command; `changes` lists the ones whose
  result changed, and `show` prints each as its formula = what it shows)

Tags
  tag \"name\" #rrggbb #rrggbb          add a tag (cell colour, text colour) or recolour it
  untag \"name\"                        remove a tag
  tag-cells CELL...                   make the texts of cells tags, with their colours
  untag-cells CELL...                 remove the tags that the texts of cells name

View (not part of the document, not undone)
  fold CELL / unfold CELL             hide or show a cell's grid

History
  undo [N|all]  redo [N|all]          undo or redo the last unit, N units, or all
  begin / end                         the commands between them undo as one unit
  now MILLISECONDS|none               the time that edits stamp on cells from here on
                                      (milliseconds since 1970; default: none)

Looking and saving
  show [ids]                          print the document as an outline
  changes                             print what the last command, group, undo or redo changed
  check                               check that the document is valid
  save                                write the document (and its folds) to the file
";

/// Splits a line into words. A double quote starts text that runs to the next unescaped
/// double quote, also in the middle of a word (`note="a b"`); the quotes are dropped and
/// the escapes replaced. The flag says whether a word had a quoted part.
fn words(line: &str) -> Result<Vec<(String, bool)>, String> {
    let mut out = Vec::new();
    let mut word = String::new();
    let (mut started, mut quoted) = (false, false);
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                started = true;
                quoted = true;
                loop {
                    match chars.next() {
                        None => return Err("a quote is not closed".into()),
                        Some('"') => break,
                        Some('\\') => word.push(escape(&mut chars)?),
                        Some(c) => word.push(c),
                    }
                }
            }
            c if c.is_whitespace() => {
                if started {
                    out.push((std::mem::take(&mut word), quoted));
                }
                started = false;
                quoted = false;
            }
            c => {
                started = true;
                word.push(c);
            }
        }
    }
    if started {
        out.push((word, quoted));
    }
    Ok(out)
}

fn escape(chars: &mut std::str::Chars) -> Result<char, String> {
    match chars.next() {
        Some('n') => Ok('\n'),
        Some('t') => Ok('\t'),
        Some('"') => Ok('"'),
        Some('\\') => Ok('\\'),
        Some('u') => {
            let digits: String = chars
                .skip_while(|c| *c == '{')
                .take_while(|c| *c != '}')
                .collect();
            u32::from_str_radix(&digits, 16)
                .ok()
                .and_then(char::from_u32)
                .ok_or_else(|| format!("\\u{{{digits}}} is not a character"))
        }
        other => Err(format!(
            "unknown escape \\{}",
            other.map(String::from).unwrap_or_default()
        )),
    }
}

/// The state of one `wombat edit` run.
pub struct Session<'a> {
    editor: Editor,
    view: ViewState,
    view_changed: bool,
    ids: SeededIds,
    now: Option<i64>,
    last: Changes,
    /// The formula cells whose results that changed.
    recalculated: Vec<CellId>,
    path: &'a Path,
    saved: bool,
}

type Args<'a> = std::slice::Iter<'a, (String, bool)>;

fn next<'a>(args: &mut Args<'a>, what: &str) -> Result<&'a str, String> {
    args.next()
        .map(|(word, _)| word.as_str())
        .ok_or_else(|| format!("missing: {what}"))
}

fn number(args: &mut Args, what: &str) -> Result<u32, String> {
    let word = next(args, what)?;
    word.parse()
        .map_err(|_| format!("{what}: {word:?} is not a whole number"))
}

/// A column or row number, which the user counts from 1.
fn index(args: &mut Args, what: &str) -> Result<u32, String> {
    number(args, what)?
        .checked_sub(1)
        .ok_or_else(|| format!("{what}: columns and rows count from 1"))
}

fn signed(args: &mut Args, what: &str) -> Result<i32, String> {
    let word = next(args, what)?;
    word.parse()
        .map_err(|_| format!("{what}: {word:?} is not a whole number"))
}

/// Takes the word `deep` if it is next.
fn deep(args: &mut Args) -> bool {
    let found = args.clone().next().is_some_and(|(word, _)| word == "deep");
    if found {
        args.next();
    }
    found
}

fn color(word: &str) -> Result<Color, String> {
    Color::parse(word).ok_or_else(|| format!("{word:?} is not a colour like #ff8800"))
}

fn done(args: &mut Args) -> Result<(), String> {
    match args.next() {
        None => Ok(()),
        Some((word, _)) => Err(format!("unexpected: {word:?}")),
    }
}

/// `+bold -italic ...` as the bits to switch on and the bits to switch off.
fn styles(args: &mut Args) -> Result<(Style, Style), String> {
    let (mut on, mut off) = (Style::default(), Style::default());
    let mut any = false;
    for (word, _) in args {
        let (target, name) = match (word.strip_prefix('+'), word.strip_prefix('-')) {
            (Some(name), _) => (&mut on, name),
            (_, Some(name)) => (&mut off, name),
            _ => return Err(format!("{word:?}: write +bold or -bold")),
        };
        let bit = match name {
            "bold" => &mut target.bold,
            "italic" => &mut target.italic,
            "fixed" => &mut target.fixed,
            "underline" => &mut target.underline,
            "strike" => &mut target.strike,
            _ => return Err(format!("{name:?} is not a style")),
        };
        *bit = true;
        any = true;
    }
    if !any {
        return Err("missing: a style, such as +bold".into());
    }
    Ok((on, off))
}

fn property(name: &str, value: &str) -> Result<Prop, String> {
    let bad = || format!("{name}: {value:?} is not a value for it");
    Ok(match name {
        "color" => Prop::CellColor(color(value)?),
        "text-color" => Prop::TextColor(color(value)?),
        "size" => Prop::RelSize(value.parse().map_err(|_| bad())?),
        "align" => Prop::TextAlign(TextAlign::parse(value).ok_or_else(bad)?),
        "valign" => Prop::VertAlign(VertAlign::parse(value).ok_or_else(bad)?),
        "kind" => Prop::Kind(CellKind::parse(value).ok_or_else(bad)?),
        "note" => Prop::Note(value.to_string()),
        "draw" => Prop::DrawStyle(DrawStyle::parse(value).ok_or_else(bad)?),
        "image" if value == "none" => Prop::Image(None),
        "image" => Prop::Image(Some(ImageRef::new(value))),
        "image-scale" => Prop::ImageScale(value.parse().map_err(|_| bad())?),
        "edited" if value == "none" => Prop::LastEdit(None),
        "edited" => Prop::LastEdit(Some(value.parse().map_err(|_| bad())?)),
        "border" => Prop::BorderColor(color(value)?),
        "spacing" => Prop::OuterSpacing(value.parse().map_err(|_| bad())?),
        "vertical" => Prop::Vertical(value.parse().map_err(|_| bad())?),
        _ => return Err(format!("{name:?} is not a property")),
    })
}

impl<'a> Session<'a> {
    /// Opens the document at `path` for editing.
    pub fn open(path: &'a Path, ids: SeededIds) -> Result<Session<'a>, String> {
        let doc = storage::load_document(path).map_err(|e| e.to_string())?;
        let folder = path.parent().unwrap_or(Path::new(""));
        // View state is a convenience: a damaged one must not stop the document being edited.
        let mut view = storage::load_view_state(folder, doc.id())
            .ok()
            .flatten()
            .unwrap_or_else(|| ViewState::new(doc.id()));
        view.retain_existing(&doc);
        Ok(Session {
            editor: Editor::new(doc).map_err(|e| e.to_string())?,
            view,
            view_changed: false,
            ids,
            now: None,
            last: Changes::default(),
            recalculated: Vec::new(),
            path,
            saved: true,
        })
    }

    /// Whether everything done so far has been saved.
    pub fn saved(&self) -> bool {
        self.saved
    }

    fn doc(&self) -> &Document {
        self.editor.document()
    }

    /// The cell that a word names: a place (`/2,1/1,3`) or an ID.
    fn cell(&self, word: &str) -> Result<CellId, String> {
        crate::links::find_cell(self.doc(), word)
    }

    fn next_cell(&self, args: &mut Args, what: &str) -> Result<CellId, String> {
        self.cell(next(args, what)?)
    }

    /// The place of a cell, as [`Session::cell`] reads it.
    fn place(&self, id: CellId) -> String {
        crate::links::place_of(self.doc(), id)
    }

    fn apply(&mut self, command: Command) -> Result<(), String> {
        let changes = self
            .editor
            .apply(command, self.now, &mut self.ids)
            .map_err(|e| e.to_string())?;
        self.changed(changes);
        Ok(())
    }

    fn changed(&mut self, changes: Changes) {
        let folds = self.view.folded.len();
        self.view.apply_changes(self.editor.document(), &changes);
        self.view_changed |= self.view.folded.len() != folds;
        self.saved &= changes.is_empty();
        self.last = changes;
        self.recalculated = self.editor.recalculated().to_vec();
    }

    fn history(&mut self, args: &mut Args, undo: bool) -> Result<(), String> {
        let count = match args.next().map(|(word, _)| word.as_str()) {
            None => 1,
            Some("all") => usize::MAX,
            Some(word) => word
                .parse()
                .map_err(|_| format!("{word:?} is neither a number nor `all`"))?,
        };
        let mut all = Changes::default();
        for n in 0..count {
            let step = if undo {
                self.editor.undo()
            } else {
                self.editor.redo()
            };
            match step.map_err(|e| e.to_string())? {
                Some(changes) => {
                    self.changed(changes.clone());
                    all = changes;
                }
                None if n == 0 && count != usize::MAX => {
                    return Err(format!("nothing to {}", if undo { "undo" } else { "redo" }));
                }
                None => break,
            }
        }
        self.last = all;
        Ok(())
    }

    fn report(&self) -> String {
        let cells = |name: &str, ids: &[CellId], out: &mut String| {
            for id in ids {
                let place = match self.doc().cell(*id) {
                    Some(_) => self.place(*id),
                    None => "(gone)".into(),
                };
                out.push_str(&format!("{name} {place} #{id}\n"));
            }
        };
        let mut out = String::new();
        cells("changed", &self.last.changed, &mut out);
        cells("created", &self.last.created, &mut out);
        cells("removed", &self.last.removed, &mut out);
        cells("grid", &self.last.grids, &mut out);
        for tag in &self.last.tags {
            out.push_str(&format!("tag {tag:?}\n"));
        }
        for id in &self.recalculated {
            if let Some(result) = self.editor.shown(*id) {
                out.push_str(&format!("recalculated {} = {result:?}\n", self.place(*id)));
            }
        }
        if out.is_empty() {
            out.push_str("nothing changed\n");
        }
        out
    }

    /// Runs one line and returns what it prints.
    pub fn line(&mut self, line: &str) -> Result<String, String> {
        let words = words(line)?;
        let mut args = words.iter();
        let Some((verb, _)) = args.next().filter(|(verb, _)| !verb.starts_with('#')) else {
            return Ok(String::new());
        };
        let args = &mut args;
        let mut out = String::new();
        match verb.as_str() {
            "text" => {
                let cell = self.next_cell(args, "a cell")?;
                let text = next(args, "the text")?.to_string();
                done(args)?;
                self.apply(Command::SetText { cell, text })?;
            }
            "enter" => {
                let cell = self.next_cell(args, "a cell")?;
                let text = next(args, "the text")?.to_string();
                done(args)?;
                self.apply(Command::EnterText { cell, text })?;
            }
            "run" => {
                done(args)?;
                self.apply(Command::Run)?;
                let failure = self
                    .editor
                    .calc()
                    .last_run()
                    .and_then(|r| r.failure.clone());
                out = match failure {
                    Some(failure) => format!("run: {failure}\n"),
                    None => "run: finished\n".to_string(),
                };
            }
            "insert" => {
                let cell = self.next_cell(args, "a cell")?;
                let at = number(args, "a position")?;
                let text = next(args, "the text")?.to_string();
                done(args)?;
                self.apply(Command::InsertText { cell, at, text })?;
            }
            "delete" => {
                let cell = self.next_cell(args, "a cell")?;
                let (start, len) = (number(args, "a position")?, number(args, "a length")?);
                done(args)?;
                self.apply(Command::DeleteText { cell, start, len })?;
            }
            "replace" => {
                let cell = self.next_cell(args, "a cell")?;
                let (start, len) = (number(args, "a position")?, number(args, "a length")?);
                let text = next(args, "the text")?.to_string();
                done(args)?;
                self.apply(Command::ReplaceText {
                    cell,
                    start,
                    len,
                    text,
                })?;
            }
            "style" => {
                let cell = self.next_cell(args, "a cell")?;
                let (start, len) = (number(args, "a position")?, number(args, "a length")?);
                let (set, clear) = styles(args)?;
                self.apply(Command::SetRangeStyle {
                    cell,
                    start,
                    len,
                    set,
                    clear,
                })?;
            }
            "color" => {
                let cell = self.next_cell(args, "a cell")?;
                let (start, len) = (number(args, "a position")?, number(args, "a length")?);
                let color = match next(args, "a colour or `none`")? {
                    "none" => None,
                    word => Some(color(word)?),
                };
                done(args)?;
                self.apply(Command::SetRangeColor {
                    cell,
                    start,
                    len,
                    color,
                })?;
            }
            "link" => {
                let cell = self.next_cell(args, "a cell")?;
                let (start, len) = (number(args, "a position")?, number(args, "a length")?);
                let target = match next(args, "`url`, `cell`, `cell-in` or `none`")? {
                    "none" => None,
                    "url" => Some(LinkTarget::External(next(args, "the target")?.to_string())),
                    "cell" => Some(LinkTarget::Cell {
                        doc: None,
                        cell: self.next_cell(args, "the cell to link to")?,
                    }),
                    "cell-in" => {
                        let file = next(args, "the document the cell is in")?;
                        let other = storage::load_document(Path::new(file))
                            .map_err(|e| format!("{file}: {e}"))?;
                        let word = next(args, "the cell to link to")?;
                        Some(LinkTarget::Cell {
                            doc: Some(other.id()),
                            cell: crate::links::find_cell(&other, word)
                                .map_err(|e| format!("{file}: {e}"))?,
                        })
                    }
                    word => return Err(format!("{word:?}: write url, cell, cell-in or none")),
                };
                done(args)?;
                self.apply(Command::SetLink {
                    cell,
                    start,
                    len,
                    target,
                })?;
            }
            "replace-all" => {
                let text = next(args, "the text to find")?.to_string();
                let with = next(args, "the replacement")?.to_string();
                let mut term = wombat_search::Term::new(text);
                let mut scope = wombat_search::replace::Scope::Document;
                for (word, _) in args {
                    match word.as_str() {
                        "case" => term.exact = true,
                        word => scope = wombat_search::replace::Scope::Branch(self.cell(word)?),
                    }
                }
                let (now, ids) = (self.now, &mut self.ids);
                let done = wombat_search::replace::replace(
                    &mut self.editor,
                    &term,
                    &with,
                    scope,
                    now,
                    ids,
                )
                .map_err(|e| e.to_string())?;
                out = format!("{} places in {} cells\n", done.places, done.cells);
                self.changed(done.changes);
            }
            "find" => {
                let query = next(args, "a query")?.to_string();
                let from = match args.next().map(|(word, _)| word.as_str()) {
                    None => None,
                    Some(way @ ("next" | "prev")) => {
                        Some((self.next_cell(args, "a cell")?, way == "prev"))
                    }
                    Some(word) => return Err(format!("unexpected: {word:?}")),
                };
                done(args)?;
                let parsed = wombat_search::parse(&query, self.now.unwrap_or(0))
                    .map_err(|e| e.to_string())?;
                let mut index = wombat_search::SearchIndex::new();
                index.insert_document(self.doc(), self.editor.calc(), "");
                let hits = match from {
                    Some((cell, reverse)) => {
                        let hit =
                            index.next_match(&parsed.query, self.doc().id(), Some(cell), reverse);
                        hit.map_err(|e| e.to_string())?.into_iter().collect()
                    }
                    None => {
                        let order = parsed.order.unwrap_or(wombat_search::Order::Document);
                        let options = wombat_search::Options::all(order);
                        let found = index.search(&parsed.query, &options);
                        found.map_err(|e| e.to_string())?.hits
                    }
                };
                for hit in &hits {
                    let marked = crate::search::marked(&hit.snippet);
                    out.push_str(&format!("{} {marked}\n", self.place(hit.cell)));
                }
                out.push_str(&format!("{} found\n", hits.len()));
            }
            "resolve-unknown" => {
                done(args)?;
                let commands = wombat_core::links::resolve_unknown(self.doc());
                out = format!("{} links got a target\n", commands.len());
                self.editor.begin_group();
                let result = commands.into_iter().try_for_each(|command| {
                    let applied = self.editor.apply(command, self.now, &mut self.ids);
                    applied.map(|_| ()).map_err(|e| e.to_string())
                });
                let changes = self.editor.end_group();
                self.changed(changes);
                result?;
            }
            "set" => {
                let (mut cells, mut props) = (Vec::new(), Vec::new());
                for (word, quoted) in args {
                    match word.split_once('=') {
                        Some((name, value)) if !word.starts_with('/') => {
                            props.push(property(name, value)?);
                        }
                        _ if *quoted => return Err(format!("unexpected: {word:?}")),
                        _ => cells.push(self.cell(word)?),
                    }
                }
                if cells.is_empty() || props.is_empty() {
                    return Err("missing: cells and name=value properties".into());
                }
                self.apply(Command::SetProps { cells, props })?;
            }
            "cell-style" => {
                let mut cells = Vec::new();
                for (word, _) in args.clone() {
                    if word.starts_with('+') || word.starts_with('-') {
                        break;
                    }
                    cells.push(self.cell(word)?);
                    args.next();
                }
                let (set, clear) = styles(args)?;
                if cells.is_empty() {
                    return Err("missing: a cell".into());
                }
                self.apply(Command::SetStyle { cells, set, clear })?;
            }
            "width" => {
                let cell = self.next_cell(args, "a cell")?;
                let (column, width) = (index(args, "a column")?, number(args, "a width")?);
                done(args)?;
                self.apply(Command::SetColumnWidth {
                    cell,
                    column,
                    width,
                })?;
            }
            "import" => {
                let cell = self.next_cell(args, "a cell")?;
                let file = next(args, "a file")?;
                let mut options = crate::convert::ReadOptions::default();
                let mut replace = false;
                for (word, _) in args.by_ref() {
                    match word.as_str() {
                        "formulas" => options.formulas = true,
                        "unguard" => options.unguard = true,
                        "replace" => replace = true,
                        other if options.word(other) => {}
                        other => options.format = Some(other),
                    }
                }
                let depth = self.doc().depth(cell).unwrap_or(0);
                let levels = wombat_core::limits::MAX_DEPTH.saturating_sub(depth);
                let read =
                    crate::convert::read_table(file, &options, levels).map_err(|failure| {
                        match failure {
                            crate::Failure::Usage(m) | crate::Failure::Failed(m) => m,
                        }
                    })?;
                // The links that point into the grid that goes, from cells that stay: of
                // this document, the only one in sight here.
                let doc = self.doc();
                let inside = doc.cell(cell).and_then(|c| c.grid()).map(|g| g.cells());
                let mut index = wombat_core::links::LinkIndex::new();
                index.insert_document(doc);
                let links = index.would_break(doc, inside.unwrap_or(&[])).len();
                let (xs, ys) = (read.table.xs(), read.table.ys());
                self.apply(Command::Import {
                    cell,
                    table: read.table,
                    replace,
                })?;
                out = format!(
                    "imported {file}: {xs} columns by {ys} rows\n  {}\n",
                    read.how
                );
                let removed = self.last.removed.len();
                if removed > 0 {
                    let cells = if removed == 1 { "cell" } else { "cells" };
                    let pointing = match links {
                        0 => "no link of this document pointed at them".to_string(),
                        1 => "1 link of this document pointed at them and now points at \
                              nothing"
                            .to_string(),
                        n => format!(
                            "{n} links of this document pointed at them and now point at nothing"
                        ),
                    };
                    out.push_str(&format!(
                        "  removed {removed} {cells} that were there; {pointing}\n"
                    ));
                }
            }
            "grid" => {
                let cell = self.next_cell(args, "a cell")?;
                let (xs, ys) = (number(args, "columns")?, number(args, "rows")?);
                done(args)?;
                self.apply(Command::CreateGrid { cell, xs, ys })?;
            }
            "remove-grid" => {
                let cell = self.next_cell(args, "a cell")?;
                done(args)?;
                self.apply(Command::RemoveGrid { cell })?;
            }
            "insert-rows" | "insert-columns" | "delete-rows" | "delete-columns" => {
                let cell = self.next_cell(args, "a cell")?;
                let (at, count) = (index(args, "a row or column")?, number(args, "a count")?);
                done(args)?;
                self.apply(match verb.as_str() {
                    "insert-rows" => Command::InsertRows { cell, at, count },
                    "insert-columns" => Command::InsertColumns { cell, at, count },
                    "delete-rows" => Command::DeleteRows { cell, at, count },
                    _ => Command::DeleteColumns { cell, at, count },
                })?;
            }
            "clear" => {
                let cells = args
                    .map(|(word, _)| self.cell(word))
                    .collect::<Result<Vec<_>, _>>()?;
                if cells.is_empty() {
                    return Err("missing: a cell".into());
                }
                self.apply(Command::ClearCells { cells })?;
            }
            "delete-cells" => {
                let cell = self.next_cell(args, "a cell")?;
                let (x, y) = (index(args, "a column")?, index(args, "a row")?);
                let (xs, ys) = (number(args, "columns")?, number(args, "rows")?);
                done(args)?;
                self.apply(Command::DeleteCells { cell, x, y, xs, ys })?;
            }
            "move-rows" | "move-columns" => {
                let cell = self.next_cell(args, "a cell")?;
                let from = index(args, "a row or column")?;
                let count = number(args, "a count")?;
                let to = index(args, "where to")?;
                done(args)?;
                self.apply(if verb == "move-rows" {
                    Command::MoveRows {
                        cell,
                        from,
                        count,
                        to,
                    }
                } else {
                    Command::MoveColumns {
                        cell,
                        from,
                        count,
                        to,
                    }
                })?;
            }
            "swap" | "move" | "copy" => {
                let a = self.next_cell(args, "a cell")?;
                let b = self.next_cell(args, "a second cell")?;
                done(args)?;
                self.apply(match verb.as_str() {
                    "swap" => Command::SwapCells { a, b },
                    "move" => Command::MoveCell { from: a, to: b },
                    _ => Command::CopyCell { from: a, to: b },
                })?;
            }
            "tag" => {
                let name = next(args, "a name")?.to_string();
                let cell_color = color(next(args, "a cell colour")?)?;
                let text_color = color(next(args, "a text colour")?)?;
                done(args)?;
                self.apply(Command::SetTag {
                    name,
                    cell_color,
                    text_color,
                })?;
            }
            "untag" => {
                let name = next(args, "a name")?.to_string();
                done(args)?;
                self.apply(Command::RemoveTag { name })?;
            }
            "sort" => {
                let cell = self.next_cell(args, "a cell")?;
                let (column, row) = (index(args, "a column")?, index(args, "a row")?);
                let count = number(args, "a count")?;
                let descending = match args.next().map(|(word, _)| word.as_str()) {
                    None => false,
                    Some("desc") => true,
                    Some(word) => return Err(format!("unexpected: {word:?}")),
                };
                done(args)?;
                self.apply(Command::SortRows {
                    cell,
                    column,
                    row,
                    count,
                    descending,
                })?;
            }
            "transpose" | "tag-cells" | "untag-cells" | "toggle-check" => {
                let cells = args
                    .map(|(word, _)| self.cell(word))
                    .collect::<Result<Vec<_>, _>>()?;
                if cells.is_empty() {
                    return Err("missing: a cell".into());
                }
                self.apply(match verb.as_str() {
                    "transpose" => Command::Transpose { cells },
                    "tag-cells" => Command::TagCells { cells },
                    "untag-cells" => Command::UntagCells { cells },
                    _ => Command::ToggleCheck { cells },
                })?;
            }
            "hierarchify" | "flatten" | "hierarchy-swap" => {
                let cell = self.next_cell(args, "a cell")?;
                done(args)?;
                self.apply(match verb.as_str() {
                    "hierarchify" => Command::Hierarchify { cell },
                    "flatten" => Command::Flatten { cell },
                    _ => Command::HierarchySwap { cell },
                })?;
            }
            "wrap" | "collapse" => {
                let cell = self.next_cell(args, "a cell")?;
                let (x, y) = (index(args, "a column")?, index(args, "a row")?);
                let (xs, ys) = (number(args, "columns")?, number(args, "rows")?);
                done(args)?;
                self.apply(if verb == "wrap" {
                    Command::Wrap { cell, x, y, xs, ys }
                } else {
                    Command::Collapse { cell, x, y, xs, ys }
                })?;
            }
            "split" => {
                let cell = self.next_cell(args, "a cell")?;
                let at = number(args, "a position")?;
                done(args)?;
                self.apply(Command::SplitCell { cell, at })?;
            }
            "size-by" => {
                let delta = signed(args, "a change of size")?;
                let recursive = deep(args);
                let cells = args
                    .map(|(word, _)| self.cell(word))
                    .collect::<Result<Vec<_>, _>>()?;
                if cells.is_empty() {
                    return Err("missing: a cell".into());
                }
                self.apply(Command::AdjustRelSize {
                    cells,
                    delta,
                    recursive,
                })?;
            }
            "width-by" => {
                let cell = self.next_cell(args, "a cell")?;
                let (column, count) = (index(args, "a column")?, number(args, "a count")?);
                let delta = signed(args, "a change of width")?;
                let recursive = deep(args);
                done(args)?;
                self.apply(Command::AdjustColumnWidths {
                    cell,
                    column,
                    count,
                    delta,
                    recursive,
                })?;
            }
            "fold" | "unfold" => {
                let cell = self.next_cell(args, "a cell")?;
                done(args)?;
                let changed = self
                    .view
                    .set_folded(self.editor.document(), cell, verb == "fold")
                    .map_err(|e| e.to_string())?;
                self.view_changed |= changed;
                self.saved &= !changed;
            }
            "undo" => self.history(args, true)?,
            "redo" => self.history(args, false)?,
            "begin" => self.editor.begin_group(),
            // After a group, `changes` shows what the group changed as a whole.
            "end" => {
                self.last = self.editor.end_group();
                self.recalculated = self.editor.recalculated().to_vec();
            }
            "now" => {
                self.now = match next(args, "a time in milliseconds, or `none`")? {
                    "none" => None,
                    word => Some(
                        word.parse()
                            .map_err(|_| format!("{word:?} is not a number of milliseconds"))?,
                    ),
                };
            }
            "show" => {
                let with_ids = match args.next().map(|(word, _)| word.as_str()) {
                    None => false,
                    Some("ids") => true,
                    Some(word) => return Err(format!("unexpected: {word:?}")),
                };
                out = crate::show_document(
                    self.doc(),
                    self.editor.calc(),
                    Some(&self.view),
                    with_ids,
                );
            }
            "changes" => out = self.report(),
            "check" => {
                self.doc().validate().map_err(|e| e.to_string())?;
                out = format!("valid: {} cells\n", self.doc().cell_count());
            }
            "save" => {
                done(args)?;
                storage::save_document(self.doc(), self.path).map_err(|e| e.to_string())?;
                if self.view_changed {
                    let folder = self.path.parent().unwrap_or(Path::new(""));
                    storage::save_view_state(folder, &self.view).map_err(|e| e.to_string())?;
                    self.view_changed = false;
                }
                self.saved = true;
                out = format!("saved {}\n", self.path.display());
            }
            _ => return Err(format!("{verb:?} is not a command; see wombat edit --help")),
        }
        Ok(out)
    }
}
