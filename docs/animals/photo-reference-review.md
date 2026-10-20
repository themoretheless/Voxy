# Photo and open-model review — 2026-10-15

This review records images actually viewed, rather than search-result titles.
The target is still realistic Voxy-owned animals. External geometry has not
been copied into the active generator.
The user explicitly confirmed reference-only use: all geometry remains authored
in Voxy, including when an open model is used to study proportions.

## Viewed references

- Real Labrador standing in profile, Royal Kennel Club:
  https://www.royalkennelclub.com/media/1307/retriever-labrador-standing.jpg?rmode=pad&v=1d5750641175780&width=1000
  Breed page: https://www.royalkennelclub.com/search/breeds-a-to-z/breeds/gundog/retriever-labrador/
- Real brown mackerel tabby standing, side/three-quarter profile:
  https://images.petful.com/posts/brown-tabby-cat/brown-mackerel-tabby.webp
  Source page: https://www.petful.com/cat-breeds/brown-tabby-cat/
- Labrador Dog, kenchoo, rebuilt from “Dog” by all of life:
  https://sketchfab.com/3d-models/labrador-dog-1f56cfbab07e4fe49b5d9e521c82073a
  The live page states CC Attribution and links CC BY 4.0. Its rendered surface
  was inspected from front/three-quarter and rear views by orbiting the viewer.
  The page reports 52800 triangles and 26900 vertices. Raw topology, weights
  and source files have not been downloaded or validated. This is a shape
  reference, not an incorporated asset.
- Cat by squibblejack, BlendSwap 23047, CC0:
  https://blendswap.com/blend/23047
  **Rejected:** the visible thumbnail has exaggerated eyes and an anthropomorphic
  face. A permissive licence does not make it a realism reference.

Photos are viewed in place. No permission to redistribute the photo files as
textures is assumed; none are embedded in the game or checked into the repository.

## Findings applied to the owned geometry

- The former union of every rounded axial section inflated the chest, skull and
  muscle roots. One nearest centreline section now defines each continuous loft.
  Only the complete loft's terminal ends are rounded; the nose end is clipped.
- Feline skull and muzzle are authored separately from canine sections. A small
  triangular nasal planum replaces the round projecting nose. Socket and eye
  feature positions share one profile method, avoiding coordinate drift.
- The photographed chest extends further down the standing legs than our old
  shallow ribcage. Authored rib depths were revised while keeping shoulder height.
  These are modelling targets informed by silhouette review, not measurements
  recovered from calibrated photographs or official breed requirements.
- Canine forehead height and shoulder-root width were reduced. The feline
  abdominal section is fuller. Fore-elbow offsets now scale with shoulder height.
- A neutral `--animal-clay` mode removes rendered fur and uses one body material;
  anatomical features remain visible. `--animal-side` adds a lateral inspection
  view. `--animals-output PATH` keeps each run's evidence separate.

## Acceptance

Native captures must be compared visually against the references. Runtime pass,
mesh counts and deterministic export do not establish realistic anatomy.
Earlier attempts and evidence are retained. The first new neutral run still
showed an overly shallow torso and was revised again; it is not a final asset.

The croup now extends behind the hip before closing, removing the visible notch
under the tail in the native lateral capture. Muscle union width was increased
to soften attachment seams. An experimental finite-span field failed the
continuity test and strand simulation and was reverted; it is not in the active
implementation. Latest neutral evidence is in
`artifacts/animal-photo-study/croup-revision/`.
Realism remains unaccepted: face detail, anatomical attachments and coat appearance
still need substantial work. Passing movement and guide simulation does not
qualify the requested full animal/fur physics.

## 2026-10-16 continuation

Hair roots now use the supporting skin's blended weights. Hairs on muscle
transitions retain those weights along their length; rigidly attached hairs
continue to use simulated guide tips. This removes the dominant-bone shortcut
that pulled roots away from blended skin. The regression poses all ten bones
and checks root positions for both species, including thousands of blended
attachments. Transition hairs currently have no guide dynamics of their own.

The fibre width increased from 65 to 120 micrometres per side, with a less abrupt
tip taper. Root colour no longer darkens by 28 percent. The photoreal fibre path
decodes its stored roughness and includes overcast sky energy, matching the
lighting sources used by the body. These changes still require native visual
inspection; width and fibre count alone do not establish a convincing coat.

Feline whisker pads and a shorter lip line were authored; an initial pad outline
hid the nasal planum in the native capture and was reduced and moved backwards.
Upper limb volumes were moved inward so the shoulder roots meet the ribcage.

The updated native Labrador/domestic-shorthair run produced three captures,
grounded both animals, travelled 2.71/2.10 metres and simulated 288 guide
particles. The animal test suite passed nine tests with one benchmark ignored;
the four model tests also passed earlier in this continuation. These are scoped
checks. Native close-ups still show an implausible speckled coat and simplified
face/feet. The assets have not passed visual realism acceptance.
After reducing excessive sky fill, two native reruns exited successfully after
only the first capture at tick 180. Their movement report is false at that early
tick; these runs are incomplete and do not replace the earlier three-capture
pass. The cause of the early window exit has not been established.

## Continuous goal: shared coat lighting

Animal fibres now carry the supporting skin normal independently of their
reconstructed comb tangent. Their new packed material modes preserve the
existing tangent-normal human hair path. The animal path shares body sky,
occlusion, environment and fog lighting; its direct response mixes the coherent
coat response with a 15 percent single-fibre lobe. This is an explicit bundle
scattering approximation, not a physically exact multiple-scattering solver.
The native comparison in `anatomy-review/coherent-coat/frame-540.png` removes the
conspicuous bright dots on the flank seen in the preceding `fur-light` capture.
Both runs completed three captures and passed movement/grounding/guide checks.

Labrador muzzle width and feline head carriage were revised. The nasal plane
also clips the whisker-pad union behind the visible nose. Anatomy and detailed
ears/paws remain inadequate; the goal is active. See `realism-acceptance.md` for
the unchanged full acceptance scope.

## Authored pinnae and digital paws

Upright pinnae are now closed thin meshes with concave front bowls, a rim and
rear wall, authored in `voxy_model/src/animals/ears.rs`. They replace the solid
loft cones for the five upright-eared breeds. Inner ear vertices carry a separate
region mask; the active animal material uses a muted inner-skin tint and omits
dense grooming there. The existing Labrador folded ears remain to be reworked.
A manifold/winding-area/normal/concavity regression covers each upright breed.

Distal paws now have interdigital clefts subtracted from the final skin field;
the former appended toe ellipsoids were removed because the paw volume buried
most of them. Grid spacing decreased from shoulder height / 90 to / 125. The
native lateral run shows the separation but its cleft edges remain visibly rough;
this is not accepted as finished paw anatomy.

A new regression constructs the actual gameplay sculpt for all six breeds,
checks normal and index validity, complete material-region masks and nasal plane
limits. All six model tests pass, including this full-sculpt regression; earlier
neutral GLB export checks alone did not cover the gameplay extraction. A missing
all-false region mask on folded-ear breeds was found and corrected.

The pinna lateral and three-quarter native runs completed three captures each.
The first digital-paw run was interrupted by a window-close event at tick 101;
the new diagnostic correctly returned failure. Its repeat completed all three
captures and passed grounding, movement and 288-guide-particle simulation.

The updated German Shepherd/British shorthair and Husky/Maine Coon native runs
also each completed three captures and passed their scoped runtime reports
(`pinna-shell/breed-checks.json`). The Maine Coon close-up is still rejected:
its face is too canine, its long coat lacks coherent volume, and the paw clefts
show bright/speckled edges. These results identify the next modelling work;
they do not support an overall realism claim.

## Maine Coon head and regional draped grooming

Primary reference: CFA breed page https://cfa.org/breed/maine-coon-cat/ and the
CFA presentation https://cfa.org/wp-content/uploads/2024/06/maine-coon-cat-full-presentation.pdf.
Page 8 was rendered and visually inspected: photographs compare a straight
profile with the desired slight concavity and a diagram shows the blunt muzzle.
The reference was viewed for anatomy only; no photo or external mesh was added
to the game. Source dimensions remain authored, not calibrated photo measurements.

Maine Coon now has separate authored head sections with a broader blunt muzzle
and deeper chin rather than sharing the tapering domestic-cat outline. Head
length is 112 mm and muzzle length 36 mm; these are modelling targets.
The long coat now samples 255000 rendered followers versus 85000 on other breeds,
with distinct ruff, belly, tail, shoulder, face and distal-limb lengths.
A facial-region regression prevents the chest from inheriting short facial hair.

The first native lateral run exposed straight upward-projecting long strands.
Long followers and guides now share `Root::rest_point` for an arched, gravity-draped
rest curve. Initialization, both palette paths and displacement reporting use the
same curve. Native movement completed all three captures with 288 simulated guide
particles; the follower count is not a simulated-particle count.

The first three-quarter draped capture still showed overlong hair below the chin
and on distal legs. The face mask was extended to include the lower jaw and the
short distal-limb rule now takes precedence over belly length. Overall coat
quality remains unaccepted: it is still visibly sparse at the silhouette and
requires better volume, clumping, antialiasing and physical contacts.

## Antialiased fibre pass

Animal fibres now render after opaque skin/scene geometry in a separate alpha
blended photoreal pipeline with depth testing but no depth writes. Upload-time
contiguous triangle ranges retain index order; mixed-material triangles remain
opaque. A regression proves that the ranges cover every input index exactly once
and preserve opaque triangles. The existing human tangent-normal fibre mode is
unchanged.

Per-vertex ribbon side and longitudinal position are packed in exact binary
fractions of the existing material coordinate. The camera uniform includes the
viewport. The vertex stage expands subpixel ribbons to a minimum screen width,
while the fragment stage integrates both physical ribbon edges with a box filter.
The physical strand positions used by the guide simulation are unchanged.

The first alpha attempt over-attenuated thin strands and is retained in
`fibre-coverage/native/`; its smooth image is not accepted as a successful coat.
The corrected screen-space attempt in `fibre-coverage/screen-space/` restores
visible long fibres without the former hard stipple. It completes three native
captures and passes the scoped movement report. Pipeline creation and draw state
also pass the noop validation backend; that test alone is not image evidence.

The screen-space image is still rejected as a finished animal: the long coat is
too translucent and lacks coherent clumps/dense undercoat, and the face/body
still look sculpted and simplified. Transparency is currently ordered by authored
triangle order, not a full order-independent transparency method. Legacy non-HDR
rendering still uses its existing opaque fibre path. Full physics and six-breed
visual acceptance remain open.

## Microclumps and world contacts

Long coats now distinguish short dense-undercoat followers from longer guard
followers. Local 8 mm follicle cells define deterministic microclumps within each
bone region. Only distal curves converge; roots stay pinned, and offsets are
projected into the local skin tangent plane and bounded by strand length. The
native microclump capture passes movement but still lacks convincing coat volume;
it is not a visual acceptance.

The strand solver now accepts validated solid box contacts in addition to skin
spheres. Free particles project out of the union of expanded boxes; pinned roots
remain unchanged. The old sphere-only API forwards an empty box list. Animal
simulation gathers nearby full-cube terrain obstacles, treats unavailable cells
as solid, and includes nearby player/companion AABBs. The bounded query is rebuilt
for the current groom pose and uses the same world registry as body collisions.

The real-voxel floor regression initially failed at shared voxel faces, exposing
an error in independent per-box projection. Connected axis intervals now remove
internal faces from the contact surface. The corrected regression includes the
actual inspection-pad blocks, neighbouring-body admission, fixed roots and free
particle clearance over 30 steps. A separate 120-step floor test and invalid-box
transaction test pass. Contacts are discrete particles, not continuous segments,
and do not establish coat self-collision or a complete articulated body solver.

The corrected world-contact build passes 12 animal tests (three manual benchmarks
ignored). The Husky/Maine Coon native run in `fur-world-contact/union/` completes
three captures and the scoped movement/grounding/288-guide-particle report. Its
image is still not accepted as realistic; the forced-contact regressions, rather
than the ordinary standing screenshot, prove floor projection behaviour.

### Close portrait inspection: continuous skin and eye construction

The native face inspection revealed severe transverse ridges which distant coat
captures concealed. Overlapping finite spans, capped continuous spans, smoothing,
and pure clay shading were individually insufficient. The retained field selects
one section by its monotone longitudinal coordinate, with rounded endpoint caps;
section generation keeps that coordinate linear between authored nodes. This
replaces the earlier nearest-centreline approach. `canonical-sweep/frame-540.png`
in the face-inspection artifacts confirms that the ridges disappear in the
actual native renderer. It does not establish realistic anatomy.

Marching tetrahedra now derives triangle orientation from the same inside/outside
samples, welds crossings at grid endpoints, removes collapsed triangles, and
recomputes normals from the oriented faces. All six breed meshes pass the closed,
consistently wound edge regression. Cat muzzle endpoint sections were rounded and
retracted; a separate cap-width regression guards against the broad planar disk.
The portrait camera uses a 20-degree field of view and a longer viewing distance.

The close portrait remains visibly stylized: over-simple cranial silhouette,
small eyes, triangular nose, rigid spoke whiskers and primitive ear placement.
The next eye revision replaces overlapping iris/pupil fans with a curved annulus,
radial colour variation, a dark peripheral limbus and a separate pupil patch.
Its broader iris is a construction correction, not a realism acceptance result.

Eye seating now shares breed-specific orientation and radius between the socket
and visible feature geometry. Feline eyes face more forwards and have a larger
aperture; the iris dome depth was reduced. The first annulus capture exposed
back-facing triangles despite the movement report passing. Winding was corrected
and a viewer-facing triangle regression was added for both species and sides;
the complete animal suite passes 13 tests, with three benchmarks ignored.

The nasal surface now contains sculpted nostril depressions and dark interiors,
rounded upper wings and a shallow central groove. Appended black nostril spheres
were removed. This is owned procedural geometry; no reference mesh is imported.

### Lid margins, curved whiskers and facial groom

The eye patch is recessed by 0.22 eye radii, uses the existing corneal material
response, and has a narrow four-ring lid margin connecting the dark aperture to
coat colour. Native `lids-whiskers/frame-541.png` confirms visible eyelid margins,
but the far-side eye still protrudes beyond the cranial silhouette. The shape is
not accepted as anatomically realistic.

Whiskers now originate in several follicle rows, follow quadratic curves through
12 tapered segments, and have varied lengths. Previous straight constant-width
spokes from one point were removed. Their subpixel rendering still aliases in the
native close portrait and is not accepted as final.

Facial groom sampling has three times the area weight of the body, while total
fibre counts stay fixed. Only triangles with all three groomable vertices are
sampled, avoiding follicles across feature boundaries. Nose-pad hair stays short;
Maine Coon forehead and cheeks receive 4 mm and 7 mm hair before variation.
The face comb flows laterally and backwards, with matching shader tangent flow.

### Cranial volume revision and side portrait diagnostics

The Maine Coon head now has a lower cranial vault, separate cheek/brow sections,
a narrowing bridge, and a shorter-depth distal muzzle section. Feline eye centres
move medially and posteriorly together with their subtractive sockets. Seven
model tests (including all six closed gameplay meshes) and thirteen app animal
tests pass after this change.

The face camera previously took precedence over `--animal-side`; side selection
now takes precedence so a close profile can actually be inspected. Several native
runs were interrupted by CloseRequested before reaching all three normal capture
points. These are failures, not successful movement qualification.

`--animal-fast-portraits` schedules the three existing native GPU captures at
20-tick intervals for geometry diagnosis. It deliberately retains the movement
acceptance condition; early portraits with no travel must report failure. Such
images are visual evidence only and do not replace the standard 540-tick check.

The retained feline gameplay field uses an asymmetric cross-section: the lower
cranial region tapers more strongly than an ellipse, transitioning continuously
to a broader muzzle section. This removes part of the spherical lower-cheek
silhouette. Native `feline-jaw-front/frame-60.png` shows the resulting taper;
`low-vault-side-portraits/frame-60.png` exposes the still-unaccepted neck/jaw
transition and flat eye construction. The fast portraits correctly end with a
failed movement report because the animals have not travelled yet. Seven model
and thirteen app tests pass; these tests are not visual acceptance.

The normal-interval `feline-jaw-coat/` native run subsequently completed all three
captures and the full scoped movement/grounding report at tick 541. Its close
three-quarter image confirms cheek taper and visible facial fibres, but also
shows unaccepted eye/lid seating, thin irregular coat coverage and whisker
aliasing. This remains a prototype, not a realistic finished breed model.

### Cross-strand ribbon basis correction

A geometry regression on actual generated facial fibres exposed that the ribbon
width basis was fixed relative to world up rather than perpendicular to the hair.
The worst absolute width/strand dot product was 0.84861475 in the failing baseline
(`/tmp/voxy-face-ribbon-reproduction.log`), collapsing laterally combed cheek
ribbons nearly edge-on. CPU ribbon generation now uses the supporting normal
crossed with the authored strand direction, with a stable parallel fallback.
The photoreal vertex shader reconstructs the same basis for subpixel expansion.

The added regression inspects both species' generated facial ribbon vertices,
requires over 1000 face fibres, and bounds width/strand alignment below 0.01.
All 14 animal app tests pass, three manual benchmarks remain ignored. This proves
noncollapsed authored ribbons, not photorealistic coat appearance or full fibre
contact coverage.

The corrected `cross-strand/` native run completes all three normal captures and
the movement/grounding report. The close frame shows more visible cheek/chest
fibres, yet still exposes the bald-looking forehead, sparse coat volume and
unrealistic eyes; the coat remains unaccepted.

Long draped fibres now use four segments instead of two, sampling their existing
curved rest shape more closely. Intermediate vertices blend the same mid/tip
guide translations continuously; short fibres retain two segments. Guide weight
quantization partitions one fixed influence between mid and tip so their sum
cannot exceed 65535. This keeps the existing 144 simulated particles per animal;
it improves rendered interpolation, not simulation resolution or self-collision.

### Spatial pigmentation and spherical iris study

The tabby coat now has separate body flank stripes, tail rings, forehead strokes,
cheek stripes and a pale muzzle/belly. Base skin and sampled fibres share the
same owned spatial colour function. Tabby fibres additionally have darker roots
and tips with a lighter middle band. The `tabby-coat/frame-541.png` native capture
shows these patterns on the actual groom; all 14 animal app tests and the normal
movement/grounding report pass. Pigmentation does not establish realistic anatomy
or coat density.

The next eye study keeps an originally circular iris clipped by the elliptical
lid aperture instead of stretching the iris into an oval. Iris and pupil share
a spherical depth/normal function and the existing corneal material response.
The intent is to remove the flat sticker appearance; visual acceptance still
requires inspection of the resulting native portrait.

Native `spherical-eye/frame-542.png` reveals that the initial 0.65-radius feline
iris leaves excessive black corners. The retained `eye-aperture/frame-540.png`
study uses a larger feline iris clipped by the lid and a shared 1.05-radius
spherical surface; dogs retain the smaller iris. Both normal native runs complete
all captures and the scoped movement report, and 14 app tests pass. The eyes
still protrude and lack a convincing wet lid transition, so this is not visual
acceptance of realistic eyes. The new coat patterns are retained, while dense
coat volume, whisker antialiasing and breed anatomy remain open.

### Pinna thickness, folds and continuous skin transition

A new paired-surface regression reproduced ear self-intersection: the old deep
front bowl crossed its planar back (first reported separation -0.0005375743 m).
The back now follows the authored bowl with positive variable cartilage thickness.
The bowl is deeper, the rim rolls forwards, and a curved basal fold interrupts the
flat triangular interior. All upright breeds retain closed shell topology.

The model exports a continuous `inner_ear_weight` alongside its grooming mask.
The app blends fur colour into muted pinna skin with this weight, replacing the
hard triangular colour patch. The native `curved-pinna/frame-541.png` shows the
curved interior and removal of most of the former white interior hair patch.
Seven model and fourteen animal app tests pass, and the normal native capture run
passes its movement/grounding report. Ear fur, silhouette and lighting remain
visually unaccepted; this fixes geometry and transitions, not the full realism
requirement.

### Eyes seated against the actual cranial volume

A regression against the authored cranial volume reproduced floating eye anchors
(the first Labrador failure was 0.0027363703 m outside the head). Eye anchors now
project onto that same section volume along the breed's eye direction, rather
than using a fixed position independently of cranial geometry. Visible features
and subtractive sockets consume the same projected anchor. The section-volume
constructor is shared by the main sculpt and projection to prevent formula drift.

All six breeds pass the surface-anchor and inside/outside offset regression;
eight model tests and fourteen app animal tests pass. The normal
`conformed-eye/frame-540.png` native run completes all captures and scoped
movement qualification. It remains visibly stylized, with thin coat volume,
oversimplified facial shapes and unconvincing eye appearance. Surface attachment
is proven; realistic anatomy and eye rendering are not.

### Whole-body review across the other four breeds

Fresh native runs in `breed-review/labrador-domestic/` and
`breed-review/shepherd-british/` complete three captures and scoped movement
reports. Visual inspection rejects all four: smooth toy-like coats, excessive
front chest/neck bulges, over-simple feet, weak breed distinction, and overly
regular stripes. The new Maine portrait must not stand in for all-breed quality.

The neck base moved deeper and posteriorly inside the chest, and British Shorthair
received its own broad cheek/short deep muzzle sections with elliptical cranial
cross-sections (the generic tapered feline field is excluded for that breed).
`neck-british/` passes normal native movement checks, but the chest bulge persists.
The next revision retracts and raises the torso's anterior closure beneath the
neck rather than merely changing the neck.

That geometry change exposed four inconsistent directed edges in the British
mesh. This was a local tetrahedral face-orientation defect, not a relaxation
index change. The retained closed-surface orientation pass propagates consistent
winding through shared edges and chooses outward orientation from component
signed volume. It rejects open, nonmanifold and nonorientable surfaces instead
of silently accepting them. The strict all-six mesh regression remains intact;
eight model and fourteen animal app tests pass after the correction.

The corrected `sternum-retry/` normal native run passes all three captures and
movement qualification. Its dog and cat whole-body frames show the lower chest
pendant largely removed after retracting the torso closure. The remaining
shoulder/leg transitions, skin-like coat appearance and simple head shapes are
still rejected as realistic. The first `sternum/` run was closed at tick 187;
its one capture is not movement qualification.

The frozen sternum build also completes native runs for Labrador/Domestic
Shorthair (`sternum-labrador/`) and Husky/Maine Coon (`sternum-husky/`). Together
with `sternum-retry/`, all six current breeds have fresh three-capture movement
reports. Their individual full-body frames were visually inspected. Chest closure
improved, but the animals remain toy-like; these reports establish only the
existing movement/grounding/guide-particle criteria, not realistic animal physics
or finished breed assets.

### Close paw inspection, authored digits and foot attachment

`--animal-paw` tracks the posed front foot for native close portraits. The old
`paw-inspection/baseline-portraits/frame-60.png` exposes severely jagged carved
interdigital grooves. Its fast portrait movement report is deliberately false;
the normal baseline run was closed before its first scheduled capture.

The retained paw replaces sub-grid subtractive grooves with four authored digit
sweeps fused into the palm. Foot-region union blending is smaller than torso
blending, with a continuous height transition. Digits and the heel were shortened
after `authored-digits/` exposed overly long fingers. The sculpt grid is now
shoulder-height/180 with a bounded 24-million-cell budget, up from /125 and
8 million; generation cost increases and has not been performance-qualified.
All six meshes still pass closed/wound/normal validation (eight model tests).

Native `compact-paws/` and `pads-claws/` complete three normal captures and scoped
movement reports. Compact digit forms are smoother, but the ankle and toe
surfaces remain insufficiently realistic. Pad pigmentation is applied to the
underside; it is not a pressure-deforming anatomical pad model. Dogs receive
owned closed curved keratin claw geometry; cats keep retracted claws. The first
claw portrait shows exaggerated protrusion, so the retained claw length and bend
were reduced afterwards. That reduction has not yet been visually inspected.

A digit attachment regression reproduced distal weights `[19105,0,46430,0]`,
leaving part of the digit attached to the torso. A smooth footprint mask now
assigns distal digits to their corresponding foot joint while retaining blending
at the ankle. Fifteen app animal tests pass after this correction.

The user confirmed that they manually close the foreground game window because
it gets in the way. Do not automatically relaunch foreground game checks during
ongoing iteration. The subsequent foot-weight and claw-length changes are checked
by tests/build only; their current native visual and movement qualification is
pending. Continue nonintrusive geometry/render verification and retain that gap.

### 2026-10-19 — silhouette diagnosis after user rejection

Added `voxy_model/examples/animal_silhouettes.rs`, a windowless CPU triangle rasterizer of the actual `build_sculpt` geometry. Matching side/front/three-quarter clay renders are saved under `artifacts/animal-rebuild/{before,after}/labrador.png`. These are diagnostic geometry views, not gameplay or fur/physics validation.

Labrador first correction broadens the authored skull from 145 to 185 mm, shortens total head length from 235 to 225 mm (muzzle 105 to 95 mm), deepens the cranial sections and increases paw half-width from 37 to 43 mm. Visual comparison still fails realism: cylindrical muzzle, simplified neck/chest transition and hind limbs remain conspicuous. No claim of acceptance. No native game window was opened, respecting the user's stated interruption concern.
