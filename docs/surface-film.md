# Тонкая жидкая плёнка

`physics::surface_film::SurfaceFilm` — нейтральный прототип на треугольных ячейках поверхности, единицы SI. Материал задаёт плотность (кг/м³), динамическую вязкость (Па·с), поверхностное натяжение (Н/м) и эффективную диффузию смачивания (м²/с). Это параметры материала, не проверенная физиологическая модель.

Объём хранится в ячейке; толщина равна объёму/площади. Потоки используют гидростатический потенциал, кубическую подвижность толщины и дискретную лапласовскую оценку капиллярного давления. Донорное ограничение сохраняет положительные объёмы. Внутренний шаг не больше 1 мс. Ограничитель не доказывает точность: сходимость по времени и сетке требуется проверить для выбранного материала и меша. Поток через открытые границы закрыт. При движении поверхности объём каждой ячейки сохраняется, толщина пересчитывается.

API: `new`, `deposit`, `update_geometry`, `step`, `total_volume`, `thickness`, `vertex_thickness`. Проверки: сохранение объёма, стекание, равновесие, смачивание сухой ячейки, изменение площади и атомарное отклонение неверной геометрии.

```sh
cargo run --release -p voxy_app --example surface_film
```

Пример наносит 0.05 мл жидкости на участок туловища с радиусом 4 см. `SceneApp::with_surface_film(center, radius, volume_m3)` позволяет выбрать другой участок исходного меша. Дубликаты вершин OBJ свариваются по исходной позиции, чтобы текстурные швы не прерывали поток. Сетка обновляется из отображаемой деформированной поверхности; расчёт использует геометрию начала шага. Цвет и приближённый блик зависят от рассчитанной толщины; отдельного физически точного жидкого BRDF пока нет.

Режим контактного угла ограничен малыми углами и описан ниже. Пока отсутствуют отрыв капель, испарение, вязкоупругость и двустороннее взаимодействие жидкости с тканью. Нельзя считать это завершённой моделью смазки в контакте или физиологически откалиброванным решателем. Анатомические области специально не размечены: привязка задаётся пространственной кистью на существующей поверхности.

## Источники и контактные связи

`add_sources(dt, [(cell, rate_m3_s)])` добавляет явно заданные источники. `exchange_with(other, dt, [(cell_a, cell_b, conductance_m2_s)])` консервативно переносит жидкость между двумя плёнками по разности толщин; одновременный отток по нескольким связям ограничен объёмом донора. Неверная связь отклоняет весь обмен. Связи можно передать явно либо получить геометрическим поиском `detect_bridges`; коэффициент переноса остаётся феноменологическим. Объём сохраняется; при разных плотностях материалов этот обмен нельзя считать сохранением массы.

## Геометрический поиск мостиков

`detect_bridges(other, BridgeConfig)` строит BVH по треугольникам второй поверхности, проверяет расстояние между треугольниками, встречное направление нормалей и положительную площадь проецированного перекрытия. Контакт только ребром/вершиной не создаёт проводимость всей грани. Число кандидатов ограничено; превышение бюджета возвращает ошибку. `exchange_contact` объединяет поиск и консервативный обмен. Плотности должны совпадать, чтобы сохранение объёма означало сохранение массы.

`transfer_speed` (м/с) задаёт феноменологическую скорость мостика; проводимость — скорость × площадь перекрытия / максимальный зазор × коэффициент близости. Это геометрически согласованная модель переноса, не решатель давления в трёхмерном жидком мостике. Поиск статический: для быстро движущихся поверхностей необходимы подшаги или CCD. Пересечение твёрдых поверхностей этот API не устраняет. Сцене нужно явно передать вторую поверхность — в одноповерхностном примере автоматический перенос не возникает.

## Контактный угол в режиме тонкой плёнки

`set_wetting(Some(Wetting { contact_angle, precursor_thickness }))` включает потенциал смачивания `f(h) = -A/(2h²) + A hp³/(5h⁵)`, где `A = 5 γ hp² θ² / 3`. Это малоугловая связь `θ² = -2 f(hp)/γ`; диапазон угла ограничен 0–0.5 радиан. В давление включается производная потенциала, регуляризованная ниже hp/4. Метод основан на подходе с прекурсорной плёнкой и расклинивающим давлением, применяемом в [тонкоплёночных моделях смачивания](https://www.uni-muenster.de/Physik.TP/~thiele/Paper/EWGT2016prf.pdf). Параметры не описывают автоматически конкретную ткань.

`seed_precursor()` явно добавляет начальную плёнку и возвращает добавленный объём: скрытого источника массы нет. Этот объём затем участвует в общем балансе. Равномерная плёнка hp на плоском основании без тяжести находится в равновесии. Лапласовское давление теперь учитывает дискретную кривизну основания; гидростатическое давление по толщине использует нормальную компоненту тяжести, а не её полную величину.

## Численные проверки и измерения

`step_with_max_substep` позволяет сравнивать шаги вплоть до бюджета 100000 внутренних шагов. Тест двух ячеек с чистой диффузией сравнивает решение с аналитическим экспоненциальным затуханием; ошибка уменьшается примерно вдвое при уменьшении шага вдвое. Обмен между совпадающими плоскими участками сохраняет результат при сгущении сетки 1×1, 2×2, 4×4. Эти проверки не доказывают сходимость капиллярного течения на произвольной криволинейной сетке.

`cargo run --release -p physics --example surface_film_bench` выдаёт CSV времени 120 шагов для 128, 512 и 2048 треугольников. Сохранённые локальные измерения: `docs/surface-film-benchmark.csv`. Это CPU-измерения синтетической плоской сетки; они не характеризуют полный рендерер или анатомический меш.

The body preview now draws a separate, fixed-topology film surface, offset by
solver vertex thickness plus a 10 micrometre depth separation. Welded vertex
normals keep OBJ seams aligned. Dry fragments discard; partial coverage ramps
between 0.1 and 3 micrometres. The substrate vertex colours remain unchanged.
The material evaluates air/water Fresnel with IOR 1.333, refracted optical path
length and Beer-Lambert transmission, plus finite emitter reflections. Absorption
coefficients are illustrative (0.2, 0.08, 0.04 inverse metres), not physiological
measurements. The current render target composites transmitted substrate colour
in the film fragment; it does not yet sample refracted scene imagery or provide
sorted alpha transparency. Thus this is a separate optical coating, not completion
of the requested transparent refractive liquid rendering.

Reproduce the clinical torso view with:
`cargo run -p voxy_app --release --example female_render -- --film --torso --snapshot docs/body-film-closeup.png`.
The brush deposits 0.05 ml over a 4 cm radius at the upper abdomen. This is an
explicit demonstration deposit, not an anatomical secretion source.

Smooth shading now uses a second vertex buffer at shader location 3, generated
from the final geometry rather than fragment derivatives. Area-weighted normals
weld exact coincident positions (including signed zero) across OBJ seams and are
rebuilt on each geometry update. `SceneVertex` remains unchanged; existing shaders
can ignore the added stream. Degenerate/cancelling fans produce zero normals.
The female shader consumes this stream for skin and film. A test checks both seam
welding and a known normal change after deformation; four scene unit tests pass.
The Metal close-up `body-film-closeup-smooth.png` was rendered and inspected: the
previous isolated triangular emitter highlight is absent. CPU mesh-update time
in this single run was 49.42 ms; it includes mesh construction, normals and GPU
buffer writes and is not a controlled performance comparison. The extra normal
stream costs 12 bytes per allocated vertex. Caching/profiling remains necessary.

Geometry now caches normals with exact positions and index topology. Changes
only to UVs/colours or repeated identical geometry skip normal rebuild and normal
buffer upload; any position/index change invalidates the cache. This preserves
fresh normals for moving tissue and changing film thickness. The cache adds CPU
copies of positions, indices and normals; it is not a deformation approximation.

On Apple M4 Max / Metal, release `female_render --film --torso --geometry-bench`,
24 updates per case (GPU completion between samples), 492599 vertices / 931091
triangles: unchanged geometry median 3.4000 ms, deformed geometry median 31.1004
ms. Timing covers `SceneGeometry::update`, excluding mesh construction and GPU
completion. Deformed samples alternate a 0.1 mm global translation to force
invalidation. See `body-normal-cache-benchmark.csv`. These measurements do not
prove whole-frame real-time performance or the cost of anatomical deformation.

Dynamic normal generation now reserves weld-table and fan storage up front and
normalizes each welded fan once before scattering to render vertices. Exact seam
welding and summation order are unchanged. A matched 24-sample release/Metal run
measured dynamic geometry update median 23.8046 ms (previous run 31.1004 ms),
unchanged update 3.3317 ms. See `body-normal-optimized-benchmark.csv`. The image
before/after this CPU optimization has identical SHA-256. These are sequential
runs on a shared host, not isolated statistical proof of speedup. Dynamic update
still exceeds a 60 Hz frame budget before other frame work.
The female material also uses bounded reciprocal length and a geometric fallback
for cancelling/zero smooth normals, avoiding normalization of a zero vector.

`SurfaceFilm::set_material` validates all new properties, rescales volumes by
old_density/new_density and preserves mass; invalid/overflow/underflow edits
leave state unchanged. `material()` exposes the applied properties. The body
film preview accepts watched `.film.json` settings via MCP and adds an independent
source over the initial deposition cell weights. It reports initial mass and
source-added mass so conservation can be checked with an active source. Rate zero
is off. This source is an explicit demonstration input, not physiology. Its spatial footprint is editable through canonical model-space coordinates
and radius. Relocation changes future input only; it does not relocate existing
liquid. Geometry selection occurs before changing any state, so a source missing
the surface rejects the whole configuration. Up to 16 named additional sources are supported alongside the legacy primary
source; all use the same fluid material. Removing a source changes future input
only. Zero-rate and zero-weight entries are skipped when constructing inputs.

The optical preview now accepts index of refraction and RGB absorption through
FilmSettings/MCP. Four per-draw parameters reach the vertex/fragment shader in a
16-byte instance stream (location 4), with no per-vertex material duplication.
`SceneMesh::with_material_parameters` validates finite values; geometry upload
and update propagate them. The film material evaluates exact unpolarized Fresnel
and scales finite-emitter reflection accordingly, including zero reflection for
index 1. Thickness continues to come from the solver. Native comparison images
`film-optics-clear.png`/`film-optics-absorbing.png` were rendered and inspected;
30384 pixels differ. Colour-dependent optical controls are not calibration.

### Area-integrated source footprint

Initial deposition and all continuous sources now integrate the radial brush
kernel over canonical triangle area with seven-point degree-five quadrature.
This removes the previous bias toward regions with more triangles. Material
cell attachment is preserved through body deformation. A fully covered
triangle uses an exact quartic integral; triangles crossing the brush boundary
use quadrature and converge under refinement. Very small brushes can still
miss all quadrature points on coarse triangles and require local refinement.

Validation: `cargo test -p voxy_app surface_film_preview --lib` passed five tests.
One locally refines only half of a square and preserves the aggregate source
fraction within 1e-14. Another compares clipped-brush subdivisions at levels
2 and 4 against level 6; the level-4 fraction error decreases and is below
1e-5. Existing tests cover total source mass, relocation, atomic rejection,
and independent sources. This is source-distribution convergence on planar
fixtures, not full-body flow or time-step convergence.

### Thickness diagnostic rendering

MCP `film_update` accepts `display_mode: "thickness"` or `"material"`.
The diagnostic shader uses interpolated solver thickness in metres with an
unlit fixed colour scale: blue at 0, green at 50 micrometres, red at 100
micrometres, saturating beyond 100. Dry fragments below 0.1 micrometres
remain discarded. The view does not change liquid mass or transport.
`body-film-thickness.png` was rendered and inspected on Metal; coarse
triangle boundaries are visible and are not evidence of anatomical detail.
Pressure and strain displays remain separate unfinished requirements.

### Shared driving-potential diagnostics

`SurfaceFilm::driving_pressure(gravity)` returns the pressure-equivalent
potential used by the transport solver, sharing its implementation with
substep integration. Units are Pa; the value includes gravitational potential
relative to the coordinate origin, substrate/free-surface capillarity, and
optional disjoining pressure. It is not tissue contact pressure. Preview
measurements expose `drivingPotentialRangePa`,
`drivingPotentialIncludesGravity`, and `drivingPotentialError`. Diagnostic
failure is reported without panicking or mutating the liquid.

The hydrostatic two-cell test checks exact expected values, zero-gravity
values with capillarity disabled, invalid gravity rejection, and unchanged
mass. All 12 surface-film physics tests passed after extraction. A visual
pressure map and actual tissue contact-pressure coupling remain unfinished.

### Whole-body temporal study

Run the explicit ignored integration study with:
`cargo test -p voxy_app body_mesh_temporal_convergence_and_mass_budget --lib -- --ignored --nocapture`.
It loads the actual body OBJ, uses the production preview deposition, evolves
0.05 seconds at substeps 1, 0.5 and 0.25 ms, and writes
`docs/body-film-time-convergence.json`. It checks positivity and a relative
mass budget below 1e-12, then compares area-weighted L1 thickness differences
normalized by initial volume against the finest run. The finer error must be
smaller than the coarse error. Passing this short static-body fixture does
not establish long-time, spatial, moving-body or tissue-coupled convergence.

Measured body study: 84,680 triangles, static substrate, 0.5 ml initial
brush and viscosity 0.001 Pa s (engineering stress fixture, no physiological
calibration). At 0.05 seconds, the area-weighted L1 change from initial
volume is 39.2 percent, so this exercises nontrivial transport. Error against
0.25 ms reference drops from 0.3086 percent at 1 ms to 0.1018 percent at
0.5 ms. The checked engineering gate is below 0.2 percent for the finer
comparison, plus more than 10 percent distribution change and mass error
below 1e-12. Maximum measured relative mass error is 6.51e-16. All thicknesses
are finite and nonnegative. This is refinement evidence against a numerical
reference, not an exact physical solution or a proof for all operating modes.

### Prescribed body deformation and source budget

`cargo test -p voxy_app body_deformation_with_sources_preserves_mass_budget --lib -- --ignored --nocapture`
loads the full 84,680-triangle body, changes height/torso depth/waist/weight
through a complete sinusoidal prescribed morph, and advances two independent
sources for 40 steps of 1 ms. Density changes from 1000 to 2000 kg/m3 halfway
through, conserving existing mass. Maximum surface displacement is 0.20065 m.
Analytical source input is 1.8e-6 kg and measured input agrees; worst relative
mass-budget error is 1.474e-15. Thickness remains finite and nonnegative at
every step. Report: `body-film-deformation-budget.json`.

The intentionally rapid morph is an engineering geometry/budget stress test,
not a realistic body motion. The solver does not account for inertial liquid
entrainment or exert two-way tissue forces. These results therefore do not
verify physical coupling or self-contact under this motion.

### Configurable precursor wetting

Film settings and MCP now expose `precursor_wetting_enabled` (default false),
`contact_angle_rad` (0..0.5), and `precursor_thickness_m` (1e-9..1e-5 m).
Preview configuration connects these to the existing small-angle wetting
potential. Enabling or disabling changes the transport potential without
seeding a hidden precursor volume; existing liquid mass is retained. Bounds
are engineering and long-wave validity constraints, not biological calibration.

Integration test verifies enabled pressure influence, preserved mass,
invalid-angle rejection and restored disabled potential. MCP tests passed;
release viewer/MCP built. The live viewer acknowledged enabled settings in
Presented frame 3 while keeping mass at 5e-5 kg; saved proof is
`film-wetting-live-proof.json`. This verifies wiring and application, not an
experimentally measured equilibrium angle, full droplet geometry or shedding.

### Fold study exposes unresolved mesh dependence

The explicit ignored test `concave_fold_accumulates_film_by_capillarity_without_mass_loss`
uses a Gaussian concave fold: depth 1 mm, width scale 3 mm, a uniform 100 um
film, viscosity 0.001 Pa s, zero gravity, and surface tension 0 versus 0.04 N/m.
It evolves 0.05 seconds at 25/49/97 columns and five rows, decreasing the
substep from 100 us to 6.25 us to 0.391 us. All measured thicknesses remain
finite/nonnegative; mass error stays below 1.7e-15. The test passes only mass
and positive central accumulation, not spatial convergence.

Central-volume accumulation above control drops from 0.017728 to 0.005918
to 0.001687 of total volume. This is substantial anisotropic-grid dependence,
which remains after much smaller time steps. The earlier same-step finest
case was also time sensitive. Therefore fold retention is NOT established.
The centroid-dual curvature/transport discretization needs further analysis
and correction before claiming mesh-independent fold physics. Historical measurements above describe the previous curvature operator.
`film-fold-retention.csv` is overwritten by each successful explicit study run. Run with
`cargo test -p physics --test surface_film concave_fold -- --ignored --nocapture`.
The fixture is engineering validation, not physiological calibration.


The substrate curvature now uses the cotangent position Laplacian projected
onto area-weighted vertex normals, with barycentric vertex areas. This is
not the mixed Voronoi-area construction of Meyer et al. Boundary values are
not established by the following interior test. The analytic cylindrical
parabola y=20*x^2 has signed curvature -40/(1+(40*x)^2)^(3/2) for upward
normals. Refining only x at 25/49/97 columns while keeping seven rows gives
maximum interior relative errors 0.0006469874, 0.0001619644, 0.0000405047.
The dry-film pressure test isolates substrate geometry from fluid transport.
The transport operator still uses centroid-distance conductances; this test
alone does not establish convergence of liquid flow or fold retention.

After the curvature replacement the same fold study passes conservation and
positive accumulation, with accumulation above control 0.027664933374,
0.016632902213, and 0.009384568804. Relative mass error is at most 3.276e-16.
Accumulation still changes substantially under refinement. The remaining
transport/discrete integration dependence is unresolved; fold retention
remains unverified as a mesh-independent prediction.


The fold measurement now clips each planar triangle against the fixed strip
-2 mm <= world x <= 2 mm and integrates cell volume with the clipped area
fraction. Previously, accepting entire cells by their centroids changed the
measurement domain across meshes (visible even in the zero-tension control).
An independent clipping test checks exact 3/4 area, empty/full intersections,
and an inclined triangle. Historical accumulation values above use the old
centroid measurement and must not be compared directly to the updated CSV.

With fixed-strip integration, zero-tension central fractions converge to
0.201760387800 / 0.201821690695 / 0.201832189085. Capillary accumulation
above control remains 0.028446385775 / 0.017312875944 / 0.009292624997.
Thus measurement-domain aliasing was real but does not explain the flow
nonconvergence. The transport discretization still requires correction.


Cross-edge conductance now uses the sum of centroid-to-edge altitudes after
unfolding the neighbouring facets: d = 2*(Aa+Ab)/(3*edge_length). The previous
Euclidean centroid distance included tangential displacement and attenuated
normal flow on elongated triangles. Both thickness Laplacian and transport
use this updated metric. A skew two-triangle gravity fixture checks flux
against length*h^3*rho*g/(3*viscosity) with relative tolerance 1e-7 and checks
volume conservation. This is an uncorrected normal-distance scheme; arbitrary
tangential gradients still need a cross-diffusion reconstruction. See the
[OpenFOAM finite-volume description](https://doc.openfoam.com/2312/tools/processing/numerics/schemes/laplacian/implementation-details/)
for face-normal gradient integration; this implementation does not claim to
implement its full corrected scheme.

The updated fold study gives accumulation above control 0.042479122136,
0.043824011796, and 0.044235402665 at 25/49/97 columns. The last increment
is 0.000411390869 (0.93% of finest accumulation), versus 0.001344889660
on the previous refinement. Mass error is at most 2.129e-15. The ignored
study now additionally requires decreasing refinement increments and less
than 2% change in this integral on the last refinement. These are engineering
acceptance criteria for this single short fixture, not general convergence,
physiological calibration or full-field validation. The former strong
attenuation with anisotropic refinement is resolved in this measured case.


The browser panel now exposes precursor wetting enable, contact angle in
degrees, precursor thickness in micrometres and material/thickness display.
Saving 12 degrees and 0.5 um was verified through the browser and persisted
as 0.20943951023931953 rad and 5e-7 m; reload restores the values. A separate
native viewer using that same test preset confirmed wetting/thickness,
disabled/material, then restored wetting/thickness in fresh Presented frames
3/6/9. Mass stayed 5e-5 kg to relative change below 1e-12. Evidence is in
body-wetting-panel-proof.json and body-wetting-panel-live-proof.json.
Presented receipts confirm renderer submission, not monitor scanout or a
captured native window. The owned test viewer was terminated after checking;
the panel is retained on port 8787 with its separate test preset.


The body panel also routes GET/POST /view to MCP view_get/view_update.
Its selector exposes material, displacement and strain with numerical legends
and compares measurements.view.mode in fresh frame receipts. Browser saving
strain was confirmed by frame 71. Displacement now shares the unlit diagnostic
palette with strain, so scene lighting no longer changes quantitative colours.
Static Metal renders body-displacement-unlit.png and its fixture variant verify
zero and prescribed nonzero displacement; these are not force-driven motion.
See body-view-panel-proof.json and body-view-panel-live-proof.json.


The full-body fixtures were rerun after changing the curvature and cross-edge
transport metric. On 84,680 triangles, area-weighted normalized L1 differences
against the 0.25 ms reference are 0.0034459862 at 1 ms and 0.0011314006 at
0.5 ms over 0.05 seconds. All three runs conserve mass to 4.34e-16 relative
error and change the distribution substantially (43% normalized L1 from its
initial state). The separate 40-step moving morph with two sources and density
1000 -> 2000 kg/m3 conserves the input-aware mass budget to 1.58e-15 relative
error. See body-film-transport-validation.json. These remain short fixtures;
full-body spatial convergence, fluid inertia and two-way tissue coupling are
not established by them.


The explicit ignored body_gravity_transport_spatial_refinement study keeps
the original piecewise-planar body surface and subdivides each triangle into
four twice: 84,680 / 338,720 / 1,354,880 triangle cells. Midpoint coordinates
use f64 and the volume of contiguous descendants is restricted to each original
triangle for comparison. Source quadrature is independently recomputed on each
mesh; initial distribution differences are reported separately. Gravity-only
transport (surface tension and wetting diffusion zero) runs for 0.02 seconds
at a common 0.25 ms step. The report distinguishes conservation, nontrivial
transport and decreasing spatial refinement differences. A decreasing trend
alone does not establish full-field accuracy, the continuum limit, temporal
independence on the finest mesh or capillary convergence. Run explicitly with
`cargo test -p voxy_app --lib body_gravity_transport_spatial_refinement -- --ignored --nocapture`.

The measured spatial study passes only the decreasing-difference trend:
normalized restricted-volume L1 errors against level 2 are 0.1855827798
(coarse) and 0.0720069718 (level 1). Initial source differences are only
0.0005171245 and 0.0000928732. Maximum relative mass error is 1.95e-15.
Therefore full-body transport accuracy is still unestablished: 7.2% final
field disagreement is substantial despite conservation. Remaining skew-gradient
reconstruction and temporal dependence on the finest mesh need separate checks.
Authoritative data: body-film-space-convergence.json.


Gravity flux now integrates the known tangential gravity gradient using the
average oriented facet conormal at the shared edge. Its old centroid-potential
difference is subtracted before adding rho*g dot conormal; hydrostatic film-
height, capillary and wetting potentials retain their existing discretization.
The upwind mobility donor follows this corrected edge driving gradient. A new
skew two-cell fixture applies gravity parallel to the edge and checks no
spurious cross-edge flow, alongside the perpendicular analytic-flux fixture.
General capillary/wetting cross-diffusion is still uncorrected; this gravity
fix does not implement a general nonorthogonal gradient reconstruction.

After the gravity conormal correction, the same body spatial fixture gives
normalized restricted-volume errors 0.0696498161 (coarse) and 0.0212888152
(level 1 against level 2), down from 0.1855827798 / 0.0720069718. Maximum
mass error is 8.67e-16. The final 2.13% disagreement still needs further
refinement and temporal separation; general spatial accuracy is not claimed.
Measured debug integration times are 3.67 / 16.73 / 64.54 seconds for the
three meshes; these are offline tests and do not establish interactive speed.

The static full-body time study was also rerun after gravity correction:
normalized L1 discrepancies against the 0.25 ms reference are 0.0017715461
at 1 ms and 0.0005873453 at 0.5 ms. Mass error is at most 4.34e-16 and
normalized distribution change is approximately 30.3%, so the fixture remains
nontrivial. These time and space studies use different durations and gravity-
only versus capillary configurations; their error values cannot be directly
subtracted to claim the cause of the remaining spatial disagreement.


Gravity centroid and conormal operands are now reused across internal substeps
of one call when the count is greater than one. They remain separate operands
to retain arithmetic order; single-substep calls avoid allocation, and each
new call recomputes from current geometry/material/gravity. A release benchmark
on 3,200 flat triangles and 50 internal steps observed identical thicknesses
(bitwise zero difference) versus 50 individual calls. Seven-sample batch
medians changed from 2.238 ms to 1.781 ms; paired batch/singles ratios changed
from approximately 1.04 to 0.84. This is a small integration fixture on a shared
host, not full-body frame-rate proof. Exact data: film-gravity-cache-before.csv,
film-gravity-cache-after.csv, film-gravity-cache-proof.json. The separate
benchmark was repeated after an all-tests run to exclude its concurrent-test
interference. The physics release suite ran 16 ordinary tests plus this
explicit ignored benchmark; the expensive zero-gravity fold study was skipped.


The browser panel now edits up to 16 additional sources through individual
cards (ASCII identifier, world coordinates/radius in mm, flow in ml/s).
It submits the entire ordered source array with the existing validated film
patch. Adding/removing draft cards, saving two flows 0.001/0.002 ml/s as
1e-9/2e-9 m3/s, retaining 30 mm as 0.03 m and restoring on reload were verified
through the browser. Duplicate IDs were rejected and the saved preset remained
byte-identical. Proof: body-source-editor-proof.json and body-source-editor.png.
This UI run did not verify a fresh native frame; previously verified underlying
source mass-budget tests are separate evidence.


Same-surface liquid exchange is available through detect_self_bridges,
exchange_self_contact and exchange_self_with. The proximity traversal queries
only wet cells, excludes shared-vertex neighbours and duplicate wet pairs
before distance testing/candidate accounting, and requires opposed normals,
projected area overlap and a gap reached by the sum of the two cell film
thicknesses. Links are canonicalized by cell index. Explicit self-link updates
validate every link and reject duplicate unordered pairs before mutation;
simultaneous outgoing transfers share the original donor budget. Returned
volume is gross volume moved, not a signed net transfer. Transfer speed remains
phenomenological and is not physiologically calibrated. Static proximity is
not CCD or a tissue nonpenetration solver. Native preview integration is not
yet enabled by this API addition.

Three regression fixtures verify one wet donor feeding two initially dry
opposed surfaces without mass loss or negative thickness, failure to bridge
a gap larger than the film, removal of contact after geometry motion,
shared-vertex exclusion, and atomic rejection of duplicate/self/invalid links.
The physics surface-film suite passes 19 ordinary tests; the two expensive
studies remain explicitly ignored in that run.


Native FilmPreview integration now supports self_contact_enabled (default false),
self_contact_max_gap_m (1e-6..0.01 m) and phenomenological
self_contact_transfer_speed_m_s (0..0.1 m/s), all persisted and bounded through
film_update. After ordinary transport, enabled contact exchange uses the current
geometry. Measurements.selfContact exposes enabled, last and cumulative gross
transferred volume and an error field. Contact failure leaves that exchange
unapplied and reports the error while retaining the completed ordinary flow;
the whole advance is not rolled back. Gross volume is not net source input and
can count repeated back-and-forth transfer of the same liquid.

The self-contact BVH is built lazily once and refitted after successful geometry
updates; its topology is retained. This reduces rebuilding work but has not yet
established full-body frame performance. Shared-reference queries remain thread
safe via OnceLock; geometry mutation needs exclusive access. The preview contact
fixture proves actual transfer, mass retention, separation and disabling. The
19 physics tests and eight ordinary preview tests pass; settings and MCP tests
also validate persistence. Browser save of 1 mm gap and 1 mm/s speed persisted
as 0.001 m and 0.001 m/s. Native frame 3 confirmed enabled mode without error
and retained 5e-5 kg mass. That body pose had zero transfer because no wetted
opposed contact existed: nonzero transfer is proved by the separate fixture,
not that full-body receipt. Evidence: body-self-contact-live-proof.json and
body-self-contact-panel.png. UI controls are hidden for older MCP processes
that do not return these settings, and older presets default to disabled.


Contact accuracy fixtures now compare the uniform opposed-plate exchange
against h_receiver(t)=h0/2*(1-exp(-2*K*t)), where
K=speed/max_gap*(1-gap/max_gap). Both patches are 10 mm square, gap 50 um,
initial donor thickness 200 um, receiver dry, speed 0.5 mm/s and maximum gap
1 mm. Only the contact operator evolves; gravity/capillary surface transport
and tissue motion are excluded. At t=1 second, errors divided by initial
thickness are 0.0091000193 / 0.0044540986 / 0.0022039717 for dt=100/50/25 ms.
The test requires decreasing errors and a finest error below 0.003, an
engineering tolerance, not physiological validation. With dt=25 ms and
4/16/64/256 triangle cells the receiver volume is 6.1766692e-9 m3, invariant
within a 1e-12 relative comparison tolerance; maximum mass error is 4.913e-15.
This symmetric aligned fixture does not establish arbitrary curved-contact
convergence. Data: film-self-contact-time-convergence.csv and
film-self-contact-convergence.csv. All 21 ordinary physics surface-film tests
pass; the two expensive unrelated studies were ignored in this run.
The plot script tools/plot_film_contact_validation.py renders these measured
CSV values and requires Matplotlib.
