//! One automatically selected frictionless proximity impact.
use super::QuadraticDynamics;
use crate::plasticity::mesh::{QuadraticClosestLimits, QuadraticFace, QuadraticNormalImpact};
#[derive(Clone, Debug)]
pub struct QuadraticAutomaticImpact {
    pub source_node: usize,
    pub target_face: QuadraticFace,
    pub gap_m: f64,
    pub impact: QuadraticNormalImpact,
}
impl QuadraticDynamics {
    /// Pick the nearest approaching exposed-node/face pair on distinct accepted
    /// components and apply one frictionless capture-distance impact. Not a
    /// simultaneous-contact solve or continuous collision step. Search failure
    /// and query-budget exhaustion leave all state unchanged.
    pub fn impact_nearest_fragment_node(
        &mut self,
        capture_distance_m: f64,
        restitution: f64,
        limits: QuadraticClosestLimits,
        max_queries: usize,
    ) -> Result<Option<QuadraticAutomaticImpact>, &'static str> {
        if !restitution.is_finite() || !(0. ..=1.).contains(&restitution) {
            return Err("invalid automatic fragment restitution");
        }
        let candidates =
            self.detect_fragment_node_contacts(capture_distance_m, limits, max_queries)?;
        let mut selected: Option<(
            QuadraticDetectedFragmentContact,
            super::QuadraticFragmentImpactConstraint,
        )> = None;
        for (detected, c) in candidates {
            let relative: f64 = (0..self.velocities.len())
                .map(|i| {
                    (c.first[i] - c.second[i])
                        * (0..3)
                            .map(|a| c.normal[a] * self.velocities[i][a])
                            .sum::<f64>()
                })
                .sum();
            if !relative.is_finite() {
                return Err("automatic fragment impact velocity overflow");
            }
            if relative < 0.
                && selected
                    .as_ref()
                    .is_none_or(|(old, _)| detected.gap_m < old.gap_m)
            {
                selected = Some((detected, c));
            }
        }
        let Some((detected, c)) = selected else {
            return Ok(None);
        };
        let impact = self.impact_fragments_along_gap(
            &c.first,
            &c.second,
            c.normal,
            restitution,
            capture_distance_m,
        )?;
        Ok(Some(QuadraticAutomaticImpact {
            source_node: detected.source_node,
            target_face: detected.target_face,
            gap_m: detected.gap_m,
            impact,
        }))
    }
    fn detect_fragment_node_contacts(
        &self,
        capture_distance_m: f64,
        limits: QuadraticClosestLimits,
        max_queries: usize,
    ) -> Result<
        Vec<(
            QuadraticDetectedFragmentContact,
            super::QuadraticFragmentImpactConstraint,
        )>,
        &'static str,
    > {
        if !capture_distance_m.is_finite()
            || capture_distance_m <= 0.
            || max_queries == 0
            || self.free_nodes.len() != self.velocities.len()
            || !limits.distance_tolerance_m.is_finite()
            || limits.distance_tolerance_m <= 0.
            || limits.max_patches == 0
            || limits.max_patches > 65536
            || limits.max_depth > 24
        {
            return Err("invalid automatic fragment impact");
        }
        let n = self.velocities.len();
        let mut owner = vec![usize::MAX; n];
        for (i, nodes) in self.body.fragment_nodes().iter().enumerate() {
            for &node in nodes {
                owner[node] = i;
            }
        }
        let faces = self.body.exposed_faces_at(&self.body.positions)?;
        let nodes: std::collections::BTreeSet<_> = faces.iter().flat_map(|f| f.nodes).collect();
        let count = nodes
            .iter()
            .map(|&node| {
                faces
                    .iter()
                    .filter(|f| owner[node] != owner[f.nodes[0]])
                    .count()
            })
            .sum::<usize>();
        if count > max_queries {
            return Err("automatic fragment impact query limit");
        }
        let mut collected = Vec::new();
        for node in nodes {
            let mut nearest = std::collections::BTreeMap::<
                usize,
                (
                    QuadraticDetectedFragmentContact,
                    super::QuadraticFragmentImpactConstraint,
                ),
            >::new();
            for face in &faces {
                if owner[node] == owner[face.nodes[0]] {
                    continue;
                }
                let closest =
                    face.closest_point_at(&self.body.positions, self.body.positions[node], limits)?;
                if !closest.converged {
                    return Err("automatic fragment impact unresolved projection");
                }
                let mut first = vec![0.; n];
                first[node] = 1.;
                let mut second = vec![0.; n];
                for (&i, &w) in face.nodes.iter().zip(&closest.shape_weights) {
                    second[i] = w;
                }
                let gap: [f64; 3] = std::array::from_fn(|a| {
                    face.nodes
                        .iter()
                        .zip(&closest.shape_weights)
                        .map(|(&i, &w)| {
                            -w * (self.body.positions[i][a] - self.body.positions[node][a])
                        })
                        .sum()
                });
                let distance = gap[0].hypot(gap[1]).hypot(gap[2]);
                if !distance.is_finite() {
                    return Err("automatic fragment impact gap overflow");
                }
                if distance > capture_distance_m
                    || nearest
                        .get(&owner[face.nodes[0]])
                        .is_some_and(|(c, _)| distance >= c.gap_m)
                {
                    continue;
                }
                let normal = if distance > 0. {
                    gap.map(|x| x / distance)
                } else {
                    closest.normal
                };
                nearest.insert(
                    owner[face.nodes[0]],
                    (
                        QuadraticDetectedFragmentContact {
                            source_node: node,
                            source_component: owner[node],
                            target_component: owner[face.nodes[0]],
                            target_face: *face,
                            gap_m: distance,
                        },
                        super::QuadraticFragmentImpactConstraint {
                            first,
                            second,
                            normal,
                        },
                    ),
                );
            }
            collected.extend(nearest.into_values());
        }
        Ok(collected)
    }
}

#[derive(Clone, Debug)]
pub struct QuadraticDetectedFragmentContact {
    pub source_node: usize,
    pub source_component: usize,
    pub target_component: usize,
    pub target_face: QuadraticFace,
    pub gap_m: f64,
}
#[derive(Clone, Debug)]
pub struct QuadraticAutomaticMultiImpact {
    pub contacts: Vec<QuadraticDetectedFragmentContact>,
    pub impact: crate::plasticity::mesh::QuadraticMultiImpact,
}
impl QuadraticDynamics {
    /// Collect nearest target surface per source node and target component,
    /// including separating constraints, then commit one coupled inelastic solve.
    /// More than 64 contacts rejects rather than silently dropping constraints.
    pub fn impact_fragment_nodes(
        &mut self,
        capture_distance_m: f64,
        limits: QuadraticClosestLimits,
        max_queries: usize,
        velocity_tolerance_m_s: f64,
        max_sweeps: usize,
    ) -> Result<Option<QuadraticAutomaticMultiImpact>, &'static str> {
        self.impact_fragment_nodes_with_restitution(
            capture_distance_m,
            0.,
            limits,
            max_queries,
            velocity_tolerance_m_s,
            max_sweeps,
        )
    }
    /// Automatic manifold with coupled Newton restitution and energy acceptance.
    /// Energetically incompatible targets reject atomically.
    pub fn impact_fragment_nodes_with_restitution(
        &mut self,
        capture_distance_m: f64,
        restitution: f64,
        limits: QuadraticClosestLimits,
        max_queries: usize,
        velocity_tolerance_m_s: f64,
        max_sweeps: usize,
    ) -> Result<Option<QuadraticAutomaticMultiImpact>, &'static str> {
        if !restitution.is_finite() || !(0. ..=1.).contains(&restitution) {
            return Err("invalid automatic coupled restitution");
        }
        if !velocity_tolerance_m_s.is_finite()
            || velocity_tolerance_m_s <= 0.
            || max_sweeps == 0
            || max_sweeps > 65536
        {
            return Err("invalid automatic coupled impact limits");
        }
        let detected =
            self.detect_fragment_node_contacts(capture_distance_m, limits, max_queries)?;
        if detected.is_empty() {
            return Ok(None);
        }
        if detected.len() > 64 {
            return Err("automatic coupled impact contact limit");
        }
        let (contacts, constraints): (Vec<_>, Vec<_>) = detected.into_iter().unzip();
        let impact = self.impact_fragment_contacts_with_restitution(
            &constraints,
            capture_distance_m,
            restitution,
            velocity_tolerance_m_s,
            max_sweeps,
        )?;
        Ok(Some(QuadraticAutomaticMultiImpact { contacts, impact }))
    }
}

impl super::FiniteQuadraticDynamics {
    pub fn impact_fragment_nodes(
        &mut self,
        capture_distance_m: f64,
        limits: QuadraticClosestLimits,
        max_queries: usize,
        velocity_tolerance_m_s: f64,
        max_sweeps: usize,
    ) -> Result<Option<QuadraticAutomaticMultiImpact>, &'static str> {
        self.inner.impact_fragment_nodes(
            capture_distance_m,
            limits,
            max_queries,
            velocity_tolerance_m_s,
            max_sweeps,
        )
    }
    pub fn impact_fragment_nodes_with_restitution(
        &mut self,
        capture_distance_m: f64,
        restitution: f64,
        limits: QuadraticClosestLimits,
        max_queries: usize,
        velocity_tolerance_m_s: f64,
        max_sweeps: usize,
    ) -> Result<Option<QuadraticAutomaticMultiImpact>, &'static str> {
        self.inner.impact_fragment_nodes_with_restitution(
            capture_distance_m,
            restitution,
            limits,
            max_queries,
            velocity_tolerance_m_s,
            max_sweeps,
        )
    }
}
