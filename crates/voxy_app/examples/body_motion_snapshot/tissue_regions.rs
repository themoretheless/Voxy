//! Authored tissue volumes reuse the asset observation provider and existing controller.
use super::{
    ModelAsset,
    tissue_demo::{TissueDemo, TissueRegionSpec, TissueSkinBinding},
};
use physics::biomechanics::{MaxwellBranch, OgdenTerm, PrescribedTriangleSurface, TetraMesh};
use serde_json::Value;
use std::{path::Path, sync::Arc};
use voxy_assets::{AssetId, FileInputs, ImportInputs, ImportedAsset};
type Error = Box<dyn std::error::Error>;
const MODEL_ID: &str = "embedded:cesium-man";

pub(super) struct Regions {
    volumes: Vec<TissueRegionSpec>,
    exclusions: Vec<Vec<usize>>,
    coverage: Option<(usize, Vec<usize>)>,
    pub(super) coverage_report: Value,
}
impl Regions {
    pub(super) fn instantiate(&self) -> Result<TissueDemo, &'static str> {
        TissueDemo::body_from_region_specs(self.volumes.clone())
    }
    pub(super) fn validate_skin_coverage(&self, binding: &TissueSkinBinding) -> Result<(), Error> {
        if let Some((minimum, required)) = &self.coverage {
            let owned = binding.tissue_owned_vertices();
            let missing: Vec<_> = required
                .iter()
                .copied()
                .filter(|v| owned.binary_search(v).is_err())
                .collect();
            if owned.len() < *minimum || !missing.is_empty() {
                return Err(format!("authored skin coverage mismatch: bound={} minimum={} missing_vertices={missing:?}", owned.len(), minimum).into());
            }
        }
        Ok(())
    }
    pub(super) fn domains(
        &self,
        surface: &PrescribedTriangleSurface,
    ) -> Result<Vec<Arc<PrescribedTriangleSurface>>, &'static str> {
        self.volumes
            .iter()
            .zip(&self.exclusions)
            .map(|(spec, excluded)| {
                let mut mask = vec![true; surface.faces().len()];
                for &face in excluded {
                    let enabled = mask
                        .get_mut(face)
                        .ok_or("authored obstacle exclusion index out of range")?;
                    if !*enabled {
                        return Err("duplicate authored obstacle exclusion");
                    }
                    *enabled = false;
                }
                // Initial native contact is an integration owner only. The physically
                // embedded source skin is installed after assembly before advancing.
                let owner = surface
                    .with_contact_faces(mask)?
                    .with_body_contact_domains(vec![(
                        spec.mesh.boundary.clone(),
                        vec![false; surface.faces().len()],
                    )])?;
                Ok(Arc::new(owner))
            })
            .collect()
    }
}
fn fields(value: &Value, allowed: &[&str]) -> Result<(), Error> {
    let object = value.as_object().ok_or("expected tissue manifest object")?;
    if object.keys().any(|k| !allowed.contains(&k.as_str())) {
        return Err("unknown tissue manifest field".into());
    }
    Ok(())
}
fn digest_hex(digest: [u8; 32]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Observe an offline volume and audit all source vertices with the same native
/// membership/binding provider as runtime. This publishes a report, not dynamics.
pub(super) fn check_volume(
    path: &Path,
    model: &ModelAsset,
    model_bytes: &'static [u8],
) -> Result<ImportedAsset<Value>, Error> {
    let path = path.canonicalize()?;
    let files = FileInputs::new(path.parent().ok_or("missing volume directory")?)?;
    let id = AssetId(
        path.file_name()
            .and_then(|n| n.to_str())
            .ok_or("invalid volume filename")?
            .into(),
    );
    if id.0 == MODEL_ID {
        return Err("reserved tissue input identity".into());
    }
    let mut inputs = ImportInputs::new(2, 64 * 1024 * 1024);
    let snapshot = inputs
        .read(id, |id, limit| files.read(id, limit))
        .map_err(|e| format!("volume observation: {e:?}"))?;
    let source = inputs
        .read(AssetId(MODEL_ID.into()), |_, limit| {
            if model_bytes.len() > limit {
                return Err("model input byte limit".into());
            }
            Ok(model_bytes.to_vec())
        })
        .map_err(|e| format!("model observation: {e:?}"))?;
    let mut report = serde_json::json!({
        "scope":"native phase-zero volume admission and full source skin membership; not animation dynamics",
        "volume_blake3":digest_hex(snapshot.digest), "source_model_blake3":digest_hex(source.digest),
        "volume_admitted":false, "complete_skin_binding":false,
    });
    let audit = (|| -> Result<(), Error> {
        let mesh = TetraMesh::from_medit_volume(std::str::from_utf8(&snapshot.bytes)?)?;
        report["volume_admitted"] = true.into();
        report["volume_nodes"] = mesh.points.len().into();
        report["volume_cells"] = mesh.cells.len().into();
        report["volume_boundary_triangles"] = mesh.boundary.len().into();
        let (skin, _) = super::contact_positions64(model, 0.)?;
        report["source_vertices"] = skin.len().into();
        let search = physics::tissue_surface::TetrahedralEmbedding::new(&mesh.points, &mesh.cells)?;
        let contained = search.contains_points(&skin)?;
        let exterior: Vec<_> = contained
            .iter()
            .enumerate()
            .filter_map(|(i, &inside)| (!inside).then_some(i))
            .collect();
        report["bound_vertices"] = (skin.len() - exterior.len()).into();
        report["exterior_source_vertices"] = serde_json::to_value(&exterior)?;
        if exterior.is_empty() {
            let binding = search.bind_relative(&skin, &contained)?;
            let deformed = binding.deform(&mesh.points)?;
            let max_error = deformed
                .iter()
                .flatten()
                .zip(skin.iter().flatten())
                .map(|(a, b)| (a - b).abs())
                .fold(0_f64, f64::max);
            if !max_error.is_finite() {
                return Err("nonfinite bound source skin".into());
            }
            report["complete_skin_binding"] = true.into();
            report["maximum_rest_component_error_m"] = max_error.into();
        }
        Ok(())
    })();
    if let Err(error) = audit {
        report["admission_error"] = error.to_string().into();
    }
    inputs
        .finish_observed(report, |id, limit| {
            if id.0 == MODEL_ID {
                if model_bytes.len() > limit {
                    return Err("model input byte limit".into());
                }
                Ok(model_bytes.to_vec())
            } else {
                files.read(id, limit)
            }
        })
        .map_err(|e| format!("volume audit inputs changed: {:?}", e.error).into())
}

fn explicit_material(
    mesh: TetraMesh,
    supports: Vec<usize>,
    joint: usize,
    value: &Value,
) -> Result<TissueRegionSpec, Error> {
    fields(
        value,
        &[
            "density_kg_m3",
            "specific_heat_j_kg_k",
            "temperature_kelvin",
            "bulk_pa",
            "ogden_terms",
            "maxwell_branches",
        ],
    )?;
    let number = |object: &Value, key: &str| -> Result<f64, Error> {
        object[key]
            .as_f64()
            .filter(|v| v.is_finite())
            .ok_or_else(|| format!("missing or invalid material parameter {key}").into())
    };
    let terms = value["ogden_terms"]
        .as_array()
        .ok_or("missing Ogden terms")?;
    let branches = value["maxwell_branches"]
        .as_array()
        .ok_or("missing Maxwell branches")?;
    if terms.is_empty() || terms.len() > 16 || branches.len() > 16 {
        return Err("authored constitutive spectrum limit".into());
    }
    let ogden_terms = terms
        .iter()
        .map(|term| {
            fields(term, &["shear_pa", "exponent"])?;
            Ok(OgdenTerm {
                shear_pa: number(term, "shear_pa")?,
                exponent: number(term, "exponent")?,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let maxwell_branches = branches
        .iter()
        .map(|branch| {
            fields(branch, &["shear_pa", "relaxation_seconds"])?;
            Ok(MaxwellBranch {
                shear_pa: number(branch, "shear_pa")?,
                relaxation_seconds: number(branch, "relaxation_seconds")?,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(TissueRegionSpec {
        mesh,
        supports,
        joint,
        support_joint_overrides: Vec::new(),
        support_joint_weights: Vec::new(),
        ogden_terms,
        maxwell_branches,
        bulk_pa: number(value, "bulk_pa")?,
        density_kg_m3: number(value, "density_kg_m3")?,
        specific_heat_j_kg_k: number(value, "specific_heat_j_kg_k")?,
        temperature_kelvin: number(value, "temperature_kelvin")?,
    })
}

pub(super) fn load(
    path: &Path,
    model: &ModelAsset,
    model_bytes: &'static [u8],
) -> Result<ImportedAsset<Regions>, Error> {
    let path = path.canonicalize()?;
    let files = FileInputs::new(path.parent().ok_or("missing tissue manifest directory")?)?;
    let manifest_id = AssetId(
        path.file_name()
            .and_then(|s| s.to_str())
            .ok_or("invalid tissue manifest filename")?
            .to_owned(),
    );
    if manifest_id.0 == MODEL_ID {
        return Err("reserved tissue input identity".into());
    }
    let mut inputs = ImportInputs::new(66, 64 * 1024 * 1024);
    let snapshot = inputs
        .read(manifest_id, |id, limit| {
            files.read(id, limit.min(1024 * 1024))
        })
        .map_err(|e| format!("tissue manifest input: {e:?}"))?;
    let root: Value = serde_json::from_slice(&snapshot.bytes)?;
    fields(
        &root,
        &[
            "version",
            "coordinate_space",
            "material_profile",
            "source_model_blake3",
            "regions",
            "coverage",
        ],
    )?;
    let illustrative = root["version"].as_u64() == Some(1)
        && root["material_profile"] == "illustrative-manikin-v1";
    let explicit = root["version"].as_u64() == Some(2)
        && root["material_profile"] == "authored-ogden-maxwell-v1";
    if (!illustrative && !explicit) || root["coordinate_space"] != "scene_phase_0_metres" {
        return Err("unsupported tissue manifest version, coordinates or material profile".into());
    }
    let source = inputs
        .read(AssetId(MODEL_ID.into()), |_, limit| {
            if model_bytes.len() > limit {
                return Err("model input byte limit".into());
            }
            Ok(model_bytes.to_vec())
        })
        .map_err(|e| format!("tissue model input: {e:?}"))?;
    if root["source_model_blake3"].as_str() != Some(digest_hex(source.digest).as_str()) {
        return Err("tissue manifest source model digest mismatch".into());
    }
    let records = root["regions"]
        .as_array()
        .ok_or("missing authored tissue regions")?;
    if records.is_empty() || records.len() > 64 {
        return Err("invalid authored tissue region count".into());
    }
    let (source_positions, source_faces) = super::contact_positions64(model, 0.)?;
    let coverage = if let Some(value) = root.get("coverage") {
        fields(value, &["minimum_bound_vertices", "required_vertices"])?;
        let minimum = value["minimum_bound_vertices"]
            .as_u64()
            .and_then(|v| usize::try_from(v).ok())
            .ok_or("invalid minimum skin coverage")?;
        let mut required: Vec<usize> = serde_json::from_value(value["required_vertices"].clone())?;
        required.sort_unstable();
        if minimum > source_positions.len()
            || required.iter().any(|&v| v >= source_positions.len())
            || required.windows(2).any(|p| p[0] == p[1])
        {
            return Err("invalid authored source skin coverage indices/count".into());
        }
        Some((minimum, required))
    } else {
        None
    };
    let mut volumes = Vec::with_capacity(records.len());
    let mut mesh_formats = Vec::with_capacity(records.len());
    let mut exclusions = Vec::with_capacity(records.len());
    for record in records {
        let mut allowed = vec![
            "mesh",
            "mesh_blake3",
            "joint",
            "supports",
            "excluded_obstacle_faces",
        ];
        if explicit {
            allowed.extend([
                "mesh_format",
                "material",
                "support_joint_overrides",
                "support_joint_weights",
            ]);
        }
        fields(record, &allowed)?;
        let mesh_id = record["mesh"]
            .as_str()
            .ok_or("missing tissue volume filename")?;
        if mesh_id == MODEL_ID {
            return Err("reserved tissue input identity".into());
        }
        let bytes = inputs
            .read(AssetId(mesh_id.to_owned()), |id, limit| {
                files.read(id, limit.min(32 * 1024 * 1024))
            })
            .map_err(|e| format!("tissue volume input: {e:?}"))?;
        if record["mesh_blake3"].as_str() != Some(digest_hex(bytes.digest).as_str()) {
            return Err("authored tissue volume digest mismatch".into());
        }
        let format = match record.get("mesh_format") {
            None => "vxtm-v1",
            Some(value) => value.as_str().ok_or("invalid tissue mesh format")?,
        };
        let mesh = match format {
            "vxtm-v1" => TetraMesh::from_bytes(&bytes.bytes)?,
            "medit-volume-v1" => TetraMesh::from_medit_volume(std::str::from_utf8(&bytes.bytes)?)?,
            "star-shaped-surface-v1" => {
                let surface: Value = serde_json::from_slice(&bytes.bytes)?;
                fields(&surface, &["points", "boundary", "interior"])?;
                TetraMesh::from_star_shaped_surface(
                    serde_json::from_value(surface["points"].clone())?,
                    serde_json::from_value(surface["boundary"].clone())?,
                    serde_json::from_value(surface["interior"].clone())?,
                )?
            }
            _ => return Err("unsupported tissue mesh format".into()),
        };
        mesh_formats.push(format);
        let supports = if illustrative {
            serde_json::from_value::<[usize; 3]>(record["supports"].clone())?.to_vec()
        } else {
            serde_json::from_value::<Vec<usize>>(record["supports"].clone())?
        };
        let joint = usize::from(
            model.resolve_joint_name(
                record["joint"]
                    .as_str()
                    .ok_or("missing authored tissue joint")?,
            )?,
        );
        let excluded: Vec<usize> =
            serde_json::from_value(record["excluded_obstacle_faces"].clone())?;
        let mut seen = std::collections::BTreeSet::new();
        if excluded
            .iter()
            .any(|&face| face >= source_faces.len() || !seen.insert(face))
        {
            return Err("invalid or duplicate authored obstacle exclusion".into());
        }
        let mut spec = if illustrative {
            TissueRegionSpec::illustrative(mesh, supports, joint)
        } else {
            explicit_material(mesh, supports, joint, &record["material"])?
        };
        if let Some(overrides) = record.get("support_joint_overrides") {
            let overrides = overrides
                .as_array()
                .ok_or("invalid support joint overrides")?;
            if overrides.len() > spec.supports.len() {
                return Err("invalid support joint overrides".into());
            }
            for entry in overrides {
                fields(entry, &["node", "joint"])?;
                let node =
                    usize::try_from(entry["node"].as_u64().ok_or("invalid support joint node")?)?;
                let bone = usize::from(
                    model.resolve_joint_name(
                        entry["joint"]
                            .as_str()
                            .ok_or("invalid support joint name")?,
                    )?,
                );
                spec.support_joint_overrides.push((node, bone));
            }
        }
        if let Some(weights) = record.get("support_joint_weights") {
            let weights = weights.as_array().ok_or("invalid support joint weights")?;
            if weights.len() > spec.supports.len() {
                return Err("invalid support joint weights".into());
            }
            for entry in weights {
                fields(entry, &["node", "influences"])?;
                let node =
                    usize::try_from(entry["node"].as_u64().ok_or("invalid support joint node")?)?;
                let influences = entry["influences"]
                    .as_array()
                    .ok_or("invalid support influences")?;
                if influences.is_empty() || influences.len() > 8 {
                    return Err("invalid support influences".into());
                }
                let mut resolved = Vec::new();
                for influence in influences {
                    fields(influence, &["joint", "weight"])?;
                    let bone = usize::from(
                        model.resolve_joint_name(
                            influence["joint"]
                                .as_str()
                                .ok_or("invalid support joint name")?,
                        )?,
                    );
                    resolved.push((
                        bone,
                        influence["weight"]
                            .as_f64()
                            .filter(|w| w.is_finite())
                            .ok_or("invalid support weight")?,
                    ));
                }
                spec.support_joint_weights.push((node, resolved));
            }
        }
        volumes.push(spec);
        exclusions.push(excluded);
    }
    let mut regions = Regions {
        volumes,
        exclusions,
        coverage,
        coverage_report: Value::Null,
    };
    // Geometry and requested source membership are admitted before publication.
    let mut demo = regions.instantiate()?;
    demo.assemble_regions()?;
    let binding = demo.bind_skin(&source_positions)?;
    regions.validate_skin_coverage(&binding)?;
    regions.coverage_report = serde_json::json!({
        "scope":"reference-space skin membership; not dynamic collision qualification",
        "source_vertices":source_positions.len(), "bound_vertices":binding.bound_vertex_count(),
        "tissue_owned_vertices":binding.tissue_owned_vertices(), "contract_supplied":regions.coverage.is_some(),
        "regions": regions.volumes.iter().zip(mesh_formats).map(|(spec, format)| serde_json::json!({
            "mesh_format":format,
            "nodes":spec.mesh.points.len(), "cells":spec.mesh.cells.len(),
            "boundary_triangles":spec.mesh.boundary.len(), "supports":spec.supports.len(),
            "joint":spec.joint, "support_joint_overrides":spec.support_joint_overrides,
            "support_joint_weights":spec.support_joint_weights,
            "density_kg_m3":spec.density_kg_m3,
            "specific_heat_j_kg_k":spec.specific_heat_j_kg_k,
            "temperature_kelvin":spec.temperature_kelvin,
        })).collect::<Vec<_>>(),
    });
    inputs
        .finish_observed(regions, |id, limit| {
            if id.0 == MODEL_ID {
                if model_bytes.len() > limit {
                    return Err("model input changed size".into());
                }
                Ok(model_bytes.to_vec())
            } else {
                files.read(id, limit)
            }
        })
        .map_err(|e| format!("authored tissue inputs changed: {:?}", e.error).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_render::ModelLimits;
    const MODEL: &[u8] = include_bytes!("../../../../assets/animation/cesium-man/CesiumMan.glb");
    fn hash(bytes: &[u8]) -> String {
        let mut inputs = ImportInputs::new(1, bytes.len());
        digest_hex(
            inputs
                .read(AssetId("fixture".into()), |_, _| Ok(bytes.to_vec()))
                .unwrap()
                .digest,
        )
    }
    #[test]
    fn nonconvex_vxtm_import_preserves_source_membership_and_authored_settings() {
        let directory = std::env::temp_dir().join(format!(
            "voxy-nonconvex-import-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let model = ModelAsset::parse(MODEL, &[], ModelLimits::default()).unwrap();
        let (positions, _) = super::super::contact_positions64(&model, 0.).unwrap();
        let origin = positions[666].map(|v| v - 0.005);
        let mesh =
            TetraMesh::from_lattice_cells(origin, [0.01; 3], &[[0; 3], [1, 0, 0], [0, 1, 0]])
                .unwrap();
        let supports: Vec<_> = mesh
            .points
            .iter()
            .enumerate()
            .filter_map(|(i, p)| (p[1] == origin[1]).then_some(i))
            .collect();
        let bytes = mesh.to_bytes().unwrap();
        std::fs::write(directory.join("volume.vxtm"), &bytes).unwrap();
        let value = serde_json::json!({
            "version":2, "coordinate_space":"scene_phase_0_metres",
            "material_profile":"authored-ogden-maxwell-v1", "source_model_blake3":hash(MODEL),
            "coverage":{"minimum_bound_vertices":1,"required_vertices":[666]},
            "regions":[{"mesh":"volume.vxtm","mesh_blake3":hash(&bytes),
                "joint":"Skeleton_torso_joint_2","supports":supports,"excluded_obstacle_faces":[],
                "material":{"density_kg_m3":1200.,"specific_heat_j_kg_k":2000.,"temperature_kelvin":295.,"bulk_pa":2e6,
                    "ogden_terms":[{"shear_pa":15000.,"exponent":2.}],
                    "maxwell_branches":[{"shear_pa":20000.,"relaxation_seconds":0.5}]} }]
        });
        let manifest = directory.join("regions.json");
        std::fs::write(&manifest, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        let imported = load(&manifest, &model, MODEL).unwrap();
        assert_eq!(imported.inputs().observations().len(), 3);
        let region = &imported.value().coverage_report["regions"][0];
        assert_eq!(region["nodes"], 16);
        assert_eq!(region["cells"], 18);
        assert_eq!(region["boundary_triangles"], 28);
        assert_eq!(region["supports"], supports.len());
        assert_eq!(region["density_kg_m3"], 1200.);
        let mut demo = imported.value().instantiate().unwrap();
        assert_eq!(
            demo.body_thermal_diagnostics().unwrap()[0],
            (295., 295., 0.)
        );
        demo.assemble_regions().unwrap();
        let binding = demo.bind_skin(&positions).unwrap();
        assert!(binding.tissue_owned_vertices().contains(&666));
        eprintln!("NONCONVEX_IMPORT {}", imported.value().coverage_report);
        let mut medit = format!(
            "MeshVersionFormatted 1\nDimension 3\nVertices {}\n",
            mesh.points.len()
        );
        for point in &mesh.points {
            medit.push_str(&format!("{} {} {} 0\n", point[0], point[1], point[2]));
        }
        medit.push_str(&format!("Triangles 0\nTetrahedra {}\n", mesh.cells.len()));
        for cell in &mesh.cells {
            medit.push_str(&format!(
                "{} {} {} {} 0\n",
                cell[0] + 1,
                cell[1] + 1,
                cell[2] + 1,
                cell[3] + 1
            ));
        }
        medit.push_str("End\n");
        std::fs::write(directory.join("volume.mesh"), &medit).unwrap();
        let mut medit_manifest = value.clone();
        medit_manifest["regions"][0]["mesh"] = "volume.mesh".into();
        medit_manifest["regions"][0]["mesh_format"] = "medit-volume-v1".into();
        medit_manifest["regions"][0]["mesh_blake3"] = hash(medit.as_bytes()).into();
        std::fs::write(&manifest, serde_json::to_vec(&medit_manifest).unwrap()).unwrap();
        let imported_medit = load(&manifest, &model, MODEL).unwrap();
        assert_eq!(imported_medit.value().volumes[0].mesh.points, mesh.points);
        assert_eq!(imported_medit.value().volumes[0].mesh.cells, mesh.cells);
        assert_eq!(
            imported_medit.value().volumes[0].mesh.boundary,
            mesh.boundary
        );
        assert_eq!(
            imported_medit.value().coverage_report["tissue_owned_vertices"],
            imported.value().coverage_report["tissue_owned_vertices"]
        );
        assert_eq!(imported_medit.inputs().observations().len(), 3);
        eprintln!(
            "MEDIT_VOLUME_IMPORT {}",
            imported_medit.value().coverage_report
        );
        let audit = check_volume(&directory.join("volume.mesh"), &model, MODEL).unwrap();
        assert_eq!(audit.inputs().observations().len(), 2);
        assert_eq!(audit.value()["volume_admitted"], true);
        assert_eq!(audit.value()["complete_skin_binding"], false);
        assert_eq!(audit.value()["bound_vertices"], 1);
        assert_eq!(audit.value()["source_vertices"], 3273);
        assert_eq!(
            audit.value()["exterior_source_vertices"]
                .as_array()
                .unwrap()
                .len(),
            3272
        );
        std::fs::write(directory.join("invalid.mesh"), b"invalid volume").unwrap();
        let rejected = check_volume(&directory.join("invalid.mesh"), &model, MODEL).unwrap();
        assert_eq!(rejected.value()["volume_admitted"], false);
        assert_eq!(rejected.value()["complete_skin_binding"], false);
        assert_eq!(
            rejected.value()["admission_error"],
            "unsupported Medit tissue volume profile"
        );
        // Import the very same nonconvex source boundary directly, without a
        // convex hull or a manually precomputed tetrahedral intermediate.
        let surface = serde_json::json!({
            "points":mesh.points, "boundary":mesh.boundary,
            "interior":origin.map(|v| v + 0.005),
        });
        let surface_bytes = serde_json::to_vec_pretty(&surface).unwrap();
        let surface_path = directory.join("surface.json");
        std::fs::write(&surface_path, &surface_bytes).unwrap();
        let mut star_manifest = value.clone();
        star_manifest["regions"][0]["mesh"] = "surface.json".into();
        star_manifest["regions"][0]["mesh_format"] = "star-shaped-surface-v1".into();
        star_manifest["regions"][0]["mesh_blake3"] = hash(&surface_bytes).into();
        std::fs::write(&manifest, serde_json::to_vec(&star_manifest).unwrap()).unwrap();
        let star = load(&manifest, &model, MODEL).unwrap();
        assert_eq!(star.inputs().observations().len(), 3);
        let built = &star.value().volumes[0].mesh;
        assert_eq!(&built.points[..mesh.points.len()], mesh.points);
        assert_eq!(built.boundary, mesh.boundary);
        assert_eq!(built.cells.len(), mesh.boundary.len());
        assert!(
            star.value().coverage_report["tissue_owned_vertices"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!(666))
        );
        assert_eq!(
            star.value().coverage_report["regions"][0]["mesh_format"],
            "star-shaped-surface-v1"
        );
        eprintln!("STAR_SURFACE_IMPORT {}", star.value().coverage_report);
        for format in [
            serde_json::json!("unknown"),
            Value::Null,
            serde_json::json!(12),
        ] {
            let mut invalid = star_manifest.clone();
            invalid["regions"][0]["mesh_format"] = format;
            std::fs::write(&manifest, serde_json::to_vec(&invalid).unwrap()).unwrap();
            assert!(load(&manifest, &model, MODEL).is_err());
        }
        let mut invalid_surface = surface.clone();
        invalid_surface["interior"] = serde_json::json!(origin.map(|v| v - 1.));
        let invalid_bytes = serde_json::to_vec(&invalid_surface).unwrap();
        let mut invalid_manifest = star_manifest.clone();
        invalid_manifest["regions"][0]["mesh_blake3"] = hash(&invalid_bytes).into();
        std::fs::write(&manifest, serde_json::to_vec(&invalid_manifest).unwrap()).unwrap();
        std::fs::write(&surface_path, &invalid_bytes).unwrap();
        assert!(load(&manifest, &model, MODEL).is_err());
        // A changed geometry file also rejects against the pinned digest.
        std::fs::write(&manifest, serde_json::to_vec(&star_manifest).unwrap()).unwrap();
        assert!(
            load(&manifest, &model, MODEL)
                .err()
                .unwrap()
                .to_string()
                .contains("digest mismatch")
        );
        std::fs::write(&surface_path, &surface_bytes).unwrap();
        if let Some(output) = std::env::var_os("VOXY_NONCONVEX_IMPORT_FIXTURE_DIR") {
            let output = std::path::PathBuf::from(output);
            std::fs::create_dir_all(&output).unwrap();
            std::fs::write(output.join("surface.json"), &surface_bytes).unwrap();
            std::fs::write(
                output.join("star-regions.json"),
                serde_json::to_vec_pretty(&star_manifest).unwrap(),
            )
            .unwrap();
            std::fs::write(
                output.join("star-coverage.json"),
                serde_json::to_vec_pretty(&star.value().coverage_report).unwrap(),
            )
            .unwrap();
        }
        let mut multi_bone = value.clone();
        multi_bone["regions"][0]["support_joint_overrides"] = serde_json::json!([
            {"node":supports[0],"joint":"leg_joint_R_1"}
        ]);
        std::fs::write(&manifest, serde_json::to_vec(&multi_bone).unwrap()).unwrap();
        let multi = load(&manifest, &model, MODEL).unwrap();
        let expected_joint = usize::from(model.resolve_joint_name("leg_joint_R_1").unwrap());
        assert_eq!(
            multi.value().volumes[0].support_joint_overrides,
            vec![(supports[0], expected_joint)]
        );
        for overrides in [
            serde_json::json!([{"node":supports[0],"joint":"missing"}]),
            serde_json::json!([{"node":99999,"joint":"leg_joint_R_1"}]),
            serde_json::json!([{"node":supports[0],"joint":"leg_joint_R_1"},{"node":supports[0],"joint":"leg_joint_R_1"}]),
        ] {
            let mut invalid = multi_bone.clone();
            invalid["regions"][0]["support_joint_overrides"] = overrides;
            std::fs::write(&manifest, serde_json::to_vec(&invalid).unwrap()).unwrap();
            assert!(load(&manifest, &model, MODEL).is_err());
        }
        let mut weighted = value.clone();
        weighted["regions"][0]["support_joint_weights"] = serde_json::json!([
            {"node":supports[0],"influences":[{"joint":"Skeleton_torso_joint_2","weight":0.25},{"joint":"leg_joint_R_1","weight":0.75}]}
        ]);
        std::fs::write(&manifest, serde_json::to_vec(&weighted).unwrap()).unwrap();
        let blended = load(&manifest, &model, MODEL).unwrap();
        assert_eq!(
            blended.value().volumes[0].support_joint_weights[0].1.len(),
            2
        );
        weighted["regions"][0]["support_joint_weights"][0]["influences"][1]["weight"] = 0.5.into();
        std::fs::write(&manifest, serde_json::to_vec(&weighted).unwrap()).unwrap();
        assert!(load(&manifest, &model, MODEL).is_err());
        // Import/membership evidence only; collision admission is deliberately
        // exercised separately by the physical-skin controller gates.
        if let Some(output) = std::env::var_os("VOXY_NONCONVEX_IMPORT_FIXTURE_DIR") {
            let output = std::path::PathBuf::from(output);
            std::fs::create_dir_all(&output).unwrap();
            std::fs::write(output.join("volume.vxtm"), &bytes).unwrap();
            std::fs::write(output.join("volume.mesh"), &medit).unwrap();
            std::fs::write(
                output.join("medit-regions.json"),
                serde_json::to_vec_pretty(&medit_manifest).unwrap(),
            )
            .unwrap();
            std::fs::write(
                output.join("volume-admission.json"),
                serde_json::to_vec_pretty(audit.value()).unwrap(),
            )
            .unwrap();
            std::fs::write(
                output.join("regions.json"),
                serde_json::to_vec_pretty(&value).unwrap(),
            )
            .unwrap();
            std::fs::write(
                output.join("coverage.json"),
                serde_json::to_vec_pretty(&imported.value().coverage_report).unwrap(),
            )
            .unwrap();
        }
        std::fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn observed_authored_volumes_resolve_joints_and_reject_incompatible_inputs() {
        let directory = std::env::temp_dir().join(format!(
            "voxy-authored-regions-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let model = ModelAsset::parse(MODEL, &[], ModelLimits::default()).unwrap();
        let reference = model
            .sample_pose_phase(Some(0), 0.)
            .unwrap()
            .skin_matrices(&model.skeleton)
            .unwrap();
        let (centers, _) = super::super::imported_attachments(&model, &reference).unwrap();
        let cage = TetraMesh::ellipsoid(centers[0], [0.08, 0.065, 0.05], 1).unwrap();
        let mesh = TetraMesh::from_convex_surface(
            cage.points[1..].to_vec(),
            cage.boundary.iter().map(|f| f.map(|i| i - 1)).collect(),
            centers[0],
        )
        .unwrap();
        let (positions, faces) = super::super::contact_positions64(&model, 0.).unwrap();
        let source = PrescribedTriangleSurface::new(positions, faces, 0.0001, 0.003, 100.).unwrap();
        let original_domains = super::super::attachment_domains(&source, centers).unwrap();
        let excluded: Vec<_> = original_domains[0]
            .contact_faces()
            .iter()
            .enumerate()
            .filter_map(|(i, &on)| (!on).then_some(i))
            .collect();
        let bytes = mesh.to_bytes().unwrap();
        std::fs::write(directory.join("pad.vxtm"), &bytes).unwrap();
        let value = serde_json::json!({
            "version":1, "coordinate_space":"scene_phase_0_metres",
            "material_profile":"illustrative-manikin-v1", "source_model_blake3":hash(MODEL),
            "coverage":{"minimum_bound_vertices":1, "required_vertices":[666]},
            "regions":[{"mesh":"pad.vxtm", "mesh_blake3":hash(&bytes), "joint":"Skeleton_torso_joint_2", "supports":[2,3,5], "excluded_obstacle_faces":excluded}]
        });
        let manifest = directory.join("regions.json");
        let write = |v: &Value| std::fs::write(&manifest, serde_json::to_vec(v).unwrap()).unwrap();
        write(&value);
        let imported = load(&manifest, &model, MODEL).unwrap();
        assert_eq!(imported.inputs().observations().len(), 3);
        assert_eq!(
            imported.value().coverage_report["tissue_owned_vertices"],
            serde_json::json!([666])
        );
        assert_eq!(imported.value().coverage_report["bound_vertices"], 1);
        let mut demo = imported.value().instantiate().unwrap();
        let domains = imported.value().domains(&source).unwrap();
        assert_eq!(domains.len(), 1);
        assert_eq!(
            domains[0]
                .response(&mesh.points, &mesh.boundary)
                .unwrap()
                .potential_j,
            0.
        );
        demo.bind_contact_surfaces(&domains).unwrap();
        demo.assemble_regions().unwrap();
        let binding = demo.bind_skin(source.positions()).unwrap();
        assert!(binding.bound_vertex_count() > 0);
        let triangles = demo.bind_skin_contact(&binding).unwrap();
        assert!(triangles > 0);
        let reference64 = model
            .sample_pose_phase64(Some(0), 0.)
            .unwrap()
            .skin_matrices(&model.skeleton)
            .unwrap();
        let mut prior = demo.body_energy_receipts().unwrap()[0];
        for _ in 0..8 {
            demo.advance_with_palette64_and_surfaces(1. / 240., |time| {
                super::super::imported_contact_sample64(&model, &reference64, &domains, time / 2.)
            })
            .unwrap();
            let actual = demo.body_energy_receipts().unwrap()[0];
            let independent =
                (actual[0] - prior[0]) - (actual[1] - prior[1]) + (actual[2] - prior[2]);
            assert!(independent.abs() <= 1e-5);
            prior = actual;
        }
        eprintln!(
            "AUTHORED_IMPORT_COMMITTED steps=8 bound_vertices={} responsive_triangles={triangles}",
            binding.bound_vertex_count()
        );
        if let Some(output) = std::env::var_os("VOXY_TISSUE_IMPORT_FIXTURE_DIR") {
            let output = std::path::PathBuf::from(output);
            std::fs::create_dir_all(&output).unwrap();
            std::fs::write(output.join("pad.vxtm"), &bytes).unwrap();
            std::fs::write(
                output.join("regions.json"),
                serde_json::to_vec_pretty(&value).unwrap(),
            )
            .unwrap();
        }
        let mut explicit = value.clone();
        explicit["version"] = 2.into();
        explicit["material_profile"] = "authored-ogden-maxwell-v1".into();
        explicit["regions"][0]["supports"] = serde_json::json!([2, 3, 5, 0]);
        explicit["regions"][0]["material"] = serde_json::json!({
            "density_kg_m3":1200., "specific_heat_j_kg_k":2000., "temperature_kelvin":295., "bulk_pa":2e6,
            "ogden_terms":[{"shear_pa":15000., "exponent":2.}],
            "maxwell_branches":[{"shear_pa":20000., "relaxation_seconds":0.5}]
        });
        write(&explicit);
        let authored = load(&manifest, &model, MODEL).unwrap();
        let mut authored_demo = authored.value().instantiate().unwrap();
        assert_eq!(
            authored_demo.body_thermal_diagnostics().unwrap()[0],
            (295., 295., 0.)
        );
        let authored_domains = authored.value().domains(&source).unwrap();
        authored_demo
            .bind_contact_surfaces(&authored_domains)
            .unwrap();
        authored_demo.assemble_regions().unwrap();
        let authored_binding = authored_demo.bind_skin(source.positions()).unwrap();
        authored_demo.bind_skin_contact(&authored_binding).unwrap();
        let before = authored_demo.body_energy_receipts().unwrap()[0];
        authored_demo
            .advance_with_palette64_and_surfaces(1. / 240., |time| {
                super::super::imported_contact_sample64(
                    &model,
                    &reference64,
                    &authored_domains,
                    time / 2.,
                )
            })
            .unwrap();
        let after = authored_demo.body_energy_receipts().unwrap()[0];
        assert!(
            ((after[0] - before[0]) - (after[1] - before[1]) + (after[2] - before[2])).abs()
                <= 1e-5
        );
        if let Some(output) = std::env::var_os("VOXY_TISSUE_IMPORT_FIXTURE_DIR") {
            std::fs::write(
                std::path::PathBuf::from(output).join("regions-v2.json"),
                serde_json::to_vec_pretty(&explicit).unwrap(),
            )
            .unwrap();
        }
        for (field, bad) in [
            ("density_kg_m3", Value::from(0)),
            ("specific_heat_j_kg_k", Value::from(-1)),
            ("temperature_kelvin", Value::from(0)),
            ("bulk_pa", Value::from(-1)),
            ("ogden_terms", serde_json::json!([])),
            (
                "maxwell_branches",
                serde_json::json!([{"shear_pa":1., "relaxation_seconds":0.}]),
            ),
        ] {
            let mut invalid = explicit.clone();
            invalid["regions"][0]["material"][field] = bad;
            write(&invalid);
            assert!(load(&manifest, &model, MODEL).is_err());
        }
        let mut missing = explicit.clone();
        missing["regions"][0]["material"]
            .as_object_mut()
            .unwrap()
            .remove("density_kg_m3");
        write(&missing);
        assert!(load(&manifest, &model, MODEL).is_err());
        for coverage in [
            serde_json::json!({"minimum_bound_vertices":2, "required_vertices":[666]}),
            serde_json::json!({"minimum_bound_vertices":1, "required_vertices":[2591]}),
            serde_json::json!({"minimum_bound_vertices":1, "required_vertices":[666,666]}),
            serde_json::json!({"minimum_bound_vertices":3274, "required_vertices":[]}),
            serde_json::json!({"minimum_bound_vertices":1, "required_vertices":[999999]}),
        ] {
            let mut incompatible = explicit.clone();
            incompatible["coverage"] = coverage;
            write(&incompatible);
            assert!(load(&manifest, &model, MODEL).is_err());
        }
        let mut incomplete = explicit.clone();
        incomplete["coverage"] =
            serde_json::json!({"minimum_bound_vertices":3273, "required_vertices":[651,666,2591]});
        write(&incomplete);
        let error = load(&manifest, &model, MODEL).err().unwrap().to_string();
        assert!(error.contains("missing_vertices=[651, 2591]"), "{error}");
        eprintln!("AUTHORED_COVERAGE_REJECTION {error}");
        for (field, bad) in [
            ("source_model_blake3", Value::String("wrong".into())),
            ("coordinate_space", Value::String("centimetres".into())),
            (
                "material_profile",
                Value::String("anatomically-calibrated".into()),
            ),
            ("version", Value::from(2)),
        ] {
            let mut changed = value.clone();
            changed[field] = bad;
            write(&changed);
            assert!(load(&manifest, &model, MODEL).is_err());
        }
        for (field, bad) in [
            ("mesh_blake3", Value::String("wrong".into())),
            ("joint", Value::String("missing-joint".into())),
            ("supports", serde_json::json!([3, 3, 6])),
            ("excluded_obstacle_faces", serde_json::json!([999999])),
            ("excluded_obstacle_faces", serde_json::json!([0, 0])),
            ("mesh", Value::String("../outside.vxtm".into())),
        ] {
            let mut changed = value.clone();
            changed["regions"][0][field] = bad;
            write(&changed);
            assert!(load(&manifest, &model, MODEL).is_err());
        }
        let mut unknown = value.clone();
        unknown["unused_setting"] = true.into();
        write(&unknown);
        assert!(load(&manifest, &model, MODEL).is_err());
        write(&value);
        std::fs::write(directory.join("pad.vxtm"), b"corrupted").unwrap();
        assert!(load(&manifest, &model, MODEL).is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
