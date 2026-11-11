//! A made-up workspace of any size with text like real notes: words of mixed lengths
//! from a vocabulary in which a few are common and most are rare, some Chinese,
//! Japanese and Korean, notes, tags, links, colours and edit times. Used by the size
//! tests and by the example that writes such a workspace to a folder.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use wombat_core::construct::Construct;
use wombat_core::{CellId, Color, Document, Id, IdSource, Link, LinkTarget, Tag};

pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

impl IdSource for Rng {
    fn next_id(&mut self) -> Id {
        Id::from_u128((u128::from(self.next()) << 64) | u128::from(self.next()))
    }
}

pub const NOW: i64 = 1_800_000_000_000;
pub const DAY: i64 = 86_400_000;
pub const TAGS: [&str; 4] = ["todo", "done", "idea", "urgent"];

const SYLLABLES: [&str; 40] = [
    "ta", "ri", "no", "mel", "con", "ver", "sta", "pro", "in", "ex", "ar", "lo", "qui", "den",
    "sch", "ump", "er", "an", "tion", "ble", "str", "ou", "ea", "th", "ing", "ly", "wa", "ke",
    "zo", "fju", "é", "ü", "ß", "ñ", "æ", "by", "gra", "pho", "mi", "x",
];
const HAN: &str = "的一是在不了有和人这中大为上个国我以要他时来用们生到作地于出就分对成会可主发年动同工也能下过子说产种面而方后多定行学法所民得经十三之进着等部度家电力里如水化高自二理起小物现实加量都两体制机当使点从业本去把性好应开它合还因由其些然前外天政四日那社义事平形相全表间样与关各重新线内数正心反你明看原又么利比或但质气第向道命此变条只没结解问意建月公无系军很情者最立代想已通并提直题党程展五果料象员革位入常文总次品式活设及管特件长求老头基资边流路级少图山统接知较将组见计别她手角期根论运农指几九区强放决西被干做必战先回则任取据处队南给色光门即保治北造百规热领七海口东导器压志世金增争济阶油思术极交受联什认六共权收证改清己美再采转更单风切打白教速花带安场身车例真务具万每目至达走积示议声报斗完类八离华名确才科张信马节话米整空元况今集温传土许步群广石记需段研界拉林律叫且究观越织装影算低持音众书布复容儿须际商非验连断深难近矿千周委素技备半办青省列习响约支般史感劳便团往酸历市克何除消构府称太准精值号率族维划选标写存候毛亲快效斯院查江型眼王按格养易置派层片始却专状育厂京识适属圆包火住调满县局照参红细引听该铁价严";
const KANA: &str = "あいうえおかきくけこさしすせそたちつてとなにぬねのはひふへほまみむめもやゆよらりるれろわをんアイウエオカキクケコサシスセソタチツテトナニヌネノ";
const HANGUL: &str = "가나다라마바사아자차카타파하고노도로모보소오조초코토포호기니디리미비시이지치키티피히한국어텍스트검색문서";

/// What a workspace is made with.
pub struct Corpus {
    rng: Rng,
    /// The vocabulary: the earlier a word, the more often it is used.
    pub words: Vec<String>,
    han: Vec<char>,
    kana: Vec<char>,
    hangul: Vec<char>,
    /// Bytes of text and notes made so far.
    pub text_bytes: usize,
    /// One of the Chinese texts that were made.
    pub chinese: String,
}

impl Corpus {
    pub fn new(seed: u64) -> Corpus {
        let mut rng = Rng(seed | 1);
        let mut words = Vec::new();
        let mut seen = std::collections::HashSet::new();
        while words.len() < 20_000 {
            // Common words are short.
            let syllables = 1 + rng.below(if words.len() < 200 { 2 } else { 4 });
            let word: String = (0..syllables).map(|_| SYLLABLES[rng.below(40)]).collect();
            if seen.insert(word.clone()) {
                words.push(word);
            }
        }
        Corpus {
            rng,
            words,
            han: HAN.chars().collect(),
            kana: KANA.chars().collect(),
            hangul: HANGUL.chars().collect(),
            text_bytes: 0,
            chinese: String::new(),
        }
    }

    /// A word: the 200 most common ones half of the time, as in real text.
    fn word(&mut self) -> &str {
        let n = match self.rng.below(10) {
            0..=4 => self.rng.below(200),
            5..=7 => self.rng.below(2000),
            _ => self.rng.below(self.words.len()),
        };
        &self.words[n]
    }

    fn sentence(&mut self, most: usize) -> String {
        let count = 1 + self.rng.below(most);
        let mut text = String::new();
        for n in 0..count {
            if n > 0 {
                text.push(' ');
            }
            let capital = n == 0 && self.rng.below(2) == 0;
            let word = self.word().to_owned();
            if capital {
                let mut chars = word.chars();
                text.extend(chars.next().map(|c| c.to_uppercase().to_string()));
                text.push_str(chars.as_str());
            } else {
                text.push_str(&word);
            }
        }
        text
    }

    /// The text of a cell: mostly words, some numbers, about one in twelve in a script
    /// without spaces or in Korean.
    fn text(&mut self) -> String {
        match self.rng.below(48) {
            0 | 1 => {
                let count = 3 + self.rng.below(16);
                let text: String = (0..count)
                    .map(|_| self.han[self.rng.below(self.han.len())])
                    .collect();
                if self.chinese.is_empty() {
                    self.chinese = text.clone();
                }
                text
            }
            2 => {
                let count = 4 + self.rng.below(14);
                let mix = |rng: &mut Rng, han: &[char], kana: &[char]| {
                    if rng.below(3) == 0 {
                        han[rng.below(han.len())]
                    } else {
                        kana[rng.below(kana.len())]
                    }
                };
                (0..count)
                    .map(|_| mix(&mut self.rng, &self.han, &self.kana))
                    .collect()
            }
            3 => {
                let words = 1 + self.rng.below(4);
                let mut text = String::new();
                for n in 0..words {
                    if n > 0 {
                        text.push(' ');
                    }
                    for _ in 0..1 + self.rng.below(4) {
                        text.push(self.hangul[self.rng.below(self.hangul.len())]);
                    }
                }
                text
            }
            4 | 5 => format!("{}.{:02}", self.rng.below(5000), self.rng.below(100)),
            6 => TAGS[self.rng.below(4)].to_owned(),
            7 => String::new(),
            _ => self.sentence(9),
        }
    }

    /// A document of about `cells` cells, nested four deep: sections of tables whose
    /// first cells hold lists.
    pub fn document(&mut self, cells: usize) -> Document {
        let mut ids = Rng(self.rng.next() | 1);
        let mut doc = Document::new(&mut ids);
        let root = doc.root();
        let title = self.sentence(3);
        self.set(&mut doc, root, title);
        let sections = (cells / 250).max(1);
        doc.add_grid(root, 1, sections as u32, &mut ids).unwrap();
        let mut made = 1 + sections;
        let mut all: Vec<CellId> = Vec::new();
        for section in doc.cell(root).unwrap().grid().unwrap().cells().to_vec() {
            let heading = self.sentence(4);
            self.set(&mut doc, section, heading);
            // 250 cells a section: 43 rows of 4, and the lists inside some of them.
            let rows = 43.min((cells / 4).max(1)) as u32;
            doc.add_grid(section, 4, rows, &mut ids).unwrap();
            for cell in doc.cell(section).unwrap().grid().unwrap().cells().to_vec() {
                let text = self.text();
                self.set(&mut doc, cell, text);
                all.push(cell);
                made += 1;
                // Some cells hold a short list.
                if made < cells && self.rng.below(12) == 0 {
                    let items = 2 + self.rng.below(8) as u32;
                    doc.add_grid(cell, 1, items, &mut ids).unwrap();
                    for item in doc.cell(cell).unwrap().grid().unwrap().cells().to_vec() {
                        let text = self.text();
                        self.set(&mut doc, item, text);
                        all.push(item);
                        made += 1;
                    }
                }
            }
        }
        // Links: one cell in fifty links to another cell or to a web address.
        for n in (0..all.len()).step_by(50) {
            let target = if self.rng.below(2) == 0 {
                LinkTarget::External(format!("https://example.org/{}/{}", self.word(), n))
            } else {
                LinkTarget::Cell {
                    doc: None,
                    cell: all[self.rng.below(all.len())],
                }
            };
            let cell = doc.cell_mut(all[n]).unwrap();
            if !cell.text.is_empty() {
                cell.links = vec![Link {
                    start: 0,
                    len: 1,
                    target,
                    extra: Default::default(),
                }];
            }
        }
        for tag in TAGS {
            doc.tags_mut().insert(tag.to_owned(), Tag::default());
        }
        doc
    }

    fn set(&mut self, doc: &mut Document, id: CellId, text: String) {
        let note = (self.rng.below(20) == 0).then(|| {
            let first = self.sentence(12);
            format!("{first}.\n{}", self.sentence(12))
        });
        let cell = doc.cell_mut(id).unwrap();
        self.text_bytes += text.len() + note.as_ref().map_or(0, String::len);
        cell.text = text;
        cell.note = note.unwrap_or_default();
        // Edited at some time in the last two years; one cell in ten never.
        if self.rng.below(10) != 0 {
            cell.last_edit = Some(NOW - (self.rng.next() % (730 * DAY as u64)) as i64);
        }
        if self.rng.below(30) == 0 {
            cell.cell_color = Color::rgb(0xff, 0xe0, 0x80);
        }
        if self.rng.below(40) == 0 {
            cell.style.bold = true;
        }
    }
}

/// The queries of the speed test: what a person types, letter by letter and whole, and
/// every kind of filter. `corpus` gives words that are in the text.
pub fn queries(corpus: &Corpus) -> Vec<String> {
    let common = corpus.words[3].clone();
    let usual = corpus.words[900].clone();
    let rare = corpus.words[15_000].clone();
    let mut list: Vec<String> = Vec::new();
    // Typing a word letter by letter.
    for end in 1..=usual.chars().count() {
        list.push(usual.chars().take(end).collect());
    }
    list.extend([
        common.clone(),
        rare.clone(),
        usual.to_uppercase(),
        format!("{common} {usual}"),
        format!("{} {}", corpus.words[10], corpus.words[30]),
        format!("\"{common} {}\"", corpus.words[7]),
        format!("{usual} OR {rare}"),
        format!("{common} -{}", corpus.words[5]),
        format!("case:{usual}"),
        "zzzzqqq".into(),
        "e".into(),
        "的".into(),
        "的一".into(),
        corpus.chinese.chars().skip(1).take(2).collect(),
        corpus.chinese.chars().take(3).collect(),
        "かき".into(),
        "한국".into(),
        "example.org".into(),
        "12.5".into(),
        "tag:todo".into(),
        format!("tag:urgent {common}"),
        "has:note".into(),
        format!("has:note {usual}"),
        "edited:7d".into(),
        "edited:30d sort:recent".into(),
        format!("{common} edited:90d"),
        "recent:5%".into(),
        "style:bold".into(),
        "bg:#ffe080".into(),
        "has:link".into(),
        "has:broken-link".into(),
        format!("{usual} doc:doc003"),
        format!("{common} sort:recent"),
        format!("{common} sort:document"),
        "".into(),
    ]);
    list
}
