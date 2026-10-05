# Prescribed translating contact plane

The existing boundary-node penalty contact can now translate within the same frozen-history Verlet step that drives FEM supports. `step_with_moving_plane` and `step_viscoelastic_with_moving_plane` accept the next offset of the installed plane; normal and stiffness stay fixed. Constitutive forces, contact forces, diagnostics and both external work terms use one integration path.

The work of the plane is independently computed from contact-potential offset derivatives at the two endpoints times prescribed offset displacement. It is returned as `DrivenSupportStep::plane_work_j`, separate from pin actuator work and viscous heat. The defect guard subtracts both actuator contributions. Positions, velocities and plane offset publish only after geometry and work admission; the surrounding viscoelastic transaction additionally owns material histories and heat.

Evidence:
- 28 focused physics tests passed: 6 moving-plane tests, 7 support tests, 8 viscoelastic-inertia tests, 3 fixed-contact tests, 4 finite-inertia tests.
- All 15 tissue tests passed after the shared solver change, including complete imported clip, external sampling/clock rollback, thermal conduction and child-substep rollback/recovery. Imported step counts and refinement depths remain identical to prior evidence.
- Analytic stationary-node plane work is 0.12 J for three nodes with a 2 mm penetration and 20000 N/m penalty. Pin work and viscous heat remain zero.
- Co-moving prescribed pins and plane cancel opposite contact actuator work; only pin kinetic actuator work remains.
- Galilean impact fixture checks all positions and velocities against a fixed-plane reference for 6000 steps, along with independent per-step plane work = plane speed times body momentum change. Actuator work totals 0.101928197409 J.
- Moving-plane energy/work envelope falls from 2.9025462174e-6 J at 20 microseconds to 7.2561544651e-7 J at 10 microseconds over the same interval.
- A coarse contact-activation trial rejects under strict energy tolerance without modifying the owner. NaN, infinity and overflowing next offsets preserve plane, node state, histories and heat; successful subsequent relaxation recovers.
- git diff --check clean.

Limits: this is a translating infinite frictionless penalty plane prescribed by an external actuator, not a finite-mass obstacle or rotating triangle mesh. Stiffness-dependent penetration remains. The API is not yet wired to the imported-character pads; no new native/GPU contact visualization or realtime claim follows from these solver tests. Character triangle contact, rotation, friction, finite obstacle momentum and coupled character/tissue contact remain required work.
