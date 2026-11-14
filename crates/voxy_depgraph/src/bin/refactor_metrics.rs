//! Prints size metrics for every workspace crate: production lines, function
//! counts over the length thresholds, long parameter lists and orphan files.
//!
//! `cargo run -p voxy_depgraph --bin refactor_metrics` prints a Markdown
//! summary; `-- --functions` prints one TSV row per production function over a
//! threshold (`path:line`, name, lines, parameters) for `INVENTORY.md`.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use voxy_depgraph::source::{
    Reach, blank_non_code, classify_sources, functions, normalize, production_lines,
    workspace_crates,
};

/// Function lengths reported in the summary columns.
const LENGTH_THRESHOLDS: [usize; 3] = [60, 100, 200];
/// Parameter count above which a function is listed.
const PARAMETER_THRESHOLD: usize = 7;
/// Rows of the "longest functions" table.
const LONGEST_SHOWN: usize = 40;

#[derive(Debug, Default)]
struct CrateMetrics {
    production_lines: usize,
    functions: usize,
    over_length: [usize; 3],
    over_parameters: usize,
    orphans: Vec<PathBuf>,
}

#[derive(Debug)]
struct FunctionRow {
    location: String,
    name: String,
    lines: usize,
    params: usize,
}

fn main() {
    let workspace = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let list_functions = std::env::args().any(|argument| argument == "--functions");
    let (crates, mut rows) = scan(workspace);
    rows.sort_by(|a, b| {
        b.lines
            .cmp(&a.lines)
            .then_with(|| a.location.cmp(&b.location))
    });
    if list_functions {
        print!("{}", function_table(&rows));
    } else {
        print!("{}", summary(&crates, &rows));
    }
}

fn scan(workspace: &Path) -> (BTreeMap<String, CrateMetrics>, Vec<FunctionRow>) {
    let workspace = normalize(workspace);
    let mut crates = BTreeMap::new();
    let mut rows = Vec::new();
    for (name, src) in workspace_crates(&workspace) {
        let mut metrics = CrateMetrics::default();
        for (file, reach) in classify_sources(&src) {
            match reach {
                None => metrics.orphans.push(relative(&workspace, &file)),
                Some(Reach::TestOnly) => {}
                Some(Reach::Production) => measure(&workspace, &file, &mut metrics, &mut rows),
            }
        }
        crates.insert(name, metrics);
    }
    (crates, rows)
}

fn measure(workspace: &Path, file: &Path, metrics: &mut CrateMetrics, rows: &mut Vec<FunctionRow>) {
    let Ok(text) = fs::read_to_string(file) else {
        return;
    };
    let code = blank_non_code(&text);
    metrics.production_lines += production_lines(&code);
    let path = relative(workspace, file);
    for function in functions(&code).into_iter().filter(|f| !f.test) {
        metrics.functions += 1;
        for (count, threshold) in metrics.over_length.iter_mut().zip(LENGTH_THRESHOLDS) {
            *count += usize::from(function.lines > threshold);
        }
        metrics.over_parameters += usize::from(function.params > PARAMETER_THRESHOLD);
        if function.lines > LENGTH_THRESHOLDS[0] || function.params > PARAMETER_THRESHOLD {
            rows.push(FunctionRow {
                location: format!("{}:{}", path.display(), function.line),
                name: function.name,
                lines: function.lines,
                params: function.params,
            });
        }
    }
}

fn relative(workspace: &Path, file: &Path) -> PathBuf {
    file.strip_prefix(workspace)
        .map_or_else(|_| file.to_path_buf(), Path::to_path_buf)
}

fn summary(crates: &BTreeMap<String, CrateMetrics>, rows: &[FunctionRow]) -> String {
    let mut out = String::new();
    let [short, medium, long] = LENGTH_THRESHOLDS;
    let _ = writeln!(
        out,
        "| Crate | Production lines | Functions | > {short} | > {medium} | > {long} | > {PARAMETER_THRESHOLD} params | Orphans |\n\
         |---|---:|---:|---:|---:|---:|---:|---:|"
    );
    let mut ordered: Vec<_> = crates.iter().collect();
    ordered.sort_by_key(|(_, metrics)| std::cmp::Reverse(metrics.production_lines));
    let mut total = CrateMetrics::default();
    for (name, metrics) in ordered {
        let _ = writeln!(out, "{}", summary_row(name, metrics));
        total.production_lines += metrics.production_lines;
        total.functions += metrics.functions;
        total.over_parameters += metrics.over_parameters;
        for (sum, count) in total.over_length.iter_mut().zip(metrics.over_length) {
            *sum += count;
        }
        total.orphans.extend(metrics.orphans.iter().cloned());
    }
    let _ = writeln!(out, "{}", summary_row("**total**", &total));
    let _ = writeln!(out, "\nLongest production functions:\n");
    let _ = writeln!(
        out,
        "| Lines | Params | Function | Location |\n|---:|---:|---|---|"
    );
    for row in rows.iter().take(LONGEST_SHOWN) {
        let _ = writeln!(
            out,
            "| {} | {} | `{}` | `{}` |",
            row.lines, row.params, row.name, row.location
        );
    }
    if !total.orphans.is_empty() {
        let _ = writeln!(out, "\nFiles outside every module tree:\n");
        for orphan in &total.orphans {
            let _ = writeln!(out, "- `{}`", orphan.display());
        }
    }
    out
}

fn summary_row(name: &str, metrics: &CrateMetrics) -> String {
    let [short, medium, long] = metrics.over_length;
    format!(
        "| {name} | {} | {} | {short} | {medium} | {long} | {} | {} |",
        metrics.production_lines,
        metrics.functions,
        metrics.over_parameters,
        metrics.orphans.len()
    )
}

fn function_table(rows: &[FunctionRow]) -> String {
    let mut out = String::from("location\tfunction\tlines\tparams\n");
    for row in rows {
        let _ = writeln!(
            out,
            "{}\t{}\t{}\t{}",
            row.location, row.name, row.lines, row.params
        );
    }
    out
}
