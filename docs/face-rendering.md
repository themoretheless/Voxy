# Facial rendering verification

Facial hairs attach to barycentric points of actual body triangles. The reference groom contains 600 brow hairs, 96 upper lashes and 220 fuzz strands. Upper lashes have four curved spans, tapered tips, varying lengths and an outward fan; the curl follows eyelid closure. Thin strands use four-sample raster coverage in hardware-ray inspection; the native female viewer also enables MSAA4.

The procedural complexion atlas contains albedo and roughness/sebum/microheight tiles. Intrinsic complexion pigment includes smooth multi-scale mottling, regional warmth and slight roughness variation, independently of light visibility. Freckles use full-cell jitter with neighbor evaluation, varied elliptical shapes, pigment strength and clustered density; their boundaries remain continuous across the sampling grid. The female shader shades skin with GGX highlights, separate eye pigmentation and per-fragment corneal reflections of finite studio light disks, and tagged oral materials. Negative oral U coordinates select an intrinsic vertex albedo and roughness stored in V; the fragment normal supplies diffuse lighting and the GGX lobe. Ray visibility modulates highlights without making opaque tissues transparent.

Eye globes receive two spherical midpoint-refinement levels before animation (16 times the original eye triangles), reducing coarse vertex-visibility shadow facets. This adds geometry and ray-query cost and does not replace per-pixel visibility. Eye shading receives radial globe normals rotated with gaze and the head, rather than triangle-corner normals. Both hemispheres share the eye material encoding; posterior UVs remain in the sclera annulus. Eye key visibility controls reflected catchlights. Corneal shading refracts the camera ray with an index of 1.376 toward an approximate iris plane recessed 3 mm, recovering the ocular frame from globe-normal and UV derivatives. Front and oblique inspection renders validate the shader path; separate anatomical corneal geometry and calibrated iris depth remain incomplete. Iris pigment uses irregular radial fibers, a varying collarette and local dark crypts.

The mouth contains two sixteen-tooth arches, gingival collars, a continuous tongue dorsum, palate, uvula, buccal lining and a recessed pharynx. Rounded rectangular incisors have a broad cutting edge; canines taper. The lower lip, dental arch and tongue follow jaw displacement. The lip-contact curve includes a central notch, raised shoulders and lowered corners. Both clip boundaries intersect the same curve, and the split lip seam uses the same jaw weights as surrounding lower-lip vertices to prevent folding the seam under the adjacent surface.

Hardware-ray inspection on supported devices:

```sh
cargo run -p voxy_ray_probe -- --experimental --face
cargo run -p voxy_ray_probe -- --experimental --face --mouth
cargo run -p voxy_ray_probe -- --experimental --face --mouth --oral-closeup
cargo run -p voxy_ray_probe -- --experimental --face --eye-closeup
```

The three-pose PNG strips are written to `/tmp/voxy-face-ray-preview.png`, `/tmp/voxy-face-ray-mouth.png` `/tmp/voxy-oral-closeup.png` and `/tmp/voxy-eye-closeup.png`. Hardware queries use the complete character triangles: sixteen samples of a finite key-light disk, four fill-light samples and four local ambient visibility directions. Batched dispatches share one acceleration structure. This inspection path currently applies visibility at vertices; it does not prove per-pixel or interactive ray tracing in the native viewer.

Remaining quality limits include lip-corner silhouette, sparse/coarse facial hairs, coarse tooth and soft-tissue tessellation, and missing calibrated subsurface light transport. Procedural pigment and passing geometry tests do not establish photorealism or absence of uncanny-valley effects. Inspect portraits, eyelid closure, close oral views and motion before declaring the lifelike-face goal achieved.

For a controlled jaw-opening comparison, use:

```sh
cargo run -p voxy_ray_probe -- --experimental --face --oral-closeup --oral-transition --tongue
```

`--oral-transition` holds animation time at zero and overrides jaw opening to 0, 0.45 and 0.9 in the three frames. With `--tongue`, lift and forward targets remain 0.7 and 0.65, gated by jaw opening through the existing tongue deformation. Output is `/tmp/voxy-oral-transition.png`. This mode changes inspection poses, not oral geometry or collision handling.

To inspect tongue articulation independently of jaw motion:

```sh
cargo run -p voxy_ray_probe -- --experimental --face --oral-closeup --oral-above --tongue-transition
```

This holds animation time at zero and jaw opening at 0.9. Tongue lift targets are -0.4, 0, 0.7; forward targets are 0, 0.35, 0.8. The existing jaw-gated motion and crown-envelope retreat remain active. `--oral-above` raises the oral closeup camera to inspect the dorsum; it requires `--oral-closeup`. The strip is saved separately at `/tmp/voxy-tongue-transition.png`.

The above-view tongue strip completed on Metal (867845 triangles and 11336736 ray segments per pose) and was viewed. The tongue is visible with modest changes of anterior position/dorsum at fixed jaw opening. It remains overly smooth and flattened; upper inner-lip edge jaggedness is more visible in this oblique view. This verifies the inspection mode and visible articulation, not anatomical realism or complete tooth/lip collision handling. Keep this view as a regression reference when refining tongue surface and lip attachment.

Tongue surface detail: dorsal vertices now carry reserved material U coordinates in -1.4..-1.2 and normalized bind X/Z coordinates. The tongue shader generates irregularly jittered papilla-like height bumps (0.9 mm cell scale, 70 micrometre peak) and perturbs the surface normal using screen-space height derivatives. GGX roughness is 0.38. The pattern follows the bind coordinates through tongue motion; these are shader normals, not extra collision/ray geometry or anatomically calibrated papillae. Ventral vertices retain the existing oral material; tag interpolation near the transition and fine-detail aliasing still require inspection.

The Metal tongue-transition strip completed and was viewed at `/tmp/voxy-tongue-papillae.png`, with unchanged triangle/ray counts. Fine surface variation is now visible; gross flattened shape and jagged lip contact remain unresolved. Eleven existing feature tests initially passed; the material whitelist test failed because it did not include tongue coordinate tags. Updated that whitelist to check the explicit range and independently reran the topology/attachment test successfully. No geometric collision behavior changed.

Tongue dorsum volume: added a smooth central dome with up to 1.5 mm lift at full jaw opening, multiplied by dorsal height and elliptical X/Z falloffs. Neutral jaw adds zero; side edges and tip fade to zero, retaining the median groove. Existing tongue articulation and tooth-envelope retreat run afterward. Twelve feature tests passed. The three-position above-view Metal strip completed (867845 triangles, 11336736 ray segments per pose) and was viewed at `/tmp/voxy-tongue-dome.png`; prior `/tmp/voxy-tongue-before-dome.png`. Retained the modest volume increase with no obvious new defect in this view. The result still resembles a simplified smooth tongue; the small central lift is not anatomical calibration, tissue-volume conservation or a complete lip/tooth contact proof. Jagged lip edge remains.

Independent brow inspection:

```sh
cargo run -p voxy_ray_probe -- --experimental --face --skin-closeup --brow-transition
```

`FacePreview::sample_brow` validates time/camera/brow inputs and overrides the facial expression with brow -1, 0 and 1 at fixed animation time zero. Blink, smile, jaw and tongue motion remain neutral for comparison. Output `/tmp/voxy-brow-transition.png`. Lowered brows now also produce two local glabellar depressions with up to 0.35 mm nominal depth before region/front falloff; raised brows retain the existing horizontal forehead crease model. These are artistic procedural deformation masks, not muscle/skin mechanics.

Nine facial tests passed. Both the baseline inspection and the new glabellar variant completed three Metal poses (867842 triangles, 11336736 ray segments per pose); viewed `/tmp/voxy-brow-before-glabella.png` and `/tmp/voxy-brow-glabella.png`. Brow height changes are visible; the new glabellar relief is subtle in this light. Retained the missing frown-specific deformation as an intermediate addition. Horizontal forehead lines remain weak, the hair/brow shapes remain stylized, and independent muscle coupling and dynamic wrinkle material response are incomplete.

Frown motion refinement: negative brow values now draw the inner brow region toward the midline, with 1.5 mm nominal lateral displacement multiplied by the existing region mask and a smooth inner-to-outer taper. Neutral/positive brow values add no lateral shift. Nine existing facial tests passed; a separately rebuilt new test verifies inward movement on both sides, unchanged neutral points, unchanged outer-point X and no crossing of the midline. The Metal brow strip completed and was viewed at `/tmp/voxy-brow-inward-frown.png` (867842 triangles and 11336736 ray segments per pose). Retained the subtle additional frown movement. This remains a procedural expression rather than independently controlled anatomical muscles; wrinkle relief is weak and photorealism is not established.

### Skin finish isolation

```sh
cargo run -p voxy_ray_probe -- --experimental --face --skin-closeup --skin-finish-transition
```

The three panels hold time/expression, geometry, pigment and microheight fixed. Left overrides material-tile roughness/sebum to 0.8/0, middle uses the current atlas unchanged, right overrides them to 0.28/0.8. Linear values are encoded into the sRGB texture just like the existing atlas. Only the material tile's R/G channels change, preserving pigment tile, microheight and alpha. This is an inspection override, not new constructor controls or calibrated physical oil-film thickness.

The Metal render completed and `/tmp/voxy-skin-finish-transition.png` was viewed. All panels report identical 867842 triangles, 11336736 ray segments and 8609184 occluded segments. Matte/default/oily highlights are visibly distinct while pores and pigmentation remain. The oily forehead highlight looks excessively point-like around microheight features; filtering/normal-scale behavior needs further work. This confirms material response in the offscreen inspection path, not photorealistic skin transport or interactive per-pixel ray tracing.

Finite-source skin highlights: replaced the single-direction skin GGX sum with a 16-direction disk average. Disk radius 0.22, square-root radial sampling and golden-angle sequence match the key-light visibility probe. Both the base and oil lobes are evaluated per direction, retaining their original weights and roughness. This avoids shading the finite softbox as a directional point; it is quadrature, not a new roughness value or pore-blurring filter. The coarse averaged vertex visibility still multiplies the integrated highlight and does not supply individually correlated per-direction fragment visibility.

The shader compiled and the three-panel Metal skin-finish render completed with unchanged geometry/ray counts. Viewed `/tmp/voxy-skin-area-highlight.png` against `/tmp/voxy-skin-finish-point-highlight.png`: the oily highlight is visibly softer and fewer microheight features become sharp white specks, while matte/oily response remains distinct. Retained the finite-source highlight model. It adds sixteen paired GGX evaluations for atlas skin fragments; native frame-time impact has not been measured. Pore filtering, diffuse skin transport and fully per-pixel ray visibility remain incomplete.

Pixel-footprint microheight filtering: each finite-difference height sample now averages four quarter-pixel offsets using explicit texture gradients. Difference spacing is the larger of one atlas texel and the half-extent of the screen-pixel UV footprint; physical gradient denominators use that same adaptive spacing. Pigment and geometry are unchanged. This is a four-tap footprint approximation rather than a full mip hierarchy or anisotropic integration; it does not establish temporal stability or conserve unresolved normal-distribution variance.

Rendered matched portrait-distance skin-finish strips before/after the filter. Both completed on Metal with 867842 triangles, 11336736 ray segments and 8598888 occluded segments in each panel. Viewed `/tmp/voxy-skin-portrait-before-filter.png` and `/tmp/voxy-skin-portrait-footprint.png`: fine specular speckling is reduced, especially on the oily skin, while the material differences remain. Retained the filter. Closeup detail retention, camera-motion flicker and native performance remain to be verified; the overall model still has visible eye/lid, mouth and hair limitations.

Closeup retention check: ran the skin-finish comparison again with `--skin-closeup` after filtering. All three Metal panels completed at 867842 triangles, 11336736 ray segments and 8609184 occluded segments. Viewed `/tmp/voxy-skin-footprint-closeup.png`: pores remain visible, matte/default/oily responses remain separate, and the oily source highlight stays soft. No obvious exterior pigment blur was seen; the shader still samples pigment independently of the filtered height. This establishes visual detail retention for this fixed camera/preset only. Camera-motion flicker, other distances/grazing angles and native frame-time impact are still unverified.

The mode completed on Metal and its strip was viewed: 867842/867844/867844 triangles and 11336736 ray segments per pose. The sequence confirms visible aperture changes while the upper dental arch stays in place. It also exposes unresolved oral quality: pointed canines, an angular band along the inner lower lip, a thin colored sliver at closed lip contact, and a tongue largely hidden behind the lower aperture. The pharynx/gingiva cannot be judged as anatomically realistic from this view alone. Next geometry work should correct those visible contacts and silhouettes before adding surface detail.

Canine shape adjustment: vertical radius reduced from 3.8 to 3.5 mm, crown exponent changed from 0.75 to 0.55 for a broader rounded cutting region, and lateral taper reduced from 0.35 to 0.20. A shared `CANINE_TAPER` is used by rendered geometry and the tongue-contact envelope. Twelve feature tests passed. The three-stage oral Metal render completed with unchanged triangle counts and was viewed at `/tmp/voxy-oral-soft-canines.png`; reference `/tmp/voxy-oral-before-canine-softening.png`. Retained the modest shortening and broadening. Canines still look pointed in the frontal view, so this is an intermediate shape adjustment, not proof of realistic dental anatomy. Lip-contact sliver, angular inner-lip band and hidden tongue remain.

Buccal isolation diagnostic: `VOXY_FACE_DIAGNOSTIC_NO_BUCCAL=1` omits only the generated cheek lining; ordinary rendering keeps it enabled. Running the oral-transition command with this environment variable completed three Metal poses (867458/867460/867460 triangles, 11331552 ray segments each). Viewed `/tmp/voxy-oral-without-buccal.png` against `/tmp/voxy-oral-soft-canines.png`: the angular lower-lip band remains, while beige source-head surfaces become exposed on both sides of the cavity. This rules out the generated buccal lining as the direct geometry source of that band, though it changes lighting/occlusion. Keep the cheek lining and investigate original/reconstructed lip geometry next. The generic `/tmp/voxy-oral-transition.png` currently contains this isolation render; use the named images to distinguish diagnostic from normal output.

Rejected finer lip split: increasing local subdivision before contact-curve clipping from two levels to three passed twelve feature tests, including neutral area preservation. Metal oral-transition rendering completed at 880431/880434/880434 triangles and 12208608 ray segments per pose. Viewed `/tmp/voxy-oral-finer-lip-split.png` against `/tmp/voxy-oral-soft-canines.png`; the angular inner-lip band persists without a clear visual improvement. Reverted the additional subdivision because it adds about 12590 triangles without resolving the requested defect. This points to the internal lip shape/attachment correspondence rather than insufficient clipping resolution. Source is restored; the built preview binary and generic oral-transition output still represent the experimental subdivision until rebuilt.

Inner-lip material separation: reconstructed posterior lip tiles with bind-space mean Z below 0.150 m and normal Z below 0.35 now use the existing wet oral material (`uv=[-1,0.36]`, intrinsic mucosal color) instead of external skin atlas UVs. Classification is a source-mesh heuristic, not an anatomical label or calibrated vermilion boundary. Geometry is unchanged. Twelve feature tests passed and all three Metal oral-transition poses completed with restored baseline triangle counts. Viewed `/tmp/voxy-oral-inner-lip-material.png`; comparison with `/tmp/voxy-oral-soft-canines.png` finds 2409 changed pixels out of 1327104, confined to the open-mouth region (bounding box 705,347..1600,426). No obvious exterior-lip staining was seen. Retained the limited material separation, but it does not resolve the angular band or verify anatomical correctness across arbitrary face presets/cameras.

Lip reconstruction isolation: `VOXY_FACE_DIAGNOSTIC_NO_LIP_REPLACEMENT=1` skips `append_lips` while retaining the source-triangle removal and all oral anatomy. Three Metal oral-transition poses completed at 862447 triangles and 10988352 ray segments each. Viewed `/tmp/voxy-oral-no-lip-replacement.png`: jagged gaps appear at the lip corners and teeth become visible at nominal closure, as expected when replacement skin is omitted. The angular lower interior band still remains in the open frames. Together with buccal isolation, this places the unresolved band's geometry in retained source skin rather than generated cheek lining or reconstructed seal triangles. Removing the reconstruction is not a production fix. Next work should inspect and reshape the retained inner lower-lip surface during jaw opening, preserving corner closure and outer vermilion shape. Ordinary rendering retains lip reconstruction; the generic oral-transition PNG currently holds the isolation render.

Posterior lower-lip roll: added up to 4 mm additional retreat with jaw opening, weighted by the existing lower-jaw mask and smooth bind-space bounds in Y/Z/absolute X. The anterior lip depths fade out of the correction and neutral jaw multiplies it by zero. Nine face tests and twelve feature tests passed. Three Metal oral-transition poses completed at 867842/867843/867845 triangles and 11336736 ray segments each; viewed `/tmp/voxy-oral-inner-lip-roll.png` against `/tmp/voxy-oral-inner-lip-material.png`. The closed frame is pixel-identical; the open frames show the posterior surface receding and stronger commissure shadows. Retained as an intermediate deformation, but the inner band remains visible and the corner silhouette is not yet anatomically convincing. No full skin/tongue intersection proof or arbitrary-morph validity is claimed; the masks remain specific to the source bind mesh.

## DLSS 5 preparation

`voxy_streamline::neural_rendering` provides an engine-owned frame handoff and
adapter trait, separate from Super Resolution, Frame Generation and Ray
Reconstruction. The frame bundles color, motion, depth, an optional authored
appearance preservation mask, normalized structure/tone controls and a history
reset request. These are Voxy's preparation fields, not a claim about the exact
NVIDIA SDK ABI or required buffer formats. Resource dimensions, formats and
color-space validation remain the responsibility of a concrete SDK adapter.

The history validator rejects stale frame IDs and invalid controls, resets on
camera cuts, resizing and skipped frames, and commits history only after
successful evaluation. The contract is tested without NVIDIA hardware. No
production SDK adapter or DLSS 5 inference is implemented; Metal continues to
use the existing rendered image. Facial identity, makeup and material appearance
must be reviewed against the original renderer before enabling enhancement.

Reference: [NVIDIA DLSS 5 developer announcement](https://developer.nvidia.com/blog/whats-new-for-game-developers-dlss-5-with-3d-guided-neural-rendering-nvidia-ace-updates-and-new-rtx-kit-capabilities).

Oral ellipsoids now use 16 latitude and 32 longitude segments, and enamel color
varies from a warmer cervical region to a paler cutting edge. This remains an
approximation of anatomy and does not simulate enamel translucency.

Lip-corner audit: tapering the seam correction between 21 and 30 mm removed some horizontal wedges but created visibly jagged commissures in the three-pose oral closeup. That experiment was reverted. The next correction must change the aperture topology/geometry rather than only blend jaw weights.

Lip-seam refinement now splits each source seal triangle into sixteen barycentric
subtriangles before clipping. Each new point corrects inherited jaw interpolation
to its own bind-space jaw weight; correcting only crossing points caused spikes
and was rejected by rendered inspection. Neutral area and topology checks pass,
and the corrected three-pose oral GPU render passes. Corner wedges and the
angular inner lower rim remain visible, so this is not a finished lip model.

The tongue uses one continuous rounded surface with a tapered anterior region and a shallow median groove (up to 0.55 mm). It follows jaw displacement; papillae, independent tongue articulation and calibrated tissue scattering remain incomplete.

`FacePreview::sample_oral` independently controls tongue lift (-1..1) and forward
motion (0..1), weighted toward the anterior tongue and gated by jaw opening.
`--tongue --mouth --oral-closeup` exercises the motion in the hardware probe.
The rest shape stays unchanged for a closed jaw and topology remains stable.
These are kinematic controls. Moving tongue vertices are kept outside analytic
crown envelopes with a 0.2 mm margin, using the same rounded-crown exponent and
canine taper as the rendered teeth. Penetrating vertices retreat toward the
oral cavity. Triangle-level contact, gingival contact, compliant tissue response
and muscle-driven articulation are not implemented. The tongue remains dark under the current
studio lighting, so these renders do not establish realistic tissue appearance.

Procedural blinks have a 12 ms inter-eye phase difference with reversed ordering
on the second blink. A shared side-specific closure value drives both lid skin
and lash curl. Explicit poses default to zero offsets and can still close both
eyes fully. `--eye-closeup --blink-transition` inspects 0.86, 0.90 and 0.94 s
into the cycle, writing `/tmp/voxy-blink-transition.png`. This is a designed
animation variation, not measured physiological timing.

Lash closure now rotates both strand direction and curvature together by a
bind-space X rotation (up to 1.35 radians), instead of scaling their vertical
components. This preserves the reference curve lengths during closure. It
remains a kinematic groom approximation; lash/lid contacts and the anatomical
rotation axis have not been calibrated.

Upper lashes now use independently seeded left/right grooms with stratified
root jitter, varying length, thickness, lateral sweep and curl. Root ordering
and attachment topology remain fixed through animation. This reduces the
uniform comb pattern without introducing frame-to-frame randomness; it does
not yet add lower lashes or model clumping from mascara.

Vermilion microfolds use elongated smooth noise in bind space, with up to 25 µm
of microheight variation and locally increased roughness. They are controlled
by the wrinkle layer and remain independent of the lipstick color mask. This
is shading-scale relief; the unresolved lip-corner silhouette is unchanged.

Camera fixation now computes a separate direction from each bind-space eye
center. Per-eye offsets combine with the authored gaze channel, and one shared
rotation helper transforms both globe geometry and its corneal normal encoding.
A test verifies both optical axes converge on a centered near target. Angular
limits still apply; extreme close or side cameras need further calibration.

Lid-support audit: narrowing the upper/lower vertical falloff to 15/19 mm created additional notches at full closure in the 0.86/0.90/0.94 s hardware strip. The experiment was reverted. Future lid corrections must address surface deformation and rim geometry rather than mask width alone.

Angular-lid audit: preserving bind-space orbital radius produced excessive bulging; interpolating that radius to the globe shell produced corner gaps and wrinkles. Both GPU-tested variants were reverted. A continuous independent lid rim and controlled surface deformation are required before further radial-path changes.

Edge-distance audit: keeping skin offsets from analytic upper/lower lid arcs
reduced the large closed-lid fold, but the three-pose hardware closeup exposed
gaps and dark spikes along the seam. Seven expression tests still passed,
demonstrating that their locality checks do not establish closure quality.
The experiment was reverted. Closure contours must be derived from the actual
mesh aperture rather than the approximate arcs used for strand placement.

`VOXY_EXPERIMENTAL_LID_CONTOUR=1` enables an aperture-derived research path:
33 vertical triangle-intersection slices per eye, filtered against the globe
front to exclude the rear socket. The bind-space profiles are computed once
before expression deformation. A source-mesh test checks central rim heights,
finite ordered samples and preservation of a narrow skin band at closure.
Both strong (65%) and reduced (15%) edge-distance preservation were inspected
on the three-pose hardware closeup. Dark seam protrusions remain even at 15%,
so the path is disabled by default. The extracted contours are available for
building a continuous lid surface; this does not establish corrected closure.

Hardware isolation audit: the reduced-contour seam protrusions remained in
three separate runs omitting lashes, omitting both lashes and eye globes, and
omitting wet rim tubes. This points to the deformed skin rather than those
attachments. Diagnostics are opt-in environment variables:
`VOXY_FACE_DIAGNOSTIC_NO_LASHES=1`, `VOXY_FACE_DIAGNOSTIC_NO_EYES=1`,
`VOXY_FACE_DIAGNOSTIC_NO_RIMS=1`. They remove the corresponding triangles
from both visible rendering and the ray occluder; they are not production
closure workarounds. Skin triangle orientation and self-intersection must be
audited next.

The explicit ignored diagnostic `audit_lid_triangle_orientation` scans 3,582
source triangles near both eyes at eleven closure fractions. Run it with
`cargo test -p voxy_app --lib audit_lid_triangle_orientation -- --ignored --nocapture`.
It writes `/tmp/voxy-lid-topology.csv` and per-triangle details in
`/tmp/voxy-lid-topology-triangles.csv`. Normal reversals and projected winding
changes are diagnostic flags, not a proof of self-intersection.

At full closure the baseline has 123 triangles below 1% bind area and 191
normal reversals; the initial reduced contour path had 2 and 67 respectively.
The current experimental path preserves 35% of bind-space depth relative to
the globe shell, with zero triangles below 1% area and 85 normal reversals.
The measured summary is saved in `docs/face-lid-topology.csv`. The hardware
closeup removes the sharp dark seam spikes but introduces excess lid bulging
and buried lashes. It remains opt-in; default rendering is unchanged. An
audit passing only confirms finite measurements, not valid anatomy or closure.

Further depth audit: restricting the depth correction to the central fold
(7–12 mm vertical falloff) increased the visible crease and was reverted.
Preserving 85% of the distance from the aperture edge raised the minimum
full-closure area ratio to 0.0842 (82 normal reversals), but the hardware strip
showed larger corner folds and lower-lid bulging. That variant was also
reverted to the prior 15% research path. Better area ratios alone did not
predict a better rendered shape. A continuous lid surface with independently
controlled free-edge geometry is the next structural correction.

The ignored `audit_lid_self_intersections` diagnostic now checks strict
transverse intersections between nonadjacent skin triangles, excluding
shared bind positions (including duplicated OBJ seam vertices). It uses
double-precision segment/triangle predicates and AABB rejection. Its separate
predicate test covers crossing, separation, identical triangles and shared-edge
contact. Coplanar overlaps, pure edge contacts and tangencies are excluded;
these counts are not a complete collision audit.

Run with `cargo test -p voxy_app --lib audit_lid_self_intersections -- --ignored --nocapture`.
The source reference has zero measured intersections. The baseline has 0 at
half closure, 6 at 80% and 374 at full closure. The depth-preserving contour
path has 0, 4 and 67 respectively. Thus normal-reversal flags are accompanied
by verified transverse skin intersections, not only lighting artifacts.
Summary: `docs/face-lid-intersections.csv`; exact source triangle pairs:
`/tmp/voxy-lid-intersection-pairs.csv`. Neither path satisfies nonintersecting
closure. Future surface changes must be evaluated against these pairs and
rendered shape together.

Independent lid surface prototype: `female_lids::LidSurface` builds four
33-column / 17-row patches. Outer boundaries are sampled from the frontmost
source-body triangle intersection; free edges use the measured aperture and
converge to one shared seam per eye. Collapsed canthus cells are omitted from
the fixed index topology. The surface carries bind UVs through closure.
`FacePreview::lid_surface(closure)` exposes isolated patches; inspect them with
`cargo run -p voxy_ray_probe -- --experimental --face --lid-surface`.
This writes `/tmp/voxy-lid-surface.png` at closure 0, 0.5 and 1, tracing 3,968
triangles in each frame. The viewed hardware strip closes the center without
the earlier spikes, but canthus pinching remains visible.

The surface test checks pinned outer vertices, unchanged indices, nonzero
triangle area, positive frontal winding at eleven stages, and exact shared
upper/lower edges at full closure. It also checks the strict transverse
intersection predicate at open, half and full closure. These isolated patches
are not stitched into the body, attached to lashes or rigged to gaze and
constructor morphology; their geometry test and hardware render do not prove
the full character's eyelids corrected. The source body still uses the prior
deformation. Integration must replace the intersecting skin region, stitch its
outer boundary, transfer strand anchors and retest the complete ray occluder.

The first intersection run exposed a 30 nm corner-coordinate mismatch caused
by interpolating nominally identical endpoints. Corner and boundary rows now
use their shared positions directly, and columns retain their exact X values.
The strengthened surface test passes with no strict transverse intersections
at open, half and full closure, while checking area and frontal orientation
at eleven stages. This guarantee applies to the isolated patches only and
excludes coplanar overlap/contact cases as described above.

`FacePreview::lid_surface_with_eyes` adds the character's actual refined eye
globes, bind UVs and radial corneal-normal encoding to the isolated surface.
Inspect with `cargo run -p voxy_ray_probe -- --experimental --face --lid-surface --lid-globes`;
output: `/tmp/voxy-lid-globes.png`. The inspected three-pose hardware strip
contains 38,784 triangles per pose and shows the globe hidden at full closure.
A geometric test verifies an open center ray has no lid hit and a closed
surface lies in front of the nominal globe at nine iris-region locations per
eye. This is sampled frontal coverage, not a full gaze/contact sweep, and the
corner pinching, body stitching and lash transfer remain unresolved.

Corner-depth correction: nonendpoint free-edge and closure-seam samples now
use the maximum of globe support and the frontmost source-body depth. This
avoids recessing the lid rim behind the skin where the globe narrows toward
the canthi. The viewed open/half/closed hardware strip shows reduced corner
folding while retaining full closure; both surface tests pass. Small unfilled
canthal regions remain in the isolated open-eye render, and conjunctival
tissue/body stitching is still required. This change does not replace the
full character's eyelid geometry.

The isolated lid/globe preview now adds four thin wet canthal tissue strips,
each spanning four edge intervals behind the moving upper/lower free edges.
Their depth is 0.8 mm behind the minimum of the source upper edge, lower edge
and closure seam depths, with 0.15 mm tapered
vertical margins. Inner and outer tissue colors differ; the wet-tissue
material is used. The inspected 38,812-triangle hardware strip fills the
previous dark corner voids and hides the strips at full closure. A test checks
fixed topology, nonzero strip areas at open/half/full closure and sampled
triangle-centroid coverage behind closed lids. These are simplified
conjunctival strips, not a calibrated volumetric caruncle, tear duct or tear
film. Tissue/globe contact, volumetric corner geometry and the full gaze sweep
still need inspection before full-character integration.

The initial 0.3 mm seam-relative placement intersected the lower lid at the
lateral canthus in the open pose. The current depth rule resolves all measured
strict transverse strip/lid intersections at closure 0, 0.5 and 1; the tissue
test now includes those pairs. Side inspection is available with
`--lid-surface --lid-globes --eye-side`, writing
`/tmp/voxy-lid-globes-side.png`. The viewed hardware strip still has a small
medial gap and exposes the rear globe beyond the isolated patch boundary.
The latter requires the surrounding head mesh; the former requires proper
volumetric canthal tissue and a continuous transition. Frontal coverage alone
does not establish side-view closure or anatomical contact.

Head replacement research path: `FacePreview::lid_surface_with_head` subtracts
the bind-space replacement region from source skin using 32 convex column
cells per eye. Cut points on the outer contour take the corresponding patch
edge depth. Original source vertex positions/indices outside the region are
retained; replacement surfaces and actual globes are appended. Skin diffuse
lighting uses accumulated, position-welded area normals and the existing key
and fill directions. Unreferenced preserved vertices use a finite fallback.

Inspect with `cargo run -p voxy_ray_probe -- --experimental --face --lid-surface --lid-head`;
output: `/tmp/voxy-lid-head.png`. The inspected 122,764-triangle hardware strip
closes the eye without the old collapsed central fold. However, normal/slope
discontinuities and angular canthi make the replacement region visibly
distinct from adjacent skin. Geometric contour matching is not a watertight
topological weld: additional boundary subdivisions and normal/tangent
continuity are still required. This path is bind-space only; the default
character renderer, skeletal rig, facial expressions, constructor parameters,
hair, lashes and physical-skin bindings are not switched to it.

Subtraction tests cover area preservation for both polygon windings and a
collapsed-corner cell. A source-body regression checks finite assembled
geometry, preservation of original source positions and identical index
topology between open and closed poses. These checks do not prove that the
combined head has no self-intersections or raster cracks.

The replacement grid now samples bind-space depth from the source skin at
every row, rather than linearly interpolating outer/free-edge depths. Closure
offsets in Y/Z use squared row position, so the moving edge has zero analytic
influence on the slope at the pinned outer border. The frontal head strip
shows a softer transition in open and half-closed poses while preserving full
closure. Angular canthi and a visible outer contour remain; a complete C1
surface join and watertight topology have not been established. The transition
regression compares the first row with the source skin at twelve locations.

Upper/lower eyelid edges now have separate 90 µm-radius wet rim tubes attached
to barycentric points of the animated skin. Each reference rim has 33 sampled
anchors. This exposes explicit edge geometry for later refinement; it is not a
tear-fluid simulation or a calibrated anatomical waterline. The three-pose
blink strip shows a subtle lower edge, while the coarse eyelid fold remains.

Scleral pigment now includes faint branching vessels that fade toward the iris.
They alter intrinsic color only; corneal reflection remains independent. The
native fragment shader now evaluates iris and scleral pigment analytically
after corneal refraction, averaging four points within the pixel footprint.
This removes the small eye atlas region as the native pigment-resolution
limit. The CPU atlas remains a fallback. The inspected three-pose hardware
closeup shows sharp pupil edges and iris fibers, but only subtle vessels;
eyelid folds and the regular lash pattern still look synthetic. Temporal
aliasing, shader cost, and parity between the CPU and WGSL pigment formulas
have not been measured.

Replacement-lid boundary audit (2026-10-01): the clipped body has 536
interior skin nodes along the 128 outer lid segments that are absent from
the regular lid border subdivision. Nodes were deduplicated per segment
at 0.1 micrometre precision and accepted within 0.2 micrometre of the 3D
segment, excluding its endpoints. This establishes mismatched boundary
subdivision, not a count of visible cracks or proof of watertightness.
Reproduce with `cargo test -p voxy_app --lib
female_lids::tests::audit_replacement_boundary_subdivision -- --ignored
--nocapture`; the audit writes `/tmp/voxy-lid-boundary-subdivision.csv`.
Both sides need a shared subdivision before the replacement can be treated
as an integrated animated skin surface.

The replacement now splits each boundary-adjacent lid triangle through the
clipped skin vertices, reusing their indices. The boundary audit consequently
finds zero missing skin nodes. All seven lid tests, including the manual
audit, pass; open and closed assembled meshes retain identical indices.
Deduplication uses exact positions: merging nearby parameters initially
discarded seven distinct skin nodes and failed the audit. This fixes the
measured subdivision mismatch, but does not prove manifoldness of the whole
clipped body, smooth tangents at the seam, or absence of small clipping
slivers. Rig, morphology, and groom integration remain outstanding.
The new Metal hardware-ray preview completes for all three closure poses
(123,262 triangles, 1,592,736 ray segments per pose). Inspection of
`/tmp/voxy-lid-head.png` still shows the polygonal outer seam and angular
canthal folds. Matching border subdivision therefore has not resolved the
visible transition; the next correction must address clipped skin quality
and surface tangents rather than declaring the seam finished.

The next boundary correction preserves the interpolated source skin depth
instead of snapping clipped vertices to the lid's linear 3D segments.
Front-surface boundary depth differs from those segments by up to 0.974316
mm. Selection now uses XY proximity plus agreement with the source's
frontmost depth within 2 micrometres; this excludes deeper socket surfaces
that projected onto the same border (up to 15.665960 mm from the linear
segment). The updated audit covers that exterior boundary and reports zero
missing nodes. Seven tests pass, including stable open/closed indices.
Interior cut boundaries and whole-body manifoldness remain unverified.
The corrected Metal preview completes for three poses (123,078 triangles,
1,592,736 ray segments each). Inspection shows fewer protruding triangular
seam folds than `/tmp/voxy-lid-head-linear-border.png`, but the outer patch
outline, angular corners, and synthetic eyelid shading remain visible.

Neutral transition diagnostics: globe support displaces no vertices in
rows 1–13, three vertices in row 14 (maximum 0.266150 mm), and nine in row
15 (maximum 1.295231 mm). Thus the neutral outer seam is not caused by the
globe-support clamp. A separate face-normal audit finds 396 paired
skin/lid edges with a length-weighted angle of 19.598 degrees and maximum
140.935 degrees. It covers only edges with exactly two incident faces of
opposite skin/lid classification, after position quantization at 0.1
micrometre; it is not a full manifold or smooth-normal test. The adjacent
fan triangles need a matched subdivision extending into the lid rows,
instead of only extra vertices along the outer edge. Reproduce with the
ignored tests `audit_globe_support_displacement` and
`audit_exterior_seam_normals` in `female_lids::tests`.

The replacement now propagates each exterior boundary subdivision through
all 17 lid rows. Intermediate bind depth is sampled from the source skin,
with its correction fading toward the free edge; globe support remains
enforced. This reduces the paired seam's length-weighted face-normal angle
to 4.534 degrees and maximum to 84.799 degrees. The boundary audit still
reports zero missing exterior nodes. A first implementation removed faces
according to posed zero area and failed the stable-index test; face omission
now depends only on the fixed canthus grid parameters, and that test passes.
Nearly coincident source cuts can still produce very thin or collapsed
faces; neither manifoldness nor the quality of all added triangles is
established by these checks. The inspected adaptive preview has less of a
polygonal outer seam but shows vertical corrugation within the lids,
especially in the closed pose. This remains a research replacement, not a
finished transition or an integrated production eyelid.

Added columns now use the same source bind-depth plus closure displacement
rule as regular columns, applying globe support afterward once. The previous
added-column calculation interpolated already constrained depths and faded
only their source correction, which was inconsistent with regular columns.
`added_neutral_lid_nodes_follow_source_depth` checks over 1,000 added interior
neutral nodes against the source front depth (or globe support), with a
2 micrometre tolerance, and passes. The new three-pose Metal preview completes
and was inspected; vertical corrugation is still visible. Consistent neutral
depth does not establish smooth deformation or remove the observed closed-lid
artifact. The first test build encountered a temporarily missing unrelated
`physics/src/liquid/thixotropy.rs`; after that file appeared, the test completed.

Rejected closed-depth Hermite experiment: a source-tangent outer endpoint
and zero-tangent seam endpoint, blended with closure, passed all ten lid
tests and completed the three-pose Metal render. Visual inspection showed
larger outer-side flaps and broad angular folds. The change was removed,
restoring the consistent bind-depth rule and its inspected preview.
Rejected image: `/tmp/voxy-lid-head-rejected-hermite.png`; retained comparison:
`/tmp/voxy-lid-head-consistent-bind.png`. Scalar depth smoothing is insufficient
with the existing boundary and row parameterization. A new scalar-profile
test passed in an isolated extracted Rust harness; its Cargo build was
interrupted by unrelated concurrent `VoxelRegionSnapshot::retained_block`
API drift. That test and the rejected helper were removed with the experiment.

Clipped-boundary cleanup merges 57 distinct exterior nodes separated by
less than 0.2 micrometre, redirecting skin indices and using the same
canonical nodes for lid columns. Original source positions are preserved;
faces whose three indices become repeated are removed before lid assembly.
Ten lid tests pass. The neutral assembly audit finds zero zero-area lid
faces, 405 paired skin/lid edges, a length-weighted angle of 4.172 degrees,
and maximum 80.142 degrees. The three-pose Metal render completes and was
inspected: vertical corrugation remains apparent, so removing near-duplicate
columns does not establish a visually smooth lid. Zero-area counts for
partially and fully closed adaptive meshes have not yet been measured.

Adaptive closure audit now covers closure 0, 0.5, and 1. All poses retain
identical indices across 14,880 lid-skin faces, with zero zero-area faces,
zero faces below 1e-12 square metres, and zero negative frontal normal-Z
values. Relative to the open-pose normal, 9 faces rotate beyond 90 degrees
at half closure and 60 at full closure. At full closure, 35 are medial
(absolute X below 25 mm) and 25 lateral (above 40 mm), with none in the
intervening central band; angles reach 137.399 degrees. At half closure,
one is medial and eight lateral. This localizes large orientation changes
to the canthal regions, but proves neither self-intersection nor the cause
of central corrugation. Summary and face coordinates are saved in
`docs/face-lid-adaptive-closure.csv` and
`docs/face-lid-adaptive-reversals.csv`; reproduce with ignored test
`female_lids::tests::audit_adaptive_closure_faces`.

Two diagnostic material comparisons are now available in the hardware-ray
probe. Add `--lid-clay` to `--experimental --face --lid-surface --lid-head`
for uniform white albedo, constant roughness, zero oil, and constant
microheight; the eye's analytic pigment remains intact. Its three-pose Metal
preview `/tmp/voxy-lid-head-clay.png` was inspected and retains the vertical
lid stripes. Adding `--lid-flat-color` also overrides skin vertex RGB after
ray shading while preserving specular visibility, producing
`/tmp/voxy-lid-head-flat-color.png`. That inspected image loses the central
stripes and most skin shape cues. Both runs use the same geometry and ray
counts/occlusion results. This locates the visible stripe contribution in
the diffuse vertex-color lighting channel, excluding atlas pigment and
microheight as necessary causes. It does not prove geometric smoothness:
constant diffuse color hides much of the normal field. Investigate normal
averaging on the irregular adaptive grid before further shape smoothing.
These flags are diagnostic; the ordinary preview remains unchanged.

Replacement skin diffuse normals now sum unit face normals weighted by
the corner angle, computed in double precision, instead of face area.
The hardware-ray probe uses position-shared, angle-weighted skin normals
in `--lid-surface` mode for its offsets, light terms, and ambient-ray
directions; eye and wet-tissue handling remains separate. The finite-color,
source-position, and stable-index test passes. Both the diffuse-only normal
change and the subsequent ray-normal change completed three-pose Metal
renders and were inspected. Some shading transitions look softer, but
vertical corrugation is still visible. Matching the averaging rule therefore
does not establish that the normal field or surface is artifact-free.
Comparisons: `/tmp/voxy-lid-head-area-normals.png` and
`/tmp/voxy-lid-head-angle-cpu.png`; latest `/tmp/voxy-lid-head.png`.

Age-fold controls now act on body skin in bind space before expression and
skeletal posing, with the remaining face morphology applied separately.
Four parameter tests and the real-model forehead/brow attachment test pass.
The attachment test requires the selected forehead vertex to move at least
0.3 mm with a raised brow and retain at least 80% of its neutral wrinkle
relief. Preset rendering is available through `--face-preset <JSON>` on the
integrated face; the experimental independent-lid mode rejects that option.
The `mature-soft.json` preset completes three Metal hardware-ray poses
(851,368–851,369 triangles, 11,127,528 ray segments per pose), saved to
`/tmp/voxy-face-preset-ray.png`. The image was inspected: folds remain subtle
at portrait scale and do not yet produce convincing age detail. The skin
material still uses its default fine-line atlas; connecting independent
wrinkle parameters to material detail remains necessary.

### Parameterized crease material and diffuse lighting

The seven crease controls now add a narrow microheight profile to the existing skin atlas. The profiles reuse the geometry control centers, radii, and crease curves in bind-space; only the material height channel changes, preserving pigmentation, roughness, oil, alpha, and eye pigments. Maximum per-control material indentation is 150 micrometres; this is separate from the millimetre-scale mesh control. Native scene resources refresh the atlas when crease values change; the integrated offscreen preview reads the preset atlas too.

The atlas test `parameterized_creases_preserve_pigmentation_and_change_microheight` passed: a forehead preset changes more than 1000 height pixels, leaves all other channels unchanged, and the default preset reproduces the cached atlas exactly. `cargo check -p voxy_app -p voxy_ray_probe` passed before the subsequent shader-only change.

The first Metal render of the parameterized atlas completed three poses, but visual inspection still showed weak fine creases. The skin shader previously perturbed only specular lighting. The new shader also corrects baked diffuse irradiance by the ratio of perturbed to base normal lighting under the existing key and fill. Vertex-level ray shadow and ambient visibility remain coarse: the correction does not trace self-shadowing within microcreases and is not tissue subsurface transport. The full face, eyelid closure, groom, and eye appearance still do not prove the realism goal.

The updated shader completed the integrated Metal hardware-ray pass for all three poses (851368/851368/851369 triangles, 11127528 segments per pose). Its compiled binary contains the new diffuse correction, and the output was viewed. Fine age lines remain visually weak at this portrait scale; the change establishes consistent micro-normal lighting but does not demonstrate convincing elderly skin. Closed eyelid silhouette and ocular appearance remain visible limitations. Output: `/tmp/voxy-face-preset-ray.png`; prior specular-only comparison: `/tmp/voxy-age-material-specular-only.png`.

### Pore distribution inspection

Added `--skin-closeup` to the integrated ray preview: its camera frames forehead and periocular skin and saves `/tmp/voxy-skin-closeup.png`. A mature-soft render exposed a regular pore lattice from jitter restricted to the central 40 percent of each cell. Pore centers now use the entire cell, with neighboring-cell support so spots cross boundaries continuously. This remains an artistic deterministic distribution, not a measured skin sample.

All six `female_complexion::tests` passed in the rebuilt test binary, including an occupied-boundary/continuity test that the prior restricted distribution would fail, independent layer effects, and crease-atlas preservation. The updated Metal render completed three poses with 11127528 ray segments per pose. Viewed before and after: pore lattice is less apparent, but fine forehead lines remain weak and the skin still looks synthetic. Before: `/tmp/voxy-skin-closeup-grid-pores.png`; after: `/tmp/voxy-skin-closeup.png`. No evidence yet establishes convincing aging or absence of uncanny-valley appearance.

### Interrupted iris bundles

CPU atlas pigmentation and the fragment shader now modulate two high-frequency angular fiber bands with different radial envelopes, plus a low-frequency pigment variation. Integer angular frequencies preserve the atan2 seam. This is an artistic procedural approximation, not measured iris microanatomy or iris light transport.

Four rebuilt `female_eyes::tests` passed (material distinctions, angular seam, equator mapping, globe refinement). An independent current-source pigment range/seam harness passed, and Naga parsed/validated the current WGSL. The first full build was temporarily stopped by an unrelated `Liquid::effective_properties` API inconsistency; after that concurrent change resolved, the normal Cargo test and Metal render both completed. The eye closeup rendered three poses with 11127528 ray segments per pose and was viewed at `/tmp/voxy-eye-closeup.png`. Bundles show radial variation, but the iris still reads as a procedural radial pattern. Eyelid geometry, regularly spaced lashes, flat-looking sclera, and pore relief remain visible limitations. This does not establish eye realism or goal completion.

### Lower lashes and anatomical blink side

Added 24 shorter, thinner lower lashes per eye, using barycentric surface anchors, deterministic per-eye jitter, a lower-lid curve, and a separate mild blink rotation. Existing lash length/thickness/density controls and diagnostic omission affect both rows. Strand blink side now comes from its bind-space root side rather than lateral hair direction; inward-directed upper lashes no longer select the opposite eye's blink offset. Total facial strands are now 964 (600 brow, 96 upper lash, 48 lower lash, 220 fuzz).

All 11 rebuilt `female_features::tests` passed, including stable rendered topology and anchored lower-lash side checks. The integrated Metal eye closeup completed three poses with 11150568 ray segments each and was viewed. Lower lashes are visible mainly near the outer/inner margins; central row visibility and attachment depth still need investigation. Existing lid closure remains visibly broad/flat and the groom still reads as thin wires at this scale. Output `/tmp/voxy-eye-closeup.png`; previous stage `/tmp/voxy-eye-upper-lashes-only.png`. This stage is not a realism completion claim.

### Lower lash front-surface selection

Lower lash root projection used a point near the internal lid surface, hiding most central hairs. Raising the query depth alone was rejected because 3D nearest projection moved medial roots onto the nose (observed abs X as low as 0.0107 m). The accepted search raises the lower-lash query Z to 0.145 m but reduces depth weight to 0.25 during triangle projection and distance comparison, preserving rim XY placement while choosing the frontal surface. Barycentric anchors still deform with the actual body triangles; upper lashes and other follicles retain their original projection metric.

All 11 feature tests passed, now including lower-root X/Y bounds that reject the observed nose migration. The integrated Metal render completed all three poses (852904/852904/852905 triangles and 11150568 ray segments per pose). Viewed output shows the central lower lashes now present across the lower eye, rather than only at corners. They still look thin and mechanically arranged; eye, eyelid, and follicle anatomy need further work. Before `/tmp/voxy-lower-lashes-before-depth-fix.png`; rejected forward-only query `/tmp/voxy-lower-lashes-rejected-forward-query.png`; accepted result `/tmp/voxy-eye-closeup.png`. This is a root-visibility fix, not a realism completion claim.

### Lash curve tessellation and taper

Curved lashes now use eight longitudinal spans and six cross-section sides, with an initial frame projected onto each tangent rather than switching reference axes at a local tangent threshold. The taper retains more shaft thickness and reduces the terminal radius to four percent, replacing the previous rapid thinning and eight-percent tip. Root anchoring, strand count, and blink rotations are unchanged.

All 11 feature tests passed, including rendered topology across closure poses. The integrated Metal eye closeup completed three poses (862120/862120/862121 triangles; 11268072 ray segments per pose), and the output was viewed. Lash curves look smoother and darker, but still read as individual wires and have overly regular arrangements. This stage does not resolve the eyelid shape or establish realism. Previous `/tmp/voxy-lashes-four-span.png`; current `/tmp/voxy-eye-closeup.png`.

### Separate lash material

Lash geometry now carries UV material marker [-0.25, 0.45], entering a separate pore-free shader branch with diffuse surface lighting and GGX highlights. This is a surface approximation rather than a measured anisotropic hair BSDF. Ray shading supplies key visibility for this marker; it also preserves zero-alpha vertices instead of reviving density-hidden geometry when writing visibility. Brows and fuzz still use their existing material routing.

All 11 feature tests passed after updating the material-routing assertion to require all 7776 lash vertices to use the new marker. The integrated Metal eye closeup completed three poses (862120/862120/862121 triangles, 11268072 ray segments each) and was viewed. Appearance changes are subtle at this scale; strands still look wire-like, and the broad closed-lid shape remains a major limitation. Previous `/tmp/voxy-lash-skin-material.png`; current `/tmp/voxy-eye-closeup.png`. Goal remains incomplete.

### Rejected contour-band distance preservation

Tested replacing the experimental contour closure's 0.15 outer-band distance factor with 1.0. Nine facial tests passed (two diagnostic tests remained ignored), and a stronger 1.8 mm separation test passed for a synthetic 2 mm upper-lid band. The initial normal preview did not exercise the change because `VOXY_EXPERIMENTAL_LID_CONTOUR` is disabled by default. A second integrated Metal render explicitly enabled this flag and completed three poses (862120/862120/862121 triangles; 11268072 segments per pose).

Visual inspection rejected the result: large angular medial/lateral folds and an inflated lower band appeared at full closure. This demonstrates that preserving a synthetic band separation does not verify whole-lid shape. Reverted the coefficient and temporary strengthened assertion; default production path remains unchanged. Saved rejected output `/tmp/voxy-lid-rejected-distance-preservation.png`, normal-mask reference `/tmp/voxy-lid-default-mask.png`. Next work must distinguish internal folded layers from the outer lid and preserve their ordered deformation; changing one scalar coefficient is insufficient.

### Local contour closure with preserved layer depth

The contour branch now translates the aperture edge toward the closure seam, with Gaussian decay into the outer skin (6 mm upper / 3 mm lower) and smooth horizontal corner attenuation. It carries the full bind-space shell-depth offset rather than collapsing it to 35 percent. Nine facial tests passed, including an upper 2 mm band remaining at least 1.8 mm apart and a pair of shell layers preserving their 1 mm depth separation. These local invariants do not establish anatomical closure.

The explicit triangle orientation audit on 3582 source eyelid-area triangles measured full-closure baseline versus contour: near-collapse (<1 percent rest area) 123 versus 0; reversed normals 191 versus 11; projected reversals 277 versus 42; minimum area ratio 0.0000242 versus 0.1815. Diagnostic CSVs are `/tmp/voxy-lid-topology.csv` and `/tmp/voxy-lid-topology-triangles.csv`. The audit completed successfully but remaining reversals are defects, not passing realism evidence.

The flagged integrated Metal render completed three poses (867842/867842/867843 triangles; 11336736 ray segments per pose) and was viewed. It reduces the flat/expanded lid band compared with the rejected distance-only mapping, but corner folds and an artificial closure seam remain. During the build the shared body asset switched to `prepared/body-nipple-refined.obj`; a direct position-set check found the same 9537 frontal facial positions as the original asset. Other project changes were preserved.

Given the large reduction in collapse and reversals, contour closure is now the default in FemaleDemo, with `VOXY_EXPERIMENTAL_LID_CONTOUR=0` retaining the old-mask diagnostic. The replacement `female_lids` research mesh remains separate. This default change is progress with visible remaining defects; it does not prove absence of self-intersections or realistic eyelids.

The subsequent normal command without the contour environment flag also completed all three Metal poses and its output was viewed at `/tmp/voxy-eye-closeup.png`, confirming that the default preview now exercises the contour branch. Counts were 867842/867842/867843 triangles and 11336736 segments per pose. Minor ray visibility counts differed in the shared evolving worktree; no exact byte-equivalence claim is made.

### Contour sample regularization

The aperture contour now applies a five-sample [1,4,6,4,1]/16 filter to each edge channel, retaining ordered lower/upper values. This reduces local displacement spikes from intersections of internal source folds. Nine facial tests passed (two manual diagnostics ignored); the explicit orientation diagnostic also completed. At full closure, reversed normals decreased from 11 to 5 and projected reversals from 42 to 30; zero near-collapsed triangles remain. Minimum area ratio changed from 0.1815 to 0.1557, so this is not a uniform improvement of every metric.

The default integrated Metal eye render completed three poses (867842/867842/867843 triangles, 11336736 ray segments per pose) and was viewed. Corner peaks are reduced, but a small visible light gap remains near the closure margin. The filtered edge can differ from the actual unfiltered aperture boundary; exact rim contact must be addressed separately from the outer-skin smoothness. Before `/tmp/voxy-lid-before-contour-smoothing.png`; after `/tmp/voxy-eye-closeup.png`. This does not establish full closure, absence of self-intersections, or realistic lids.

### Preserve raw contact boundary near the rim

LidContour retains both source aperture samples and filtered outer-skin samples. Closure uses the raw contact boundary within 0.3 mm of the rim and smoothly blends to the filtered profile over the next 1.7 mm. Nine facial tests passed; the expanded source-contour test also passed in its subsequent rebuild, checking upper/lower sampled contact below 50 micrometres at six X locations across both eyes. These samples have controlled frontal depth and do not prove contact of every source rim vertex or of the complete posed/rigged mesh.

The orientation audit completed: full closure has zero near-collapse, 8 reversed normals and 37 projected reversals versus the prior filtered-only 0/5/30. Minimum area ratio improves from 0.1557 to 0.2078. This tradeoff preserves the real rim near contact at the expense of some local smoothness; corner defects remain. The integrated Metal render completed and was viewed (867842/867842/867843 triangles, 11336736 segments per pose). The bright closure margin is reduced but not proved absent everywhere. Before `/tmp/voxy-lid-filtered-rim-gap.png`; current `/tmp/voxy-eye-closeup.png`. Complete contact and anatomical validation remain pending.

### Rejected uniform front-layer blink mask

Tested extending full contour closure strength from Z >= 0.129 m to Z >= 0.120 m (with fade at 0.110..0.120), to prevent internal rim layers lagging the outer layer. Nine facial tests passed; an expanded sampled-rim test at depths 0.124/0.129/0.134 m also passed. The integrated Metal eye closeup completed three poses (867842/867842/867843 triangles, 11336736 segments each) and was viewed.

The orientation audit rejects this change despite sampled contact: full closure reversals rose from 8 to 22, minimum area ratio fell from 0.2078 to 0.02690, while near-collapse remained zero and projected reversals changed 37 to 29. This is evidence that a uniform depth gate applies the same closure to geometrically distinct internal folds. Reverted the depth mask and restored the contact gate to its prior outer-depth scope. Saved rejected render `/tmp/voxy-lid-rejected-uniform-layer-mask.png`; retained prior render `/tmp/voxy-lid-depth-mask-before.png`. Next deformation work must identify actual rim-connected layers instead of treating every frontal depth as the same lid layer.

### Source topology and texture-free closure inspection

Added reproducible `tools/audit_face_lid_topology.py` using OBJ triangulation, position welding at 1e-7 m resolution, edge incidence, and induced-region connectivity. Both original body.obj and active prepared/body-nipple-refined.obj report the same audited region: 1386 vertices, 3924 edges, zero boundary edges, zero nonmanifold edges, two connected components of 694 and 692 vertices. This is a cropped graph diagnostic, not an anatomical layer classification. The lid has no open topological boundary that could identify its aperture by boundary-edge traversal.

Commands:

```sh
python3 tools/audit_face_lid_topology.py assets/characters/blender-female/body.obj --output /tmp/voxy-lid-source-topology
python3 tools/audit_face_lid_topology.py assets/characters/blender-female/prepared/body-nipple-refined.obj --output /tmp/voxy-lid-prepared-topology
cargo run -p voxy_ray_probe -- --experimental --face --eye-closeup --lid-clay
```

The material-neutral Metal render completed all three poses (867842/867842/867843 triangles, 11336736 segments per pose) and was viewed. Its constant atlas removes skin pigment/pores/material microheight; baked vertex irradiance, geometry, ray visibility and eye/lash material branches remain. Corner creases and the swollen-looking lower band persist without skin microtexture. Saved diagnostic `/tmp/voxy-lid-current-clay.png`; regular-material output restored at `/tmp/voxy-eye-closeup.png`. Next layer identification must use surface orientation/visibility and connectivity, rather than an assumed open rim or depth gate. Goal remains incomplete.

### Rejected front-facing-only contour selection

Tested excluding source triangles with normal.z <= 0 when collecting aperture intersections. Nine facial tests passed and the explicit orientation diagnostic completed. Full closure still has 8 reversed normals and zero near-collapsed triangles; projected reversals increased from 37 to 39, with unchanged minimum area ratio 0.2078. The default integrated Metal render completed three poses (867842/867842/867843 triangles, 11336736 segments per pose) and was viewed; no convincing corner-fold improvement was visible.

Removed the filter because it did not improve the requested result. Orientation alone does not separate all visible rim surfaces from folded layers: the next contour criterion must use actual front-surface visibility relative to the globe. Rejected render `/tmp/voxy-lid-rejected-front-only-contour.png`; retained prior render `/tmp/voxy-lid-before-front-contour.png`. Diagnostic values do not establish complete closure or absence of self-intersections.

### Actual globe coverage diagnostic

Added ignored/manual `audit_lid_globe_coverage`, explicitly run against the current source mesh. It projects the real refined eye meshes and posed source skin along bind-space +Z, evaluates frontmost triangle depths on a 0.5 mm grid, and marks the globe exposed when skin is absent or at least 10 micrometres behind it. This is a coarse orthographic bind-space diagnostic, not a production-camera, rig, gaze, material or complete-contact proof.

Measured visible globe samples: open 730/1688 (positive X) and 730/1691 (negative X); half closure 380 per side; full closure 12/1688 and 8/1691. Full-closure exposures lie near abs X 0.0225..0.023 and 0.0415..0.0445 m, Y 0.710..0.711 m. The nearest sampled skin behind these gaps is about 9.9..19.0 mm behind the eye, consistent with a genuine aperture exposing rear skin rather than a pigment highlight. CSV `/tmp/voxy-lid-globe-coverage.csv`; log `/tmp/voxy-lid-globe-coverage.log`.

The default intermediate-blink Metal render also completed three poses (867842 triangles each, 11336736 segments per pose) and was viewed at `/tmp/voxy-blink-transition.png`. It shows broad upper-lid movement, remaining corner peaks, and unnatural lash sweeps during the transition. Current mesh still does not fully occlude the globe at closure. The new diagnostic provides a geometric measure for subsequent contact corrections, without changing the deformation in this stage.

### Local aperture rim strength

Contour closure now increases the depth-mask strength only close to either source aperture edge, fully inside a 0.3 mm band and smoothly fading by 1.5 mm, additionally restricted to frontal depths 0.120..0.126 m. This avoids the previous globally uniform layer-mask change. Nine facial tests passed; the sampled-contact test was rebuilt with 0.127/0.129/0.134 m depth cases and passed.

The direct actual-globe coverage diagnostic measured unchanged open exposure (730 samples per eye), half closure 375/1688 and 377/1691, full closure 3/1688 and 0/1691 versus previous 12 and 8. Remaining positive-X samples are (0.0225,0.7105), (0.023,0.7105), and (0.0445,0.7105) m. The orientation audit exposes a tradeoff: normal reversals increase from 8 to 19, projected reversals decrease from 37 to 30, near-collapse remains zero, and minimum area ratio changes from 0.2078 to 0.1494. This is not a general mesh-validity improvement.

The integrated default Metal eye render completed all three poses (867842/867842/867843 triangles, 11336736 ray segments per pose) and was viewed. No clear visual regression was apparent, so the local contact correction is retained as an intermediate improvement in globe occlusion. Corner folds and three sampled exposures remain; zero exposures on the other eye's coarse orthographic grid do not prove complete closure. Previous `/tmp/voxy-lid-before-local-contact-mask.png`; current `/tmp/voxy-eye-closeup.png`. Goal remains incomplete.

### Finer source contour sampling

LidContour now uses 129 source aperture samples per eye instead of 33, reducing plane spacing from 0.875 mm to 0.21875 mm. The outer profile uses a normalized 17-tap Gaussian with approximately 0.875 mm sigma so smoothing scale stays physical rather than shrinking with sample spacing. Raw contact and filtered outer-skin profiles remain separate.

Nine facial tests passed, including local band/depth preservation and sampled contact. Both manual diagnostics completed. Full-closure actual-globe exposure decreases from 3/1688 to 1/1688 on positive X and remains 0/1691 on negative X; the remaining coarse-grid point is (0.023,0.7105) m. Normal reversals decrease 19 to 18, projected reversals increase 30 to 33, zero near-collapse remains, and minimum area ratio improves 0.1494 to 0.2628. These are mixed diagnostic measures, not full closure or anatomy proof.

The default Metal eye closeup completed three poses (867842/867842/867843 triangles, 11336736 ray segments each), and its output was viewed. Retained the finer sampling for improved globe occlusion and less area distortion. Corner peaks, one sampled exposure, and nonphysical-looking skin/lash movement still require work. Before `/tmp/voxy-lid-contour-33-samples.png`; current `/tmp/voxy-eye-closeup.png`. Goal remains incomplete.

### Fine full-closure coverage map

Added `VOXY_LID_AUDIT_FINE=1` to the manual globe coverage diagnostic: it evaluates full closure on a 0.125 mm grid instead of 0.5 mm, saving a separate `/tmp/voxy-lid-globe-coverage-fine.csv`. The run completed in 11.84 seconds and found 68/26713 positive-X and 49/26715 negative-X exposed samples. This invalidates interpreting the earlier coarse-grid zero on one eye as complete closure. Exposures include a thin central contact-line strip, not only corners (Y 0.71025..0.71075 m overall). The diagnostic still measures source skin against actual refined globe geometry in orthographic bind space; wet-rim/strand feature geometry, production camera and rig are not included.

Added `tools/plot_lid_coverage.py` to render full-footprint and vertically enlarged contact-strip plots. The generated `/tmp/voxy-lid-contact-map.png` was viewed: narrow exposed samples run along the seam, with larger offsets at the corners. The plotting helper uses Matplotlib installed in temporary `/tmp/voxy-lid-plot-env`; no repository dependency or runtime renderer changed for plotting.

```sh
VOXY_LID_AUDIT_FINE=1 cargo test -p voxy_app audit_lid_globe_coverage --lib -- --ignored --nocapture
python tools/plot_lid_coverage.py /tmp/voxy-lid-globe-coverage-fine.csv /tmp/voxy-lid-contact-map.png
```

No deformation change in this stage. Next contact work must address sub-grid rim separation over the complete seam as well as the corner gaps, and verify on the fine grid rather than optimizing one coarse sample. Goal remains incomplete.

### Fine coverage including the wet lid margin

The completed `VOXY_LID_AUDIT_FINE=1 VOXY_LID_AUDIT_RIMS=1` run includes the actual wet-margin triangles produced by `FaceFeatures::append`, using area-weighted normals from the posed source skin. It finished in 17.76 seconds. Full closure leaves 61/26713 positive-X and 47/26715 negative-X globe samples exposed, compared with 68 and 49 when considering skin alone. The wet margin therefore covers only nine additional sample positions; it does not resolve the contact defect.

Exposures remain in both the central strip and corners: absolute X 22..44.5 mm, Y 710.25..710.75004 mm across both eyes. Sampled skin behind exposed globe ranges from about 0.019 mm to 21 mm. This includes small depth/contact errors and samples looking through the aperture toward rear skin, so a uniform surface-depth offset alone is not established as a sufficient correction.

CSV: `/tmp/voxy-lid-globe-coverage-rims-fine.csv`; viewed plot: `/tmp/voxy-lid-contact-map-with-rims.png`. This is orthographic source-bind geometry, including wet margins but excluding production rig, arbitrary face presets, gaze and camera transformations. Passing the diagnostic test means the measurement completed; it is not a closure gate. No production deformation was changed in this stage. The next correction must address the actual aperture contact mapping and retain checks for triangle distortion and intermediate-blink appearance.

### Rejected wider contact band

Experimentally widened both the full rim-depth band and the start of raw-to-filtered contour blending from 0.3 to 0.6 mm. All 12 facial tests (including three manually enabled diagnostics) completed, but full-closure exposure worsened from 61/47 to 62/47 samples. Normal reversals increased from 18 to 20, projected reversals from 33 to 35, and minimum triangle-area ratio decreased from 0.2628 to 0.2421; near-collapse remained zero. The strict transverse-intersection diagnostic recorded zero pairs for the contour deformation at its four sampled closure values; it does not detect every possible contact or overlap.

Reverted both constants because this change improved neither coverage nor triangle distortion. Preserved measurements at `/tmp/voxy-lid-rejected-wide-contact.csv` and `/tmp/voxy-lid-rejected-wide-contact-topology.csv`; plot `/tmp/voxy-lid-rejected-wide-contact.png`. The default coverage CSV now holds the experimental result, so use the named rejected CSV for this run and the prior plot/log for baseline comparisons. The attempted new Metal render did not compile: unresolved `voxy_time` imports in `voxy_scene` and `voxy_runtime` during concurrent workspace edits. No new model render or restored-source test pass is claimed for this stage. The failure of a broader band strengthens the case for correcting aperture correspondence instead of simply increasing deformation strength.

### Front-depth contour experiment (rejected)

The workspace time-dependency edits subsequently reached a buildable state without face-task changes to those dependencies. Tested reducing contour candidates' permitted depth behind the ideal globe from 1.5 mm to 0.1 mm. All twelve facial tests, including measurement diagnostics, completed in 19.98 seconds. Fine skin-plus-rim exposure decreased substantially to 17/26713 and 18/26715 samples, indicating that selecting deeper contour intersections contributes to the contact defect. However, full-closure strict transverse intersections increased to twelve pairs; normal reversals increased to 27, projected reversals to 53, and minimum area ratio decreased to 0.0511. The diagnostic tests report measurements and do not reject those values themselves.

The Metal eye closeup with intermediate-blink poses completed (867842 triangles and 11336736 ray segments per pose) and was viewed at `/tmp/voxy-lid-front-depth-100um-render.png`. It still shows sharp corner folds and unnatural lash sweeps. Reverted the depth threshold to 1.5 mm because the improved occlusion introduces skin intersections. Preserved CSVs `/tmp/voxy-lid-front-depth-100um.csv`, `/tmp/voxy-lid-front-depth-100um-topology.csv`, and `/tmp/voxy-lid-front-depth-100um-intersections.csv`. Generic diagnostic outputs and the built preview binary represent this experiment until rebuilt/rerun; source is restored. Next work should choose the actual anterior aperture boundary with continuous correspondence, rather than globally tightening the depth threshold.

### Lash orientation follows the attachment surface

Each strand now records an orthonormal bind frame from its attachment triangle. Upper and lower lashes transport both their length vector and curved bend through the posed triangle frame relative to that bind frame. The posed body already includes the facial deformation, rig and head movement, so no extra head transform is applied to transported lashes. For a collapsed attachment triangle, the previous blink/head orientation remains a fallback. Barycentric roots and strand length/thickness controls are retained.

Eleven existing feature tests passed. A separately rebuilt new test verifies rigid rotation/translation correspondence, preservation of strand-vector length and detection of collapsed triangles; it passed. The actual Metal closeup completed all three blink-transition poses, at 867842 triangles and 11336736 ray segments per pose. Viewed `/tmp/voxy-lash-skin-frame-render.png`: lashes now change orientation with their local lid attachment instead of sharing a fixed blink rotation; strong downward sweeps are reduced. Retained this as a kinematic improvement. Upper lashes still look overly upright at closure, and corner folds/contact remain unresolved. Local triangle transport preserves a rigid strand shape; it is not a calibrated follicle model or hair/contact simulation.

## Регуляторы материала кожи — 2026-10-01

В конструктор добавлены `skin_roughness_scale` (0.25–2) и `skin_oil_scale` (0–2), оба по умолчанию 1. Настройки входят в сигнатуру обновления нативного атласа и сохраняются в JSON. Пресеты `matte-skin.json` и `oily-skin.json` меняют только эти два параметра.

Проверено: 7 тестов `female_complexion::tests`, включая сохранность пигментации, микрорельефа и alpha при изменении R/G материала; тест соответствия каталога регуляторам. Реальный пресет проверен командой `cargo run -p voxy_ray_probe -- --experimental --face --skin-closeup --face-preset assets/characters/face-presets/oily-skin.json` на Apple M4 Max / Metal: три позы, 11 336 736 лучевых сегментов на позу. Результат просмотрен: `/tmp/voxy-oily-skin-constructor-preset.png`. Интерактивное изменение ползунков в нативном окне на этом этапе не проверялось. Складки век и закрытие глаз всё ещё требуют доработки.

Поры и макияж теперь регулируются через `skin_pores` и `skin_makeup` (0–1). Каталог содержит 233 регулятора. Восемь тестов материала и проверка каталога прошли, включая проверку неизменной пигментации глаз при отключении каждого слоя. Пресет `bare-skin.json` отрендерен на Metal в трёх позах; результат просмотрен: `/tmp/voxy-bare-skin-constructor.png`. Перестроение полного атласа при этих настройках пока выполняется синхронно; отзывчивость интерактивного ползунка не измерена.

## Ограниченная коррекция глубины края века — 2026-10-01

Около исходного края века добавлена передняя коррекция глубины с пределом 0.15 мм. Неограниченное прижатие слоёв к поверхности сферы отклонено: оно сокращало щели до 31/24 точек, но увеличивало число обращённых нормалей с 18 до 27. Коррекция только в полосе разделения 0.2 мм не улучшила покрытие.

Принятый предел 0.15 мм уменьшил открытые отсчёты при полном моргании с 61/26713 и 47/26715 до 45/26713 и 26/26715. Ортографический аудит исходной позы с шагом 0.125 мм включает реальные треугольники глаз, кожи и влажного края. Он не доказывает контакт при произвольном ракурсе, взгляде или настройках формы. В выбранной области кожи остаются 18 обращённых нормалей и 33 обращения проекции; минимальное отношение площади выросло с 0.2628 до 0.2755, почти вырожденных треугольников нет. Диагностика строгих поперечных самопересечений дала ноль пар для contour при закрытии 0, 0.5, 0.8 и 1; касания и копланарные наложения она исключает.

Артефакты измерения: `/tmp/voxy-lid-shell-contact-capped-coverage.csv`, `/tmp/voxy-lid-shell-contact-capped-map.png`, `/tmp/voxy-lid-shell-contact-capped-topology.csv`, `/tmp/voxy-lid-shell-contact-capped-intersections.csv`. Оставшиеся щели, заломы у уголков и направление ресниц при закрытии требуют дальнейшей доработки.

После принятой коррекции прошли 10 обычных тестов лица и три отдельных диагностических аудита. Metal-рендер трёх фаз моргания завершён и просмотрен: `/tmp/voxy-lid-shell-contact-capped-render.png`; 11 336 736 лучевых сегментов на позу. Изображение подтверждает оставшиеся складки у уголков и проблемы направления ресниц.

## Ограничение поворота ресниц при моргании — 2026-10-01

Корни ресниц остаются привязаны к деформируемым треугольникам кожи. Начиная с blink=0.5, ориентация плавно ограничивается вокруг положения шарнира века; при полном закрытии отклонение от него не превышает 0.35 рад. Это художественное ограничение рига, а не измеренная анатомическая норма. Оно предотвращает перенос локальных заломов века на полный поворот ресницы. Ограничение учитывает движение головы, переносит одновременно направление и изгиб волоска и не меняет ориентацию при blink=0.

Прошли 13 тестов facial features и отдельный тест всех 96 верхних ресниц на деформированной исходной сетке: кончик каждой направлен ниже корня при полном закрытии. Это не проверка столкновений волосков с кожей или друг с другом. Metal-рендер трёх фаз моргания просмотрен: `/tmp/voxy-lash-hinge-render.png`; верхние ресницы при смыкании перестали торчать вверх, но у уголков остаются чрезмерные изгибы и пересечения. Закрытие самого века и правдоподобная форма ресничного края ещё не завершены.

## Независимые слои макияжа — 2026-10-01

Добавлены `makeup_lipstick`, `makeup_blush` и `makeup_eyeshadow`, каждый 0–1 с исходным значением 1. Общий `skin_makeup` умножает каждый слой. Подключены JSON, каталог и сигнатура обновления атласа. Помада меняет оттенок и материал губ, не изменяя их микроскладки. Всего в каталоге 236 регуляторов в 42 группах.

Прошли 8 существующих тестов материала, отдельная проверка независимых областей (губы, щека, веко) и проверка каталога. Пресет `lipstick-only.json` с выключенными румянами и тенями отрендерен в трёх позах на Metal и просмотрен: `/tmp/voxy-lipstick-only-render.png`. Цвета пока фиксированы, размещение задано процедурными масками, взаимодействие кисти с кожей не реализовано. Перестроение атласа синхронное; интерактивная отзывчивость не проверена.

## Цвет помады — 2026-10-01

Добавлены три линейных цветовых канала `lipstick_red/green/blue` (0–1), JSON-пресет `burgundy-lipstick.json` и обновление атласа при изменении цвета. Исходные значения 0.92/0.43/0.53 сохраняют прежний результат. Всего 239 параметров в 42 группах. Прошли 10 тестов материала и проверка каталога; отдельный тест подтверждает сохранность рельефа, roughness и sebum при смене цвета и отсутствие цветового эффекта при выключенной помаде. Пресет отрендерен на Metal в трёх позах и просмотрен: `/tmp/voxy-burgundy-lipstick-render.png`. Направление света и число затенённых лучей совпадают с прежним пресетом только помады, поскольку геометрия не менялась. Нативные ползунки интерактивно не проверены. Маска помады пока процедурная, рисование кистью отсутствует.

## Параллельное построение атласа — 2026-10-01

Расчёт атласа разделён по независимым блокам строк с максимумом 8 потоков; общий буфер 33 554 432 байта не дублируется между потоками. Диагностический тест `atlas_parallel_matches_serial` сравнивает все байты последовательного и параллельного расчёта бордового пресета. В одном debug-прогоне на Apple M4 Max: последовательный 18.2569 с, восемь потоков 4.1649 с, примерно 4.38× быстрее. Это один локальный замер, не release-бенчмарк или гарантия задержки интерфейса.

Повторный Metal-рендер просмотрен: `/tmp/voxy-atlas-parallel-render.png`. Все RGBA-пиксели совпали с `/tmp/voxy-burgundy-lipstick-render.png`. Обновление атласа по-прежнему синхронное; 4.16 с недостаточно для интерактивного перетаскивания ползунка. Требуется фоновой расчёт с отбрасыванием устаревших результатов и дальнейшая проверка нативного окна.

## Фоновое обновление кожи — 2026-10-01

В нативном `scene_app` повторное построение атласа выполняется через `AtlasUpdate` в отдельном потоке. Одновременно работает не более одного задания; во время расчёта показывается предыдущая текстура. На каждом кадре проверяется готовность без ожидания. Готовый результат применяется только при совпадении сигнатуры с текущими настройками; устаревший результат отбрасывается, затем запускается расчёт последних настроек. Возврат к уже показанным настройкам не запускает лишний расчёт. Загрузка готовой текстуры остаётся на потоке рендера. Первоначальная загрузка атласа остаётся синхронной.

`cargo check -p voxy_app --lib` и тест `atlas_update_does_not_wait_and_discards_stale_results` прошли. Тест удерживает рабочий поток каналом, проверяет возврат опроса до разрешения продолжить, отбрасывание устаревшего задания и применение актуального. Он использует малый буфер и не доказывает плавность окна или скорость GPU-загрузки полного атласа. Новый визуальный кадр на этом этапе не снимался; последний проверенный внешний вид — `/tmp/voxy-atlas-parallel-render.png`. Формула материала этим изменением не меняется.

## Реальный фоновый атлас и попытка нативной проверки — 2026-10-01

Тест `real_atlas_background_polling` прошёл на настоящем атласе 33 554 432 байта. В одном debug-прогоне: получение результата 1.8888 с, 1476 опросов, максимальная длительность опроса 379.292 мкс. Готовые байты совпали с синхронным `atlas_for_parameters` для тех же настроек. Это измерение CPU-опроса, без GPU-загрузки и оконного цикла.

Запущен нативный пример `face_constructor` с `/tmp/voxy-face-async-check.json`, затем через временный пакет `/tmp/VoxyFaceAsync.app`; первоначальный процесс завершён перед запуском пакета. Окно пакета снято системным захватом именно его window ID: `/tmp/voxy-native-async-before.png` и `/tmp/voxy-native-async-after.png`. Оба снимка просмотрены: область рендера пустая, заголовок Playing. Проверка смены обычной помады на бордовую в этом окне не подтверждена. CUA вернул тайм-аут доступа к окну. Сэмпл живого процесса `/tmp/voxy-native-face-live-sample.txt` показывает главный поток в `FemaleDemo::mesh -> skin_light_diffusion::diffuse_measured`. Это указывает на тяжёлый расчёт диффузии как следующий объект проверки, но не доказывает единственную причину пустого окна. На момент записи процесс пакета продолжает работать.

## Параллельное решение диффузии света — 2026-10-01

Три независимых цветовых канала поверхностной диффузии решаются в трёх scoped-потоках. Каждый использует прежний порядок операций внутри своего канала, критерии сходимости и энергетический контроль. Сборка массы и рёбер остаётся общей и последовательной. Для проверки сохранён последовательный путь того же решателя.

Прошли три физических теста и аудит импортированной сетки (42 342 вершины, 84 680 треугольников): результат каждого параллельного прогона точно совпал с последовательным Vec<f64>. После прогрева собраны семь замеров. В debug на M4 Max последовательное решение 933.47 мс, медиана параллельного 423.77 мс (2.20×); сборка 211.79 против медианы 209.82 мс. Полная стоимость этого статического ядра остаётся около 634 мс, без остального кадра. Отчёт `/tmp/voxy-diffusion-debug-parallel.json`, просмотренный график `/tmp/voxy-diffusion-channel-timing.png`.

Пустое окно нативного конструктора пока не исправлено и не объяснено полностью. Предыдущие процессы завершились; второй запуск с сохранением stderr закончился с кодом 0 без SCENE ERROR. Нативный кадр после этой оптимизации не проверялся. Новый шаг должен проверить весь путь обновления сцены, а не считать ускорение статического ядра доказательством исправного окна.

## Подтверждение нативного вывода и смены помады — 2026-10-01

Добавлен диагностический флаг `VOXY_FACE_FRAME_TRACE`: первые восемь кадров показывают начало, готовность сетки и исход render/present. В запуске `VOXY_FACE_FRAME_TRACE=1 cargo run -p voxy_app --example face_constructor -- /tmp/voxy-face-async-check.json` журнал `/tmp/voxy-face-frame-trace.log` подтверждает `Presented`. Для кадров 4–7 до готовности сетки 1368–1384 мс, общий кадр 1370–1386 мс. Это около 0.72 кадра/с, не плавная анимация.

Просмотрен реальный снимок нативного окна `/tmp/voxy-native-face-presented.png`: лицо с бордовой помадой видно. В том же процессе scratch-пресет заменён на `lipstick-only.json`; через десять секунд снят и просмотрен `/tmp/voxy-native-face-live-preset.png`, на нём обычная розовая помада. Журнал подтверждает чтение изменённого пресета. Это доказательство живого применения материала без перезапуска окна; два снимка не измеряют отсутствие пропущенных кадров во время фонового расчёта. Причина прежнего пустого окна не установлена, но в текущем запуске оно отображает лицо. Полный путь подготовки сетки остаётся главным объектом оптимизации.

## Release-проверка нативного кадра — 2026-10-01

Диагностический trace дополнен временем завершения обновления анимации. Собран `cargo build -p voxy_app --example face_constructor --release`; первая попытка попала на временную ошибку соседнего importer (`InputError::to_string`), актуальный исходник уже содержал исправление и повторная сборка прошла без изменений в importer. Запуск `VOXY_FACE_FRAME_TRACE=1 target/release/examples/face_constructor /tmp/voxy-face-async-check.json` подтвердил восемь исходов Presented.

В `/tmp/voxy-face-release-trace.log` медиана кадров 1–7 — 89.437 мс; в прежнем debug trace — 1379.747 мс. Обновление анимации в release около 0.03 мс, почти вся стоимость приходится на подготовку сетки до mesh_ready. Это измерение CPU-вызова draw до возврата present, без GPU fence; восемь кадров не доказывают устойчивый FPS или отсутствие задержек при изменении пресета. Процесс завершился с кодом 0; новый снимок release-окна не снимался. Сравнение просмотрено на графике `/tmp/voxy-face-release-timing.png`. Даже 89 мс недостаточно для плавной лицевой анимации; следующий профиль должен разложить FemaleDemo::mesh по стадиям.

## Профиль стадий mesh и повторное использование PCG-буфера — 2026-10-01

Добавлен `VOXY_FACE_MESH_TRACE`, измеряющий отдельные стадии FemaleDemo::mesh. На запущенном release-конструкторе медианы: деформация 7.06 мс, нормали 2.74 мс, диффузия света 51.72 мс, tint 0.49 мс, детали лица 3.53 мс, волосы/форма 1.63 мс, film/probe 0.003 мс, material coordinates/валидация 2.40 мс. График `/tmp/voxy-face-mesh-stages.png` просмотрен; исходный журнал `/tmp/voxy-face-mesh-reused-profile.log`. `mesh_ready_ms` в scene_app включает последующую запись GPU-буфера, поэтому его нельзя целиком приписывать CPU-расчёту mesh.

PCG теперь использует один буфер матричного произведения на канал вместо выделения нового Vec на каждой итерации. Release-профиль импортированной сетки прошёл; файл результата `/tmp/voxy-diffusion-reused.bin` точно совпал через cmp с прежним `/tmp/voxy-diffusion-before.bin`. Последовательный и параллельный варианты также точно совпали в тесте. Существенного ускорения нативного кадра не подтверждено: время осталось около 90 мс. Это уменьшение повторных выделений памяти, не заявленное улучшение FPS.

Для сборки включена serde-поддержка glam в voxy_editor: текущая ViewportCamera сериализует Vec3. Пресеты, математическая модель диффузии и форма лица не менялись. Цель реалистичной плавной анимации остаётся незавершённой; основной измеренный участок — диффузия света.

## Начальное приближение освещения из предыдущего кадра — 2026-10-01

FemaleDemo сохраняет последний результат поверхностной диффузии и передаёт его как начальное приближение PCG. Матрица и источник света собираются по текущей сетке; прежние проверки остатка, сходимости и энергии сохраняются. Неверная длина, NaN или отрицательные значения начального приближения приводят к расчёту от текущего источника. Это ускорение численного решения, а не применение старого освещения вместо нового.

Четыре обычных release-теста диффузии прошли, включая изменение формы и света. Отдельный actual_blink_warm_light_matches_cold прошёл на реальной модели: индексы и позиции совпали точно, максимальная разница RGB 8.940697e-8. Волосы в этом сравнении отключены; тест проверяет освещение и геометрию лица, не весь вывод окна.

В коротких отдельных нативных release-запусках медиана диффузии снизилась с 51.719 мс (88 отсчётов) до 42.639 мс (96 отсчётов). Медиана CPU-пути draw для кадров 1–7: 90.977 → 80.687 мс. Журналы: /tmp/voxy-face-mesh-reused-profile.log и /tmp/voxy-face-warm-light-profile.log. Это разные короткие последовательности поз, без GPU fence; устойчивое улучшение FPS не доказано. Снимок текущего нативного окна /tmp/voxy-native-warm-light.png просмотрен: лицо отображается с розовой помадой. Плавная анимация и полная реалистичность лица остаются незавершёнными.

## Освещение и процедурный рельеф радужки — 2026-10-01

В female_material.wgsl освещение радужки отделено от склеры: радужка использует восстановленную ось глаза и нормаль небольшого процедурного рельефа, склера — прежнюю сферическую нормаль. Волокна дают высоту 25+15 мкм, углубления около колларетки — до 90 мкм; маска ограничена кольцом радужки. Производная высоты рассчитывается центральной разностью с шагом не меньше 40 мкм и увеличивается с размером пикселя. Это художественная процедурная модель, не измеренная анатомия и не новая геометрия для ray queries. Диффузная составляющая глаза умножается на дополнение используемого приближения Fresnel; отражение конечных источников сохраняется.

Release-команды voxy_ray_probe --experimental --face --eye-closeup и вариант --eye-side прошли на Apple M4 Max/Metal с тремя позами и аппаратными ray queries для видимости. Просмотрены /tmp/voxy-eye-iris-relief.png и /tmp/voxy-eye-iris-relief-side.png. Базовый кадр сохранён в /tmp/voxy-eye-iris-before.png. Различия фронтального рендера ограничены областями глаз (47869 пикселей); окружающая кожа совпадает. Четыре существующих release-теста female_eyes прошли: координаты склеры, материалы, замыкание волокон по углу, геометрия сферы. Эти тесты не доказывают физическую точность нового shader-рельефа; его компиляция и вывод проверены реальным GPU-рендером. Нативная производительность нового материала отдельно не измерена. Ресницы, край век, полное закрытие глаза и общая реалистичность остаются предметом дальнейшей работы.

## Ограничение изгиба ресниц длиной пряди — 2026-10-01

Фиксированный bow верхних ресниц мог менять знак начальной касательной коротких прядей. Новая lash_bend масштабирует вертикальный и передний изгиб компонентами направления конкретной пряди: коэффициенты 0.14 и 0.18 с детерминированной вариацией curl 0.7..1.3. Поэтому касательная открытой пряди сохраняет вертикальное направление и положительный Z по всей квадратичной кривой. Перенос формы при моргании и ограничение вращения у закрытого века сохраняются. Это ограничение формы groom, не модель контакта ресницы с кожей.

Добавлен тест actual_lash_curls_never_reverse_the_open_lid_root_tangent: все 144 верхние/нижние пряди настоящей модели, 33 положения касательной на каждой. Первый вариант коэффициента Z 0.22 не прошёл этот тест, заменён на 0.18. Новый аппаратный Metal-рендер трёх поз прошёл; просмотрен /tmp/voxy-lash-bounded-curl.png. Крючки у открытого века уменьшились, но часть корней скрывается в коже и кажется оторванной от края; при закрытии видны пересечения и завитки. Полная реалистичность и контакт ресниц не подтверждены, нужны дальнейшие изменения крепления.

Итоговый cargo test -p voxy_app --release female_features --lib прошёл: 15 тестов, включая новую проверку касательных и направления верхних ресниц при полном закрытии. Журнал /tmp/voxy-lash-bounded-final-tests.log. git diff --check прошёл.

## Корни ресниц по контуру глазной щели — 2026-10-01

FaceFeatures строит LidContour по исходным треугольникам и использует его несглаженный aperture_at для целевой высоты корней вместо условной синусоидальной дуги. Верхний ряд расположен на 0.20 мм выше верхней границы, нижний на 0.15 мм ниже нижней. Проекция обоих рядов на поверхность использует метрику (1,1,0.25) и переднюю целевую глубину 0.145 м; хранит прежние барицентрические координаты и привязку к треугольнику. Корни продолжают двигаться с деформацией кожи; это не отдельное смещение каждый кадр и не универсальный groom для любой модели.

Release-тесты female_features прошли. Аппаратный Metal-рендер трёх поз завершился успешно; просмотрен /tmp/voxy-lash-aperture-root.png. На открытом глазе верхние ресницы выходят из края века, прежний видимый отрыв уменьшился. При закрытии остаются завитки, пересечения и дефекты края глазной щели. Контакт ресниц и полное закрытие глаза не подтверждены. Журналы /tmp/voxy-lash-aperture-tests.log и /tmp/voxy-lash-aperture-root.log.

## Жевательные углубления задних коронок — 2026-10-01

В tooth_crown премоляры получают продольное углубление, моляры — также поперечное. Максимальная глубина 0.5 мм, воздействие ограничено жевательной стороной (cutting_side > 0.45) и направлено внутрь существующего объёма. Внешние размеры, режущие поверхности передних зубов и движение челюсти сохраняются. CrownEnvelope остаётся консервативной оболочкой: она не учитывает внутренние вырезы, поэтому контакт языка с этими вырезами не доказан.

Новый posterior_crowns_have_inward_fissures_and_mirrored_cutting_faces проверяет верхние и нижние коронки премоляра/моляра: центр на 2.1 мм вместо 2.6 мм, окружающие бугры выше 2.3 мм, размеры ограничены, индексы совпадают. Все 16 release-тестов female_features прошли; журнал /tmp/voxy-dental-fissure-final-tests.log. По VOXY_DENTAL_MESH_CAPTURE тест экспортировал настоящие позиции и индексы в /tmp/voxy-dental-fissure-mesh.json. Их научная 3D-визуализация /tmp/voxy-dental-fissure-geometry.png просмотрена: центральная впадина есть, но радиальная сетка слишком грубая для подробных жевательных бугров.

Рендер voxy_ray_probe --experimental --face --oral-closeup --mouth --oral-above прошёл на Metal с аппаратной видимостью; просмотрен /tmp/voxy-dental-fissure-render.png. Задние жевательные поверхности частично скрыты губами и языком, поэтому общий вид рта не доказывает видимость всех новых деталей. Передний край губ и внутренняя поверхность рта по-прежнему имеют заметные геометрические дефекты. В рабочем дереве идут параллельные изменения; изменение числа треугольников между сохранённым baseline и новым рендером не позволяет считать их изолированным пиксельным сравнением только этой правки.

## Уплотнение жевательных поверхностей — 2026-10-01

Задние коронки теперь строятся с 32 широтными рядами и 48 сегментами вместо 16×32. Широты распределены по 0.5*(1-cos(pi*t)): больше точек около полюсов, где преобразование суперэллипсоида раньше растягивало первый ряд и оставляло большой веер треугольников над углублениями. Передние коронки и остальные эллипсоиды сохраняют прежнее равномерное 16×32 построение. Число вершин задней коронки 561 → 1617, треугольников 960 → 2976; двадцать задних коронок добавляют 40320 треугольников. Нативная стоимость кадра отдельно не измерена.

Все 16 release-тестов female_features прошли; /tmp/voxy-dental-dense-tests.log. Настоящая сетка экспортирована в /tmp/voxy-dental-dense-mesh.json, визуализация /tmp/voxy-dental-dense-geometry.png просмотрена: углубления представлены дополнительными кольцами вершин вместо прежнего крупного центрального веера. Рендер открытого рта сверху на Metal с ray queries прошёл, просмотрен /tmp/voxy-dental-dense-render.png; журнал /tmp/voxy-dental-dense-render.log. Основные дефекты уголков губ, внутреннего края рта и ограниченная видимость задних коронок сохраняются. Уплотнение не доказывает анатомическую точность коронок или корректный контакт языка внутри фиссур.

## Контакт языка с жевательными углублениями — 2026-10-01

CrownEnvelope задних зубов теперь вычисляет высоту жевательной поверхности из суперэллипсоида по X/Z и вычитает crown_fissure_depth. Эта функция общая с tooth_crown, поэтому прежний гладкий объём не заполняет фиссуры в проверке контакта. Противоположная, пришеечная сторона остаётся прежней. Имеющийся запас радиусов 0.2 мм и алгоритм отступа языка сохраняются: это аналитическая консервативная оболочка, не точное пересечение полигональной сетки и не динамика мягких тканей.

Новый crown_contact_excludes_fissure_space_but_keeps_cusps_and_cervical_face проверяет премоляр и моляр, обе челюсти, с переносом и поворотом: точка над центральной впадиной свободна, ниже впадины и внутри бугра — занята, пришеечная сторона сохраняется, внешняя точка свободна. Все 17 release-тестов female_features прошли; /tmp/voxy-dental-contact-tests.log. Рендер --oral-closeup --tongue-transition --oral-above прошёл на Metal с аппаратной видимостью трёх поз. Просмотрен /tmp/voxy-dental-fissure-contact.png; /tmp/voxy-dental-contact-render.log. Видимые дефекты уголков губ и внутренней границы рта сохраняются. Три изображения не доказывают отсутствие всех пересечений языка с зубами при любых параметрах конструктора.

## Конечная площадь света для влажных поверхностей рта — 2026-10-01

В female_material.wgsl блик языка и общей ветки oral (зубы, десна, полость) заменён с одного GGX-направления на среднее 16 направлений диска ключевого света радиуса 0.22. Шероховатость и существующий коэффициент видимости сохраняются. Видимость по-прежнему одна на вершину для всего диска; это не 16 отдельных лучей на пиксель. Кожа, глаза и геометрия не менялись этой правкой. GPU-стоимость нового shader отдельно не измерена.

Рендер --oral-closeup --tongue-transition --oral-above прошёл на Apple M4 Max/Metal с тремя позами; журнал /tmp/voxy-oral-area-light.log. Просмотрен /tmp/voxy-oral-area-light.png. При сравнении с /tmp/voxy-dental-fissure-contact.png изменились 355 пикселей, максимум 9 уровней 8-битного цвета. В этом затенённом ракурсе эффект слабый; значительное визуальное улучшение не заявляется. Компиляция shader и вывод подтверждены настоящим GPU-рендером; новых математических unit-тестов для этой ограниченной замены не добавлено. Геометрические дефекты края губ остаются.

## Передний участок губ и вершины вне поверхности диффузии — 2026-10-01

Расширение заменяемых треугольников на весь пояс ±3 мм захватило 1201 исходный треугольник; вариант отклонён. Текущая выборка ограничивает дополнительные треугольники передним слоем Z=0.150..0.155 м и оставляет прежние пересечения шва: 340 исходных треугольников вместо 227. Проверочный предел числа удаляемых треугольников изменён с 250 на 400 для этого расширенного участка; вариант 1201 остаётся за пределом. Нейтральная площадь и сохранение остальной исходной сетки прошли в тестах female_features. Замена по-прежнему использует двухуровневое подразделение и коррекцию нелинейного веса челюсти.

Оба первых рендера отказали. Добавленный trace сохранил первопричину: face mesh diffusion rejected: unsupported diffusion vertex. После удаления участка некоторые исходные вершины не принадлежат ни одному треугольнику. diffuse_with_initial теперь сжимает используемые вершины, индексы, источник и начальное приближение для решения; возвращает результат на прежние индексы, а значения вне области треугольников сохраняет равными источнику. Математическое ядро и проверки сходимости не изменены. Новый тест unused_render_vertices_do_not_change_the_surface_solution проверяет эквивалентность настоящей области, сохранение неиспользуемой вершины и отказ неверного индекса.

Интегрированный Metal-рендер закрытого, полуоткрытого и открытого рта прошёл; просмотрен /tmp/voxy-lip-front-integrated-render.png. Угловатая внутренняя полоса остаётся, её устранение не заявляется. Журналы /tmp/voxy-inner-lip-rejection-trace.log и /tmp/voxy-lip-front-integrated-render.log.

Полный cargo test -p voxy_app --release --lib завершился: 172 passed, 2 failed, 25 ignored; /tmp/voxy-lip-front-integrated-tests.log. Два нерешённых случая: complexion_follows_face_animation_and_leaves_eyes_untinted требует одинаковых UV всех вершин при смене позы; grasp_preset_switch_preserves_pose_and_failed_load_keeps_target требует побитово одинакового цвета (наблюдаемая разница зелёного канала 0.33229366 против 0.33229363). Их происхождение относительно этого изменения отдельно не установлено; общий набор не считается зелёным. Требуется локализация UV и проверка контракта стабильности численного освещения.

## Стабильные UV языка и повторное освещение одной позы — 2026-10-01

Локализован прежний UV-отказ: вершина языка 154972 меняла V с 0.5 на 0.4999998 при движении головы. append_tongue раньше строил эллипсоид в мировой системе и восстанавливал локальные координаты через обратную матрицу головы. Теперь эллипсоид, форма и материал вычисляются локально, затем готовая вершина один раз переносится матрицей головы. Тест неизменности UV остаётся точным; добавлена диагностика индекса и позиций при отказе.

Для повторного освещения FemaleDemo сохраняет SurfaceLightInput (точки, треугольники, текущий источник) вместе с последним решением. Только точное равенство всех входных массивов позволяет вернуть готовый результат без нового численного решения. При любом изменении используется прежний warm-start с расчётом текущей системы и проверками сходимости. Это не сравнение времени позы и не приблизительный hash; изменение геометрии или освещения не допускает возврат старого результата. Радиусы диффузии по-прежнему постоянны в этом вызове. Кэш хранит дополнительные входные массивы; ускорение изменяющейся анимации этим изменением не заявляется.

Повторный полный release-набор: 174 passed, 1 failed, 25 ignored; /tmp/voxy-material-stability-all-tests.log. Прежний точный тест complexion_follows_face_animation_and_leaves_eyes_untinted теперь проходит. Проверка grasp_preset_switch_preserves_pose_and_failed_load_keeps_target всё ещё падает с прежней разницей зелёного канала 0.33229366 → 0.33229363; её критерий не ослаблен. Кэш точного входа не доказал устранение этого случая, требуется сравнение входов до и после переключения. Общая регрессионная проверка остаётся незавершённой.

Новый аппаратный Metal-рендер трёх положений языка прошёл; просмотрен /tmp/voxy-tongue-bind-uv-stable.png, журнал /tmp/voxy-material-stability-render.log. Этот кадр показывает сохранение вывода, а стабильность координат материала подтверждает точный тест, не визуальная оценка снимка. Геометрические дефекты внутреннего края губ сохраняются.

## Более строгая сходимость освещения — 2026-10-01

Диагностика оставшегося grasp-теста установила точное равенство точек, треугольников и источника до/после переключения. Цвет менялся у вершины 214 на один f32-уровень. Между сравниваемыми кадрами тест рисует другую позу, поэтому кэш последнего входа закономерно не возвращает первоначальное решение. Warm-start приходит к той же системе с другой численной историей.

Квадрат относительного критерия остатка PCG ужесточён с 1e-20 до 1e-28 (по норме 1e-10 → 1e-14). Проверка фактического остатка и энергии сохраняется. Точный критерий grasp_preset_switch_preserves_pose_and_failed_load_keeps_target не изменён; тест прошёл. Пять обычных тестов skin_light_diffusion прошли; один диагностический benchmark проигнорирован. Журналы /tmp/voxy-grasp-tight-residual.log и /tmp/voxy-tight-diffusion-tests.log. Это проверенная стабильность данного сценария, не доказательство побитового равенства любых решений на всех платформах.

Аппаратный Metal-рендер трёх положений языка прошёл; /tmp/voxy-tight-diffusion-render.log, просмотрен /tmp/voxy-tight-residual-mouth.png. В этом offscreen-запуске стадия диффузии: первый расчёт 81.590 мс, два точных повторения входа 1.237 и 1.206 мс благодаря кэшу. Это короткий CPU-профиль данной сцены, не нативный FPS или сравнение с одинаковым baseline. Цена пересчёта остаётся высокой; более строгая сходимость требует дальнейшей оптимизации. Геометрические дефекты внутреннего края губ не устранены.

Повторный полный release-набор приложения завершился успешно; итоговый журнал /tmp/voxy-tight-residual-all-tests.log. Ранее упавшие точные тесты UV и повторного выбора grasp-объекта проходят. Это закрывает найденные два регрессионных случая, но не полную цель реалистичной лицевой анимации.

## Непрерывные десневые дуги — 2026-10-01

Отдельные эллипсоиды-воротники вокруг 32 зубов заменены двумя связными десневыми дугами. Каждая: 128 продольных интервалов, 16 сторон сечения, замкнутые задние торцы, 2066 вершин и 4128 треугольников. Небольшой scallop 0.6 мм следует интервалам зубов; сечение расширяется к задним зубам. Нижняя дуга использует прежнее смещение gum_y вместе с челюстью. Модель процедурная: индивидуальная форма десневых сосочков и контакт с каждой коронкой ещё не проверены. По сравнению с прежними 32 эллипсоидами убрано 22464 треугольника, но производительность отдельно не измерена.

Все 18 release-тестов female_features прошли. Новая gingival_arch_is_closed_connected_and_faces_outward дополнительно проверена после добавления обхода графа: все вершины достижимы, каждое ребро принадлежит двум треугольникам с противоположной ориентацией, нет вырожденных треугольников, характеристика Эйлера 2, передняя поверхность направлена наружу. Журналы /tmp/voxy-continuous-gums-final-tests.log и /tmp/voxy-gums-connected-capture.log. Настоящие позиции и индексы выгружены тестом в /tmp/voxy-gums-connected-mesh.json; визуализация /tmp/voxy-connected-gums-geometry.png просмотрена.

Рендер трёх положений челюсти на Metal с аппаратной видимостью прошёл; просмотрен /tmp/voxy-continuous-gums.png, журнал /tmp/voxy-continuous-gums-final-render.log. В этом ракурсе дёсны почти скрыты, общий вид изменился мало. Дефекты внутреннего края губ сохраняются. Полный набор приложения после этой геометрической правки не повторялся; предыдущий полный успешный прогон относится к более строгой диффузии до замены дёсен.

## Волна только по краю десны — 2026-10-01

Scallop десневой дуги теперь умножается на квадрат положительной компоненты сечения, обращённой к зубам. Противоположное основание не смещается целиком вверх/вниз вместе с каждым зубным интервалом. Реальная экспортированная верхняя сетка: диапазон высоты основания 0 мм, диапазон зубного края около 1.20003 мм. Число вершин и индексов не изменилось. Это уточнение процедурной формы, не доказательство индивидуального соответствия каждой коронке.

Все 18 release-тестов female_features прошли, включая замкнутость, связность, ориентацию и движение нижней челюсти. Журнал /tmp/voxy-gums-edge-tests.log. Просмотрена визуализация фактических позиций/индексов /tmp/voxy-gums-edge-geometry.png из /tmp/voxy-gums-edge-mesh.json. Новый Metal-кадр на этом этапе не снимался; предыдущий общий рендер относится к версии до ограничения волны. Визуализация показывает именно сетку текущего изменения.

## Интегрированная проверка края дёсен и материал глотки — 2026-10-01

Текущая форма дёсен с ограниченной волной проверена в Metal-рендере закрытого, полуоткрытого и открытого рта: /tmp/voxy-gums-edge-integrated.png просмотрен; /tmp/voxy-gum-edge-integrated.log. В этом ракурсе видимость дёсен ограничена, отсутствие пересечений с каждой коронкой не доказано.

Вершины туннеля глотки имели UV=[0,0], направлявшие их в общий материал. Теперь они используют oral-метку [-1,0.55], то есть существующий влажный материал с roughness=0.55 и конечной площадью ключевого света. Геометрия, цветовой градиент глубины и движение туннеля сохраняются; анатомическая точность глотки этим не устанавливается. Все 18 release-тестов female_features прошли (/tmp/voxy-throat-wet-tests.log). Новый аппаратный Metal-рендер трёх положений челюсти прошёл и просмотрен: /tmp/voxy-throat-wet-material.png, журнал /tmp/voxy-throat-wet-render.log. Полный набор приложения на этом этапе не повторялся. Угловатый внутренний край губ остаётся видимым.

## Неравномерные динамические складки лба — 2026-10-01

Равномерная синусоидальная волна заменена тремя отдельными складками в bind-пространстве: уровни Y 0.752/0.764/0.776 м, ширины 1.0/1.2/1.4 мм, максимальные коэффициенты углубления 0.8/0.65/0.45 мм до маски лба и величины подъёма бровей. Линии слегка изгибаются по X, имеют различную длину и плавное затухание по бокам. Рядом с каждой впадиной формируется небольшой положительный валик (0.25 глубины). При нейтральных и сведённых бровях вклад горизонтальных складок равен нулю; прежние вертикальные складки межбровья сохраняются. Это художественная модель деформации, не анатомический расчёт мышц.

Все 10 обычных release-тестов female_face прошли, 3 диагностических теста пропущены; /tmp/voxy-forehead-fold-tests.log. Аппаратный Metal-рендер --brow-transition --skin-closeup прошёл; просмотрены исходный /tmp/voxy-forehead-current.png и новый /tmp/voxy-forehead-folds.png. Журнал /tmp/voxy-forehead-fold-render.log. В текущем освещении горизонтальный рельеф всё ещё слабый; убедительная реалистичность морщин не подтверждена. Нужна проверка передачи нормалей/разрешения рельефа и более крупного ракурса. CPU-стоимость нового расчёта отдельно не измерена; полный набор приложения не повторялся.

## Сравнение нормалей лба — 2026-10-01

Добавлен исключительно диагностический VOXY_FACE_DIAGNOSTIC_GEOMETRIC_NORMALS=1. Он рассчитывает нормали по площадям текущих треугольников через PreparedNormals::geometric, выводит отличие от переносимых исходных нормалей и использует прямой вариант для CPU-источника освещения и привязанных деталей. Обычный режим не меняется. Прямой вариант не сваривает дубли индексов по UV-швам; это диагностическое сравнение, не новый универсальный алгоритм нормалей. GPU-нормали SceneRenderer по-прежнему рассчитываются его обычным путём.

На 432 вершинах области X±0.05 м, Y=0.748..0.785 м, Z>0.11 м: среднее/максимум в градусах для сведённых бровей 0.2274/1.1843, покоя 0.2309/1.1431, поднятых 0.3676/2.9972. Аппаратный Metal-рендер прошёл; /tmp/voxy-forehead-direct-normals.log, просмотрен /tmp/voxy-forehead-direct-normals.png. В сравнении с /tmp/voxy-forehead-folds.png прямые нормали не сделали горизонтальные складки убедительно выраженными. Вывод ограничен этим ракурсом и вариантом расчёта: перенос исходных нормалей не подтверждён как главная причина слабого рельефа. Следующие проверяемые факторы — разрешение геометрии и сглаживание источника освещения диффузией. Тесты surface_normals прошли; /tmp/voxy-forehead-normal-tests.log.

## Лоб без диффузии света — 2026-10-02

Диагностический VOXY_FACE_DIAGNOSTIC_NO_DIFFUSION=1 отключает только CPU-диффузию источника освещения поверхности. Без переменной обычный режим сохраняется. Геометрия, перенос нормалей, материал и аппаратная проверка видимости остаются прежними. Release-сборка и Metal-рендер трёх положений бровей прошли: /tmp/voxy-forehead-no-diffusion.log; просмотрены /tmp/voxy-forehead-no-diffusion.png и обычный /tmp/voxy-forehead-folds.png.

Изменилось 470177 пикселей, максимальная разница канала 36/255, средняя по всем каналам изображения 0.3613/255. Эти числа относятся ко всему кадру, а не только к морщинам. Визуально горизонтальные складки по-прежнему слабо различимы; отключение диффузии не дало убедительного улучшения в данном ракурсе. Основная причина слабого рельефа ещё не установлена. Следующий диагностический этап — плотность сетки относительно миллиметровой ширины складок. Полный набор тестов приложения в этом этапе не повторялся.

## Разрешение геометрии лба — 2026-10-02

Прочитаны реальные позиции и грани body-nipple-refined.obj. В области X±0.05 м, Y=0.748..0.785 м, Z>0.11 м находятся 432 вершины; 766 треугольников полностью лежат в области. Для 1195 уникальных рёбер этих треугольников длины в bind-пространстве: P10=2.8856 мм, медиана=3.5601 мм, P90=4.8636 мм, максимум=5.8112 мм. Это исходная геометрия, не размеры экранных пикселей и не статистика после деформации.

Просмотрен /tmp/voxy-forehead-mesh-resolution.png: реальная триангуляция в проекции XY вместе с центрами трёх процедурных складок и полосами ±1/1.2/1.4 мм. Узкий профиль слабо дискретизируется исходной сеткой; увеличение глубины само по себе не добавит недостающих отсчётов формы. Это измеренное ограничение, но ещё не доказательство единственной причины слабой видимости. Следующая реализационная проверка — локальное уплотнение с сохранением общих границ, UV и привязок SkinEmbedding; после неё нужен новый Metal-рендер. Геометрия приложения на этом диагностическом этапе не изменена.

## Локальное уплотнение лба — 2026-10-02

tools/refine_forehead.py создаёт отдельные body-forehead-refined.obj и body-forehead-refined-skin.json из прежних nipple-refined источников. FemaleDemo теперь использует новые файлы. Конформное разбиение отмеченных рёбер действует в области X±0.060 м, Y=0.742..0.795 м, Z>0.10 м; соседние треугольники разделяются по тем же рёбрам, чтобы не создавать T-стыки. Максимальная длина локального ребра 1.199857 мм. 45203→60866 вершин, 90402→121728 треугольников, то есть +15663 вершины и +31326 треугольников. Исходные позиции/нормали сохранены; новые позиции — середины рёбер, новые нормали интерполируются. Источник не имеет UV-швов; preparer явно отклоняет неподдерживаемую структуру OBJ.

Аудит body-forehead-refined.audit.json: границ, неманифолдных рёбер, неверной ориентации, дубликатов и вырожденных треугольников нет; изменение площади около 4.1e-14 м². Все данные физической оболочки и исходные позиции/веса/треугольники привязок независимо сравнены и совпадают. Для новых точек созданы выпуклые ближайшие барицентрические привязки к прежней оболочке; максимальное расстояние 2.904 мм. Это расстояние между подробной поверхностью и грубой оболочкой, не смещение нейтральной поверхности: SkinEmbedding переносит разность перемещений. Влияние добавленных вершин на время кадра отдельно не измерено.

Два release-теста лба прошли: /tmp/voxy-forehead-refined-tests.log. Metal-рендер трёх положений бровей прошёл с 918719 треугольниками всей сцены и 12018024 лучевыми сегментами на позу; /tmp/voxy-forehead-refined-render.log, просмотрен /tmp/voxy-forehead-refined-render.png. Повтор с прямыми CPU-нормалями также прошёл: /tmp/voxy-forehead-refined-normals.log, просмотрен /tmp/voxy-forehead-refined-direct-normals.png. В области прежней диагностики теперь 9287 вершин; средние/максимальные отличия нормалей в градусах: 0.8462/3.0565, 0.8532/3.1274, 0.8921/6.6329. Уплотнение изменило детализацию теней, но горизонтальные складки всё ещё слабо читаются в текущем свете; ни плотная сетка, ни прямые нормали сами по себе не подтвердили желаемую реалистичность. Следующая проверка — раздельный вклад макрорельефа в диффузное освещение и бликовый материал.

Полный release-прогон приложения: 181 успешный тест, 2 ошибки устаревших точных ожиданий 45203 вместо 60866 вершин, 25 пропущенных; /tmp/voxy-forehead-refined-all-tests.log. После обновления этих ожиданий оба теста отдельно прошли: /tmp/voxy-forehead-refined-landmark-test.log и /tmp/voxy-forehead-refined-source-switch-test.log. Они по-прежнему проверяют полное покрытие привязками, нулевой перенос при одинаковых состояниях оболочки, локальную деформацию и переключение модели; проверки не ослаблены. Полный набор после изменения только этих ожиданий повторно не запускался. git diff --check прошёл.

## Прямой и рассеянный свет кожи — 2026-10-02

Совместная диагностика прямых CPU-нормалей и отключённой диффузии на плотной сетке показала различимую нижнюю горизонтальную складку при поднятых бровях. Просмотрен /tmp/voxy-forehead-dense-direct-no-diffusion.png, журнал /tmp/voxy-forehead-dense-direct-no-diffusion.log. Этот комбинированный опыт сам по себе не разделяет вклад двух изменений.

Добавлен диагностический флаг voxy_ray_probe --skin-macro-light: только для текстурированной кожи он подставляет фиксированный пигмент, рассчитывает диффузный свет и конечный блик непосредственно по гладкой нормали текущей GPU-геометрии, исключая микрорельеф атласа и запечённый CPU-свет. Ключевой/заполняющий источники и существующая вершинная аппаратная видимость сохраняются. Без флага материал прежний. Подстановка проверяет наличие ожидаемого исходного фрагмента и выдаёт ошибку при несовпадении. Metal-рендер и проверка WGSL прошли; /tmp/voxy-forehead-macro-light.log, просмотрен /tmp/voxy-forehead-macro-light.png: при поднятых бровях различимы три полосы рельефа. Это диагностический материал, не готовая реалистичная кожа.

В обычном FemaleDemo диффузный RGB-свет теперь смешивает 35% локального источника и 65% результата прежней поверхностной диффузии. Выпуклая смесь сохраняет уровень равномерного освещения и не отключает рассеяние. Доли являются художественным приближением двух глубин отклика, не измеренными оптическими свойствами кожи. Атлас, перенос нормалей, геометрия, блики и аппаратная видимость не менялись. Новый обычный Metal-рендер прошёл; /tmp/voxy-forehead-surface-blend.log, просмотрен /tmp/voxy-forehead-surface-blend.png. Нижняя складка стала заметнее, верхние остаются слабыми; полноценная реалистичность ещё не доказана. Следующий этап — управление глубиной отклика и бликами с проверкой в разных ракурсах.

После изменения смеси прошли release-регрессии complexion_follows_face_animation_and_leaves_eyes_untinted и grasp_preset_switch_preserves_pose_and_failed_load_keeps_target, включая точные сравнения UV/цвета и сохранение позы при переключении: /tmp/voxy-forehead-surface-blend-complexion-test.log и /tmp/voxy-forehead-surface-blend-grasp-test.log. Полный набор на данном этапе не повторялся. git diff --check прошёл.

## Управление глубиной отклика кожи — 2026-10-02

В FaceParameters и face-controls.json добавлен skin_surface_light, раздел «Материал кожи», подпись «Доля поверхностного света», диапазон 0..1 с шагом 0.01, значение по умолчанию 0.35. Ноль оставляет весь диффузный источник рассеянным, единица использует локальный источник; промежуточные значения смешивают их. Это художественный коэффициент, не длина проникновения света в миллиметрах. Он применяется после расчёта источника и не входит в сигнатуру атласа: изменение не требует перестроения пор/макияжа. При единице диффузия вообще не вызывается, поскольку её вес нулевой; ранее сохранённое решение остаётся в кэше для последующего использования с проверкой исходных данных.

В ray probe добавлен --skin-depth-transition: три кадра с одинаковыми позой поднятых бровей, камерой и временем, коэффициенты 0 / 0.35 / 1. При переданном --face-preset остальные его параметры сохраняются. Выход /tmp/voxy-skin-depth-transition.png. Флаг предназначен для сравнения материала, не для анимации возраста.

Четыре release-теста face_parameters прошли, включая совпадение каталога со схемой: /tmp/voxy-skin-depth-parameters-tests.log. Новая интеграционная проверка surface_light_control_changes_irradiance_without_rebuilding_diffusion_or_geometry прошла после оптимизации крайнего значения: /tmp/voxy-skin-depth-integration-test.log. Она сравнивает реальные сетки для 0 и 1: позиции, UV и индексы совпадают, меняется цвет более 100 вершин, сигнатура атласа и сохранённое решение диффузии остаются точными. Это проверка подключения управления, не доказательство физической достоверности коэффициента.

Итоговый release Metal-рендер после оптимизации прошёл, /tmp/voxy-skin-depth-transition-final.log; просмотрен /tmp/voxy-skin-depth-transition.png. Во всех трёх кадрах одинаковы 918719 треугольников, 12018024 лучевых сегмента и 8913706 перекрытых сегментов; таким образом сравнение освещения не меняет геометрию или лучевую видимость. В правом кадре нижняя складка контрастнее, верхние остаются слабыми. Native UI в этом этапе не открывался; доказательство визуального эффекта относится к offscreen Metal-рендеру. Полный набор приложения не повторялся; git diff --check прошёл.

## Масляный блик поверх кожи — 2026-10-02

skin_area_highlight теперь ослабляет нижний GGX-блик коэффициентом (1−coverage·F_in)(1−coverage·F_out), где coverage=0.35·oil, а F — Schlick с F0=0.035 для верхнего масляного слоя. Верхний GGX использует F0=0.035 вместо прежнего общего 0.028; исходный GGX кожи/оральных поверхностей через ggx сохраняет F0=0.028. Оба вклада по-прежнему усредняются по 16 направлениям конечного источника. Это приближение слоистого отражения с художественным коэффициентом покрытия; рефракция в слое и полный энергетический баланс рассеянного света не рассчитываются. Новые оптические коэффициенты не заявлены как измеренные свойства себума.

--skin-finish-transition в сочетании с --skin-closeup теперь фиксирует поднятые брови во всех трёх кадрах. Сравниваются матовый материал (roughness=0.8, oil=0), исходный атлас и жирный материал (roughness=0.28, oil=0.8); пигмент и высота атласа сохраняются. Добавлен --skin-side: камера X=0.065 м при прежнем центре наведения, остальные параметры прежние.

Просмотрены исходный /tmp/voxy-forehead-finish-baseline.png и новый /tmp/voxy-forehead-oil-coat.png; GPU-сборка/Metal-рендер прошли, журналы /tmp/voxy-forehead-finish-baseline.log и /tmp/voxy-forehead-oil-coat.log. Матовый кадр побитово совпадает. В исходном материале изменилось 12355 пикселей (максимум канала 2/255, среднее 0.01191/255), в жирном 15027 (3/255 и 0.02436/255). Правка верхнего слоя визуально небольшая. Само сравнение материалов показывает три складки в блике жирного лба; на матовой коже верхние складки остаются слабыми. Это свидетельство наличия рельефа и зависимости его читаемости от материала, не доказательство естественности формы складок.

Боковой Metal-рендер также прошёл: /tmp/voxy-forehead-oil-coat-side.log, просмотрен /tmp/voxy-forehead-oil-coat-side.png. Во всех трёх материалах одинаковы 918719 треугольников, 12018024 сегмента и 8911630 перекрытых сегментов. В боковом ракурсе масляный блик следует форме лба и проявляет складки; их регулярная процедурная форма всё ещё заметна. Полный набор CPU-тестов для изменения WGSL не запускался: проверены компиляция/валидация реального GPU-материала, фронтальные изображения до/после и боковой результат. Во время сборки устранён конфликт заимствования в voxy_editor::tick: mutable-доступ к import worker повторно получен перед отправкой следующего импорта, после завершения работы с каталогом. Логика очереди не изменялась. git diff --check прошёл.

## Неравномерный профиль мимических складок — 2026-10-02

Три складки теперь используют разные постоянные фазы 0/1.7/3.4 в bind-пространстве. Ширина меняется плавно в пределах 80–100% прежней, сила в пределах 55–100%; центр дополнительно изгибается двумя волнами амплитуд 0.4 и 0.15 мм. Исходные уровни, длины, максимальные коэффициенты глубины и плавное боковое затухание сохранены. Вариация не зависит от времени и следует коже при движении. Это художественная процедурная модель; данные анатомических измерений не использованы. Расчёт профилей теперь выполняется только при положительном подъёме бровей и ненулевой маске лба; отдельный выигрыш времени не измерялся.

Два release-теста лба прошли, включая восстановление без накопления и сохранение статического рельефа при подъёме бровей: /tmp/voxy-forehead-varied-folds-tests.log. Боковой Metal-рендер матового/исходного/жирного материалов прошёл: /tmp/voxy-forehead-varied-folds.log, просмотрен /tmp/voxy-forehead-varied-folds.png и предыдущий /tmp/voxy-forehead-oil-coat-side.png. Линии стали менее равномерными, но изменение небольшое; естественность формы и реалистичность всего лица ещё не подтверждены. Полный набор приложения не повторялся; git diff --check прошёл.

## Точное сравнение фаз закрытия век — 2026-10-02

На текущей модели снят и просмотрен крупный Metal-рендер штатного моргания /tmp/voxy-blink-transition.png; /tmp/voxy-current-blink.log. В фазе максимального закрытия видны острые защипы кожи выше обоих уголков глаза и изломы ресниц у контактного края. Это наблюдение изображения, не измерение числа пересечений или полной герметичности века.

FacePreview::sample_blink теперь позволяет независимо задавать одинаковое закрытие обоих глаз, проверяя конечность входов и диапазон 0..1. Ray probe --lid-close-transition фиксирует время/позу головы и выводит закрытие 0 / 0.5 / 1; прочие выражения нейтральны, взгляд направлен к той же камере. Снятый и просмотренный /tmp/voxy-lid-close-transition.png воспроизводит защипы при полном закрытии; release-сборка, GPU-валидация и три аппаратных лучевых прохода прошли, /tmp/voxy-exact-lid-closure.log. Сам алгоритм деформации век на этом этапе не менялся. Следующая правка должна уменьшить локальное сжатие кожи возле уголков, сохраняя реальную границу контакта с глазом; одного успешного рендера недостаточно для доказательства отсутствия щелей. Полный набор тестов приложения не повторялся; git diff --check прошёл.

## Отклонённое расширение сглаживания контура век — 2026-10-02

Проверено увеличение sigma сглаживания вспомогательного контура с 0.875 до 1.53 мм (29 отсчётов вместо 17); сырой контактный контур не менялся. 10 обычных release-тестов female_face прошли, 3 диагностических пропущены; /tmp/voxy-lid-smoother-transfer-tests.log. Metal-рендер 0/0.5/1 закрытия прошёл, /tmp/voxy-lid-smoother-transfer.log; просмотрен кандидат /tmp/voxy-lid-smoother-transfer.png. Защипы остались почти такими же.

Отдельно выполнен audit_lid_triangle_orientation; /tmp/voxy-lid-smoother-orientation.log и /tmp/voxy-lid-smoother-topology.csv. При полном закрытии среди 3582 треугольников области минимальное отношение площади к исходной стало 0.14093347, 16 обращённых нормалей, 32 обращённых проекции, ни одного треугольника меньше 1% площади. Это диагностические измерения, не gate отсутствия дефектов.

Правка откатана. Повторный аудит восстановленного алгоритма прошёл; /tmp/voxy-lid-restored-orientation.log, /tmp/voxy-lid-restored-topology.csv: минимальное отношение 0.27550527, 18 обращённых нормалей, 33 проекции. Несмотря на небольшое уменьшение числа обращений в кандидате, худшее локальное сжатие и отсутствие убедительного визуального улучшения не оправдывают его сохранение. Текущий алгоритм снова использует sigma 0.875 мм. Изображение smoother-transfer относится только к отклонённому опыту. Следующая проверка должна исследовать классификацию верхних/нижних слоёв и поле перемещений возле уголков, а не ещё сильнее сглаживать профиль. git diff --check прошёл.

## Ограничение скольжения по сфере глаза — 2026-10-02

Проверенная замена классификации верхнего/нижнего слоя на локальную середину сырой апертуры не изменила проблему. 10 тестов прошли, рендер просмотрен (/tmp/voxy-lid-local-midline.png), показатели ориентации остались прежними; изменение откатано. При разборе фактических записей audit_lid_triangle_orientation обнаружены обращённые треугольники над наружным уголком с отношениями площади около 3.9–5.7: они растягиваются, а не только сжимаются. Их центры Y≈0.721–0.723 м.

В deform_with_contour поправка Z, переносящая кожу по изменившейся кривизне сферы глаза, теперь полностью действует на расстоянии до 1.5 мм от сырого края апертуры и плавно затухает до нуля к 4 мм. Окружающая кожа больше не обязана следовать сфере далеко от контактного края. Вертикальное движение, исходный профиль, граница апертуры, ограничение коррекции разделённых слоёв и sigma 0.875 мм сохранены.

10 обычных release-тестов female_face прошли, 3 диагностических пропущены; /tmp/voxy-lid-bounded-glide-tests.log. Три отдельные диагностики выполнены: ориентация (/tmp/voxy-lid-bounded-glide-orientation.log, /tmp/voxy-lid-bounded-glide-topology.csv), покрытие реального глаза с ободками и шагом 125 мкм (/tmp/voxy-lid-bounded-glide-coverage.log, /tmp/voxy-lid-bounded-glide-coverage.csv), строгие поперечные пересечения (/tmp/voxy-lid-bounded-glide-intersections.log). При полном закрытии обращённых нормалей 14 вместо 18, минимальное отношение площади прежнее 0.27550527, обращённых проекций прежние 33. В диагностике покрытия видны прежние 71 из 53428 отсчётов: полная герметичность ещё не достигнута, ухудшения этого показателя не обнаружено.

Metal-рендер закрытия 0/0.5/1 прошёл; /tmp/voxy-lid-bounded-glide.log, просмотрен /tmp/voxy-lid-bounded-glide.png. Выраженные защипы над уголками исчезли в данном ракурсе; дефекты контактного края и ресниц сохраняются. Полный набор приложения не повторялся. git diff --check прошёл.

## Более плотная дискретизация ресниц — 2026-10-02

curved_lash использует 16 пролётов вместо 8 при прежних 6 сторонах сечения. Опорные точки, барицентрические привязки, аналитическая кривая, завивка, ограничение поворота, толщина и затухание радиуса не менялись. На ресницу теперь 102 вершины и 192 треугольника вместо 54/96. Для 144 ресниц это +6912 вершин и +13824 треугольника; вся сцена в ray probe 932543 треугольника. Время кадра отдельно не измерялось. Для квадратичной центральной линии максимальная ошибка линейной хорды каждого равномерного пролёта пропорциональна 1/N², поэтому ошибка центральной линии уменьшена в четыре раза; это не оценка всей поверхности трубки или контактной ошибки.

Metal-рендер 0/0.5/1 закрытия прошёл, /tmp/voxy-lash-finer-curves.log. Просмотрен /tmp/voxy-lash-finer-curves.png и предыдущий /tmp/voxy-lid-bounded-glide.png. Изменение визуально небольшое; увеличение детализации само по себе не устраняет загибы при закрытии или возможный контакт ресниц с кожей. Дальнейшая проверка должна исследовать ориентацию и контакт, а не только число сегментов.

Прогон female_features: 17 тестов прошли, один завершился на старом ожидании 7776 вместо 14688 lash-вершин, /tmp/voxy-lash-finer-curves-tests.log. После обновления точного числа вершин тест attachments_follow_surface_and_topology_is_stable повторно прошёл, /tmp/voxy-lash-finer-curves-attachment-test.log; проверки привязок и стабильности топологии сохранены. Остальные 17 проверок относятся к той же геометрической правке до изменения только тестового ожидания. Полный набор приложения не повторялся; git diff --check прошёл.

## Пересечения центральных линий ресниц с кожей — 2026-10-02

Добавлена игнорируемая диагностика FemaleDemo::tests::audit_lash_skin_crossings. Она строит текущую фактическую mesh для закрытия 0/0.5/1, выделяет 144 ресницы по material UV, восстанавливает центры 17 колец усреднением шести реальных вершин и проверяет каждую хорду на строгое поперечное пересечение с текущими треугольниками кожи. Используется общий f64-предикат из диагностики век, а не другой упрощённый тест. Для отбора области кожа помечается по исходным координатам возле глаз, но пересечения вычисляются по окончательным деформированным позициям. Волосы головы отключены; поза, привязки ресниц и кожа соответствуют текущему FemaleDemo при времени 0.

Release-диагностика прошла: /tmp/voxy-lash-skin-crossings.log. Экспорт /tmp/voxy-lash-skin-crossings.json содержит фактические центральные линии и номера пересекающих пролётов. Просмотрена научная визуализация /tmp/voxy-lash-skin-crossings.png: красным выделены пересекающие кожу хорды в проекции XY. Для 0/0.5 закрытия одна нижняя ресница (индекс 50) пересекается на пролёте 11/10 соответственно; при полном закрытии она пересекается на пролёте 14, и верхняя ресница 77 — на пролётах 0 и 3. Это одна ресница с пересечением возле корня и две с пересечением дальше корня при полном закрытии; корень и дальний участок одной ресницы учитываются в разных категориях.

Диагностика проверяет центральную линию, не радиус трубки, не столкновения ресниц друг с другом, не касания концами и не копланарные совпадения. Поэтому остальные загибы нельзя объявлять свободными от всех видов контакта. Наблюдение сужает следующую правку: исследовать конкретные внутренние привязки 50/77 и длину/ориентацию их грума, вместо общего изменения завивки всех 144 ресниц. Производственный алгоритм на этом этапе не менялся. Полный набор приложения не повторялся; git diff --check прошёл.

## Отклонённые простые коррекции внутренних ресниц — 2026-10-02

Проверен плавный коэффициент длины 0.3→1 на первых 18% ширины верхнего/нижнего ряда возле внутреннего уголка. Привязки, остальные ресницы и их количество сохранялись. Все 18 female_features release-тестов прошли, /tmp/voxy-canthal-lash-tests.log; Metal-рендер просмотрен, /tmp/voxy-canthal-lash-render.png и /tmp/voxy-canthal-lash-render.log. Диагностика фактических центральных линий показала ухудшение: 4/5/4 ресницы с пересечениями дальше первого пролёта для закрытия 0/0.5/1, /tmp/voxy-canthal-lash-crossings.log и /tmp/voxy-canthal-lash-crossings.json. Укорочение откатано.

Затем проверено направление отступа основания 80 мкм по нормали текущего треугольника привязки, ориентированной в сторону прежней усреднённой нормали. Остальная кривая сохранялась. 18 тестов прошли, /tmp/voxy-lash-geometric-root-tests.log; Metal-рендер просмотрен, /tmp/voxy-lash-geometric-root-render.png и /tmp/voxy-lash-geometric-root-render.log. Пересечения остались прежними: дальше первого пролёта 1/1/2 ресницы, возле корня 0/0/1; /tmp/voxy-lash-geometric-root-crossings.log и /tmp/voxy-lash-geometric-root-crossings.json. Эта коррекция также откатана, поскольку не решила измеренный дефект. Оба изображения относятся к отклонённым кандидатам.

После отката повторная release-диагностика прошла и подтвердила прежние 1/1/2 дальних пересечения и 0/0/1 корневое, /tmp/voxy-lash-restored-crossings.log. Следующая реализация должна учитывать столкновение кривой с кожей на её длине, сохранять корневую привязку и плавность формы. Простое изменение длины или нормали отступа недостаточно; эти опыты не подтверждают готовность контакта ресниц. Полный набор приложения не повторялся; git diff --check прошёл.

## Локальная коррекция контакта ресниц с кожей — 2026-10-02

## Сравнение геометрических возрастных признаков — 2026-10-02

## Изоляция дефектов открытого рта — 2026-10-02

Снят и просмотрен текущий Metal-рендер 0/0.45/0.9 открытия (/tmp/voxy-current-oral.log, /tmp/voxy-current-oral.png): угловатые светлые выступы в полости и широкие тёмные полосы за губами сохраняются. Добавлен VOXY_FACE_DIAGNOSTIC_NO_TEETH=1, отключающий только геометрию коронок; gingiva и crown envelopes для ограничения языка сохраняются. По умолчанию геометрия не меняется.

Сравнительный release-рендер выполнен и просмотрен (/tmp/voxy-oral-no-teeth.log, /tmp/voxy-oral-no-teeth.png). Светлые выступы остаются без зубных коронок: их происхождение от зубов исключено этим сравнением. Следующий анализ должен изолировать нёбо, внутренние губы и прочие поверхности; одно отключение зубов ещё не определяет конкретную mesh дефекта. Это диагностический результат, не исправление полости рта. Полные тесты не повторялись; git diff --check прошёл.

Аудит ресниц теперь принимает VOXY_FACE_DIAGNOSTIC_PRESET с абсолютным путём к JSON, применяя его до построения фактической mesh. С aged-defined и VOXY_FACE_DIAGNOSTIC_LASH_TUBE=1 выполнена release-проверка открытого/полузакрытого/закрытого века (/tmp/voxy-aged-blink-tube.log и JSON): 0/0/0 трубок с поперечным пересечением их рёбер с кожей, осевых пересечений также нет. Первый запуск с относительным путём не нашёл файл, потому что test runner работает из каталога crate; повторён с абсолютным путём и прошёл.

Metal-рендер --face-preset aged-defined.json --eye-closeup --lid-close-transition выполнен и просмотрен (/tmp/voxy-aged-blink-render.log, /tmp/voxy-aged-blink.png). Крупный план показывает морщины уголка и под глазом в трёх положениях. Их рисунок остаётся стилизованным и местами регулярным; совместимость одного пресета в трёх кадрах не доказывает непрерывную плавность, все варианты конструктора, отсутствие касаний или полную герметичность века. Обычная деформация на данном этапе не менялась. git diff --check прошёл.

Профили wrinkles_under_eyes/upper_lip теперь имеют плавный изгиб и меняющуюся по длине амплитуду. Под глазами: параболический подъём с коэффициентом 6 вместо 0.7, вариация траектории 250 мкм, ширина 420..700 мкм, сила 0.55..1. Над губой: наклон 0.10, вариация 200 мкм, ширина 650 мкм, сила 0.45..1. Прежние настройки и нулевой эффект при amount=0 сохраняются; профиль используется согласованно для геометрии и основного микрорельефа. Это художественная коррекция рисунка, не анатомические измерения.

Четыре face_parameters release-теста прошли (/tmp/voxy-age-irregular-tests.log). Три аппаратных лучевых прохода выполнены (/tmp/voxy-age-irregular-render.log), /tmp/voxy-age-irregular.png просмотрен. Над верхней губой рисунок менее похож на прямую решётку; под глазами остаётся регулярность мелких линий. Правка сохранена, но полный реализм старения и совместимость крайних настроек с морганием ещё не доказаны. Полный набор приложения не повторялся; git diff --check прошёл.

Добавлен отдельный aged-defined.json, сохраняя mature-soft: выраженные морщины и носогубные складки, уменьшение объёма щёк −2.4 мм, локальное опущение 3.5/4 мм, мешки 1.5 мм, общий skin_laxity=2.5, височные впадины 1.5. Художественный материал roughness_scale=1.3/oil_scale=0.65; это не универсальная биологическая зависимость возраста и жирности кожи. Новый --age-strong-transition интерполирует youthful-soft→aged-defined, включая дополнительные параметры материала: отсутствующие в youthful значения берутся из базового пресета. Атлас регенерируется на каждом кадре.

Release-сборка, валидация каждого патча и три аппаратных лучевых прохода выполнены (/tmp/voxy-age-strong-render.log). /tmp/voxy-age-strong-transition.png просмотрен: складки под глазами, лоб и верхняя губа теперь явно различаются. Некоторые линии ещё слишком регулярные, общий силуэт лица остаётся молодым; пресет не доказывает реалистичное полное старение. Следующие улучшения должны корректировать форму и нерегулярность, а не просто повышать амплитуду. Полные тесты не повторялись; git diff --check прошёл.

Добавлен отдельный wrinkle_microprofile для материала, сохраняя прежний профиль геометрических складок: crow_feet получает четыре тонких ветви с неровной траекторией ±120 мкм, under_eyes — три линии; Gaussian width=220 мкм, прерывистая сила 0.35..1, вклад 0.65 от основной амплитуды. Исходный профиль pow4 сохраняется через max; существующие параметры управляют обеими масштабами. Пигментация и материал при нулевых возрастных параметрах не меняются этой функцией. Художественный рисунок не является анатомически калиброванной моделью.

Тест parameterized_creases_preserve_pigmentation_and_change_microheight прошёл (/tmp/voxy-age-microfold-tests.log). Release-рендеры выполнены и просмотрены: общий /tmp/voxy-age-microfold.png (/tmp/voxy-age-microfold-render.log) и крупный глаз /tmp/voxy-age-microfold-closeup.png (/tmp/voxy-age-microfold-closeup.log). На общем плане эффект слабый; требуется дальнейшая проверка выразительности микрорельефа и согласованности материала. Полный набор не повторялся; git diff --check прошёл.

Исправлен --age-transition: после применения каждого интерполированного пресета заново генерируется material_texture и загружается GPU-атлас. До этого текстура создавалась до переключения параметров, поэтому три кадра использовали микрорельеф базового пресета даже при разных wrinkles. Генератор материала уже поддерживает material_creases; новые формулы старения на этом этапе не добавлялись. Release-сборка и аппаратные лучевые проходы прошли (/tmp/voxy-age-material-render.log). /tmp/voxy-age-material.png просмотрен: результат всё ещё выглядит мягким, а не убедительно пожилым лицом. Обновление атласа делает визуальную проверку пресетов достовернее, но не завершает возрастную модель. Полные тесты не повторялись; git diff --check прошёл.

Сохранённые youthful-soft/mature-soft теперь явно задают новые локальные опущения: 0/0 мм для youthful и 1.2/1.5 мм для mature. --age-transition использует именно эти JSON-пресеты и численно интерполирует их в степени 0/0.5/1, сохраняя прочие базовые параметры. Он больше не подменяет реальный пресет отдельным набором максимальных значений. Это сравнение двух художественных заготовок, без привязки к возрасту в годах. Release-сборка, валидация патчей параметров и три аппаратных лучевых прохода прошли (/tmp/voxy-age-presets-render.log). Результат /tmp/voxy-age-presets.png просмотрен: различия намеренно мягкие, выраженное старение не доказано. Следующий этап должен исследовать поверхностный материал и мелкие морщины, а не объявлять эти пресеты завершённой возрастной моделью. git diff --check прошёл.

Добавлены отдельные параметры age_cheek_descent и age_jowl_descent в производственный каталог и face-controls.json: «Опущение щёк»/«Опущение у линии челюсти», группа «Возрастные особенности», 0..4 мм, default=0, step=0.01. Они дополняют существующий общий skin_laxity локальными C1-полями вниз по Y; центры (±0.039,0.682,0.13)/(±0.043,0.644,0.115), радиусы (0.035,0.032,0.045)/(0.028,0.027,0.04). Это художественная деформация, не моделирование упругости или возраст в годах. --age-transition теперь включает оба параметра в степени 0/0.5/1.

Release-рендер выполнен и просмотрен (/tmp/voxy-age-descent-render.log, /tmp/voxy-age-descent.png): влияние умеренное, убедительное полное старение всё ещё не достигнуто. Первый тест каталога выявил отсутствие двух настроек в UI-схеме (240 vs 242); схема дополнена без ослабления проверки. Повторный прогон face_parameters записан в /tmp/voxy-age-descent-tests.log. Сборка также потребовала добавить отсутствующий liquid::Error::WorkBudgetExceeded, на который ссылался текущий droplet_finite_gas; расчёты капель не менялись. Полные наборы приложения/физики не повторялись; git diff --check прошёл.

Добавлен ray-probe --age-transition (после --experimental --face): три степени 0/0.5/1 в одной позе времени 0, глаза открыты, камера Z=0.37 м, одинаковые материалы и освещение. Настройки геометрии: wrinkles_forehead/glabella до 0.8, crow_feet/under_eyes/upper_lip до 0.6, folds_nasolabial до 1.5, folds_marionette до 1.2, age_cheek_volume до −2, under_eye_bags до 1.5. Это художественные значения, не соответствие возрасту в годах. Прочие базовые настройки сохраняются; atlas материала фиксирован, поэтому сравнение изолирует геометрические изменения.

Release-сборка и три аппаратных лучевых прохода выполнены (/tmp/voxy-age-transition.log); результат /tmp/voxy-age-transition.png просмотрен. На крайнем варианте видны горизонтальные складки лба, межбровные борозды, изменения под глазами и щёк. Различие выглядит умеренным; полного убедительного старения не достигнуто. Мелкие возрастные морщины, опущение тканей и согласованные изменения материала требуют отдельной доработки. Существующие параметры не следует объявлять реалистичной возрастной моделью по одному этому рендеру. На данном этапе добавлен воспроизводимый визуальный аудит, производственные функции возрастной деформации не менялись. git diff --check прошёл.

Сохранён контакт целых пролётов: новый TriangleMesh::capsule_contact использует существующий segment_triangle и BVH с AABB, расширенным на радиус. Выбирает ближайшую конечную грань в пределах радиуса, возвращает параметр сегмента и смещение. Направление — от ближайшей поверхности к оси; при нулевом расстоянии используется нормаль грани, ориентированная в переднюю полусферу. Запрос не определяет внутри/снаружи и не выполняет временной CCD. После прежней коррекции выполняются шесть проходов 16 капсул каждой ресницы; радиус берётся консервативно по началу пролёта плюс 10 мкм. Смещение распределяется по соседним кольцам прежним гауссовым весом; корень неподвижен, длина не сохраняется.

Аудит реальных рёбер трубок дал 0/0/0 пересечений в положениях 0/0.5/1, исправив ресницу 77 (/tmp/voxy-lash-capsule-tube.log и JSON). Все 20 профильных release-тестов прошли (/tmp/voxy-lash-capsule-tests.log). Отдельный тест capsule_query_detects_interior_contact_and_rejects_distant_plane_extension прошёл (/tmp/voxy-lash-capsule-query-test.log): проверяет близкий сегмент, пересечение его середины и отсутствие контакта с продолжением плоскости за конечной гранью. Sweep 51 положения последнего участка выполнен (/tmp/voxy-lash-capsule-close-sweep.log и JSON). Metal-рендер выполнен и просмотрен (/tmp/voxy-lash-capsule-render.png и log). Не доказаны обратные пересечения рёбер кожи с гранями трубки, касания, межресничные столкновения и все варианты конструктора. Стоимость дополнительных запросов пока не измерена; полные наборы physics/app не повторялись.

Аудит экспортирует конкретное пересекающее ребро, текущую грань, центры и кольца в LASH TUBE DETAIL; сняты /tmp/voxy-lash-tube-detail.log и /tmp/voxy-lash-tube-detail.json. Нормаль грани (−0.39090012,0.84740744,0.35930172). Подписанные расстояния концов ребра +12.669/−66.427 мкм, радиусы колец 2/3 примерно 41.63/39.74 мкм; это пересечение оболочки между кольцами. Знак расстояния до бесконечной плоскости не является классификацией относительно конечного треугольника.

Совместный кандидат (середины пролётов плюс геометрическая нормаль возле контакта) также проверен: /tmp/voxy-lash-combined-contact-tube.log, результат трубок прежний 0/0/1. Metal-рендер выполнен и просмотрен (/tmp/voxy-lash-combined-contact-render.png и log); профильные тесты записаны в /tmp/voxy-lash-combined-contact-tests.log. Кандидат откатан; восстановлена сохранённая коррекция по кольцам со сглаженными нормалями. Следующий шаг — запрос минимального расстояния от целого сегмента с радиусом до конечного треугольника: в physics::hair уже есть segment_triangle для контактов HairRod. Использование только нескольких осевых отсчётов оказалось недостаточным для обнаруженного пересечения.

Аудит трубок теперь записывает source IDs пересекаемой грани и результат исходного collider-фильтра. Для ресницы 77 при закрытии 1: [43553,43569,45223], исходный центр (0.021907965,0.708527,0.13109408), исходная нормаль Z положительна, collider_filter=true (/tmp/voxy-lash-tube-source.log). Пропуск этой грани фильтром исключён как причина.

Проверено плавное смешивание интерполированной нормали с геометрической нормалью ближайшей грани в пределах 3→1 локального clearance. Аудит трубок остался 0/0/1 (/tmp/voxy-lash-near-normal-tube.log), все 20 профильных тестов прошли (/tmp/voxy-lash-near-normal-tests.log), Metal-рендер выполнен и просмотрен (/tmp/voxy-lash-near-normal-render.png и log). Кандидат откатан: не устранил измеренное пересечение, обычная коррекция восстановлена. Следующий анализ должен сопоставить конкретное пересекающее ребро с ближайшим расстоянием и зазором у соответствующих колец; смена нормали без этого недостаточна. git diff --check прошёл.

Проверено удвоение контактных отсчётов за счёт середин пролётов: 32 вместо 16 при той же трубке 17×6 вершин. Все 20 female_features release-тестов прошли (/tmp/voxy-lash-midspan-tests.log); аудит трубок по-прежнему 0/0/1 пересечений (/tmp/voxy-lash-midspan-tube.log и JSON), дефект не устранён. Metal-рендер выполнен и просмотрен (/tmp/voxy-lash-midspan-render.png и log), убедительного улучшения не видно. Удвоение запросов откатано, поскольку оно не решило пересечение поверхности. Следующая правка должна проверять действительное ближайшее расстояние от оболочки и направление отступа, а не только добавлять осевые отсчёты с прежней интерполированной нормалью. Производственный алгоритм восстановлен; git diff --check прошёл.

Добавлен аудит VOXY_FACE_DIAGNOSTIC_LASH_TUBE=1: три положения 0/0.5/1, проверки реальных кольцевых/продольных рёбер и диагоналей mesh трубок против текущих треугольников кожи. Release-прогон выполнен (/tmp/voxy-lash-tube-crossings.log), JSON сохранён отдельно, график /tmp/voxy-lash-tube-crossings.png построен и просмотрен. Результат 0/0/1 пересекающихся трубок: при полном закрытии верхняя ресница 77 пересекает кожу в области пролёта 2, несмотря на отсутствие осевых пересечений. Проверка не покрывает обратное пересечение рёбер кожи с треугольниками трубки, касания концами, копланарные совпадения или межресничные столкновения. Следующая коррекция должна учитывать реальный радиус около этого контакта и повторно проверить временную плавность. Производственный алгоритм не менялся; git diff --check прошёл.

Сохранён переход контакта на barycentric-интерполяцию переданных нормалей текущих вершин кожи: отдельный TriangleMesh::closest_surface_interpolated возвращает ту же ближайшую позицию, но смешивает нормали вершин выбранной грани. Существующий closest_surface и физические сертификаты не изменены. Это приближённая нормаль поверхности, не signed-distance certificate; для теста плоскости используется прежняя геометрическая нормаль. При нулевом смешанном векторе запрос возвращает нормаль грани; невалидные/короткие массивы нормалей отклоняются.

Release-sweep последних 5% (51 отсчёт) прошёл: /tmp/voxy-lash-interpolated-contact-sweep.log и JSON. Максимальное перемещение за шаг снизилось с 1.160021 до 0.024545 мм, максимальный угол на этом участке с 36.77367° до 13.64824°, ноль осевых пересечений. Полный sweep 21 положения также выполнен (/tmp/voxy-lash-interpolated-contact-full-sweep.log и JSON). Все 20 female_features release-тестов прошли (/tmp/voxy-lash-interpolated-contact-tests.log), включая угол, закреплённое основание и зазор трубки над плоскостью. Metal-рендер выполнен и просмотрен (/tmp/voxy-lash-interpolated-contact-render.png и log). Это устраняет измеренный скачок в проверенном диапазоне, но не доказывает непрерывный контакт толщины трубки, межресничные столкновения, все варианты конструктора или полную герметичность век. Полный набор physics/app не повторялся.

Добавлена трассировка VOXY_FACE_DIAGNOSTIC_LASH_CONTACT_TRACE=1 для проблемного корня X≈−0.021336 м; диагностика в этом режиме снимает закрытие 0.996/0.997. Release-прогон прошёл (/tmp/voxy-lash-contact-trace.log). До коррекции отсчёт 7 при 0.996 имеет ближайшую поверхность (−0.022056272,0.71192676,0.13039541), нормаль (0.4200624,−0.78495365,0.45540684), distance=1.606740 мм и gap=+1.599662 мм. При 0.997 поверхность переключается на (−0.022168271,0.70957035,0.13044474), нормаль (−0.338536,−0.936408,0.0923767), distance=1.603651 мм, gap=−1.266083 мм. Сам исходный отсчёт меняется лишь на несколько микрометров. Значит, ближайшая грань меняет классификацию стороны и запускает большую коррекцию; это объясняет, почему сглаживание только distance-порога не помогает. Инверсия самой грани этим логом не доказана: её исходный ID и ориентация ещё не сопоставлены. Построен и просмотрен график signed-gap по отсчётам /tmp/voxy-lash-contact-normal-switch.png. Далее нужен корректный выбор контактной стороны/непрерывной нормали возле складки, а не ослабление теста. Обычная геометрическая коррекция на этом этапе не менялась; трассировка по умолчанию выключена.

Сравнительный sweep с VOXY_FACE_DIAGNOSTIC_NO_LASH_CONTACT=1 выполнен: /tmp/voxy-lash-final-closure-no-contact.log и JSON. Без контакта максимальное перемещение за шаг 0.1 процентного пункта 0.024545 мм против 1.160021 мм с контактом. Это подтверждает связь скачка с коррекцией, но не определяет конкретную ветвь.

Проверено плавное включение по smoothstep на расстоянии 3→1 мм вместо жёстких 2 мм. Оно не устранило скачок: максимум перемещения 1.195787 мм, максимум угла в последнем участке 52.76998°, осевых пересечений нет (/tmp/voxy-lash-smooth-activation-sweep.log и JSON). Прогон female_features: 19 passed, 1 failed — регрессионный тест угла выявил ресницу 5 при полном закрытии, пролёт 5 (/tmp/voxy-lash-smooth-activation-tests.log). Metal-рендер выполнен и просмотрен (/tmp/voxy-lash-smooth-activation-render.png и log). Кандидат откатан; тест не ослаблен. Гипотеза, что одного сглаживания порога расстояния достаточно, опровергнута. Далее нужно измерить переключение ближайшего треугольника/нормали и знак gap вокруг 99.6–99.7% закрытия. Производственный алгоритм восстановлен; git diff --check прошёл.

Добавлен режим диагностики VOXY_FACE_DIAGNOSTIC_LASH_CLOSE_SWEEP=1: 51 положение закрытия от 95% до 100% с шагом 0.1 процентного пункта. Release-аудит выполнен (/tmp/voxy-lash-final-closure-sweep.log), данные сохранены в /tmp/voxy-lash-final-closure-sweep.json. По реальным центрам колец построен и просмотрен /tmp/voxy-lash-final-closure-sweep.png. Пересечений осевых линий во всех отсчётах нет; максимальный угол 36.7737° при закрытии 99.7%. Однако обнаружен временной скачок: между 99.6% и 99.7% точка 5 ресницы 5 перемещается на 1.16002 мм. Значит, прежние проверки трёх/21 кадров и ограничения статического угла не подтверждают плавную анимацию. Необходим анализ включения контакта, в частности жёсткого условия distance<2 мм; причинность ещё не измерена. Производственный алгоритм на этом этапе не менялся. Непрерывный контакт и плавность не завершены.

Диагностика audit_lash_skin_crossings расширена с трёх кадров до 21 положения закрытия с шагом 5%. Release-прогон прошёл (/tmp/voxy-lash-blink-sweep.log); экспорт сохранён отдельно в /tmp/voxy-lash-blink-sweep.json. Во всех 21 положениях ноль корневых/дальних осевых пересечений 144 ресниц. По фактическим центрам колец построен и просмотрен график /tmp/voxy-lash-blink-sweep.png, численные значения /tmp/voxy-lash-blink-sweep-metrics.json. Максимум угла остаётся ниже 45° во всех отсчётах; общий максимум 36.65° при полном закрытии. Между 95% и 100% максимум резко растёт с 13.67° до 36.65°, поэтому непрерывная временная плавность этим дискретным аудитом не доказана: следующая проверка должна уплотнить последний участок и измерить перемещения соответствующих точек, а также рассмотреть непрерывный контакт толщины трубок. Обычный алгоритм на данном этапе не менялся; git diff --check прошёл.

Новый тест actual_closed_lash_contact_does_not_form_sharp_segment_reversals отдельно прошёл в release: /tmp/voxy-lash-contact-curvature-test.log. Проверка углов дополняет 19 ранее прошедших профильных тестов; git diff --check прошёл. Полный набор приложения на этом этапе не повторялся.

Сохранена коррекция с распределением каждого контактного смещения по соседним кольцам: вес exp(-d²/8), дополнительно min(neighbor/contact_row,1) для затухания к неподвижному основанию. Центральный отсчёт получает полную поправку; последующие проверки снова учитывают уже изменённую кривую. Три прохода и лимит отдельной поправки 0.5 мм сохранены. Это приближённое распределение изгиба, не упругая симуляция; суммарное смещение соседней точки может превышать лимит одной поправки, длина не сохраняется.

На фактической mesh ноль корневых и дальних осевых пересечений для 0/0.5/1 (/tmp/voxy-lash-contact-neighborhood-crossings.log и JSON). Максимальные углы между хордами 20.39°/22.33°/36.65° вместо 20.44°/16.10°/131.89°: худший излом полного закрытия существенно уменьшен, половинное закрытие слегка ухудшилось, все три положения ниже 45°. Медианы максимумов около 4.65° не изменились. 19 профильных release-тестов прошли (/tmp/voxy-lash-contact-neighborhood-tests.log), включая зазор трубки над плоскостью и неподвижное основание. Metal-рендер выполнен и просмотрен (/tmp/voxy-lash-contact-neighborhood-render.png и log); мелкие загибы возле закрытого края остаются. Добавлен отдельный регрессионный тест actual_closed_lash_contact_does_not_form_sharp_segment_reversals по текущему FacePreview, 144 реальным трубкам, трём положениям и пределу угла 45°: прежний излом 131.89° эту проверку не проходил. Полная герметичность века, непрерывный контакт всей поверхности трубки и межресничные столкновения пока не доказаны.

Проверено смещение контакта вдоль head-forward вместо скачущих нормалей треугольников, с пересчётом величины по скалярному произведению нормали и направления (нижняя граница 0.1). Кандидат откатан: хотя аудит вновь дал ноль корневых/дальних осевых пересечений для 0/0.5/1 (/tmp/voxy-lash-forward-contact-crossings.log и JSON), максимальные углы между хордами выросли до 25.59°/38.55°/157.59°, число ресниц с углом >30° стало 0/1/2. Ресница 5 осталась худшей при полном закрытии. До коррекции её два соседних пролёта имели длины 0.647/0.462 мм при окружающих около 0.25 мм: проблема находится в середине кривой, не в закреплённом основании. Metal-рендер кандидата выполнен и просмотрен (/tmp/voxy-lash-forward-contact-render.png, /tmp/voxy-lash-forward-contact-render.log). Отсутствие пересечений не заменяет проверку плавности; обычный алгоритм снова использует нормаль ближайшего треугольника и три прохода.

По экспортированным фактическим центрам колец измерены углы между соседними хордами (/tmp/voxy-lash-orientation-comparison.json). В обычном режиме максимумы для закрытия 0/0.5/1: 20.44°/16.10°/131.89°; медианы максимумов по 144 ресницам около 4.65°. При полном закрытии один выброс более 30° — ресница 5. В режиме чистого шарнира выброс той же ресницы становится 145.42°, и возвращается одно дальнее осевое пересечение. Это конкретный дефект формы, который проверка только отсутствия пересечений не выявляла. Режим шарнира не заменяет производственный алгоритм; следующая правка должна исследовать локальные смещения ресницы 5 при контакте, сохраняя закреплённый корень.

Добавлен диагностический режим VOXY_FACE_DIAGNOSTIC_LASH_HINGE_ONLY=1: сохраняет реальные корни и коррекцию контакта, но заменяет перенос ориентации по треугольнику чистым поворотом шарнира. Обычный режим не меняется. Metal-рендер выполнен и просмотрен (/tmp/voxy-lash-hinge-only-render.png, /tmp/voxy-lash-hinge-only-render.log): при полном закрытии форма рядов заметно отличается, однако при половинном закрытии верхние ресницы становятся почти горизонтальными и сильнее выглядят крючками. Это опровергает простую замену переноса одним шарниром; такой режим оставлен только для диагностики. Выполнен аудит пересечений (/tmp/voxy-lash-hinge-only-crossings.log, /tmp/voxy-lash-hinge-only-crossings.json). Следующая проверка должна измерять переход ориентации и кривизну вдоль реальных ресниц, а не оценивать только закрытый кадр.

Рендер сглаженного кандидата выполнен и просмотрен: /tmp/voxy-lash-smooth-contact-render.png, /tmp/voxy-lash-smooth-contact-render.log. Убедительного уменьшения изломов закрытого века не видно. Восемь проходов сглаживания откатаны: сохраняется прежняя трёхпроходная коррекция контакта. Изображение относится к отклонённому кандидату; сохранено только восстановление двух диагностических флагов парсера. Следующая проверка должна разбирать исходный грум и транспорт ориентации, поскольку локальное сглаживание контактного смещения не решило видимый дефект.

Следующий кандидат распределяет смещение контакта по соседним отсчётам: восемь проходов, смешивание собственного смещения с усреднённым соседним на 50%, затем повторная проекция на зазор. Исходная кривизна не сглаживается напрямую, основание сохраняется. Все 19 female_features release-тестов прошли (/tmp/voxy-lash-smooth-contact-tests.log); диагностика снова дала ноль осевых пересечений в трёх положениях (/tmp/voxy-lash-smooth-contact-crossings.log, /tmp/voxy-lash-smooth-contact-crossings.json). Это не доказательство непрерывного контакта трубки или сохранения длины. Новый строгий парсер ray probe требовал явного восстановления приёма --eye-closeup и --lid-close-transition после --face; остальные неизвестные аргументы по-прежнему отклоняются.

FaceFeatures сохраняет топологию передней области кожи возле глаз и строит TriangleMesh/BVH по текущим деформированным позициям. После построения 17 точек каждой ресницы выполняются три прохода ближайшей поверхности. Основание неподвижно; остальные точки получают ограниченный отступ по нормали с учётом локального радиуса трубки и зазора 10 мкм. Проверка действует в пределах 2 мм от поверхности, одно смещение ограничено 0.5 мм. Нормаль ориентируется в переднюю полусферу головы. После коррекции касательные вычисляются по соседним точкам; количество вершин и треугольников сохраняется. Переменная VOXY_FACE_DIAGNOSTIC_NO_LASH_CONTACT=1 отключает коррекцию для сравнения.

На фактической mesh диагностика дала ноль корневых и дальних пересечений осевых линий для закрытия 0/0.5/1: /tmp/voxy-lash-contact-crossings.log и /tmp/voxy-lash-contact-crossings.json. До коррекции при полном закрытии пересекались две уникальные ресницы, одна одновременно в корневой и дальней категориях. 18 ранее существовавших female_features release-тестов прошли, /tmp/voxy-lash-contact-tests.log. Отдельный новый тест lash_contact_preserves_root_and_clears_a_plane_with_its_radius прошёл, /tmp/voxy-lash-contact-plane-test.log: проверяет неподвижное основание, конечность геометрии и зазор поверхности трубки над плоскостью, которую исходная кривая пересекала.

Metal-рендер выполнен и просмотрен: /tmp/voxy-lash-contact-render.png, /tmp/voxy-lash-contact-render.log. Изломы возле закрытого века остаются видимыми. Это локальная проекция отсчётов, не упругая симуляция волос: длина не сохраняется, непрерывный контакт трубки между отсчётами и столкновения ресниц между собой не доказаны. Передняя область является открытой поверхностью; выбор направления нормали приближённый. При ошибке построения TriangleMesh применяется прежняя кривая без коррекции. Стоимость перестроения BVH и коррекции на каждом кадре пока не измерена; полный набор приложения не повторялся. Складки, возрастные признаки и остальные пункты общей цели этим этапом не завершены.

## Поэлементная диагностика полости рта — 2026-10-02

Сравнения на Metal выполнены и изображения просмотрены: /tmp/voxy-oral-no-lip-replacement.png, /tmp/voxy-oral-no-palate.png, /tmp/voxy-oral-no-buccal.png и /tmp/voxy-oral-no-gingiva.png; соответствующие логи имеют тот же stem. Во всех сравнениях отключены процедурные зубы. Серые дугообразные выступы сохраняются при отдельном отключении восстановления губ, нёба, слизистой щёк и дёсен. Без слизистой щёк серые боковые поверхности становятся значительно заметнее. Это сужает поиск, но ещё не доказывает принадлежность конкретных граней исходной голове: необходима идентификация видимой геометрии.

Добавлен VOXY_FACE_DIAGNOSTIC_NO_GINGIVA=1, который пропускает только создание двух десневых дуг. Обычный режим при отсутствии флага сохраняется. Отсутствие восстановления губ оставляет отверстия в удалённых исходных треугольниках и используется только как диагностическое сравнение. Эти режимы не являются исправлением дефекта. Release-рендеры завершились успешно; git diff --check прошёл. Общая цель остаётся незавершённой.

## Совмещение слизистой щёк и входа в глотку — 2026-10-02

Диагностическая окраска VOXY_FACE_DIAGNOSTIC_ORAL_MATERIAL_IDS=1 помечает UV материалов кожи зелёным, отрицательные oral UV пурпурным после ray shading. Просмотрено /tmp/voxy-oral-material-ids.png. Серые дуги получили цвет кожи. CPU ray query по пикселю (250,363) третьего кадра встретил грань [16948,16951,16952] в (-0.008523271,0.64455247,0.062014624): видна задняя поверхность головы через щель. Лог /tmp/voxy-oral-material-ids.log.

Исправлен стык: первая окружность глотки совпадает по X/Y/Z с последней окружностью слизистой щёк, включая jaw-dependent радиус и кривизну Z. Глотка теперь использует те же 24 угловых сегмента вместо 16; положение глубокого конца сохранено. Первый кандидат с различным числом сегментов оставлял тонкие щели (/tmp/voxy-oral-joined-throat.png), окончательный /tmp/voxy-oral-matched-throat.png выполнен на Metal и просмотрен: широкие серые дуги и тонкие щели этого стыка исчезли в показанных трёх положениях.

Все 21 female_features release-тест прошли (/tmp/voxy-oral-joined-throat-tests.log). Новый тест buccal_lining_and_pharynx_share_the_same_boundary проверяет совпадение 24 вершин при jaw 0/0.45/0.9/1 с допуском 0.1 мкм. git diff --check прошёл. Боковые границы у губ, видимость дёсен, грубая форма/цвет зубов и полный временной диапазон остаются отдельными задачами; цель не завершена.

## Небольшой рельеф резцов и макродиагностика — 2026-10-02

Передняя поверхность резцов получила три плавные labial lobes с амплитудой 80 мкм и затуханием к краям. Это художественная модель, а не индивидуальная стоматологическая анатомия. Шероховатость эмали меняется от 0.32 у шейки до 0.24 у режущего края, тёплый градиент цвета сохранён. Тест резца теперь проверяет допустимый диапазон материала и нахождение всех вершин внутри CrownEnvelope с запасом 0.2 мм; прежняя проверка одного постоянного UV была заменена. Первый прогон упал на старом постоянном UV, повторный после изменения теста прошёл: 21 female_features test, /tmp/voxy-enamel-lobes-tests.log.

Metal-рендеры /tmp/voxy-enamel-lobes.png и /tmp/voxy-enamel-macro.png выполнены и просмотрены. VOXY_FACE_DIAGNOSTIC_DENTAL_MACRO=1 приближает камеру oral-closeup до Z=0.205, без изменения обычной камеры. В общем плане различие невелико. Макро показывает оставшиеся серые плоские коронки, угловатые края внутренней губы и плохо видимые дёсны; реалистичность зубов этим шагом не достигнута. Следующая проверка должна разделить влияние освещения/видимости и модели эмали и разобраться с внутренним краем губ. git diff --check прошёл; цель активна.

## Разделение цвета эмали и затенения — 2026-10-02

Добавлен VOXY_FACE_DIAGNOSTIC_UNSHADOWED_TEETH=1 в ray probe: только вершины oral UV=-1 с исходным R>0.6 пропускают множитель ray visibility и получают alpha=1 для незатенённого блика. Это диагностический классификатор по текущей палитре, не полноценный material ID и не производственное исправление. При обычной геометрии и цвете макрорендер /tmp/voxy-teeth-unshadowed.png выполнен на Metal и просмотрен; коронки светлые вместо серых. Таким образом существенное потемнение связано с текущим приближённым освещением/видимостью, а не только цветом эмали. Отключение теней не сохранено как обычное поведение.

Также проверены 64 вместо 16 подразделений каждого исходного lip seam triangle. /tmp/voxy-lip-refinement.png выполнен и просмотрен: угловатый внутренний край и полосы сохраняются. Кандидат откатан к двум уровням подразделения, поскольку визуальный дефект не устранён. Изображение относится к отклонённому кандидату. Следующий шаг должен определить материал и исходные грани внутренней полосы, а для зубов проверить вклад прямого/непрямого света; увеличение плотности само по себе не помогает. git diff --check прошёл до отката; цель активна.

## Локализация полосы нижней губы — 2026-10-02

Просмотрен Metal-рендер /tmp/voxy-lip-material-macro.png (лог /tmp/voxy-lip-material-macro.log): большая часть полосы на внутреннем крае нижней губы помечена skin UV, не oral UV. Макродиагностика теперь выбирает лучи по этому участку вместо прежних пикселей глотки. Фактические попадания третьего кадра (/tmp/voxy-lip-band-hits.log): (288,480) → [119594,119595,119596] в (0,0.6366523,0.14717568); (240,478) → [119588,119589,119590] в (-0.0027190796,0.63768744,0.15287386); (330,478) → [114385,114386,114387] в (0.0026332538,0.6367998,0.14730765). IDs выше размера исходного body: нужно проверять добавленные поверхности и классификацию inner_lip, а не удалять исходные body-треугольники вслепую. Добавлен вывод свойств вершин попадания для следующего запуска; этот дополнительный вывод пока не прогонялся. Производственная геометрия этим этапом не изменена; git diff --check прошёл.

## Проверка классификации внутренней губы — 2026-10-02

Предыдущий cargo check завершился успешно: /tmp/voxy-lip-hit-check.log. Диагностика VOXY_FACE_DIAGNOSTIC_LIP_BIND=1 записывает bind-треугольники трёх ранее найденных попаданий. /tmp/voxy-lip-bind.log: [119594…] и [114385…] имеют normal.z около -0.918/-0.894 и rest.z около 0.1500–0.15036; inner_lip=false из-за порога centroid.z<0.150. Участок [119588…] имеет rest.z около 0.1545–0.1548 и normal.z=0.849, то есть переднюю ориентацию. Диагностические IDs привязаны к текущей topology и не являются стабильными идентификаторами.

Проверен кандидат порога 0.153 при прежнем normal.z<0.35. Все 21 female_features release-тест прошли (/tmp/voxy-lip-mucosa-tests.log); Metal-рендер /tmp/voxy-lip-mucosa.png выполнен и просмотрен. Кандидат оставил полосу и добавил видимые тёмные треугольники на нижней губе, включая закрытую позу. Порог восстановлен к 0.150: кандидат не принят. Следующая правка должна учитывать непрерывную границу слизистой и переход материала, а не расширять классификацию целых треугольников одной глубиной. Общая цель активна.

## Заполняющий блик влажных тканей — 2026-10-02

Общий oral branch female_material.wgsl теперь добавляет GGX-блик заполняющего источника с весом 1/3 основного, соответствующим используемым diffuse weights 0.2/0.6. Его видимость не подменяется видимостью основного источника: ray probe передаёт измеренный fill_visibility линейно через ранее постоянный oral U: -0.75-0.25*visibility. V остаётся roughness, alpha остаётся key visibility. Диапазон U [-1,-0.75] не пересекает ветви tongue/hair/eye. Native preview с прежним U=-1 получает fill_visibility=1, как и его незатенённый заполняющий diffuse. Трассировка оценивает видимость по вершинам, не по каждому фрагменту; нового непрямого света или path tracing этот шаг не добавляет. Отдельная ветвь tongue не изменена.

Release Metal-рендер /tmp/voxy-oral-fill-highlight.png выполнен и просмотрен (/tmp/voxy-oral-fill-highlight.log), включая GPU shader validation scope. При данном ракурсе улучшение слабое; серый diffuse зубов, полосы внутренней губы и общий уровень реализма остаются нерешёнными. git diff --check прошёл. Общая цель активна.

## Проверка амплитуды загиба нижней губы — 2026-10-02

Проверено уменьшение posterior lower-lip roll с 4 до 1 мм при неизменных jaw weights и материалах. Metal-рендер /tmp/voxy-lip-roll-limited.png выполнен и просмотрен, /tmp/voxy-lip-roll-limited.log. Полоса внутреннего края сохранилась; боковые светлые поверхности стали заметнее при большом открытии. Кандидат откатан, производственная амплитуда снова 4 мм. Этот эксперимент не доказывает корректность исходного загиба, но исключает простое уменьшение амплитуды как достаточное исправление. Следующий шаг должен исследовать согласование внутренней поверхности губ с передним краем buccal lining: предыдущая правка согласовала только задний стык с глоткой. Общая цель остаётся активной.

## Передний контур слизистой следует раскрытию губ — 2026-10-02

Передний контур buccal lining теперь использует mouth_seam(front_x) и jaw_weight для нижней половины; верхняя половина и уголки не опускаются вместе с глобальным центром эллипса. Поперечная кривизна Z переднего края уменьшена с коэффициента 20 до 5, плавно возвращается к 20 на заднем крае. Задний контур по-прежнему совпадает с глоткой. Это процедурное приближение; ещё не привязка каждого переднего отсчёта к фактической деформированной mesh губ.

Первый вариант с прежней кривизной оставлял боковые просветы (/tmp/voxy-buccal-aperture.png). Окончательный /tmp/voxy-buccal-front-depth.png выполнен на Metal и просмотрен, боковые светлые поверхности меньше заметны, однако угловатая полоса нижней губы сохраняется. Все 22 female_features release-теста прошли /tmp/voxy-buccal-front-depth-tests.log, включая новый buccal_aperture_keeps_upper_edge_and_corners_fixed (верхние 13 отсчётов неподвижны, нижний центральный открывается) и прежний тест совпадения заднего стыка. git diff --check прошёл. Эта проверка не покрывает непрерывность всех материалов и отсутствие пересечений со всеми пресетами лица. Общая цель активна.

## Проверка численной сварки нормалей губ — 2026-10-02

Проверен текущий renderer smooth_normals: совпадающие вершины объединяются по точным f32 position bits. Кандидат округлял только добавленные clipped lip vertices до сетки 0.1 мкм, чтобы уменьшить влияние различий barycentric arithmetic на weld. Все 22 female_features release-теста прошли (/tmp/voxy-lip-normal-weld-tests.log). Metal-рендер /tmp/voxy-lip-normal-weld.png выполнен и просмотрен. Полоса и угловатость сохраняются; округление откатано.

Численное сравнение с /tmp/voxy-buccal-front-depth.png: 17142 отличающихся пикселя, среднее абсолютное отклонение канала 0.007624685, максимум 26 в 8-битном изображении. Это показывает небольшой эффект кандидата, но не доказывает, что все нормы уже корректны или что все пограничные вершины были объединены. Следующий шаг должен анализировать форму внутренней поверхности и её нормали на фактической mesh, а не продолжать численные косметические правки. Производственная геометрия восстановлена; цель активна.

## Срез фактической поверхности рта — 2026-10-02

Добавлен VOXY_FACE_DIAGNOSTIC_MOUTH_SECTION=1: в третьем кадре ray probe сохраняет фактические mesh triangles, пересекающие плоскость X=0 в диапазоне Y 0.625–0.665 и Z 0.075–0.17. /tmp/voxy-mouth-surface-section.json содержит 761 треугольник; лог /tmp/voxy-mouth-section.log. По ним построен и просмотрен научный график /tmp/voxy-mouth-section.png: 84 сегмента skin UV и 248 oral UV. Цвета графика показывают UV-классы, не отдельные анатомические органы. Срез может пересекать щели между центральными резцами и не является доказательством отсутствия пересечений объёмов.

На нижней губе виден skin-профиль с почти горизонтальным участком примерно Z=147–151 мм, Y=636–637 мм и передним подъёмом; этот участок соответствует ранее найденной видимой полосе. Анализ подтверждает необходимость изменения геометрического профиля, а не только материалов или численной сварки нормалей. Производственная геометрия не менялась. Release Metal-рендер/экспорт завершились успешно; git diff --check прошёл. Следующая правка должна учитывать переход внутреннего профиля губы к слизистой и сохранить внешнюю форму и закрытое положение. Общая цель активна.

## Проверка локального изменения профиля внутренней губы — 2026-10-02

Проверен кандидат опускания posterior lip transition до 1.5 мм с плавной пространственной маской и множителем lower*jaw. Все 10 активных female_face release-тестов прошли, 3 ignored не выполнялись (/tmp/voxy-inner-lip-profile-tests.log). Metal-рендер /tmp/voxy-inner-lip-profile.png выполнен и просмотрен: закрытое положение сохранено, но при открытии полоса стала складкой, край верхней губы также изменился. Кандидат полностью откатан.

Новый offset применялся только к исходным body vertices, тогда как append_lips интерполирует их и корректирует split seam лишь для прежнего jaw Y/Z offset. Это оставляет новую нелинейную деформацию интерполированной через границу верхней/нижней губы. Следующий шаг должен сначала согласовать вычисление mouth displacement в body и clipped replacements, затем повторять изменение профиля. Прохождение профильных тестов не доказывает визуальную корректность mouth replacements. Производственная деформация восстановлена; цель активна.

## Единая деформация jaw и posterior lip — 2026-10-02

В female_face выделена mouth_displacement(p,jaw): прежние jaw Y/Z и posterior roll вычисляются один раз. append_lips исправляет интерполированное полное векторное смещение, включая roll, и при clipping устанавливает upper boundary displacement=0, lower boundary через ту же функцию. Раньше корректировался лишь скалярный jaw_weight без posterior roll. Metal-рендер /tmp/voxy-shared-mouth-displacement.png выполнен и просмотрен; полоса остаётся, но точность переноса теперь проверяется отдельно. Новая проверка split_upper_lip_does_not_inherit_posterior_lower_lip_roll прошла, /tmp/voxy-upper-lip-roll-test.log. 23 female_features release-теста прошли, /tmp/voxy-shared-mouth-features-tests.log.

Широкий запуск female_ дал 122 passed, 1 failed, 21 ignored: complexion_follows_face_animation_and_leaves_eyes_untinted обнаружил дрейф roughness UV зубов при head rotation на ~2.7e-7. Цвет/roughness коронки теперь вычисляются из канонического latitude sample, а не round-trip world transform. Повтор этой проверки прошёл (/tmp/voxy-mouth-material-invariant-test.log); весь широкий набор повторно не выполнялся. Текущие изменения других частей рендерера временно препятствовали сборке: LOD SceneGeometry требовал device.clone() (добавлено), shadow depth fields и receiver transparent pass к следующему запуску уже были исправлены параллельной работой.

## Начало обзора форумов и исходников 100 репозиториев — 2026-10-02

По прямому запросу пользователя начат обзор. Реестр docs/research/face-source-study-100.json содержит 100 уникальных репозиториев с фактически полученными файлами, tree/blob SHA и SHA256 файлов. Источники без доступного кода заменены, NVIDIA Streamline находится в NVIDIA-RTX/Streamline. Скачивание исходников не равно прочтению: сейчас отмечено 7 частичных обзоров конкретных функций, 0 завершённых глубоких обзоров. Остальные 93 требуют чтения и сопоставления; исследование не завершено.

Прочитаны обсуждения https://polycount.com/discussion/186971/realistic-character-teeth-rendering-shading-suggestions, https://polycount.com/discussion/comment/2680696/, https://polycount.com/discussion/comment/2716292/, https://gamedev.net/forums/topic/585052-real-time-skin-shader/4721292/ и https://polycount.com/discussion/80005/face-topology-breakdown-guide/p1. Форумные советы являются практическими мнениями; обсуждение GameDev.net 2010 года не представляет современный performance baseline. Для технических выводов используются прочитанные первичные исходники FaceWorks, HumanShaders, separable-sss, EyeShader, tension-tools, smplx и Simplex. Частичные выводы и ограничения записаны в реестре. Общая цель и исследование остаются активными.

## Перенос управления морщинами через сжатие сетки — 2026-10-02

Из прочитанного apilola/tension-tools, Packages/com.ap.tension-tools/Runtime/Shaders/TensionHelper.hlsl, перенесён принцип сравнения длины ребра в покое и после деформации. В female_face подготовлены уникальные рёбра выбранной области лба; положение покоя берётся из текущего параметризованного лица до движения бровей. Положительное относительное сжатие усредняется по рёбрам вершины, растяжение даёт ноль. Сжатие активирует существующий художественный профиль складок; шкала насыщения 1.5% не является биомеханической моделью. Измерение выполняется до добавления самих складок. Прежний механизм доступен через VOXY_FACE_DIAGNOSTIC_NO_FOREHEAD_TENSION=1 для сравнения.

Metal-рендеры /tmp/voxy-tension-forehead.png и /tmp/voxy-forehead-pose-baseline.png выполнены и просмотрены для brow -1, 0, 1. Новый вариант делает складки локальнее и менее выраженными, чем прежние три длинные линии. Диагностика при brow=1 дала максимум сжатия 0.15347575 и среднее 0.023563972; это геометрические значения конкретной сетки, не измеренные свойства кожи. 11 female_face release-тестов прошли, 3 проигнорированы (/tmp/voxy-tension-forehead-tests.log), включая новую проверку вращения, растяжения, сжатия и нулевой длины ребра. git diff --check прошёл. Стоимость CPU, полный временной переход и все пресеты не проверены. Это первый внедрённый подход из текущего обзора; обзор 100 репозиториев и общая цель остаются незавершёнными.

## Раздельные нормали кожи по принципу FaceWorks — 2026-10-02

В female_material.wgsl текстурированная кожа сохраняет исходную pixel-filtered microheight normal для skin_area_highlight. Diffuse correction теперь вычисляется из отдельно отфильтрованного рельефа с минимальным размером фильтра 1 мм в текущей UV-to-metre mapping. Четыре filtered_microheight вызова используют explicit gradients и центральную разность с соответствующим физическим шагом. Это адаптация принципа detailed/blurred normals из samples/d3d11/shaders/skin.hlsli FaceWorks, не копирование SDK и не добавление объёмного SSS. В procedural fallback wrapped diffuse использует исходную гладкую нормаль; микрорельеф остаётся в specular.

До/после выполнены одинаковой командой cargo run -p voxy_ray_probe --release -- --experimental --face --skin-closeup --brow-transition: /tmp/voxy-split-normal-before.png и /tmp/voxy-split-normal-after.png просмотрены. Мелкое дробление освещения пор уменьшилось, кожа визуально мягче. Сборка, GPU shader validation и три Metal poses прошли (/tmp/voxy-split-normal-after.log); git diff --check прошёл. Дополнительные texture samples увеличивают работу fragment shader; стоимость пока не измерена. Procedural fallback отдельно визуально не проверен. Толщина тканей, полный SSS, corrective shapes губ и оставшийся обзор требуют следующих этапов.

## Перераспределение радужки при изменении зрачка — 2026-10-02

Добавлен pupil_diameter_mm в FaceParameters и каталог UI: 2..8 мм, default 4.05. Радиус передаётся в eye-only material_coordinates.z; skin bind coordinates не меняются. В shader iris_rest_point преобразует радиус кусочно-линейно между текущим зрачком и неизменным limbus 5.7 мм; центр защищён от деления на ноль. Цвет и высота радужки используют один mapping; высотная нормаль вычисляется центральными разностями уже после mapping. Это адаптация принципа dilation UV из FaceWorks eye.hlsli, не его power formula. Автоматический световой рефлекс не добавлен.

Ray probe --pupil-transition рендерит 2/4.05/8 мм в одной позе. Обнаружено, что ray visibility reconstruction терял explicit_material_coordinates; теперь этот массив сохраняется. GPU shader validation и три Metal renders прошли; /tmp/voxy-pupil-transition.png просмотрен. pupil_parameter_changes_only_eye_metadata_not_geometry_or_skin прошёл (/tmp/voxy-pupil-test.log): вершины неизменны, только eye metadata различаются, 9 мм отвергается.

## Фиксация этапа перед исследованием 500 репозиториев — 2026-10-02

Зрачок 2/4.05/8 мм рендерился и просмотрен, 4 eye-теста и 5 catalog/mask-тестов прошли. Проверка масок обнаружила движение upper-lip vertex при nose_width=1.3; добавлен C1 multiplier для nose_*: zero ниже Y=.653, one выше .663. local_feature_masks_preserve_remote_anatomy_on_the_actual_mesh после этого прошёл вместе с каталогом (/tmp/voxy-mask-catalog-test.log). Это анатомическая дополнительная маска, не редактор painted targetWeights.

Пробная quadratic jaw-open correction нижней губы -1.5 мм по Y/+0.8 мм по Z усилила складку при открытии (/tmp/voxy-lip-corrective-after.png против /tmp/voxy-lip-corrective-baseline.png, оба просмотрены). Кандидат полностью удалён; полезная authored corrective ещё не найдена.

female_transmission содержит экспериментальный geometric first-segment exit и six-Gaussian RGB profile по принципу SeparableSSS.h. Ограничен ушами/носом; lips и ambiguous/non-exit paths пропускаются. Нет solid certification, air-cavity model или отдельной external visibility для exit point. Не полноценный объёмный transport. Первый uncached Metal render выполнен/просмотрен при обычном и контровом свете, но visual gain слабый; trace cost 50.295/95.834/46.976 ms для трёх poses. Query slab 2 мм и monotonic spectral profile tests прошли. Последующая Cache/refit версия прошла cargo check, но её release-render и стоимость ещё не подтверждены: предыдущая попытка не собрала отсутствие Debug у Cache, исправлено.

По новой инструкции пользователя дальнейший перенос приёмов следует после сканирования 500 тематических репозиториев и записи в mcp-rag. Transmission теперь выключен по умолчанию; только VOXY_FACE_EXPERIMENTAL_TRANSMISSION=1 разрешает эксперимент. Текущие изменения сохранены. Исследование/исходники не исполняются.

## Сканирование 500 тематических репозиториев перед дальнейшим переносом — 2026-10-02

По новой инструкции пользователя порядок изменён: сначала корпус 500 репозиториев и подтверждённая запись в mcp-rag, затем дальнейшее внедрение и отдельный разбор mouth/pharynx/tongue/teeth/gums. GitHub search дал 1741 уникального кандидата; выборка строилась по тематическим очередям с отсечением явных FLAME/fire и text-rendering/emoji совпадений. После 600 проверенных кандидатов получены исходники 502; выбран корпус 500. Сохранены 1781 полный исходный файл, pinned SHA, blob SHA, SHA256, topic paths, candidate function signatures и lexical keyword counts. Это механическое сканирование выбранных файлов, не глубокое чтение всех исходников каждого репозитория. 7 прежних selected-function reviews остаются частичными; ни одного полного deep review 500 репозиториев не заявлено.

Реестр docs/research/face-source-study-500.json и воспроизводимые scripts в docs/research/tools. Полный локальный корпус .research/face-source-study-500 исключён из Git, не исполнялся. 500 Markdown bundles с полным полученным кодом занимают 19,433,799 bytes; максимальный пакет 393,139 bytes. Хеши файлов проверены при сборке пакетов. RAG doctor перед ingest подтвердил schema/FTS/embed/relational integrity ready_for_search. RAG_INGEST_ROOTS ограничен Documents/Sources; пакеты скопированы в /Users/themoretheless/Documents/Sources/voxy-face-reference-study-500. sync_sources запущен для wing=voxy, room=face-source-study-500, remove_deleted=false; результат/наличие 500 источников ещё проверяются.

## Подтверждённая запись окончательного корпуса в RAG — 2026-10-02

После дополнительной проверки тематики заменены 24 исходных нерелевантных пакета; их локальные копии сохранены в excluded-rag-bundles. Окончательная выборка: 500 уникальных имён репозиториев, 1801 исходный файл, 500 пакетов полного полученного кода (19,438,925 bytes). Три группы имеют одинаковые source samples, отмечены отдельно. Дальнейший перенос по новой цели разрешён после этого исследовательского этапа, но перенос конкретного механизма ещё требует чтения соответствующего кода.

Первый MCP sync превысил 300-секундный срок ожидания клиента, сервер продолжил обработку; сохранились 499 пакетов, все BLAKE3 совпали. Один исходник содержал NUL, поэтому поисковое представление явно экранирует его как literal \u0000; исходный файл и его SHA256 сохранены, transformation отмечен в metadata. Ошибочно выбранные 24 RAG records, созданные этой задачей, удалены по точным source_file путям; чужие источники не затронуты.

Background source_sync job a480588e-eba1-45fd-b0ca-497884604890 завершился succeeded: added=25, updated=1, skipped=474, errors=0, processed=500. Scoped list_sources: ровно 500 raw records в wing=voxy / room=face-source-study-500; missing=[], extra=[], hash_mismatches=[] для всех BLAKE3. Протокол docs/research/face-source-study-500-rag-proof.json содержит document IDs и хеши. Doctor подтвердил schema/FTS/embed readiness, ноль orphan chunks/document nodes/edges, relational_integrity_ok=true. Wiki wiki://voxy-face-source-study-500 создана (document_id 65a434fa-abf3-4925-9b1f-4b76d005dcaa), индекс перестроен. Это завершение механического сканирования и записи; глубокий разбор всех 500 репозиториев не заявлен.
