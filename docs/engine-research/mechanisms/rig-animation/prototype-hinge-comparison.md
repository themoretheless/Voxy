# Hand prototype: hinge-gradient isolation

The isolated 54-bone v4 candidate is not the application rig. Its geometry,
weights, joint transforms and corrective limits were held fixed. Only the
unsigned central hinge difference was replaced with the wrapped signed
finite difference used by the application. Both hands were evaluated at grasp
fractions 0.5, 0.75 and 1.0 without a grasp object.

At full grasp the extra crease changes from 1.2972131 to 1.297163 radians,
while minimum hinge triangle area changes from 0.5598111 to 0.56064206 of rest.
Both violate the intended healthy deformation: crease exceeds the 0.30-radian
limit. The signed-gradient fix addresses the exact-pi numerical trap but does
not repair this prototype. This experiment does not identify which joint axes,
weight distribution or geometric constraints are responsible.

The two diagnostic tests passed finite-value assertions; they deliberately do
not claim production acceptance. Full 121-frame object-contact regressions and
native visual validation are separate gates. No prototype was promoted into
the active application.

Reproduce with `tools/prepare_hand_hinge_comparison.py <candidate-v4.rs>`;
then run the printed Cargo command. The supplied candidate must contain the
original corrective block and resolve its own asset/weight includes. Generated
comparison projects live under ignored `target/`. Source and weight hashes are
recorded in `prototype-hinge-comparison.json`. The observed test log is
`/tmp/voxy-hand-prototype-hinge-comparison.log`.

Next: isolate thumb base articulation from the corrective solver, inspect
uncorrected hinge collapse and influence changes over the continuous pose
range, and verify any repair against both hands and contact shapes without
relaxing thresholds.
