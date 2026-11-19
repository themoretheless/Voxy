//! Map the event loop's nodes to the bounded reaction network.
use super::{Error, LiquidBodyWorld, Node, RigidSupportPoint, RigidSupportPointMotion};
use crate::contact::{ContactBody, ContactWrench, NetworkSupport};
use std::collections::BTreeSet;

pub(super) struct SupportParticipants {
    pub states: Vec<ContactBody>,
    pub radii: Vec<f64>,
    particle_nodes: Vec<usize>,
    particle_count: usize,
}

impl SupportParticipants {
    pub fn new(
        nodes: &[Node],
        particle_count: usize,
        contacted: &BTreeSet<usize>,
        radius: f64,
    ) -> Self {
        let particle_nodes: Vec<_> = contacted.iter().copied().collect();
        let states = particle_nodes
            .iter()
            .map(|&node| nodes[node].contact())
            .chain(nodes[particle_count..].iter().map(|node| node.contact()))
            .collect();
        Self {
            states,
            radii: vec![radius; particle_nodes.len()],
            particle_nodes,
            particle_count,
        }
    }

    pub fn node(&self, slot: usize) -> usize {
        self.particle_nodes
            .get(slot)
            .copied()
            .unwrap_or_else(|| self.particle_count + slot - self.particle_nodes.len())
    }

    pub fn slot(&self, node: usize) -> Option<usize> {
        if node >= self.particle_count {
            Some(self.radii.len() + node - self.particle_count)
        } else {
            self.particle_nodes.binary_search(&node).ok()
        }
    }

    pub fn point_motion(
        &self,
        world: &impl LiquidBodyWorld,
        loads: &[ContactWrench],
        rates: &[ContactWrench],
        support: NetworkSupport,
        geometry: RigidSupportPoint,
    ) -> Result<Option<RigidSupportPointMotion>, Error> {
        let motion = if let Some(&radius) = self.radii.get(support.first) {
            let second = support.second.ok_or(Error::InvalidCollision)?;
            world.particle_body_support_point_motion(
                &self.states[support.first],
                radius,
                self.node(second) - self.particle_count,
                &self.states[second],
                geometry,
                [loads[support.first], loads[second]],
                [rates[support.first], rates[second]],
            )?
        } else {
            world.rigid_support_point_motion(
                &self.states[self.radii.len()..],
                &loads[self.radii.len()..],
                &rates[self.radii.len()..],
                self.rigid_support(support),
                geometry,
            )?
        };
        motion.map(RigidSupportPointMotion::validate).transpose()
    }

    pub fn normal_acceleration(
        &self,
        world: &impl LiquidBodyWorld,
        loads: &[ContactWrench],
        support: NetworkSupport,
        geometry: RigidSupportPoint,
    ) -> Result<Option<[f64; 3]>, Error> {
        if let Some(&radius) = self.radii.get(support.first) {
            let second = support.second.ok_or(Error::InvalidCollision)?;
            world.particle_body_support_normal_acceleration(
                &self.states[support.first],
                radius,
                self.node(second) - self.particle_count,
                &self.states[second],
                geometry,
                [loads[support.first], loads[second]],
            )
        } else {
            world.rigid_support_normal_acceleration(
                &self.states[self.radii.len()..],
                &loads[self.radii.len()..],
                self.rigid_support(support),
                geometry,
            )
        }
    }

    fn rigid_support(&self, support: NetworkSupport) -> NetworkSupport {
        NetworkSupport {
            first: support.first - self.radii.len(),
            second: support.second.map(|slot| slot - self.radii.len()),
            ..support
        }
    }
}
