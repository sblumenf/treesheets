//! The Wombat search service (milestone 6; decisions A13, B3, A70-A79).
//!
//! - [`fold`]: the form in which text is compared (case, accents, width).
//! - [`query`]: what to look for, as a value and as text.
//! - [`index`]: what is kept per document, and how it follows edits.
//! - [`engine`]: the index of a set of documents, searching and ranking.
//! - [`naive`]: the same answers by reading every cell of every document, slowly: the
//!   reference that the tests compare the index with.
//! - [`replace`]: find and replace, as one undoable step per document.
//! - [`workspace`]: the index of a workspace folder, with its cache on disk. The only
//!   module here that touches files, and it does so through `wombat_core::scan`.
//!
//! Like the core, nothing here prints, reads the clock or panics on bad input.

pub mod engine;
pub mod fold;
pub mod index;
pub mod naive;
pub mod query;
pub mod replace;
pub mod workspace;

pub use engine::{Field, Hit, Options, Results, SearchIndex, Snippet};
pub use index::{searchable, DocIndex, DocSearch, Searchable};
pub use query::{parse, Filter, Has, Order, Parsed, Query, QueryError, Term};
