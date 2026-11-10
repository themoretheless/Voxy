//! Layering of the top-level modules inside `crates/physics/src`
//! (`docs/refactor/ARCHITECTURE.md` §3).
//!
//! A module may import (`crate::<module>`) only from modules on an earlier
//! line of [`LAYERS`] or from its own line. Edges that break the order today
//! are listed in [`KNOWN_VIOLATIONS`] with the plan step that removes them;
//! the list may only shrink. Test code is not checked.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use voxy_depgraph::source::{blank_non_code, normalize, rust_files, test_spans};

/// Target layer order, lowest first.
const LAYERS: &[&[&str]] = &[
    &[
        "math",
        "symmetric_eigen",
        "triangle_index",
        "spatial_bounds",
        "positive_transport",
        "heat_exchange",
        "osmotic",
        "rotation",
    ],
    &[
        "gravity",
        "mass_properties",
        "planar",
        "strand",
        "soft_body",
        "secondary_motion",
        "respiration",
        "tissue_surface",
        "static_triangle_world",
        "friction",
        "circulation",
        "suspension",
        "hair",
    ],
    &[
        "astrophysics",
        "spin_path",
        "contact",
        "cohesive",
        "lymph",
        "character",
    ],
    &["rigid_motion"],
    &["liquid", "surface_film"],
    &["biomechanics", "medium", "moisture"],
    &["plasticity", "skin", "tissue"],
    &["wear"],
    &["debris"],
];

/// Upward edges that still exist: (from, to, plan step that removes it).
const KNOWN_VIOLATIONS: &[(&str, &str, &str)] = &[];

fn physics_src() -> PathBuf {
    normalize(Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../physics/src"
    )))
}

/// Top-level module a source file belongs to.
fn owner(src: &Path, file: &Path) -> Option<String> {
    let first = file.strip_prefix(src).ok()?.components().next()?;
    let name = first.as_os_str().to_string_lossy();
    let name = name.strip_suffix(".rs").unwrap_or(&name);
    (name != "lib").then(|| name.to_owned())
}

/// Maps a `crate::` path head to its top-level module (flat aliases included).
fn module_of(head: &str) -> &str {
    if head.starts_with("astrophysics_") {
        "astrophysics"
    } else if head.starts_with("gravity_") {
        "gravity"
    } else {
        head
    }
}

fn identifier_at(code: &[u8], start: usize) -> Option<&str> {
    let end = code[start..]
        .iter()
        .position(|byte| !(byte.is_ascii_alphanumeric() || *byte == b'_'))
        .map_or(code.len(), |offset| start + offset);
    (end > start).then(|| std::str::from_utf8(&code[start..end]).unwrap_or(""))
}

/// Heads of every `crate::x` and `crate::{x, y::z}` path outside test code.
fn crate_heads(code: &str) -> BTreeSet<String> {
    let spans = test_spans(code);
    let bytes = code.as_bytes();
    let mut heads = BTreeSet::new();
    let mut from = 0;
    while let Some(found) = code[from..].find("crate::") {
        let at = from + found;
        from = at + 7;
        let preceded = at > 0 && (bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'_');
        if preceded || spans.iter().any(|&(start, end)| (start..end).contains(&at)) {
            continue;
        }
        if bytes.get(from) == Some(&b'{') {
            let mut depth = 0;
            let mut expect = true;
            for (offset, &byte) in bytes[from..].iter().enumerate() {
                match byte {
                    b'{' => {
                        depth += 1;
                        expect = depth == 1;
                    }
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    b',' if depth == 1 => expect = true,
                    byte if byte.is_ascii_whitespace() => {}
                    _ => {
                        if expect
                            && depth == 1
                            && let Some(head) = identifier_at(bytes, from + offset)
                        {
                            heads.insert(head.to_owned());
                        }
                        expect = false;
                    }
                }
            }
        } else if let Some(head) = identifier_at(bytes, from) {
            heads.insert(head.to_owned());
        }
    }
    heads
}

/// Every edge between distinct top-level modules, with a sample file.
fn edges() -> BTreeMap<(String, String), String> {
    let src = physics_src();
    let mut edges = BTreeMap::new();
    for file in rust_files(&src) {
        let Some(from) = owner(&src, &file) else {
            continue;
        };
        if file.to_string_lossy().contains("test") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&file) else {
            continue;
        };
        for head in crate_heads(&blank_non_code(&text)) {
            let to = module_of(&head).to_owned();
            if to != from && layer(&to).is_some() {
                let sample = file
                    .strip_prefix(&src)
                    .unwrap_or(&file)
                    .display()
                    .to_string();
                edges.entry((from.clone(), to)).or_insert(sample);
            }
        }
    }
    edges
}

fn layer(module: &str) -> Option<usize> {
    LAYERS.iter().position(|line| line.contains(&module))
}

#[test]
fn every_top_level_module_has_a_layer() {
    let src = physics_src();
    let missing: BTreeSet<String> = rust_files(&src)
        .iter()
        .filter_map(|file| owner(&src, file))
        .filter(|module| layer(module).is_none())
        .collect();
    assert!(
        missing.is_empty(),
        "add these physics modules to LAYERS in tests/physics_modules.rs: {missing:?}"
    );
}

#[test]
fn physics_modules_import_only_from_lower_layers() {
    let upward: Vec<String> = edges()
        .into_iter()
        .filter(|((from, to), _)| layer(to) > layer(from))
        .filter(|((from, to), _)| {
            !KNOWN_VIOLATIONS
                .iter()
                .any(|(f, t, _)| f == from && t == to)
        })
        .map(|((from, to), sample)| format!("    (\"{from}\", \"{to}\", \"{sample}\"),"))
        .collect();
    assert!(
        upward.is_empty(),
        "physics modules importing from a higher layer (move the shared item down; \
         see docs/refactor/ARCHITECTURE.md §3):\n{}",
        upward.join("\n")
    );
}

#[test]
fn known_physics_layer_violations_still_exist() {
    let edges = edges();
    let stale: Vec<String> = KNOWN_VIOLATIONS
        .iter()
        .filter(|(from, to, _)| !edges.contains_key(&((*from).to_owned(), (*to).to_owned())))
        .map(|(from, to, step)| format!("{from} -> {to} ({step})"))
        .collect();
    assert!(
        stale.is_empty(),
        "these edges are gone; remove them from KNOWN_VIOLATIONS:\n{}",
        stale.join("\n")
    );
}
