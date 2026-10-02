use physics::{
    cohesive::Material as Bond,
    plasticity::{
        Material,
        mesh::{QuadraticAdvanceLimits, QuadraticBody, QuadraticDynamics, QuadraticEnergy},
    },
};
use voxy_render::SceneMesh;
fn coupon() -> (QuadraticBody, Vec<bool>, [usize; 6], [usize; 6]) {
    let material = Material::new(1e7, 0.3, 1e9, 0.).unwrap();
    let mut body = QuadraticBody::from_linear(
        vec![
            [0.; 3],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [0.; 3],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., -1.],
        ],
        vec![([0, 1, 2, 3], material), ([4, 5, 6, 7], material)],
    )
    .unwrap();
    let edges = body.edge_midpoints();
    let midpoint = |a: usize, b: usize| {
        edges
            .iter()
            .find(|(edge, _)| *edge == [a.min(b), a.max(b)])
            .unwrap()
            .1
    };
    let minus = [4, 5, 6, midpoint(4, 5), midpoint(5, 6), midpoint(4, 6)];
    let plus = [0, 1, 2, midpoint(0, 1), midpoint(1, 2), midpoint(0, 2)];
    let mut upper = vec![false; body.positions().len()];
    for value in &mut upper[..4] {
        *value = true;
    }
    for (edge, node) in edges {
        upper[node] = edge[0] < 4 && edge[1] < 4;
    }
    body.add_cohesive_interface(minus, plus, Bond::new(1e6, 2e6, 1000., 10.).unwrap())
        .unwrap();
    (body, upper, minus, plus)
}

#[derive(Debug)]
pub(crate) struct FemDemo {
    dynamics: QuadraticDynamics,
    initial: QuadraticEnergy,
    elapsed_s: f64,
    pub(crate) steps: usize,
    energy_defect: f64,
    interface_error: f64,
}
impl FemDemo {
    pub(crate) fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (body, upper, _, _) = coupon();
        let velocities = upper
            .iter()
            .map(|u| [0.03, -0.02, if *u { 0.5 } else { -0.5 }])
            .collect();
        let dynamics = QuadraticDynamics::new(body, &[1000.; 2], velocities)?;
        let initial = dynamics.energy()?;
        Ok(Self {
            dynamics,
            initial,
            elapsed_s: 0.,
            steps: 0,
            energy_defect: 0.,
            interface_error: 0.,
        })
    }
    pub(crate) fn advance(&mut self, wall_dt_s: f64) -> Result<(), Box<dyn std::error::Error>> {
        if !wall_dt_s.is_finite() || wall_dt_s < 0. {
            return Err("invalid FEM frame time".into());
        }
        let interval = (wall_dt_s * 0.03).min(0.04 - self.elapsed_s);
        if interval <= 0. {
            return Ok(());
        }
        let report = self.dynamics.advance_loaded_with_fracture_work(
            interval,
            &vec![[0.; 3]; self.dynamics.velocities().len()],
            [0.; 3],
            QuadraticAdvanceLimits {
                minimum_dt_s: 1e-10,
                maximum_dt_s: 1e-4,
                max_attempts: 20000,
                energy_tolerance_j: 0.01 * interval / 0.04,
            },
            0.05 * interval / 0.04,
        )?;
        self.steps += report.substeps.len();
        self.energy_defect += report.absolute_energy_defect_j;
        self.interface_error += report.interface_absolute_error_j;
        self.elapsed_s = (self.elapsed_s + interval).min(0.04);
        Ok(())
    }
    pub(crate) fn topology(&self) -> Result<(usize, usize), Box<dyn std::error::Error>> {
        Ok((
            self.dynamics
                .body()
                .exposed_faces_at(self.dynamics.body().positions())?
                .len(),
            self.dynamics.fragments()?.len(),
        ))
    }
    pub(crate) fn mesh(&self) -> Result<SceneMesh, Box<dyn std::error::Error>> {
        crate::fem_surface::fem_surface_scene_mesh(
            self.dynamics.body(),
            2,
            1000,
            [0.35, 0.35, 0.],
            1.,
            |component, normal| {
                let shade = (0.35 + 0.65 * (0.6 * normal[0] + 0.8 * normal[1]).max(0.)) as f32;
                let base = if component == 0 {
                    [0.9, 0.5, 0.08]
                } else {
                    [0.08, 0.5, 0.95]
                };
                [base[0] * shade, base[1] * shade, base[2] * shade, 1.]
            },
        )
    }
    pub(crate) fn verify(&self) -> Result<(), Box<dyn std::error::Error>> {
        let final_energy = self.dynamics.energy()?;
        if self.elapsed_s < 0.04 - 1e-12
            || self.dynamics.fragments()?.len() != 2
            || (final_energy.mass_kg - self.initial.mass_kg).abs() > 1e-10
            || (final_energy.fracture_dissipated_j - 5.).abs() > 1e-8
            || self.energy_defect > 0.01
            || self.interface_error > 0.05
        {
            return Err("FEM fracture scene invariants failed".into());
        }
        for a in 0..3 {
            if (final_energy.momentum_kg_m_s[a] - self.initial.momentum_kg_m_s[a]).abs() > 1e-7
                || (final_energy.angular_momentum_kg_m2_s[a]
                    - self.initial.angular_momentum_kg_m2_s[a])
                    .abs()
                    > 1e-7
            {
                return Err("FEM fracture scene momentum failed".into());
            }
        }
        Ok(())
    }
}
