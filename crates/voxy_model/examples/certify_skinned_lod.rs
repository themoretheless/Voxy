//! Offline shared-vertex skeletal GLB LOD certification.
//! Usage: certify_skinned_lod BASE.glb VARIANT_INDICES.txt OUTPUT.lod [depth]
use std::{error::Error, path::Path};
use voxy_render::{
    CertifiedLodSubdivisionVariant, LodArchiveLimits, LodSurface, ModelAsset, ModelGeometry,
    ModelLimits,
};
fn read(path: &Path, cap: u64) -> Result<Vec<u8>, Box<dyn Error>> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(cap + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > cap {
        return Err("input byte limit exceeded".into());
    }
    Ok(bytes)
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !(3..=4).contains(&args.len()) {
        return Err(
            "usage: certify_skinned_lod BASE.glb VARIANT_INDICES.txt OUTPUT.lod [depth]".into(),
        );
    }
    let depth: u8 = args.get(3).map_or(Ok(0), |value| value.parse())?;
    if depth > 4 {
        return Err("certificate depth exceeds offline budget".into());
    }
    let bytes = read(Path::new(&args[0]), 64 * 1024 * 1024)?;
    let model = ModelAsset::parse(&bytes, &[], ModelLimits::default())?;
    if model.primitives.len() != 1 {
        return Err("one skeletal primitive required".into());
    }
    let ModelGeometry::Skinned(mesh) = &model.primitives[0].geometry else {
        return Err("skinned primitive required".into());
    };
    let index_bytes = read(Path::new(&args[1]), 8 * 1024 * 1024)?;
    let indices: Vec<u32> = std::str::from_utf8(&index_bytes)?
        .split_whitespace()
        .map(str::parse)
        .collect::<Result<_, _>>()?;
    let positions: Vec<_> = mesh
        .vertices()
        .iter()
        .map(|vertex| vertex.position)
        .collect();
    let base = LodSurface {
        positions: &positions,
        indices: mesh.indices(),
    };
    let variant = LodSurface {
        positions: &positions,
        indices: &indices,
    };
    let (proof, work) = voxy_render::generate_indexed_lod_witnesses(
        base,
        variant,
        depth,
        voxy_render::LodSearchBudget {
            indexed_triangles: 1_000_000,
            output_cells: 262_144,
            triangle_tests: 4_000_000,
            node_visits: 64_000_000,
        },
    )
    .map_err(|error| format!("witness generation: {error:?}"))?;
    eprintln!(
        "certificate_depth={depth} triangle_tests={} node_visits={}",
        work.triangle_tests, work.node_visits
    );
    let limits = LodArchiveLimits {
        bytes: 16 * 1024 * 1024,
        positions: 65_536,
        levels: 8,
        indices: 1_572_864,
        cells: 262_144,
    };
    let archive = voxy_render::encode_lod_archive(
        &positions,
        mesh.indices(),
        &[CertifiedLodSubdivisionVariant {
            indices,
            source_to_variant: proof.source_to_approximation,
            variant_to_source: proof.approximation_to_source,
        }],
        limits,
    )
    .map_err(|error| format!("archive verification: {error:?}"))?;
    voxy_render::decode_skinned_lod_archive(std::sync::Arc::new(mesh.clone()), &archive, limits)?;
    std::fs::write(&args[2], &archive)?;
    println!(
        "skeletal_lod_archive_bytes={} base_indices={}",
        archive.len(),
        mesh.indices().len()
    );
    Ok(())
}
