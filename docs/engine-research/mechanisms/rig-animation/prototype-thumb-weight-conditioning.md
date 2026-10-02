# Rejected thumb weight diffusion

The existing 54-bone prototype was measured at 121 no-object closure values on
both hands. The test keeps joint articulation and source positions unchanged,
remaps non-thumb influences within the thumb region to the hand-local identity,
and diffuses four normalized nonnegative weights over mesh adjacency. Distal
vertices with tip influence above 0.5 and boundary vertices are pinned. The
corrective solver is excluded to isolate skin binding.

The no-diffusion remap changes maximum position by only 0.0000001013 metres;
crease remains 1.89107 radians and area remains 0.05449 of rest. Therefore the
foreign metacarpal influence is not sufficient to explain this no-object fold.
Bone 0 is explicitly converted to identity in the hand-local palette; its use
here is an identity/palm slot, not application of animated body-root rotation.

Four diffusion rounds increase crease to 2.96833 radians with 2.23 mm maximum
shape change. At 16, 64 and 256 rounds crease remains 2.77534, 2.70239 and
2.81840 radians respectively; the last introduces 15.24 mm maximum displacement.
Tip displacement is zero in every variant, but the thumb web gets worse.
None of these candidates is accepted or promoted to the application.

This invalidates uniform adjacency diffusion as this prototype's repair. It
also shows that preserving the fingertip alone is insufficient acceptance.
The test asserts finite metrics only; its passing status does not mean healthy
rig deformation. See the accompanying JSON for exact measured values.

Generate the isolated crate with `tools/prepare_hand_hinge_comparison.py` and
run its `thumb_weight_conditioning_diagnostic` filter. The original candidate
and source weights remain unchanged. Next investigation must distinguish base
pivot/axis/topology incompatibility and consider anatomical thumb metacarpal
articulation while preserving opposition and contact behavior.
