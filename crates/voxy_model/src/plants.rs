//! Seeded botanical surfaces and self-contained glTF assets, in metres.
use glam::Vec3;
use serde_json::json;
use std::f32::consts::TAU;
use std::io::Cursor;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlantKind {
    Tree,
    Shrub,
}
#[derive(Clone, Copy, Debug)]
pub struct PlantOptions {
    pub kind: PlantKind,
    pub seed: u64,
    pub height: f32,
}
impl Default for PlantOptions {
    fn default() -> Self {
        Self {
            kind: PlantKind::Tree,
            seed: 42,
            height: 7.0,
        }
    }
}
#[derive(Debug, Default)]
struct Surface {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
    indices: Vec<u32>,
}
#[derive(Debug)]
struct Random(u64);
impl Random {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut x = self.0;
        x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
        ((x ^ (x >> 31)) >> 40) as f32 / 16_777_216.0
    }
    fn signed(&mut self) -> f32 {
        self.next() * 2.0 - 1.0
    }
}
impl Surface {
    fn vertex(&mut self, p: Vec3, uv: [f32; 2]) -> u32 {
        let id = self.positions.len() as u32;
        self.positions.push(p.to_array());
        self.uv.push(uv);
        self.normals.push([0.; 3]);
        id
    }
    fn quad(&mut self, a: u32, b: u32, c: u32, d: u32) {
        for triangle in [[a, b, c], [a, c, d]] {
            let p = |i: u32| Vec3::from_array(self.positions[i as usize]);
            if (p(triangle[1]) - p(triangle[0]))
                .cross(p(triangle[2]) - p(triangle[0]))
                .length_squared()
                > 1e-20
            {
                self.indices.extend(triangle);
            }
        }
    }
    fn tangents(&self) -> Vec<[f32; 4]> {
        let mut t = vec![Vec3::ZERO; self.positions.len()];
        let mut b = t.clone();
        for ids in self.indices.chunks_exact(3) {
            let [a, c, d] = [ids[0] as usize, ids[1] as usize, ids[2] as usize];
            let e1 = Vec3::from_array(self.positions[c]) - Vec3::from_array(self.positions[a]);
            let e2 = Vec3::from_array(self.positions[d]) - Vec3::from_array(self.positions[a]);
            let uv1 = glam::Vec2::from_array(self.uv[c]) - glam::Vec2::from_array(self.uv[a]);
            let uv2 = glam::Vec2::from_array(self.uv[d]) - glam::Vec2::from_array(self.uv[a]);
            let det = uv1.x * uv2.y - uv1.y * uv2.x;
            if det.abs() > 1e-12 {
                let tangent = (e1 * uv2.y - e2 * uv1.y) / det;
                let bitangent = (e2 * uv1.x - e1 * uv2.x) / det;
                for i in [a, c, d] {
                    t[i] += tangent;
                    b[i] += bitangent;
                }
            }
        }
        self.normals
            .iter()
            .enumerate()
            .map(|(i, normal)| {
                let n = Vec3::from_array(*normal);
                let tangent = (t[i] - n * n.dot(t[i])).normalize_or(n.any_orthonormal_vector());
                let sign = if n.cross(tangent).dot(b[i]) < 0. {
                    -1.
                } else {
                    1.
                };
                [tangent.x, tangent.y, tangent.z, sign]
            })
            .collect()
    }
    fn normals(&mut self) {
        for t in self.indices.chunks_exact(3) {
            let p = |i: u32| Vec3::from_array(self.positions[i as usize]);
            let n = (p(t[1]) - p(t[0])).cross(p(t[2]) - p(t[0]));
            for &i in t {
                self.normals[i as usize] =
                    (Vec3::from_array(self.normals[i as usize]) + n).to_array();
            }
        }
        for n in &mut self.normals {
            *n = Vec3::from_array(*n).normalize_or(Vec3::Y).to_array();
        }
    }
}
struct Growth {
    world_detail: bool,
    wood: Surface,
    leaves: Surface,
    rng: Random,
    scale: f32,
}
/// Leaf texture atlas: 2×2 tint variants of one alpha-cut blade.
const LEAF_CELLS: u32 = 2;
const LEAF_VERTICES: usize = 12;
impl Growth {
    /// One alpha-cut leaf card, folded along the midrib and curled lengthwise.
    fn leaf(&mut self, base: Vec3, outward: Vec3) {
        let length = (self.scale / 6.).clamp(0.6, 1.4) * (0.085 + self.rng.next() * 0.04);
        // Blades turn towards the sky but keep the twig's outward heading.
        let forward = (outward * 0.8
            + Vec3::new(
                self.rng.signed(),
                self.rng.signed() * 0.6 - 0.15,
                self.rng.signed(),
            ) * 0.55)
            .normalize_or(Vec3::Y);
        let up = (Vec3::Y * 1.2
            + outward * 0.4
            + Vec3::new(self.rng.signed(), 0., self.rng.signed()) * 0.4)
            .normalize();
        let side = forward
            .cross(up)
            .normalize_or(forward.any_orthonormal_vector());
        let normal = side.cross(forward).normalize();
        let fold = 0.12 + self.rng.next() * 0.18;
        let curl = self.rng.signed() * 0.18 - 0.08;
        let cell =
            (self.rng.next() * (LEAF_CELLS * LEAF_CELLS) as f32) as u32 % (LEAF_CELLS * LEAF_CELLS);
        let cell_u = (cell % LEAF_CELLS) as f32 / LEAF_CELLS as f32;
        let cell_v = (cell / LEAF_CELLS) as f32 / LEAF_CELLS as f32;
        let width = length * 0.62;
        let start = self.leaves.positions.len() as u32;
        for row in 0..=3 {
            let t = row as f32 / 3.;
            let centre = base + forward * (length * t) + normal * (length * curl * t * t);
            for col in 0..=2 {
                let s = col as f32 - 1.;
                let p = centre + side * (width * 0.5 * s) + normal * (width * fold * s.abs());
                let u = cell_u + (col as f32 / 2. * 0.96 + 0.02) / LEAF_CELLS as f32;
                let v = cell_v + (t * 0.96 + 0.02) / LEAF_CELLS as f32;
                self.leaves.vertex(p, [u, v]);
            }
        }
        for row in 0..3 {
            for col in 0..2 {
                let a = start + row * 3 + col;
                self.leaves.quad(a, a + 3, a + 4, a + 1);
            }
        }
    }
    /// Sweep a tube through skeleton points; `flare` widens the trunk base into root buttresses.
    fn tube(&mut self, points: &[Vec3], radii: &[f32], flare: Option<f32>) {
        let n = points.len();
        if n < 2 {
            return;
        }
        let base_r = radii[0];
        let sides: u32 = match (self.world_detail, base_r) {
            (true, r) if r > 0.06 => 18,
            (false, r) if r > 0.06 => 28,
            (true, r) if r > 0.02 => 8,
            (false, r) if r > 0.02 => 12,
            (true, _) => 4,
            (false, _) => 6,
        };
        let dirs: Vec<Vec3> = (0..n)
            .map(|i| {
                let a = points[i.saturating_sub(1)];
                let b = points[(i + 1).min(n - 1)];
                (b - a).normalize_or(Vec3::Y)
            })
            .collect();
        // Parallel-transport frames keep the bark UVs from twisting.
        let mut side = d_any(dirs[0]);
        let mut frames = Vec::with_capacity(n);
        for i in 0..n {
            if i > 0 {
                let axis = dirs[i - 1].cross(dirs[i]);
                if axis.length_squared() > 1e-12 {
                    let angle = dirs[i - 1].dot(dirs[i]).clamp(-1., 1.).acos();
                    side = glam::Quat::from_axis_angle(axis.normalize(), angle) * side;
                }
                side = (side - dirs[i] * side.dot(dirs[i])).normalize_or(d_any(dirs[i]));
            }
            frames.push((side, dirs[i].cross(side)));
        }
        let wraps = (base_r / 0.08).clamp(1., 3.).round();
        let mut v_coord = 0.;
        let start = self.wood.positions.len() as u32;
        for i in 0..n {
            if i > 0 {
                v_coord += points[i].distance(points[i - 1]) / (base_r.max(0.01) * TAU / wraps);
            }
            let (s, o) = frames[i];
            for col in 0..=sides {
                let angle = col as f32 / sides as f32 * TAU;
                let ridges = 1.
                    + 0.035 * (angle * 9. + v_coord * 0.7).sin()
                    + 0.02 * (angle * 17. - v_coord * 1.3).sin();
                let widen = flare.map_or(1., |height| {
                    let lobes = 1. + 0.35 * (angle * 5. + 0.7).sin().max(0.);
                    1. + 0.9 * (-points[i].y.max(0.) / height).exp().powi(2) * lobes
                });
                self.wood.vertex(
                    points[i] + (s * angle.cos() + o * angle.sin()) * radii[i] * ridges * widen,
                    [col as f32 / sides as f32 * wraps, v_coord],
                );
            }
        }
        for ring in 0..n as u32 - 1 {
            for col in 0..sides {
                let a = start + ring * (sides + 1) + col;
                self.wood.quad(a, a + 1, a + sides + 2, a + sides + 1);
            }
        }
        // Close the tip; the basal ring stays embedded in its parent or the ground.
        let tip = self
            .wood
            .vertex(points[n - 1] + dirs[n - 1] * radii[n - 1], [0.5, v_coord]);
        for col in 0..sides {
            let a = start + (n as u32 - 1) * (sides + 1) + col;
            let q = |i: u32| Vec3::from_array(self.wood.positions[i as usize]);
            if (q(a + 1) - q(a)).cross(q(tip) - q(a)).length_squared() > 1e-20 {
                self.wood.indices.extend([a, a + 1, tip]);
            }
        }
    }
}
/// One internode plus the node at its tip (Palubicki et al. 2009, "Self-organizing tree models").
/// `main` continues the axis from the terminal bud; `lateral` grows from the axillary bud.
#[derive(Clone, Debug)]
struct Metamer {
    base: Vec3,
    tip: Vec3,
    dir: Vec3,
    parent: Option<usize>,
    main: Option<usize>,
    lateral: Option<usize>,
    /// Axillary bud heading, set by phyllotaxis when the metamer forms.
    lateral_dir: Vec3,
    /// Buds below the crown base stay dormant, as on a forest-grown bole.
    dormant: bool,
    alive: bool,
    /// Light/space exposure gathered this year, per bud and per subtree.
    q_main: f32,
    q_lateral: f32,
    q: f32,
    optimal_main: Vec3,
    optimal_lateral: Vec3,
    radius: f32,
    age: u32,
}
/// Growth constants; defaults follow the ranges reported by Palubicki et al. and Runions et al.
struct GrowthModel {
    /// Internode length D (m).
    internode: f32,
    /// Apical control λ in the Borchert–Honda resource split.
    lambda: f32,
    /// Resource scale α: v_base = α·Q_base.
    alpha: f32,
    /// Weights of default heading, environmental optimum, and tropism.
    xi: f32,
    eta: f32,
    tropism: Vec3,
    /// Space colonisation: perception radius, kill radius (in D) and perception half-angle.
    influence: f32,
    kill: f32,
    cone_cos: f32,
    /// Lateral bud divergence from its axis and phyllotactic rotation (radians).
    branch_angle: f32,
    rotation: f32,
    years: u32,
    max_metamers: usize,
    /// Shed lateral branches whose exposure per metamer falls below this.
    shed_ratio: f32,
    /// Murray exponent n in r_parentⁿ = Σ r_childⁿ (2 is Leonardo's rule).
    murray: f32,
    tip_radius: f32,
}
struct Skeleton {
    metamers: Vec<Metamer>,
}
impl Skeleton {
    fn push(
        &mut self,
        parent: Option<usize>,
        tip: Vec3,
        dir: Vec3,
        lateral_dir: Vec3,
        dormant: bool,
    ) -> usize {
        // Roots start one internode back along their heading; callers may override.
        let base = parent.map_or(tip - dir, |p| self.metamers[p].tip);
        self.metamers.push(Metamer {
            base,
            tip,
            dir,
            parent,
            main: None,
            lateral: None,
            lateral_dir,
            dormant,
            alive: true,
            q_main: 0.,
            q_lateral: 0.,
            q: 0.,
            optimal_main: Vec3::ZERO,
            optimal_lateral: Vec3::ZERO,
            radius: 0.,
            age: 0,
        });
        self.metamers.len() - 1
    }
    fn base(&self, id: usize) -> Vec3 {
        self.metamers[id].base
    }
    /// Children are always pushed after their parent, so reverse index order is post-order.
    fn order(&self) -> std::ops::Range<usize> {
        0..self.metamers.len()
    }
}
fn phyllotaxis(dir: Vec3, previous: Vec3, model: &GrowthModel, rng: &mut Random) -> Vec3 {
    let side = (previous - dir * previous.dot(dir)).normalize_or(d_any(dir));
    let rotated = glam::Quat::from_axis_angle(dir, model.rotation + rng.signed() * 0.25) * side;
    let angle = model.branch_angle * (0.85 + 0.3 * rng.next());
    (dir * angle.cos() + rotated * angle.sin()).normalize()
}
/// Grow a skeleton by competition for space inside `envelope` (centre, radii).
fn grow_skeleton(
    model: &GrowthModel,
    stems: &[(Vec3, Vec3)],
    crown_base: f32,
    envelope: (Vec3, Vec3),
    markers: usize,
    rng: &mut Random,
) -> Skeleton {
    let d = model.internode;
    let mut sk = Skeleton {
        metamers: Vec::new(),
    };
    // Seed axes: a straight bole up to the crown base, buds dormant below it.
    for &(origin, heading) in stems {
        let mut parent = None;
        let mut tip = origin;
        let mut lateral = d_any(heading);
        let mut dir = heading.normalize();
        loop {
            dir = (dir + Vec3::new(rng.signed(), 0., rng.signed()) * 0.03).normalize();
            tip += dir * d;
            lateral = phyllotaxis(dir, lateral, model, rng);
            let id = sk.push(parent, tip, dir, lateral, tip.y < crown_base);
            if parent.is_none() {
                sk.metamers[id].base = origin;
            }
            if let Some(p) = parent {
                sk.metamers[p].main = Some(id);
            }
            parent = Some(id);
            if tip.y >= crown_base {
                break;
            }
        }
    }
    // Attraction points fill the crown volume uniformly (rejection sampling).
    let (centre, radii) = envelope;
    let mut points = Vec::with_capacity(markers);
    while points.len() < markers {
        let p = Vec3::new(rng.signed(), rng.signed(), rng.signed());
        if p.length_squared() <= 1. {
            let world = centre + p * radii;
            if world.y > crown_base * 0.85 {
                points.push(world);
            }
        }
    }
    let cell = model.influence * d;
    let key = |p: Vec3| {
        (
            (p.x / cell).floor() as i32,
            (p.y / cell).floor() as i32,
            (p.z / cell).floor() as i32,
        )
    };
    for _year in 0..model.years {
        if sk.metamers.len() >= model.max_metamers {
            break;
        }
        for m in &mut sk.metamers {
            m.q_main = 0.;
            m.q_lateral = 0.;
            m.optimal_main = Vec3::ZERO;
            m.optimal_lateral = Vec3::ZERO;
        }
        // Buds: (metamer, is_lateral, position, heading).
        let mut buds = Vec::new();
        for (id, m) in sk.metamers.iter().enumerate() {
            if !m.alive {
                continue;
            }
            if m.main.is_none() {
                buds.push((id, false, m.tip, m.dir));
            }
            if m.lateral.is_none() && !m.dormant {
                buds.push((id, true, m.tip, m.lateral_dir));
            }
        }
        let mut grid: std::collections::HashMap<(i32, i32, i32), Vec<usize>> = Default::default();
        for (b, bud) in buds.iter().enumerate() {
            grid.entry(key(bud.2)).or_default().push(b);
        }
        // Each marker is perceived by its closest bud whose cone contains it.
        let mut owner = vec![None; points.len()];
        let mut killed = vec![false; points.len()];
        for (i, &p) in points.iter().enumerate() {
            let (cx, cy, cz) = key(p);
            let mut best = (f32::INFINITY, None);
            for dz in -1..=1 {
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        for &b in grid.get(&(cx + dx, cy + dy, cz + dz)).into_iter().flatten() {
                            let (_, _, pos, heading) = buds[b];
                            let offset = p - pos;
                            let dist = offset.length();
                            if dist < model.kill * d {
                                killed[i] = true;
                            }
                            if dist < cell
                                && dist > 1e-6
                                && offset.dot(heading) / dist >= model.cone_cos
                                && dist < best.0
                            {
                                best = (dist, Some(b));
                            }
                        }
                    }
                }
            }
            owner[i] = best.1;
        }
        for (i, b) in owner.iter().enumerate() {
            if let Some(b) = *b {
                let (id, lateral, pos, _) = buds[b];
                let toward = (points[i] - pos).normalize();
                let m = &mut sk.metamers[id];
                if lateral {
                    m.q_lateral = 1.;
                    m.optimal_lateral += toward;
                } else {
                    m.q_main = 1.;
                    m.optimal_main += toward;
                }
            }
        }
        let mut keep = killed.iter().map(|k| !k);
        points.retain(|_| keep.next().unwrap_or(true));
        // Basipetal pass: a subtree's exposure is the sum of its buds'.
        for id in sk.order().rev() {
            let m = &sk.metamers[id];
            if !m.alive {
                continue;
            }
            let qm = m.main.map_or(m.q_main, |c| sk.metamers[c].q);
            let ql = m.lateral.map_or(m.q_lateral, |c| sk.metamers[c].q);
            let m = &mut sk.metamers[id];
            m.q_main = qm;
            m.q_lateral = ql;
            m.q = qm + ql;
        }
        // Acropetal pass: Borchert–Honda split of the resource between axis and lateral.
        let mut vigour = vec![0f32; sk.metamers.len()];
        for (id, m) in sk.metamers.iter().enumerate() {
            if m.parent.is_none() && m.alive {
                vigour[id] = model.alpha * m.q;
            }
        }
        let mut growth = Vec::new();
        for id in sk.order() {
            let m = &sk.metamers[id];
            if !m.alive {
                continue;
            }
            let v = vigour[id];
            let (qm, ql) = (m.q_main, m.q_lateral);
            let denominator = model.lambda * qm + (1. - model.lambda) * ql;
            let (vm, vl) = if denominator > 0. {
                (
                    v * model.lambda * qm / denominator,
                    v * (1. - model.lambda) * ql / denominator,
                )
            } else {
                (0., 0.)
            };
            match m.main {
                Some(c) => vigour[c] = vm,
                None if vm >= 1. => growth.push((id, false, vm)),
                None => {}
            }
            match m.lateral {
                Some(c) => vigour[c] = vl,
                None if vl >= 1. && !m.dormant => growth.push((id, true, vl)),
                None => {}
            }
        }
        // Each active bud extends into ⌊v⌋ metamers along a blended heading.
        for (id, lateral, v) in growth {
            let count = (v.floor() as usize).min(4);
            let m = &sk.metamers[id];
            let (mut heading, optimal) = if lateral {
                (m.lateral_dir, m.optimal_lateral)
            } else {
                (m.dir, m.optimal_main)
            };
            let optimal = optimal.normalize_or(heading);
            let mut tip = m.tip;
            let mut lateral_dir = m.lateral_dir;
            let mut parent = id;
            for k in 0..count {
                if sk.metamers.len() >= model.max_metamers {
                    break;
                }
                heading = (heading * model.xi + optimal * model.eta + model.tropism).normalize();
                tip += heading * d;
                lateral_dir = phyllotaxis(heading, lateral_dir, model, rng);
                let child = sk.push(Some(parent), tip, heading, lateral_dir, false);
                if k == 0 && lateral {
                    sk.metamers[parent].lateral = Some(child);
                } else {
                    sk.metamers[parent].main = Some(child);
                }
                parent = child;
            }
        }
        // Shedding: lateral limbs that no longer pay for themselves are dropped.
        let mut size = vec![0usize; sk.metamers.len()];
        for id in sk.order().rev() {
            let m = &sk.metamers[id];
            if m.alive {
                size[id] = 1 + m.main.map_or(0, |c| size[c]) + m.lateral.map_or(0, |c| size[c]);
            }
        }
        for id in sk.order() {
            sk.metamers[id].age += 1;
            if let Some(c) = sk.metamers[id].lateral {
                let child = &sk.metamers[c];
                if child.alive
                    && child.age > 2
                    && size[c] > 3
                    && child.q / (size[c] as f32) < model.shed_ratio
                {
                    let mut stack = vec![c];
                    while let Some(x) = stack.pop() {
                        sk.metamers[x].alive = false;
                        stack.extend(sk.metamers[x].main);
                        stack.extend(sk.metamers[x].lateral);
                    }
                    sk.metamers[id].lateral = None;
                    // A shed limb leaves a scar, not a fresh bud.
                    sk.metamers[id].dormant = true;
                }
            }
        }
    }
    // Murray's law, tips upward: r_parentⁿ = r_mainⁿ + r_lateralⁿ.
    for id in sk.order().rev() {
        let m = &sk.metamers[id];
        if !m.alive {
            continue;
        }
        let n = model.murray;
        let sum = m.main.map_or(0., |c| sk.metamers[c].radius.powf(n))
            + m.lateral.map_or(0., |c| sk.metamers[c].radius.powf(n));
        sk.metamers[id].radius = if sum > 0. {
            sum.powf(1. / n)
        } else {
            model.tip_radius
        };
    }
    sk
}
impl Growth {
    /// Mesh each axis as one tube and dress thin twigs with leaves.
    fn build(&mut self, sk: &Skeleton, flare_height: Option<f32>, twig_radius: f32) {
        let leaves_per_metamer = if self.world_detail { 4 } else { 6 };
        for (start, m) in sk.metamers.iter().enumerate() {
            // An axis starts at a root metamer or at a lateral shoot.
            let starts_axis = m.alive
                && m.parent
                    .is_none_or(|p| sk.metamers[p].lateral == Some(start));
            if !starts_axis {
                continue;
            }
            let mut points =
                vec![sk.base(start) - m.dir * if m.parent.is_none() { 0.05 } else { 0. }];
            let mut radii = vec![m.radius];
            let mut id = Some(start);
            while let Some(i) = id {
                let x = &sk.metamers[i];
                points.push(x.tip);
                radii.push(x.radius);
                if x.radius < twig_radius {
                    for k in 0..leaves_per_metamer {
                        let angle = (i * 7 + k) as f32 * 2.399963 + self.rng.next() * 0.5;
                        let side = d_any(x.dir);
                        let outward = (glam::Quat::from_axis_angle(x.dir, angle) * side
                            + x.dir * 0.6)
                            .normalize();
                        let along = sk.base(i).lerp(
                            x.tip,
                            (k as f32 + self.rng.next()) / leaves_per_metamer as f32,
                        );
                        self.leaf(along + outward * x.radius, outward);
                    }
                }
                id = x.main.filter(|&c| sk.metamers[c].alive);
            }
            // Laplacian smoothing hides the internode polyline without moving the ends.
            for _ in 0..2 {
                let copy = points.clone();
                for j in 1..points.len() - 1 {
                    points[j] = copy[j] * 0.5 + (copy[j - 1] + copy[j + 1]) * 0.25;
                }
            }
            let flare = if m.parent.is_none() {
                flare_height
            } else {
                None
            };
            self.tube(&points, &radii, flare);
        }
    }
}
fn d_any(d: Vec3) -> Vec3 {
    d.cross(if d.y.abs() > 0.95 { Vec3::X } else { Vec3::Y })
        .normalize()
}
fn noise(x: u32, y: u32, seed: u64) -> f32 {
    let mut r = Random(
        seed ^ u64::from(x).wrapping_mul(0x517cc1b7) ^ u64::from(y).wrapping_mul(0x9e3779b9),
    );
    r.next()
}
fn tile_noise(u: f32, v: f32, nx: u32, ny: u32, seed: u64) -> f32 {
    let x = u * nx as f32;
    let y = v * ny as f32;
    let ix = x.floor() as u32;
    let iy = y.floor() as u32;
    let sx = x.fract();
    let sy = y.fract();
    let sx = sx * sx * (3. - 2. * sx);
    let sy = sy * sy * (3. - 2. * sy);
    let a = noise(ix % nx, iy % ny, seed);
    let b = noise((ix + 1) % nx, iy % ny, seed);
    let c = noise(ix % nx, (iy + 1) % ny, seed);
    let d = noise((ix + 1) % nx, (iy + 1) % ny, seed);
    (a + (b - a) * sx) * (1. - sy) + (c + (d - c) * sx) * sy
}
/// Tileable cellular noise: (F1, F2) distances to jittered feature points.
fn cells(u: f32, v: f32, nx: u32, ny: u32, seed: u64) -> (f32, f32) {
    let x = u * nx as f32;
    let y = v * ny as f32;
    let (ix, iy) = (x.floor() as i64, y.floor() as i64);
    let mut f = (f32::INFINITY, f32::INFINITY);
    for dy in -1..=1 {
        for dx in -1..=1 {
            let cx = (ix + dx).rem_euclid(nx as i64) as u32;
            let cy = (iy + dy).rem_euclid(ny as i64) as u32;
            let px = (ix + dx) as f32 + noise(cx, cy, seed);
            let py = (iy + dy) as f32 + noise(cx, cy, seed ^ 0x55);
            let d = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
            if d < f.0 {
                f = (d, f.0);
            } else if d < f.1 {
                f.1 = d;
            }
        }
    }
    f
}
fn fbm(u: f32, v: f32, base: u32, seed: u64) -> f32 {
    let mut sum = 0.;
    let mut amplitude = 0.5;
    let mut n = base;
    for octave in 0..4 {
        sum += amplitude * tile_noise(u, v, n, n, seed ^ octave);
        amplitude *= 0.5;
        n *= 2;
    }
    sum / 0.9375
}
/// Bark: vertical plates split by deep fissures, with lichen in sheltered patches.
fn bark_texel(u: f32, v: f32, seed: u64) -> (f32, [f32; 3], f32, f32) {
    let warp = fbm(u, v, 4, seed ^ 7) - 0.5;
    let (f1, f2) = cells(u + warp * 0.08, v, 11, 4, seed);
    let edge = f2 - f1;
    let plate = (edge * 5.).clamp(0., 1.).powf(0.6);
    let grain = tile_noise(u, v, 96, 12, seed ^ 31);
    let flakes = fbm(u, v, 16, seed ^ 0x29);
    let h = plate * (0.55 + flakes * 0.35) + grain * 0.08;
    let tone = 0.75 + 0.5 * fbm(u, v, 3, seed ^ 3);
    let mut rgb = [
        (0.30 + 0.16 * plate + 0.05 * flakes) * tone,
        (0.26 + 0.13 * plate + 0.04 * flakes) * tone,
        (0.21 + 0.10 * plate + 0.03 * flakes) * tone,
    ];
    // Fissure floors are darker and redder where bark is fresh.
    let floor = 1. - plate;
    rgb = [
        rgb[0] * (1. - 0.55 * floor) + 0.04 * floor,
        rgb[1] * (1. - 0.62 * floor),
        rgb[2] * (1. - 0.68 * floor),
    ];
    let lichen = ((fbm(u, v, 6, seed ^ 0x63) - 0.58) * 9.).clamp(0., 1.) * plate;
    rgb = [
        rgb[0] + (0.46 - rgb[0]) * lichen * 0.75,
        rgb[1] + (0.52 - rgb[1]) * lichen * 0.75,
        rgb[2] + (0.40 - rgb[2]) * lichen * 0.75,
    ];
    (
        h + lichen * 0.05,
        rgb,
        0.82 + 0.12 * floor - 0.06 * lichen,
        1.,
    )
}
/// Leaf atlas cell: serrated ovate blade with petiole, midrib and pinnate veins.
fn leaf_texel(u: f32, v: f32, seed: u64) -> (f32, [f32; 3], f32, f32) {
    let cell = (u * LEAF_CELLS as f32).floor() as u32
        + (v * LEAF_CELLS as f32).floor() as u32 * LEAF_CELLS;
    let cu = (u * LEAF_CELLS as f32).fract();
    let cv = (v * LEAF_CELLS as f32).fract();
    let x = (cu - 0.5) * 2.;
    // Blade runs from the petiole at the bottom of the cell to the acuminate tip.
    let s = ((cv - 0.12) / 0.86).clamp(0., 1.);
    let profile = (std::f32::consts::PI * s.powf(0.82))
        .sin()
        .max(0.)
        .powf(0.78)
        * (1. - 0.18 * s);
    let teeth = 1. - 0.05 * ((s * 46.).fract() - 0.5).abs() * 2.;
    let half = 0.92 * profile * teeth;
    let blade = cv > 0.12 && cv < 0.98 && x.abs() < half;
    let petiole = cv <= 0.13 && x.abs() < 0.035 * (1.2 - cv * 3.);
    let alpha = if blade || petiole { 1. } else { 0. };
    let mid = x.abs();
    let rib = (-mid * mid * 900.).exp();
    let vein_phase = s * 8. - mid * 2.2;
    let vein = (-((vein_phase.fract() - 0.5).powi(2)) * 160.).exp()
        * (1. - mid / half.max(0.05)).clamp(0., 1.);
    let fine = tile_noise(u, v, 128, 128, seed ^ 5);
    let speck = tile_noise(u, v, 40, 40, seed ^ 9);
    let tints: [[f32; 3]; 4] = [
        [0.20, 0.33, 0.07],
        [0.27, 0.40, 0.09],
        [0.16, 0.27, 0.08],
        [0.33, 0.40, 0.11],
    ];
    let base = tints[cell as usize % 4];
    let tone = 0.85 + 0.25 * fbm(u, v, 6, seed ^ (u64::from(cell) + 1)) + 0.05 * fine;
    // Margins age first: slight yellowing and a thin necrotic rim on some leaves.
    let edge = ((mid / half.max(0.01)) - 0.82).max(0.) * 5.5;
    let rim = if cell == 3 { edge } else { edge * 0.35 };
    let rgb = [
        (base[0] * tone + rib * 0.10 + vein * 0.05 + rim * 0.12 + (speck - 0.5) * 0.02).max(0.),
        (base[1] * tone + rib * 0.10 + vein * 0.06 + rim * 0.02).max(0.),
        (base[2] * tone + rib * 0.04 + vein * 0.015 - rim * 0.02).max(0.),
    ];
    let h = -rib * 0.35 - vein * 0.18 + fine * 0.03;
    (h, rgb, 0.42 + 0.12 * fine - rib * 0.08, alpha)
}
fn texture_set(leaf: bool, seed: u64) -> Result<[image::RgbaImage; 3], Box<dyn std::error::Error>> {
    const N: u32 = 1024;
    let mut heights = Vec::with_capacity((N * N) as usize);
    let mut colours = Vec::with_capacity((N * N * 4) as usize);
    let mut surface = Vec::with_capacity((N * N * 4) as usize);
    let srgb = |c: f32| (c.clamp(0., 1.).powf(1. / 2.2) * 255.) as u8;
    for y in 0..N {
        for x in 0..N {
            let u = (x as f32 + 0.5) / N as f32;
            let v = (y as f32 + 0.5) / N as f32;
            let (h, rgb, rough, alpha) = if leaf {
                leaf_texel(u, v, seed)
            } else {
                bark_texel(u, v, seed)
            };
            heights.push(h);
            colours.extend([
                srgb(rgb[0]),
                srgb(rgb[1]),
                srgb(rgb[2]),
                (alpha * 255.) as u8,
            ]);
            surface.extend([255, (rough.clamp(0.05, 1.) * 255.) as u8, 0, 255]);
        }
    }
    // Spread leaf colour into the cut-out margin so filtered edges stay green.
    if leaf {
        for _ in 0..6 {
            let snapshot = colours.clone();
            for y in 0..N {
                for x in 0..N {
                    let i = ((y * N + x) * 4) as usize;
                    if snapshot[i + 3] != 0 {
                        continue;
                    }
                    for (dx, dy) in [(1, 0), (N - 1, 0), (0, 1), (0, N - 1)] {
                        let j = ((((y + dy) % N) * N + (x + dx) % N) * 4) as usize;
                        if snapshot[j + 3] != 0 || snapshot[j] != 0 || snapshot[j + 1] != 0 {
                            colours[i..i + 3].copy_from_slice(&snapshot[j..j + 3]);
                            break;
                        }
                    }
                }
            }
        }
    }
    let sample = |x: u32, y: u32| heights[((y % N) * N + x % N) as usize];
    let strength = if leaf { 2.5 } else { 6. };
    let mut normals = Vec::with_capacity(colours.len());
    for y in 0..N {
        for x in 0..N {
            let dx = sample(x + 1, y) - sample(x + N - 1, y);
            let dy = sample(x, y + 1) - sample(x, y + N - 1);
            let n = Vec3::new(-dx * strength, -dy * strength, 1.).normalize();
            normals.extend([
                ((n.x * 0.5 + 0.5) * 255.) as u8,
                ((n.y * 0.5 + 0.5) * 255.) as u8,
                ((n.z * 0.5 + 0.5) * 255.) as u8,
                255,
            ]);
        }
    }
    Ok([
        image::RgbaImage::from_raw(N, N, colours).ok_or("colour image")?,
        image::RgbaImage::from_raw(N, N, normals).ok_or("normal image")?,
        image::RgbaImage::from_raw(N, N, surface).ok_or("roughness image")?,
    ])
}
fn plant_images(seed: u64) -> Result<Vec<image::RgbaImage>, Box<dyn std::error::Error>> {
    let [bark, bark_normal, bark_mr] = texture_set(false, seed)?;
    let [leaf, leaf_normal, leaf_mr] = texture_set(true, seed ^ 0x1eaf)?;
    Ok(vec![bark, bark_normal, leaf, leaf_normal, bark_mr, leaf_mr])
}
/// Native render-ready plant data; no intermediate image encoding, filesystem or import pass.
#[derive(Debug)]
pub struct PlantMeshAsset {
    pub geometry: Vec<voxy_render::archive_scene::ArchiveMeshSource>,
    pub materials: Vec<voxy_render::archive_scene::ArchiveMaterialSource>,
    pub images: Vec<voxy_render::ImageAsset>,
}
/// Generate instanced-world geometry and material pixels directly.
/// # Errors
/// Rejects invalid dimensions and image/geometry limits.
pub fn generate_world_plant_asset(
    options: PlantOptions,
) -> Result<PlantMeshAsset, Box<dyn std::error::Error>> {
    use voxy_render::archive_scene::*;
    let g = grow(options, true)?;
    // Ground dressing stays within the root's supporting voxel footprint.
    let mut grass = Surface::default();
    let mut rng = Random(options.seed ^ 0x006a_7261_7373);
    for _ in 0..180 {
        let angle = rng.next() * TAU;
        let radial = rng.next().sqrt() * 0.35;
        let base = Vec3::new(angle.cos() * radial, 0.004, angle.sin() * radial);
        let bend_angle = rng.next() * TAU;
        let side = Vec3::new(bend_angle.cos(), 0., bend_angle.sin());
        let bend = Vec3::new(-side.z, 0., side.x) * (0.025 + rng.next() * 0.06);
        let height = 0.07 + rng.next() * 0.15;
        let width = 0.003 + rng.next() * 0.004;
        let start = grass.positions.len() as u32;
        for row in 0..=3 {
            let t = row as f32 / 3.;
            let centre = base + Vec3::Y * height * t + bend * t * t;
            for col in 0..=1 {
                // Sample the opaque midrib band of the first leaf-atlas cell.
                grass.vertex(
                    centre + side * ((col as f32 * 2. - 1.) * width * (1. - t)),
                    [0.24 + col as f32 * 0.02, 0.12 + t * 0.3],
                );
            }
        }
        for row in 0..3 {
            let a = start + row * 2;
            grass.quad(a, a + 2, a + 3, a + 1);
        }
    }
    grass.normals();
    let mut geometry = Vec::new();
    let mut materials = Vec::new();
    for (id, surface) in [g.wood, g.leaves, grass].into_iter().enumerate() {
        let tangents = surface.tangents();
        let vertices = (0..surface.positions.len())
            .map(|i| ArchiveVertex {
                position: surface.positions[i],
                normal: surface.normals[i],
                tangent: tangents[i],
                uv: surface.uv[i],
            })
            .collect();
        geometry.push(ArchiveMeshSource {
            vertices,
            indices: surface.indices,
            instances: vec![ArchiveInstance::new(glam::Mat4::IDENTITY)],
            material: id,
            center: Vec3::ZERO,
        });
        materials.push(ArchiveMaterialSource {
            parameters: ArchiveMaterial {
                diffuse: if id == 2 {
                    [0.72, 0.9, 0.56, 1.]
                } else {
                    [1.; 4]
                },
                specular_gloss: [1.; 4],
                emissive_normal: [0., 0., 0., if id == 0 { 0.8 } else { 0.35 }],
                surface: [0., 1., 0., if id == 1 { 0.5 } else { -1. }],
                workflow: [1., 1., 0., if id > 0 { 0.65 } else { 0. }],
            },
            images: [
                Some(id.min(1) * 2),
                Some(4 + id.min(1)),
                Some(id.min(1) * 2 + 1),
                None,
            ],
            double_sided: id > 0,
        });
    }
    let images = plant_images(options.seed)?
        .into_iter()
        .map(|p| {
            voxy_render::ImageAsset::from_rgba(
                p.width(),
                p.height(),
                p.into_raw(),
                Default::default(),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PlantMeshAsset {
        geometry,
        materials,
        images,
    })
}

/// Generate a bounded, deterministic GLB with separate bark/leaf PBR materials,
/// normal maps, metric geometry and double-sided leaf surfaces.
/// # Errors
/// Rejects nonfinite or unsupported heights, and image/serialization failures.
pub fn generate_plant_glb(options: PlantOptions) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    generate(options, true, false)
}

/// Simplified asset for Voxy's current model editor, which rejects normal-map materials.
/// The full PBR asset should be retained separately.
/// # Errors
/// Same input and encoding errors as `generate_plant_glb`.
pub fn generate_plant_editor_glb(
    options: PlantOptions,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    generate(options, false, false)
}
/// Bounded world instance detail, with the same materials and botanical growth rules.
/// # Errors
/// Same validation and encoding errors as `generate_plant_glb`.
pub fn generate_world_plant_glb(
    options: PlantOptions,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    generate(options, true, true)
}
fn grow(options: PlantOptions, world_detail: bool) -> Result<Growth, Box<dyn std::error::Error>> {
    if !options.height.is_finite() || !(0.25..=30.).contains(&options.height) {
        return Err("plant height must be 0.25..=30 metres".into());
    }
    let mut g = Growth {
        world_detail,
        wood: Surface::default(),
        leaves: Surface::default(),
        rng: Random(options.seed),
        scale: options.height,
    };
    let height = options.height;
    let internode = (height * 0.024).clamp(0.025, 0.35);
    let mut model = GrowthModel {
        internode,
        lambda: 0.54,
        alpha: 3.0,
        xi: 1.0,
        eta: 0.35,
        tropism: Vec3::new(0., -0.04, 0.),
        influence: 10.,
        kill: 1.5,
        cone_cos: (50f32).to_radians().cos(),
        branch_angle: 48f32.to_radians(),
        rotation: 137.5f32.to_radians(),
        years: 32,
        max_metamers: if world_detail { 4500 } else { 7000 },
        shed_ratio: 0.08,
        murray: 2.3,
        tip_radius: (height * 0.0011).clamp(0.002, 0.009),
    };
    let mut rng = Random(options.seed ^ 0x5eed_7ee5);
    let (skeleton, flare, twig) = match options.kind {
        PlantKind::Tree => {
            let lean = Vec3::new(rng.signed(), 0., rng.signed()) * 0.05;
            let crown_base = height * (0.28 + 0.08 * rng.next());
            let centre = Vec3::new(lean.x * height, height * 0.6, lean.z * height);
            let radii =
                Vec3::new(height * 0.36, height * 0.4, height * 0.36) * (0.9 + 0.2 * rng.next());
            let markers = if world_detail { 5000 } else { 7000 };
            (
                grow_skeleton(
                    &model,
                    &[(Vec3::ZERO, Vec3::Y + lean)],
                    crown_base,
                    (centre, radii),
                    markers,
                    &mut rng,
                ),
                Some(height * 0.05),
                model.tip_radius * 2.2,
            )
        }
        PlantKind::Shrub => {
            model.lambda = 0.5;
            model.branch_angle = 40f32.to_radians();
            model.tropism = Vec3::new(0., 0.02, 0.);
            let stems: Vec<_> = (0..6)
                .map(|i| {
                    let a = i as f32 * 2.399963 + rng.signed() * 0.3;
                    (
                        Vec3::new(a.cos(), 0., a.sin()) * internode * 0.6,
                        Vec3::new(a.cos() * 0.35, 1., a.sin() * 0.35),
                    )
                })
                .collect();
            let centre = Vec3::new(0., height * 0.55, 0.);
            let radii = Vec3::new(height * 0.55, height * 0.5, height * 0.55);
            (
                grow_skeleton(
                    &model,
                    &stems,
                    internode * 2.5,
                    (centre, radii),
                    1800,
                    &mut rng,
                ),
                None,
                model.tip_radius * 2.5,
            )
        }
    };
    g.build(&skeleton, flare, twig);
    g.wood.normals();
    // Bend leaf normals towards the crown hull so the canopy shades as one volume,
    // as foliage does at any distance beyond a few leaf lengths.
    let (low, high) = g.leaves.positions.iter().fold(
        (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
        |(l, h), p| (l.min(Vec3::from_array(*p)), h.max(Vec3::from_array(*p))),
    );
    if low.is_finite() {
        let centre = (low + high) * 0.5;
        let extent = ((high - low) * 0.5).max(Vec3::splat(0.05));
        // Wind every blade so its front face looks out of the crown.
        let inward: Vec<bool> = g
            .leaves
            .positions
            .chunks_exact(LEAF_VERTICES)
            .map(|leaf| {
                let p = |i: usize| Vec3::from_array(leaf[i]);
                // Same handedness as `quad(a, a + 3, a + 4, a + 1)`.
                let n = (p(3) - p(0)).cross(p(2) - p(0));
                n.dot((p(4) - centre) / extent) < 0.
            })
            .collect();
        for t in g.leaves.indices.chunks_exact_mut(3) {
            if inward[t[0] as usize / LEAF_VERTICES] {
                t.swap(1, 2);
            }
        }
    }
    g.leaves.normals();
    if low.is_finite() {
        let centre = (low + high) * 0.5;
        let extent = ((high - low) * 0.5).max(Vec3::splat(0.05));
        for (n, p) in g.leaves.normals.iter_mut().zip(&g.leaves.positions) {
            let hull = ((Vec3::from_array(*p) - centre) / extent).normalize_or(Vec3::Y);
            *n = Vec3::from_array(*n)
                .lerp(hull, 0.55)
                .normalize_or(Vec3::Y)
                .to_array();
        }
    }
    Ok(g)
}
fn generate(
    options: PlantOptions,
    full_material: bool,
    world_detail: bool,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let g = grow(options, world_detail)?;
    let mut binary = Vec::new();
    let mut views = Vec::new();
    let mut accessors = Vec::new();
    let mut primitives = Vec::new();
    fn append(binary: &mut Vec<u8>, views: &mut Vec<serde_json::Value>, bytes: &[u8]) -> usize {
        while !binary.len().is_multiple_of(4) {
            binary.push(0);
        }
        let index = views.len();
        views.push(json!({"buffer":0,"byteOffset":binary.len(),"byteLength":bytes.len()}));
        binary.extend_from_slice(bytes);
        index
    }
    for (material, surface) in [g.wood, g.leaves].into_iter().enumerate() {
        let mut attrs = serde_json::Map::new();
        for (name, width, values) in [
            (
                "POSITION",
                3,
                surface
                    .positions
                    .iter()
                    .flatten()
                    .copied()
                    .collect::<Vec<_>>(),
            ),
            (
                "NORMAL",
                3,
                surface.normals.iter().flatten().copied().collect(),
            ),
            (
                "TANGENT",
                4,
                surface.tangents().iter().flatten().copied().collect(),
            ),
            (
                "TEXCOORD_0",
                2,
                surface.uv.iter().flatten().copied().collect(),
            ),
        ] {
            let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
            let view = append(&mut binary, &mut views, &bytes);
            let mut accessor = json!({"bufferView":view,"componentType":5126,"count":values.len()/width,"type":match width { 4=>"VEC4",3=>"VEC3",_=>"VEC2" }});
            if name == "POSITION" {
                let min = surface
                    .positions
                    .iter()
                    .fold(Vec3::splat(f32::INFINITY), |a, p| {
                        a.min(Vec3::from_array(*p))
                    });
                let max = surface
                    .positions
                    .iter()
                    .fold(Vec3::splat(f32::NEG_INFINITY), |a, p| {
                        a.max(Vec3::from_array(*p))
                    });
                accessor["min"] = json!(min.to_array());
                accessor["max"] = json!(max.to_array());
            }
            attrs.insert(name.into(), json!(accessors.len()));
            accessors.push(accessor);
        }
        let bytes: Vec<u8> = surface
            .indices
            .iter()
            .flat_map(|i| i.to_le_bytes())
            .collect();
        let view = append(&mut binary, &mut views, &bytes);
        let indices = accessors.len();
        accessors.push(json!({"bufferView":view,"componentType":5125,"count":surface.indices.len(),"type":"SCALAR"}));
        primitives.push(json!({"attributes":attrs,"indices":indices,"material":material}));
    }
    let mut images = Vec::new();
    for pixels in plant_images(options.seed)? {
        // Interchange GLBs stay inside default model image budgets; native assets keep full maps.
        let pixels =
            image::imageops::resize(&pixels, 512, 512, image::imageops::FilterType::Triangle);
        let mut output = Cursor::new(Vec::new());
        pixels.write_to(&mut output, image::ImageFormat::Png)?;
        let view = append(&mut binary, &mut views, &output.into_inner());
        images.push(json!({"bufferView":view,"mimeType":"image/png"}));
    }
    let mut root = json!({"asset":{"version":"2.0","generator":"Voxy native botanical generator v2"},"scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"mesh":0,"name":format!("{:?} seed {}",options.kind,options.seed)}],"meshes":[{"primitives":primitives}],"buffers":[{"byteLength":binary.len()}],"bufferViews":views,"accessors":accessors,"images":images,"samplers":[{"magFilter":9729,"minFilter":9987,"wrapS":10497,"wrapT":10497}],"textures":[{"source":0,"sampler":0},{"source":1,"sampler":0},{"source":2,"sampler":0},{"source":3,"sampler":0},{"source":4,"sampler":0},{"source":5,"sampler":0}],"materials":[{"name":"Bark","pbrMetallicRoughness":{"baseColorTexture":{"index":0},"metallicFactor":0.,"roughnessFactor":1.,"metallicRoughnessTexture":{"index":4}},"normalTexture":{"index":1,"scale":0.8}},{"name":"Leaves","doubleSided":true,"alphaMode":"MASK","alphaCutoff":0.5,"pbrMetallicRoughness":{"baseColorTexture":{"index":2},"metallicFactor":0.,"roughnessFactor":1.,"metallicRoughnessTexture":{"index":5}},"normalTexture":{"index":3,"scale":0.35}}]});
    if !full_material {
        for material in root["materials"].as_array_mut().expect("material array") {
            material
                .as_object_mut()
                .expect("material object")
                .remove("normalTexture");
            for key in ["alphaMode", "alphaCutoff"] {
                material
                    .as_object_mut()
                    .expect("material object")
                    .remove(key);
            }
            material["pbrMetallicRoughness"]
                .as_object_mut()
                .expect("PBR material")
                .remove("metallicRoughnessTexture");
        }
    }
    let mut json = serde_json::to_vec(&root)?;
    while json.len() % 4 != 0 {
        json.push(b' ');
    }
    while binary.len() % 4 != 0 {
        binary.push(0);
    }
    let mut output = Vec::new();
    for n in [
        0x46546c67,
        2,
        (12 + 8 + json.len() + 8 + binary.len()) as u32,
        json.len() as u32,
        0x4e4f534a,
    ] {
        output.extend(n.to_le_bytes());
    }
    output.extend(json);
    output.extend((binary.len() as u32).to_le_bytes());
    output.extend(0x004e4942u32.to_le_bytes());
    output.extend(binary);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn botanical_assets_round_trip_with_textures_and_finite_geometry() {
        for kind in [PlantKind::Tree, PlantKind::Shrub] {
            let options = PlantOptions {
                kind,
                seed: 7,
                height: 3.,
            };
            let bytes = generate_plant_glb(options).unwrap();
            assert_eq!(bytes, generate_plant_glb(options).unwrap());
            let gltf = gltf::Gltf::from_slice(&bytes).unwrap();
            assert_eq!(gltf.document.materials().count(), 2);
            assert_eq!(gltf.document.images().count(), 6);
            let container = gltf::binary::Glb::from_slice(&bytes).unwrap();
            let archive = crate::SceneArchive::decode(
                &container.json,
                vec![container.bin.unwrap().into_owned().into()],
                Default::default(),
            )
            .unwrap();
            for primitive in &archive.meshes[0] {
                assert!(primitive.positions.iter().flatten().all(|v| v.is_finite()));
                assert!(
                    primitive
                        .normals
                        .as_ref()
                        .unwrap()
                        .iter()
                        .all(|n| (Vec3::from_array(*n).length() - 1.).abs() < 1e-4)
                );
                for ids in primitive.indices.chunks_exact(3) {
                    let p = |i: u32| Vec3::from_array(primitive.positions[i as usize]);
                    assert!(
                        (p(ids[1]) - p(ids[0]))
                            .cross(p(ids[2]) - p(ids[0]))
                            .length_squared()
                            > 1e-20
                    );
                }
            }

            let asset = crate::ModelAsset::parse(
                &generate_plant_editor_glb(options).unwrap(),
                &[],
                crate::ModelLimits::default(),
            )
            .unwrap();
            let meshes = asset.scene_meshes(&asset.skeleton.bind_pose()).unwrap();
            assert_eq!(meshes.len(), 2);
            assert!(meshes.iter().all(|m| m.vertices().len() > 1000));
            assert_ne!(
                bytes,
                generate_plant_glb(PlantOptions { seed: 8, ..options }).unwrap()
            );
        }
    }
    #[test]
    fn native_world_asset_has_real_surface_maps_and_grounded_grass() {
        let asset = generate_world_plant_asset(PlantOptions {
            height: 5.,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(asset.geometry.len(), 3);
        assert_eq!(asset.materials.len(), 3);
        assert_eq!(asset.images.len(), 6);
        for image in &asset.images[4..6] {
            let mut low = 255;
            let mut high = 0;
            for p in image.rgba().chunks_exact(4) {
                assert_eq!(p[2], 0);
                low = low.min(p[1]);
                high = high.max(p[1]);
            }
            assert!(high - low > 20, "roughness must vary across the surface");
        }
        for mesh in &asset.geometry {
            assert!(
                mesh.vertices.len() < 200_000,
                "world detail must remain bounded"
            );
            for v in &mesh.vertices {
                let n = Vec3::from_array(v.normal);
                let t = Vec3::from_array([v.tangent[0], v.tangent[1], v.tangent[2]]);
                assert!(Vec3::from_array(v.position).is_finite());
                assert!((n.length() - 1.).abs() < 1e-4 && (t.length() - 1.).abs() < 1e-4);
                assert!(n.dot(t).abs() < 1e-3);
            }
        }
        for v in asset.geometry[2]
            .vertices
            .iter()
            .filter(|v| v.uv[1] == 0.12)
        {
            assert!((v.position[1] - 0.004).abs() < 1e-5);
            assert!((v.position[0] * v.position[0] + v.position[2] * v.position[2]).sqrt() < 0.36);
        }
    }

    #[test]
    fn self_organising_skeleton_obeys_murray_sheds_and_keeps_a_clear_bole() {
        let model = GrowthModel {
            internode: 0.15,
            lambda: 0.54,
            alpha: 3.,
            xi: 1.,
            eta: 0.35,
            tropism: Vec3::new(0., -0.04, 0.),
            influence: 10.,
            kill: 1.5,
            cone_cos: 50f32.to_radians().cos(),
            branch_angle: 48f32.to_radians(),
            rotation: 137.5f32.to_radians(),
            years: 32,
            max_metamers: 7000,
            shed_ratio: 0.08,
            murray: 2.3,
            tip_radius: 0.007,
        };
        let crown_base = 1.8;
        let sk = grow_skeleton(
            &model,
            &[(Vec3::ZERO, Vec3::Y)],
            crown_base,
            (Vec3::new(0., 3.6, 0.), Vec3::new(2.2, 2.4, 2.2)),
            7000,
            &mut Random(3),
        );
        let alive: Vec<_> = sk.metamers.iter().filter(|m| m.alive).collect();
        assert!(alive.len() > 1000, "crown must fill out: {}", alive.len());
        assert!(
            sk.metamers.iter().any(|m| !m.alive),
            "shading must shed some limbs"
        );
        for m in &alive {
            let sum: f32 = [m.main, m.lateral]
                .into_iter()
                .flatten()
                .map(|c| sk.metamers[c].radius.powf(model.murray))
                .sum();
            if sum > 0. {
                assert!((m.radius.powf(model.murray) - sum).abs() <= sum * 1e-3);
            }
            if m.tip.y < crown_base - 0.2
                && m.parent
                    .is_none_or(|p| sk.metamers[p].tip.y < crown_base - 0.2)
            {
                assert!(m.lateral.is_none(), "bole below the crown stays clear");
            }
        }
        assert!(sk.metamers[0].radius > model.tip_radius * 10.);
    }

    #[test]
    fn rejects_invalid_dimensions() {
        for height in [f32::NAN, f32::INFINITY, 0., 31.] {
            assert!(
                generate_plant_glb(PlantOptions {
                    height,
                    ..Default::default()
                })
                .is_err()
            );
        }
    }
}
