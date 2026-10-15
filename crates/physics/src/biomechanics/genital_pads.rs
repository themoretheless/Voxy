//! Idealized volumetric labia majora cores; skin/core coupling is not implied.
use super::{Body, Material};
#[derive(Clone, Debug)]
pub struct LabialPad {
    pub body: Body,
    pub base_nodes: Vec<usize>,
    pub apex_node: usize,
}
#[derive(Clone, Copy, Debug)]
pub struct LabiaMajoraGeometry {
    pub length_m: f64,
    pub width_m: f64,
    pub height_m: f64,
    pub separation_m: f64,
    pub sectors: usize,
    pub latitude_rings: usize,
}
impl LabiaMajoraGeometry {
    /// Two mirrored half-ellipsoid solid pads with fully fixed basal disk.
    /// Y is longitudinal, X lateral, Z outward. Material is explicit and must
    /// represent the selected adipose/core experiment; no fitted defaults.
    /// # Errors
    /// Invalid geometry, overlap, resolution or tetrahedral constitutive failure.
    pub fn build(self, material: Material) -> Result<[LabialPad; 2], &'static str> {
        if [
            self.length_m,
            self.width_m,
            self.height_m,
            self.separation_m,
        ]
        .iter()
        .any(|x| !x.is_finite() || *x <= 0.)
            || self.separation_m <= self.width_m
            || !(8..=128).contains(&self.sectors)
            || !(2..=32).contains(&self.latitude_rings)
        {
            return Err("invalid labia majora pad geometry");
        }
        let build = |side: f64| {
            let center_x = side * self.separation_m / 2.;
            let index = |r: usize, a: usize| r * self.sectors + a % self.sectors;
            let mut points = Vec::new();
            for ring in 0..self.latitude_rings {
                let theta =
                    std::f64::consts::FRAC_PI_2 * (1. - ring as f64 / self.latitude_rings as f64);
                for sector in 0..self.sectors {
                    let phi = std::f64::consts::TAU * sector as f64 / self.sectors as f64;
                    points.push([
                        center_x + side * self.width_m / 2. * theta.sin() * phi.cos(),
                        self.length_m / 2. * theta.sin() * phi.sin(),
                        if ring == 0 {
                            0.
                        } else {
                            self.height_m * theta.cos()
                        },
                    ]);
                }
            }
            let apex = points.len();
            points.push([center_x, 0., self.height_m]);
            let base_center = points.len();
            points.push([center_x, 0., 0.]);
            let seed = points.len();
            points.push([center_x, 0., self.height_m / 3.]);
            let mut faces = Vec::new();
            for ring in 0..self.latitude_rings - 1 {
                for a in 0..self.sectors {
                    let [u, v, w, q] = [
                        index(ring, a),
                        index(ring, a + 1),
                        index(ring + 1, a),
                        index(ring + 1, a + 1),
                    ];
                    faces.extend([[u, v, q], [u, q, w]]);
                }
            }
            for a in 0..self.sectors {
                faces.push([
                    index(self.latitude_rings - 1, a),
                    index(self.latitude_rings - 1, a + 1),
                    apex,
                ]);
                faces.push([index(0, a + 1), index(0, a), base_center]);
            }
            let roots: Vec<_> = (0..self.sectors).chain([base_center]).collect();
            let pins = (0..points.len())
                .map(|i| i < self.sectors || i == base_center)
                .collect();
            let cells = faces
                .into_iter()
                .map(|f| ([seed, f[0], f[1], f[2]], material.clone()))
                .collect();
            Ok(LabialPad {
                body: Body::new(points, pins, cells)?,
                base_nodes: roots,
                apex_node: apex,
            })
        };
        Ok([build(-1.)?, build(1.)?])
    }

    /// Construct dynamic nonlinear FEM bodies for left and right labia majora pads
    /// with basal disk nodes configured as prescribed supports.
    pub fn build_dynamic(
        self,
        materials: [Material; 2],
        density_kg_m3: [f64; 2],
    ) -> Result<([super::InertialBody; 2], [Vec<usize>; 2], [usize; 2]), &'static str> {
        if density_kg_m3.iter().any(|d| !d.is_finite() || *d <= 0.) {
            return Err("invalid labia majora density");
        }
        let [left_pad, right_pad] = [
            self.build(materials[0].clone())?[0].clone(),
            self.build(materials[1].clone())?[1].clone(),
        ];
        let make_dynamic =
            |pad: LabialPad,
             density: f64|
             -> Result<(super::InertialBody, Vec<usize>, usize), &'static str> {
                let densities = vec![density; pad.body.elements.len()];
                let velocities = vec![[0.; 3]; pad.body.positions.len()];
                let dynamic =
                    super::InertialBody::new_with_fixed_supports(pad.body, &densities, velocities)?;
                Ok((dynamic, pad.base_nodes, pad.apex_node))
            };
        let (left_dyn, left_base, left_apex) = make_dynamic(left_pad, density_kg_m3[0])?;
        let (right_dyn, right_base, right_apex) = make_dynamic(right_pad, density_kg_m3[1])?;
        Ok((
            [left_dyn, right_dyn],
            [left_base, right_base],
            [left_apex, right_apex],
        ))
    }

    /// Assemble left and right labia majora pads into a single inertial owner
    /// with non-penetrating inter-labial surface contact.
    pub fn build_contact_assembly(
        self,
        materials: [Material; 2],
        density_kg_m3: [f64; 2],
        minimum_distance_m: f64,
        activation_gap_m: f64,
        pair_stiffness_n_m: f64,
    ) -> Result<(super::InertialAssembly, [Vec<usize>; 2], [usize; 2]), &'static str> {
        let (parts, bases, apices) = self.build_dynamic(materials, density_kg_m3)?;
        let mut assembly = super::InertialBody::assemble_tissues(&parts)?;
        let faces = assembly.body.body().surface();
        assembly
            .body
            .set_surface_contacts(vec![super::TissueSurfaceContact {
                faces,
                minimum_distance_m,
                activation_gap_m,
                pair_stiffness_n_m,
            }])?;
        Ok((assembly, bases, apices))
    }

    /// Build a coupled labia majora nonlinear FEM + inter-labial contact simulation
    /// driven by 3D pelvis gait kinematics (walk, jog, run).
    pub fn build_gait_contact_simulation(
        self,
        materials: [Material; 2],
        density_kg_m3: [f64; 2],
        minimum_distance_m: f64,
        activation_gap_m: f64,
        pair_stiffness_n_m: f64,
    ) -> Result<LabiaGaitSimulation, &'static str> {
        let (mut assembly, bases, local_apices) = self.build_contact_assembly(
            materials,
            density_kg_m3,
            minimum_distance_m,
            activation_gap_m,
            pair_stiffness_n_m,
        )?;
        assembly.body.set_uniform_acceleration([0., -9.81, 0.])?;
        let mut support_nodes = Vec::new();
        for (side, locals) in bases.iter().enumerate() {
            for &local in locals {
                let node = assembly.node_ranges[side].start + local;
                let rest_pos = assembly.body.body().positions()[node];
                support_nodes.push((node, rest_pos));
            }
        }
        let apex_nodes = [
            assembly.node_ranges[0].start + local_apices[0],
            assembly.node_ranges[1].start + local_apices[1],
        ];
        let min_contact_gap_m = assembly
            .body
            .body()
            .minimum_surface_contact_distance()?
            .unwrap_or(f64::INFINITY);
        Ok(LabiaGaitSimulation {
            assembly,
            support_nodes,
            apex_nodes,
            time_s: 0.,
            min_contact_gap_m,
        })
    }
}

/// Coupled left/right labia majora FEM + inter-labial contact simulation under pelvis gait motion.
#[derive(Clone, Debug)]
pub struct LabiaGaitSimulation {
    pub assembly: super::InertialAssembly,
    support_nodes: Vec<(usize, super::Vec3)>,
    apex_nodes: [usize; 2],
    time_s: f64,
    min_contact_gap_m: f64,
}

impl LabiaGaitSimulation {
    #[must_use]
    pub fn time_s(&self) -> f64 {
        self.time_s
    }

    #[must_use]
    pub fn min_contact_gap_m(&self) -> f64 {
        self.min_contact_gap_m
    }

    #[must_use]
    pub fn apex_nodes(&self) -> [usize; 2] {
        self.apex_nodes
    }

    /// Advances both labia majora pads by `dt_s` under 3D harmonic pelvis gait motion.
    pub fn step_gait(
        &mut self,
        kinematics: &super::PelvisGaitKinematics,
        dt_s: f64,
        tolerance_j: f64,
    ) -> Result<super::DrivenSupportStep, &'static str> {
        kinematics.validate()?;
        if !dt_s.is_finite() || dt_s <= 0. {
            return Err("invalid labia gait timestep");
        }
        let next_time = self.time_s + dt_s;
        let offset = kinematics.displacement_at(next_time)?;
        let targets: Vec<_> = self
            .support_nodes
            .iter()
            .map(|&(node, base)| super::SupportTarget {
                node,
                position_m: [
                    base[0] + offset[0],
                    base[1] + offset[1],
                    base[2] + offset[2],
                ],
            })
            .collect();
        let report = self
            .assembly
            .body
            .step_with_support_targets(&targets, dt_s, tolerance_j)?;
        if let Some(gap) = self
            .assembly
            .body
            .body()
            .minimum_surface_contact_distance()?
        {
            self.min_contact_gap_m = self.min_contact_gap_m.min(gap);
        }
        self.time_s = next_time;
        Ok(report)
    }

    /// Lateral distance (m) between the right (`+X`) and left (`-X`) labia majora apex nodes.
    #[must_use]
    pub fn apex_separation_m(&self) -> f64 {
        let pos = self.assembly.body.body().positions();
        pos[self.apex_nodes[1]][0] - pos[self.apex_nodes[0]][0]
    }

    /// Advances both labia majora pads by `dt_s` during axial tube insertion/withdrawal,
    /// applying radial parting velocity impulses and axial mucosal drag to free medial nodes
    /// while maintaining the inter-labial surface contact barrier.
    pub fn step_tube_insertion(
        &mut self,
        insertion_phase: f64,
        axial_velocity_m_s: f64,
        radial_spreading_speed_m_s: f64,
        dt_s: f64,
        tolerance_j: f64,
    ) -> Result<super::DrivenSupportStep, &'static str> {
        if !insertion_phase.is_finite()
            || !(0.0..=1.0).contains(&insertion_phase)
            || !axial_velocity_m_s.is_finite()
            || !radial_spreading_speed_m_s.is_finite()
            || radial_spreading_speed_m_s < 0.0
            || !dt_s.is_finite()
            || dt_s <= 0.0
        {
            return Err("invalid labia majora tube insertion step parameters");
        }
        let next_time = self.time_s + dt_s;
        let gain = (dt_s * 240.0).clamp(0.02, 0.45);
        let impulses = [
            (
                self.apex_nodes[0],
                [
                    -radial_spreading_speed_m_s * insertion_phase * gain,
                    0.0,
                    0.25 * axial_velocity_m_s * insertion_phase * gain,
                ],
            ),
            (
                self.apex_nodes[1],
                [
                    radial_spreading_speed_m_s * insertion_phase * gain,
                    0.0,
                    0.25 * axial_velocity_m_s * insertion_phase * gain,
                ],
            ),
        ];
        self.assembly
            .body
            .apply_free_nodal_velocity_impulses(&impulses)?;

        let lateral_support_shift = 0.018 * radial_spreading_speed_m_s * insertion_phase;
        let targets: Vec<_> = self
            .support_nodes
            .iter()
            .map(|&(node, base)| {
                let sign = if base[0] < 0.0 { -1.0 } else { 1.0 };
                super::SupportTarget {
                    node,
                    position_m: [base[0] + sign * lateral_support_shift, base[1], base[2]],
                }
            })
            .collect();
        let report = self
            .assembly
            .body
            .step_with_support_targets(&targets, dt_s, tolerance_j)?;

        if let Some(gap) = self
            .assembly
            .body
            .body()
            .minimum_surface_contact_distance()?
        {
            self.min_contact_gap_m = self.min_contact_gap_m.min(gap);
        }
        self.time_s = next_time;
        Ok(report)
    }
}
