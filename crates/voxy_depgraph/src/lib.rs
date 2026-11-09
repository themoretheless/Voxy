//! Workspace structure ratchets.
//!
//! `cargo test -p voxy_depgraph` checks the crate graph against
//! `docs/architecture.md` §4 and the source tree against the rules of
//! `docs/refactor/ARCHITECTURE.md` §1 (see `tests/`). [`source`] is the scanner
//! those tests and the `refactor_metrics` binary share.

pub mod source;
