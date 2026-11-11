//! `wombat search`, `replace` and `import-all`: a thin layer over `wombat_search`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use wombat_core::scan::CacheMode;
use wombat_core::{storage, DocId, Document, Editor, Workspace};
use wombat_search::query::{parse, Filter, Order, Query, Term};
use wombat_search::replace::{replace, Scope};
use wombat_search::{workspace, Field, Hit, Options, Snippet};

use crate::links::{find_cell, place_of};
use crate::Failure;

/// The text of `wombat search --help`.
pub const HELP: &str = "\
wombat search <folder | file.wombat> <query...> [options]

Searches every document of the folder (or the one file), open or not, and prints the best
matches first, each with where it is: file, place, text, the cells it is inside of, and
the part that matched with the matches between « and ».

The query (several arguments are joined with spaces; quote it for the shell):
  apple pie            cells that have both words, anywhere: in their text, their note,
                       the name of their image, the address of a link, or the text of a
                       cell that a link of theirs leads to
  app                  a word need not be complete, and need not start a word
  \"apple pie\"          the words together
  apple OR pear        either (also: apple | pear)       -apple   not
  (a OR b) -c          brackets
  case:Apple           exactly these characters; otherwise upper and lower case, accents
                       (é = e) and full-width forms count as the same
Filters, alone or with words:
  tag:NAME             cells whose text is the tag NAME          has:tag    any tag
  has:note  has:image  has:grid  has:link  has:broken-link  has:text
  in:ID                the cell with this ID and everything inside it (or: --in FILE CELL)
  doc:NAME             only the document NAME (with or without .wombat)
  links-to:ID  linked-from:ID
  edited:7d            edited in the last 7 days (also m, h, w)
  edited:2026-01-31    on that day (UTC);  edited:2026-01-01..2026-01-31;  edited:2026-01-01..
  recent:5%            the most recently edited 5% of each document
  kind:formula  bg:#rrggbb  fg:#rrggbb  style:bold+italic  style:none
  text:\"exactly this\"  image:FILE
  sort:relevance (default)  sort:recent  sort:relevance-recent  sort:document

Options:
  --limit N            print at most N results (default 20)
  --offset N           leave out the first N
  --in FILE CELL       only inside this cell of FILE (CELL as in `wombat edit`)
  --next FILE CELL     only the next match after this cell in FILE, in the order of the
  --prev FILE CELL     document, going round at the end (or the previous one)
  --cache              keep the index in <folder>/.wombat-index and use it next time; a
                       document is read again if its length or time has changed
  --cache=verify       the same, but every document is read and compared each time
  --now MILLISECONDS   the time that edited: counts from (default: the clock)
  --time               also print how long indexing and searching took
";

/// The text of `wombat replace --help`.
pub const REPLACE_HELP: &str = "\
wombat replace <folder | file.wombat> <text> <replacement> [options]

Replaces every place where <text> is found in the text of a cell (not in notes), in every
document of the folder or in the one file, and saves the documents that changed. As in
`wombat search`, upper and lower case, accents and full-width forms count as the same.

Options:
  --case               only exactly these characters
  --in FILE CELL       only in this cell of FILE and the cells inside it
  --dry-run            say what would be replaced, and change nothing
";

fn usage<T>(message: impl Into<String>) -> Result<T, Failure> {
    Err(Failure::Usage(message.into()))
}

fn number(word: Option<&&str>, what: &str) -> Result<usize, Failure> {
    let parsed = word.and_then(|word| word.parse().ok());
    parsed.ok_or_else(|| Failure::Usage(format!("{what} takes a number")))
}

/// A folder to search, or one document of it.
struct Target {
    workspace: Workspace,
    /// The file name, if one document was named.
    only: Option<String>,
}

fn target(path: &str) -> Result<Target, Failure> {
    let path = Path::new(path);
    if path.is_dir() {
        return Ok(Target {
            workspace: Workspace::open(path)?,
            only: None,
        });
    }
    let name = path.file_name().and_then(|name| name.to_str());
    let Some(name) = name.filter(|_| path.is_file()) else {
        return Err(Failure::Failed(format!(
            "{}: neither a folder nor a document",
            path.display()
        )));
    };
    let folder = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::from("."),
    };
    Ok(Target {
        workspace: Workspace::open(folder)?,
        only: Some(name.to_owned()),
    })
}

/// The documents of a folder, loaded when a result in them is shown.
struct Loaded<'a> {
    workspace: &'a Workspace,
    names: HashMap<DocId, String>,
    docs: HashMap<DocId, Option<Document>>,
}

impl Loaded<'_> {
    fn doc(&mut self, id: DocId) -> Option<&Document> {
        let (workspace, names) = (self.workspace, &self.names);
        self.docs
            .entry(id)
            .or_insert_with(|| workspace.open_name(names.get(&id)?).ok())
            .as_ref()
    }
}

/// A snippet with its matches between « and », and … where the text goes on.
pub fn marked(snippet: &Snippet) -> String {
    let mut out = String::new();
    if snippet.cut_start {
        out.push('…');
    }
    let mut ranges = snippet.ranges.iter().peekable();
    for (at, c) in snippet.text.chars().enumerate() {
        let at = at as u32;
        if ranges.peek().is_some_and(|range| range.0 == at) {
            out.push('«');
        }
        out.push(c);
        if ranges
            .peek()
            .is_some_and(|range| range.0.saturating_add(range.1) == at.saturating_add(1))
        {
            out.push('»');
            ranges.next();
        }
    }
    if snippet.cut_end {
        out.push('…');
    }
    out
}

fn short(text: &str) -> String {
    let mut out: String = text.chars().take(60).collect();
    if text.chars().nth(60).is_some() {
        out.push('…');
    }
    out
}

fn print_hit(hit: &Hit, loaded: &mut Loaded, out: &mut String) {
    let name = loaded.names.get(&hit.doc).cloned().unwrap_or_default();
    let place = match loaded.doc(hit.doc) {
        Some(doc) => place_of(doc, hit.cell),
        None => format!("#{}", hit.cell),
    };
    let trail: Vec<String> = hit
        .trail
        .iter()
        .map(|text| format!("{:?}", short(text)))
        .collect();
    let inside = if trail.is_empty() {
        String::new()
    } else {
        format!(" (in: {})", trail.join(" > "))
    };
    out.push_str(&format!("{name} {place} {:?}{inside}\n", short(&hit.text)));
    let field = match hit.field {
        Field::Text => "text",
        Field::Shown => "shows",
        Field::Note => "note",
        Field::LinkedText => "links to",
        Field::Url => "link",
        Field::Image => "image",
    };
    if !hit.ranges.is_empty() {
        out.push_str(&format!("    {field}: {}\n", marked(&hit.snippet)));
    }
}

fn milliseconds(started: Instant) -> String {
    format!("{:.1} ms", started.elapsed().as_secs_f64() * 1000.0)
}

/// `wombat search`.
pub fn search(args: &[&str]) -> Result<String, Failure> {
    let mut words: Vec<&str> = Vec::new();
    let (mut limit, mut offset, mut cache, mut time) = (20usize, 0usize, CacheMode::Off, false);
    let mut now: Option<i64> = None;
    let mut within: Option<(&str, &str)> = None;
    let mut next: Option<(&str, &str, bool)> = None;
    let mut rest = args.iter();
    let Some(path) = rest.next() else {
        return usage("search needs a folder or a document");
    };
    while let Some(word) = rest.next() {
        match *word {
            "--limit" => limit = number(rest.next(), "--limit")?,
            "--offset" => offset = number(rest.next(), "--offset")?,
            "--cache" => cache = CacheMode::Fast,
            "--cache=verify" => cache = CacheMode::Verify,
            "--time" => time = true,
            "--now" => {
                let value = rest.next().and_then(|word| word.parse().ok());
                now = Some(value.ok_or_else(|| Failure::Usage("--now takes a number".into()))?);
            }
            "--in" | "--next" | "--prev" => {
                let (Some(file), Some(cell)) = (rest.next(), rest.next()) else {
                    return usage(format!("{word} takes a file and a cell"));
                };
                if *word == "--in" {
                    within = Some((file, cell));
                } else {
                    next = Some((file, cell, *word == "--prev"));
                }
            }
            _ => words.push(word),
        }
    }
    let now = now.unwrap_or_else(|| {
        let since = SystemTime::now().duration_since(UNIX_EPOCH);
        since.map_or(0, |since| since.as_millis() as i64)
    });
    let parsed = parse(&words.join(" "), now).map_err(|e| Failure::Failed(e.to_string()))?;
    let target = target(path)?;

    let started = Instant::now();
    let built = workspace::index(&target.workspace, cache)?;
    let indexing = milliseconds(started);
    let index = built.index;
    let mut loaded = Loaded {
        workspace: &target.workspace,
        names: built
            .names
            .iter()
            .map(|(id, name)| (*id, name.clone()))
            .collect(),
        docs: HashMap::new(),
    };
    let id_of = |name: &str| {
        built
            .names
            .iter()
            .find(|(_, n)| n.as_str() == name)
            .map(|(id, _)| *id)
    };
    let file_name = |file: &str| {
        let name = Path::new(file).file_name().and_then(|name| name.to_str());
        name.map(str::to_owned).unwrap_or_default()
    };

    let mut parts = vec![parsed.query];
    if let Some(only) = &target.only {
        let id = id_of(only).ok_or_else(|| Failure::Failed(format!("{only} could not be read")))?;
        parts.push(Query::Filter(Filter::Doc(id)));
    }
    let mut cell_in = |file: &str, cell: &str| -> Result<(DocId, wombat_core::CellId), Failure> {
        let name = file_name(file);
        let id =
            id_of(&name).ok_or_else(|| Failure::Failed(format!("{name} is not in the folder")))?;
        let doc = loaded
            .doc(id)
            .ok_or_else(|| Failure::Failed(format!("{name} could not be read")))?;
        Ok((id, find_cell(doc, cell).map_err(Failure::Failed)?))
    };
    if let Some((file, cell)) = within {
        let (doc, cell) = cell_in(file, cell)?;
        parts.push(Query::Filter(Filter::Doc(doc)));
        parts.push(Query::Filter(Filter::Within(cell)));
    }
    let from = match next {
        Some((file, cell, reverse)) => Some((cell_in(file, cell)?, reverse)),
        None => None,
    };
    let query = if parts.len() == 1 {
        parts.pop().unwrap_or(Query::All)
    } else {
        Query::And(parts)
    };

    let mut out = String::new();
    let started = Instant::now();
    let failed = |e: wombat_search::QueryError| Failure::Failed(e.to_string());
    if let Some(((doc, cell), reverse)) = from {
        match index
            .next_match(&query, doc, Some(cell), reverse)
            .map_err(failed)?
        {
            Some(hit) => print_hit(&hit, &mut loaded, &mut out),
            None => out.push_str("no match in this document\n"),
        }
    } else {
        let options = Options {
            order: parsed.order.unwrap_or(Order::Relevance),
            offset,
            limit,
        };
        let found = index.search(&query, &options).map_err(failed)?;
        let searching = milliseconds(started);
        for hit in &found.hits {
            print_hit(hit, &mut loaded, &mut out);
        }
        let shown = match (offset, found.hits.len()) {
            (0, n) if n == found.total => String::new(),
            (_, 0) => ", none shown".into(),
            (_, n) => format!(
                ", {} to {} shown",
                offset.saturating_add(1),
                offset.saturating_add(n)
            ),
        };
        out.push_str(&format!(
            "{} results in {} cells of {} documents{shown}\n",
            found.total,
            index.cell_count(),
            index.documents().count()
        ));
        if time {
            out.push_str(&format!(
                "index: {indexing} ({} documents read, {} from the cache); search: {searching}\n",
                built.parsed, built.reused
            ));
        }
    }
    for (name, error) in &built.failed {
        out.push_str(&format!("warning: {name} could not be read: {error}\n"));
    }
    for (id, files) in &built.duplicates {
        out.push_str(&format!(
            "warning: {} have the same document ID {id}; only the first is searched\n",
            files.join(" and ")
        ));
    }
    Ok(out)
}

/// `wombat replace`.
pub fn replace_all(args: &[&str]) -> Result<String, Failure> {
    let mut words: Vec<&str> = Vec::new();
    let (mut exact, mut dry) = (false, false);
    let mut within: Option<(&str, &str)> = None;
    let mut rest = args.iter();
    while let Some(word) = rest.next() {
        match *word {
            "--case" => exact = true,
            "--dry-run" => dry = true,
            "--in" => {
                let (Some(file), Some(cell)) = (rest.next(), rest.next()) else {
                    return usage("--in takes a file and a cell");
                };
                within = Some((file, cell));
            }
            _ => words.push(word),
        }
    }
    let [path, text, with] = words.as_slice() else {
        return usage("replace needs a folder or document, the text and the replacement");
    };
    let term = Term {
        text: (*text).to_owned(),
        exact,
    };
    let target = target(path)?;
    let names = match (&target.only, within) {
        (_, Some((file, _))) => {
            let name = Path::new(file).file_name().and_then(|name| name.to_str());
            vec![name.unwrap_or_default().to_owned()]
        }
        (Some(only), None) => vec![only.clone()],
        (None, None) => {
            let listed = target.workspace.list()?;
            listed.into_iter().map(|entry| entry.name).collect()
        }
    };
    let mut ids = crate::os_ids()?;
    let mut out = String::new();
    let (mut places, mut cells, mut files) = (0usize, 0usize, 0usize);
    for name in names {
        let path = target.workspace.path_of(&name)?;
        let doc = match storage::load_document(&path) {
            Ok(doc) => doc,
            Err(error) => {
                out.push_str(&format!("warning: {name} could not be read: {error}\n"));
                continue;
            }
        };
        let scope = match within {
            Some((_, cell)) => Scope::Branch(find_cell(&doc, cell).map_err(Failure::Failed)?),
            None => Scope::Document,
        };
        let mut editor = Editor::new(doc)?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH);
        let now = now.ok().map(|since| since.as_millis() as i64);
        let done = replace(&mut editor, &term, with, scope, now, &mut ids)
            .map_err(|e| Failure::Failed(format!("{name}: {e}")))?;
        if done.places == 0 {
            continue;
        }
        if !dry && !done.changes.is_empty() {
            storage::save_document(editor.document(), &path)?;
        }
        out.push_str(&format!(
            "{name}: {} places in {} cells\n",
            done.places, done.cells
        ));
        places = places.saturating_add(done.places);
        cells = cells.saturating_add(done.cells);
        files = files.saturating_add(1);
    }
    let verb = if dry { "would be replaced" } else { "replaced" };
    out.push_str(&format!(
        "{places} places in {cells} cells of {files} documents {verb}\n"
    ));
    Ok(out)
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), Failure> {
    let failed = |e: std::io::Error| Failure::Failed(format!("{}: {e}", dir.display()));
    for entry in std::fs::read_dir(dir).map_err(failed)? {
        let path = entry.map_err(failed)?.path();
        if path.is_dir() {
            collect(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "cts") {
            out.push(path);
        }
    }
    Ok(())
}

/// `wombat import-all`: every `.cts` file under a folder, converted into one workspace
/// folder. A convenience for trying search on real documents.
pub fn import_all(source: &str, folder: &str) -> Result<String, Failure> {
    let mut files = Vec::new();
    collect(Path::new(source), &mut files)?;
    files.sort();
    let target = Path::new(folder);
    std::fs::create_dir_all(target).map_err(|e| Failure::Failed(format!("{folder}: {e}")))?;
    let mut ids = crate::os_ids()?;
    let (mut done, mut cells) = (0usize, 0usize);
    let mut out = String::new();
    for file in &files {
        let stem = file
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("document");
        // Files of the same name in different folders get a number.
        let mut path = target.join(format!("{stem}.wombat"));
        let mut n = 1u32;
        while path.exists() {
            n = n.saturating_add(1);
            path = target.join(format!("{stem}-{n}.wombat"));
        }
        match wombat_cts::import_file(file, &mut ids) {
            Ok(imported) => {
                imported.save(&path)?;
                done = done.saturating_add(1);
                cells = cells.saturating_add(imported.document.cell_count());
            }
            Err(error) => out.push_str(&format!("{}: not imported: {error}\n", file.display())),
        }
    }
    out.push_str(&format!(
        "imported {done} of {} files, {cells} cells, into {folder}\n",
        files.len()
    ));
    Ok(out)
}
