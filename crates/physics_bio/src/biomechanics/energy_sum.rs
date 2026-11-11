//! Compensated endpoint energy accumulation; independent of integrated force work.
#[derive(Default)]
pub(super) struct EnergySum {
    sum: f64,
    correction: f64,
}
impl EnergySum {
    pub(super) fn new(initial: f64) -> Self {
        Self {
            sum: initial,
            correction: 0.,
        }
    }
    pub(super) fn add(&mut self, contribution: f64) {
        let old = self.sum;
        let sum = old + contribution;
        self.correction += if old.abs() >= contribution.abs() {
            (old - sum) + contribution
        } else {
            (contribution - sum) + old
        };
        self.sum = sum;
    }
    pub(super) fn finish(self) -> f64 {
        self.sum + self.correction
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn endpoint_material_keeps_small_cells_across_element_order() {
        let material = Material::from_young_poisson(300., 0.4).unwrap();
        let mut small = material.clone();
        small.shear_pa *= 2_f64.powi(-54);
        small.bulk_pa *= 2_f64.powi(-54);
        let mut rest = Vec::new();
        let mut positions = Vec::new();
        let mut cells = Vec::new();
        for cell in 0..65 {
            let offset = 2. * cell as f64;
            rest.extend([
                [offset, 0., 0.],
                [offset + 1., 0., 0.],
                [offset, 1., 0.],
                [offset, 0., 1.],
            ]);
            positions.extend([
                [offset, 0., 0.],
                [offset + 1., 0., 0.],
                [offset, 1.125, 0.],
                [offset, 0., 1.],
            ]);
            cells.push((
                [4 * cell, 4 * cell + 1, 4 * cell + 2, 4 * cell + 3],
                if cell == 0 {
                    material.clone()
                } else {
                    small.clone()
                },
            ));
        }
        let body = Body::new(rest.clone(), vec![false; rest.len()], cells.clone()).unwrap();
        let big_body = Body::new(
            rest[..4].to_vec(),
            vec![false; 4],
            vec![([0, 1, 2, 3], material)],
        )
        .unwrap();
        let small_body = Body::new(
            rest[..4].to_vec(),
            vec![false; 4],
            vec![([0, 1, 2, 3], small)],
        )
        .unwrap();
        // Independent single-cell evaluations define the exact power-of-two scale oracle.
        let (big, _) = big_body.evaluate(&positions[..4]).unwrap();
        let (tiny, _) = small_body.evaluate(&positions[..4]).unwrap();
        assert_eq!(tiny, big * 2_f64.powi(-54));
        let expected = big + 64. * tiny;
        assert!(expected > big);
        assert_eq!((0..64).fold(big, |sum, _| sum + tiny), big);
        let (energy, gradient) = body.evaluate(&positions).unwrap();
        assert_eq!(energy, expected);
        cells.reverse();
        let reversed = Body::new(rest.clone(), vec![false; rest.len()], cells).unwrap();
        let (reverse_energy, reverse_gradient) = reversed.evaluate(&positions).unwrap();
        assert_eq!(reverse_energy, expected);
        assert_eq!(gradient, reverse_gradient);
    }
}
