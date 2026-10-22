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
                    f64::midpoint(self.inner_radius_m, self.outer_radius_m) - h * phi.cos(),
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

/// Instantaneous 3D continuum response of a periorificial mucosal/anoderm point
/// to radial tube dilation, circumferential hoop stretch, LuGre frictional shear,
/// and capillary lubricant meniscus wetting.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrificePointResponse {
    /// Monotonically dilated radial distance $R(r_{xy})$ from the tube axis (m).
    pub dilated_radius_m: f64,
    /// Radial spatial derivative $\frac{dR}{dr_{xy}} > 0$ (strictly positive everywhere, ensuring zero mesh folding).
    pub dr_dilated_dr_xy: f64,
    /// In-plane continuum displacement `[dx_m, dy_m]` combining $C^\infty$ radial dilation and withdrawal cuff grip.
    pub radial_displacement_xy_m: [f64; 2],
    /// Normalized circumferential hoop stretch ratio $\in [0, 1]$ at the orifice rim.
    pub hoop_stretch: f64,
    /// Vertical flattening displacement ($\le 0$, m) that effaces radiating rugae/folds under hoop tension.
    pub rugae_effacement_dz_m: f64,
    /// Axial frictional shear displacement (m): negative (inward invagination) during insertion ($v_z < 0$)
    /// and positive (upward everted mucosal/anoderm cuff) during withdrawal ($v_z > 0$).
    pub frictional_axial_dz_m: f64,
    /// Capillary lubricant collar elevation ($\ge 0$, m) at the tube-orifice contact ring.
    pub meniscus_ridge_dz_m: f64,
    /// Local lubricant film wetness contribution $\in [0, 1]$ around the orifice collar.
    pub contact_wetness: f64,
}

/// Reusable $C^\infty$ continuum orifice dilation, rugae effacement, and LuGre frictional
/// invagination/eversion kernel for anatomical orifices (vaginal introitus, anal sphincter).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrificeDilationKernel {
    /// Rest `[x_m, y_m]` coordinates of the orifice center.
    pub orifice_xy_m: [f64; 2],
    /// Rest semi-axis along $X$ (m).
    pub rest_radius_x_m: f64,
    /// Rest semi-axis along $Y$ (m).
    pub rest_radius_y_m: f64,
    /// Depth of the 3D anatomical lumen along $-Z$ (m).
    pub canal_depth_m: f64,
    /// Radial Gaussian decay scale $r_0$ (m) of the $C^\infty$ incompressible dilation field.
    pub dilation_falloff_radius_m: f64,
    /// Peak axial frictional drag amplitude (m) for inward invagination ($v_z < 0$) and upward cuff eversion ($v_z > 0$).
    pub frictional_axial_amplitude_m: f64,
    /// Inward radial collar tightening amplitude (m) during withdrawal cuff eversion.
    pub withdrawal_cuff_grip_m: f64,
    /// Capillary lubricant meniscus ridge height (m) at the shaft-lip contact boundary.
    pub meniscus_ridge_height_m: f64,
    /// Fraction $\in [0, 1]$ of radiating rugae height effaced at peak circumferential hoop stretch.
    pub rugae_effacement_ratio: f64,
}

impl OrificeDilationKernel {
    /// Default parameters for a vaginal introitus and labia minora mucosal vestibule.
    #[must_use]
    pub const fn vaginal_introitus_default() -> Self {
        Self {
            orifice_xy_m: [0.0, -0.0035],
            rest_radius_x_m: 0.0017,
            rest_radius_y_m: 0.0042,
            canal_depth_m: 0.0142,
            dilation_falloff_radius_m: 0.00988,
            frictional_axial_amplitude_m: 0.0038,
            withdrawal_cuff_grip_m: 0.00045,
            meniscus_ridge_height_m: 0.0012,
            rugae_effacement_ratio: 0.65,
        }
    }

    /// Default parameters for a concentric external anal sphincter and puckered anoderm verge.
    #[must_use]
    pub const fn anal_sphincter_default() -> Self {
        Self {
            orifice_xy_m: [0.0, 0.0],
            rest_radius_x_m: 0.00175,
            rest_radius_y_m: 0.00185,
            canal_depth_m: 0.0150,
            dilation_falloff_radius_m: 0.00912,
            frictional_axial_amplitude_m: 0.0042,
            withdrawal_cuff_grip_m: 0.00058,
            meniscus_ridge_height_m: 0.0011,
            rugae_effacement_ratio: 0.72,
        }
    }

    /// Validates finite positive geometric and mechanical kernel parameters.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.orifice_xy_m.iter().any(|v| !v.is_finite())
            || !self.rest_radius_x_m.is_finite()
            || self.rest_radius_x_m <= 0.0
            || !self.rest_radius_y_m.is_finite()
            || self.rest_radius_y_m <= 0.0
            || !self.canal_depth_m.is_finite()
            || self.canal_depth_m <= 0.0
            || !self.dilation_falloff_radius_m.is_finite()
            || self.dilation_falloff_radius_m <= 0.0
            || !self.frictional_axial_amplitude_m.is_finite()
            || self.frictional_axial_amplitude_m < 0.0
            || !self.withdrawal_cuff_grip_m.is_finite()
            || self.withdrawal_cuff_grip_m < 0.0
            || !self.meniscus_ridge_height_m.is_finite()
            || self.meniscus_ridge_height_m < 0.0
            || !self.rugae_effacement_ratio.is_finite()
            || !(0.0..=1.0).contains(&self.rugae_effacement_ratio)
        {
            return Err("invalid orifice dilation kernel parameters");
        }
        Ok(())
    }

    /// Computes $(\cos(n\theta), \sin(n\theta))$ directly from $(\cos\theta, \sin\theta)$ using
    /// Chebyshev recurrence without `atan2`, avoiding any branch-cut discontinuity at $\theta = \pm\pi$.
    #[must_use]
    pub fn chebyshev_radial_harmonic(cos_theta: f64, sin_theta: f64, n: usize) -> (f64, f64) {
        if n == 0 {
            return (1.0, 0.0);
        }
        let mut cos_prev = 1.0;
        let mut sin_prev = 0.0;
        let mut cos_curr = cos_theta;
        let mut sin_curr = sin_theta;
        for _ in 1..n {
            let next_cos = cos_curr * cos_theta - sin_curr * sin_theta;
            let next_sin = sin_curr * cos_theta + cos_curr * sin_theta;
            cos_prev = cos_curr;
            sin_prev = sin_curr;
            cos_curr = next_cos;
            sin_curr = next_sin;
        }
        let _ = (cos_prev, sin_prev);
        (cos_curr, sin_curr)
    }

    /// Evaluates the $C^\infty$ monotonic incompressible radial dilation mapping
    /// $$R(r) = \sqrt{r^2 + \alpha \, r_{\text{target}}^2 e^{-(r / r_0)^2}}$$
    /// and its exact radial derivative $\frac{dR}{dr}$, which remains strictly positive for all $r > 0$
    /// whenever $r_0 \ge r_{\text{target}}$.
    #[must_use]
    pub fn monotonic_radial_dilation(
        r_xy_m: f64,
        target_radius_m: f64,
        falloff_radius_m: f64,
        engagement: f64,
    ) -> (f64, f64) {
        let r = r_xy_m.max(0.0);
        let alpha = engagement.clamp(0.0, 1.0);
        let r0 = falloff_radius_m.max(1e-6);
        let u = (r / r0).powi(2);
        let exp_u = (-u).exp();
        let shift_sq = alpha * target_radius_m * target_radius_m * exp_u;
        let dilated_r = (r * r + shift_sq).sqrt();
        let dr_dilated_dr = if dilated_r > 1e-12 {
            (r * (1.0 - shift_sq / (r0 * r0))) / dilated_r
        } else {
            1.0
        };
        (dilated_r, dr_dilated_dr)
    }

    /// Evaluates the 3D continuum dilation, circumferential rugae effacement, LuGre frictional
    /// invagination/eversion, and capillary meniscus collar at `rest_xy_m` for `tube` at `time_s`.
    pub fn evaluate_point(
        &self,
        rest_xy_m: [f64; 2],
        rugae_height_m: f64,
        tube: &LabiaTubeInsertion,
        time_s: f64,
        lugre_bristle_z: f64,
    ) -> Result<OrificePointResponse, &'static str> {
        self.validate()?;
        if rest_xy_m.iter().any(|v| !v.is_finite())
            || !rugae_height_m.is_finite()
            || !lugre_bristle_z.is_finite()
        {
            return Err("invalid orifice point evaluation inputs");
        }
        let (tip_pos, tip_vel, _) = tube.tip_state_at(time_s)?;
        let rx = rest_xy_m[0] - tip_pos[0];
        let ry = rest_xy_m[1] - tip_pos[1];
        let r_xy = rx.hypot(ry);

        let engage_raw = ((0.0046 - tip_pos[2]) / 0.0112).clamp(0.0, 1.0);
        let smooth_engage = engage_raw * engage_raw * (3.0 - 2.0 * engage_raw);
        let pen_raw = ((0.0016 - tip_pos[2]) / 0.0085).clamp(0.0, 1.0);
        let smooth_pen = pen_raw * pen_raw * (3.0 - 2.0 * pen_raw);

        let target_radius_m = tube.radius_m * 1.005;
        let (dilated_r, dr_dilated_dr) = Self::monotonic_radial_dilation(
            r_xy,
            target_radius_m,
            self.dilation_falloff_radius_m,
            smooth_engage,
        );
        let delta_r = dilated_r - r_xy;
        let (dir_x, dir_y) = if r_xy > 1e-7 {
            (rx / r_xy, ry / r_xy)
        } else {
            (1.0, 0.0)
        };

        let max_speed = tube.max_speed_m_s().max(1e-4);
        let norm_vel_z =
            (0.75 * (tip_vel[2] / max_speed) + 0.25 * lugre_bristle_z).clamp(-1.0, 1.0);

        let cuff_ring =
            (-(((dilated_r - tube.radius_m * 1.03) / (tube.radius_m * 0.50)).powi(2))).exp();
        let withdrawal_grip =
            -self.withdrawal_cuff_grip_m * norm_vel_z.max(0.0) * cuff_ring * smooth_pen;
        let total_radial_shift = delta_r + withdrawal_grip;

        let hoop_stretch = (delta_r / (tube.radius_m * 0.50)).clamp(0.0, 1.0);
        let rugae_effacement_dz_m =
            -rugae_height_m.max(0.0) * self.rugae_effacement_ratio * hoop_stretch * smooth_engage;

        let ox = (rest_xy_m[0] - self.orifice_xy_m[0]) / self.rest_radius_x_m;
        let oy = (rest_xy_m[1] - self.orifice_xy_m[1]) / self.rest_radius_y_m;
        let orifice_r = ox.hypot(oy);
        let verge_w = (-(((orifice_r - 1.18) / 0.58).powi(2))).exp();
        let canal_w = (1.0 - (orifice_r / 1.45).clamp(0.0, 1.0)).powi(2);
        let contact_collar = (verge_w + 0.92 * cuff_ring + 0.45 * canal_w).clamp(0.0, 1.35);

        let frictional_axial_dz_m =
            self.frictional_axial_amplitude_m * norm_vel_z * contact_collar * smooth_engage;
        let meniscus_ridge_dz_m = self.meniscus_ridge_height_m * cuff_ring * smooth_engage;
        let periorificial_pool = (-((r_xy / (tube.radius_m * 1.85)).powi(2))).exp();
        let contact_wetness = ((0.98 * cuff_ring + 0.88 * verge_w + 0.68 * periorificial_pool)
            * (0.52 + 0.48 * smooth_engage))
            .clamp(0.0, 1.0);

        Ok(OrificePointResponse {
            dilated_radius_m: dilated_r,
            dr_dilated_dr_xy: dr_dilated_dr,
            radial_displacement_xy_m: [dir_x * total_radial_shift, dir_y * total_radial_shift],
            hoop_stretch,
            rugae_effacement_dz_m,
            frictional_axial_dz_m,
            meniscus_ridge_dz_m,
            contact_wetness,
        })
    }
}

#[cfg(test)]
mod orifice_tests {
    use super::{LabiaTubeInsertion, OrificeDilationKernel};

    #[test]
    fn chebyshev_radial_harmonics_are_branch_cut_free_and_exact() {
        for k in 0..=64 {
            let theta = -std::f64::consts::PI + (std::f64::consts::TAU * (k as f64) / 64.0);
            let (sin_t, cos_t) = theta.sin_cos();
            for n in [1_usize, 2, 7, 14, 28] {
                let (c_n, s_n) = OrificeDilationKernel::chebyshev_radial_harmonic(cos_t, sin_t, n);
                let expected_c = ((n as f64) * theta).cos();
                let expected_s = ((n as f64) * theta).sin();
                assert!((c_n - expected_c).abs() < 1e-12);
                assert!((s_n - expected_s).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn monotonic_radial_dilation_has_strictly_positive_derivative() {
        let r_tube = 0.0038;
        let r0 = r_tube * 2.4;
        for alpha_step in 0..=10 {
            let alpha = (alpha_step as f64) / 10.0;
            let mut prev_r = -1.0;
            for i in 1..=200 {
                let r_xy = (i as f64) * 0.0001;
                let (dilated_r, dr_dr) =
                    OrificeDilationKernel::monotonic_radial_dilation(r_xy, r_tube, r0, alpha);
                assert!(
                    dr_dr > 0.0,
                    "derivative must be strictly positive at r={r_xy}, alpha={alpha}, got {dr_dr}"
                );
                assert!(dilated_r > prev_r);
                prev_r = dilated_r;
            }
        }
    }

    #[test]
    fn orifice_kernel_captures_insertion_invagination_rugae_effacement_and_withdrawal_eversion() {
        let kernel = OrificeDilationKernel::anal_sphincter_default();
        let tube = LabiaTubeInsertion {
            radius_m: 0.0038,
            length_m: 0.025,
            axis_xy_m: [0.0, 0.0],
            start_tip_z_m: 0.0072,
            max_depth_m: 0.0150,
            frequency_hz: 1.0,
        };
        let verge_pt = [0.0020, 0.0];
        let rugae_h = 0.00055;

        // 1. Mid-insertion (t = 0.32s, v_z < 0): inward frictional invagination (dz < 0)
        let ins = kernel
            .evaluate_point(verge_pt, rugae_h, &tube, 0.32, -0.8)
            .expect("insertion eval");
        assert!(ins.frictional_axial_dz_m < -0.0010);
        assert!(ins.dilated_radius_m > 0.0038);
        assert!(ins.rugae_effacement_dz_m < -0.0001);

        // 2. Full depth (t = 0.50s): maximum dilation & rugae effacement
        let full = kernel
            .evaluate_point(verge_pt, rugae_h, &tube, 0.50, 0.0)
            .expect("full depth eval");
        assert!(full.hoop_stretch > 0.45);
        assert!(full.rugae_effacement_dz_m < ins.rugae_effacement_dz_m);
        assert!(full.contact_wetness > 0.85);

        // 3. Withdrawal (t = 0.72s, v_z > 0): upward anoderm cuff eversion (dz > 0)
        let wit = kernel
            .evaluate_point(verge_pt, rugae_h, &tube, 0.72, 0.85)
            .expect("withdrawal eval");
        assert!(wit.frictional_axial_dz_m > 0.0010);
        assert!(wit.meniscus_ridge_dz_m > 0.0002);
    }
}
