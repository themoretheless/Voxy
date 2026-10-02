//! Parse and validate every repository WGSL source with the renderer's Naga version.
//! This is source validation; device pipeline creation and GPU execution are separate gates.
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
};
use wgpu::naga::{
    front::wgsl,
    valid::{Capabilities, ValidationFlags, Validator},
};

fn collect(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            collect(&entry.path(), files)?;
        } else if kind.is_file() && entry.path().extension().is_some_and(|ext| ext == "wgsl") {
            files.push(entry.path());
        }
    }
    Ok(())
}

fn validate(source: &str) -> Result<usize, String> {
    let module = wgsl::parse_str(source).map_err(|error| error.emit_to_string(source))?;
    Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .map_err(|error| error.emit_to_string(source))?;
    if module.entry_points.is_empty() {
        return Err("shader has no entrypoints".into());
    }
    Ok(module.entry_points.len())
}

fn main() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    collect(&root, &mut files)?;
    files.sort();
    if files.is_empty() {
        return Err("no WGSL sources found".into());
    }
    let mut failures = 0;
    let mut variants = 0;
    let mut entrypoints = 0;
    for file in &files {
        let source = fs::read_to_string(file)?;
        let relative = file.strip_prefix(&root)?;
        let mut sources = vec![("original", source.clone())];
        if source.contains("rgba16float") {
            sources.push(("rgba32float", source.replace("rgba16float", "rgba32float")));
        }
        for (variant, text) in sources {
            variants += 1;
            match validate(&text) {
                Ok(count) => {
                    entrypoints += count;
                    println!(
                        "PASS {} [{variant}] {count} entrypoints",
                        relative.display()
                    );
                }
                Err(error) => {
                    failures += 1;
                    eprintln!("FAIL {} [{variant}]\n{error}", relative.display());
                }
            }
        }
    }
    println!(
        "WGSL inventory: {} files, {variants} variants, {entrypoints} validated entrypoints, {failures} failures; no GPU execution",
        files.len()
    );
    if failures != 0 {
        return Err("WGSL source validation failed".into());
    }
    Ok(())
}
