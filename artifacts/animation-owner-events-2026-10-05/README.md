# Animation owner event delivery foundation

AnimationRuntime registers markers against the owner's current animation source
and drains (NodeId, ClipEventOccurrence) pairs. Owner ordering is deterministic;
per-owner authored occurrence order is retained. Drain is one-shot. Candidate
runtime queues remain isolated until adoption; removal and reset discard them.

Regression verifies rejected candidate isolation, successful retry, identity,
no duplicate drain, no duplicate next tick, invalid clip and removed owner.
Initial test used dt=0.25 exceeding the existing fixed-step admission bound;
it was corrected to an admitted dt=0.0625 without changing that contract.

Validation: release voxy_editor library suite, including all normally ignored
GPU controls: 173 passed, zero failed/ignored. git diff --check passed.

This is an internal foundation. App callback integration, authored marker
persistence, hot-reload marker rebinding and source crossfade event policy are
pending. No broad engine parity or production-completion claim. Local changes.
