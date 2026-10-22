//! Conservative relative intervals from this query owner's checked vertex cache.
#[derive(Clone, Copy, Debug)]
pub(super) struct CacheLayout {
    pub header: u32,
    pub records: u32,
    pub vertices: u32,
    pub slots: u32,
    pub words: u32,
    pub revision: u64,
}
pub(super) fn source(original: &str, layout: CacheLayout) -> String {
    let marker = " var delta:array<WorldInterval,3>;\n";
    assert_eq!(original.matches(marker).count(), 1);
    let begin = original.find(marker).unwrap();
    let end = original[begin..]
        .find(" let direction=WorldInterval3(")
        .unwrap()
        + begin;
    let fallback = &original[begin + marker.len()..end];
    let replacement = format!(
        " let cached_delta=source_cached_relative(slot,va,ray);\n var delta=cached_delta.delta;\n if cached_delta.valid==0u {{\n{fallback} }}\n"
    );
    let modified = format!("{}{}{}", &original[..begin], replacement, &original[end..]);
    let constants = format!(
        "const CR_HEADER:u32={}u;\nconst CR_RECORDS:u32={}u;\nconst CR_VERTICES:u32={}u;\nconst CR_SLOTS:u32={}u;\nconst CR_WORDS:u32={}u;\nconst CR_REVISION:vec2<u32>=vec2<u32>({}u,{}u);\n",
        layout.header,
        layout.records,
        layout.vertices,
        layout.slots,
        layout.words,
        layout.revision as u32,
        (layout.revision >> 32) as u32
    );
    [
        constants,
        include_str!("world_cached_relative.wgsl").to_owned(),
        include_str!("world_cached_relative_read.wgsl").to_owned(),
        modified,
    ]
    .join("\n")
}
