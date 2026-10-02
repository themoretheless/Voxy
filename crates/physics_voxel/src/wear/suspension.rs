//! Atomic voxel recession with owned emitted grains and liquid feedback.
use super::VoxelWear;
use physics::wear::{WearSuspension, WearSuspensionInput, WearSuspensionStep};
use voxy_core::split_voxel;
use voxy_world::{
    BlockStateId, CommitReceipt, EditSource, EditTxn, Sample, VoxelView, VoxelWrite, World,
};

#[derive(Clone, Debug)]
pub struct VoxelWearSuspension {
    binding: VoxelWear,
    suspension: WearSuspension,
}
#[derive(Clone, Debug)]
pub struct VoxelWearSuspensionStep {
    pub physics: WearSuspensionStep,
    pub receipt: Option<CommitReceipt>,
}
impl VoxelWearSuspension {
    /// Bind a fresh voxel inventory to prescribed contact and an owned liquid.
    /// Forces/work are supplied by the contact caller; this is not body dynamics.
    pub fn new(
        binding: VoxelWear,
        contact: physics::friction::Material,
        liquid: physics::liquid::Liquid,
        surface_energy_j_m2: f64,
    ) -> Result<Self, &'static str> {
        if binding.removed || binding.debris_mass_kg() != 0. {
            return Err("suspension requires fresh voxel wear inventory");
        }
        let suspension = WearSuspension::new(
            binding.layer.clone(),
            contact,
            binding.material,
            liquid,
            surface_energy_j_m2,
        )?;
        Ok(Self {
            binding,
            suspension,
        })
    }
    /// Geometry and collision consumers use this same accepted recession state.
    pub fn binding(&self) -> &VoxelWear {
        &self.binding
    }
    pub fn suspension(&self) -> &WearSuspension {
        &self.suspension
    }
    /// Compute all physical candidates before committing an exhausted voxel.
    /// Any failure leaves both owned inventories and world unchanged.
    pub fn step_contact(
        &mut self,
        world: &mut World,
        input: WearSuspensionInput<'_>,
    ) -> Result<VoxelWearSuspensionStep, &'static str> {
        if input.normal != self.binding.face.map(f64::from) {
            return Err("contact normal disagrees with receding voxel face");
        }
        let chunk = split_voxel(self.binding.pos).0;
        if self.binding.removed
            || world.sample(self.binding.pos) != Sample::Loaded(self.binding.block)
            || world.chunk(chunk).ok_or("wear chunk unavailable")?.revision != self.binding.revision
        {
            return Err("stale wear suspension binding");
        }
        let mut next = self.clone();
        let physics = next.suspension.step_contact(input)?;
        next.binding.layer = next.suspension.layer().clone();
        let receipt = if physics.wear.exhausted {
            Some(
                world
                    .commit(EditTxn {
                        source: EditSource::Simulation,
                        expected: vec![(chunk, self.binding.revision)],
                        writes: vec![VoxelWrite {
                            pos: self.binding.pos,
                            block: BlockStateId::AIR,
                        }],
                    })
                    .map_err(|_| "wear suspension world commit failed")?,
            )
        } else {
            None
        };
        next.binding.removed = physics.wear.exhausted;
        *self = next;
        Ok(VoxelWearSuspensionStep { physics, receipt })
    }
}
