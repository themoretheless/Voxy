# Fixed-tick animation event delivery

SceneSimulation now offers advance_scoped_with_animation_events, sharing the
existing clock/command/lifecycle/pose path through one private tick runner.
Scheduled systems return accepted occurrences; dispatch occurs after the serial
plan and before pose capture/next fixed behavior update. This avoids borrowing
the BehaviorRunner inside arbitrary scheduled scene capabilities.

The editor uses this path and drains its adopted animation owner state at each
successful system boundary. Earlier accepted systems remain committed under the
existing SimulationStepError policy; their events are dispatched before a later
system failure is propagated. A failing system must not return unaccepted events.
No frame-end dispatch remains in the editor. Legacy callers remain compatible.

Regression: three catch-up ticks in one display frame; event callbacks increment
an object transform and subsequent fixed hooks observe 0,1,2. Late system failure
delivers the earlier accepted occurrence, retains mutation, and reports zero
fully completed ticks. Zero-tick frame invokes no systems or occurrences.
The initial new test omitted required SystemSpec.phase fields; fixture corrected.

Validation: editor release library suite including all ignored GPU controls:
174 passed. Scene release library suite: 68 passed. Zero failed/ignored tests.
git diff --check passed. All changes local/uncommitted.

Still pending: marker editing UX and partial collision receipt boundary
qualification. No broad engine parity/hardware/physics completion claim.
