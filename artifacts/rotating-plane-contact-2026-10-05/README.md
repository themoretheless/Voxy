# Rotating prescribed contact plane

The existing inertial/viscoelastic contact step now accepts an installed plane's next normal and offset through step_with_plane_motion and step_viscoelastic_with_plane_motion. The same Verlet force, geometry, external-work and Maxwell/heat admission path is reused. Fixed-normal translation APIs delegate to this path. Stiffness changes and ambiguous antipodal rotations reject atomically.

The shortest normal rotation uses a stable cross-product norm and atan2 angle. Contact angular-gradient work is independent of observed potential changes and retained separately from offset-gradient work. Both contribute to plane_work_j; pinned-node work and viscous heat remain distinct. A lost work component above tolerance rejects rather than silently disappearing in the sum.

A rotating plane may hide a contact between apparently open endpoints. Boundary-node gap curvature and endpoint-derivative bounds conservatively admit open/monotone intervals and reject unresolved ones. This uses spherical normal motion, linear drift and linear signed-offset motion. Rejection is intentionally conservative; callers must subdivide. This is not a complete triangle-mesh CCD implementation.

Evidence:
- 33 physics tests passed: 11 moving/rotating plane, 7 support, 8 viscoelastic inertia, 3 fixed contact and 4 finite inertia tests.
- All 15 tissue regressions passed, including the full imported rig clip and transactional sampling/history/heat rollback. Imported step receipts are identical to prior evidence.
- Fixed-node rotating-contact angular work converges with 2000/4000 steps: work 87.7609767397/87.7609788322 J, independently measured energy defects 2.8338353388e-6/7.4134973715e-7 J.
- Combined translation/rotation has matching contact energy, actuator components and defects after a proper 90-degree world rotation.
- Endpoint-open fixture has node-zero endpoint gaps +0.03 m but mid-arc gap -0.02 m. It rejects without publishing any state.
- Antipodal and changed-stiffness rejection preserve plane/history/thermal state; subsequent viscoelastic rotating contact succeeds without incorrectly turning actuator work into heat.
- A 1e-200 normal rotation yields representable -4e-200 J torque work even when potential change rounds away. Strict 1e-210 J tolerance rejects; the separate 1e-190 J trial retains its nonzero work/defect receipt.
- git diff --check clean.

Limits: prescribed infinite frictionless penalty plane, without finite obstacle momentum. Character triangle contact, friction, calibrated material/anatomical data, imported-pad coupling and realtime/native contact visualization remain outstanding. No admission tolerances or material parameters were weakened.
