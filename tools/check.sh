#!/bin/sh
# Fast workspace gate: formatting, lints, structure ratchets.
# Usage: tools/check.sh [crate ...]   (crates limit clippy; default is the workspace)
set -eu
cd "$(dirname "$0")/.."

cargo fmt --all --check
if [ "$#" -eq 0 ]; then
    cargo clippy --workspace --all-targets -- -D warnings
else
    for crate in "$@"; do
        cargo clippy -p "$crate" --all-targets -- -D warnings
    done
fi
cargo test -q -p voxy_depgraph
