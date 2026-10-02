//! Offline verification of a shared-vertex OBJ LOD candidate.
use std::{collections::HashMap, error::Error, io::Read, path::Path};
use voxy_assets::{ArtifactCache, AssetId, ImportInputs};
use voxy_render::{
    CertifiedLodIndexSet, CertifiedLodSubdivisionVariant, CertifiedLodVariant, LodArchiveLimits,
    LodSearchBudget, LodSurface, ObjAsset, ObjLimits, decode_lod_archive, encode_lod_archive,
    generate_indexed_lod_witnesses, generate_lod_witnesses, generate_subdivided_lod_witnesses,
};

type VertexKey = ([u32; 3], [u32; 2], [u32; 4], Option<[u32; 3]>);
fn key(asset: &ObjAsset, index: usize) -> VertexKey {
    let vertex = &asset.mesh.vertices()[index];
    (
        vertex.position.map(f32::to_bits),
        vertex.uv.map(f32::to_bits),
        vertex.color.map(f32::to_bits),
        asset.normals[index].map(|n| n.map(f32::to_bits)),
    )
}
fn read_obj(path: &Path, inputs: &mut ImportInputs) -> Result<ObjAsset, Box<dyn Error>> {
    let limits = ObjLimits::default();
    let path = path.canonicalize()?;
    let snapshot = inputs
        .read(
            AssetId(path.to_string_lossy().into_owned()),
            |_, remaining| {
                let mut bytes = Vec::new();
                std::fs::File::open(&path)
                    .and_then(|file| {
                        file.take(remaining.min(limits.source_bytes) as u64 + 1)
                            .read_to_end(&mut bytes)
                    })
                    .map_err(|error| error.to_string())?;
                Ok(bytes)
            },
        )
        .map_err(|error| format!("source capture: {error:?}"))?;
    Ok(ObjAsset::parse(
        std::str::from_utf8(&snapshot.bytes)?,
        limits,
    )?)
}
fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if !(2..=6).contains(&arguments.len()) {
        return Err(
            "usage: certify_obj_lod BASE.obj VARIANT.obj [triangle_budget] [depth] [reference|indexed] [cache_dir]"
                .into(),
        );
    }
    let budget = arguments
        .get(2)
        .map_or(Ok(1_000_000_u64), |value| value.parse())?;
    let depth = arguments.get(3).map_or(Ok(0_u8), |value| value.parse())?;
    let indexed = match arguments.get(4).map_or("reference", String::as_str) {
        "reference" => false,
        "indexed" => true,
        _ => return Err("search mode must be reference or indexed".into()),
    };
    if arguments.len() == 6 && !indexed {
        return Err("disk cache currently requires indexed mode".into());
    }
    let mut inputs = ImportInputs::new(
        2,
        ObjLimits::default()
            .source_bytes
            .checked_mul(2)
            .ok_or("source cap overflow")?,
    );
    let base = read_obj(Path::new(&arguments[0]), &mut inputs)?;
    let candidate = read_obj(Path::new(&arguments[1]), &mut inputs)?;
    // Bump identity when parser, search, verifier or archive semantics change.
    let options = format!(
        "budget={budget};depth={depth};indexed={indexed};indexed_triangles=2000000;cells=1000000;node_factor=16"
    );
    let build_key = inputs
        .build_key(
            "voxy.obj-lod",
            "obj1-search1-verifier1-VOXYLCD1",
            "portable-f32-u32",
            options.as_bytes(),
        )
        .map_err(|error| format!("build identity: {error:?}"))?;
    let cache = arguments
        .get(5)
        .map(|root| ArtifactCache::new(root, archive_limits().bytes))
        .transpose()?;

    let (positions, indices) = remap_candidate(&base, &candidate)?;
    let cached = cache
        .as_ref()
        .map(|cache| cache.load(&build_key))
        .transpose()?
        .flatten();
    let artifact = if let Some(bytes) = cached {
        let artifact = decode_lod_archive(&bytes, archive_limits())
            .map_err(|error| format!("cached archive re-verification: {error:?}"))?;
        if artifact.positions().len() != positions.len()
            || !artifact
                .positions()
                .iter()
                .zip(&positions)
                .all(|(a, b)| a.map(f32::to_bits) == b.map(f32::to_bits))
            || artifact.indices().levels().len() != 2
            || artifact.indices().indices(0) != Some(base.mesh.indices())
            || artifact.indices().indices(1) != Some(indices.as_slice())
        {
            return Err("cached archive does not match current imported geometry".into());
        }
        eprintln!("cache_hit=true archive_bytes={}", bytes.len());
        artifact
    } else {
        build(
            positions,
            base.mesh.indices().to_vec(),
            indices,
            budget,
            depth,
            indexed,
            cache.as_ref().map(|cache| (cache, &build_key)),
        )?
    };
    println!(
        "base_triangles={} variant_triangles={} geometric_bound={} logical_index_bytes={}",
        artifact.indices().levels()[0].index_count / 3,
        artifact.indices().levels()[1].index_count / 3,
        artifact.indices().levels()[1].object_error,
        artifact.indices().index_bytes()
    );
    Ok(())
}

type RemappedCandidate = (Vec<[f32; 3]>, Vec<u32>);
fn remap_candidate(
    base: &ObjAsset,
    candidate: &ObjAsset,
) -> Result<RemappedCandidate, Box<dyn Error>> {
    let mut vertices = HashMap::new();
    for index in 0..base.mesh.vertices().len() {
        vertices
            .entry(key(base, index))
            .or_insert(u32::try_from(index)?);
    }
    let remap = (0..candidate.mesh.vertices().len())
        .map(|index| {
            vertices
                .get(&key(candidate, index))
                .copied()
                .ok_or("candidate vertex must reuse exact base position/UV/color/normal")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let indices: Vec<_> = candidate
        .mesh
        .indices()
        .iter()
        .map(|index| remap[*index as usize])
        .collect();
    let positions: Vec<_> = base
        .mesh
        .vertices()
        .iter()
        .map(|vertex| vertex.position)
        .collect();
    Ok((positions, indices))
}

fn build(
    positions: Vec<[f32; 3]>,
    base: Vec<u32>,
    indices: Vec<u32>,
    budget: u64,
    depth: u8,
    indexed: bool,
    cache: Option<(&ArtifactCache, &[u8; 32])>,
) -> Result<CertifiedLodIndexSet, Box<dyn Error>> {
    if indexed {
        let limits = LodSearchBudget {
            indexed_triangles: 2_000_000,
            output_cells: 1_000_000,
            triangle_tests: budget,
            node_visits: budget.checked_mul(16).ok_or("node budget overflow")?,
        };
        let (generated, work) = generate_indexed_lod_witnesses(
            LodSurface {
                positions: &positions,
                indices: &base,
            },
            LodSurface {
                positions: &positions,
                indices: &indices,
            },
            depth,
            limits,
        )
        .map_err(|error| format!("indexed witness generation: {error:?}"))?;
        eprintln!(
            "indexed_triangles={} output_cells={} triangle_tests={} node_visits={}",
            work.indexed_triangles, work.output_cells, work.triangle_tests, work.node_visits
        );
        return archive_roundtrip(
            &positions,
            &base,
            &[CertifiedLodSubdivisionVariant {
                indices,
                source_to_variant: generated.source_to_approximation,
                variant_to_source: generated.approximation_to_source,
            }],
            cache,
        );
    }
    if depth > 0 {
        let generated = generate_subdivided_lod_witnesses(
            LodSurface {
                positions: &positions,
                indices: &base,
            },
            LodSurface {
                positions: &positions,
                indices: &indices,
            },
            depth,
            budget,
        )
        .map_err(|error| format!("witness generation: {error:?}"))?;
        return Ok(CertifiedLodIndexSet::new_subdivided(
            positions,
            base,
            vec![CertifiedLodSubdivisionVariant {
                indices,
                source_to_variant: generated.source_to_approximation,
                variant_to_source: generated.approximation_to_source,
            }],
        )
        .map_err(|error| format!("artifact verification: {error:?}"))?);
    }
    let generated = generate_lod_witnesses(
        LodSurface {
            positions: &positions,
            indices: &base,
        },
        LodSurface {
            positions: &positions,
            indices: &indices,
        },
        budget,
    )
    .map_err(|error| format!("witness generation: {error:?}"))?;
    Ok(CertifiedLodIndexSet::new(
        positions,
        base,
        vec![CertifiedLodVariant {
            indices,
            source_to_variant: generated.source_to_approximation,
            variant_to_source: generated.approximation_to_source,
        }],
    )
    .map_err(|error| format!("artifact verification: {error:?}"))?)
}

fn archive_roundtrip(
    positions: &[[f32; 3]],
    base: &[u32],
    variants: &[CertifiedLodSubdivisionVariant],
    cache: Option<(&ArtifactCache, &[u8; 32])>,
) -> Result<CertifiedLodIndexSet, Box<dyn Error>> {
    let limits = archive_limits();
    let bytes = encode_lod_archive(positions, base, variants, limits)
        .map_err(|error| format!("archive encoding: {error:?}"))?;
    let restored = decode_lod_archive(&bytes, limits)
        .map_err(|error| format!("archive re-verification: {error:?}"))?;
    if let Some((cache, key)) = cache {
        cache.store(key, &bytes)?;
        eprintln!("cache_hit=false");
    }
    eprintln!("archive_bytes={} ", bytes.len());
    Ok(restored)
}

fn archive_limits() -> LodArchiveLimits {
    LodArchiveLimits {
        bytes: 64 * 1024 * 1024,
        positions: 1_000_000,
        levels: 2,
        indices: 6_000_000,
        cells: 1_000_000,
    }
}
