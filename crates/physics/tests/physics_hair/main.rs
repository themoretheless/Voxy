//! Physics integration tests: hair and strands.
//!
//! One binary per domain instead of one per file. A module is one former test
//! file: `cargo test -p physics --test physics_hair <module>::` runs it alone.

mod hair;
mod hair_rebase;
mod strands;
