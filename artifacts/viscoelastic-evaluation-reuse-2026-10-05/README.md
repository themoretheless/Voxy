# Reuse exact-pose force and energy evaluations

Velocity Verlet previously evaluated the constitutive body twice at each
endpoint: once for diagnostics and again for the force gradient. The endpoint
force evaluation now supplies potential/contact energy to the shared diagnostic
assembly. Kinetic energy, momentum and angular momentum are evaluated after the
final half kick using current velocities. There is no persistent cache and no
reuse across a pose or material-history change.

`viscoelastic_step_bench` runs a 32-cell rounded body for 1000 prescribed-support
steps, with gravity, Maxwell relaxation and all path/work/heat guards enabled.
Three repetitions before and after have identical printed endpoint pose,
velocity, potential/kinetic energy, heat, work and accumulated numerical defect.
Median time for this fixture drops from 0.037293 to 0.032940 seconds (11.67%).
This is a local CPU fixture measurement, not an editor FPS, full-scene, GPU or
hardware-wide benchmark. Before/after timings and source hashes are in report.json.

The full engine parity, research, hardware, destruction and fluid goal remains
active. Existing rendered artifacts preserve the source state of their own run.
