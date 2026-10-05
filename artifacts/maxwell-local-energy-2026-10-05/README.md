# Independent local energy accounting for held-pose Maxwell relaxation

During each relaxation half-step the pose and every external/equilibrium
parameter are fixed. Only reference Maxwell memory changes, so ΔU_total equals
the volume-weighted sum of ΔU_Maxwell. The check now evaluates that stored branch
energy before and after relaxation independently of the heat receipt. It does
not derive stored-energy change by negating the returned heat.

This replaces four full-body energy/force assemblies per split step with local
branch-energy evaluations. The mechanical stage still admits complete forces,
potentials, support work and the positive-volume path. Existing energy tolerances,
subdivision controller and rollback boundaries are unchanged.

The new regression places a small relaxing branch over a large fixed gravity
potential. Subtracting rounded global potentials demonstrably fails the declared
heat tolerance; local stored-energy loss agrees with released heat and a zero-
gravity control. Analytic shear relaxation and time refinement remain qualified.

`trace.csv` and `run.txt` cover the same full 12-second skeletal cycle as the
preceding clone-elision run. Source hashes, trace comparison, test results and
local wall-time observations are recorded in report.json. These timings are
single observations with other processes active, not controlled editor FPS proof.
No new GPU render or full-goal completion is claimed.
