# Thumb articulation isolation

The v4 prototype was evaluated without the corrective solver, body motion or
contact objects. The test samples 121 closure values, both hands, and resets
specified local joint rotations before calculating the same dual-quaternion
finger palette. Weight bindings and rest geometry stay fixed.

| Articulation | Extra crease, radians | Minimum area/rest |
| --- | ---: | ---: |
| Full thumb | 1.89117 | 0.05449 |
| Thumb disabled | 0.00012 | 1.00000 |
| Base only | 1.79757 | 0.09502 |
| Middle only | 0.57299 | 0.52878 |
| Tip only | 0.02782 | 0.93776 |
| Base flexion only, 0.20 rad | 0.15125 | 0.83460 |
| Base spread only, 0.70 rad | 1.99479 | 0.08948 |
| Base twist only, 0.20 rad | 0.15053 | 0.87630 |
| Base flexion plus spread | 1.68572 | 0.24582 |

The largest isolated failure is base spread. It is reproduced before surface
correction or object collision; replacing the hinge derivative cannot fix its
origin. The middle joint also independently exceeds the crease limit. This
localizes the defect to articulation/binding compatibility, not solely the
corrective solver; it does not yet distinguish incorrect pivot, axis, weights
or insufficient mesh topology.

At the full-thumb worst hinge, source vertices 19641, 19642, 19427 and 19640
mix the right index metacarpal (20), thumb base (39) and thumb middle (40).
Vertex 19640 also carries 0.012194127 on bone 0. A foreign anatomical influence
is a binding audit target, but the base-only experiment freezes that metacarpal,
so its own animation is not required to trigger the collapse. Simply reducing
spread to pass a threshold would remove intended thumb opposition and is not
an accepted repair.

The isolated tests only assert finite measures. They passed but the measured
prototype fails production deformation criteria. No candidate was promoted.

Use `tools/prepare_hand_hinge_comparison.py <candidate-v4.rs>` to generate the
comparison crate; run Cargo with filters `raw_thumb_articulation_isolation` and
`raw_thumb_base_channel_isolation`. Complete metrics, hinge indices and log
paths are in the accompanying JSON. These checks complement continuous contact
and visual acceptance rather than replace them.

Next repair must preserve thumb opposition, remove anatomically invalid
influences, and verify a suitable thumb metacarpal/pivot and smooth weights
against the full pose range and all contact objects on both hands.
