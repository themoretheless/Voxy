//! Outward-rounded degree-six polynomial bounds shared by continuous contact queries.
#[derive(Clone, Copy, Debug)]
pub(in crate::hair) struct Bound {
    pub(in crate::hair) lo: f64,
    pub(in crate::hair) hi: f64,
}
impl Bound {
    pub(in crate::hair) fn exact(x: f64) -> Self {
        Self { lo: x, hi: x }
    }
    pub(in crate::hair) fn add(self, b: Self) -> Self {
        Self {
            lo: (self.lo + b.lo).next_down(),
            hi: (self.hi + b.hi).next_up(),
        }
    }
    pub(in crate::hair) fn neg(self) -> Self {
        Self {
            lo: -self.hi,
            hi: -self.lo,
        }
    }
    pub(in crate::hair) fn sub(self, b: Self) -> Self {
        self.add(b.neg())
    }
    pub(in crate::hair) fn mul(self, b: Self) -> Self {
        let products = [
            self.lo * b.lo,
            self.lo * b.hi,
            self.hi * b.lo,
            self.hi * b.hi,
        ];
        if products.iter().any(|x| !x.is_finite()) {
            return Self {
                lo: f64::NEG_INFINITY,
                hi: f64::INFINITY,
            };
        }
        Self {
            lo: products
                .into_iter()
                .fold(f64::INFINITY, f64::min)
                .next_down(),
            hi: products
                .into_iter()
                .fold(f64::NEG_INFINITY, f64::max)
                .next_up(),
        }
    }
}
pub(in crate::hair) type Poly = [Bound; 7];
pub(in crate::hair) fn zero() -> Poly {
    [Bound::exact(0.); 7]
}
pub(in crate::hair) fn sum(a: Poly, b: Poly) -> Poly {
    std::array::from_fn(|i| a[i].add(b[i]))
}
pub(in crate::hair) fn difference(a: Poly, b: Poly) -> Poly {
    std::array::from_fn(|i| a[i].sub(b[i]))
}
pub(in crate::hair) fn product(a: Poly, da: usize, b: Poly, db: usize) -> Poly {
    assert!(da + db <= 6);
    let mut out = zero();
    for i in 0..=da {
        for j in 0..=db {
            out[i + j] = out[i + j].add(a[i].mul(b[j]));
        }
    }
    out
}
pub(in crate::hair) fn choose(n: usize, k: usize) -> u32 {
    (0..k).fold(1, |value, i| value * (n - i) as u32 / (i + 1) as u32)
}
pub(in crate::hair) fn bernstein(power: Poly, degree: usize) -> Poly {
    let mut out = zero();
    for i in 0..=degree {
        for (j, coefficient) in power.iter().enumerate().take(i + 1) {
            let ratio = choose(i, j) as f64 / choose(degree, j) as f64;
            let enclosed = Bound {
                lo: ratio.next_down(),
                hi: ratio.next_up(),
            };
            out[i] = out[i].add(coefficient.mul(enclosed));
        }
    }
    out
}
pub(in crate::hair) fn split(mut values: Poly, degree: usize) -> (Poly, Poly) {
    let mut left = zero();
    let mut right = zero();
    left[0] = values[0];
    right[degree] = values[degree];
    for level in 1..=degree {
        for i in 0..=degree - level {
            values[i] = values[i].add(values[i + 1]).mul(Bound::exact(0.5));
        }
        left[level] = values[0];
        right[degree - level] = values[degree - level];
    }
    (left, right)
}

/// Joint sufficient certificate; unknown/overflow/budget exhaustion is false.
pub(in crate::hair) fn certify_nonnegative(input: &[(Poly, usize, bool)], budget: usize) -> bool {
    let initial: Vec<_> = input
        .iter()
        .map(|(power, degree, strict)| (bernstein(*power, *degree), *degree, *strict))
        .collect();
    let mut stack = vec![(initial, 0usize)];
    let mut visited = 0;
    while let Some((polys, depth)) = stack.pop() {
        visited += 1;
        if visited > budget {
            return false;
        }
        if polys.iter().any(|(p, d, _)| {
            p[..=*d]
                .iter()
                .any(|b| !b.lo.is_finite() || !b.hi.is_finite())
        }) {
            return false;
        }
        if polys.iter().all(|(p, d, strict)| {
            p[..=*d]
                .iter()
                .all(|b| if *strict { b.lo > 0. } else { b.lo >= 0. })
        }) {
            continue;
        }
        if depth >= 48
            || polys
                .iter()
                .any(|(p, d, _)| p[..=*d].iter().all(|b| b.hi < 0.))
        {
            return false;
        }
        let mut left = Vec::with_capacity(polys.len());
        let mut right = Vec::with_capacity(polys.len());
        for (p, d, strict) in polys {
            let (a, b) = split(p, d);
            left.push((a, d, strict));
            right.push((b, d, strict));
        }
        stack.push((right, depth + 1));
        stack.push((left, depth + 1));
    }
    true
}

/// Each temporal interval must satisfy one complete sufficient certificate.
/// Different intervals may use different separating features.
pub(in crate::hair) fn certify_alternatives(input:&[Vec<(Poly,usize,bool)>],budget:usize)->bool {
    let initial:Vec<Vec<_>>=input.iter().map(|rows|rows.iter().map(|(p,d,s)|(bernstein(*p,*d),*d,*s)).collect()).collect();
    let mut stack=vec![(initial,0usize)];let mut visited=0;
    while let Some((alternatives,depth))=stack.pop() {
        visited+=1;if visited>budget {return false;}
        let finite=|p:&Poly,d:usize|p[..=d].iter().all(|b|b.lo.is_finite()&&b.hi.is_finite());
        if alternatives.iter().any(|rows| !rows.is_empty() && rows.iter().all(|(p,d,strict)|finite(p,*d)&&p[..=*d].iter().all(|b|if *strict {b.lo>0.} else {b.lo>=0.}))) {continue;}
        if depth>=48 {return false;}
        let mut left=Vec::new();let mut right=Vec::new();
        for rows in alternatives {
            // A nonfinite or wholly negative required row makes this feature
            // unavailable throughout this interval; another feature may prove it.
            if rows.iter().any(|(p,d,_)|!finite(p,*d)||p[..=*d].iter().all(|b|b.hi<0.)) {continue;}
            let mut l=Vec::new();let mut r=Vec::new();
            for (p,d,s) in rows {let (a,b)=split(p,d);l.push((a,d,s));r.push((b,d,s));}
            left.push(l);right.push(r);
        }
        if left.is_empty() {return false;}
        stack.push((right,depth+1));stack.push((left,depth+1));
    }
    true
}
