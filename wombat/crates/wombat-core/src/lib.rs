//! The Wombat core: nested-grid documents, with no interface.
//!
//! See `docs/PLAN.md` and `docs/DECISIONS.md` in the `wombat` folder.

/// The version of the core library.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_set() {
        assert!(!super::VERSION.is_empty());
    }
}
