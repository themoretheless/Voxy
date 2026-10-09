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
/// Immutable normalized input as represented by the shader's hi/lo words.
/// Consumers own reference solving; this codec does not implement another solver.
#[derive(Debug)]
pub struct BandedPackedReference {
    pub matrix:Vec<f64>,
    pub rhs:Vec<f64>,
    pub scales:Vec<f64>,
}
#[derive(Debug)]
pub struct BandedSolveInput {
    words: Vec<u32>,
    scales: Vec<Vec<f64>>,
    packing_error: [f64; 2],
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
        let mut packing_error=[0f64;2];
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
            let mut push = |value: f64, kind: usize| -> Result<(), BandedSolveError> {
                let hi = value as f32;
                let lo = (value - hi as f64) as f32;
                if !value.is_finite()
                    || !hi.is_finite()
                    || !lo.is_finite()
                    || (value.abs() > max_error && hi == 0. && lo == 0.)
                {
                    return Err(BandedSolveError::NumericRange);
                }
                packing_error[kind]=packing_error[kind].max(((hi as f64+lo as f64)-value).abs());
                words.extend([hi.to_bits(), lo.to_bits()]);
                Ok(())
            };
            for i in 0..n {
                for offset in 0..9 {
                    push(if offset <= i {
                        system.matrix[i * 9 + offset] * diagonal[i] * diagonal[i - offset]
                    } else {
                        0.
                    },0)?;
                }
            }
            for (&value, &scale) in system.rhs.iter().zip(&diagonal) {
                push(value * scale,1)?;
            }
            words.push(0);
            scales.push(diagonal);
        }
        Ok(Self {
            words,
            scales,
            packing_error,
            n,
            active,
        })
    }
    /// Repack force columns using the exact admitted coefficient words/scales.
    /// Coefficients are immutable; no matrix re-equilibration is performed.
    pub fn with_rhs(&self,rhs:&[&[f64]],max_error:f64)->Result<Self,BandedSolveError> {
        if rhs.len()!=self.scales.len() || !max_error.is_finite() || !(0.0..=1e-40).contains(&max_error) || rhs.iter().any(|values|values.len()!=self.n || values.iter().any(|value|!value.is_finite())) {return Err(BandedSolveError::InvalidInput);}
        let mut words=self.words.clone();let mut rhs_error=0f64;
        for (system,(values,scales)) in rhs.iter().zip(&self.scales).enumerate() {
            let base=4+system*(self.n*20+1)+self.n*18;
            for (i,(&value,&scale)) in values.iter().zip(scales).enumerate() {
                let normalized=value*scale;let hi=normalized as f32;let lo=(normalized-hi as f64) as f32;
                if !normalized.is_finite() || !hi.is_finite() || !lo.is_finite() || (normalized.abs()>max_error && hi==0. && lo==0.) {return Err(BandedSolveError::NumericRange);}
                rhs_error=rhs_error.max(((hi as f64+lo as f64)-normalized).abs());
                words[base+i*2]=hi.to_bits();words[base+i*2+1]=lo.to_bits();
            }
            words[base+self.n*2]=0;
        }
        Ok(Self {words,scales:self.scales.clone(),packing_error:[self.packing_error[0],rhs_error],n:self.n,active:self.active.clone()})
    }
    /// Maximum absolute hi/lo round-trip error in equilibrated matrix/RHS units.
    /// This measures input representation only, not correction or trajectory error.
    pub fn packing_error(&self)->[f64;2] {self.packing_error}
    /// Decode one immutable input system for an independent arithmetic audit.
    /// Multiplying a solved normalized correction by `scales` restores its units.
    pub fn packed_reference(&self,index:usize)->Option<BandedPackedReference> {
        let scales=self.scales.get(index)?.clone();
        let base=4+index*(self.n*20+1);
        let read=|offset:usize|f32::from_bits(self.words[offset]) as f64+f32::from_bits(self.words[offset+1]) as f64;
        Some(BandedPackedReference {
            matrix:(0..self.n*9).map(|i|read(base+i*2)).collect(),
            rhs:(0..self.n).map(|i|read(base+self.n*18+i*2)).collect(),
            scales,
        })
    }
    pub fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.words)
    }
    /// Update only right-hand sides in a successfully factored GPU batch.
    /// Exact normalized matrix/header equality is required; factors are never
    /// reused across a changed coefficient or active-range layout.
    pub fn factored_rhs_updates(&self,factored_input:&Self)->Result<Vec<(u64,Vec<u8>)>,BandedSolveError> {
        if self.words[..4]!=factored_input.words[..4] {return Err(BandedSolveError::InvalidInput);}
        let mut updates=Vec::with_capacity(self.scales.len());
        for system in 0..self.scales.len() {
            let base=4+system*(self.n*20+1);let rhs=base+self.n*18;
            if self.words[base..rhs]!=factored_input.words[base..rhs] {return Err(BandedSolveError::InvalidInput);}
            let mut payload=self.words[rhs..rhs+self.n*2+1].to_vec();
            // Shader consumes this command and publishes normal status zero.
            payload[self.n*2]=2;
            updates.push((rhs as u64*4,bytemuck::cast_slice(&payload).to_vec()));
        }
        Ok(updates)
    }
    /// Only the immutable header and solved RHS/status need host transfer.
    pub fn compact_output_size(&self)->u64 {16+self.scales.len() as u64*(self.n as u64*2+1)*4}
    /// Ordered source/destination byte ranges for a complete compact snapshot.
    pub fn compact_output_ranges(&self)->Vec<(u64,u64,u64)> {
        let mut ranges=vec![(0,0,16)];
        for system in 0..self.scales.len() {
            let source=(4+system*(self.n*20+1)+self.n*18)*4;
            let target=(4+system*(self.n*2+1))*4;
            ranges.push((source as u64,target as u64,(self.n*2+1) as u64*4));
        }
        ranges
    }
    /// Uses the same admission arithmetic as full-storage decoding.
    pub fn decode_compact_checked(&self,bytes:&[u8],tolerance:f64)->Result<Vec<Vec<f64>>,BandedSolveError> {
        self.decode_output(bytes,tolerance,true)
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
        self.decode_output(bytes,tolerance,false)
    }
    fn decode_output(&self,bytes:&[u8],tolerance:f64,compact:bool)->Result<Vec<Vec<f64>>,BandedSolveError> {
        let expected=if compact {self.compact_output_size() as usize} else {self.bytes().len()};
        if bytes.len() != expected || !tolerance.is_finite() || tolerance <= 0. {
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
            let output_rhs=if compact {4+system*(self.n*2+1)} else {rhs};
            let status = words[output_rhs + self.n * 2];
            if status != 0 {
                return Err(BandedSolveError::Factorization { system, status });
            }
            for i in (0..self.active.start).chain(self.active.end..self.n) {
                if words[output_rhs + i * 2..output_rhs + i * 2 + 2] != self.words[rhs + i * 2..rhs + i * 2 + 2] {
                    return Err(BandedSolveError::InvalidOutput);
                }
            }
            let values: Vec<_> = (0..self.n).map(|i| read(&words, output_rhs + i * 2)).collect();
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
    fn rhs_repacking_is_bitwise_equal_to_fresh_equilibration_and_checks_ranges() {
        let mut matrix=vec![0.;36];for i in 0..4 {matrix[i*9]=(i+1) as f64*0.013;}
        matrix[10]=-0.001;
        let zero=[0.;4];let a=[0.,1.234567890123e-9,-1e-12,0.];let b=[0.,-8e-8,2e-9,0.];
        let first=BandedSolveInput::new(&[BandedSystem {matrix:&matrix,rhs:&zero};2],1..3).unwrap();
        let repacked=first.with_rhs(&[&a,&b],0.).unwrap();
        let fresh=BandedSolveInput::new(&[BandedSystem {matrix:&matrix,rhs:&a},BandedSystem {matrix:&matrix,rhs:&b}],1..3).unwrap();
        assert_eq!(repacked.bytes(),fresh.bytes());assert_eq!(repacked.packing_error(),fresh.packing_error());
        assert_eq!(first.packed_reference(0).unwrap().rhs,vec![0.;4]);
        assert!(first.with_rhs(&[&a],0.).is_err());assert!(first.with_rhs(&[&a,&a[..3]],0.).is_err());
        for value in [f64::NAN,f64::INFINITY,1e300,1e-300] {
            let bad=[0.,value,0.,0.];assert!(first.with_rhs(&[&bad,&b],0.).is_err());
        }
        let tiny=[0.,1e-300,0.,0.];assert!(first.with_rhs(&[&tiny,&b],1e-40).is_ok());
        assert!(first.with_rhs(&[&a,&b],f64::NAN).is_err());assert!(first.with_rhs(&[&a,&b],1e-20).is_err());
    }
    #[test]
    fn compact_readback_preserves_full_admission_and_rejects_corruption() {
        let mut matrix=vec![0.;27];for i in 0..3 {matrix[i*9]=1.;}
        let row=BandedSystem {matrix:&matrix,rhs:&[0.,2.,0.]};
        let input=BandedSolveInput::new(&[row,row],1..2).unwrap();
        let full=input.bytes();let mut compact=vec![0;input.compact_output_size() as usize];
        let mut end=0;
        for (source,target,size) in input.compact_output_ranges() {
            assert_eq!(target,end);end=target+size;
            compact[target as usize..end as usize].copy_from_slice(&full[source as usize..(source+size) as usize]);
        }
        assert_eq!(end,input.compact_output_size());
        assert!(compact.len()<full.len()/2);
        assert_eq!(input.decode_checked(full,1e-8),input.decode_compact_checked(&compact,1e-8));
        assert!(input.decode_compact_checked(&compact[..compact.len()-4],1e-8).is_err());
        for (word,value) in [(0,999u32),(4,1f32.to_bits()),(6,f32::NAN.to_bits()),(6,0),(10,1),(10,2),(13,1f32.to_bits())] {
            let mut bad=compact.clone();bad[word*4..word*4+4].copy_from_slice(&value.to_ne_bytes());
            assert!(input.decode_compact_checked(&bad,1e-8).is_err(),"word {word} corruption admitted");
        }
    }
    #[test]
    fn factored_rhs_updates_preserve_headers_and_factor_storage() {
        let mut matrix=vec![0.;18];matrix[0]=4.;matrix[9]=9.;matrix[10]=1.;
        let old=[1.,2.];let new=[3.,4.];
        let original=BandedSolveInput::new(&[BandedSystem {matrix:&matrix,rhs:&old};2],0..2).unwrap();
        let next=BandedSolveInput::new(&[BandedSystem {matrix:&matrix,rhs:&new};2],0..2).unwrap();
        let mut gpu=original.bytes().to_vec();
        for system in 0..2 {
            let base=4+system*41;
            gpu[base*4..(base+36)*4].fill(0x5a);
        }
        let before=gpu.clone();
        for (offset,payload) in next.factored_rhs_updates(&original).unwrap() {
            let offset=offset as usize;gpu[offset..offset+payload.len()].copy_from_slice(&payload);
        }
        assert_eq!(&gpu[..16],&before[..16]);
        for system in 0..2 {
            let base=4+system*41;let rhs=base+36;
            assert_eq!(&gpu[base*4..rhs*4],&before[base*4..rhs*4]);
            assert_eq!(&gpu[rhs*4..(rhs+4)*4],&next.bytes()[rhs*4..(rhs+4)*4]);
            assert_eq!(u32::from_ne_bytes(gpu[(rhs+4)*4..(rhs+5)*4].try_into().unwrap()),2);
        }
        assert!(matches!(next.decode_checked(&gpu,1e-8),Err(BandedSolveError::Factorization {status:2,..})));
    }
    #[test]
    fn factored_rhs_updates_reject_changed_matrix_or_layout() {
        let mut matrix=vec![0.;18];matrix[0]=4.;matrix[9]=9.;matrix[10]=1.;let rhs=[1.,2.];
        let original=BandedSolveInput::new(&[BandedSystem {matrix:&matrix,rhs:&rhs}],0..2).unwrap();
        let changed_layout=BandedSolveInput::new(&[BandedSystem {matrix:&matrix,rhs:&rhs}],1..2).unwrap();
        assert_eq!(changed_layout.factored_rhs_updates(&original).unwrap_err(),BandedSolveError::InvalidInput);
        matrix[10]=2.;
        let changed_matrix=BandedSolveInput::new(&[BandedSystem {matrix:&matrix,rhs:&rhs}],0..2).unwrap();
        assert_eq!(changed_matrix.factored_rhs_updates(&original).unwrap_err(),BandedSolveError::InvalidInput);
    }
    #[test]
    fn packed_reference_decodes_immutable_words_and_physical_scales() {
        let mut matrix=vec![0.;18];matrix[0]=4.;matrix[9]=9.;matrix[10]=3.;
        let rhs=[8.,18.];
        let input=BandedSolveInput::new(&[BandedSystem {matrix:&matrix,rhs:&rhs}],0..2).unwrap();
        let saved=input.bytes().to_vec();let reference=input.packed_reference(0).unwrap();
        assert_eq!(reference.scales,vec![0.5,1./3.]);
        assert_eq!(reference.matrix[0],1.);assert_eq!(reference.matrix[9],1.);
        assert_eq!(reference.matrix[10],0.5);assert_eq!(reference.rhs,vec![4.,6.]);
        assert_eq!(input.bytes(),saved);assert!(input.packed_reference(1).is_none());
    }
    #[test]
    fn packing_audit_distinguishes_lost_low_bits_from_exact_inputs() {
        let value=0.125+2f64.powi(-28)+2f64.powi(-54);
        let mut matrix=vec![0.;18];matrix[0]=1.;matrix[9]=1.;matrix[10]=value;
        let rhs=[value,1.];let row=BandedSystem {matrix:&matrix,rhs:&rhs};
        let input=BandedSolveInput::new(&[row],0..2).unwrap();
        assert_eq!(input.packing_error(),[2f64.powi(-54);2]);
        matrix[10]=0.125;let rhs=[0.125,1.];
        let exact=BandedSolveInput::new(&[BandedSystem {matrix:&matrix,rhs:&rhs}],0..2).unwrap();
        assert_eq!(exact.packing_error(),[0.;2]);
    }
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
