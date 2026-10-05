# Affine contact precision and neutral skeletal tissue preview

Closest-feature deltas now use relative affine coordinates with compensated sums and product rounding. Two analytic oblique-distance tests preserve binary-representable small separations under large common translations. A public contact regression checks translated energy and feature forces. 82 library tests pass (2 manual benchmarks ignored), 23 existing prescribed contact tests plus the new translated test, 8 viscoelastic, 52 other contact/geometry tests, and 19 tissue tests pass.

The imported character close-up now honors --cesium --close-up. The non-contact render completed 480 simulation steps over two seconds on Apple M4 Max / Metal, producing 41 frames. close-up.png was visually inspected; motion-close-up.gif uses the actual rendered frames. The blue FEM regions are separate diagnostic deformable volumes attached to the imported skeleton, not integrated skin or calibrated anatomy. The close-up intentionally crops the head and lower legs.

The complete contact render still rejects step 65 at 0.270833333 s with implicit contact nonlinear nonconvergence. The prefix contact frames are not a full clip. No contact/energy admission budget was loosened. Additional optional iteration diagnostics now report residual impulse work alongside its budget; the diagnostic rerun also terminated at step 65. At dt=2.5431315104166666e-7 s, its final main-solve trace shows a maximum direction of 1.1894286896427828e-17 m and residual impulse work 3.762915575229903e-7 J versus 3.8146972656250003e-11 J. This motivates retaining small trajectory displacements and endpoint consistency; it does not prove interpolation alone is sufficient.

Changes remain local and uncommitted.
