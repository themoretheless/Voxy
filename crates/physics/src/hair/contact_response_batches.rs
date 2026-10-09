//! Assemble contact loads once per rod and admit the complete backend batch.
use super::{Constraint, HairRod, PositionIncrement, Response};
use crate::hair::math::dot;
use crate::hair::{HairLinearSolver, HairLinearSystem, HairResponseSystem, direct};

pub(super) fn prepare(
    constraints: &mut [Constraint],
    rods: &[HairRod],
    dt: f64,
    solver: &mut dyn HairLinearSolver,
) -> Result<(), &'static str> {
    prepare_impl(constraints, rods, dt, solver, None)
}

pub(super) fn prepare_with_free(
    constraints: &mut [Constraint],
    rods: &[HairRod],
    dt: f64,
    solver: &mut dyn HairLinearSolver,
    free: &mut PositionIncrement,
) -> Result<(), &'static str> {
    prepare_impl(constraints, rods, dt, solver, Some(free))
}

fn prepare_impl(
    constraints: &mut [Constraint],
    rods: &[HairRod],
    dt: f64,
    solver: &mut dyn HairLinearSolver,
    mut free: Option<&mut PositionIncrement>,
) -> Result<(), &'static str> {
    let mut requests: Vec<Option<HairResponseSystem>> = vec![None; rods.len()];
    // A free Newton step owns every rod, including rods outside the contact
    // graph. Omitting them would silently freeze unconstrained guides.
    if free.is_some() {
        for (index, rod) in rods.iter().enumerate() {
            requests[index] = Some(request_for_rod(rod, dt, true)?);
        }
    }
    let mut mapping = Vec::new();
    for (constraint_index, constraint) in constraints.iter().enumerate() {
        let mut indices: Vec<_> = constraint
            .entries
            .iter()
            .filter(|entry| dot(entry.gradient, entry.gradient) > 0.)
            .map(|entry| entry.rod)
            .collect();
        indices.sort_unstable();
        indices.dedup();
        for index in indices {
            let rod = &rods[index];
            let n = rod.x.len();
            if requests[index].is_none() {
                requests[index] = Some(request_for_rod(rod, dt, free.is_some())?);
            }
            let request = requests[index].as_mut().unwrap();
            let mut load = vec![0.; n * 6];
            for entry in constraint
                .entries
                .iter()
                .filter(|entry| entry.rod == index && entry.point != 0)
            {
                for axis in 0..3 {
                    load[entry.point * 6 + axis] += entry.gradient[axis];
                }
            }
            mapping.push((constraint_index, index, request.loads.len()));
            request.loads.push(load);
        }
    }
    let mut rod_to_batch = vec![None; rods.len()];
    let mut batch = Vec::new();
    for (index, request) in requests.into_iter().enumerate() {
        if let Some(request) = request {
            rod_to_batch[index] = Some(batch.len());
            batch.push(request);
        }
    }
    if batch.is_empty() {
        return Ok(());
    }
    let result = solver.solve_responses(&batch)?;
    if result.len() != batch.len() {
        return Err("contact response batch count mismatch");
    }
    // Validate everything before modifying even the temporary constraint set.
    for (request, values) in batch.iter().zip(&result) {
        if values.len() != request.loads.len() {
            return Err("contact response load count mismatch");
        }
        for (load, value) in request.loads.iter().zip(values) {
            request.system.validate_load_correction(value, load)?;
        }
    }
    let mut staged: Vec<Vec<Response>> = (0..constraints.len()).map(|_| Vec::new()).collect();
    for (constraint, index, load) in mapping {
        let n = rods[index].x.len();
        let force = &result[rod_to_batch[index].unwrap()][load];
        staged[constraint].push(Response {
            rod: index,
            linear: (0..n)
                .map(|i| std::array::from_fn(|axis| force[i * 6 + axis]))
                .collect(),
            angular: (0..n - 1)
                .map(|i| std::array::from_fn(|axis| force[i * 6 + 3 + axis]))
                .collect(),
        });
    }
    let diagonals: Vec<_> = constraints
        .iter()
        .zip(&staged)
        .map(|(constraint, responses)| {
            constraint
                .entries
                .iter()
                .map(|entry| {
                    responses
                        .iter()
                        .find(|response| response.rod == entry.rod)
                        .map_or(0., |response| {
                            dot(entry.gradient, response.linear[entry.point])
                        })
                })
                .sum::<f64>()
        })
        .collect();
    if diagonals
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.)
    {
        return Err("invalid shared contact rod compliance");
    }
    if let Some(free) = free.as_deref_mut() {
        for (rod, batch_index) in rod_to_batch.iter().enumerate() {
            if let Some(batch_index) = batch_index {
                let value = &result[*batch_index][0];
                for point in 0..rods[rod].x.len() {
                    free.linear[rod][point] = std::array::from_fn(|axis| value[point * 6 + axis]);
                }
                for point in 0..rods[rod].q.len() {
                    free.angular[rod][point] =
                        std::array::from_fn(|axis| value[point * 6 + 3 + axis]);
                }
            }
        }
    }
    for ((constraint, response), diagonal) in constraints.iter_mut().zip(staged).zip(diagonals) {
        constraint.response = response;
        constraint.diagonal = diagonal;
    }
    Ok(())
}

pub(super) fn request_for_rod(
    rod: &HairRod,
    dt: f64,
    include_free: bool,
) -> Result<HairResponseSystem, &'static str> {
    let n = rod.x.len();
    let mut staged = rod.clone();
    staged.contacts.clear();
    let (mut matrix, mut rhs) = direct::assemble(&mut staged, dt)?;
    for value in &mut matrix {
        *value *= dt * dt;
    }
    let mut loads = Vec::new();
    if include_free {
        let mut load = rhs.clone();
        for value in &mut load {
            *value *= dt * dt;
        }
        load[..6].fill(0.);
        load[n * 6 - 3..].fill(0.);
        loads.push(load);
    }
    rhs.fill(0.);
    Ok(HairResponseSystem {
        system: HairLinearSystem {
            band_width: 9,
            matrix,
            rhs,
            active: 6..n * 6 - 3,
        },
        loads,
    })
}
