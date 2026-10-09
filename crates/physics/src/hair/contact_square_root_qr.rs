//! Minimum-energy equality coordinates without forming a compliance Gram.
pub(super) fn minimum_norm(columns: &[Vec<f64>], bounds: &[f64], tolerance: f64) -> Option<Vec<f64>> {
    equality_with_reactions(columns, bounds, tolerance).map(|solution| solution.0)
}

fn equality_with_reactions(columns: &[Vec<f64>], bounds: &[f64], tolerance: f64) -> Option<(Vec<f64>, Vec<f64>)> {
    EqualityQr::new(columns).solve(&(0..columns.len()).collect::<Vec<_>>(), bounds, tolerance)
}

// Scoped to one immutable operator. Sorted active-set changes invalidate the
// suffix only; every retained prefix has exactly the original arithmetic.
struct EqualityQr<'a> {
    columns: &'a [Vec<f64>],
    valid_columns: Vec<bool>,
    supports: Vec<Vec<usize>>,
    active: Vec<usize>,
    q: Vec<Vec<f64>>,
    q_supports:Vec<Vec<usize>>,
    sparse_basis:bool,
    r: Vec<Vec<f64>>,
    rebuilt_columns: usize,
}
impl<'a> EqualityQr<'a> {
    fn new(columns: &'a [Vec<f64>]) -> Self {
        // The borrowed operator cannot change during this solver's lifetime.
        // Cache validity per identity rather than rescanning active columns.
        // Invalid inactive columns retain the original lazy rejection behavior.
        let width=columns.first().map(Vec::len);
        let valid_columns=columns.iter().map(|column|Some(column.len())==width
            && column.iter().all(|value|value.is_finite())).collect();
        let supports=columns.iter().map(|column|column.iter().enumerate()
            .filter_map(|(i,&value)|(value!=0.).then_some(i)).collect()).collect();
        Self { columns, valid_columns, supports, active: Vec::new(), q: Vec::new(), q_supports:Vec::new(), sparse_basis:false, r: Vec::new(), rebuilt_columns: 0 }
    }
    fn residuals(&self,active:&[usize],bounds:&[f64],state:&[f64])->Option<Vec<f64>> {
        if state.iter().any(|value|!value.is_finite()) {return None;}
        Some(active.iter().zip(bounds).map(|(&id,bound)|bound-accurate_products(
            self.supports[id].iter().map(|&i|(self.columns[id][i],state[i])))).collect())
    }
    fn solve(&mut self, active: &[usize], bounds: &[f64], tolerance: f64) -> Option<(Vec<f64>, Vec<f64>)> {
        let n = active.len();
        let m = self.columns.first()?.len();
        if n == 0 || n > m || bounds.len() != n
            || active.iter().any(|&i|self.valid_columns.get(i)!=Some(&true))
            || bounds.iter().any(|v| !v.is_finite())
            || !tolerance.is_finite() || tolerance <= 0. { return None; }
        let prefix = self.active.iter().zip(active).take_while(|(a,b)| a == b).count();
        self.active.truncate(prefix); self.q.truncate(prefix); self.q_supports.truncate(prefix); self.r.truncate(prefix);
        for &id in &active[prefix..] {
            self.rebuilt_columns += 1;
            let k = self.q.len();
            let mut v = self.columns[id].clone();
            let mut r = vec![0.; k + 1];
            for _ in 0..2 {
                for (j, basis) in self.q.iter().enumerate() {
                    let projection = if self.sparse_basis {
                        accurate_products(self.q_supports[j].iter().map(|&i|(basis[i],v[i])))
                    } else {accurate_dot(basis, &v)};
                    r[j] += projection;
                    for (value, &axis) in v.iter_mut().zip(basis) {
                        *value = (-projection).mul_add(axis, *value);
                    }
                    // Sparse products must never hide zero * infinity. Any
                    // nonfinite orthogonalization state is a rejected factor.
                    if self.sparse_basis && v.iter().any(|value|!value.is_finite()) {return None;}
                }
            }
            let scale = v.iter().map(|x| x.abs()).fold(0., f64::max);
            if scale == 0. || !scale.is_finite() { return None; }
            let norm = scale * v.iter().map(|x| (x / scale) * (x / scale)).sum::<f64>().sqrt();
            if norm == 0. || !norm.is_finite() { return None; }
            r[k] = norm;
            for value in &mut v { *value /= norm; }
            if self.sparse_basis {self.q_supports.push(v.iter().enumerate().filter_map(|(i,&x)|(x!=0.).then_some(i)).collect());}
            self.q.push(v); self.r.push(r); self.active.push(id);
        }
    // W=Q*R, so W^T*y=b gives R^T*z=b and y=Q*z.
    let mut z = bounds.to_vec();
    for i in 0..n {
        z[i] -= accurate_products((0..i).map(|j| (self.r[i][j], z[j])));
        z[i] /= self.r[i][i];
    }
    let mut result = accurate_column_combination(&self.q, &z, m);
    // y=W*lambda=Q*R*lambda, so the same factor gives R*lambda=z.
    for i in (0..n).rev() {
        z[i] -= accurate_products((i + 1..n).map(|j| (self.r[j][i], z[j])));
        z[i] /= self.r[i][i];
    }
    // Correct rounding in Q*z against the original columns using the same
    // factor. Keep reactions consistent with every coordinate correction.
    for _ in 0..8 {
        // Reject overflow before skipping zero products, so 0*infinity
        // cannot be hidden by exact structural sparsity.
        let mut delta=self.residuals(active,bounds,&result)?;
        // Stop at the caller's unchanged admission tolerance. Continuing
        // after admission can oscillate between neighboring rounded points.
        if delta.iter().all(|v| v.abs() <= tolerance) { break; }
        if delta.iter().any(|v| !v.is_finite()) { return None; }
        for i in 0..n {
            for j in 0..i { delta[i] -= self.r[i][j] * delta[j]; }
            delta[i] /= self.r[i][i];
        }
        for (column, value) in self.q.iter().zip(&delta) {
            for (out, axis) in result.iter_mut().zip(column) { *out += axis * value; }
        }
        for i in (0..n).rev() {
            for j in i + 1..n { delta[i] -= self.r[j][i] * delta[j]; }
            delta[i] /= self.r[i][i];
        }
        for (value, correction) in z.iter_mut().zip(delta) { *value += correction; }
    }
    (result.iter().chain(&z).all(|v| v.is_finite())).then_some((result, z))
}

}

// One immutable coordinate map for all defect refinements of an operator.
// Remove only identically zero coordinates; no magnitude/rank threshold.
pub(super) struct NonzeroCoordinates<'a> {
    columns: std::borrow::Cow<'a, [Vec<f64>]>,
    ids: Vec<usize>,
    width: usize,
}
impl<'a> NonzeroCoordinates<'a> {
    pub(super) fn new(columns: &'a [Vec<f64>]) -> Option<Self> {
        let width=columns.first()?.len();
        if columns.iter().any(|c|c.len()!=width||c.iter().any(|v|!v.is_finite())) {return None;}
        let ids:Vec<_>=(0..width).filter(|&i|columns.iter().any(|c|c[i]!=0.)).collect();
        let reduced=if ids.len()==width {std::borrow::Cow::Borrowed(columns)} else {
            std::borrow::Cow::Owned(columns.iter().map(|c|ids.iter().map(|&i|c[i]).collect()).collect())
        };
        Some(Self {columns:reduced,ids,width})
    }
    pub(super) fn coordinate_count(&self)->usize {self.ids.len()}
    pub(super) fn solve(&self,bounds:&[f64],tolerance:f64)->Option<(Vec<f64>,Vec<f64>)> {
        self.solve_with(bounds,tolerance,unilateral)
    }
    pub(super) fn solve_appended(&self,bounds:&[f64],tolerance:f64)->Option<(Vec<f64>,Vec<f64>)> {
        self.solve_with(bounds,tolerance,unilateral_appended)
    }
    fn solve_with(&self,bounds:&[f64],tolerance:f64,
        solver:fn(&[Vec<f64>],&[f64],f64)->Option<(Vec<f64>,Vec<f64>)>)->Option<(Vec<f64>,Vec<f64>)> {
        if bounds.len()!=self.columns.len() || bounds.iter().any(|v|!v.is_finite())
            || !tolerance.is_finite() || tolerance<=0. {return None;}
        if self.ids.is_empty() {
            return bounds.iter().all(|&v|v<=tolerance).then(||(vec![0.;self.width],vec![0.;bounds.len()]));
        }
        let (solution,reactions)=solver(&self.columns,bounds,tolerance)?;
        if self.ids.len()==self.width {return Some((solution,reactions));}
        let mut full=vec![0.;self.width];
        for (&i,&v) in self.ids.iter().zip(&solution) {full[i]=v;}
        Some((full,reactions))
    }
}
#[cfg(test)]
fn unilateral_nonzero_coordinates(columns:&[Vec<f64>],bounds:&[f64],tolerance:f64)->Option<(Vec<f64>,Vec<f64>)> {
    NonzeroCoordinates::new(columns)?.solve(bounds,tolerance)
}

pub(super) fn unilateral(columns:&[Vec<f64>],bounds:&[f64],tolerance:f64)->Option<(Vec<f64>,Vec<f64>)> {
    unilateral_ordered(columns,bounds,tolerance,true)
}
fn unilateral_appended(columns:&[Vec<f64>],bounds:&[f64],tolerance:f64)->Option<(Vec<f64>,Vec<f64>)> {
    unilateral_ordered(columns,bounds,tolerance,false)
}
fn unilateral_ordered(columns:&[Vec<f64>],bounds:&[f64],tolerance:f64,sort_active:bool)->Option<(Vec<f64>,Vec<f64>)> {
    unilateral_supported(columns,bounds,tolerance,sort_active,true)
}
fn unilateral_supported(columns:&[Vec<f64>],bounds:&[f64],tolerance:f64,sort_active:bool,sparse:bool)->Option<(Vec<f64>,Vec<f64>)> {
    unilateral_with_basis_support(columns,bounds,tolerance,sort_active,sparse,true)
}
fn unilateral_with_basis_support(columns:&[Vec<f64>],bounds:&[f64],tolerance:f64,sort_active:bool,sparse:bool,sparse_basis:bool)->Option<(Vec<f64>,Vec<f64>)> {
    let n = columns.len();
    let m = columns.first()?.len();
    if bounds.len() != n
        || columns
            .iter()
            .any(|c| c.len() != m || c.iter().any(|v| !v.is_finite()))
        || bounds.iter().any(|v| !v.is_finite())
        || !tolerance.is_finite()
        || tolerance <= 0.
    {
        return None;
    }
    // Exact structural zeros only. Keep product order and compensated/FMA
    // arithmetic for every nonzero term; never drop a small coefficient.
    // EqualityQr owns the same immutable support used by residuals and gaps;
    // avoid assembling a second identical map for every contact solve.
    let mut state = vec![0.; m];
    let mut multipliers = vec![0.; n];
    let mut active: Vec<usize> = Vec::new();
    let mut factor = EqualityQr::new(columns);
    factor.sparse_basis=sparse_basis;
    for iteration in 0..512 {
        if !active.is_empty() {
            let targets: Vec<_> = active.iter().map(|&i| bounds[i]).collect();
            let (candidate, reactions) = factor.solve(&active, &targets, tolerance)?;
            let release = active
                .iter()
                .zip(&reactions)
                .enumerate()
                .filter(|(_, (_, v))| **v < 0.)
                .map(|(k, (&i, &v))| (k, multipliers[i] / (multipliers[i] - v)))
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((k, fraction)) = release {
                if !fraction.is_finite() || !(0. ..=1.).contains(&fraction) {
                    return None;
                }
                for (value, &next) in state.iter_mut().zip(&candidate) {
                    *value += fraction * (next - *value);
                }
                for (&i, &next) in active.iter().zip(&reactions) {
                    multipliers[i] = (multipliers[i] + fraction * (next - multipliers[i])).max(0.);
                }
                multipliers[active[k]] = 0.;
                active.remove(k);
                continue;
            }
            state = candidate;
            multipliers.fill(0.);
            for (&i, &v) in active.iter().zip(&reactions) {
                multipliers[i] = v;
            }
        }
        // Reject nonfinite state explicitly: dropping 0*infinity must never
        // hide the NaN which the original dense gap computation rejected.
        if state.iter().any(|v|!v.is_finite()) {return None;}
        let gaps: Vec<_> = columns.iter().zip(bounds).enumerate().map(|(row,(c,b))|
            (if sparse {accurate_products(factor.supports[row].iter().map(|&i|(c[i],state[i])))}
                else {accurate_dot(c,&state)})-b).collect();
        if gaps.iter().any(|v| !v.is_finite()) {
            return None;
        }
        if (0..n).all(|i| {
            if multipliers[i] > 0. {
                gaps[i].abs() <= tolerance
            } else {
                gaps[i] >= -tolerance
            }
        }) {
            if std::env::var_os("VOXY_HAIR_QR_ITERATION_TRACE").is_some() {
                eprintln!("HAIR QR ITERATIONS sorted={sort_active} rows={n} width={m} iterations={} active={} rebuilt_columns={}",iteration+1,active.len(),factor.rebuilt_columns);
            }
            return Some((state, multipliers));
        }
        let enter = (0..n)
            .filter(|i| !active.contains(i) && gaps[*i] < -tolerance)
            .min_by(|&a, &b| gaps[a].total_cmp(&gaps[b]))?;
        active.push(enter);
        if sort_active {active.sort_unstable();}
    }
    None
}

// Traverse each basis column contiguously while retaining the original
// per-coordinate product order, Neumaier correction, and FMA product error.
// No zero or small coefficient is omitted.
fn accurate_column_combination(columns:&[Vec<f64>],weights:&[f64],width:usize)->Vec<f64> {
    let mut sums=vec![0f64;width];
    let mut corrections=vec![0f64;width];
    for (column,&weight) in columns.iter().zip(weights) {
        for ((sum,correction),&axis) in sums.iter_mut().zip(&mut corrections).zip(column) {
            let product=axis*weight;
            let next=*sum+product;
            *correction+=if sum.abs()>=product.abs() {(*sum-next)+product} else {(product-next)+*sum};
            *correction+=axis.mul_add(weight,-product);
            *sum=next;
        }
    }
    for (sum,correction) in sums.iter_mut().zip(corrections) {*sum+=correction;}
    sums
}

// A small clearance must survive cancellation between much larger terms.
// Neumaier summation retains addition error; FMA also retains product error.
pub(super) fn accurate_dot(a: &[f64], b: &[f64]) -> f64 {
    accurate_products(a.iter().copied().zip(b.iter().copied()))
}

pub(super) fn accurate_products(products: impl Iterator<Item = (f64, f64)>) -> f64 {
    let mut sum = 0f64;
    let mut correction = 0f64;
    for (x, y) in products {
        let product = x * y;
        let next = sum + product;
        correction += if sum.abs() >= product.abs() {
            (sum - next) + product
        } else {
            (product - next) + sum
        };
        correction += x.mul_add(y, -product);
        sum = next;
    }
    sum + correction
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn contiguous_coordinates_preserve_compensation_and_nonfinite_results() {
        let check=|columns:&[Vec<f64>],weights:&[f64]| {
            let width=columns[0].len();
            let actual=accurate_column_combination(columns,weights,width);
            for i in 0..width {
                let expected=accurate_products(columns.iter().zip(weights).map(|(c,&w)|(c[i],w)));
                if expected.is_nan() {assert!(actual[i].is_nan());}
                else {assert_eq!(actual[i].to_bits(),expected.to_bits());}
            }
        };
        check(&[vec![1.,f64::MIN_POSITIVE,0.,f64::MAX],
                vec![3e-15,0.,-0.,f64::MAX],vec![-1.,-f64::MIN_POSITIVE,0.,0.]],&[1.,1.,1.]);
        let mut seed=0x42f13579abcdefu64;
        for _ in 0..512 {
            let mut next=|| {
                seed=seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                let exponent=(seed>>52)%2047;
                f64::from_bits((seed&0x800fffffffffffff)|(exponent<<52))
            };
            let columns:Vec<Vec<_>>=(0..7).map(|_|(0..13).map(|_|next()).collect()).collect();
            let weights:Vec<_>=(0..7).map(|_|next()).collect();
            check(&columns,&weights);
        }
    }
    #[test]
    fn exact_zero_support_preserves_cancellation_subnormals_and_overflow_rejection() {
        let cases=[
            (vec![1.,0.,1.,-0.,1.],vec![1.,f64::MAX,3e-15,-f64::MAX,-1.]),
            (vec![f64::MIN_POSITIVE,0.,-f64::MIN_POSITIVE],vec![f64::EPSILON,f64::MAX,f64::EPSILON]),
            (vec![f64::MAX,0.,f64::MAX],vec![1.,-f64::MAX,1.]),
            (vec![0.,-0.,0.],vec![-1.,-f64::MAX,f64::MAX]),
        ];
        let check=|column:&[f64],state:&[f64]| {
            let dense=accurate_dot(column,state);
            let sparse=accurate_products(column.iter().zip(state).filter(|(c,_)|**c!=0.).map(|(&c,&v)|(c,v)));
            if dense.is_nan() {assert!(sparse.is_nan());} else {assert_eq!(dense.to_bits(),sparse.to_bits());}
        };
        for (column,state) in cases {check(&column,&state);}
        let mut seed=0x123456789abcdefu64;
        for _ in 0..1024 {
            let mut next=|| {seed=seed.wrapping_mul(6364136223846793005).wrapping_add(1);seed};
            let mut column=Vec::new();let mut state=Vec::new();
            for i in 0..64 {
                let bits=next();
                let exponent=(bits>>52)%2047; // finite, including subnormals
                let value=f64::from_bits((bits&0x800fffffffffffff)|(exponent<<52));
                column.push(if i%3==0 {if i%2==0 {0.} else {-0.}} else {value});
                let bits=next();let exponent=(bits>>52)%2047;
                state.push(f64::from_bits((bits&0x800fffffffffffff)|(exponent<<52)));
            }
            check(&column,&state);
        }
    }
    #[test]
    fn exact_sparse_gaps_preserve_dense_solutions_and_tiny_coefficients() {
        let columns=vec![vec![1.,0.,1e-200,0.],vec![0.,1.,0.,1e-10],vec![1.,-1.,0.,0.]];
        for bounds in [[1.,2.,-3.],[0.,0.,0.],[-1.,-1.,-1.],[1.,1.,0.1]] {
            for sorted in [false,true] {
                let dense=unilateral_supported(&columns,&bounds,1e-12,sorted,false);
                let sparse=unilateral_supported(&columns,&bounds,1e-12,sorted,true);
                let bits=|result:Option<(Vec<f64>,Vec<f64>)>|result.map(|(x,y)|x.into_iter().chain(y).map(f64::to_bits).collect::<Vec<_>>());
                assert_eq!(bits(sparse),bits(dense));
            }
        }
    }
    #[test]
    fn immutable_coordinate_map_reuses_loads_with_changed_bounds() {
        let columns=vec![vec![0.,1.,0.,1e-10],vec![0.,1.,0.,-1e-10]];
        let prepared=NonzeroCoordinates::new(&columns).unwrap();
        for bounds in [[1.,-3.],[-1.,-2.],[0.1,-0.5],[1.,1.]] {
            assert_eq!(prepared.solve(&bounds,1e-12),uncached_unilateral_reference(&columns,&bounds,1e-12));
        }
        let full=vec![vec![1.,1e-10]];
        assert!(matches!(NonzeroCoordinates::new(&full).unwrap().columns,std::borrow::Cow::Borrowed(_)));
    }

    #[test]
    fn exact_zero_reduction_preserves_tiny_directions_and_zero_load_feasibility() {
        let columns=vec![vec![0.,1.,0.,1e-10,0.],vec![0.,1.,0.,-1e-10,0.]];
        let bounds=[1.,-3.];
        let actual=unilateral_nonzero_coordinates(&columns,&bounds,1e-12).unwrap();
        let original=unilateral(&columns,&bounds,1e-12).unwrap();
        assert_eq!(actual,original);
        assert!(actual.0[3]!=0.,"tiny direction must not be thresholded away");
        assert_eq!(actual.0[0],0.);assert_eq!(actual.0[2],0.);assert_eq!(actual.0[4],0.);
        assert_eq!(unilateral_nonzero_coordinates(&[vec![0.;5]],&[-1.],1e-12),Some((vec![0.;5],vec![0.])));
        assert!(unilateral_nonzero_coordinates(&[vec![0.;5]],&[1.],1e-12).is_none());
    }

    #[test]
    #[ignore = "requires a captured VQC1 operator and original physical loads"]
    fn captured_guarded_append_order_retains_original_physical_admission() {
        let data=std::fs::read(std::env::var("VOXY_HAIR_QR_INPUT_FIXTURE").unwrap()).unwrap();
        assert_eq!(&data[..4],b"VQC1");
        let rows=u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
        let width=u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;
        let mut offset=20;
        let mut scalar=|| {let value=f64::from_le_bytes(data[offset..offset+8].try_into().unwrap());offset+=8;value};
        let tolerance=scalar();let original_bounds:Vec<_>=(0..rows).map(|_|scalar()).collect();
        let bounds:Vec<_>=(0..rows).map(|_|scalar()).collect();
        let columns:Vec<Vec<_>>=(0..rows).map(|_|(0..width).map(|_|scalar()).collect()).collect();
        use std::io::Read;
        let mut reader=std::io::Cursor::new(data[offset..].to_vec());
        fn integer(r:&mut std::io::Cursor<Vec<u8>>)->usize {let mut b=[0;4];r.read_exact(&mut b).unwrap();u32::from_le_bytes(b) as usize}
        fn value(r:&mut std::io::Cursor<Vec<u8>>)->f64 {let mut b=[0;8];r.read_exact(&mut b).unwrap();f64::from_le_bytes(b)}
        let systems=u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
        let mut requests=Vec::new();
        for _ in 0..systems {
            let n=integer(&mut reader);let band=integer(&mut reader);let lo=integer(&mut reader);let hi=integer(&mut reader);
            let matrix=(0..n*band).map(|_|value(&mut reader)).collect();
            let rhs=(0..n).map(|_|value(&mut reader)).collect();
            let loads=(0..rows).map(|_|(0..n).map(|_|value(&mut reader)).collect()).collect();
            requests.push(super::super::HairResponseSystem {system:super::super::HairLinearSystem {band_width:band,matrix,rhs,active:lo..hi},loads});
        }
        assert_eq!(reader.position() as usize,reader.get_ref().len());
        let reduced=NonzeroCoordinates::new(&columns).unwrap();
        let mut times=Vec::new();
        for repeat in 0..7 {
            let run=|append| {
                let start=std::time::Instant::now();
                let solution=if append {unilateral_appended(&reduced.columns,&bounds,tolerance)}
                    else {unilateral(&reduced.columns,&bounds,tolerance)}.unwrap();
                (solution,start.elapsed().as_secs_f64())
            };
            let (old,new)=if repeat%2==0 {(run(false),run(true))} else {let new=run(true);(run(false),new)};
            for (solution,_) in [&old,&new] {
                for i in 0..rows {
                    let gap=accurate_dot(&reduced.columns[i],&solution.0)-bounds[i];
                    let reaction=solution.1[i];assert!(reaction>=0.);
                    assert!(if reaction>0. {gap.abs()<=tolerance} else {gap>=-tolerance});
                }
            }
            for append in [false,true] {
                let (responses,reactions)=if append {
                    super::super::HairResponseSystem::solve_joint_load_inequalities_native(&requests,&original_bounds,tolerance)
                } else {
                    super::super::HairResponseSystem::solve_joint_load_inequalities_with_coordinates(
                        &requests,&original_bounds,tolerance,false,|prepared,bounds,tolerance|prepared.solve(bounds,tolerance))
                }.unwrap_or_else(|error|panic!("canonical physical owner guarded={append}: {error}"));
                for i in 0..rows {
                    let gap=accurate_products(requests.iter().zip(&responses).flat_map(|(r,x)|r.loads[i].iter().copied().zip(x.iter().copied())))-original_bounds[i];
                    assert!(reactions[i]>=0.);
                    assert!(if reactions[i]>0. {gap.abs()<=tolerance} else {gap>=-tolerance},"original physical row {i}, gap={gap}");
                }
            }
            let max_difference=old.0.0.iter().zip(&new.0.0).map(|(a,b)|(a-b).abs()).fold(0.,f64::max);
            times.push((old.1,new.1,max_difference));
        }
        eprintln!("append experiment rows={rows} compact_width={} sorted/append seconds and max coordinate difference={times:?}",reduced.coordinate_count());
    }
    // Experiment only: keep each coordinate's exact product order while
    // traversing source columns contiguously. Includes accumulator allocation.
    #[test]
    #[ignore = "requires a captured VQC1 input; does not change production arithmetic"]
    fn captured_coordinate_reconstruction_layout_experiment() {
        let data=std::fs::read(std::env::var("VOXY_HAIR_QR_INPUT_FIXTURE").unwrap()).unwrap();
        assert_eq!(&data[..4],b"VQC1");
        let rows=u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
        let width=u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;
        let mut offset=20;
        let mut scalar=|| {let v=f64::from_le_bytes(data[offset..offset+8].try_into().unwrap());offset+=8;v};
        scalar();for _ in 0..rows {scalar();}
        let weights:Vec<_>=(0..rows).map(|_|scalar()).collect();
        let columns:Vec<Vec<_>>=(0..rows).map(|_|(0..width).map(|_|scalar()).collect()).collect();
        let mut timings=Vec::new();
        for repeat in 0..7 {
            let run=|streaming| {
                let began=std::time::Instant::now();let mut last=Vec::new();
                for _ in 0..100 {
                    let columns=std::hint::black_box(&columns);
                    last=if streaming {
                        accurate_column_combination(columns,&weights,width)
                    } else {(0..width).map(|i|accurate_products(columns.iter().zip(&weights).map(|(c,&w)|(c[i],w)))).collect()};
                    std::hint::black_box(&last);
                }
                (last,began.elapsed().as_secs_f64())
            };
            let (original,streaming)=if repeat%2==0 {(run(false),run(true))} else {let new=run(true);(run(false),new)};
            assert_eq!(original.0.iter().map(|v|v.to_bits()).collect::<Vec<_>>(),streaming.0.iter().map(|v|v.to_bits()).collect::<Vec<_>>());
            timings.push((original.1,streaming.1));
        }
        eprintln!("coordinate reconstruction rows={rows} width={width} 100 evaluations original/streaming seconds={timings:?}");
    }
    #[test]
    #[ignore = "paired timing requires a captured VQC1 physical contact input"]
    fn captured_prefix_reuse_matches_reference_and_reports_paired_time() {
        let data = std::fs::read(std::env::var("VOXY_HAIR_QR_INPUT_FIXTURE").unwrap()).unwrap();
        assert_eq!(&data[..4], b"VQC1");
        let rows = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
        let width = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;
        let mut offset = 20;
        let mut scalar = || { let v = f64::from_le_bytes(data[offset..offset+8].try_into().unwrap()); offset += 8; v };
        let tolerance = scalar();
        for _ in 0..rows { scalar(); }
        let bounds: Vec<_> = (0..rows).map(|_| scalar()).collect();
        let columns: Vec<Vec<_>> = (0..rows).map(|_| (0..width).map(|_| scalar()).collect()).collect();
        let mut timings = Vec::new();
        for repeat in 0..7 {
            let run = |cached| {
                let start = std::time::Instant::now();
                let result = if cached { unilateral_nonzero_coordinates(&columns, &bounds, tolerance) }
                    else { uncached_unilateral_reference(&columns, &bounds, tolerance) }.unwrap();
                (result, start.elapsed().as_secs_f64())
            };
            let (cached, original) = if repeat % 2 == 0 { (run(true), run(false)) }
                else { let old = run(false); (run(true), old) };
            assert_eq!(cached.0.0.iter().chain(&cached.0.1).map(|v| v.to_bits()).collect::<Vec<_>>(),
                original.0.0.iter().chain(&original.0.1).map(|v| v.to_bits()).collect::<Vec<_>>());
            timings.push((original.1, cached.1));
        }
        eprintln!("captured QR rows={rows} width={width}, paired original/cached seconds={timings:?}");
        let reduced=NonzeroCoordinates::new(&columns).unwrap();
        let mut sparse_timings=Vec::new();
        for repeat in 0..7 {
            let run=|sparse| {let began=std::time::Instant::now();
                let result=unilateral_supported(&reduced.columns,&bounds,tolerance,false,sparse).unwrap();
                (result,began.elapsed().as_secs_f64())};
            let (dense,sparse)=if repeat%2==0 {(run(false),run(true))} else {let sparse=run(true);(run(false),sparse)};
            assert_eq!(dense.0.0.iter().chain(&dense.0.1).map(|v|v.to_bits()).collect::<Vec<_>>(),sparse.0.0.iter().chain(&sparse.0.1).map(|v|v.to_bits()).collect::<Vec<_>>());
            sparse_timings.push((dense.1,sparse.1));
        }
        eprintln!("captured exact gap dense/sparse seconds={sparse_timings:?}");
        let mut basis_timings=Vec::new();
        for repeat in 0..7 {
            let run=|sparse_basis| {let began=std::time::Instant::now();
                let result=unilateral_with_basis_support(&reduced.columns,&bounds,tolerance,false,true,sparse_basis).unwrap();
                (result,began.elapsed().as_secs_f64())};
            let (dense,sparse)=if repeat%2==0 {(run(false),run(true))} else {let new=run(true);(run(false),new)};
            assert_eq!(dense.0.0.iter().chain(&dense.0.1).map(|v|v.to_bits()).collect::<Vec<_>>(),sparse.0.0.iter().chain(&sparse.0.1).map(|v|v.to_bits()).collect::<Vec<_>>());
            basis_timings.push((dense.1,sparse.1));
        }
        eprintln!("captured orthogonalization dense/sparse basis seconds={basis_timings:?}");

        let qr=EqualityQr::new(&reduced.columns);
        let active:Vec<_>=(0..rows).collect();
        let state=unilateral_supported(&reduced.columns,&bounds,tolerance,false,true).unwrap().0;
        let mut residual_timings=Vec::new();
        for repeat in 0..7 {
            let run=|sparse| {
                let began=std::time::Instant::now();let mut last=Vec::new();
                for _ in 0..100 {
                    let state=std::hint::black_box(&state);
                    last=if sparse {qr.residuals(&active,&bounds,state).unwrap()} else {
                        assert!(state.iter().all(|v|v.is_finite()));
                        active.iter().zip(&bounds).map(|(&id,b)|b-accurate_dot(&reduced.columns[id],state)).collect()
                    };
                    std::hint::black_box(&last);
                }
                (last,began.elapsed().as_secs_f64())
            };
            let (dense,sparse)=if repeat%2==0 {(run(false),run(true))} else {let sparse=run(true);(run(false),sparse)};
            assert_eq!(dense.0.iter().map(|v|v.to_bits()).collect::<Vec<_>>(),sparse.0.iter().map(|v|v.to_bits()).collect::<Vec<_>>());
            residual_timings.push((dense.1,sparse.1));
        }
        eprintln!("captured QR residual 100 evaluations dense/sparse seconds={residual_timings:?}");
    }

    #[test]
    fn immutable_column_validation_rejects_only_selected_invalid_identities() {
        let columns=vec![vec![1.,0.],vec![f64::NAN,0.],vec![1.],vec![0.,1.]];
        let mut qr=EqualityQr::new(&columns);
        assert_eq!(qr.solve(&[0],&[2.],1e-12),uncached_equality_reference(&[columns[0].clone()],&[2.],1e-12));
        for id in [1,2,4] {assert!(qr.solve(&[id],&[1.],1e-12).is_none());}
        assert_eq!(qr.solve(&[0,3],&[2.,3.],1e-12),uncached_equality_reference(&[columns[0].clone(),columns[3].clone()],&[2.,3.],1e-12));
    }
    #[test]
    fn active_prefix_reuse_matches_uncached_arithmetic_bitwise() {
        let columns = vec![vec![1., 1e-10, 0., 0.], vec![1., -1e-10, 0., 0.],
            vec![0., 0., 1., 0.2], vec![0., 0., 0.1, 1.]];
        for sparse_basis in [false,true] {
        let mut factor = EqualityQr::new(&columns);
        factor.sparse_basis=sparse_basis;
        for (step, active) in [vec![0], vec![0,2], vec![0,1,2], vec![0,2],
            vec![0,2,3], vec![2,3], vec![0,1,2,3], vec![0]].iter().enumerate() {
            let bounds: Vec<_> = active.iter().map(|&i| (i + step + 1) as f64 * 0.001).collect();
            let selected: Vec<_> = active.iter().map(|&i| columns[i].clone()).collect();
            let actual = factor.solve(active, &bounds, 1e-10).unwrap();
            let expected = uncached_equality_reference(&selected, &bounds, 1e-10).unwrap();
            assert_eq!(actual.0.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                expected.0.iter().map(|v| v.to_bits()).collect::<Vec<_>>());
            assert_eq!(actual.1.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                expected.1.iter().map(|v| v.to_bits()).collect::<Vec<_>>());
        }
        }
    }

    #[test]
    fn contact_clearance_survives_cancellation_across_response_systems() {
        // Separate systems contribute large opposite terms around a tiny gap.
        let products = [(1., 1.), (1., 3e-15), (1., -1.)];
        assert_eq!(accurate_products(products.into_iter()), 3e-15);
        assert_eq!(accurate_dot(&[1., 1., 1.], &[1., 3e-15, -1.]), 3e-15);
    }
    #[test]
    #[ignore = "requires a captured VQC1 contact input"]
    fn captured_active_contacts_retain_original_clearance() {
        use std::io::Read;
        let path = std::env::var("VOXY_HAIR_QR_INPUT_FIXTURE").unwrap();
        let mut input = std::io::Cursor::new(std::fs::read(path).unwrap());
        let mut magic = [0; 4]; input.read_exact(&mut magic).unwrap();
        assert_eq!(&magic, b"VQC1");
        fn integer(input: &mut std::io::Cursor<Vec<u8>>) -> usize {
            let mut b = [0; 4]; input.read_exact(&mut b).unwrap();
            u32::from_le_bytes(b) as usize
        }
        fn scalar(input: &mut std::io::Cursor<Vec<u8>>) -> f64 {
            let mut b = [0; 8]; input.read_exact(&mut b).unwrap();
            f64::from_le_bytes(b)
        }
        let rows = integer(&mut input); let width = integer(&mut input);
        let systems = integer(&mut input); let _refinement = integer(&mut input);
        let tolerance = scalar(&mut input);
        assert!(rows <= 512 && width <= 65536);
        let original_bounds: Vec<_> = (0..rows).map(|_| scalar(&mut input)).collect();
        let bounds: Vec<_> = (0..rows).map(|_| scalar(&mut input)).collect();
        let columns: Vec<Vec<_>> = (0..rows).map(|_| (0..width)
            .map(|_| scalar(&mut input)).collect()).collect();
        let (state, reactions) = unilateral(&columns, &bounds, tolerance).unwrap();
        for i in 0..rows {
            let gap = accurate_dot(&columns[i], &state) - bounds[i];
            assert!(reactions[i] >= 0.);
            assert!(if reactions[i] > 0. { gap.abs() <= tolerance } else { gap >= -tolerance });
        }
        // The whitened point alone is insufficient: replay immutable original
        // loads through the canonical factors and physical admission too.
        let mut requests = Vec::new();
        for _ in 0..systems {
            let n = integer(&mut input); let band = integer(&mut input);
            let lo = integer(&mut input); let hi = integer(&mut input);
            assert!(n <= 65536 && band == super::super::direct::BAND && lo <= hi && hi <= n);
            let matrix = (0..n*band).map(|_| scalar(&mut input)).collect();
            let rhs = (0..n).map(|_| scalar(&mut input)).collect();
            let loads = (0..rows).map(|_| (0..n).map(|_| scalar(&mut input)).collect()).collect();
            requests.push(super::super::HairResponseSystem {
                system: super::super::HairLinearSystem { band_width: band, matrix, rhs, active: lo..hi },
                loads,
            });
        }
        assert_eq!(input.position() as usize, input.get_ref().len());
        let (responses, reactions) = super::super::HairResponseSystem::solve_joint_load_inequalities_native(
            &requests, &original_bounds, tolerance).expect("original physical load admission");
        for i in 0..rows {
            let gap = accurate_products(requests.iter().zip(&responses).flat_map(|(r,x)|
                r.loads[i].iter().copied().zip(x.iter().copied()))) - original_bounds[i];
            assert!(reactions[i] >= 0.);
            assert!(if reactions[i] > 0. { gap.abs() <= tolerance } else { gap >= -tolerance });
        }
        if let Some(path) = std::env::var_os("VOXY_HAIR_QR_PHYSICAL_AUDIT_EXPORT") {
            use std::io::Write;
            let mut out = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
            out.write_all(b"VQA1").unwrap();
            for value in [rows, systems] {out.write_all(&(value as u32).to_le_bytes()).unwrap();}
            out.write_all(&tolerance.to_le_bytes()).unwrap();
            for value in original_bounds.iter().chain(&reactions) {out.write_all(&value.to_le_bytes()).unwrap();}
            for (request, response) in requests.iter().zip(&responses) {
                out.write_all(&(response.len() as u32).to_le_bytes()).unwrap();
                for value in response.iter().chain(request.loads.iter().flatten()) {out.write_all(&value.to_le_bytes()).unwrap();}
            }
            out.flush().unwrap();
        }
    }

    #[test]
    fn opening_contacts_release_and_coupled_closing_contacts_enter() {
        let columns = vec![vec![1., 0.], vec![-0.5, 3f64.sqrt() * 0.5]];
        let (state, reactions) = unilateral(&columns, &[-1., -1.], 1e-14).unwrap();
        assert_eq!(state, vec![0.; 2]);
        assert_eq!(reactions, vec![0.; 2]);
        let (state, reactions) = unilateral(&columns, &[1., 0.], 1e-14).unwrap();
        assert!((state[0] - 1.).abs() < 1e-14);
        assert!((state[1] - 1. / 3f64.sqrt()).abs() < 1e-14);
        assert!((reactions[0] - 4. / 3.).abs() < 1e-14);
        assert!((reactions[1] - 2. / 3.).abs() < 1e-14);
    }
    #[test]
    fn near_parallel_constraint_keeps_one_clearance_budget() {
        let columns = vec![vec![1., 0.], vec![1., 1e-10]];
        let (state, reactions) = unilateral(&columns, &[1., 1. - 1e-6], 1e-14).unwrap();
        assert_eq!(state, vec![1., 0.]);
        assert_eq!(reactions, vec![1., 0.]);
        assert!(
            columns[1]
                .iter()
                .zip(&state)
                .map(|(a, b)| a * b)
                .sum::<f64>()
                >= 1. - 1e-6
        );
        assert!(unilateral(&columns, &[1., f64::NAN], 1e-14).is_none());
    }
    #[test]
    fn two_active_constraints_retain_the_direction_missing_from_gram() {
        let columns = vec![vec![1., 0.], vec![-1., 1e-10]];
        let norm = columns[1].iter().map(|v| v * v).sum::<f64>();
        assert_eq!(
            norm - 1.,
            0.,
            "rounded Gram has lost the independent direction"
        );
        let (state, reactions) = unilateral(&columns, &[1., 1.], 1e-14).unwrap();
        assert_eq!(state, vec![1., 2e10]);
        assert!(reactions.iter().all(|v| v.is_finite() && *v > 0.));
        for column in &columns {
            assert!(
                (column.iter().zip(&state).map(|(a, b)| a * b).sum::<f64>() - 1.).abs() <= 1e-14
            );
        }
    }
}

#[cfg(test)]
fn uncached_equality_reference(columns: &[Vec<f64>], bounds: &[f64], tolerance: f64) -> Option<(Vec<f64>, Vec<f64>)> {
    let n = columns.len();
    let m = columns.first()?.len();
    if bounds.len() != n
        || n > m
        || columns
            .iter()
            .any(|c| c.len() != m || c.iter().any(|v| !v.is_finite()))
        || bounds.iter().any(|v| !v.is_finite())
        || !tolerance.is_finite() || tolerance <= 0.
    {
        return None;
    }
    let mut q: Vec<Vec<f64>> = Vec::with_capacity(n);
    let mut r = vec![0.; n * n];
    for (k, column) in columns.iter().enumerate() {
        let mut v = column.clone();
        // Twice-orthogonalized modified Gram-Schmidt retains the original
        // column direction rather than subtracting squared dot products.
        for _ in 0..2 {
            for (j, basis) in q.iter().enumerate() {
                let projection = accurate_dot(basis, &v);
                r[j * n + k] += projection;
                for (value, &axis) in v.iter_mut().zip(basis) {
                    *value = (-projection).mul_add(axis, *value);
                }
            }
        }
        let scale = v.iter().map(|x| x.abs()).fold(0., f64::max);
        if scale == 0. || !scale.is_finite() {
            return None;
        }
        let norm = scale
            * v.iter()
                .map(|x| (x / scale) * (x / scale))
                .sum::<f64>()
                .sqrt();
        if norm == 0. || !norm.is_finite() {
            return None;
        }
        r[k * n + k] = norm;
        for value in &mut v {
            *value /= norm;
        }
        q.push(v);
    }
    // W=Q*R, so W^T*y=b gives R^T*z=b and y=Q*z.
    let mut z = bounds.to_vec();
    for i in 0..n {
        z[i] -= accurate_products((0..i).map(|j| (r[j * n + i], z[j])));
        z[i] /= r[i * n + i];
    }
    let mut result: Vec<_> = (0..m).map(|i|
        accurate_products(q.iter().zip(&z).map(|(column, &value)| (column[i], value)))
    ).collect();
    // y=W*lambda=Q*R*lambda, so the same factor gives R*lambda=z.
    for i in (0..n).rev() {
        z[i] -= accurate_products((i + 1..n).map(|j| (r[i * n + j], z[j])));
        z[i] /= r[i * n + i];
    }
    // Correct rounding in Q*z against the original columns using the same
    // factor. Keep reactions consistent with every coordinate correction.
    for _ in 0..8 {
        let mut delta: Vec<f64> = columns.iter().zip(bounds)
            .map(|(column, bound)| bound - accurate_dot(column, &result)).collect();
        // Stop at the caller's unchanged admission tolerance. Continuing
        // after admission can oscillate between neighboring rounded points.
        if delta.iter().all(|v| v.abs() <= tolerance) { break; }
        if delta.iter().any(|v| !v.is_finite()) { return None; }
        for i in 0..n {
            for j in 0..i { delta[i] -= r[j * n + i] * delta[j]; }
            delta[i] /= r[i * n + i];
        }
        for (column, value) in q.iter().zip(&delta) {
            for (out, axis) in result.iter_mut().zip(column) { *out += axis * value; }
        }
        for i in (0..n).rev() {
            for j in i + 1..n { delta[i] -= r[i * n + j] * delta[j]; }
            delta[i] /= r[i * n + i];
        }
        for (value, correction) in z.iter_mut().zip(delta) { *value += correction; }
    }
    (result.iter().chain(&z).all(|v| v.is_finite())).then_some((result, z))
}

#[cfg(test)]
fn uncached_unilateral_reference(
    columns: &[Vec<f64>],
    bounds: &[f64],
    tolerance: f64,
) -> Option<(Vec<f64>, Vec<f64>)> {
    let n = columns.len();
    let m = columns.first()?.len();
    if bounds.len() != n
        || columns
            .iter()
            .any(|c| c.len() != m || c.iter().any(|v| !v.is_finite()))
        || bounds.iter().any(|v| !v.is_finite())
        || !tolerance.is_finite()
        || tolerance <= 0.
    {
        return None;
    }
    let mut state = vec![0.; m];
    let mut multipliers = vec![0.; n];
    let mut active: Vec<usize> = Vec::new();
    for _ in 0..512 {
        if !active.is_empty() {
            let targets: Vec<_> = active.iter().map(|&i| bounds[i]).collect();
            let (candidate, reactions) = uncached_equality_reference(&active.iter().map(|&i| columns[i].clone()).collect::<Vec<_>>(), &targets, tolerance)?;
            let release = active
                .iter()
                .zip(&reactions)
                .enumerate()
                .filter(|(_, (_, v))| **v < 0.)
                .map(|(k, (&i, &v))| (k, multipliers[i] / (multipliers[i] - v)))
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((k, fraction)) = release {
                if !fraction.is_finite() || !(0. ..=1.).contains(&fraction) {
                    return None;
                }
                for (value, &next) in state.iter_mut().zip(&candidate) {
                    *value += fraction * (next - *value);
                }
                for (&i, &next) in active.iter().zip(&reactions) {
                    multipliers[i] = (multipliers[i] + fraction * (next - multipliers[i])).max(0.);
                }
                multipliers[active[k]] = 0.;
                active.remove(k);
                continue;
            }
            state = candidate;
            multipliers.fill(0.);
            for (&i, &v) in active.iter().zip(&reactions) {
                multipliers[i] = v;
            }
        }
        let gaps: Vec<_> = columns
            .iter()
            .zip(bounds)
            .map(|(c, b)| accurate_dot(c, &state) - b)
            .collect();
        if gaps.iter().any(|v| !v.is_finite()) {
            return None;
        }
        if (0..n).all(|i| {
            if multipliers[i] > 0. {
                gaps[i].abs() <= tolerance
            } else {
                gaps[i] >= -tolerance
            }
        }) {
            return Some((state, multipliers));
        }
        let enter = (0..n)
            .filter(|i| !active.contains(i) && gaps[*i] < -tolerance)
            .min_by(|&a, &b| gaps[a].total_cmp(&gaps[b]))?;
        active.push(enter);
        active.sort_unstable();
    }
    None
}
