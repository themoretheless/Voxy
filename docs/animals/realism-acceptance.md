# Animal realism acceptance

Status: **not accepted**. Scope remains six Voxy-authored breeds, convincing fur,
anatomical movement and physical interaction, verified in the game window.
External models and photographs are references only; no external geometry is
incorporated. Passing automated checks does not replace visual acceptance.

## Required evidence

For Labrador, German Shepherd, Siberian Husky, domestic shorthair, British
shorthair and Maine Coon, inspect native close-up front, lateral and three-quarter
views in neutral material and with the finished coat. Compare anatomical
silhouettes and landmarks against actually viewed photographic references.
All feet and the complete tail must be in frame. Preserve rejected evidence.

- Correct species and breed silhouette: skull, nasal bridge, jaw, ribcage,
  scapula, pelvis, elbow, carpus, stifle, hock, digital paws, ears and tail.
- Continuous skin with no primitive intersections, cuts, unattached features,
  inflated muscle balls, cone ears, toy eyes or generic canine feline face.
- Coat follows regional growth, has plausible short/plush/long breed variation,
  undercoat coverage and silhouette guard hairs. No isolated dark/white points,
  glitter, floating roots, bald transitions or visible polygon ribbons.
- Walk, turn, stop, follow, land and make terrain/player/companion contact.
  Check paw placement, planted stance, knee/hock articulation and root stability.
  Physics must remain stable across repeated landings and abrupt changes in gait.
- Fur remains attached through blended joints; guides collide with the skin and
  relevant environment. Explicitly distinguish rendered followers from simulated
  guides. No claim of full coat self-collision from a local sphere-only test.
- Native run completes its whole inspection sequence and reports grounding,
  movement and simulated guides. Early closure is incomplete, even with exit 0.
- Focused model, rig, skinning and physics checks pass on the final source.

## Current blockers to acceptance (not external blockers)

Anatomy is still simplified; full face/paw/ear detail is missing. The coat has
been visibly speckled. Body contact uses a conservative root collider, and fur
includes voxel/body free-particle contacts and sparse local skin colliders, but
still lacks segment contact and coat self-collision.
These are remaining implementation work, not reasons to mark the goal complete.
