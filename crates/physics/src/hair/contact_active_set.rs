//! Coupled unilateral penalty solve, with damped active-set updates.
use super::{cholesky, constraint, BAND};
use std::ops::Range;

#[derive(Clone, Copy)]
pub(super) struct Plane {
    pub(super) jacobian: [(usize, f64); 6],
    pub(super) gap: f64,
    pub(super) stiffness: f64,
}

#[allow(dead_code)]
pub(super) struct ContactSolution {
    pub(super) correction: Vec<f64>,
    pub(super) active: Vec<bool>,
    pub(super) iterations: usize,
}

fn gap(plane: &Plane, correction: &[f64]) -> f64 {
    plane.gap
        + plane
            .jacobian
            .iter()
            .map(|&(i, n)| n * correction[i])
            .sum::<f64>()
}

// For stiff rods, diagonal and off-diagonal terms can nearly cancel in x^T A x.
// Evaluate ||L^T x||^2 instead, using the same banded factor as the linear solve.
fn quadratic_factor(factor: &[f64], vector: &[f64]) -> f64 {
    (0..vector.len())
        .map(|column| {
            let transformed = (column..vector.len().min(column + BAND))
                .map(|row| factor[row * BAND + row - column] * vector[row])
                .sum::<f64>();
            transformed * transformed
        })
        .sum()
}

fn energy(factor: &[f64], correction: &[f64], free: &[f64], planes: &[Plane]) -> f64 {
    let displacement: Vec<_> = correction.iter().zip(free).map(|(a, b)| a - b).collect();
    0.5 * quadratic_factor(factor, &displacement)
        + planes
            .iter()
            .map(|plane| 0.5 * plane.stiffness * gap(plane, correction).min(0.).powi(2))
            .sum::<f64>()
}

// Evaluate F(x+a*d)-F(x) directly. Subtracting two total energies can
// erase the descent of a small mode next to a very stiff component.
fn energy_change(factor:&[f64],current:&[f64],free:&[f64],direction:&[f64],fraction:f64,planes:&[Plane])->f64 {
    let mut change=0.;
    for column in 0..current.len() {
        let mut displacement=0.;let mut step=0.;
        for row in column..current.len().min(column+BAND) {
            let value=factor[row*BAND+row-column];
            displacement+=value*(current[row]-free[row]);
            step+=value*direction[row];
        }
        change+=fraction*step*(displacement+0.5*fraction*step);
    }
    for plane in planes {
        let old=gap(plane,current);
        let step=plane.jacobian.iter().map(|&(i,n)|n*direction[i]).sum::<f64>()*fraction;
        let new=old+step;
        // Keep the small difference explicitly when both endpoints penetrate.
        let delta=if old<0. && new<0. {step} else {new.min(0.)-old.min(0.)};
        change+=0.5*plane.stiffness*delta*(new.min(0.)+old.min(0.));
    }
    change
}
pub(super) fn solve_contact_set(
    matrix: &[f64],
    rhs: &[f64],
    dofs: Range<usize>,
    planes: &[Plane],
) -> Result<ContactSolution, &'static str> {
    if rhs.len() < 12
        || rhs.len() % 6 != 0
        || matrix.len() != rhs.len() * BAND
        || dofs != (6..rhs.len() - 3)
        || rhs
            .iter()
            .enumerate()
            .any(|(i, &x)| !dofs.contains(&i) && x != 0.)
        || matrix.iter().chain(rhs).any(|x| !x.is_finite())
        || planes.iter().any(|plane| {
            !plane.gap.is_finite()
                || !plane.stiffness.is_finite()
                || plane.stiffness <= 0.
                || plane
                    .jacobian
                    .iter()
                    .any(|&(i, n)| !dofs.contains(&i) || !n.is_finite())
        })
    {
        return Err("invalid coupled contact input");
    }
    let mut factor = matrix.to_vec();
    let mut free = rhs.to_vec();
    cholesky(&mut factor, &mut free, dofs.clone());
    if free.iter().any(|x| !x.is_finite()) {
        return Err("coupled contact free solve overflow");
    }
    // The free minimizer also minimizes the unilateral energy when every
    // plane is open. No penalty is active, so a second identical factorization
    // cannot change the solution or discover another contact.
    if planes.iter().all(|plane| gap(plane, &free) >= 0.) {
        return Ok(ContactSolution {
            correction: free,
            active: vec![false; planes.len()],
            iterations: 0,
        });
    }
    let base_factor = factor.clone();
    let mut current = free.clone();
    let mut hessian = matrix.to_vec();
    let mut target = rhs.to_vec();
    for iteration in 1..=128 {
        let active: Vec<_> = planes
            .iter()
            .map(|plane| gap(plane, &current) < 0.)
            .collect();
        hessian.clone_from_slice(matrix);
        target.clone_from_slice(rhs);
        for (plane, &enabled) in planes.iter().zip(&active) {
            if enabled {
                constraint(
                    &mut hessian,
                    &mut target,
                    &plane.jacobian,
                    plane.gap,
                    plane.stiffness,
                );
            }
        }
        if hessian.iter().chain(&target).any(|x| !x.is_finite()) {
            return Err("coupled contact assembly overflow");
        }
        factor.clone_from(&hessian);
        cholesky(&mut factor, &mut target, dofs.clone());
        if target.iter().any(|x| !x.is_finite()) {
            return Err("coupled contact solve overflow");
        }
        if planes
            .iter()
            .zip(&active)
            .all(|(plane, &enabled)| (gap(plane, &target) < 0.) == enabled)
        {
            return Ok(ContactSolution {
                correction: target,
                active,
                iterations: iteration,
            });
        }
        // The piecewise quadratic is strictly convex for an SPD rod system.
        // Backtrack the Newton direction when crossing another contact boundary.
        let direction: Vec<_> = target.iter().zip(&current).map(|(a, b)| a - b).collect();
        let derivative = -quadratic_factor(&factor, &direction);
        let previous = energy(&base_factor, &current, &free, planes);
        if !derivative.is_finite() || derivative >= 0. || !previous.is_finite() {
            return Err("coupled contact lost descent direction");
        }
        let mut accepted = false;
        let mut fraction = 1.;
        for _ in 0..32 {
            for i in 0..target.len() {
                target[i] = current[i] + fraction * direction[i];
            }
            let change = energy_change(&base_factor,&current,&free,&direction,fraction,planes);
            if change.is_finite() && change <= 1e-4 * fraction * derivative {
                current.clone_from(&target);
                accepted = true;
                break;
            }
            fraction *= 0.5;
        }
        if !accepted {
            return Err("coupled contact line search failed");
        }
    }
    Err("coupled contact active set did not converge")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn energy_difference_retains_descent_hidden_by_a_stiff_constant_mode() {
        let mut factor=vec![0.;2*BAND];factor[0]=1e12;factor[BAND]=1.;
        let current=[1.,1.];let free=[0.,0.];let direction=[0.,-0.5];
        let candidate=[1.,0.5];
        assert_eq!(energy(&factor,&candidate,&free,&[]),energy(&factor,&current,&free,&[]));
        assert_eq!(energy_change(&factor,&current,&free,&direction,1.,&[]),-0.375);
    }
    #[test]
    fn energy_difference_tracks_closing_and_opening_penalty_boundaries() {
        let mut factor=vec![0.;2*BAND];factor[0]=1.;factor[BAND]=1.;
        let planes=[Plane {jacobian:[(0,1.),(0,0.),(0,0.),(0,0.),(0,0.),(0,0.)],gap:0.,stiffness:7.}];
        for (current,direction) in [([-1.,0.],[2.,0.]),([1.,0.],[-2.,0.]),([-1.,0.],[0.25,0.])] {
            for fraction in [0.25,0.5,1.] {
                let next=std::array::from_fn::<_,2,_>(|i|current[i]+fraction*direction[i]);
                let reference=energy(&factor,&next,&[0.,0.],&planes)-energy(&factor,&current,&[0.,0.],&planes);
                assert!((energy_change(&factor,&current,&[0.,0.],&direction,fraction,&planes)-reference).abs()<1e-12);
            }
        }
    }
    #[test]
    fn open_contacts_return_the_free_minimizer_without_refactorization() {
        let (matrix, rhs, mut planes) = problem(0.75, [1., 2.]);
        let mut factor = matrix.clone();
        let mut expected = rhs.clone();
        cholesky(&mut factor, &mut expected, 6..15);
        for plane in &mut planes {
            plane.gap = 1.;
        }
        let result = solve_contact_set(&matrix, &rhs, 6..15, &planes).unwrap();
        assert_eq!(result.correction, expected);
        assert_eq!(result.active, vec![false; 2]);
        assert_eq!(result.iterations, 0);
    }

    fn problem(coupling: f64, load: [f64; 2]) -> (Vec<f64>, Vec<f64>, [Plane; 2]) {
        let mut matrix = vec![0.; 18 * BAND];
        for row in 0..18 {
            matrix[row * BAND] = 1.;
        }
        matrix[6 * BAND] = 2.;
        matrix[12 * BAND] = 2.;
        matrix[12 * BAND + 6] = coupling;
        let mut rhs = vec![0.; 18];
        rhs[6] = load[0];
        rhs[12] = load[1];
        let planes = [6, 12].map(|i| Plane {
            jacobian: [(i, 1.), (i + 1, 0.), (i + 2, 0.), (i, 0.), (i, 0.), (i, 0.)],
            gap: 0.,
            stiffness: 1000.,
        });
        (matrix, rhs, planes)
    }

    #[test]
    fn factored_energy_preserves_a_small_positive_mode_in_a_stiff_system() {
        let (mut matrix, mut rhs, _) = problem(0., [0.; 2]);
        let diagonal = 1. + 1e-12;
        matrix[6 * BAND] = 1.;
        matrix[7 * BAND] = diagonal;
        matrix[7 * BAND + 1] = 1.;
        cholesky(&mut matrix, &mut rhs, 6..15);
        let mut mode = vec![0.; 18];
        mode[6] = 1e8;
        mode[7] = -1e8;
        let expected = (diagonal - 1.) * 1e16;
        let actual = quadratic_factor(&matrix, &mode);
        assert!(
            (actual - expected).abs() < 1e-8,
            "small elastic mode lost: {actual} != {expected}"
        );
        assert!(actual > 0.);
    }
    #[test]
    fn invalid_fixed_dofs_and_contact_assembly_overflow_are_rejected() {
        let (matrix, mut rhs, mut planes) = problem(1., [-0.19, -0.08]);
        rhs[0] = 1.;
        assert!(solve_contact_set(&matrix, &rhs, 6..15, &planes).is_err());
        rhs[0] = 0.;
        assert!(solve_contact_set(&matrix, &rhs, 6..14, &planes).is_err());
        planes[0].jacobian[0] = (0, 1.);
        assert!(solve_contact_set(&matrix, &rhs, 6..15, &planes).is_err());
        planes[0].jacobian[0] = (6, 1e100);
        planes[0].stiffness = 1e308;
        planes[0].gap = -1.;
        assert!(matches!(
            solve_contact_set(&matrix, &rhs, 6..15, &planes),
            Err("coupled contact assembly overflow")
        ));
    }

    fn check_stationarity(
        matrix: &[f64],
        rhs: &[f64],
        planes: &[Plane],
        solution: &ContactSolution,
    ) {
        for row in 6..15 {
            let mut residual = -rhs[row];
            for column in 6..15 {
                let (a, b) = (row.max(column), row.min(column));
                if a - b < BAND {
                    residual += matrix[a * BAND + a - b] * solution.correction[column];
                }
            }
            for plane in planes {
                for &(i, n) in &plane.jacobian {
                    if i == row {
                        residual += plane.stiffness * gap(plane, &solution.correction).min(0.) * n;
                    }
                }
            }
            assert!(residual.abs() < 1e-12, "stationarity row {row}: {residual}");
        }
    }

    #[test]
    fn coupled_reaction_activates_a_contact_that_was_opening_in_free_motion() {
        let (matrix, rhs, planes) = problem(1., [-0.19, -0.08]);
        let solution = solve_contact_set(&matrix, &rhs, 6..15, &planes).unwrap();
        assert_eq!(solution.active, [true, true]);
        assert!(solution.iterations > 1);
        let determinant = 1002f64.powi(2) - 1.;
        assert!((solution.correction[6] - (-0.19 * 1002. + 0.08) / determinant).abs() < 1e-14);
        assert!((solution.correction[12] - (-0.08 * 1002. + 0.19) / determinant).abs() < 1e-14);
        check_stationarity(&matrix, &rhs, &planes, &solution);
    }

    #[test]
    fn coupled_reaction_releases_a_contact_that_was_closing_in_free_motion() {
        let (matrix, rhs, planes) = problem(-1., [-0.19, 0.08]);
        let solution = solve_contact_set(&matrix, &rhs, 6..15, &planes).unwrap();
        assert_eq!(solution.active, [true, false]);
        assert!(solution.iterations > 1);
        let first = (-0.19 + 0.08 / 2.) / 1001.5;
        assert!((solution.correction[6] - first).abs() < 1e-14);
        assert!((solution.correction[12] - (0.08 + first) / 2.).abs() < 1e-14);
        check_stationarity(&matrix, &rhs, &planes, &solution);
    }
}
