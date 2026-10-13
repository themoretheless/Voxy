#![allow(dead_code)]
mod baseline {
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
    q_spares: Vec<Vec<f64>>,
    r_spares: Vec<Vec<f64>>,
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
        Self { columns, valid_columns, supports, active: Vec::new(), q: Vec::new(), q_spares:Vec::new(), r_spares:Vec::new(), q_supports:Vec::new(), sparse_basis:false, r: Vec::new(), rebuilt_columns: 0 }
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
        self.active.truncate(prefix);
        self.q_spares.extend(self.q.drain(prefix..));
        self.r_spares.extend(self.r.drain(prefix..));
        self.q_supports.truncate(prefix);
        for &id in &active[prefix..] {
            self.rebuilt_columns += 1;
            let k = self.q.len();
            // Recycle discarded suffix storage within this immutable operator.
            // Copy every original coefficient and clear the entire new R row;
            // no previous numerical value participates in the rebuilt factor.
            let mut v=self.q_spares.pop().unwrap_or_default();
            v.clone_from(&self.columns[id]);
            let mut r=self.r_spares.pop().unwrap_or_default();
            r.resize(k+1,0.);r.fill(0.);
            for _ in 0..2 {
                for (j, basis) in self.q.iter().enumerate() {
                    let projection = if self.sparse_basis {
                        accurate_products(self.q_supports[j].iter().map(|&i|(basis[i],v[i])))
                    } else {accurate_dot(basis, &v)};
                    r[j] += projection;
                    for (value, &axis) in v.iter_mut().zip(basis) {
                        *value = (-projection).mul_add(axis, *value);
                    }
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

fn accurate_column_combination(columns:&[Vec<f64>],weights:&[f64],width:usize)->Vec<f64> {
    accurate_column_combination_impl(columns,weights,width,None)
}

// Support belongs to the same immutable columns. Nonfinite weights require
// dense evaluation: omitting 0 * infinity would otherwise hide invalid input.
fn accurate_supported_column_combination(columns:&[Vec<f64>],weights:&[f64],width:usize,supports:&[Vec<usize>])->Vec<f64> {
    let support=weights.iter().all(|w|w.is_finite()).then_some(supports);
    accurate_column_combination_impl(columns,weights,width,support)
}

fn accurate_column_combination_impl(columns:&[Vec<f64>],weights:&[f64],width:usize,supports:Option<&[Vec<usize>]>)->Vec<f64> {
    let mut sums=vec![0f64;width];
    let mut corrections=vec![0f64;width];
    for (column_index,(column,&weight)) in columns.iter().zip(weights).enumerate() {
        let mut accumulate=|i:usize,axis:f64| {
            let sum=&mut sums[i];
            let correction=&mut corrections[i];
            let product=axis*weight;
            let next=*sum+product;
            *correction+=if sum.abs()>=product.abs() {(*sum-next)+product} else {(product-next)+*sum};
            *correction+=axis.mul_add(weight,-product);
            *sum=next;
        };
        if let Some(supports)=supports {
            for &i in &supports[column_index] {accumulate(i,column[i]);}
        } else {
            for (i,&axis) in column.iter().take(width).enumerate() {accumulate(i,axis);}
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

pub fn run(c:&[Vec<f64>],b:&[f64],tol:f64)->Vec<Option<Vec<u64>>> { let mut q=EqualityQr::new(c);q.sparse_basis=true; (1..=c.len().min(20)).map(|n| q.solve(&(0..n).collect::<Vec<_>>(),&b[..n],tol).map(|(x,y)|x.into_iter().chain(y).map(f64::to_bits).collect())).collect() }
}
mod candidate {
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
    q_spares: Vec<Vec<f64>>,
    r_spares: Vec<Vec<f64>>,
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
        Self { columns, valid_columns, supports, active: Vec::new(), q: Vec::new(), q_spares:Vec::new(), r_spares:Vec::new(), q_supports:Vec::new(), sparse_basis:false, r: Vec::new(), rebuilt_columns: 0 }
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
        self.active.truncate(prefix);
        self.q_spares.extend(self.q.drain(prefix..));
        self.r_spares.extend(self.r.drain(prefix..));
        self.q_supports.truncate(prefix);
        for &id in &active[prefix..] {
            self.rebuilt_columns += 1;
            let k = self.q.len();
            // Recycle discarded suffix storage within this immutable operator.
            // Copy every original coefficient and clear the entire new R row;
            // no previous numerical value participates in the rebuilt factor.
            let mut v=self.q_spares.pop().unwrap_or_default();
            v.clone_from(&self.columns[id]);
            let mut r=self.r_spares.pop().unwrap_or_default();
            r.resize(k+1,0.);r.fill(0.);
            for _ in 0..2 {
                for (j, basis) in self.q.iter().enumerate() {
                    let projection = if self.sparse_basis {
                        accurate_products(self.q_supports[j].iter().map(|&i|(basis[i],v[i])))
                    } else {accurate_dot(basis, &v)};
                    r[j] += projection;
                    if self.sparse_basis {
                        // Validate while writing: avoid a second traversal of
                        // the same coordinate vector after every basis update.
                        // Keep every FMA, including exact zero coefficients.
                        let mut finite = true;
                        for (value, &axis) in v.iter_mut().zip(basis) {
                            *value = (-projection).mul_add(axis, *value);
                            finite &= value.is_finite();
                        }
                        if !finite { return None; }
                    } else {
                        for (value, &axis) in v.iter_mut().zip(basis) {
                            *value = (-projection).mul_add(axis, *value);
                        }
                    }
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

fn accurate_column_combination(columns:&[Vec<f64>],weights:&[f64],width:usize)->Vec<f64> {
    accurate_column_combination_impl(columns,weights,width,None)
}

// Support belongs to the same immutable columns. Nonfinite weights require
// dense evaluation: omitting 0 * infinity would otherwise hide invalid input.
fn accurate_supported_column_combination(columns:&[Vec<f64>],weights:&[f64],width:usize,supports:&[Vec<usize>])->Vec<f64> {
    let support=weights.iter().all(|w|w.is_finite()).then_some(supports);
    accurate_column_combination_impl(columns,weights,width,support)
}

fn accurate_column_combination_impl(columns:&[Vec<f64>],weights:&[f64],width:usize,supports:Option<&[Vec<usize>]>)->Vec<f64> {
    let mut sums=vec![0f64;width];
    let mut corrections=vec![0f64;width];
    for (column_index,(column,&weight)) in columns.iter().zip(weights).enumerate() {
        let mut accumulate=|i:usize,axis:f64| {
            let sum=&mut sums[i];
            let correction=&mut corrections[i];
            let product=axis*weight;
            let next=*sum+product;
            *correction+=if sum.abs()>=product.abs() {(*sum-next)+product} else {(product-next)+*sum};
            *correction+=axis.mul_add(weight,-product);
            *sum=next;
        };
        if let Some(supports)=supports {
            for &i in &supports[column_index] {accumulate(i,column[i]);}
        } else {
            for (i,&axis) in column.iter().take(width).enumerate() {accumulate(i,axis);}
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

pub fn run(c:&[Vec<f64>],b:&[f64],tol:f64)->Vec<Option<Vec<u64>>> { let mut q=EqualityQr::new(c);q.sparse_basis=true; (1..=c.len().min(20)).map(|n| q.solve(&(0..n).collect::<Vec<_>>(),&b[..n],tol).map(|(x,y)|x.into_iter().chain(y).map(f64::to_bits).collect())).collect() }
}
fn main(){let bytes=std::fs::read(std::env::args().nth(1).unwrap()).unwrap();assert_eq!(&bytes[..4],b"VQC1");let n=u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;let m=u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;let mut k=20;let mut f=||{let x=f64::from_le_bytes(bytes[k..k+8].try_into().unwrap());k+=8;x};let tol=f();for _ in 0..n{f();}let b:Vec<_>=(0..n).map(|_|f()).collect();let c:Vec<Vec<_>>=(0..n).map(|_|(0..m).map(|_|f()).collect()).collect();let old=baseline::run(&c,&b,tol);assert_eq!(old,candidate::run(&c,&b,tol));println!("bitwise matched cases={} successful={} rows={n} width={m}",old.len(),old.iter().filter(|x|x.is_some()).count());for repeat in 0..7{let mut times=[0.;2];for mode in [repeat%2,1-repeat%2]{let t=std::time::Instant::now();for _ in 0..200{let result=if mode==0{baseline::run(&c,&b,tol)}else{candidate::run(&c,&b,tol)};assert_eq!(result,old);std::hint::black_box(result);}times[mode]=t.elapsed().as_secs_f64()*1000.;}println!("paired repeat={repeat} baseline_ms={} candidate_ms={}",times[0],times[1]);}}