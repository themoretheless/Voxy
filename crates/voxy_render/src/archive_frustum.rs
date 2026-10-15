//! Four-lane presentation culling with the scalar equation and stable ordering.
use glam::{Vec3, Vec4};
#[derive(Clone, Debug, Default)]
struct Group {
    x: [f32; 4],
    y: [f32; 4],
    z: [f32; 4],
    radius: [f32; 4],
}
#[derive(Debug)]
pub(crate) struct ArchiveFrustumGroups {
    groups: Vec<Group>,
    count: usize,
}
impl ArchiveFrustumGroups {
    pub fn new(spheres: &[(Vec3, f32)]) -> Self {
        let mut groups = Vec::with_capacity(spheres.len().div_ceil(4));
        for chunk in spheres.chunks(4) {
            let mut g = Group::default();
            for (i, (c, r)) in chunk.iter().enumerate() {
                g.x[i] = c.x;
                g.y[i] = c.y;
                g.z[i] = c.z;
                g.radius[i] = *r;
            }
            groups.push(g);
        }
        Self {
            groups,
            count: spheres.len(),
        }
    }
    pub fn collect(&self, planes: &[(Vec4, f32); 6], out: &mut Vec<usize>) {
        out.clear();
        for (group, g) in self.groups.iter().enumerate() {
            let mask = visible(g, planes);
            for (lane, keep) in mask.into_iter().enumerate() {
                let i = group * 4 + lane;
                if i < self.count && keep != 0 {
                    out.push(i);
                }
            }
        }
    }
}
fn visible(g:&Group,planes:&[(Vec4,f32);6])->[u32;4] {
    // glam selects the native SIMD backend through its safe vector API. Keep
    // multiply/add separate to match the scalar expression's rounding sequence.
    let x=Vec4::from_array(g.x);let y=Vec4::from_array(g.y);let z=Vec4::from_array(g.z);let radius=Vec4::from_array(g.radius);
    let mut keep=Vec4::ONE.cmpge(Vec4::ZERO);
    for &(p,length) in planes {
        let xy=x*p.x+y*p.y;
        let xyz=xy+z*p.z;
        let distance=xyz+Vec4::splat(p.w);
        let cutoff=(-radius)*length;
        keep &= distance.cmpge(cutoff);
    }
    let mask=keep.bitmask();std::array::from_fn(|i|if mask&(1<<i)!=0 {u32::MAX}else{0})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vectorized_selection_equals_scalar_including_boundary_cases() {
        let planes = [
            Vec4::new(1., 0., 0., 1.),
            Vec4::new(-1., 0., 0., 1.),
            Vec4::new(0., 1., 0., 1.),
            Vec4::new(0., -1., 0., 1.),
            Vec4::new(0., 0., 1., 1.),
            Vec4::new(0., 0., -1., 1.),
        ]
        .map(|p| (p, p.truncate().length()));
        let mut spheres: Vec<_> = (0..10003)
            .map(|i| {
                let x = ((i * 977) % 65536) as f32 / 16000. - 2.;
                let y = ((i * 919) % 65536) as f32 / 16000. - 2.;
                let z = ((i * 853) % 65536) as f32 / 16000. - 2.;
                (Vec3::new(x, y, z), (i % 37) as f32 / 127.)
            })
            .collect();
        for x in [
            1_f32,
            -1.,
            f32::from_bits(1_f32.to_bits() + 1),
            f32::from_bits(1_f32.to_bits() - 1),
            f32::NAN,
        ] {
            spheres.push((Vec3::new(x, 0., 0.), 0.));
        }
        let expected: Vec<_> = spheres
            .iter()
            .enumerate()
            .filter_map(|(i, (c, r))| {
                planes
                    .iter()
                    .all(|(p, len)| p.truncate().dot(*c) + p.w >= -r * len)
                    .then_some(i)
            })
            .collect();
        let groups = ArchiveFrustumGroups::new(&spheres);
        let mut actual = Vec::new();
        groups.collect(&planes, &mut actual);
        assert_eq!(actual, expected);
    }
}
