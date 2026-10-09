//! Sufficient continuous separation certificate for moving supporting lines.
//! Every arithmetic operation encloses its exact result using outward rounding.
//! Failure to prove separation is unknown, never permission to publish motion.
use crate::hair::CapsuleMotion;

use crate::hair::contact::polynomial::*;
fn linear_difference(
    a: CapsuleMotion,
    pa: usize,
    b: CapsuleMotion,
    pb: usize,
    axis: usize,
) -> Poly {
    let mut out = zero();
    let start = Bound::exact(a.start[pa][axis]).sub(Bound::exact(b.start[pb][axis]));
    let end = Bound::exact(a.end[pa][axis]).sub(Bound::exact(b.end[pb][axis]));
    out[0] = start;
    out[1] = end.sub(start);
    out
}
pub(super) fn clear(a: CapsuleMotion, b: CapsuleMotion, budget: usize) -> bool {
    let u: [Poly; 3] = std::array::from_fn(|i| linear_difference(a, 1, a, 0, i));
    let v: [Poly; 3] = std::array::from_fn(|i| linear_difference(b, 1, b, 0, i));
    let w: [Poly; 3] = std::array::from_fn(|i| linear_difference(a, 0, b, 0, i));
    let cross: [Poly; 3] = std::array::from_fn(|i| {
        difference(
            product(u[(i + 1) % 3], 1, v[(i + 2) % 3], 1),
            product(u[(i + 2) % 3], 1, v[(i + 1) % 3], 1),
        )
    });
    let mut norm = zero();
    let mut triple = zero();
    for i in 0..3 {
        norm = sum(norm, product(cross[i], 2, cross[i], 2));
        triple = sum(triple, product(w[i], 1, cross[i], 2));
    }
    let threshold = Bound::exact(a.radius)
        .add(Bound::exact(b.radius))
        .sub(Bound::exact(1e-10));
    if threshold.lo <= 0. || !threshold.hi.is_finite() {
        return false;
    }
    let squared = threshold.mul(threshold);
    let clearance = difference(
        product(triple, 3, triple, 3),
        std::array::from_fn(|i| norm[i].mul(squared)),
    );
    if certify_nonnegative(&[(clearance,6,false),(norm,4,true)],budget) {return true;}
    let mut alternatives=vec![vec![(clearance,6,false),(norm,4,true)]];
    for (segment,line) in [(a,b),(b,a)] {
        for endpoint in 0..2 {
            // The inward direction points from this endpoint into its segment.
            let inward:[Poly;3]=std::array::from_fn(|i|linear_difference(segment,1-endpoint,segment,endpoint,i));
            let direction:[Poly;3]=std::array::from_fn(|i|linear_difference(line,1,line,0,i));
            let offset:[Poly;3]=std::array::from_fn(|i|linear_difference(segment,endpoint,line,0,i));
            let mut length=zero();let mut along=zero();let mut inward_along=zero();let mut inward_offset=zero();let mut cross_squared=zero();
            for i in 0..3 {
                length=sum(length,product(direction[i],1,direction[i],1));
                along=sum(along,product(offset[i],1,direction[i],1));
                inward_along=sum(inward_along,product(inward[i],1,direction[i],1));
                inward_offset=sum(inward_offset,product(inward[i],1,offset[i],1));
                let c=difference(product(offset[(i+1)%3],1,direction[(i+2)%3],1),product(offset[(i+2)%3],1,direction[(i+1)%3],1));
                cross_squared=sum(cross_squared,product(c,2,c,2));
            }
            let separation=difference(cross_squared,std::array::from_fn(|i|length[i].mul(squared)));
            // inward dot (endpoint - projection_on_line) >= 0 ensures
            // every point of the finite segment stays on the safe side.
            let side=difference(product(inward_offset,2,length,2),product(inward_along,2,along,2));
            alternatives.push(vec![(separation,4,false),(side,4,false),(length,2,true)]);
        }
    }
    certify_alternatives(&alternatives,budget)
}

#[cfg(test)]
#[path = "contact_line_fixture_tests.rs"]
mod fixture_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finite_endpoint_certificate_keeps_crossings_unknown_and_endpoint_order_independent() {
        let a=CapsuleMotion {start:[[0.,0.,0.],[1.,0.,0.]],end:[[0.,0.,0.],[1.,0.,0.]],radius:1e-4};
        let b=CapsuleMotion {start:[[1.001,-0.1,0.],[1.001,0.1,0.]],end:[[1.001,-0.1,0.],[1.001,0.1,0.]],radius:1e-4};
        let reverse=|m:CapsuleMotion|CapsuleMotion {start:[m.start[1],m.start[0]],end:[m.end[1],m.end[0]],..m};
        for first in [a,reverse(a)] {
            for second in [b,reverse(b)] {
                assert!(clear(first,second,512));
                assert!(clear(second,first,512));
                assert!(!clear(first,second,0));
                let crossing=CapsuleMotion {end:second.end.map(|p|[0.999,p[1],p[2]]),..second};
                assert!(!clear(first,crossing,512),"finite endpoint proof admitted a swept crossing");
            }
        }
    }
    #[test]
    fn continuous_line_bound_rejects_tunnel_and_unknown_parallel_lines() {
        let a = CapsuleMotion {
            start: [[-0.001, -0.01, 0.], [-0.001, 0.01, 0.]],
            end: [[0.001, -0.01, 0.], [0.001, 0.01, 0.]],
            radius: 40e-6,
        };
        let b = CapsuleMotion {
            start: [[0., 0., -0.01], [0., 0., 0.01]],
            end: [[0., 0., -0.01], [0., 0., 0.01]],
            radius: 40e-6,
        };
        assert!(!clear(a, b, 512));
        let mut stationary = a;
        stationary.end = stationary.start;
        assert!(clear(stationary, b, 512));
        assert!(!clear(stationary, b, 0));
        let mut parallel = stationary;
        parallel.start = [[0.001, -0.01, 0.], [0.001, 0.01, 0.]];
        parallel.end = parallel.start;
        assert!(!clear(stationary, parallel, 512));
    }
    #[test]
    fn overflow_never_certifies_clearance() {
        let a = CapsuleMotion {
            start: [[1e300, -1e300, 0.], [1e300, 1e300, 0.]],
            end: [[1e300, -1e300, 0.], [1e300, 1e300, 0.]],
            radius: 40e-6,
        };
        let b = CapsuleMotion {
            start: [[0., 0., -1e300], [0., 0., 1e300]],
            end: [[0., 0., -1e300], [0., 0., 1e300]],
            radius: 40e-6,
        };
        assert!(!clear(a, b, 512));
    }
}
