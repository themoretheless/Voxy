//! Editor document/graph limits are independent of subsystem owner budgets.
pub(crate) const OBJECTS: usize = 4096;
pub(crate) const DOCUMENT_BYTES: usize = 4 * 1024 * 1024;
pub(crate) const HISTORY_BYTES: usize = 64 * 1024 * 1024;

/// Project validation and editor composition must admit the same prefab topology.
pub(crate) const PREFABS: voxy_scene::PrefabLimits = voxy_scene::PrefabLimits {
    max_objects: OBJECTS,
    max_instances: 128,
    max_depth: 16,
};

/// Shared Play and checkpoint admission contract; changing it requires migration.
pub(crate) const SIMULATION: voxy_scene::SimulationLimits = voxy_scene::SimulationLimits {
    fixed_step: 1.0 / 60.0,
    max_steps: 8,
    max_behaviors: 128,
    max_commands: 128,
};
