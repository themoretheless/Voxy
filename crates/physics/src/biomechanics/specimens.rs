use super::{Body, Cavity, Fiber, Material};
/// Idealised annular mesh. Radii and length are metres; one material per radial
/// layer. Two end planes are fixed axially, avoiding arbitrary rigid-body drift.
/// # Errors
/// Rejects invalid dimensions/resolution or constitutive parameters.
#[allow(clippy::cast_precision_loss)] // Mesh dimensions are bounded below 2^14.
pub fn tube(
    radii: &[f64],
    length: f64,
    sectors: usize,
    segments: usize,
    materials: &[Material],
    anchor_base: bool,
) -> Result<Body, &'static str> {
    elliptical_tube(
        radii,
        length,
        sectors,
        segments,
        materials,
        anchor_base,
        [1., 1.],
    )
}
/// Homothetic elliptical layers with dimensionless X/Y axis scales.
/// Fibers use an orthonormal frame: tangent, outward normal, axial direction.
/// Pressure acts on a closed triangulated lumen with idealized end caps.
/// # Errors
/// Invalid axis scales, dimensions, material or degenerate tetrahedra.
#[allow(clippy::too_many_arguments, clippy::cast_precision_loss)]
pub fn elliptical_tube(
    radii: &[f64],
    length: f64,
    sectors: usize,
    segments: usize,
    materials: &[Material],
    anchor_base: bool,
    axis_scales: [f64; 2],
) -> Result<Body, &'static str> {
    build_elliptical_tube(
        radii,
        length,
        sectors,
        segments,
        materials,
        anchor_base,
        axis_scales,
        None,
    )
}
/// Explicit material array for every longitudinal segment and radial layer.
/// Region IDs are `segment * layer_count + layer`, supporting local activation.
/// # Errors
/// Empty/mismatched profiles or invalid geometry/materials.
#[allow(clippy::too_many_arguments)]
pub fn elliptical_tube_axial(
    radii: &[f64],
    length: f64,
    sectors: usize,
    segments: usize,
    profiles: &[Vec<Material>],
    anchor_base: bool,
    axis_scales: [f64; 2],
) -> Result<Body, &'static str> {
    if profiles.len() != segments
        || profiles.is_empty()
        || profiles.iter().any(|p| p.len() + 1 != radii.len())
    {
        return Err("invalid axial tube profile");
    }
    build_elliptical_tube(
        radii,
        length,
        sectors,
        segments,
        &profiles[0],
        anchor_base,
        axis_scales,
        Some(profiles),
    )
}
#[allow(clippy::too_many_arguments, clippy::cast_precision_loss)]
fn build_elliptical_tube(
    radii: &[f64],
    length: f64,
    sectors: usize,
    segments: usize,
    materials: &[Material],
    anchor_base: bool,
    axis_scales: [f64; 2],
    profiles: Option<&[Vec<Material>]>,
) -> Result<Body, &'static str> {
    if axis_scales.iter().any(|s| !s.is_finite() || *s <= 0.)
        || radii.len() < 2
        || radii.len() > 16
        || materials.len() + 1 != radii.len()
        || !(8..=128).contains(&sectors)
        || !(1..=64).contains(&segments)
        || !length.is_finite()
        || length <= 0.
        || radii.iter().any(|r| !r.is_finite() || *r <= 0.)
        || radii.windows(2).any(|r| r[0] >= r[1])
    {
        return Err("invalid tube dimensions");
    }
    let index = |z: usize, r: usize, a: usize| (z * radii.len() + r) * sectors + a % sectors;
    let mut points = Vec::new();
    let mut pinned = Vec::new();
    for z in 0..=segments {
        for (radial_index, &r) in radii.iter().enumerate() {
            for a in 0..sectors {
                let theta = a as f64 * std::f64::consts::TAU / sectors as f64;
                points.push([
                    axis_scales[0] * r * theta.cos(),
                    axis_scales[1] * r * theta.sin(),
                    length * z as f64 / segments as f64,
                ]);
                // Shaft: fixed base. Ring: only three outer nodes at midlength remove
                // rigid motion; this support is explicitly an idealised boundary condition.
                pinned.push(if anchor_base {
                    z == 0
                } else {
                    z == segments / 2
                        && radial_index == radii.len() - 1
                        && [0, sectors / 3, 2 * sectors / 3].contains(&a)
                });
            }
        }
    }
    let mut cells = Vec::new();
    for z in 0..segments {
        for (r, base_material) in materials.iter().enumerate() {
            let material = profiles.map_or(base_material, |p| &p[z][r]);
            for a in 0..sectors {
                let corners = [
                    index(z, r, a),
                    index(z, r, a + 1),
                    index(z, r + 1, a),
                    index(z, r + 1, a + 1),
                    index(z + 1, r, a),
                    index(z + 1, r, a + 1),
                    index(z + 1, r + 1, a),
                    index(z + 1, r + 1, a + 1),
                ];
                let theta = (a as f64 + 0.5) * std::f64::consts::TAU / sectors as f64;
                let mut mat = material.clone();
                for f in &mut mat.fibers {
                    // Template directions: circumferential X, radial Y, longitudinal Z.
                    let [c, r, l] = f.direction;
                    let mut tangent = [-axis_scales[0] * theta.sin(), axis_scales[1] * theta.cos()];
                    if axis_scales != [1., 1.] {
                        let norm = tangent[0].hypot(tangent[1]);
                        tangent[0] /= norm;
                        tangent[1] /= norm;
                    }
                    f.direction = [
                        c * tangent[0] + r * tangent[1],
                        c * tangent[1] - r * tangent[0],
                        l,
                    ];
                }
                for ids in [
                    [0, 1, 3, 7],
                    [0, 3, 2, 7],
                    [0, 2, 6, 7],
                    [0, 6, 4, 7],
                    [0, 4, 5, 7],
                    [0, 5, 1, 7],
                ] {
                    cells.push((ids.map(|i| corners[i]), mat.clone()));
                }
            }
        }
    }
    let mut body = Body::new(points, pinned, cells)?;
    for (i, element) in body.elements.iter_mut().enumerate() {
        let axial_layer = i / (sectors * 6);
        element.region = if profiles.is_some() {
            axial_layer
        } else {
            axial_layer % materials.len()
        };
    }
    let mut faces = Vec::new();
    for z in 0..segments {
        for a in 0..sectors {
            let [u, v, w, q] = [
                index(z, 0, a),
                index(z, 0, a + 1),
                index(z + 1, 0, a),
                index(z + 1, 0, a + 1),
            ];
            faces.extend([[u, v, q], [u, q, w]]);
        }
    }
    for a in 1..sectors - 1 {
        faces.push([index(0, 0, 0), index(0, 0, a + 1), index(0, 0, a)]);
        faces.push([
            index(segments, 0, 0),
            index(segments, 0, a),
            index(segments, 0, a + 1),
        ]);
    }
    body.add_cavity(Cavity {
        faces,
        pressure_pa: 0.,
    })?;
    Ok(body)
}
/// Table 1 neo-Hookean TA constants from Fereidoonnezhad et al. (2023),
/// DOI 10.1016/j.compbiomed.2023.107524, converted `MPa` -> Pa. The fiber
/// parameters below are user-adjustable exploratory values, NOT measured fits.
#[must_use]
pub fn tunica_reference() -> Material {
    Material {
        shear_pa: 4.2857e6,
        bulk_pa: 20e6,
        fibers: vec![
            Fiber {
                direction: [1., 0., 0.],
                stiffness_pa: 1e5,
                exponent: 10.,
                active_pa: 0.,
            },
            Fiber {
                direction: [0., 0., 1.],
                stiffness_pa: 1e5,
                exponent: 10.,
                active_pa: 0.,
            },
        ],
    }
}
/// Uncalibrated smooth/striated muscle scenario. Do not use as human constants.
#[must_use]
pub fn muscle_scenario(external: bool) -> Material {
    Material {
        shear_pa: if external { 15e3 } else { 10e3 },
        bulk_pa: 500e3,
        fibers: vec![Fiber {
            direction: [1., 0., 0.],
            stiffness_pa: 8e3,
            exponent: 8.,
            active_pa: if external { 20e3 } else { 10e3 },
        }],
    }
}
/// Two separate pressurised tunica tubes. This idealisation omits the spongiosum,
/// septum, skin, vascular porosity and anatomical attachments.
/// # Errors
/// Propagates mesh construction errors.
pub fn penile_chambers() -> Result<[Body; 2], &'static str> {
    let build = || tube(&[0.006, 0.007], 0.08, 16, 4, &[tunica_reference()], true);
    Ok([build()?, build()?])
}
/// Bonded concentric IAS/EAS layers, with circumferential active fibers.
/// # Errors
/// Propagates mesh construction errors.
pub fn sphincter_layers() -> Result<Body, &'static str> {
    tube(
        &[0.008, 0.011, 0.016],
        0.025,
        24,
        2,
        &[muscle_scenario(false), muscle_scenario(true)],
        false,
    )
}
