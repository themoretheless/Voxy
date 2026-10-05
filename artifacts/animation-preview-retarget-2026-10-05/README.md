# Retarget authoring preview qualification

Real imported GLB source and separately imported target with renamed pelvis,
offset bind translation (3,4,0), and no target animations. Source clip `move`
at phase 0.5 transfers through a 90-degree translation basis and scale 2.
Independent expected target local translation (3,6,0) and target inverse-bind
palette are checked; this does not use the preview implementation as its oracle.

Changing extraction to target Y at the same phase invalidates the old frame and
rebuilds in-place translation (3,4,0), rather than toggling the stale preview off.
A missing target bone rejects preparation without overwriting the accepted
frame; restoring the profile restores access to that same immutable frame.
Scene unchanged by preview, no gameplay clock advancement or queued events.

Release editor suite including real GPU controls: 179 passed, 0 failed/ignored.
git diff --check passed. Local uncommitted verification only. No native retarget
pixel proof, multi-joint mannequin or realistic tissue calibration claimed.
Overall engine/research/hardware/physics objective remains incomplete and active.
