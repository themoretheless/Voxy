//! Parameterized adult anatomical fold specimens, not fitted anatomical meshes.
use super::{Point, Skin, SkinMaterial};
/// A mechanically continuous fold and explicit support/loading identities.
#[derive(Clone, Debug)]
pub struct TissueFold {
    pub skin: Skin,
    pub root_nodes: Vec<usize>,
    /// Distal fold crest, not a prescribed kinematic trajectory.
    pub crest_nodes: Vec<usize>,
}
#[derive(Clone, Copy, Debug)]
pub struct PrepuceGeometry {
    pub inner_radius_m: f64,
    pub outer_radius_m: f64,
    pub length_m: f64,
    pub sectors: usize,
    /// Rows per inner sheet, rounded distal fold and outer sheet.
    pub rows_per_section: usize,
}
impl PrepuceGeometry {
    /// Connected inner/outer sheets joined by a rounded distal fold; two basal
    /// rings fixed. Z is the longitudinal axis; material follows the circumference.
    /// # Errors
    /// Invalid geometry/resolution, excessively thick shell, material/mesh error.
    pub fn build(self, material: SkinMaterial) -> Result<TissueFold, &'static str> {
        let h = (self.outer_radius_m - self.inner_radius_m) / 2.;
        if [self.inner_radius_m, self.outer_radius_m, self.length_m]
            .iter()
            .any(|v| !v.is_finite() || *v <= 0.)
            || h <= 0.
            || self.length_m <= h
            || !(8..=128).contains(&self.sectors)
            || !(2..=32).contains(&self.rows_per_section)
            || material.layers.iter().map(|l| l.thickness).sum::<f64>() >= 2. * h
        {
            return Err("invalid prepuce fold geometry");
        }
        let n = self.rows_per_section;
        let rows = 3 * n;
        let index = |row: usize, a: usize| row * self.sectors + a % self.sectors;
        let mut points = Vec::new();
        for row in 0..=rows {
            let (r, z) = if row <= n {
                (
                    self.inner_radius_m,
                    (self.length_m - h) * row as f64 / n as f64,
                )
            } else if row <= 2 * n {
                let phi = std::f64::consts::PI * (row - n) as f64 / n as f64;
                (
                    (self.inner_radius_m + self.outer_radius_m) / 2. - h * phi.cos(),
                    self.length_m - h + h * phi.sin(),
                )
            } else {
                (
                    self.outer_radius_m,
                    (self.length_m - h) * (rows - row) as f64 / n as f64,
                )
            };
            for a in 0..self.sectors {
                let angle = std::f64::consts::TAU * a as f64 / self.sectors as f64;
                points.push([r * angle.cos(), r * angle.sin(), z]);
            }
        }
        let mut faces = Vec::new();
        let mut directions = Vec::new();
        for row in 0..rows {
            for a in 0..self.sectors {
                let [u, v, w, q] = [
                    index(row, a),
                    index(row, a + 1),
                    index(row + 1, a),
                    index(row + 1, a + 1),
                ];
                let direction = std::array::from_fn(|k| points[v][k] - points[u][k]);
                faces.extend([[u, v, q], [u, q, w]]);
                directions.extend([direction; 2]);
            }
        }
        let roots: Vec<_> = (0..self.sectors)
            .flat_map(|a| [index(0, a), index(rows, a)])
            .collect();
        let crest = (0..self.sectors).map(|a| index(n + n / 2, a)).collect();
        Ok(TissueFold {
            skin: Skin::new(points, faces, &roots, material, directions)?,
            root_nodes: roots,
            crest_nodes: crest,
        })
    }
}
#[derive(Clone, Copy, Debug)]
pub struct LabiaMinoraGeometry {
    pub length_m: f64,
    pub fold_width_m: f64,
    pub fold_height_m: f64,
    /// Distance between the two fold center lines.
    pub separation_m: f64,
    pub longitudinal_segments: usize,
    pub transverse_segments: usize,
}
impl LabiaMinoraGeometry {
    /// Two separate mirrored folds, each fixed along its basal perimeter.
    /// Y is longitudinal, X lateral, Z outward. Each retains its own material history.
    /// # Errors
    /// Invalid dimensions/resolution, overlapping rest folds or material/mesh error.
    pub fn build(self, material: SkinMaterial) -> Result<[TissueFold; 2], &'static str> {
        if [
            self.length_m,
            self.fold_width_m,
            self.fold_height_m,
            self.separation_m,
        ]
        .iter()
        .any(|v| !v.is_finite() || *v <= 0.)
            || self.separation_m <= self.fold_width_m
            || !(2..=64).contains(&self.longitudinal_segments)
            || !(2..=32).contains(&self.transverse_segments)
        {
            return Err("invalid labia minora fold geometry");
        }
        let build = |side: f64| {
            let ny = self.longitudinal_segments;
            let nx = self.transverse_segments;
            let index = |y: usize, x: usize| y * (nx + 1) + x;
            let mut points: Vec<Point> = Vec::new();
            let mut roots = Vec::new();
            let mut crest = Vec::new();
            for y in 0..=ny {
                for x in 0..=nx {
                    let u = x as f64 / nx as f64;
                    let v = y as f64 / ny as f64;
                    points.push([
                        side * (self.separation_m / 2. + self.fold_width_m * (u - 0.5)),
                        self.length_m * (v - 0.5),
                        self.fold_height_m
                            * (std::f64::consts::PI * u).sin()
                            * (std::f64::consts::PI * v).sin(),
                    ]);
                    if x == 0 || x == nx || y == 0 || y == ny {
                        roots.push(index(y, x));
                    }
                    if x == nx / 2 && y > 0 && y < ny {
                        crest.push(index(y, x));
                    }
                }
            }
            let mut faces = Vec::new();
            for y in 0..ny {
                for x in 0..nx {
                    let [u, v, w, q] = [
                        index(y, x),
                        index(y, x + 1),
                        index(y + 1, x),
                        index(y + 1, x + 1),
                    ];
                    if side > 0. {
                        faces.extend([[u, v, q], [u, q, w]]);
                    } else {
                        faces.extend([[u, q, v], [u, w, q]]);
                    }
                }
            }
            let directions = vec![[0., 1., 0.]; faces.len()];
            Ok(TissueFold {
                skin: Skin::new(points, faces, &roots, material.clone(), directions)?,
                root_nodes: roots,
                crest_nodes: crest,
            })
        };
        Ok([build(-1.)?, build(1.)?])
    }

    /// Construct a coupled left/right labia minora viscoelastic fold simulation
    /// whose basal perimeter roots are driven by 3D pelvis gait kinematics.
    pub fn build_gait_simulation(
        self,
        material: SkinMaterial,
    ) -> Result<LabiaMinoraGaitSimulation, &'static str> {
        let folds = self.build(material)?;
        let rest_roots = [
            folds[0]
                .root_nodes
                .iter()
                .map(|&i| (i, folds[0].skin.rest_positions()[i]))
                .collect(),
            folds[1]
                .root_nodes
                .iter()
                .map(|&i| (i, folds[1].skin.rest_positions()[i]))
                .collect(),
        ];
        Ok(LabiaMinoraGaitSimulation {
            folds,
            rest_roots,
            time_s: 0.,
        })
    }
}

/// Coupled left and right labia minora viscoelastic collagenous folds driven by 3D pelvis gait motion,
/// with optional triangle-cell [`crate::surface_film::SurfaceFilm`] wetting and lubrication.
#[derive(Clone, Debug)]
pub struct LabiaMinoraGaitSimulation {
    pub folds: [TissueFold; 2],
    rest_roots: [Vec<(usize, Point)>; 2],
    time_s: f64,
}

impl LabiaMinoraGaitSimulation {
    #[must_use]
    pub fn time_s(&self) -> f64 {
        self.time_s
    }

    /// Advances both left and right labia minora folds by `dt_s` under `kinematics`,
    /// gravity, and barrier `contacts` (atomic on error).
    pub fn step_gait(
        &mut self,
        kinematics: &crate::biomechanics::PelvisGaitKinematics,
        dt_s: f64,
        gravity: Point,
        contacts: &super::ContactScene,
        config: super::SolverConfig,
    ) -> Result<[super::StepReport; 2], &'static str> {
        kinematics.validate()?;
        if !dt_s.is_finite() || dt_s <= 0. {
            return Err("invalid labia minora gait timestep");
        }
        let next_time = self.time_s + dt_s;
        let offset = kinematics.displacement_at(next_time)?;
        let mut candidate = self.clone();
        let mut reports = [super::StepReport::default(); 2];
        for side in 0..2 {
            for &(node, base) in &candidate.rest_roots[side] {
                candidate.folds[side].skin.set_pin_target(
                    node,
                    [
                        base[0] + offset[0],
                        base[1] + offset[1],
                        base[2] + offset[2],
                    ],
                )?;
            }
            let forces = vec![[0.; 3]; candidate.folds[side].skin.positions().len()];
            reports[side] = candidate.folds[side].skin.step_with_contacts(
                dt_s,
                gravity,
                &forces,
                &[],
                contacts,
                config,
            )?;
        }
        candidate.time_s = next_time;
        *self = candidate;
        Ok(reports)
    }

    /// Builds a triangle-cell [`crate::surface_film::SurfaceFilm`] on each deformed labia minora fold
    /// for wetting, capillary flow, and sliding contact smearing (`advance_smear`).
    pub fn build_surface_films(
        &self,
        material: crate::surface_film::Material,
    ) -> Result<[crate::surface_film::SurfaceFilm; 2], &'static str> {
        Ok([
            crate::surface_film::SurfaceFilm::new(
                self.folds[0].skin.positions(),
                self.folds[0].skin.triangles().to_vec(),
                material,
            )?,
            crate::surface_film::SurfaceFilm::new(
                self.folds[1].skin.positions(),
                self.folds[1].skin.triangles().to_vec(),
                material,
            )?,
        ])
    }

    /// Mean lateral separation distance (m) between the right (`+X`) and left (`-X`)
    /// labia minora fold crests.
    #[must_use]
    pub fn mean_crest_separation_m(&self) -> f64 {
        let mean_x = |fold: &TissueFold| -> f64 {
            if fold.crest_nodes.is_empty() {
                return 0.0;
            }
            let sum: f64 = fold
                .crest_nodes
                .iter()
                .map(|&i| fold.skin.positions()[i][0])
                .sum();
            sum / (fold.crest_nodes.len() as f64)
        };
        mean_x(&self.folds[1]) - mean_x(&self.folds[0])
    }

    /// Advances both left and right labia minora folds by `dt_s` under axial insertion/withdrawal
    /// of a rounded-tip cylindrical tube [`LabiaTubeInsertion`], combining positive-gap sphere-chain
    /// [`super::ContactScene`] barriers with radial parting and axial mucosal drag forces.
    #[allow(clippy::too_many_arguments)]
    pub fn step_tube_insertion(
        &mut self,
        tube: &LabiaTubeInsertion,
        dt_s: f64,
        gravity: Point,
        sphere_count: usize,
        barrier_distance_m: f64,
        stiffness: f64,
        radial_spreading_force_n: f64,
        config: super::SolverConfig,
    ) -> Result<[super::StepReport; 2], &'static str> {
        tube.validate()?;
        if !dt_s.is_finite()
            || dt_s <= 0.0
            || !radial_spreading_force_n.is_finite()
            || radial_spreading_force_n < 0.0
        {
            return Err("invalid labia tube insertion step parameters");
        }
        let next_time = self.time_s + dt_s;
        let (tip_pos, tip_vel, phase) = tube.tip_state_at(self.time_s)?;
        let contacts =
            tube.to_contact_scene(self.time_s, sphere_count, barrier_distance_m, stiffness)?;

        let mut candidate = self.clone();
        let mut reports = [super::StepReport::default(); 2];
        let influence_radius = (tube.radius_m * 3.0).max(0.008);

        for side in 0..2 {
            let sign = if side == 0 { -1.0 } else { 1.0 };
            let positions = candidate.folds[side].skin.positions();
            let mut forces = vec![[0.0; 3]; positions.len()];
            let crest_len = candidate.folds[side].crest_nodes.len().max(1) as f64;
            for &node in &candidate.folds[side].crest_nodes {
                let p = positions[node];
                let dy = p[1] - tip_pos[1];
                let longitudinal_weight = (-((dy / influence_radius).powi(2))).exp();
                // Penetration engagement increases as the rounded tip descends past the fold crest
                let depth_engagement = ((p[2] + 0.003 - tip_pos[2]) / (tube.max_depth_m.max(1e-4)))
                    .clamp(0.0, 1.0)
                    .max(phase * 0.85);
                let w = longitudinal_weight * depth_engagement;
                let f_lat = sign * (radial_spreading_force_n / crest_len) * w;
                let f_axial = 0.35
                    * (radial_spreading_force_n / crest_len)
                    * w
                    * (tip_vel[2] / (tube.max_speed_m_s().max(1e-4))).clamp(-1.0, 1.0);
                forces[node] = [f_lat, 0.0, f_axial];
            }
            reports[side] = candidate.folds[side].skin.step_with_contacts(
                dt_s,
                gravity,
                &forces,
                &[],
                &contacts,
                config,
            )?;
        }
        candidate.time_s = next_time;
        *self = candidate;
        Ok(reports)
    }
}

/// Kinematic and geometric specification of a smooth rounded-tip cylindrical tube
/// inserted along the $-Z$ axis into the interlabial cleft (`LabiaMinora` / `LabiaMajora`).
#[derive(Clone, Copy, Debug)]
pub struct LabiaTubeInsertion {
    /// Outer radius of the cylindrical tube shaft and hemispherical tip (m).
    pub radius_m: f64,
    /// Exposed shaft length above the hemispherical tip (m).
    pub length_m: f64,
    /// Lateral and longitudinal target coordinates `[x_m, y_m]` of the tube axis.
    pub axis_xy_m: [f64; 2],
    /// Initial $Z$ coordinate of the rounded tip before insertion (m).
    pub start_tip_z_m: f64,
    /// Maximum axial travel depth along $-Z$ from `start_tip_z_m` (m).
    pub max_depth_m: f64,
    /// Complete insertion-and-withdrawal cycle frequency (Hz).
    pub frequency_hz: f64,
}

impl LabiaTubeInsertion {
    /// Validates finite positive tube geometry and kinematics.
    pub fn validate(&self) -> Result<(), &'static str> {
        if !self.radius_m.is_finite()
            || self.radius_m <= 0.0
            || !self.length_m.is_finite()
            || self.length_m <= self.radius_m
            || self.axis_xy_m.iter().any(|v| !v.is_finite())
            || !self.start_tip_z_m.is_finite()
            || !self.max_depth_m.is_finite()
            || self.max_depth_m <= 0.0
            || !self.frequency_hz.is_finite()
            || self.frequency_hz <= 0.0
        {
            return Err("invalid labia tube insertion parameters");
        }
        Ok(())
    }

    /// Peak axial speed (m/s) during the harmonic insertion/withdrawal cycle.
    #[must_use]
    pub fn max_speed_m_s(&self) -> f64 {
        std::f64::consts::PI * self.frequency_hz * self.max_depth_m
    }

    /// Returns `(tip_position_m, tip_velocity_m_s, normalized_insertion_phase)` at `time_s`,
    /// where `normalized_insertion_phase` $\in [0, 1]$ is $0$ at `start_tip_z_m` and $1$ at
    /// `start_tip_z_m - max_depth_m`.
    pub fn tip_state_at(&self, time_s: f64) -> Result<(Point, Point, f64), &'static str> {
        self.validate()?;
        if !time_s.is_finite() || time_s < 0.0 {
            return Err("invalid labia tube time");
        }
        let omega = std::f64::consts::TAU * self.frequency_hz;
        let phase = 0.5 * (1.0 - (omega * time_s).cos());
        let dphase_dt = 0.5 * omega * (omega * time_s).sin();
        let tip_z = self.start_tip_z_m - self.max_depth_m * phase;
        let vel_z = -self.max_depth_m * dphase_dt;
        Ok((
            [self.axis_xy_m[0], self.axis_xy_m[1], tip_z],
            [0.0, 0.0, vel_z],
            phase,
        ))
    }

    /// Builds a [`super::ContactScene`] representing the rounded-tip cylindrical tube at `time_s`
    /// as a dense chain of `sphere_count` overlapping [`super::ContactSphere`] primitives along $+Z$.
    pub fn to_contact_scene(
        &self,
        time_s: f64,
        sphere_count: usize,
        barrier_distance_m: f64,
        stiffness: f64,
    ) -> Result<super::ContactScene, &'static str> {
        if !(2..=64).contains(&sphere_count)
            || !barrier_distance_m.is_finite()
            || barrier_distance_m <= 0.0
            || !stiffness.is_finite()
            || stiffness <= 0.0
        {
            return Err("invalid labia tube contact scene configuration");
        }
        let (tip, velocity, _) = self.tip_state_at(time_s)?;
        let mut spheres = Vec::with_capacity(sphere_count);
        for i in 0..sphere_count {
            let frac = i as f64 / ((sphere_count - 1) as f64);
            let z = tip[2] + self.radius_m + frac * (self.length_m - self.radius_m);
            spheres.push(super::ContactSphere {
                center: [tip[0], tip[1], z],
                radius: self.radius_m,
                velocity,
            });
        }
        Ok(super::ContactScene {
            spheres,
            planes: Vec::new(),
            distance: barrier_distance_m,
            stiffness,
        })
    }
}

