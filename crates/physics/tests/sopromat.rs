//! Strength-of-materials validation suite (plan step E): classic closed-form
//! solutions against the small-strain solvers. Bending and torsion use the
//! quadratic (T10) body, whose 512-node budget allows one hexahedron across
//! the section but captures linear bending strain exactly; tension and yield
//! use the linear body. Each bending/torsion case runs two mesh levels and
//! requires the finer error to be the smaller one plus an absolute bound.
use physics::plasticity::{
    Material,
    mesh::{Body, QuadraticBody},
};

const E: f64 = 1e7;
const NU: f64 = 0.3;
const L: f64 = 1.;
const A: f64 = 0.1;

fn elastic() -> Material {
    Material::new(E, NU, 1e12, 0.).unwrap()
}

fn shear_modulus() -> f64 {
    E / (2. * (1. + NU))
}

/// Six-tetrahedra hexes for a `nx × n × n` bar, as linear corner cells.
fn bar_cells(nx: u32, n: u32, material: Material) -> (Vec<[f64; 3]>, Vec<([usize; 4], Material)>) {
    let width = usize::try_from(n + 1).unwrap();
    let id = |i: u32, j: u32, k: u32| {
        (usize::try_from(i).unwrap() * width + usize::try_from(j).unwrap()) * width
            + usize::try_from(k).unwrap()
    };
    let mut positions = Vec::new();
    for i in 0..=nx {
        for j in 0..=n {
            for k in 0..=n {
                positions.push([
                    L * f64::from(i) / f64::from(nx),
                    A * f64::from(j) / f64::from(n),
                    A * f64::from(k) / f64::from(n),
                ]);
            }
        }
    }
    let mut cells = Vec::new();
    for i in 0..nx {
        for j in 0..n {
            for k in 0..n {
                let v = [
                    id(i, j, k),
                    id(i + 1, j, k),
                    id(i, j + 1, k),
                    id(i + 1, j + 1, k),
                    id(i, j, k + 1),
                    id(i + 1, j, k + 1),
                    id(i, j + 1, k + 1),
                    id(i + 1, j + 1, k + 1),
                ];
                for t in [
                    [0, 1, 3, 7],
                    [0, 3, 2, 7],
                    [0, 2, 6, 7],
                    [0, 6, 4, 7],
                    [0, 4, 5, 7],
                    [0, 5, 1, 7],
                ] {
                    cells.push((t.map(|a| v[a]), material));
                }
            }
        }
    }
    (positions, cells)
}

/// Quadratic bar with one hexahedron across the section.
struct QuadBar {
    body: QuadraticBody,
    rest: Vec<[f64; 3]>,
}

impl QuadBar {
    fn new(nx: u32) -> Self {
        Self::with_section(nx, 1)
    }

    /// `n` hexahedra across the section; the 512-node budget allows `n = 2`
    /// only for short bars.
    fn with_section(nx: u32, n: u32) -> Self {
        let (positions, cells) = bar_cells(nx, n, elastic());
        let body = QuadraticBody::from_linear(positions, cells).unwrap();
        let rest = body.positions().to_vec();
        Self { body, rest }
    }

    fn prescribed(&self, rule: impl Fn([f64; 3]) -> [Option<f64>; 3]) -> Vec<[Option<f64>; 3]> {
        self.rest.iter().map(|&p| rule(p)).collect()
    }

    /// Uniform traction on the exposed faces selected by `on`, as consistent
    /// T6 nodal loads (corners 0, mid-sides one third each).
    fn face_load(&self, on: impl Fn([f64; 3]) -> bool, force: [f64; 3]) -> Vec<[f64; 3]> {
        let faces = self.body.exposed_faces_at(&self.rest).unwrap();
        let selected: Vec<_> = faces
            .into_iter()
            .filter(|face| face.nodes.iter().all(|&node| on(self.rest[node])))
            .collect();
        let total: f64 = selected.iter().map(|f| f.reference_area_m2).sum();
        let mut loads = vec![[0.; 3]; self.rest.len()];
        for face in selected {
            for &node in &face.nodes[3..] {
                for axis in 0..3 {
                    loads[node][axis] += force[axis] * face.reference_area_m2 / (3. * total);
                }
            }
        }
        loads
    }

    fn solve(&mut self, loads: &[[f64; 3]], prescribed: &[[Option<f64>; 3]]) {
        let report = self.body.equilibrate(loads, prescribed, 30, 1e-9).unwrap();
        assert!(report.converged, "{report:?}");
    }

    fn displacement(&self, node: usize) -> [f64; 3] {
        let x = self.body.positions()[node];
        let r = self.rest[node];
        [x[0] - r[0], x[1] - r[1], x[2] - r[2]]
    }

    /// Mean displacement of nodes selected by `on`.
    fn mean_displacement(&self, on: impl Fn([f64; 3]) -> bool) -> [f64; 3] {
        let mut sum = [0.; 3];
        let mut count = 0.;
        for (node, &p) in self.rest.iter().enumerate() {
            if on(p) {
                let d = self.displacement(node);
                for axis in 0..3 {
                    sum[axis] += d[axis];
                }
                count += 1.;
            }
        }
        sum.map(|v| v / count)
    }
}

fn relative_error(actual: f64, reference: f64) -> f64 {
    ((actual - reference) / reference).abs()
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn clamp_at_root(p: [f64; 3]) -> [Option<f64>; 3] {
    if near(p[0], 0.) {
        [Some(0.); 3]
    } else {
        [None; 3]
    }
}

/// Pin the bottom edge at x = 0 and roll it at x = L.
fn simple_supports(p: [f64; 3]) -> [Option<f64>; 3] {
    if near(p[2], 0.) && near(p[0], 0.) {
        [Some(0.), Some(0.), Some(0.)]
    } else if near(p[2], 0.) && near(p[0], L) {
        [None, Some(0.), Some(0.)]
    } else {
        [None; 3]
    }
}

fn check_convergence(name: &str, coarse: f64, fine: f64, reference: f64, bound: f64) {
    let (ec, ef) = (
        relative_error(coarse, reference),
        relative_error(fine, reference),
    );
    println!(
        "{name}: coarse {coarse:e} ({ec:.4}), fine {fine:e} ({ef:.4}), reference {reference:e}"
    );
    assert!(ef <= ec, "{name}: fine error {ef} not below coarse {ec}");
    assert!(ef < bound, "{name}: fine error {ef} above {bound}");
}

#[test]
fn cantilever_tip_load_converges_to_euler_bernoulli_with_timoshenko_shear() {
    let p = 0.01;
    let inertia = A.powi(4) / 12.;
    let reference =
        p * L.powi(3) / (3. * E * inertia) + p * L / ((5. / 6.) * shear_modulus() * A * A);
    let run = |nx| {
        let mut bar = QuadBar::new(nx);
        let prescribed = bar.prescribed(clamp_at_root);
        let loads = bar.face_load(|x| near(x[0], L), [0., 0., -p]);
        bar.solve(&loads, &prescribed);
        // Work-conjugate tip deflection.
        loads
            .iter()
            .enumerate()
            .map(|(node, f)| f[2] * bar.displacement(node)[2])
            .sum::<f64>()
            / p
    };
    check_convergence("cantilever", run(4), run(16), reference, 0.05);
}

#[test]
fn simply_supported_beam_under_uniform_load_matches_the_closed_form() {
    let q = 0.01;
    let inertia = A.powi(4) / 12.;
    let reference = 5. * q * L.powi(4) / (384. * E * inertia)
        + q * L * L / (8. * (5. / 6.) * shear_modulus() * A * A);
    let run = |nx| {
        let mut bar = QuadBar::new(nx);
        let prescribed = bar.prescribed(simple_supports);
        let loads = bar.face_load(|x| near(x[2], A), [0., 0., -q * L]);
        bar.solve(&loads, &prescribed);
        -bar.mean_displacement(|x| near(x[0], L / 2.) && near(x[2], 0.))[2]
    };
    check_convergence("simply supported", run(4), run(16), reference, 0.05);
}

#[test]
fn three_point_bending_deflection_matches_the_closed_form() {
    let p = 0.01;
    let inertia = A.powi(4) / 12.;
    let reference =
        p * L.powi(3) / (48. * E * inertia) + p * L / (4. * (5. / 6.) * shear_modulus() * A * A);
    let run = |nx| {
        let mut bar = QuadBar::new(nx);
        let prescribed = bar.prescribed(simple_supports);
        // Load spread over the top nodes at mid-span.
        let top: Vec<usize> = (0..bar.rest.len())
            .filter(|&node| near(bar.rest[node][0], L / 2.) && near(bar.rest[node][2], A))
            .collect();
        let mut loads = vec![[0.; 3]; bar.rest.len()];
        for &node in &top {
            loads[node][2] = -p / top.len() as f64;
        }
        bar.solve(&loads, &prescribed);
        -bar.mean_displacement(|x| near(x[0], L / 2.) && near(x[2], 0.))[2]
    };
    check_convergence("three-point", run(4), run(16), reference, 0.05);
}

#[test]
fn uniaxial_tension_reproduces_hooke_and_poisson_exactly() {
    let p = 1e-3;
    let (positions, cells) = bar_cells(4, 2, elastic());
    let mut body = Body::new(positions.clone(), cells).unwrap();
    let prescribed: Vec<[Option<f64>; 3]> = positions
        .iter()
        .map(|&x| {
            // Kill rigid modes without constraining lateral contraction.
            [
                near(x[0], 0.).then_some(0.),
                near(x[1], 0.).then_some(0.),
                near(x[2], 0.).then_some(0.),
            ]
        })
        .collect();
    let faces = body.exposed_faces().unwrap();
    let mut loads = vec![[0.; 3]; positions.len()];
    let end: Vec<_> = faces
        .into_iter()
        .filter(|face| face.nodes.iter().all(|&n| near(positions[n][0], L)))
        .collect();
    let total: f64 = end.iter().map(|f| f.area_m2).sum();
    for face in end {
        for node in face.nodes {
            loads[node][0] += p * face.area_m2 / (3. * total);
        }
    }
    let report = body.equilibrate(&loads, &prescribed, 30, 1e-10).unwrap();
    assert!(report.converged, "{report:?}");
    let strain = p / (A * A) / E;
    let tip = positions.len() - 1;
    let d = [
        body.positions()[tip][0] - positions[tip][0],
        body.positions()[tip][1] - positions[tip][1],
        body.positions()[tip][2] - positions[tip][2],
    ];
    println!(
        "tension: axial {:e} vs {:e}; lateral {:e} vs {:e}",
        d[0],
        strain * L,
        d[1],
        -NU * strain * A
    );
    assert!(relative_error(d[0], strain * L) < 1e-5);
    assert!(relative_error(d[1], -NU * strain * A) < 1e-5);
    assert!(relative_error(d[2], -NU * strain * A) < 1e-5);
    let stress = body.responses().unwrap()[0].stress.cauchy_pa[0][0];
    assert!(relative_error(stress, p / (A * A)) < 1e-5);
}

#[test]
fn displacement_controlled_coupon_yields_at_the_von_mises_stress_with_the_hardening_slope() {
    let (young, yield_pa, hardening) = (210e9, 250e6, 1e9);
    let material = Material::new(young, NU, yield_pa, hardening).unwrap();
    let mut body = Body::new(
        vec![[0.; 3], [0.1, 0., 0.], [0., 0.1, 0.], [0., 0., 0.1]],
        vec![([0, 1, 2, 3], material)],
    )
    .unwrap();
    let mut supports = [
        [Some(0.); 3],
        [Some(0.), Some(0.), Some(0.)],
        [Some(0.), None, Some(0.)],
        [Some(0.), Some(0.), None],
    ];
    let yield_strain = yield_pa / young;
    let mut history = Vec::new();
    for step in 1..=40 {
        let strain = f64::from(step) * 0.0001;
        supports[1][0] = Some(0.1 * strain);
        let report = body
            .equilibrate(&[[0.; 3]; 4], &supports, 60, 1e-5)
            .unwrap();
        assert!(report.converged);
        let stress = body.responses().unwrap()[0].stress.cauchy_pa[0][0];
        history.push((strain, stress, body.states()[0].equivalent_plastic_strain()));
    }
    // Elastic branch: σ = E ε, no plastic strain.
    for &(strain, stress, plastic) in history.iter().filter(|h| h.0 < yield_strain * 0.98) {
        assert!(
            relative_error(stress, young * strain) < 1e-3,
            "{strain} {stress}"
        );
        assert_eq!(plastic, 0.);
    }
    // Plastic branch: slope E H / (E + H), onset at the von Mises stress.
    let plastic: Vec<_> = history
        .iter()
        .filter(|h| h.0 > yield_strain * 1.5)
        .collect();
    assert!(plastic.len() >= 10);
    let (s0, s1) = (plastic[0], plastic[plastic.len() - 1]);
    let slope = (s1.1 - s0.1) / (s1.0 - s0.0);
    let expected = young * hardening / (young + hardening);
    println!("yield: slope {slope:e} vs {expected:e}; onset stress {yield_pa:e}");
    assert!(
        relative_error(slope, expected) < 0.01,
        "{slope} vs {expected}"
    );
    assert!(plastic.iter().all(|h| h.2 > 0.));
    let predicted = yield_pa + expected * (s0.0 - yield_strain);
    assert!(relative_error(s0.1, predicted) < 0.01);
}

#[test]
fn square_shaft_torsion_converges_to_the_saint_venant_twist() {
    let torque = 1e-3;
    // Saint-Venant torsion constant of a square section.
    let j = 0.1406 * A.powi(4);
    let reference = torque * L / (shear_modulus() * j);
    // Torsional stiffness converges with the section, not the length: the
    // warping of a square section needs more than one hexahedron across.
    let run = |n| {
        let mut bar = QuadBar::with_section(4, n);
        let prescribed = bar.prescribed(clamp_at_root);
        let center = [A / 2., A / 2.];
        let end: Vec<usize> = (0..bar.rest.len())
            .filter(|&node| near(bar.rest[node][0], L))
            .collect();
        let lever: f64 = end
            .iter()
            .map(|&node| {
                let r = [bar.rest[node][1] - center[0], bar.rest[node][2] - center[1]];
                r[0] * r[0] + r[1] * r[1]
            })
            .sum();
        // Tangential nodal forces ⟂ r with Σ r × f = torque.
        let mut loads = vec![[0.; 3]; bar.rest.len()];
        for &node in &end {
            let r = [bar.rest[node][1] - center[0], bar.rest[node][2] - center[1]];
            loads[node][1] = -torque * r[1] / lever;
            loads[node][2] = torque * r[0] / lever;
        }
        bar.solve(&loads, &prescribed);
        // Least-squares twist of the end face.
        let (mut num, mut den) = (0., 0.);
        for &node in &end {
            let r = [bar.rest[node][1] - center[0], bar.rest[node][2] - center[1]];
            let d = bar.displacement(node);
            num += -d[1] * r[1] + d[2] * r[0];
            den += r[0] * r[0] + r[1] * r[1];
        }
        num / den
    };
    check_convergence("torsion", run(1), run(2), reference, 0.1);
}

#[test]
fn cantilever_quadrature_stress_field_matches_navier_bending_formula() {
    let p = 0.01;
    let inertia = A.powi(4) / 12.;
    let mut bar = QuadBar::new(8);
    let prescribed = bar.prescribed(clamp_at_root);
    let loads = bar.face_load(|x| near(x[0], L), [0., 0., -p]);
    bar.solve(&loads, &prescribed);

    let samples = bar.body.quadrature_samples().unwrap();
    let mut num = 0.;
    let mut den = 0.;
    let mut checked = 0usize;
    for cell_samples in &samples {
        for q in cell_samples {
            let [x, _y, z] = q.reference_position_m;
            // Sample the Saint-Venant interior away from clamp (x=0) and tip (x=L).
            if (0.25 * L..=0.75 * L).contains(&x) {
                let z_rel = z - A / 2.;
                let navier_sigma_xx = p * (L - x) * z_rel / inertia;
                let actual_sigma_xx = q.cauchy_pa[0][0];
                num += actual_sigma_xx * navier_sigma_xx;
                den += navier_sigma_xx * navier_sigma_xx;
                checked += 1;
            }
        }
    }
    assert!(checked >= 40, "checked={checked}");
    let slope = num / den;
    println!("navier bending stress regression slope: {slope:.6} (checked {checked} Gauss points)");
    assert!(
        (slope - 1.0).abs() < 0.03,
        "Navier bending stress slope {slope} deviated from 1.0"
    );
}

#[test]
fn combined_axial_tension_and_bending_obeys_stress_and_deflection_superposition() {
    let n_axial = 0.02;
    let p_transverse = 0.01;
    let inertia = A.powi(4) / 12.;

    let solve_case = |force: [f64; 3]| {
        let mut bar = QuadBar::new(8);
        let prescribed = bar.prescribed(clamp_at_root);
        let loads = bar.face_load(|x| near(x[0], L), force);
        bar.solve(&loads, &prescribed);
        let tip = bar.mean_displacement(|x| near(x[0], L));
        let samples = bar.body.quadrature_samples().unwrap();
        (tip, samples)
    };

    let (tip_axial, samples_axial) = solve_case([n_axial, 0., 0.]);
    let (tip_bend, samples_bend) = solve_case([0., 0., -p_transverse]);
    let (tip_comb, samples_comb) = solve_case([n_axial, 0., -p_transverse]);

    for axis in [0, 2] {
        let superposed = tip_axial[axis] + tip_bend[axis];
        assert!(
            relative_error(tip_comb[axis], superposed) < 1e-6,
            "axis {axis}: combined {} vs superposed {}",
            tip_comb[axis],
            superposed
        );
    }

    let mut interior_sum_sigma = 0.;
    let mut interior_sum_expected = 0.;
    let mut num = 0.;
    let mut den = 0.;
    let mut interior_checked = 0usize;
    for ((ca, cb), cc) in samples_axial.iter().zip(&samples_bend).zip(&samples_comb) {
        for ((qa, qb), qc) in ca.iter().zip(cb).zip(cc) {
            let superposed_sigma_xx = qa.cauchy_pa[0][0] + qb.cauchy_pa[0][0];
            assert!(
                (qc.cauchy_pa[0][0] - superposed_sigma_xx).abs() < 1e-6,
                "Gauss point superposition mismatch: {} vs {}",
                qc.cauchy_pa[0][0],
                superposed_sigma_xx
            );
            let [x, _y, z] = qc.reference_position_m;
            if (0.25 * L..=0.75 * L).contains(&x) {
                let expected = n_axial / (A * A) + p_transverse * (L - x) * (z - A / 2.) / inertia;
                interior_sum_sigma += qc.cauchy_pa[0][0];
                interior_sum_expected += expected;
                num += qc.cauchy_pa[0][0] * expected;
                den += expected * expected;
                interior_checked += 1;
            }
        }
    }
    assert!(interior_checked >= 40);
    let mean_sigma = interior_sum_sigma / interior_checked as f64;
    let mean_expected = interior_sum_expected / interior_checked as f64;
    let slope = num / den;
    println!(
        "combined loading: mean axial {:e} vs {:e}, regression slope {slope:.6}",
        mean_sigma, mean_expected
    );
    assert!(relative_error(mean_sigma, mean_expected) < 0.02);
    assert!((slope - 1.0).abs() < 0.02);
}

#[test]
fn simply_supported_square_plate_under_uniform_pressure_converges_to_kirchhoff_mindlin() {
    let side = 1.0_f64;
    let h = 0.1_f64;
    let q0 = 100.0_f64; // Pa uniform downward pressure
    let flexural_rigidity = E * h.powi(3) / (12.0 * (1.0 - NU * NU));
    let shear_stiffness = (5.0 / 6.0) * shear_modulus() * h;
    // Navier double-sine series fundamental + Reissner-Mindlin transverse shear deflection
    let reference = 0.004062 * q0 * side.powi(4) / flexural_rigidity
        + 0.07367 * q0 * side.powi(2) / shear_stiffness;

    let run_plate = |n: u32| -> f64 {
        let width = usize::try_from(n + 1).unwrap();
        let id = |i: u32, j: u32, k: u32| {
            (usize::try_from(i).unwrap() * width + usize::try_from(j).unwrap()) * 2
                + usize::try_from(k).unwrap()
        };
        let mut positions = Vec::new();
        for i in 0..=n {
            for j in 0..=n {
                for k in 0..=1 {
                    positions.push([
                        side * f64::from(i) / f64::from(n),
                        side * f64::from(j) / f64::from(n),
                        h * f64::from(k),
                    ]);
                }
            }
        }
        let mut cells = Vec::new();
        for i in 0..n {
            for j in 0..n {
                let v = [
                    id(i, j, 0),
                    id(i + 1, j, 0),
                    id(i, j + 1, 0),
                    id(i + 1, j + 1, 0),
                    id(i, j, 1),
                    id(i + 1, j, 1),
                    id(i, j + 1, 1),
                    id(i + 1, j + 1, 1),
                ];
                for t in [
                    [0, 1, 3, 7],
                    [0, 3, 2, 7],
                    [0, 2, 6, 7],
                    [0, 6, 4, 7],
                    [0, 4, 5, 7],
                    [0, 5, 1, 7],
                ] {
                    cells.push((t.map(|a| v[a]), elastic()));
                }
            }
        }
        let mut body = QuadraticBody::from_linear(positions, cells).unwrap();
        let rest = body.positions().to_vec();
        let prescribed: Vec<[Option<f64>; 3]> = rest
            .iter()
            .map(|&p| {
                let on_rim =
                    near(p[0], 0.) || near(p[0], side) || near(p[1], 0.) || near(p[1], side);
                if near(p[2], 0.) && near(p[0], 0.) && near(p[1], 0.) {
                    [Some(0.), Some(0.), Some(0.)]
                } else if near(p[2], 0.) && near(p[0], side) && near(p[1], 0.) {
                    [None, Some(0.), Some(0.)]
                } else if near(p[2], 0.) && on_rim {
                    [None, None, Some(0.)]
                } else {
                    [None; 3]
                }
            })
            .collect();
        let faces = body.exposed_faces_at(&rest).unwrap();
        let top_faces: Vec<_> = faces
            .into_iter()
            .filter(|f| f.nodes.iter().all(|&node| near(rest[node][2], h)))
            .collect();
        let total_force = -q0 * side * side;
        let total_area: f64 = top_faces.iter().map(|f| f.reference_area_m2).sum();
        let mut loads = vec![[0.; 3]; rest.len()];
        for face in top_faces {
            for &node in &face.nodes[3..] {
                loads[node][2] += total_force * face.reference_area_m2 / (3.0 * total_area);
            }
        }
        let eq = body.equilibrate(&loads, &prescribed, 30, 1e-8).unwrap();
        assert!(eq.converged);
        let center_nodes: Vec<usize> = (0..rest.len())
            .filter(|&node| near(rest[node][0], side / 2.0) && near(rest[node][1], side / 2.0))
            .collect();
        let mean_w: f64 = center_nodes
            .iter()
            .map(|&node| -(body.positions()[node][2] - rest[node][2]))
            .sum::<f64>()
            / center_nodes.len() as f64;
        mean_w
    };

    check_convergence("square plate", run_plate(4), run_plate(6), reference, 0.08);
}
