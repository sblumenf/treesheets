//! Queries: what to look for (decision B3: words, tags and filters are parts of one
//! search).
//!
//! A [`Query`] is a tree of words ([`Term`]), filters ([`Filter`]) and the three ways to
//! combine them. An interface can build one directly; [`parse`] reads the text syntax
//! that a person types, which `docs/milestones/M6.md` describes in full:
//!
//! ```text
//! apple pie            both words (in any field of the cell)
//! "apple pie"          the phrase
//! apple OR pear        either; also apple | pear
//! -apple               not
//! (a OR b) -c          grouping
//! case:Apple           the exact characters, upper and lower case as written
//! tag:todo  has:note  kind:formula  bg:#ff0000  fg:#000000  style:bold+italic
//! in:CELL  doc:NAME  links-to:CELL  linked-from:CELL  has:broken-link
//! edited:7d  edited:2026-01-01..2026-01-31  recent:5%  text:"exactly this"  image:NAME
//! sort:recent
//! ```
//!
//! Parsing never fails on punctuation: a quote or bracket left open is closed at the
//! end, as when a query is still being typed. It fails only when a filter has a value
//! that means nothing, and when the query is over the limits below, which exist so that
//! no query can take unbounded time or memory.

use std::fmt;

use wombat_core::{CellId, CellKind, Color, DocId, Style};

/// Longest query text that [`parse`] reads, in bytes.
pub const MAX_QUERY_BYTES: usize = 4096;
/// Most words and filters in one query.
pub const MAX_PARTS: usize = 64;
/// Deepest nesting of a query.
pub const MAX_NESTING: usize = 16;
/// Longest word or phrase, in bytes.
pub const MAX_TERM_BYTES: usize = 1024;

/// A word or phrase to find.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Term {
    /// The characters to find, anywhere in a field.
    pub text: String,
    /// Whether they must be found exactly as written (upper and lower case, accents and
    /// forms all count), instead of in folded form ([`crate::fold`]).
    pub exact: bool,
}

impl Term {
    /// A word or phrase, found whatever its case, accents or form.
    pub fn new(text: impl Into<String>) -> Term {
        Term {
            text: text.into(),
            exact: false,
        }
    }

    /// A word or phrase that must be found exactly as written.
    pub fn exact(text: impl Into<String>) -> Term {
        Term {
            text: text.into(),
            exact: true,
        }
    }
}

/// Something a cell can have.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Has {
    /// Text that is not empty.
    Text,
    /// A note.
    Note,
    /// An image.
    Image,
    /// A grid.
    Grid,
    /// A link in its text.
    Link,
    /// A link that points at nothing ([`wombat_core::links::Resolved::Broken`]).
    BrokenLink,
    /// A tag: its text is the name of a tag of its document.
    Tag,
}

/// A condition on a cell that is not about finding words.
#[derive(Clone, PartialEq, Debug)]
pub enum Filter {
    /// The cell has this tag: its text is exactly this name, and its document has a tag
    /// of that name (TreeSheets' "show cells with tags").
    Tag(String),
    /// The cell has this.
    Has(Has),
    /// The cell has a link that leads to this cell.
    LinksTo(CellId),
    /// This cell has a link that leads to the cell.
    LinkedFrom(CellId),
    /// The cell was last edited in this time span, both ends included, in milliseconds
    /// since 1970 UTC; an open end is `None`. A cell with no edit time never passes.
    Edited {
        /// The earliest time.
        from: Option<i64>,
        /// The latest time.
        to: Option<i64>,
    },
    /// The cell is among the most recently edited cells of its document: this many
    /// percent of them (TreeSheets' "show 5% of last edits"; 1 to 99).
    Recent(u8),
    /// The cell is in the document with this ID.
    Doc(DocId),
    /// The cell is in the document with this file name (with or without `.wombat`, in
    /// any letter case).
    DocName(String),
    /// The cell is this cell or inside it: search within a branch.
    Within(CellId),
    /// The cell is of this kind.
    Kind(CellKind),
    /// The cell has this background colour (TreeSheets' "same cell color").
    CellColor(Color),
    /// The cell has this text colour.
    TextColor(Color),
    /// The cell's base style is exactly this (TreeSheets' "same style").
    Style(Style),
    /// The cell's text is exactly this (TreeSheets' "same text").
    TextIs(String),
    /// The cell shows the image with this file name.
    Image(String),
}

/// What to look for.
#[derive(Clone, PartialEq, Debug)]
pub enum Query {
    /// Every cell.
    All,
    /// Cells where the word or phrase is found.
    Term(Term),
    /// Cells that pass the filter.
    Filter(Filter),
    /// Cells that match every part.
    And(Vec<Query>),
    /// Cells that match any part.
    Or(Vec<Query>),
    /// Cells that do not match.
    Not(Box<Query>),
}

/// In what order results come.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Order {
    /// Best matches first: see the ranking rule in [`crate::engine`].
    #[default]
    Relevance,
    /// Best matches first, and among equally good matches the most recently edited.
    RelevanceThenRecent,
    /// Most recently edited first, whatever the quality of the match.
    Recent,
    /// By document name and then in the order of the document.
    Document,
}

/// Why a query cannot be carried out.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct QueryError(pub String);

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for QueryError {}

fn error<T>(message: impl Into<String>) -> Result<T, QueryError> {
    Err(QueryError(message.into()))
}

impl Query {
    /// Checks the query against the limits of this module, as [`parse`] does for text:
    /// every search starts with this.
    pub fn check(&self) -> Result<(), QueryError> {
        let mut parts = 0usize;
        self.check_at(0, &mut parts)
    }

    fn check_at(&self, depth: usize, parts: &mut usize) -> Result<(), QueryError> {
        if depth > MAX_NESTING {
            return error(format!("the query is nested deeper than {MAX_NESTING}"));
        }
        let long = |text: &str| text.len() > MAX_TERM_BYTES;
        match self {
            Query::All => {}
            Query::Term(term) => {
                *parts = parts.saturating_add(1);
                if long(&term.text) {
                    return error(format!("a word is longer than {MAX_TERM_BYTES} bytes"));
                }
            }
            Query::Filter(filter) => {
                *parts = parts.saturating_add(1);
                let text = match filter {
                    Filter::Tag(text)
                    | Filter::DocName(text)
                    | Filter::TextIs(text)
                    | Filter::Image(text) => text.as_str(),
                    _ => "",
                };
                if long(text) {
                    return error(format!("a filter is longer than {MAX_TERM_BYTES} bytes"));
                }
                if matches!(filter, Filter::Recent(percent) if !(1..=99).contains(percent)) {
                    return error("recent: takes 1% to 99%");
                }
            }
            Query::And(list) | Query::Or(list) => {
                for query in list {
                    query.check_at(depth.saturating_add(1), parts)?;
                }
            }
            Query::Not(query) => query.check_at(depth.saturating_add(1), parts)?,
        }
        if *parts > MAX_PARTS {
            return error(format!("more than {MAX_PARTS} words and filters"));
        }
        Ok(())
    }

    /// The first plain word or phrase of the query, if it has one: what "replace" takes
    /// as the text to replace.
    pub fn first_term(&self) -> Option<&Term> {
        match self {
            Query::Term(term) => Some(term),
            Query::And(list) | Query::Or(list) => list.iter().find_map(Query::first_term),
            _ => None,
        }
    }
}

/// A query read from text, with the order it asks for, if any (`sort:`).
#[derive(Clone, PartialEq, Debug)]
pub struct Parsed {
    /// The query.
    pub query: Query,
    /// The order asked for with `sort:`.
    pub order: Option<Order>,
}

/// Reads the text syntax. `now` is the current time in milliseconds since 1970 UTC, for
/// `edited:7d` and the like (the core does not read the clock).
pub fn parse(text: &str, now: i64) -> Result<Parsed, QueryError> {
    if text.len() > MAX_QUERY_BYTES {
        return error(format!("the query is longer than {MAX_QUERY_BYTES} bytes"));
    }
    let mut parser = Parser {
        chars: text.chars().collect(),
        at: 0,
        now,
        order: None,
    };
    let query = parser.or(0)?;
    query.check()?;
    Ok(Parsed {
        query,
        order: parser.order,
    })
}

struct Parser {
    chars: Vec<char>,
    at: usize,
    now: i64,
    order: Option<Order>,
}

/// One word of the text: its characters, and whether any of it was in quotes.
struct Word {
    text: String,
    /// The part before the first colon outside quotes, if there is one.
    key: Option<String>,
    /// The part after that colon.
    value: String,
    quoted: bool,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn skip_spaces(&mut self) {
        while self
            .peek()
            .is_some_and(|c| c.is_whitespace() || c.is_control())
        {
            self.at = self.at.saturating_add(1);
        }
    }

    fn or(&mut self, depth: usize) -> Result<Query, QueryError> {
        if depth > MAX_NESTING {
            return error(format!("the query is nested deeper than {MAX_NESTING}"));
        }
        let mut any = Vec::new();
        loop {
            let mut all = Vec::new();
            let mut more = false;
            loop {
                self.skip_spaces();
                match self.peek() {
                    None => break,
                    Some(')') => {
                        // The end of a group; one too many at the top is ignored.
                        if depth > 0 {
                            break;
                        }
                        self.at = self.at.saturating_add(1);
                    }
                    Some('|') => {
                        self.at = self.at.saturating_add(1);
                        more = true;
                        break;
                    }
                    Some(_) => {
                        if self.keyword("OR") {
                            more = true;
                            break;
                        }
                        if let Some(part) = self.unary(depth)? {
                            all.push(part);
                        }
                    }
                }
            }
            any.push(all);
            if !more {
                break;
            }
        }
        // An empty side of an OR counts for nothing: `a OR` is `a`.
        let mut sides: Vec<Query> = any
            .into_iter()
            .filter(|all| !all.is_empty())
            .map(|mut all| match all.len() {
                1 => all.pop().unwrap_or(Query::All),
                _ => Query::And(all),
            })
            .collect();
        Ok(match sides.len() {
            0 => Query::All,
            1 => sides.pop().unwrap_or(Query::All),
            _ => Query::Or(sides),
        })
    }

    /// Whether the word at the cursor is exactly `word`; if so, it is passed over.
    fn keyword(&mut self, word: &str) -> bool {
        let end = self.at.saturating_add(word.chars().count());
        let is = self
            .chars
            .get(self.at..end)
            .is_some_and(|c| c.iter().copied().eq(word.chars()))
            && self
                .chars
                .get(end)
                .is_none_or(|c| c.is_whitespace() || matches!(c, '(' | ')'));
        if is {
            self.at = end;
        }
        is
    }

    fn unary(&mut self, depth: usize) -> Result<Option<Query>, QueryError> {
        match self.peek() {
            Some('-')
                if self
                    .chars
                    .get(self.at.saturating_add(1))
                    .is_some_and(|c| !c.is_whitespace()) =>
            {
                self.at = self.at.saturating_add(1);
                if depth >= MAX_NESTING {
                    return error(format!("the query is nested deeper than {MAX_NESTING}"));
                }
                let inner = self.unary(depth.saturating_add(1))?;
                Ok(inner.map(|query| Query::Not(Box::new(query))))
            }
            Some('(') => {
                self.at = self.at.saturating_add(1);
                let inner = self.or(depth.saturating_add(1))?;
                if self.peek() == Some(')') {
                    self.at = self.at.saturating_add(1);
                }
                Ok(Some(inner))
            }
            _ => {
                let word = self.word();
                self.atom(word)
            }
        }
    }

    /// Reads up to the next space or bracket outside quotes. A quote left open runs to
    /// the end of the text.
    fn word(&mut self) -> Word {
        let mut word = Word {
            text: String::new(),
            key: None,
            value: String::new(),
            quoted: false,
        };
        let mut in_quotes = false;
        while let Some(c) = self.peek() {
            if c == '"' {
                in_quotes = !in_quotes;
                word.quoted = true;
            } else if !in_quotes
                && (c.is_whitespace() || c.is_control() || matches!(c, '(' | ')' | '|'))
            {
                break;
            } else if !in_quotes && c == ':' && word.key.is_none() && !word.quoted {
                word.key = Some(std::mem::take(&mut word.text));
            } else {
                // Control characters cannot be in cell text; a space is what they become.
                word.text.push(if c.is_control() { ' ' } else { c });
            }
            self.at = self.at.saturating_add(1);
        }
        if word.key.is_some() {
            word.value = std::mem::take(&mut word.text);
        }
        word
    }

    fn atom(&mut self, word: Word) -> Result<Option<Query>, QueryError> {
        let Word {
            text,
            key,
            value,
            quoted,
        } = word;
        let Some(key) = key else {
            return Ok((!text.is_empty()).then(|| Query::Term(Term::new(text))));
        };
        let known = [
            "case",
            "tag",
            "has",
            "is",
            "kind",
            "bg",
            "fg",
            "style",
            "in",
            "doc",
            "links-to",
            "linked-from",
            "edited",
            "recent",
            "text",
            "image",
            "sort",
        ];
        let lower = key.to_ascii_lowercase();
        if !known.contains(&lower.as_str()) {
            // Not a filter: a word with a colon in it, such as a web address.
            let whole = format!("{key}:{value}");
            return Ok(Some(Query::Term(Term::new(whole))));
        }
        if value.is_empty() && !(quoted && lower == "text") {
            return error(format!("{lower}: needs a value, as in {}", example(&lower)));
        }
        let bad = || {
            error(format!(
                "{lower}:{value} is not understood; try {}",
                example(&lower)
            ))
        };
        let cell = |value: &str| CellId::parse(value);
        let filter = match lower.as_str() {
            "case" => return Ok(Some(Query::Term(Term::exact(value)))),
            "sort" => {
                self.order = Some(match value.to_ascii_lowercase().as_str() {
                    "relevance" | "best" => Order::Relevance,
                    "recent" | "newest" => Order::Recent,
                    "relevance-recent" => Order::RelevanceThenRecent,
                    "document" | "doc" => Order::Document,
                    _ => return bad(),
                });
                return Ok(None);
            }
            "tag" => Filter::Tag(value),
            "text" => Filter::TextIs(value),
            "image" => Filter::Image(value),
            "doc" => Filter::DocName(value),
            "has" | "is" => Filter::Has(match value.to_ascii_lowercase().as_str() {
                "text" => Has::Text,
                "note" => Has::Note,
                "image" => Has::Image,
                "grid" => Has::Grid,
                "link" => Has::Link,
                "broken-link" | "broken" => Has::BrokenLink,
                "tag" | "tagged" => Has::Tag,
                _ => return bad(),
            }),
            "kind" => match CellKind::parse(&value.to_ascii_lowercase()) {
                Some(kind) => Filter::Kind(kind),
                None => return bad(),
            },
            "bg" | "fg" => {
                let Some(color) = Color::parse(&value.to_ascii_lowercase()) else {
                    return bad();
                };
                if lower == "bg" {
                    Filter::CellColor(color)
                } else {
                    Filter::TextColor(color)
                }
            }
            "style" => {
                let mut style = Style::default();
                for name in value.to_ascii_lowercase().split(['+', ',']) {
                    match name {
                        "bold" => style.bold = true,
                        "italic" => style.italic = true,
                        "fixed" => style.fixed = true,
                        "underline" => style.underline = true,
                        "strike" => style.strike = true,
                        "none" | "plain" => {}
                        _ => return bad(),
                    }
                }
                Filter::Style(style)
            }
            "in" | "links-to" | "linked-from" => {
                let Some(id) = cell(&value) else {
                    return bad();
                };
                match lower.as_str() {
                    "in" => Filter::Within(id),
                    "links-to" => Filter::LinksTo(id),
                    _ => Filter::LinkedFrom(id),
                }
            }
            "edited" => match edited(&value, self.now) {
                Some((from, to)) => Filter::Edited { from, to },
                None => return bad(),
            },
            "recent" => {
                let digits = value.strip_suffix('%').unwrap_or(&value);
                match digits.parse::<u8>() {
                    Ok(percent) if (1..=99).contains(&percent) => Filter::Recent(percent),
                    _ => return bad(),
                }
            }
            _ => return bad(),
        };
        Ok(Some(Query::Filter(filter)))
    }
}

fn example(key: &str) -> &'static str {
    match key {
        "case" => "case:Word",
        "tag" => "tag:todo",
        "has" | "is" => "has:note, has:image, has:grid, has:link, has:broken-link, has:tag, has:text",
        "kind" => "kind:data, kind:code, kind:formula, kind:var_assign, kind:var_read, kind:view_h, kind:view_v",
        "bg" => "bg:#ff0000",
        "fg" => "fg:#000000",
        "style" => "style:bold, style:bold+italic, style:none (also fixed, underline, strike)",
        "in" => "in: with the ID of a cell",
        "links-to" => "links-to: with the ID of a cell",
        "linked-from" => "linked-from: with the ID of a cell",
        "doc" => "doc:notes",
        "edited" => "edited:7d, edited:12h, edited:2026-01-31, edited:2026-01-01..2026-01-31, edited:2026-01-01..",
        "recent" => "recent:5%",
        "text" => "text:\"exactly this\"",
        "image" => "image:photo.png",
        _ => "sort:relevance, sort:recent, sort:relevance-recent, sort:document",
    }
}

const DAY: i64 = 86_400_000;

/// The start of a day given as `YYYY-MM-DD`, in milliseconds since 1970 UTC.
fn day(text: &str) -> Option<i64> {
    let mut parts = text.split('-');
    let (year, month, day) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || year.len() != 4 || month.len() != 2 || day.len() != 2 {
        return None;
    }
    let digits = |text: &str| {
        text.bytes()
            .all(|b| b.is_ascii_digit())
            .then(|| text.parse::<i64>().ok())?
    };
    let (year, month, day) = (digits(year)?, digits(month)?, digits(day)?);
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let in_month = *days.get(usize::try_from(month.checked_sub(1)?).ok()?)?;
    if day < 1 || day > in_month {
        return None;
    }
    // Days since 1970-01-01 (the civil-from-days rule, for the proleptic Gregorian calendar).
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let of_era = y.rem_euclid(400);
    let of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let days = era * 146_097 + of_era * 365 + of_era / 4 - of_era / 100 + of_year - 719_468;
    days.checked_mul(DAY)
}

/// The time span of an `edited:` value.
fn edited(value: &str, now: i64) -> Option<(Option<i64>, Option<i64>)> {
    let value = value.to_ascii_lowercase();
    // A length of time back from now: 90m, 12h, 7d, 2w.
    if let Some(unit) = value.chars().last().filter(|c| "mhdw".contains(*c)) {
        let digits = value.strip_suffix(unit)?;
        if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
            let count: i64 = digits.parse().ok()?;
            let each = match unit {
                'm' => 60_000,
                'h' => 3_600_000,
                'd' => DAY,
                _ => 7 * DAY,
            };
            let from = now.checked_sub(count.checked_mul(each)?)?;
            return Some((Some(from), None));
        }
    }
    let end_of = |start: i64| start.checked_add(DAY - 1);
    match value.split_once("..") {
        None => {
            let start = day(&value)?;
            Some((Some(start), Some(end_of(start)?)))
        }
        Some(("", "")) => None,
        Some((from, to)) => {
            let from = if from.is_empty() {
                None
            } else {
                Some(day(from)?)
            };
            let to = if to.is_empty() {
                None
            } else {
                Some(end_of(day(to)?)?)
            };
            Some((from, to))
        }
    }
}
