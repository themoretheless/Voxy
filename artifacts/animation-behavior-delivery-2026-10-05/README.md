# Animation events delivered to scene behaviors

Behavior adds a default animation_event(scene, owner, name, unwrapped_phase)
hook, preserving existing implementers. BehaviorRunner delivers in attachment
order to active behaviors of the specific owner, starts once, and rechecks
activation/validity after hooks. Invalid event data is ignored before lifecycle
changes. Removed owners retire through the existing lifecycle owner.

SceneSimulation validates its scene/stopped state, dispatches the callback and
refreshes current render poses after hook mutations. Editor drains adopted
animation queues after scheduled frame execution, including earlier committed
owner updates on later system failure, before propagating the system error.
Callbacks are post-frame; they are not interleaved between individual catch-up
ticks. Panics follow the existing unrecovered Behavior policy.

Regression through actual editor save/load/play attaches a Rust listener and
asserts one owner-tagged event, no residual queue and no duplicate over subsequent
ticks. Scene lifecycle regression covers start ordering, owner filtering,
nonfinite event rejection, callback-driven deactivation and owner deletion.

Validation: editor release library suite including all normally ignored GPU
controls: 174 passed; scene release library suite: 67 passed. No failed/ignored
tests. git diff --check passed. Work is local and uncommitted.

Remaining: marker authoring UX, per-fixed-tick delivery and explicit partial
collision event boundary qualification, integration with authored script/audio
actions. Full engine parity/research/hardware/physics objective remains active.
