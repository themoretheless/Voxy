//! Lexical scanner over the workspace sources.
//!
//! Understands just enough Rust syntax to walk module trees and measure
//! functions without a parser dependency: comments and literals are blanked
//! first, then braces, `mod` and `fn` are matched on what is left.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

/// How a source file is reached from its crate root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// Compiled in normal builds.
    Production,
    /// Only reachable through `#[cfg(test)]` module declarations.
    TestOnly,
}

/// One `mod name;` declaration (a module whose body lives in another file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModDecl {
    /// Enclosing inline modules, outermost first.
    pub inline_path: Vec<String>,
    pub name: String,
    /// Value of a `#[path = "..."]` attribute on the declaration.
    pub path_attr: Option<String>,
    /// The declaration or an enclosing inline module is `#[cfg(test)]`.
    pub cfg_test: bool,
}

/// One function with a body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    /// 1-based line of the `fn` keyword.
    pub line: usize,
    /// Lines from the `fn` keyword to the closing brace, inclusive.
    pub lines: usize,
    /// Parameters including a `self` receiver.
    pub params: usize,
    /// Declared `#[test]` or located inside a `#[cfg(test)]` item.
    pub test: bool,
}

/// Returns a copy of `text` in which comment bodies and the contents of string
/// and character literals are spaces. Newlines and byte offsets are preserved,
/// so an offset into the result is an offset into `text`.
#[must_use]
pub fn blank_non_code(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = bytes.to_vec();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                let end = find_byte(bytes, i, b'\n').unwrap_or(bytes.len());
                blank(&mut out, i, end);
                i = end;
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                let end = block_comment_end(bytes, i);
                blank(&mut out, i, end);
                i = end;
            }
            b'"' => {
                let (content_end, end) = match raw_hashes(bytes, i) {
                    Some(hashes) => raw_string_end(bytes, i + 1, hashes),
                    None => string_end(bytes, i + 1),
                };
                blank(&mut out, i + 1, content_end);
                i = end;
            }
            b'\'' => match char_literal_end(bytes, i) {
                Some(end) => {
                    blank(&mut out, i + 1, end - 1);
                    i = end;
                }
                None => i += 1,
            },
            _ => i += 1,
        }
    }
    // Every blanked range starts and ends at an ASCII delimiter, so multi-byte
    // sequences are replaced whole and the buffer stays valid UTF-8.
    String::from_utf8(out).unwrap_or_default()
}

/// Lists the `mod name;` declarations of a file. `code` must be
/// [`blank_non_code`] of `text`.
#[must_use]
pub fn module_declarations(text: &str, code: &str) -> Vec<ModDecl> {
    let bytes = code.as_bytes();
    let mut declarations = Vec::new();
    // (name, brace depth inside the module body, cfg(test))
    let mut inline: Vec<(String, usize, bool)> = Vec::new();
    let mut depth = 0_usize;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth = depth.saturating_sub(1);
                while inline.last().is_some_and(|(_, open, _)| *open > depth) {
                    inline.pop();
                }
            }
            b'm' if keyword_at(bytes, i, b"mod") => {
                if let Some((name, after)) = identifier_after(bytes, i + 3) {
                    let prefix = item_prefix(bytes, i);
                    let cfg_test = is_cfg_test(&code[prefix..i]);
                    match bytes.get(skip_whitespace(bytes, after)) {
                        Some(b';') => declarations.push(ModDecl {
                            inline_path: inline.iter().map(|(n, _, _)| n.clone()).collect(),
                            name,
                            path_attr: path_attribute(text, code, prefix, i),
                            cfg_test: cfg_test || inline.iter().any(|(_, _, test)| *test),
                        }),
                        Some(b'{') => inline.push((name, depth + 1, cfg_test)),
                        _ => {}
                    }
                    i = after;
                    continue;
                }
            }
            _ => {}
        }
        i += 1;
    }
    declarations
}

/// Paths of the Rust sources a file pulls in with `include!("...")`.
#[must_use]
pub fn included_sources(text: &str, code: &str) -> Vec<String> {
    const MACRO: &str = "include!(";
    let mut paths = Vec::new();
    let mut from = 0;
    while let Some(found) = code[from..].find(MACRO) {
        let argument = from + found + MACRO.len();
        from = argument;
        let Some(open) = code[argument..].find('"').map(|offset| argument + offset) else {
            break;
        };
        let Some(close) = code[open + 1..].find('"').map(|offset| open + 1 + offset) else {
            break;
        };
        let path = &text[open + 1..close];
        let is_rust = Path::new(path).extension().is_some_and(|ext| ext == "rs");
        if code[argument..open].trim().is_empty() && is_rust {
            paths.push(path.to_owned());
        }
    }
    paths
}

/// Walks the module tree rooted at `root` (a `lib.rs`, `main.rs` or binary
/// source) and returns every file it reaches, including `root`.
#[must_use]
pub fn module_tree(root: &Path) -> BTreeMap<PathBuf, Reach> {
    let mut reached = BTreeMap::new();
    walk_module(&normalize(root), true, Reach::Production, &mut reached);
    reached
}

fn walk_module(file: &Path, owns_dir: bool, reach: Reach, reached: &mut BTreeMap<PathBuf, Reach>) {
    match reached.get(file) {
        Some(Reach::Production) => return,
        Some(Reach::TestOnly) if reach == Reach::TestOnly => return,
        _ => {}
    }
    let Ok(text) = fs::read_to_string(file) else {
        return;
    };
    reached.insert(file.to_path_buf(), reach);
    let code = blank_non_code(&text);
    let dir = file.parent().unwrap_or_else(|| Path::new(""));
    for included in included_sources(&text, &code) {
        let target = normalize(&dir.join(included));
        if reached.get(&target) != Some(&Reach::Production) {
            reached.insert(target, reach);
        }
    }
    // A `mod.rs`, crate root or `#[path]` file owns its directory; any other
    // module file keeps its children in a directory named after itself.
    let child_dir = if owns_dir {
        dir.to_path_buf()
    } else {
        dir.join(file.file_stem().unwrap_or_default())
    };
    for declaration in module_declarations(&text, &code) {
        let child_reach = if declaration.cfg_test {
            Reach::TestOnly
        } else {
            reach
        };
        let base = declaration
            .inline_path
            .iter()
            .fold(child_dir.clone(), |path, part| path.join(part));
        if let Some(path) = &declaration.path_attr {
            let target = if declaration.inline_path.is_empty() {
                dir.join(path)
            } else {
                base.join(path)
            };
            walk_module(&normalize(&target), true, child_reach, reached);
            continue;
        }
        let flat = base.join(format!("{}.rs", declaration.name));
        if flat.is_file() {
            walk_module(&normalize(&flat), false, child_reach, reached);
        } else {
            let nested = base.join(&declaration.name).join("mod.rs");
            walk_module(&normalize(&nested), true, child_reach, reached);
        }
    }
}

/// Reach of every `.rs` file under a crate's `src` directory; `None` marks a
/// file outside every module tree of the crate (the compiler never sees it).
#[must_use]
pub fn classify_sources(src: &Path) -> BTreeMap<PathBuf, Option<Reach>> {
    let mut reached: BTreeMap<PathBuf, Reach> = BTreeMap::new();
    for root in crate_roots(src) {
        for (file, reach) in module_tree(&root) {
            let known = reached.entry(file).or_insert(reach);
            if reach == Reach::Production {
                *known = Reach::Production;
            }
        }
    }
    rust_files(src)
        .into_iter()
        .map(|file| {
            let reach = reached.get(&file).copied();
            (file, reach)
        })
        .collect()
}

/// `(name, src directory)` of every crate under `<workspace>/crates`, sorted.
#[must_use]
pub fn workspace_crates(workspace: &Path) -> Vec<(String, PathBuf)> {
    let mut crates: Vec<(String, PathBuf)> = fs::read_dir(workspace.join("crates"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| {
            (
                entry.file_name().to_string_lossy().into_owned(),
                normalize(&entry.path().join("src")),
            )
        })
        .filter(|(_, src)| src.is_dir())
        .collect();
    crates.sort();
    crates
}

/// Crate roots under a `src` directory: `lib.rs`, `main.rs` and `bin/*.rs`.
#[must_use]
pub fn crate_roots(src: &Path) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = ["lib.rs", "main.rs"]
        .iter()
        .map(|name| src.join(name))
        .filter(|path| path.is_file())
        .collect();
    if let Ok(entries) = fs::read_dir(src.join("bin")) {
        let mut binaries: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
            .collect();
        binaries.sort();
        roots.extend(binaries);
    }
    roots
}

/// Every `.rs` file below `dir`, sorted.
#[must_use]
pub fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(normalize(&path));
            }
        }
    }
    files.sort();
    files
}

/// Byte ranges of `#[cfg(test)]` items and `#[test]` functions, merged and
/// sorted. `code` must come from [`blank_non_code`].
#[must_use]
pub fn test_spans(code: &str) -> Vec<(usize, usize)> {
    let bytes = code.as_bytes();
    let mut spans = Vec::new();
    for marker in ["#[cfg(test)]", "#[cfg(all(test", "#[test]"] {
        let mut from = 0;
        while let Some(found) = code[from..].find(marker) {
            let start = from + found;
            from = start + marker.len();
            if let Some(open) = body_open(bytes, from) {
                spans.push((start, matching_close(bytes, open) + 1));
            }
        }
    }
    spans.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (start, end) in spans {
        match merged.last_mut() {
            Some((_, last_end)) if start < *last_end => *last_end = (*last_end).max(end),
            _ => merged.push((start, end)),
        }
    }
    merged
}

/// Lines of `code` outside `#[cfg(test)]` items and `#[test]` functions.
#[must_use]
pub fn production_lines(code: &str) -> usize {
    let test_lines: usize = test_spans(code)
        .iter()
        .map(|&(start, end)| code[start..end].matches('\n').count() + 1)
        .sum();
    code.lines().count().saturating_sub(test_lines)
}

/// Every function with a body in `code` (from [`blank_non_code`]). Nested
/// functions are reported separately and also counted in their parent.
#[must_use]
pub fn functions(code: &str) -> Vec<Function> {
    let bytes = code.as_bytes();
    let spans = test_spans(code);
    let line_starts: Vec<usize> = std::iter::once(0)
        .chain(code.match_indices('\n').map(|(index, _)| index + 1))
        .collect();
    let line_of = |offset: usize| line_starts.partition_point(|&start| start <= offset);
    let mut found = Vec::new();
    let mut i = 0;
    while i + 2 < bytes.len() {
        if bytes[i] != b'f' || !keyword_at(bytes, i, b"fn") {
            i += 1;
            continue;
        }
        let Some((name, after_name)) = identifier_after(bytes, i + 2) else {
            i += 2;
            continue;
        };
        let mut cursor = skip_whitespace(bytes, after_name);
        if bytes.get(cursor) == Some(&b'<') {
            cursor = skip_whitespace(bytes, matching_angle(bytes, cursor) + 1);
        }
        if bytes.get(cursor) != Some(&b'(') {
            i = after_name;
            continue;
        }
        let params_close = matching_close(bytes, cursor);
        let params = parameter_count(&bytes[cursor + 1..params_close.min(bytes.len())]);
        if let Some(open) = body_open(bytes, params_close + 1) {
            let close = matching_close(bytes, open);
            let line = line_of(i);
            found.push(Function {
                name,
                line,
                lines: line_of(close) + 1 - line,
                params,
                test: spans.iter().any(|&(start, end)| (start..end).contains(&i)),
            });
        }
        i = after_name;
    }
    found
}

/// Resolves `.` and `..` lexically so equal files compare equal.
#[must_use]
pub fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other),
        }
    }
    out
}

fn blank(out: &mut [u8], start: usize, end: usize) {
    let end = end.min(out.len());
    if start < end {
        for byte in &mut out[start..end] {
            if *byte != b'\n' {
                *byte = b' ';
            }
        }
    }
}

fn find_byte(bytes: &[u8], from: usize, needle: u8) -> Option<usize> {
    bytes[from..]
        .iter()
        .position(|&byte| byte == needle)
        .map(|offset| from + offset)
}

fn block_comment_end(bytes: &[u8], start: usize) -> usize {
    let mut depth = 0_usize;
    let mut i = start;
    while i < bytes.len() {
        if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
            depth += 1;
            i += 2;
        } else if bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/') {
            depth -= 1;
            i += 2;
            if depth == 0 {
                return i;
            }
        } else {
            i += 1;
        }
    }
    bytes.len()
}

/// Number of `#` of a raw string whose opening quote is at `quote`, if the
/// quote opens a raw string at all.
fn raw_hashes(bytes: &[u8], quote: usize) -> Option<usize> {
    let mut start = quote;
    while start > 0 && bytes[start - 1] == b'#' {
        start -= 1;
    }
    if start == 0 || bytes[start - 1] != b'r' {
        return None;
    }
    let r = start - 1;
    let standalone = |index: usize| index == 0 || !is_identifier(bytes[index - 1]);
    let prefixed = r > 0 && matches!(bytes[r - 1], b'b' | b'c') && standalone(r - 1);
    (standalone(r) || prefixed).then_some(quote - start)
}

/// Returns (end of contents, end of literal) for a string starting after its
/// opening quote.
fn string_end(bytes: &[u8], start: usize) -> (usize, usize) {
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'"' => return (i, i + 1),
            _ => i += 1,
        }
    }
    (bytes.len(), bytes.len())
}

fn raw_string_end(bytes: &[u8], start: usize, hashes: usize) -> (usize, usize) {
    let mut i = start;
    while let Some(quote) = find_byte(bytes, i, b'"') {
        let closing = &bytes[quote + 1..];
        if closing.len() >= hashes && closing[..hashes].iter().all(|&byte| byte == b'#') {
            return (quote, quote + 1 + hashes);
        }
        i = quote + 1;
    }
    (bytes.len(), bytes.len())
}

/// End of the character literal opening at `quote`, or `None` for a lifetime.
fn char_literal_end(bytes: &[u8], quote: usize) -> Option<usize> {
    let first = *bytes.get(quote + 1)?;
    if first == b'\\' {
        // Longest escape is `'\u{10FFFF}'`.
        let limit = (quote + 14).min(bytes.len());
        return (quote + 3..limit)
            .find(|&index| bytes[index] == b'\'')
            .map(|index| index + 1);
    }
    if first == b'\'' {
        return None;
    }
    let width = match first {
        0xF0..=0xFF => 4,
        0xE0..=0xEF => 3,
        0xC0..=0xDF => 2,
        _ => 1,
    };
    (bytes.get(quote + 1 + width) == Some(&b'\'')).then_some(quote + 2 + width)
}

fn is_identifier(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte >= 0x80
}

/// `keyword` starts at `index` as a whole word followed by whitespace.
fn keyword_at(bytes: &[u8], index: usize, keyword: &[u8]) -> bool {
    bytes[index..].starts_with(keyword)
        && (index == 0 || !is_identifier(bytes[index - 1]))
        && bytes
            .get(index + keyword.len())
            .is_some_and(u8::is_ascii_whitespace)
}

fn skip_whitespace(bytes: &[u8], mut index: usize) -> usize {
    while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
        index += 1;
    }
    index
}

/// Identifier after optional whitespace, with the offset just past it.
fn identifier_after(bytes: &[u8], from: usize) -> Option<(String, usize)> {
    let start = skip_whitespace(bytes, from);
    let mut end = start;
    while bytes.get(end).is_some_and(|&byte| is_identifier(byte)) {
        end += 1;
    }
    (end > start && !bytes[start].is_ascii_digit()).then(|| {
        (
            String::from_utf8_lossy(&bytes[start..end]).into_owned(),
            end,
        )
    })
}

/// Start of the attributes and visibility that precede the item at `index`.
fn item_prefix(bytes: &[u8], index: usize) -> usize {
    bytes[..index]
        .iter()
        .rposition(|byte| matches!(byte, b';' | b'{' | b'}'))
        .map_or(0, |boundary| boundary + 1)
}

fn is_cfg_test(prefix: &str) -> bool {
    prefix.contains("cfg(test)") || prefix.contains("cfg(all(test")
}

fn path_attribute(text: &str, code: &str, prefix: usize, item: usize) -> Option<String> {
    let attribute = prefix + code[prefix..item].find("#[path")?;
    let open = attribute + code[attribute..item].find('"')?;
    let close = open + 1 + code[open + 1..item].find('"')?;
    Some(text[open + 1..close].to_owned())
}

/// Offset of the `{` that opens the body of the item whose header starts at
/// `from`, or `None` when the item ends with `;` first.
fn body_open(bytes: &[u8], from: usize) -> Option<usize> {
    let mut nesting = 0_usize;
    for (offset, &byte) in bytes.iter().enumerate().skip(from) {
        match byte {
            b'(' | b'[' => nesting += 1,
            b')' | b']' => nesting = nesting.saturating_sub(1),
            b'{' if nesting == 0 => return Some(offset),
            b';' if nesting == 0 => return None,
            _ => {}
        }
    }
    None
}

/// Offset of the bracket closing the one at `open` (`(`, `[` or `{`).
fn matching_close(bytes: &[u8], open: usize) -> usize {
    let mut depth = 0_usize;
    for (offset, &byte) in bytes.iter().enumerate().skip(open) {
        match byte {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return offset;
                }
            }
            _ => {}
        }
    }
    bytes.len().saturating_sub(1)
}

/// Offset of the `>` closing the generics opened at `open`; `->` is skipped.
fn matching_angle(bytes: &[u8], open: usize) -> usize {
    let mut depth = 0_usize;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'-' if bytes.get(i + 1) == Some(&b'>') => i += 1,
            b'<' => depth += 1,
            b'>' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return i;
                }
            }
            b'(' | b'[' | b'{' => i = matching_close(bytes, i),
            _ => {}
        }
        i += 1;
    }
    bytes.len().saturating_sub(1)
}

/// Counts comma-separated parameters at the top nesting level.
fn parameter_count(list: &[u8]) -> usize {
    let mut count = 0;
    let mut nesting = 0_usize;
    let mut pending = false;
    let mut i = 0;
    while i < list.len() {
        match list[i] {
            b'-' if list.get(i + 1) == Some(&b'>') => i += 1,
            b'(' | b'[' | b'{' | b'<' => nesting += 1,
            b')' | b']' | b'}' | b'>' => nesting = nesting.saturating_sub(1),
            b',' if nesting == 0 => {
                count += usize::from(pending);
                pending = false;
            }
            byte if !byte.is_ascii_whitespace() => pending = true,
            _ => {}
        }
        i += 1;
    }
    count + usize::from(pending)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literals_and_comments_are_blanked_without_moving_offsets() {
        let text = "let a = \"{ // }\"; // } trailing\nlet b = '{'; /* { */ let c: &'static str = r#\"}\"#;";
        let code = blank_non_code(text);
        assert_eq!(code.len(), text.len());
        assert!(!code.contains('{') && !code.contains('}'));
        assert!(code.contains("&'static str"));
        assert_eq!(code.matches('\n').count(), 1);
    }

    #[test]
    fn declarations_carry_inline_parents_path_and_cfg_test() {
        let text = "mod a;\npub(crate) mod app {\n    mod viewport;\n}\n\
                    #[cfg(test)]\n#[path = \"x_tests.rs\"]\nmod x;\n\
                    #[cfg(test)]\nmod tests {\n    mod nested;\n}\nmod inline_only {}\n";
        let declarations = module_declarations(text, &blank_non_code(text));
        let summary: Vec<_> = declarations
            .iter()
            .map(|d| {
                (
                    d.inline_path.join("/"),
                    d.name.as_str(),
                    d.path_attr.as_deref(),
                    d.cfg_test,
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                (String::new(), "a", None, false),
                ("app".to_owned(), "viewport", None, false),
                (String::new(), "x", Some("x_tests.rs"), true),
                ("tests".to_owned(), "nested", None, true),
            ]
        );
    }

    #[test]
    fn functions_report_length_parameters_and_test_scope() {
        let text = "trait T { fn declared(&self); }\n\
                    pub fn generic<F: Fn(u32) -> u32>(f: F, pair: (u8, u8)) -> u32 {\n    f(1)\n}\n\
                    #[cfg(test)]\nmod tests {\n    #[test]\n    fn checks() {}\n}\n\
                    type Callback = fn(u32) -> u32;\n";
        let found = functions(&blank_non_code(text));
        let summary: Vec<_> = found
            .iter()
            .map(|f| (f.name.as_str(), f.line, f.lines, f.params, f.test))
            .collect();
        assert_eq!(
            summary,
            [("generic", 2, 3, 2, false), ("checks", 8, 1, 0, true)]
        );
    }

    #[test]
    fn production_lines_exclude_test_items() {
        let text =
            "fn a() {}\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\nfn b() {}\n";
        assert_eq!(production_lines(&blank_non_code(text)), 2);
    }

    #[test]
    fn included_rust_sources_are_listed() {
        let text = "include!(\"fixture.rs\");\nconst S: &str = include_str!(\"a.wgsl\");\ninclude!(concat!(env!(\"OUT_DIR\"), \"/gen.rs\"));";
        assert_eq!(
            included_sources(text, &blank_non_code(text)),
            ["fixture.rs"]
        );
    }

    #[test]
    fn paths_normalize_lexically() {
        assert_eq!(
            normalize(Path::new("a/b/../c/./d.rs")),
            PathBuf::from("a/c/d.rs")
        );
    }
}
