# Collision-limited animation marker qualification

Extended the actual CharacterPhysics certified original-source fade test with
an active static box. The character has 0.05 m half-extents, a wall at x=0.13 m
with 0.05 m x half-extent, and authored unit translation over one second. The
0.0625 s requested tick is interrupted by collision before its endpoint.

Markers at phases 0.01 and 0.05 bracket the accepted prefix. The real receipt is
incomplete and the accepted contact phase is asserted strictly between markers.
Only the early marker is present in the candidate queue. A later palette failure
preserves both scene/physics and the original event queue. Successful retry emits
the early marker once; advancing accepted playback later emits only the deferred
marker. That final continuation uses ordinary playback, not another wall sweep.

No algorithm change was needed. This adds direct physical boundary evidence to
the event implementation. Final release editor suite with all ignored GPU
controls enabled: 175 passed, zero failed/ignored. git diff --check passed.

This is one collision geometry and translation fade; rotational/multiple-contact
boundary coverage and marker authoring UX remain pending. Overall engine goal
remains active. Local uncommitted changes only.
