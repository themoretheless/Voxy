//! Admitted, diagonally equilibrated nine-band systems for compensated GPU solving.
#[derive(Clone, Copy, Debug)]
pub struct BandedSystem<'a> {
    pub matrix: &'a [f64],
    pub rhs: &'a [f64],
}
#[derive(Clone, Debug, PartialEq)]
pub enum BandedSolveError {
    InvalidInput,
    Capacity,
    NumericRange,
    InvalidOutput,
    Factorization { system: usize, status: u32 },
    Residual { system: usize },
}
impl std::fmt::Display for BandedSolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "banded solve: {self:?}")
    }
}
impl std::error::Error for BandedSolveError {}
#[derive(Debug)]
pub struct BandedSolveInput {
    words: Vec<u32>,
    scales: Vec<Vec<f64>>,
    n: usize,
    active: std::ops::Range<usize>,
}
impl BandedSolveInput {
    pub fn new(
        systems: &[BandedSystem<'_>],
        active: std::ops::Range<usize>,
    ) -> Result<Self, BandedSolveError> {
        Self::new_with_underflow_tolerance(systems, active, 0.)
    }
    /// Allow only bounded absolute underflow in equilibrated input. The physics
    /// caller must still audit corrections against its original f64 matrices.
    pub fn new_with_underflow_tolerance(
        systems: &[BandedSystem<'_>], active: std::ops::Range<usize>, max_error: f64,
    ) -> Result<Self, BandedSolveError> {
        if !max_error.is_finite() || !(0.0..=1e-40).contains(&max_error) {return Err(BandedSolveError::InvalidInput);}
        let n = systems
            .first()
            .ok_or(BandedSolveError::InvalidInput)?
            .rhs
            .len();
        if active.start >= active.end
            || active.end > n
            || n > u32::MAX as usize
            || systems.len() > u32::MAX as usize
        {
            return Err(BandedSolveError::InvalidInput);
        }
        let matrix_len = n.checked_mul(9).ok_or(BandedSolveError::Capacity)?;
        let stride = n
            .checked_mul(20)
            .and_then(|v| v.checked_add(1))
            .ok_or(BandedSolveError::Capacity)?;
        let total = stride
            .checked_mul(systems.len())
            .and_then(|v| v.checked_add(4))
            .filter(|v| *v <= u32::MAX as usize)
            .ok_or(BandedSolveError::Capacity)?;
        let mut words = Vec::new();
        words
            .try_reserve_exact(total)
            .map_err(|_| BandedSolveError::Capacity)?;
        words.extend([
            systems.len() as u32,
            n as u32,
            active.start as u32,
            active.end as u32,
        ]);
        let mut scales = Vec::new();
        for system in systems {
            if system.matrix.len() != matrix_len
                || system.rhs.len() != n
                || system
                    .matrix
                    .iter()
                    .chain(system.rhs)
                    .any(|v| !v.is_finite())
            {
                return Err(BandedSolveError::InvalidInput);
            }
            if (0..n).any(|i| system.matrix[i * 9] <= 0.) {
                return Err(BandedSolveError::InvalidInput);
            }
            let diagonal: Vec<_> = (0..n).map(|i| 1. / system.matrix[i * 9].sqrt()).collect();
            if diagonal.iter().any(|v| !v.is_finite()) {
                return Err(BandedSolveError::NumericRange);
            }
            let mut push = |value: f64| -> Result<(), BandedSolveError> {
                let hi = value as f32;
                let lo = (value - hi as f64) as f32;
                if !value.is_finite()
                    || !hi.is_finite()
                    || !lo.is_finite()
                    || (value.abs() > max_error && hi == 0. && lo == 0.)
                {
                    return Err(BandedSolveError::NumericRange);
                }
                words.extend([hi.to_bits(), lo.to_bits()]);
                Ok(())
            };
            for i in 0..n {
                for offset in 0..9 {
                    push(if offset <= i {
                        system.matrix[i * 9 + offset] * diagonal[i] * diagonal[i - offset]
                    } else {
                        0.
                    })?;
                }
            }
            for (&value, &scale) in system.rhs.iter().zip(&diagonal) {
                push(value * scale)?;
            }
            words.push(0);
            scales.push(diagonal);
        }
        Ok(Self {
            words,
            scales,
            n,
            active,
        })
    }
    pub fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.words)
    }
    pub fn dispatch(&self) -> [u32; 3] {
        [(self.scales.len() as u32).div_ceil(32), 1, 1]
    }
    /// Admit the complete batch before returning any corrections. Residual audit
    /// uses immutable equilibrated input, never the overwritten GPU factors.
    pub fn decode_checked(
        &self,
        bytes: &[u8],
        tolerance: f64,
    ) -> Result<Vec<Vec<f64>>, BandedSolveError> {
        if bytes.len() != self.bytes().len() || !tolerance.is_finite() || tolerance <= 0. {
            return Err(BandedSolveError::InvalidOutput);
        }
        let words: Vec<u32> = bytes
            .chunks_exact(4)
            .map(|v| u32::from_ne_bytes(v.try_into().unwrap()))
            .collect();
        if words[..4] != self.words[..4] {
            return Err(BandedSolveError::InvalidOutput);
        }
        let read = |data: &[u32], index: usize| {
            f32::from_bits(data[index]) as f64 + f32::from_bits(data[index + 1]) as f64
        };
        let mut result = Vec::new();
        for (system, scales) in self.scales.iter().enumerate() {
            let base = 4 + system * (self.n * 20 + 1);
            let rhs = base + self.n * 18;
            let status = words[base + self.n * 20];
            if status != 0 {
                return Err(BandedSolveError::Factorization { system, status });
            }
            for i in (0..self.active.start).chain(self.active.end..self.n) {
                if words[rhs + i * 2..rhs + i * 2 + 2] != self.words[rhs + i * 2..rhs + i * 2 + 2] {
                    return Err(BandedSolveError::InvalidOutput);
                }
            }
            let values: Vec<_> = (0..self.n).map(|i| read(&words, rhs + i * 2)).collect();
            if values.iter().any(|v| !v.is_finite()) {
                return Err(BandedSolveError::InvalidOutput);
            }
            for i in self.active.clone() {
                let b = read(&self.words, rhs + i * 2);
                let term = read(&self.words, base + i * 18) * values[i];
                let mut ax = term;
                let mut scale = term.abs() + b.abs();
                for j in i.saturating_sub(8).max(self.active.start)..i {
                    let term = read(&self.words, base + (i * 9 + i - j) * 2) * values[j];
                    ax += term;
                    scale += term.abs();
                }
                for j in i + 1..(i + 9).min(self.active.end) {
                    let term = read(&self.words, base + (j * 9 + j - i) * 2) * values[j];
                    ax += term;
                    scale += term.abs();
                }
                if !ax.is_finite() || (ax - b).abs() > tolerance * scale.max(1e-30) {
                    return Err(BandedSolveError::Residual { system });
                }
            }
            let corrections: Vec<_> = values.iter().zip(scales).map(|(v, d)| v * d).collect();
            if corrections.iter().any(|v| !v.is_finite()) {
                return Err(BandedSolveError::InvalidOutput);
            }
            result.push(corrections);
        }
        Ok(result)
    }
}
pub const BANDED_SOLVE_SHADER: &str = include_str!("hair_banded_compensated.wgsl");
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn admitted_batch_rejects_bad_status_nonfinite_and_residual_before_publication() {
        let mut matrix = vec![0.; 18];
        matrix[0] = 4.;
        matrix[9] = 9.;
        let rhs = [8., 18.];
        let system = BandedSystem {
            matrix: &matrix,
            rhs: &rhs,
        };
        let input = BandedSolveInput::new(&[system, system], 0..2).unwrap();
        let mut output = input.words.clone();
        for base in [4, 45] {
            output[base + 36] = 4f32.to_bits();
            output[base + 38] = 6f32.to_bits();
        }
        assert_eq!(
            input
                .decode_checked(bytemuck::cast_slice(&output), 1e-10)
                .unwrap(),
            vec![vec![2., 2.]; 2]
        );
        output[85] = 1;
        assert_eq!(
            input.decode_checked(bytemuck::cast_slice(&output), 1e-10),
            Err(BandedSolveError::Factorization {
                system: 1,
                status: 1
            })
        );
        output[85] = 0;
        output[81] = f32::NAN.to_bits();
        assert_eq!(
            input.decode_checked(bytemuck::cast_slice(&output), 1e-10),
            Err(BandedSolveError::InvalidOutput)
        );
        output[81] = 0;
        assert_eq!(
            input.decode_checked(bytemuck::cast_slice(&output), 1e-10),
            Err(BandedSolveError::Residual { system: 1 })
        );
        assert_eq!(
            input.words[81],
            4f32.to_bits(),
            "output rejection preserves immutable input"
        );
    }
    #[test]
    fn bounded_underflow_is_explicit_and_does_not_admit_large_losses() {
        let mut matrix=vec![0.;9];matrix[0]=1.;
        let row=BandedSystem {matrix:&matrix,rhs:&[1e-300]};
        assert_eq!(BandedSolveInput::new(&[row],0..1).unwrap_err(),BandedSolveError::NumericRange);
        let input=BandedSolveInput::new_with_underflow_tolerance(&[row],0..1,1e-40).unwrap();
        assert_eq!(input.decode_checked(input.bytes(),1e-8).unwrap(),vec![vec![0.]]);
        assert!(BandedSolveInput::new_with_underflow_tolerance(&[row],0..1,1e-20).is_err());
        assert!(BandedSolveInput::new_with_underflow_tolerance(&[row],0..1,f64::NAN).is_err());
    }
    #[test]
    fn inactive_degrees_of_freedom_must_remain_unchanged() {
        let mut matrix = vec![0.; 27];
        for i in 0..3 {
            matrix[i * 9] = 1.;
        }
        let input = BandedSolveInput::new(
            &[BandedSystem {
                matrix: &matrix,
                rhs: &[0., 2., 0.],
            }],
            1..2,
        )
        .unwrap();
        assert_eq!(
            input.decode_checked(input.bytes(), 1e-10).unwrap(),
            vec![vec![0., 2., 0.]]
        );
        let mut output = input.words.clone();
        output[4 + 54] = 1f32.to_bits();
        assert_eq!(
            input.decode_checked(bytemuck::cast_slice(&output), 1e-10),
            Err(BandedSolveError::InvalidOutput)
        );
    }
    #[test]
    fn malformed_shape_and_unrepresentable_numbers_are_rejected() {
        assert!(BandedSolveInput::new(&[], 0..1).is_err());
        assert!(
            BandedSolveInput::new(
                &[BandedSystem {
                    matrix: &[1.],
                    rhs: &[1.]
                }],
                0..1
            )
            .is_err()
        );
        let mut matrix = [0.; 9];
        matrix[0] = 1.;
        for value in [f64::NAN, f64::INFINITY, 1e300, 1e-300] {
            assert!(
                BandedSolveInput::new(
                    &[BandedSystem {
                        matrix: &matrix,
                        rhs: &[value]
                    }],
                    0..1
                )
                .is_err()
            );
        }
        matrix[0] = -1.;
        assert!(
            BandedSolveInput::new(
                &[BandedSystem {
                    matrix: &matrix,
                    rhs: &[1.]
                }],
                0..1
            )
            .is_err()
        );
    }
}
