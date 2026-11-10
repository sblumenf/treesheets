//! The error type of the core.

use std::fmt;
use std::path::PathBuf;

/// Everything that can go wrong in the core. Nothing in the core panics on bad input.
#[derive(Debug)]
pub enum Error {
    /// A file could not be read or written.
    Io {
        /// The file or folder concerned.
        path: PathBuf,
        /// What the operating system reported.
        source: std::io::Error,
    },
    /// The input is not JSON text (damaged, truncated or not a Wombat file at all).
    Syntax(String),
    /// The input is JSON but does not say it is the expected kind of Wombat file.
    WrongFormat(String),
    /// The file was written in a newer major format version than this core understands.
    NewerVersion {
        /// The major version found in the file.
        found: u64,
        /// The newest major version this core reads.
        supported: u32,
    },
    /// The document breaks a rule of the format or of the model (says which and where).
    Invalid(String),
    /// A command cannot be carried out on the document as it is (says why): a position
    /// outside the text, a cell without the grid the command needs, and the like. The
    /// document is unchanged.
    Refused(String),
    /// A size, count or depth is over a limit in [`crate::limits`].
    Limit(String),
    /// No document with this ID or name is in the workspace.
    NotFound(String),
    /// A document with this name is already in the workspace.
    AlreadyExists(PathBuf),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Error::Syntax(m) => write!(f, "not valid JSON: {m}"),
            Error::WrongFormat(m) => write!(f, "not a Wombat file: {m}"),
            Error::NewerVersion { found, supported } => write!(
                f,
                "the file has format version {found}, but this version of Wombat reads only up \
                 to version {supported}; use a newer Wombat to open it"
            ),
            Error::Invalid(m) => write!(f, "invalid document: {m}"),
            Error::Refused(m) => write!(f, "not possible: {m}"),
            Error::Limit(m) => write!(f, "over a limit: {m}"),
            Error::NotFound(m) => write!(f, "not found: {m}"),
            Error::AlreadyExists(p) => write!(f, "{} already exists", p.display()),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Shorthand for an [`Error::Invalid`].
pub(crate) fn invalid<T>(message: impl Into<String>) -> Result<T, Error> {
    Err(Error::Invalid(message.into()))
}

/// Shorthand for an [`Error::Refused`].
pub(crate) fn refused<T>(message: impl Into<String>) -> Result<T, Error> {
    Err(Error::Refused(message.into()))
}
