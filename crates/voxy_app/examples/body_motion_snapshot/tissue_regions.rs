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
            allowed.push("material");
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
        let mesh = TetraMesh::from_bytes(&bytes.bytes)?;
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
        let spec = if illustrative {
            TissueRegionSpec::illustrative(mesh, supports, joint)
        } else {
            explicit_material(mesh, supports, joint, &record["material"])?
        };
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
