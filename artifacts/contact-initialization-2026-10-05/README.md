# Skeletal supports and soft tissue initialization

The implicit solver now checks the complete initial contact trajectory, including crossings that finite Gauss samples miss. Invalid initial trajectories cannot be published. The force law, contact checks and energy admission remain authoritative.

Final working solver physics: 93 library, 8 static L-BFGS, 29 prescribed contact and 8 viscoelastic tests passed; 3 manual library tests ignored. Tissue integration: 20 passed. `git diff --check` passed.

`neutral-motion.gif` contains 241 freshly rendered frames of the existing neutral 12-second demonstration, with walking, jumping and settling. It is a coarse technical mannequin with separate volumetric specimens, not anatomical tissue integrated into character skin. This preview does not qualify imported-character contact.

Failed search-metric experiments are preserved for diagnosis. Generalized secant scaling and a frozen contact metric rejected the imported clip at step 65; the elastic metric rejected at step 76. They are excluded from the working solver. The elastic source and qualified operator tests are archived under `elastic-experiment/`.

The support transport experiment passed analytic rotation/scale/translation and invalid-input tests, but the full imported clip rejected at step 65 (245.63 seconds). It is excluded from the working solver and archived under `support-transport-experiment/`. Final working solver passed the 68-step imported contact regression (143.29 seconds). Full 480-step qualification remains outstanding. The previously qualified baseline reached step 130 and rejected step 131.
