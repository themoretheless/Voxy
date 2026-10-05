# Per-region contact domains

TissueDemo can bind one prescribed contact owner per continuum region via bind_contact_surfaces and sample the corresponding animated surfaces with the bone palette via advance_with_palette_and_surfaces. The existing shared-surface API delegates to this same path. Region count must match exactly; no zip truncation is admitted. Binding and frame advancement remain transactional.

Each region retains its own immutable mask and owner identity throughout initial subdivisions and recursive retries. A mask-owner swap in a later region rejects after earlier regions have been staged without publishing any state. Energy receipts retain each region's own initial contact baseline and mesh work.

The dedicated regression uses four independently authored domains over the same source geometry. It checks count rejection, correct pose/domain publication and full-state rollback on domain swaps or wrong sample counts. Contact-filter regression: 15 passed, 2 existing ignored tests, 36.89 seconds. Full tissue regression is recorded separately.

Actual CesiumMan attachment masks still require authoring and verification. This change enables distinct domains; it does not claim the current overlapping character setup is admitted or visually qualified.
