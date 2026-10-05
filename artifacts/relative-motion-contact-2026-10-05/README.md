# Relative motion bounds and imported step-65 diagnosis

Conservative triangle advancement now bounds relative feature speed by the maximum of the nine cross-triangle vertex-velocity differences, with a numerical margin. Feature velocities are convex combinations of vertex velocities, so this remains a conservative bound and removes common rigid translation. An analytic constant-gap translation regression passes for culled and exact paths; a swept crossing remains rejected. The old sum of absolute speeds could exhaust its finite advancement budget under common translation.

78 library tests passed (2 manual benchmarks ignored), 21 prescribed-contact, 8 viscoelastic and 18 tissue tests passed. CCD failure tracing is optional and read-only; it reports reason, sample time, gap, speed bound and source face identity. Nonlinear trace distinguishes internal gap, volume and external contact admission.

The full imported render still rejects step 65 (0.270833333 s); this change does not fix that particular failure. All 48 failed nonlinear trials pass internal gap and volume but fail external CCD. Fatal dt is 2.5431315104166666e-7 s, weighted residual 2.3724e-4 J versus 3.0518e-14 J threshold. Body face [10,7,11], obstacle source face 1308: computed closed gap -6.0715e-18 m at path time 0.9999999995951657, after 35 advancement steps. This is near endpoint/coordinate resolution, not exhaustion of the iteration budget. The trace does not prove a physically resolvable penetration.

Next: CCD-location-driven endpoint quadrature refinement before nonlinear search failure, keeping strict trajectory/work admission. Existing adaptive work refinement happens only after equilibrium and cannot address this search failure. Full clip remains unqualified. All changes are local and unpushed.
