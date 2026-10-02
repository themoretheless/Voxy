# Статус доработки тела

Реализованы подготовка геометрии, базовая разметка, локальная перестройка физики вторичных областей, независимые стороны, панель через MCP и обмен между жидкими плёнками.

`tools/prepare_body_geometry.py` сваривает точные дубликаты позиций, проверяет индексы, вырожденность, дубликаты граней, граничные/неманифолдные рёбра и направление граней. Опциональная BVH-диагностика ищет пересечения несмежных граней, включая копланарные; пары с общими вершинами исключены. Это диагностика с допуском, не доказательство отсутствия всех пересечений и не CCD.

Подготовленная версия `assets/characters/blender-female/prepared/body-v1.obj`: 76052 вершины, 152100 треугольников, ребро не длиннее 12 мм. Линейное согласованное сгущение сохраняет форму поверхности и не создаёт анатомических деталей. Исходные UV и нормали не переносятся; этот геометрический результат нельзя подменять в рендерере без новых привязок и материалов. Sidecar содержит хеш источника, индексы областей, топологические метрики и кандидаты на пересечения. Разметка эвристическая и требует анатомического ревью. Внутренняя анатомия не создана; наличие замкнутой наружной сетки не означает наличие внутренних поверхностей.

Остаются: проверка/исправление обнаруженных пересечений, достоверные внутренние поверхности, полноценные морфы с обновлением кожи/скелета/волос, неоднородные материалы и самоконтакт тканей, расширение малоугловой модели смачивания и подключение геометрического поиска жидкостных контактов к сцене, капли, отдельный жидкий BRDF, рассеивание кожи, свет и разрезы, захват текущего окна через MCP (отдельный снимок и подтверждение Presented реализованы), исследования сходимости и производительности.

Проверки подготовки: `python3 -m unittest discover -s tools -p test_prepare_body_geometry.py`. Подготовка новой версии: `python3 tools/prepare_body_geometry.py assets/characters/blender-female/body.obj /absolute/new/body.obj --max-edge-mm 12 --intersections`. Исходный меш не перезаписывается.

## Перестройка кожи и региональные свойства

В `Skin` добавлены `rebased(rest)` и `set_face_properties(stiffness, density)`. Перестройка заново рассчитывает исходные метрики, массы, шарниры и закрепления, сохраняет региональные коэффициенты и настройки параллелизма; движение и релаксационная память сбрасываются. Это библиотечный API для редактирования формы, не перенос динамического состояния.

В демонстрации полной кожи используются условные региональные множители жёсткости: 0.7 для грудной области, 0.85 для живота, 1.6 для кистей/дистальных участков рук и 1 для остальных областей. Калибровка по измеренным тканям отсутствует. Плотность и контактная площадь разделены: изменение региональной массы не увеличивает площадь контактного штрафа. Смена параметров в UI/MCP теперь перестраивает оболочку кожи из неизменной исходной формы; массы и коэффициенты привязок пересчитываются, динамика оболочки сбрасывается. Волосы пересобираются в изменённых координатах; полная перестройка скелетных опор ещё не подключена.

Ограниченный ремонт исходного меша в `body-repair-candidate-v1.audit.json` не смог принять ни одного улучшения в пределах 5 мм без нарушения ориентации граней. Кандидат совпадает с исходной геометрией; `repair_complete: false`. Пересечения не устранены. Их необходимо исправить в исходных анатомических областях либо более полноценным ограниченным геометрическим решателем; результат не подменён гладкой оболочкой.

MCP `model_status` и панель теперь различают сохранение файла и представленную рендерером модель. Реальный обмен проверен на Metal; подтверждения сохраняются рядом с пресетом, доказательство проверки — `docs/body-presentation-live-proof.json`.

MCP `model_snapshot` now renders an immutable copy of current body parameters at pose time zero, returning a PNG resource link and exact parameters. Native Metal execution and visual inspection passed; evidence is in `model-snapshot-proof.json` and `body-model-snapshot.png`. This is offscreen rendering, not live viewer capture. The screenshot still shows simplified skin and hair; it does not establish anatomical or physiological accuracy.

Film rendering now uses a separate displaced surface with solver thickness,
air/water Fresnel and thickness-dependent optical path/transmission. A unit test
proves unchanged substrate and liquid volume and the expected geometric offset;
the actual model advance test and Metal offscreen render passed. Full scene
refraction, sorted transparency and calibrated absorption remain incomplete.

The final skin/film geometry now supplies area-weighted smooth normals through a
separate GPU vertex buffer, recomputed after mesh deformation. Seam/deformation
unit tests and Metal rendering passed; the earlier triangular highlight was not
present in the inspected close-up. Measured single-run CPU mesh update was
49.42 ms, so this does not establish the performance requirement. Full liquid
scene refraction, calibrated materials and the other outstanding plan items are
still incomplete.

Normal caching skips rebuild/upload for identical geometry and colour-only
changes while invalidating on deformation/topology updates. Four scene tests
pass. Native measured geometry-update medians are 3.40 ms unchanged / 31.10 ms
changed for the 492599-vertex body+hair+film mesh. Dynamic cost remains unresolved;
full frame throughput still needs measurement and optimization.

Dynamic normal storage preallocation and one normalization per welded fan reduced
the observed update median from 31.10 to 23.80 ms in matched sequential runs.
The CPU optimization preserved the rendered PNG byte for byte. Zero-normal shader
fallback also rendered successfully on Metal. This remains above a 60 Hz frame
budget and does not complete the performance or realistic-material requirements.

MCP simulation measurements are now available through `model_measurements` from
actual presented-frame receipts. Native Metal verification returned film volume,
configured-density mass, maximum cell thickness and four coarse cage volumes with
frame/time/freshness metadata; evidence is `model-measurements-live-proof.json`.
These are simulation metrics, not calibrated anatomical measurements. Material
and source controls, physiological calibration and long-run whole-model gates
remain open.

При смене пропорций оболочка кожи теперь получает новые исходные метрики и массы через `Skin::rebased`. Коэффициенты упругих и вязких привязок масштабируются по изменению локальной массы/площади; опорная анимация строится из исходной сетки и затем преобразуется теми же параметрами. Смещение кожи переносится на уже изменённую форму, без повторного масштабирования. Переход сбрасывает скорости, релаксационную память и глубину пробника. Это намеренная перестройка, а не сохранение динамического состояния. Барицентрическая топология привязок сохраняется; полноценные анатомические морфы и перестройка волос/суставных опор остаются открытыми.

Проверки перестройки: восемь тестов параметров; тест оболочки на повторное применение, возврат к исходной массе/сетке, атомарный отказ и шаг полной физики при росте 190 см и глубине торса 1.3; прежний тест деформации модели на 30 шагах; тест сохранения топологии и движения. Все прошли. Снимок на Metal: `body-skin-rebased.png`. Камера общего плана теперь учитывает рост, чтобы верх модели не обрезался при увеличении высоты. Это не проверка предельной устойчивости всех сочетаний параметров.

Для перестройки волос добавлены `HairRod::rest_positions/rebased` и `HairSystem::rebased`. Они пересчитывают длины, исходные материальные рамки, массы и инерцию, сохраняют материал и настройки решателя, атомарно отклоняют неправильные кривые. Это библиотечный этап: интеграция с параметризованными корнями, коллайдером и системой координат модели подключена и проверена на одном физическом шаге.

Волосы теперь пересобираются при смене параметров: канонические кривые преобразуются до создания стержней, коллайдер использует ту же изменённую поверхность, фолликулы сохраняют исходные индексы кожи с новыми смещениями. Рендер не применяет морф второй раз. Для движения головы используется преобразование через локальный аффинный кадр головы; положения корней и коллайдера берутся из полного нелинейного преобразования. Это приближение ориентаций при анизотропном масштабировании, не полная перестройка скелета. Смена формы заново создаёт укладку и сбрасывает её динамику. Тест роста 182 см/головы 1.2 на одном шаге физики прошёл, изображение `body-hair-rebased.png` просмотрено. Длительная устойчивость сочетаний параметров и физиологическая масса всей причёски не подтверждены.

MCP `film_get/film_update` подключены к наблюдаемому `.film.json`: плотность, вязкость, поверхностное натяжение, феноменологическое растекание и независимый расход источника. При замене плотности сохраняется масса (объём пересчитывается). Учёт источника включает массу начального нанесения и массу введённой жидкости. Проверка Metal+MCP: кадры 0/3/6 подтвердили исходные свойства, удвоенную плотность и включённый источник; при смене плотности объём уменьшился вдвое без изменения массы, после источника масса равна начальной плюс введённая. Доказательство: `film-control-live-proof.json`. Контролируется один источник в области исходного нанесения; редактирование его положения, несколько источников и оптические материалы ещё не подключены. Это инженерная демонстрация, не физиологическая калибровка.

Положение и радиус одиночного источника теперь редактируются через MCP (`source_x/y/z_m`, `source_radius_m`). Область выбирается в канонических координатах и следует за ячейками при деформации. Тест на двух раздельных поверхностях подтвердил сохранение прежней жидкости и ввод только в новую область; попадание мимо поверхности атомарно отклоняет конфигурацию. Metal+MCP подтвердили координаты/радиус/расход в кадре и баланс массы (`film-source-relocation-live-proof.json`). Несколько одновременных источников и физиологическая калибровка остаются открытыми.

Добавлены до 16 дополнительных независимых источников через `sources` в MCP. Все источники и основной скалярный источник используют один материал; каждый задаёт уникальное имя, область и расход. Замена списка атомарна при применении, а удаление источника сохраняет нанесённую жидкость. Тесты подтвердили сумму вводов и отключение; Metal+MCP подтвердили два источника и их отключение в кадрах 0/3/6 (`film-multiple-sources-live-proof.json`). Физиологическая калибровка, капли, оптические параметры и оставшиеся этапы плана ещё не завершены.

Подключены оптические параметры плёнки через MCP: показатель преломления и RGB-поглощение. Они передаются в шейдер отдельным 16-байтным буфером на геометрию и влияют на отражение/оптический путь/поглощение. Metal+MCP подтвердили применение в кадре 3 без изменения массы (`film-optics-live-proof.json`); сравнение двух рендеров дало 30384 изменённых пикселя, снимки просмотрены (`film-optics-render-proof.json`). Это ещё приближённое покрытие поверхности: преломление полной сцены, прозрачная композиция и рассеивание кожи остаются незавершёнными.

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

### Surface-area morph preflight

`BodyParameters::surface_quality` evaluates the same morph used by rendering
(default parameters preserve reference positions exactly). It rejects missing
triangles, invalid indices, nonfinite vertices, degenerate reference/output
faces, and relative area below 1e-12. It reports min/max area ratio, minimum
area in m2, and normal rotations beyond 90 degrees. Rotations are diagnostic,
not treated as proof of inversion or self-intersection.

`FemaleDemo::set_body_parameters` invokes this check before rebuilding hair,
volume cells and skin, so failure leaves existing state intact. Six real-body
parameter combinations pass: baseline, height/weight limits, opposed torso
and local-volume limits, and independent sides. An injected collapsed face
is rejected. Measured report: `body-morph-surface-quality.json` (84,680 faces).
This does not check triangle intersections, internal surfaces, region accuracy
or guarantee all combinations in the permitted parameter domain.

### Untextured skin shader fallback

The female shader's zero-UV fallback now adds GGX highlights, mesoscopic
roughness variation (0.38..0.58), and twelve mixed-direction microheight
waves. Analytic projected-pixel filtering suppresses subpixel relief. This
is procedural normal perturbation, not geometry displacement or measured
pores. Coordinates are world-space and may slide under deformation;
material-attached texture coordinates remain necessary. Zero-UV untextured
props in the same shader also use this fallback. It retains existing vertex
lighting and does not implement physical subsurface scattering.

Release renderer built and the WGSL executed successfully on Metal. The
same torso framing was rendered before and after and inspected; the initial
regular wave pattern was replaced after visible artifact review. Final PNGs:
`body-skin-material-before.png`, `body-skin-material-after.png`; hashes in
`body-skin-material-proof.json`. Visual refinement is partial and does not
establish photorealism or calibrated optical skin properties.

### Material coordinates attached to the reference body

The earlier world-space microrelief limitation is now superseded for the
body: `SceneMesh::with_material_coordinates` supplies explicit bind positions
at vertex location 5. The female mesh uses canonical body coordinates and
constant zero coordinates for appended parts that do not use this fallback.
Coordinates are validated for count and finiteness. GPU geometry stores and
caches the coordinate stream; unchanged values do not trigger a GPU rewrite.
Generic meshes without explicit values resolve coordinates from their current
positions, retaining the world-position fallback.

The shader evaluates relief and roughness in material space, uses material
screen derivatives for filtering, and reconstructs the surface gradient from
current position derivatives. This accommodates stretching without sliding
the pattern across tissue. It still uses procedural relief rather than
measured skin textures and does not add subsurface scattering.

Six scene tests pass, including explicit coordinate preservation across vertex
deformation and invalid-input rejection. The release shader executes on Metal;
`body-skin-attached.png` was rendered and visually inspected. Additional vertex
stream memory and per-frame CPU construction/comparison need inclusion in
whole-frame performance measurements; no frame-rate improvement is claimed.

### Coordinate update allocation and benchmark

Explicit material coordinates now resolve as a borrowed slice, avoiding the
previous full clone before equality comparison. The persistent cache is copied
only when values change. Fallback coordinates still allocate from positions.
At 505,989 vertices the avoided temporary is 6,071,868 bytes per update.
The deformation benchmark now preserves explicit material coordinates while
moving geometry, matching attached-material semantics.

Six scene tests pass and release shader rendering on Metal succeeds. In one
24-sample paired run, unchanged geometry median is 4.3594 ms before versus
3.9606 ms after. The corrected dynamic benchmark median is 24.6967 ms; its
old result is not comparable because it also changed the material coordinate
stream. Shared host load limits causal interpretation of these timings.
These exclude mesh construction and GPU completion, so do not prove 60 FPS.
CSV and render hashes: `body-coordinate-copy-before.csv`,
`body-coordinate-copy-after.csv`, `body-coordinate-copy-proof.json`.

### Shell deformation measurements

Native presentation receipts and MCP `model_measurements` now include
`measurements.skinShell`: physical triangle count, principal-stretch range,
area-ratio range, incompressible thickness range in metres, shell mass in kg,
and stored energy in J. Metrics are relative to the current rebased stress-free
shell. `solverActive` distinguishes full-shell simulation from secondary-only
and animation-only modes; these values do not describe the separate cages or
all rendered triangles. Invalid metric evaluation returns an error instead
of panicking. Material data remains uncalibrated physiologically.

A live Metal viewer in secondary-only mode reported 1,268 physical triangles,
stretches and area ratios near 1, thickness 1.65 mm, and `solverActive:false`.
The exact receipt is `body-shell-measurements-live-proof.json`. This confirms
MCP delivery, not tissue stability under large deformation. The affine fixture
checks stretches 0.8/1.2, area ratio 0.96, finite energy, solver-state reporting,
and atomic rejection of a collapsed state. A spatial strain map is still pending.

### Unlit physical-shell strain map

The existing native `K` strain toggle is now rendered with an explicit unlit
material tag rather than skin highlights/complexion. Colour encodes maximum
absolute deviation of either principal stretch from 1: blue 0%, green 5%,
red 10% and above. This includes compression and extension magnitude, not
stress. Physical triangle values use the existing render embedding; shell
resolution is 1,268 faces and boundaries remain visible.

Offscreen renderer supports `--strain` and `--strain-fixture 1.05`. The latter
prescribes an affine width stretch on the physical shell after body-preset
application, solely for diagnostic validation; it is not a simulated loading
experiment. Both reference and prescribed fixtures rendered on Metal; final
images were inspected. Images and hashes: `body-shell-strain-rest.png`,
`body-shell-strain-stretched.png`, `body-shell-strain-render-proof.json`.
The previous statement that a spatial strain map was pending is superseded.
MCP display-mode control and a visible legend in the native viewport remain
unfinished, as do stress/contact-pressure maps and full validation.

### Persisted view control through MCP

`view_get` reads `<body>.view.json`; `view_update` patches `{settings:{mode}}`
with `material`, `displacement`, or `strain`. The native viewer validates and
loads changes on advance; presented measurements include the actual mode at
`measurements.view.mode`, including keyboard changes. Compare this against
saved settings in a fresh receipt rather than treating save as application.
The file selects presentation only and does not restart or alter simulation.
MCP now exposes eleven tools.

Offscreen `model_snapshot` respects the saved mode and returns `view` metadata.
It remains a fresh offscreen body state, not a capture of live deformation.
The release MCP/viewer/renderers built and view validation tests passed. A
real stdio session switched all modes and received matching presented-frame
receipts; an offscreen snapshot preserved saved strain mode. Proof:
`model-view-live-proof.json`, `model-mcp-strain-snapshot.png`.
Native legends, pressure mode, panel view selectors, and physiological
calibration are still pending.

### Spectral diffuse approximation (2026-10-01)

The untextured skin fallback now uses normalized wrapped diffuse for each
colour channel (red/green/blue widths 0.35/0.18/0.08). The extra normalization
keeps the hemispherical integral of each diffuse lobe equal to Lambert's.
This is a local surface shading approximation: it has no thickness,
nonlocal light transport, transmission or shadow diffusion, and does not
fulfil the planned subsurface-scattering requirement.

`cargo run --release -p voxy_app --example female_render -- --torso` passed
on Apple M4 Max / Metal. The same built renderer produced
`body-skin-spectral-diffuse.png` with `--torso --snapshot`; the image was
inspected. The change is subtle at this framing and the material still
looks smooth; no photorealism or calibrated tissue optics is established.

### Procedural pigment heterogeneity (2026-10-01)

The same fallback now modulates base colour with smooth 3D value noise at
80, 240, 720 and 2160 inverse metres in existing canonical material
coordinates. Independent fields vary general absorption-like tint and
redness, with decaying amplitudes; this is authored appearance rather than
measured melanin, perfusion or physiology. Each scale fades to its mean
when its pixel footprint becomes unresolved. Existing relief, optics,
diagnostic colours and textured face shading retain their own paths.

Release torso rendering passed on Metal and `body-skin-pigmentation.png`
was visually inspected. `body-skin-pigmentation-proof.json` records the
matched static image difference and file hashes. This evidence does not
establish deformation attachment, temporal antialiasing, GPU performance,
photorealism or complete skin materials. Real subsurface transport remains
unfinished.

### In-frame diagnostic numerical labels (2026-10-01)

The shared gradient mesh now includes small geometric bitmap labels and
an unlit vertex-colour shader path. Labels follow the active body mode:
0/5/10+ percent for strain magnitude, 0/5/10+ mm for displacement,
or 0/50/100+ um for film-only thickness display. Native overlay geometry
updates from the current mode; snapshots use the same label generator.
Strain takes precedence over displacement and film when simultaneous
diagnostics are active; separate film/tissue legends remain unfinished.

Release render `--strain --strain-fixture 1.05 --snapshot
docs/body-strain-labelled-legend.png` passed on Metal. The image was
inspected and labels are visible beneath the bar. This is a prescribed
deformation fixture, not a simulated loading experiment. Native screenshot
capture and narrow-window layout are still unverified.

### Simultaneous tissue and film legends (2026-10-01)

The shared legend generator now builds independent stacked rows for the
active tissue diagnostic and film thickness. Percent/mm labels identify
the tissue row; um labels identify film thickness. Film no longer loses
its scale when strain/displacement is enabled. Native geometry updates
only when its label rows change, rather than every frame.

Release rendering of prescribed 1.05 shell stretch plus a neutral torso
film patch in thickness mode passed on Metal. The saved
`body-strain-film-legends.png` was inspected: both numerical scales are
visible and the film patch uses its own palette. This supersedes the
single-scale limitation above. Narrow-window layout, native screenshot
verification, descriptive headings and measured contact-pressure maps
remain unfinished; this image is not a physiological validation.

### Scale-corrected static intersection audit (2026-10-01)

The preparation tool's segment/triangle determinant tolerance was absolute
despite having cubic length units, so small triangles could miss true
crossings. It now scales with the three vector lengths. Coplanar distance
and projected containment tolerances likewise scale with edge length.
Two independent geometric fixture tests passed for crossings, separated
triangles, coplanar overlap and parallel planes at scales 1e-6, 1e-3,
1 and 1e3. These are floating-point tolerance tests, not exact predicates.

The corrected full audit of prepared `body-v1.obj` tested 58,262 candidate
pairs and found 484 crossing pairs involving 426 triangles, without
reaching its 100,000-hit cap. Heuristic region face occurrences: head 512,
left/right foot 222/218, left/right hand 12/4. These counts include repeat
occurrences across pairs, not unique triangles per region. Report and
source hash: `body-intersection-scale-corrected-audit.json`; inspected
location plot: `body-intersection-locations.png`.

This prepared geometry is not substituted into the runtime model. It
still fails the requested intersection-free condition. Shared-vertex
pairs are excluded and this is a static test, not CCD or a general proof
of geometric validity. Existing earlier capped audits are superseded for
this specific prepared file; repair and internal anatomy remain required.

### Bounded local intersection repair candidate (2026-10-01)

The original unsmoothed separation accepted no steps on the corrected
audit. Repair now smooths the displacement field over three adjacency
passes. During each line-search proposal it freezes vertices of reversed
faces, rechecks their neighbors, then applies the existing global topology,
normal-direction and strictly-decreasing crossing-count gates. It does not
smooth the original positions or relax final validity checks.

Four accepted steps reduced crossings 484 -> 393 -> 385 -> 349 -> 308.
Maximum displacement is 1.781833 mm, within the 5 mm bound. Candidate:
`assets/characters/blender-female/prepared/body-local-freeze-repair-v4.obj`.
After saving at 17 significant digits and reloading, a complete static
audit again found 308 pairs, with unchanged topology, zero boundary and
nonmanifold edges, zero degenerate/duplicate faces, consistent winding and
zero face-normal reversals against the source. Reports:
`body-local-freeze-repair-proof.json` and
`body-local-freeze-repair-reload-proof.json`. Geometric fixture tests passed.

The source/runtime model remains untouched. The candidate is still not
intersection-free, contains no newly authored internal anatomy, and has
not been accepted as anatomically faithful. Larger-deformation and
shared-vertex intersection checks remain required.

### Extended bounded repair and export precision (2026-10-01)

A 16-step allowance from the same original prepared mesh accepted six
steps and then stopped at a local count minimum: 484 -> 393 -> 385 ->
349 -> 308 -> 307 -> 300. Subsequent proposals found 301, 307 and 305
pairs and were rejected. Maximum displacement is 2.143534 mm against the
original source, not a reset bound against intermediate candidates.

Candidate `body-local-repair-v5.obj` was saved separately and reloaded.
Complete static search confirmed 300 pairs, unchanged topology, no
boundary/nonmanifold/duplicate/degenerate faces and no face-normal
reversals. Evidence: `body-local-repair-16-step-proof.json` and
`body-local-repair-v5-reload-proof.json`. This remains an intersecting
candidate, not a runtime replacement or completed geometry preparation.

CLI OBJ export now uses 17 significant digits so audited double-precision
coordinates round-trip. Repair rejects nonfinite limits and capped initial
crossing counts rather than comparing incomplete totals. Three fixture
tests pass, including nonfinite limit rejection. Further local geometry
repair is needed; repeating the same proposals does not establish progress.

### Local diagonal replacement candidate (2026-10-01)

`tools/repair_body_diagonals.py` performs local two-triangle edge flips
without moving vertices. It requires two incident faces, a previously
absent new diagonal, bounded edge length, positive normal alignment and
nondegenerate replacement faces. A spatial hash computes changed-face
crossings; only strict count decreases are accepted. A final complete
static audit must exactly match the incremental pair set, and topology
checks must pass before export.

On the v5 candidate, 132 accepted flips reduced 300 pairs to 107 while
retaining 76,052 vertices and 152,100 triangles. Saved candidate:
`body-diagonal-repair-v6.obj`. Reloading confirmed coordinates exactly
unchanged, maximum edge still below 12 mm, no boundary/nonmanifold edges,
duplicates, degenerates or winding inconsistencies, and 107 crossing pairs
in an uncapped search. Remaining heuristic face occurrences: head 190,
left foot 10, right foot 14. Reports: `body-diagonal-repair-proof.json`
and `body-diagonal-repair-reload-proof.json`.

The source/runtime mesh is still untouched. Changed triangulation changes
the piecewise surface between fixed vertices; anatomical faithfulness and
shape deviation require review. Shared-vertex intersections are still
excluded. This candidate is not intersection-free or ready to substitute
into rendering/physics, and internal anatomy is still missing.

### Combined bounded and region-limited repair (2026-10-01)

Separation repair now accepts an explicit original displacement reference
and an optional movable-vertex set. Cumulative movement stays bounded
against v1 across triangulation changes; face-normal validation uses the
input triangulation, since its faces differ from v1. Invalid references,
already-out-of-bound inputs and invalid movable indices are rejected.
Four geometric tests pass, including cumulative-reference behavior.

A full v6 separation proposal increased crossings (118/124/115) and was
rejected. A foot-only pass accepted one step, 107 -> 105, then stopped
without an additional count reduction. Exported v9 was reloaded: unchanged
triangulation, coordinates outside feet exactly preserved, 105 pairs in
an uncapped search, zero boundary/nonmanifold/duplicate/degenerate faces
and consistent winding. Reports: `body-combined-bounded-repair-proof.json`,
`body-foot-local-repair-proof.json`, `body-foot-local-repair-reload-proof.json`.
Maximum cumulative displacement remains 2.143534 mm (5 mm bound).

The candidate is still intersecting and runtime geometry remains unchanged.
Region restriction is geometric, not anatomical authoring. The remaining
geometry, internal surfaces and overall original plan are unfinished.

### Adaptive local subdivision experiment (2026-10-01)

Extracted reusable conforming `split_edges`; it rejects absent/invalid
edges and budget overflow before producing a result. A closed-tetrahedron
fixture confirms area, winding and manifold closure preservation. Five
geometric tests pass.

Around v9's remaining crossings, 221 edge splits produced 76,497 vertices
and 152,990 triangles. Subdivision preserved area to floating-point error
and introduced no topological defects. Subsequent diagonal replacement
accepted 101 flips and reduced pairs on that refined mesh from 331 to 218.
Both local and full audits agreed. Reloading v10 confirmed the count and
topological checks. Evidence: `body-adaptive-diagonal-repair-proof.json`
and `body-adaptive-repair-comparison.json`.

Counts across different triangulations are not directly comparable. The
sum of unique implicated face areas was 6.274174e-5 m2 for v9 and
1.740752e-5 m2 for v10; this is mesh-dependent localization, not actual
penetration area/volume or proof of a better surface. v10 remains an
experimental intersecting candidate and is not adopted. v9 remains the
previous coarse candidate with 105 pairs. The requirement for an
intersection-free anatomical surface remains unfinished.

### Contact versus crossing classification (2026-10-01)

`classify_body_intersections.py` distinguishes transverse noncoplanar
intersection segments, tangential segments, point contacts, coplanar
positive-area overlaps, coplanar boundary contacts and separated parallel
planes. It clips projected coplanar polygons and compares noncoplanar
plane-intersection intervals with scale-relative tolerances. Known
fixtures for six classes passed in both argument orders at four unit
scales (1e-6 through 1e3). This remains floating-point static geometry,
not exact predicates, shared-vertex validation or CCD.

All 105 v9 findings classify as transverse segment crossings. None can
be dismissed as simple tangential/coplanar contact. Evidence:
`body-intersection-classification.json`. Classification does not remove
findings or relax the requested intersection-free requirement; further
surface repair remains necessary.

### Equal-count intersection segment descent (2026-10-01)

Added a scale-relative nonparallel triangle intersection-segment length
in metres, verified against a 0.6-unit fixture in both argument orders at
four scales with translation. Optional repair descent first minimizes
crossing count, then accepts equal-count proposals only when total segment
length strictly decreases. Coplanar pairs disable this secondary score;
count increases remain rejected. This is not a penetration volume metric.
Seven geometric test methods passed across the two test files.

A four-step foot-only run on unchanged v9 triangulation accepted
105 -> 105 -> 101 -> 98 -> 98 while reducing total intersection-segment
length from 0.028770868 to 0.023559893 m. The first equal-count step enabled
later count reduction, avoiding the earlier count-only plateau. Saved v11
was reloaded and independently verified: 98 pairs, identical triangle
indices, coordinates outside feet preserved, no topology defects or face
normal reversals, cumulative displacement 2.143534 mm against v1 (5 mm
bound). Evidence: `body-segment-descent-repair-proof.json` and
`body-segment-descent-reload-proof.json`. Runtime source remains untouched;
the candidate still fails the intersection-free requirement.

### Connected-component head repair (2026-10-01)

A whole-head proposal increased counts at every tested fraction and was
rejected. Audit-face connectivity identified five independent groups with
36, 23, 20, 16 and 3 pairs, respectively. Component records include the
exact source hash (`body-intersection-components.json`); these groups are
geometric, not anatomy labels.

The largest group's 139-vertex support mask accepted one equal-count step:
98 pairs retained, total segment length 23.559893 -> 22.476937 mm. Further
steps did not improve the objective. A second 90-vertex group's proposals
increased counts and were rejected. Saved v13 was reloaded: 98 pairs,
unchanged indices, coordinates outside the head preserved, zero topology
defects/normal reversals, cumulative displacement 2.143534 mm. Reports:
`body-component-repair-proof.json`, `body-component-second-repair-proof.json`,
`body-component-repair-reload-proof.json`. Runtime geometry is untouched.
The remaining 98 crossings are unresolved; neither reduced line length nor
the component decomposition establishes anatomical validity or completion.

### Nonlocal surface irradiance diffusion (2026-10-01)

Added `skin_light_diffusion.rs`: a positive edge-conductance screened
surface diffusion system `(M + radius^2 L) u = M source`, with barycentric
vertex areas and preconditioned conjugate-gradient solution. Conductances
are positive barycentric dual approximations, not a validated nonorthogonal
Laplace-Beltrami discretization. Solve convergence and area-weighted energy
budget are checked; failed solves reject mesh production. This operates on
the current deformed body geometry before colour shading and skips the
unlit displacement/strain diagnostics.

The body currently enables surface diffusion with illustrative RGB radii
2/1/0.5 mm. `female_render --no-surface-diffusion` disables it for comparison.
Three tests pass: nonlocal transport/channel order, constant irradiance and
invalid input, unit scaling and unequal vertex-area energy conservation.
These invariants apply to the diffusion field, not the full shaded image.

Matched torso snapshots from the same release binary passed on Metal and
the enabled image was inspected. `body-surface-diffusion-proof.json` records
hashes and differences: 52,300 changed pixels, maximum channel change
4/255. The visible effect is subtle at this framing. One CPU mesh sample
was 115.08 ms enabled versus 20.48 ms disabled: this flags substantial
cost requiring optimization, not a benchmark or GPU/FPS measurement.

This is a nonlocal surface approximation, not complete subsurface tissue
transport: no volumetric scattering, measured absorption/scattering,
thickness transmission or light/shadow transport through the body. Surface
mesh convergence, skin-material controls/MCP and performance validation
remain incomplete. It does not fulfil the full realistic-materials item.

### Diffusion stage profiling and rejected micro-optimization (2026-10-01)

Added an ignored release profile on the actual imported 42,342-vertex,
84,680-triangle body. Seven samples after warmup separate system assembly
and solve timing and record iteration counts. Baseline medians were
14.771875 ms assembly and 73.580417 ms solve, with RGB iterations 86/60/42.
The fixture uses static imported normals and is not full renderer timing.

Precomputing scaled edge weights and zipped vector updates preserved the
whole result bitwise but did not establish a speedup. A noisy initial
measurement and a repeat are both retained; repeat solve median was
74.456292 ms. The experimental changes were removed. Three release
invariant tests pass after removal. Profile instrumentation remains.
Evidence: `body-diffusion-profile-before.json`,
`body-diffusion-profile-after.json`,
`body-diffusion-profile-after-isolated.json`,
`body-diffusion-optimization-proof.json` (result and source hashes).

The costly solve, rather than just assembly, now has measured priority
for algorithmic/preconditioner improvement. No acceleration, interactive
frame rate, GPU timing or complete skin rendering is established.

### Material attachment under body edits (2026-10-01)

`skin_material_coordinates_stay_attached_across_morph_and_animation` passed
on the actual imported body. It changes height to 190 cm and torso depth to
1.3, advances animation, requires more than 10 cm actual vertex movement,
and compares every body vertex's explicit canonical material coordinate
exactly against the reference. It restores default proportions, advances
again and repeats the comparison. This confirms the CPU material-coordinate
stream remains attached through this morph/animation path. It does not
validate rendered texture derivatives, temporal filtering or all possible
physical deformations.

### Native diagnostic title scales (2026-10-01)

The native window title now prefixes the active diagnostic scale before
all body/animation/face/secondary-motion title branches: displacement
0/5/10 mm, strain magnitude 0/5/10 percent, and optionally film thickness
0/50/100 micrometres with its 0.1 micrometre discard threshold. The upper
endpoint is saturated, not a measured maximum. The film legend reads the
actual preview's current display setting. `cargo check -p voxy_app --lib`
passed after this change. A rendered in-viewport colour bar and live
window verification remain unfinished; a title is not a substitute for
that requirement.

### Native diagnostic colour bar wiring (2026-10-01)

The existing native overlay now contains a blue-green-red gradient for
female-model diagnostics, using the same unlit shader palette at values
0, 0.5 and 1. It is included only while displacement, strain or film
thickness display is active. The bar is positioned in screen coordinates
and numerical scales remain in the window title. `cargo check -p voxy_app
--lib` passed. This wiring has not yet been verified in a live native
window; embedded text, simultaneous separately labelled scales, narrow
window layout and offscreen snapshot legends remain unfinished.

The current release native viewer was subsequently launched with saved
strain mode and reached frame 52 (`rendererOutcome: Presented`) on Metal.
`body-legend-runtime-proof.json` records that receipt. This validates
submission through the diagnostic overlay path, not the bar's appearance.
Native screenshot capture could not bind the direct executable; a temporary
app wrapper returned `AXError.cannotComplete`. No visual confirmation is
claimed and the native legend requirement remains partially verified.

### Shared native/offscreen diagnostic gradient (2026-10-01)

Gradient geometry now lives in `diagnostic_legend.rs`, shared by the native
window and `female_render`. The offscreen renderer includes it only when
a diagnostic is active. Release rendering with `--strain --strain-fixture
1.05 --snapshot docs/body-strain-legend.png` passed on Metal; the resulting
image was inspected and visibly contains the gradient in the upper left,
without overlapping the body. This validates the shared geometry/shader
through offscreen rendering, not native window capture. In-frame numerical
labels and distinct legends for simultaneous tissue and film modes remain
unfinished.

### Refined nipple landmarks (2026-10-01)
Runtime `FemaleDemo` now loads the prepared reference mesh (45,203 vertices,
90,402 triangles) and corresponding full-coverage skin bindings. The 42,342
original bindings are preserved; 2,861 new points project onto the unchanged
636-vertex shell. Maximum added binding distance is 8.20 mm; this remains a
coarse displacement shell, not a newly resolved nipple tissue simulation.
Conforming midpoint refinement preserves the piecewise surface and area,
with local edge lengths at most 1.993 mm and no new topology defects.
`refine_body_landmarks.py` prepares geometry; `bind_refined_body.py` requires
NumPy and generates new bindings and normals. Run them in that order.
Release test `nipple_refinement_tests` verifies complete coverage, rest-pose
identity and a resolved cold morph. Both neutral and cold renders ran on Metal.
See `body-nipple-refined-proof.json` for image comparison; faceted lighting
around the deformation remains visible. Cold response is a normalized,
uncalibrated geometric stimulus, not temperature-dependent physiology.
Separate male geometry, volumetric tissue response and full original plan
requirements remain incomplete. Source reference assets are preserved.

### Transported surface normals (2026-10-01)
`surface_normals.rs` transports authored vertex normals through each triangle's
surface deformation using inverse-transpose frames and reference corner-angle
weights. Newly refined reference normals now interpolate source triangle normals
rather than use area-weighted normals of the refined triangulation. The source
positions and displacement bindings remain unchanged. Release tests cover
identity after nonuniform refinement, rotation, affine stretch, complete runtime
bindings and the cold morph. `body-nipple-transported-cold.png` was rendered and
visually inspected on Metal: the conspicuous triangular lighting patches around
the nipples are removed. This is still procedural appearance, not a validated
photorealistic anatomical model. One offscreen mesh update measured 57.67 ms;
this is not an FPS benchmark and indicates normal transport needs profiling and
caching before claiming interactive performance. General original plan remains
incomplete, including male geometry and physiological response calibration.

### Cached normal transport (2026-10-01)
`PreparedNormals` now stores source triangle frames and corner angles once at
model construction, after mouth topology is prepared. Morph changes retain the
same immutable reference mesh; current triangle frames are still recomputed for
each deformed pose. The actual 45,203-vertex body test compares cached and
uncached normal arrays bitwise. Release profiling (10 measured runs after one
warmup) gave upper-median body-only normal transport 1.973 ms cached versus
2.609 ms uncached, a 24.37% reduction. This excludes eyes and other frame work,
cache construction, and is not an FPS claim. The subsequent Metal PNG is
byte-identical to the preceding render; full mesh CPU update was 29.31 ms in
one sample, insufficient for a whole-frame speedup claim. Evidence:
`body-normal-transport-profile.json`, `body-normal-cache-render-proof.json`.

### Independent areola appearance controls (2026-10-01)
Body parameters/MCP/panel now expose `areola_radius_mm` (8..40 reference-space
mm) and `areola_pigmentation` (0..1 illustrative colour strength). These do not
alter geometry or cold response. Defaults retain the previous appearance.
Untextured-body UV metadata (x=2+radius in metres, y=strength) sends nondefault
settings to the shader; canonical coordinates keep pigment attached to the body.
The shader computes the smooth border per fragment, avoiding vertex-colour
faceting outside the locally refined mesh. Skin diagnostics override the metadata;
textured face and eye UVs retain their existing routes. Release parameter tests
cover bounds, JSON round trip, symmetry, back-surface exclusion and unchanged
geometry. MCP schema/save were verified in a temporary parameter file (not a
live viewer frame acknowledgement). Metal renders of radius 8 and 30 mm were
inspected. This remains uncalibrated appearance; independent left/right controls
and male geometry remain outstanding.

### Independent areola side sizes (2026-10-01)
`left_areola_size` and `right_areola_size` (0.6..1.6) multiply the shared radius,
using the same reference-side convention as breast controls (negative x = left).
Combined radii must stay within 8..40 mm; invalid partial MCP updates are rejected
without changing the saved parameters. Radius metadata blends only across the
central x=-40..40 mm strip, outside the landmark centres, avoiding a midline
attribute discontinuity. Two release tests verify pigment independence, bounds,
JSON preservation, and unchanged geometry. MCP success and atomic rejection
were verified separately in `body-areola-sides-mcp-proof.json`. The asymmetric
preset renders with left factor 1.6 and right 0.6. This adds appearance control;
left/right nipple projection and physiological response are still shared.

### Separate male source and rendered preview (2026-10-01)
A separately exported `GEO-body_male_realistic` source from the same verified
Blender CC0 bundle is now available, normalized to the library's 1.64 m reference
height. Export tools accept an optional `male` argument; default female behavior
is retained. `FemaleDemo::new_male` uses its body, eyes and skin-shell bindings;
`female_render --male` selects it. Runtime hair display/simulation is disabled
for this constructor. Local refinement produced 44,880 vertices and 89,756
triangles; every vertex has a binding to the male 636-node shell. Metal chest
render was inspected (`body-male-nipple-cold.png`); male asset test checks full
coverage, parameter acceptance, finite geometry and animation advancement.
This addresses the absence of a separate male mesh. Interactive/MCP selection,
male-specific rig/eye/face/region validation and physiological cold calibration
are still pending. The torso render is not proof of whole-body contact safety or
male tissue correctness. See the male asset README for source and reproduction.

### Body source selection through parameters/MCP (2026-10-01)
`body_model` is a validated `female|male` enum, defaulting to female for legacy
presets. The panel builds its selector from MCP schema. `model_get` reports the
selected source and snapshot rendering reads the saved selection. Partial CLI
presets patch the selected model, preserving `--male` when the preset omits the
selection. Runtime switching stages the selected mesh/shell/bindings, applies
parameters and commits only after success; camera/view controls and file watches
are retained, simulation state resets. Existing fluid preview blocks the switch
without mutation because cross-mesh liquid transfer is not implemented. Two
release tests cover source switching, complete bindings and view retention, plus
fluid preservation on rejection. `surface_film` example was rebuilt with this
path, but native desktop switching has not been visually demonstrated.
MCP initialize/schema/update/get/snapshot were exercised against a temporary
parameter file. Metadata reported `blender-male` and the saved source parameter
was retained in the returned snapshot receipt. The resulting offscreen PNG was
inspected (`body-male-mcp-snapshot.png`), with full protocol evidence saved in
`body-model-selection-mcp-proof.json`. This is an offscreen capture, not live
viewer presentation acknowledgement or anatomical correctness proof.

### Conservative film remapping foundation (2026-10-01)
`SurfaceFilm::remapped` constructs a new film with explicit donor-to-recipient
fractions; wet donors must be fully covered and every nonempty row must sum to
one. It preserves material, wetting model, volume and mass, seeds no new liquid,
and never modifies the source. It validates indices, finite/nonnegative weights,
mesh geometry and final volume conservation. Refinement/merge tests preserve
uniform thickness; rejection tests cover missing donors, incorrect sums, bad
indices, NaN and negative weights. All 23 ordinary surface-film tests pass;
two expensive existing studies remain ignored. This is a transfer primitive,
not a physical correspondence detector or an enabled body-source switch.
The refinement exporter now writes parent-cell provenance; correspondence-only
mode does not replace the adopted mesh. Both runtime topologies match generated
maps; child area sums reproduce every source triangle to about 1e-14 relative.
The male/female base triangle topology hashes differ, so direct cell-index
mapping is invalid. `body-parent-correspondence-proof.json` records the evidence.
Runtime switching with fluid remains guarded pending verified correspondence
and overlap weights, source/state reattachment and conservation verification.

### Shared-quad fluid correspondence (2026-10-01)
Further inspection resolved the apparent topology mismatch: all 42,340 unordered
quad vertex keys and their cyclic boundary edges match, while face ordering and
triangulation diagonals differ. `body_film_correspondence.py` constructs canonical
unit-square charts for these quads, maps refined child vertices by parent
barycentric coordinates and clips donor/recipient triangles to compute overlap
fractions. It validates mesh/source hashes, parent containment, matching quad
boundaries and full chart coverage. It does not use closest-point matching or
claim physiological calibration. Generated directional maps have 116,794 and
116,542 entries; raw row-sum errors are <=2.60e-13 and normalized rows satisfy
the library's 1e-12 tolerance. Two Python overlap tests cover opposite diagonals,
refinement, reversed winding, boundary contact and disjoint triangles.
The actual 90,402/89,756-cell Rust remap test passed in both directions, with
relative mass errors 3.14e-15 and 1.002e-14 and finite nonnegative thickness.
This validates conservative transfer on the prepared complete surfaces. The
interactive film mesh also has mouth-surface edits; its cell correspondence,
source/counter reattachment and atomic runtime model switch remain to integrate.
Fluid-enabled source switching therefore remains guarded. Evidence is in
`body-film-correspondence-proof.json` and the directional asset maps.

### Runtime source switch with conserved fluid (2026-10-01)
`FilmPreview::remapped_body` now matches canonical runtime cells to prepared
reference triangles, applies directional shared-quad overlap maps and rebuilds
recipient solver/render bindings and source weights. It retains material/optics,
wetting configuration, initial/source-added mass and contact-transfer counters.
Dry unsupported cells need no correspondence; any wet unsupported donor or
recipient (including changed mouth seals) rejects the staged switch without
modifying the old model. `set_body_parameters` commits the replacement only
after successful fluid transfer. Simulation pose resets as before.
Two runtime model-selection tests pass: bidirectional fluid transfer retains
mass/counters and sources continue supplying the configured amount; view/camera
settings and full tissue bindings remain intact. Unsupported wet-surface rejection
passes separately. All 10 ordinary film-preview tests pass (3 costly existing
studies ignored). The offscreen Metal demonstration starts with a female film,
switches to male and displays transferred thickness (`body-fluid-runtime-switch.png`).
Its measured before/after mass is 5e-5 kg with relative error 1.36e-16; source
counters are identical. This is a remeshing operation, not simulated contact
between two bodies, a live native-window proof or physiological calibration.
Use `female_render --film --switch-body-model male --switch-proof PATH` to
reproduce. Full original anatomical/physical/material plan remains incomplete.

### First-frame fluid geometry after source and proportion switch (2026-10-01)
A staged source replacement now updates film cell areas/curvature from the
replacement's rendered substrate after applying body parameters and retaining
view controls, before committing the model. This prevents first-frame thickness
from using the unscaled canonical recipient surface. `update_substrate` is an
explicit geometry-only operation: it adds no source fluid and advances no time,
validates bound vertex availability and delegates atomic geometric validation
to the solver. Film advancement reuses that method. Release regression switches
to male at height 190 cm and torso depth 1.2, checks mass retention and confirms
first-frame maximum thickness is unchanged by a subsequent geometry-only refresh
from the same displayed mesh. Ordinary same-source proportion updates still
need an equivalent immediate staged film update; this does not establish full
morph/time convergence or complete the original plan.

### Immediate film refresh for same-source morphs (2026-10-01)
Parameter edits with an existing film now stage a rebuilt body and identity
cell transfer through canonical triangle keys, then update film geometry from
the displayed morphed substrate before commit. Same-source edits retain pose
and animation time, timestep remainder, step count and hair toggles; source
settings and mass counters persist. Returning to source defaults forces a rebase
at the retained pose time rather than keeping constructor-time attachments.
Appearance-only areola radius/pigmentation/side edits update parameters directly,
without changing live shell positions or film state. Two release regressions
cover a live-film morph at 190 cm/torso-depth 1.2, first-frame thickness, unchanged
mass/time, reset to defaults, and unchanged physics for appearance edits. The
model-selection regressions also pass. This supersedes the earlier note that
same-source film geometry waits for the next animation step. Staging resets
physical tissue velocities as the existing morph rebase did; preserving physical
momentum through arbitrary morphs and broader stability validation remain open.

### Invalid preview timesteps reject before geometry mutation (2026-10-01)
`FilmPreview::advance` now validates its finite (0,0.1] second timestep before
updating substrate geometry or adding source liquid. Previously an invalid
step could change cell areas/thickness before the solver rejected the timestep.
The regression tries zero, negative, oversized, NaN and infinite steps against
a different-area substrate, plus missing bound vertices, and requires unchanged
measurements, mass and source accounting. This covers input rejection; it does
not make all later numerical/contact failures transactional across an entire
frame or establish arbitrary-step physical stability.

### Atomic geometry/source/transport film frame (2026-10-01)
`SurfaceFilm::advance_on_geometry` stages the new substrate, existing volumes,
wetting model, source input and transport solve. It commits only after success;
input or numerical failure preserves original geometry, mass and thickness.
The existing self-contact BVH is moved/refitted after success, avoiding rebuilding
it for each staged frame. `FilmPreview::advance` uses this operation and updates
source mass accounting only after commit. The overflow regression adds a finite
but extreme source, forcing solver failure after source addition; old geometry
and volume survive. The successful path is compared bitwise with explicit prior
geometry/source/step operations. All 11 ordinary preview tests pass (3 costly
studies ignored). Self-contact exchange remains a separately reported, atomic
operation after successful transport; its failure is recorded, not a rollback
of the completed transport step. Frame staging adds topology/volume copies;
full-body performance needs measurement before claiming unchanged frame speed.

### Atomic film frame cost on prepared body (2026-10-01)

Paired release benchmark on 45,203 vertices / 90,402 cells: 2 warmup rounds, 10 measured rounds, alternating execution order. Upper median geometry/source/transport cost was 26.41825 ms for the sequential path and 26.785167 ms for the staged atomic path (about 1.39% higher). Every round produced bitwise identical thickness and total mass. Individual samples overlap; this run does not establish a statistically significant overhead.

Evidence: `body-atomic-frame-profile.json`; reproduce with `VOXY_ATOMIC_FRAME_REPORT=docs/body-atomic-frame-profile.json cargo test --release -p voxy_app --lib actual_body_atomic_frame_profile -- --ignored --nocapture`. The fixture disables gravity, surface tension and wetting and excludes self-contact, tissue physics and rendering. These timings are not complete viewer frame times or an FPS claim.

### Adopted body geometry audit (2026-10-01)

`tools/audit_runtime_body_geometry.py` checks the exact female and male refined OBJ assets included by the demo, without rewriting normals, bindings or fluid correspondence. Both have zero boundary/nonmanifold/winding errors, duplicate triangles and degenerate triangles. The static nonadjacent-triangle test nevertheless detects 484 female and 245 male intersecting pairs. Neither traversal reached its 10,000-pair limit. Female pairs: head 256, feet 220, hands 8; male pairs: head 23, feet 68, hands 154. Regions are coordinate heuristics.

Evidence with source SHA-256 and exact triangle pairs: `body-runtime-geometry-audit.json`. This audits adopted source geometry before runtime mouth-seal removal, morphing and deformation. Pairs sharing a vertex are excluded; this is not full adjacent-fold checking, a posed runtime audit or CCD. Geometry preparation remains incomplete. Repairs must retain or regenerate authored-normal transport, tissue bindings and conservative film correspondence.

### Bounded male repair candidate (2026-10-01)

A separate candidate `assets/characters/blender-male/body-repair-candidate.obj` reduced the static nonadjacent intersection count from 245 to 156 with maximum vertex movement 0.824494 mm. Five accepted steps retained triangle indices, closed manifold topology, nondegenerate faces and original face orientation. Reloading the exported OBJ reproduced the exact remaining pair list. Five preparation-tool unit tests passed. The descent stopped when every tried step increased the crossing count.

The candidate is not adopted: 156 intersections remain, authored normals require deformation transport, and tissue bindings / fluid correspondence must be revalidated against changed coordinates. Evidence: `body-male-bounded-repair-proof.json`.

### Male candidate diagonal repair (2026-10-01)

Generalized `tools/repair_body_diagonals.py` to explicit source/output/report arguments, positive validated limits, refusal to overwrite an existing candidate, SHA-256 provenance and exact export-reload verification. On the bounded male candidate, 66 accepted local edge flips reduced static nonadjacent intersections from 156 to 70 without moving any vertex. The incremental intersection set matched a complete BVH audit; closed manifold topology, consistent winding and nondegenerate triangles were preserved.

Candidate: `assets/characters/blender-male/body-diagonal-repair-candidate.obj`; proof: `body-male-diagonal-repair-proof.json`. This geometry-only export is not adopted. Changing triangle connectivity invalidates existing triangle-based binding/correspondence assumptions, and authored normal/UV data must be reconstructed before runtime use. Seventy detected crossing pairs remain.

### Male residual repair search (2026-10-01)

A new bounded displacement pass after triangulation (24-step budget, 1 smoothing step, 3 mm cumulative bound relative to the original asset) accepted no move: all tried proposals failed to improve the crossing count/segment score. Expanding the edge-flip limit from 12 to 24 mm likewise accepted no flip; the output hash was unchanged. Seventy crossing pairs remain. Reports: `body-male-combined-repair-proof.json`, `body-male-wide-diagonal-repair-proof.json`. Residual pair centers, heuristic regions and segment lengths are recorded in `body-male-remaining-crossings.json` to guide local component repair rather than repeating the same global search. No new candidate was adopted.

### Local component repair tooling (2026-10-01)

Added `tools/repair_body_region.py`: explicit source/reference/output, region-selected crossing seeds, exactly two adjacency rings independent of face ordering, unchanged vertices outside that support, cumulative 3 mm bound, global intersection acceptance and export-reload verification. Six geometry-tool tests pass, including exact ring expansion. The right-hand trial restricted movement to 197 vertices but accepted no improving proposal; 70 global crossing pairs remain. A preceding foot trial likewise accepted no move. Reports: `body-male-right-hand-repair-proof.json`, `body-male-left-foot-repair-proof.json`. No repair was adopted. The existing averaged face-normal displacement field is insufficient for these residual configurations; subsequent repair requires a different contact separation direction or local remeshing, followed by full binding/normal/film validation.

### Penetrating-vertex separation trial (2026-10-01)

Added an optional `penetrating_vertices` separation mode to the preparation solver. It applies the signed clearance correction per penetrating vertex instead of translating all vertices of an intersecting face by the deepest penetration. Existing default behavior remains unchanged. A known crossing fixture separates within the prescribed bound and retains a nonpenetrating vertex unchanged; seven geometry tests pass.

On the actual residual male candidate, one-ring displacement smoothing and a 3 mm cumulative bound accepted no proposal, leaving 70 pairs. Export reload/full BVH audit confirms the unchanged count. Evidence: `body-male-vertex-projection-repair-proof.json`. The synthetic success is not evidence of repaired anatomy. The candidate remains unadopted.

### Single-vertex residual descent (2026-10-01)

Added `tools/repair_body_vertex_descent.py`: incident-face spatial buckets, individual vertex proposals along opposing-face normals and coordinate axes, global crossing-count descent, cumulative 3 mm displacement cap and original candidate face-orientation constraints. Every accepted move strictly decreases the crossing count. At completion the incremental pair set matched a full BVH traversal, and the geometry-only export reloaded exactly.

On the male candidate this reduced crossings from 70 to 21. Maximum movement relative to the original adopted asset is 1.995744 mm. The final full topology audit reports no degenerate/duplicate triangles, boundaries, nonmanifold edges or inconsistent winding. Eight preparation tests pass, including a known crossing removed by exactly one moved vertex. Evidence: `body-male-vertex-descent-proof.json`; candidate: `assets/characters/blender-male/body-vertex-descent-candidate.obj`. This is not yet adopted: 21 static nonadjacent crossing pairs remain; authored normals, tissue bindings and film correspondence still require regeneration and visual/runtime validation.

### Male static nonadjacent crossings eliminated in candidate (2026-10-01)

After an unchanged post-descent diagonal trial, a finer single-vertex search using signed 0.025 / 0.05 / 0.1 / 0.3 / 1.5 / 2 mm proposals reduced the remaining 21 nonadjacent crossing pairs to zero. Maximum cumulative movement relative to the adopted original asset is 2.404920 mm. The incremental intersection set matched a complete BVH traversal; the OBJ exported and reloaded exactly. All topology gates remain clear. Eight preparation tests pass, including proposal-distance validation.

Candidate: `assets/characters/blender-male/body-fine-vertex-descent-candidate.obj`; proof: `body-male-fine-vertex-descent-proof.json`. Not adopted yet. Zero here applies only to the static nonadjacent-face predicate: pairs sharing any vertex are excluded. Adjacent foldovers, anatomical shape, runtime mouth edits, morph/pose deformation and continuous collision still require checking. Authored normal/UV data, skin bindings and male/female conservative film correspondence must be reconstructed for this triangulation before adoption and visual validation. The female mesh is not repaired by this result.

### Male candidate adjacent-face audit (2026-10-01)

Added `tools/audit_body_adjacent_faces.py` to inspect the previously excluded shared-vertex/edge triangle pairs. Distinct nonparallel planes sharing an edge can only intersect on that edge; coplanar pairs are polygon-clipped and checked for positive overlap area. Shared-vertex nonparallel pairs are checked for positive intersection segment length beyond their shared point. Relative tolerance is 1e-10 against local edge scale. The final male candidate has zero forbidden pairs among 546,913 adjacent pairs. Nine preparation tests pass, including legitimate shared contacts, coplanar foldovers and shared-vertex noncoplanar crossings.

Evidence: `body-male-adjacent-face-audit.json` with source SHA-256. Combined with the zero nonadjacent pair result, this provides stronger static geometry evidence. It is still a floating-point test, excludes isolated extra point contacts and does not certify anatomical shape, runtime morphing, animation or CCD. Candidate adoption remains pending normal/binding/film reconstruction and rendered validation.

### Repaired candidate authored-normal reconstruction (2026-10-01)

Added `tools/restore_repaired_body_normals.py`: reference/candidate frames use repaired connectivity with stable vertex identities, inverse-transpose surface-frame transport and reference corner-angle accumulation. Generated `assets/characters/blender-male/body-repaired-render-candidate.obj` with 44,880 normals, preserving all repaired positions and triangles exactly. Rest self-transport preserved normalized authored normals within 5.14e-16 component error. Maximum normal change is 36.9343 degrees and requires visual review around repaired hands/feet. Three independent tests cover affine shear, authored smoothing at rest and degenerate-frame rejection.

Evidence: `body-male-repaired-normals-proof.json`. The output remains a candidate. No new UV mapping was synthesized (reference OBJ has normals and no UV stream). Tissue bindings, fluid correspondence, rendered shape and posed collision checks remain pending before adoption.

### Repaired male surface bindings (2026-10-01)

Added `tools/rebind_repaired_body.py` with vertex-identity verification against original f32 binding coordinates. The repaired candidate retains 43,366 original bindings and reprojects 1,514 moved vertices onto the existing 636-node / 1,268-triangle skin shell. Maximum reprojected distance is 5.929018 mm. Binding weights are finite, nonnegative and sum to one; shell topology/positions/pins/directions and per-binding fade remain unchanged. Three projection tests cover triangle interior, edge/vertex clamping and nearest-face selection.

Output: `assets/characters/blender-male/body-repaired-skin-candidate.json`; hash/projection proof in the adjacent `.audit.json`. This establishes geometric bindings, not acceptable physical error: millimetre-scale separation from a coarse shell and posed contact still need validation. Candidate remains unadopted; conservative male/female film maps and rendered/runtime validation remain pending.

### Repaired male candidate native-renderer preview (2026-10-01)

`female_render` accepts paired `--body-obj` / `--skin-bindings` with `--male` through the shared male asset constructor, retaining stock asset defaults. Source switching is rejected for custom candidates because correspondence has not been rebuilt. Added `--feet` framing alongside existing `--hands`; framing choices are mutually exclusive.

Release build passed. The repaired OBJ and candidate bindings loaded with all 44,880 vertices / 89,756 triangles and 44,880 bindings. Offscreen Metal snapshots of the hand and both feet were generated and visually inspected: no obvious open seams in these views, but anatomical detail/material realism remains limited. Feet framing was widened after the initial crop excluded toes. Final files: `body-male-repaired-hand.png`, `body-male-repaired-feet.png`; hashes and scope: `body-male-repaired-render-proof.json`. Surface diffusion was disabled for these comparisons. These are static offscreen views, not native monitor capture, a full anatomical review, full-body intersection checking after runtime mouth edits or posed/physical contact proof.

### Repaired male conservative material correspondence (2026-10-01)

Added `tools/body_repair_film_map.py`. The 110 changed triangle cells form 22 patches whose original/repaired boundaries agree. Harmonic disk charts use original boundary arc lengths and original positive graph weights; both triangulations are checked for consistent nondegenerate chart orientation. Polygon overlap yields bidirectional conservative distributions with maximum raw row-sum error 4.22e-15. Unchanged triangle cells map identically despite their geometry deformation.

Generated old-male/repaired-male maps and composed both female/repaired-male directions. Deterministic nonuniform per-cell volume tests on all four full maps verify finite nonnegative weights, valid recipient indices, unit donor coverage and relative total-volume error below 1e-12. Three unit tests cover opposite diagonal splitting, composition and boundary mismatch rejection. Evidence: `body-male-repair-film-map-proof.json`, `body-male-repair-film-mass-proof.json`. These maps encode material-space redistribution, not nearest physical surface projection or physiological calibration. Runtime loaders still require explicit integration and actual `SurfaceFilm` tests before candidate adoption.

### Repaired correspondence in solver and preview (2026-10-01)

Extracted `FilmPreview::remapped_reference` for explicit verified source/target mesh references and maps; stock `remapped_body` delegates to it, preserving existing model-switch behavior. Full-mesh tests exercise all four repaired correspondence directions in `SurfaceFilm` with nonuniform deposited volume. Relative mass error is at most 1.04e-14 after remapping and 1.18e-14 after a 1 microsecond gravity step; thickness remains finite/nonnegative and donors unchanged. These runs disable capillarity/wetting and are not physiological stability or normal-frame-rate evidence.

A preview transfer test advances an active source, remaps original male to repaired male, checks initial/source mass counters and settings exactly, then resumes source input on the repaired geometry. All 13 normal surface-film preview tests pass; four expensive studies are explicitly ignored. Candidate adoption, user-facing switch integration, mouth-edit coverage and posed collision validation remain pending.

### Actual repaired render pose collision audit (2026-10-01)

An ignored app test exports the actual body portion of `FemaleDemo::mesh()` at 0 / 1.75 / 3 seconds, using the repaired OBJ and rebuilt skin bindings, animation-only mode and runtime mouth-seal removal. All three distinct f32 render meshes contain 44,880 vertices / 89,592 triangles. Complete static nonadjacent and adjacent positive-area/length audits find zero forbidden pairs in each sampled pose. There are 168 boundary edges after the intentional runtime mouth opening; this is not a closed-surface proof. Report: `body-male-repaired-pose-audit.json`.

The initial adjacent predicate reported two spurious shared-point segments of 2.61e-12 / 4.57e-13 m in the rest chest, caused by interval cancellation on nearly parallel planes. Anchoring both plane intervals at the exact shared vertex removed cancellation without widening tolerances. A recorded real-coordinate regression checks all nine cyclic triangle orderings; ten geometry tests pass. Full pose audits were rerun after this fix.

Reproduce export: `cargo test --release -p voxy_app --lib export_repaired_body_pose_surfaces -- --ignored --nocapture`; audit: `python3 tools/audit_body_pose_surfaces.py /tmp/voxy-repaired-body-poses/samples.json docs/body-male-repaired-pose-audit.json`. These three snapshots cover rig/face deformation only, with no XPBD dynamic advance, morph sweep or CCD. Candidate is not yet adopted.

### Repaired male sampled morph collision audit (2026-10-01)

Extended the ignored actual-surface exporter with `VOXY_BODY_MORPH_EXPORT=1`, preserving baseline-pose mode. It applies parameters through the normal `set_body_parameters` path (physical rebase/bindings), then exports the actual posed render mesh. Two configurations at animation time 1.75 s were checked: height 190 cm; and baseline height with weight 100 kg plus leg/arm length multipliers 1.2. Both have zero nonadjacent or forbidden adjacent intersection pairs and no degenerate triangles. Runtime mouth boundaries remain 168.

Evidence: `body-male-repaired-morph-audit.json`, including full parameter values and OBJ hashes. Export directory: `/tmp/voxy-repaired-body-morphs`. This is two sampled illustrative morph configurations, not exhaustive parameter-range certification, anthropometric calibration, tissue dynamics or CCD. Candidate adoption remains pending; the female mesh and other original plan requirements are still incomplete.

### Repaired male adopted by stock constructor (2026-10-01)

`FemaleDemo::new_male` now includes the repaired render mesh and rebuilt skin bindings. Stock `FilmPreview::remapped_body` uses the composed repaired-male/female maps in both directions. Original male assets remain intact for reproducibility. Historical candidate filenames/headers and preparation proofs marked `adopted:false` are superseded by this constructor change. Current runtime source audit finds zero male nonadjacent pairs; the female asset still has 484.

Release example and `model_mcp` build passed. In the 39-test female-demo suite, 35 passed, three were ignored, and one failed: `watched_file_updates_and_invalid_json_keeps_last_body` throws `surface contact conflicts with pins` after a female preset reload. The same failure reproduces independently. Model switching with fluid, first-frame morphed geometry and same-source film updates passed. This is an unresolved physics issue, not a passing broad verification gate.

The rebuilt offscreen example switched from female to stock repaired male with existing film, saved `body-repaired-runtime-switch-proof.json` and `body-repaired-runtime-switch.png`, and the rendered thickness view was inspected. Mass changed from 5e-5 to 5.000000000000001e-5 kg (rounding only). Source settings/counters were retained. This is offscreen runtime proof; running desktop instances are not asserted updated. The full original plan remains incomplete, including female repair, dynamic contact/pin reconciliation, internals, calibrated physiology and advanced optics.

### Parameter reload contact/pin conflict fixed (2026-10-01)

The reproduced female preset-reload error came from regional torso sphere scaling: the maximum transformed axis stretch created a circumscribed sphere when height increased at fixed mass, even though the body narrowed. This proxy reached pinned skin anchors. Regional colliders now use the minimum transformed axial radius, retaining an inscribed spherical approximation instead of inflating it through anchors. Contact/pin conflict checks remain active. An ellipsoid/nonlinear anatomical collider would be more faithful and remains outside this fix.

The previously failing watched-preset test now passes, including rejection of invalid subsequent JSON without discarding the valid body. A regression checks nine height/weight combinations (130/182/210 cm, 35/60/180 kg, breast multiplier 1.3): pinned positions lie outside sphere plus particle radius and each region advances. All three regional-volume tests pass. The broader female-demo suite was rerun: 36 passed, zero failed, three ignored. This supersedes the failure recorded in the male-adoption section, but does not certify all parameter combinations, anatomical contact or full-plan completion.

### Female adopted-surface diagonal candidate (2026-10-01)

Applying the validated local diagonal search directly to the currently adopted female nipple-refined asset accepted 195 flips and reduced nonadjacent intersections from 484 to 205 without changing vertex positions. The incremental pair set matched the complete BVH audit. Export reload matched exactly; no degeneracy, duplicate faces, boundary/nonmanifold or winding errors were introduced. Candidate: `assets/characters/blender-female/prepared/body-new-diagonal-repair-candidate.obj`; proof: `body-female-new-diagonal-repair-proof.json`. This geometry-only candidate is not adopted, and all previous normal/binding/film data remain associated with the original mesh.

Generalized `repair_body_vertex_descent.py` to explicit source/reference/output/report arguments, cumulative displacement bounds, pass count and optional fine proposal distances so the successful male method can be applied reproducibly to female residuals. Ten geometry tests pass. Remaining work includes 205 crossings, adjacent/posed tests, normal/binding/film reconstruction and rendered validation; none is implied complete by the diagonal reduction.

### Female adjacent-face baseline and active residual search (2026-10-01)

The female diagonal candidate has 24 forbidden adjacent intersections among 553,389 shared-vertex/edge pairs, in addition to its 205 nonadjacent pairs. Report: `body-female-diagonal-adjacent-audit.json`, source SHA-256 included. A global nonadjacent count alone cannot certify this candidate. The single-vertex search on this candidate has been launched with four passes and a cumulative 3 mm bound relative to the adopted nipple-refined source. Its result is pending and must be checked against both predicates before adoption.

### Joint adjacent/nonadjacent vertex descent (2026-10-01)

The single-vertex repair tool now supports `--adjacent`. Its initial objective includes both nonadjacent intersections and positive-area/length intersections beyond shared topology. Candidate checks include all incident shared contacts, and the final incremental pair set must equal the union of full nonadjacent/adjacent audits. A known coplanar shared-edge overlap is invisible to legacy mode but repaired by the joint mode; eleven preparation tests pass.

The already-running female search was started before this option existed and still uses nonadjacent-only mode. It has not been restarted or assumed terminal. Its result must first be retrieved and audited, then used as a starting point for joint repair. No new geometry result or adoption is claimed by this tool update.

### Female adjacent residual localization (2026-10-01)

While the confirmed live nonadjacent vertex search continues, the 24 baseline adjacent overlaps were localized: left foot 8, right foot 10, head 4, left hand 2. `body-female-adjacent-regions.json` records exact faces, shared vertex ids and centers. Coordinate region labels are heuristic. These counts describe the diagonal candidate before the running vertex search; they must not be treated as counts for its unfinished output.

### Vertex-search broadphase filter (2026-10-01)

Added per-triangle bounds to the candidate vertex search so bucket neighbors whose bounds do not overlap are discarded before geometric predicates. Accepted moves update bounds together with triangle coordinates; the 1e-12 broadphase tolerance matches the full BVH audit. All eleven preparation tests pass. No whole-body speedup is claimed without a paired benchmark. The confirmed live female process loaded its code before this change, so this optimization applies only to subsequent searches; it was not restarted.

### Joint search full-mesh validation (2026-10-01)

Ran the new `--adjacent` vertex-search mode, including the updated per-triangle bounds filter, on the entire adopted repaired male geometry with its original reference and cumulative 3 mm limit. Initial/final union of adjacent and nonadjacent forbidden pairs is zero, no proposal was accepted and no vertex changed. Incremental/full verification and export reload passed on all 89,756 triangles. Proof: `body-male-joint-search-validation.json`; geometry-only temporary output is not an adopted asset and does not replace authored normals. The female search remains pending on its original live process.

### Female vertex descent completed (2026-10-01)

The original confirmed live search completed normally: nonadjacent pairs reduced from 205 to 20 with maximum displacement 1.378125 mm relative to the adopted female mesh. Incremental/full intersection verification and exact export reload passed; all topology gates remain clear. Proof: `body-female-vertex-descent-repair-proof.json`. A complete adjacent audit on its output finds 10 forbidden adjacent overlaps among 553,389 pairs, down from 24 on the preceding diagonal candidate. Proof: `body-female-vertex-descent-adjacent-audit.json`.

Started a new joint fine search from this verified output, considering both 20 nonadjacent and 10 adjacent pairs, eight passes, 3 mm cumulative bound and fine distances. This is a dependent next repair pass, not a restart of the completed original process. Candidate remains unadopted; no zero-intersection claim is made while the new pass is pending.

### Female joint fine descent completed (2026-10-01)

The subsequent joint fine search completed: union count fell from 30 to 24 (20 nonadjacent, 4 adjacent). Maximum cumulative movement is 2.237442 mm. Full union audit matches the incrementally maintained set; topology and export reload checks pass. Proof: `body-female-joint-fine-repair-proof.json`. Twenty-four forbidden pairs remain, so the candidate is not ready for normal/binding/map regeneration and adoption. Further local separation/remeshing is required; neither the female mesh nor the full original plan is complete.

### Female grouped-face residual repair (2026-10-01)

Localized all 24 preceding residual pairs to the feet: nonadjacent left 6/right 14 and adjacent left 2/right 2. Added optional `--face-groups` to the bounded joint vertex solver. A proposal translates all three vertices of one intersecting face together while checking every incident triangle, cumulative bounds, original face orientation and the union of both intersection predicates. Twelve preparation tests pass, including a known crossing eliminated by a common three-vertex translation.

The actual female grouped search reduced the union from 24 to 18 (16 nonadjacent, 2 adjacent) with maximum cumulative movement 2.946771 mm. Incremental/full audit, topology and exact export reload pass. Proof: `body-female-face-group-repair-proof.json`; candidate: `body-face-group-repair-candidate.obj`. This candidate is still not adopted. Remaining foot intersections require further repair; the existing 3 mm bound is nearly reached at some vertices.

### Female residual fine/diagonal alternation (2026-10-01)

A bounded fine single-vertex pass after grouped moves reduced the union from 18 to 14, retaining the 2.946771 mm cumulative maximum and clear topology. Proof: `body-female-post-group-fine-proof.json`. Extended `repair_body_diagonals.py` with `--adjacent`: both its local acceptance and full final verification now use the union of shared-contact and nonadjacent predicates. A closed tetrahedron regression remains unchanged; all thirteen preparation tests pass.

The subsequent joint diagonal pass accepted one flip and reduced the union from 14 to 13 (11 nonadjacent, 2 adjacent), without moving vertices. Incremental/full union and exact reload checks pass. Proof: `body-female-joint-diagonal-proof.json`. Candidate remains unadopted with thirteen forbidden pairs. Bounds, anatomical shape, binding/normal/film reconstruction and posed checks remain required.

### Female edge-group separation trial (2026-10-01)

Added `--edge-groups` to the joint separation tool: move two vertices of an incident edge with a common translation, validating all incident triangles and rejecting mixed group modes. Fourteen preparation tests pass, including a known crossing removed by exactly two translated vertices. The actual residual female mesh accepted no move within the 3 mm cumulative bound; all 13 pairs remain and the geometry hash is unchanged. Full union audit, topology and reload verification pass. Proof: `body-female-edge-group-proof.json`. Candidate remains unadopted. Further progress requires additional local degrees of freedom (e.g. conforming subdivision), rather than repeating the same bounded vertex/edge/face proposal set.

### Female residual conforming subdivision and bounded repair (2026-10-01)

Added paired `tools/refine_body_residuals.py`: exact candidate/report SHA-256 match, shared edge-midpoint/face-center refinement, identical reference connectivity, explicit old-vertex weights and parent cells, complete topology gates, area preservation and export reload. Refining 31 residual edges added 63 vertices / 126 triangles, giving 45,266 vertices / 90,528 triangles. The geometry remained piecewise unchanged, with 2.946771 mm cumulative displacement unchanged. Parent/reference correspondence is saved beside the candidate; public proof: `body-female-residual-refinement-proof.json`.

The refined initial union contains 56 triangle-pair intersections because subdivision splits existing intersecting triangles. This count is not directly comparable to 13 on the coarser mesh and does not establish geometric regression. On that same refined mesh, bounded joint vertex descent reduced the union from 56 to 48 (46 nonadjacent, 2 adjacent), retaining the 3 mm bound and topology. Proof: `body-female-refined-joint-repair-proof.json`. No zero-intersection claim or adoption: additional foot repair, shape review and all normal/binding/film reconstruction are still required.

### Refined female edge/projection trials (2026-10-01)

Joint edge-group descent on the refined mesh reduced the forbidden-pair union from 48 to 46. Full union, topology and export-reload checks pass, with maximum cumulative movement still 2.946771 mm. Proof: `body-female-refined-edge-repair-proof.json`.

Added optional `--project-bound`: proposals beyond the existing displacement ball are projected to its boundary with a tiny inward rounding margin instead of immediately rejected. It preserves the bound; group proposals may cease to be rigid translations after per-vertex projection. A controlled fixture verifies that projected search removes a crossing with a 0.15 mm cap while unprojected larger proposals are rejected. All fifteen preparation tests pass.

The actual refined female projected trial accepted no move and left 46 pairs (44 nonadjacent, 2 adjacent), with unchanged geometry hash. Proof: `body-female-projected-joint-proof.json`. This is not a failed external dependency or a reason to declare the goal blocked; another geometric search objective/local repair can still be developed. Candidate is not adopted and the full original scope remains incomplete.

### Residual intersection extent descent (2026-10-01)

Added optional `--reduce-measure` to accept equal-pair-count steps when the existing local intersection measure strictly decreases. In this mode proposals cannot introduce new pair identities. The measure sums noncoplanar segment lengths and square roots of coplanar overlap areas (both expressed in metres); it is a search objective, not tissue penetration depth or physiological calibration. Counts remain authoritative for completion, and final full union checking remains enabled.

Sixteen preparation tests pass, including a plateau fixture whose pair count remains one while its geometric extent shrinks, without concealing that pair. On the refined female mesh, eight passes kept 46 forbidden pairs but reduced the measure from 0.0159295285 to 0.0149228850 m (6.32%). Maximum cumulative movement remains 2.946771 mm; topology, full union verification and reload pass. Proof: `body-female-measure-descent-proof.json`. Candidate is unadopted, and this reduction does not establish repaired geometry.

### Largest-first bounded intersection descent (2026-10-01)

The completed twelve-pass largest-first search retained 46 forbidden pairs, while decreasing the same refined-mesh intersection extent objective from 0.014922885029322493 to 0.012302284578333893 m (17.56%). Maximum cumulative displacement is 2.993754 mm, within the 3 mm bound. Full union audit, closed topology, nondegeneracy and exact export reload pass. Proof: `body-female-large-measure-descent-proof.json`. This candidate remains unadopted; a smaller intersection extent does not satisfy the zero-intersection gate.

### Expanded bounded separation directions (2026-10-01)

Added opt-in `--diagonal-directions` to the local vertex/group search. Ten additional normalized lattice directions complement XYZ and incident opposing face normals; signed proposal distances provide both senses. The option does not alter the displacement bound, orientation/nondegeneracy gates, no-new-pair requirement in measure mode, or final complete union verification. Seventeen preparation tests pass, including an obliquely rotated crossing removed with projected proposals inside a 0.15 mm bound. This control is not evidence that anatomical residual intersections have been removed.

The four-pass largest-first edge-group measure search remains running (session 26183, observed process 62491 consuming CPU). Its result is pending; no candidate adoption or zero-intersection claim is made. The new diagonal option has not yet been evaluated on the actual residual candidate.

### Largest-first edge-group result (2026-10-01)

The previously pending four-pass edge-group run completed. Forbidden pairs remain 46, while the same refined-mesh extent objective decreased from 0.012302284578333893 to 0.010638800345280908 m (13.52%). Maximum cumulative movement is 2.946771 mm. Complete union verification, closed topology, nondegeneracy and exact exported reload pass. Proof: `body-female-large-edge-measure-proof.json`. The next diagonal-direction search was started on this output; its result is pending. No female replacement is adopted.

### Residual geometry diagnostic (2026-10-01)

Added `tools/plot_body_residuals.py` with explicit candidate SHA matching, uncapped report requirement and exact adjacent/nonadjacent union count checking. Rendered and visually inspected `body-female-current-residuals.png`: the remaining 46 pairs involve 45 triangles in two local clusters of one foot, near y=-0.80 m. The red outlines show involved triangles, not calculated intersection segments or penetration depths. Both projections use millimetres. This is a static diagnostic of the unadopted edge-group candidate and not a native render, dynamic contact validation or anatomy-completion proof.

### Actual diagonal-direction descent (2026-10-01)

The four-pass diagonal-direction trial completed with all 46 forbidden pairs still present. The measure decreased from 0.010638800345280908 to 0.010040476698689458 m (5.62%); maximum cumulative displacement is 2.999999999997 mm. Full union verification, topology and export reload pass. Proof: `body-female-diagonal-direction-proof.json`. This is an unadopted search candidate, not repaired geometry. Since pair-count plateau persists at the motion bound, the next trial changes local triangulation on this output rather than repeating the identical translation proposals.

### Post-direction refined topology repair (2026-10-01)

The six-pass joint diagonal run accepted ten flips without vertex motion and reduced the complete adjacent/nonadjacent forbidden-pair union from 46 to 35. Full union matches the maintained set; exported reload matches exactly. No boundary/nonmanifold/winding, duplicate or degenerate triangle errors were found. Proof: `body-female-refined-post-direction-diagonal-proof.json`. This output remains unadopted: 35 pairs still violate the preparation gate, and normals, bindings and film maps require reconstruction after any adoption.

Added opt-in `--reduce-measure` to diagonal repair. Equal-count flips require a strict decrease of the same intersection extent objective; this mode also rejects new pair identities, including for count-reducing proposals. The mode retains full union and topology gates and explicitly records before/after measures. All seventeen preparation tests pass, including untouched closed-tetrahedron checks with zero measure. A six-pass actual candidate trial is pending (session 61410); no result is inferred from the control test.

### Diagonal measure trial result (2026-10-01)

The pending trial completed: thirteen accepted flips reduced the complete union from 35 to 30 and extent from 0.010017000642306595 to 0.00973005987320895 m. No vertices moved; topology, full union and exact reload gates pass. Proof: `body-female-diagonal-measure-proof.json`. A bounded projected vertex trial with diagonal directions is now running on that topology. Thirty forbidden pairs remain; original female runtime geometry remains unchanged.

### Post-topology vertex result and MCP verification (2026-10-01)

The completed four-pass bounded diagonal-direction vertex search retained 30 forbidden pairs and decreased extent from 0.00973005987320895 to 0.009030399097739675 m (7.19%). Maximum displacement remains below 3 mm; closed topology, full union and exact reload pass. Proof: `body-female-post-diagonal-vertex-proof.json`. A new measure-guided diagonal trial is pending on this output. The previous topology candidate was plotted and inspected in `body-female-thirty-residuals.png`; the plot tool now validates either vertex or diagonal report provenance.

Current-source `cargo test --release -p voxy_app --bin model_mcp` passes the partial-update persistence/invalid-update atomicity/protocol test. This does not verify live viewer acknowledgement, material control, or snapshots of live liquid. Source inspection confirms `model_snapshot` creates a separate offscreen renderer from saved parameters and optionally a fresh demo film deposit, with explicit `liveViewerCapture=false` and `liveLiquidCapture=false`. Live-state snapshot delivery remains an original-plan gap.

The follow-up measure-guided diagonal trial completed with seven flips, reducing the forbidden-pair union from 30 to 25 and extent to 0.008515615411216336 m. No vertices moved; full union, closed topology and reload pass. Proof: `body-female-post-vertex-diagonal-proof.json`. Twenty-five pairs remain; no adoption or complete-plan claim is justified.

### Presentation receipt validation (2026-10-01)

`model_presentation::record` now rejects non-object parameters/measurements, nonfinite or negative simulation time, and zero frame dimensions before touching the existing receipt. Status additionally validates object measurements, positive viewer PID, finite nonnegative simulation time and exactly two positive u32 dimensions. Malformed receipts yield unknown application status rather than a valid frame acknowledgement. The release regression test passes and verifies valid/mismatched/stale receipt behavior plus invalid-write preservation and malformed receipt rejection. This validates receipt data, not monitor scanout or live-liquid screenshot delivery.

The bounded four-pass face-group search with diagonal directions is running on the 25-pair topology candidate (session 84238); no completed result or adoption is claimed.

### Face-group result and neighbourhood search (2026-10-01)

The four-pass face-group diagonal-direction trial completed with 25 pairs unchanged, reducing extent from 0.008515615411216336 to 0.005964476448802668 m (29.96%). Maximum cumulative movement remains below 3 mm. Complete union, topology and export reload pass. Proof: `body-female-post-topology-face-direction-proof.json`. The follow-up diagonal trial accepted two measure-reducing flips but also retained 25 pairs (extent 0.0059636397593398486 m). Proof: `body-female-post-face-diagonal-proof.json`. Neither candidate is adopted.

Added optional `--ring-groups`: each crossing vertex seeds a group containing all vertices of its incident triangles, deduplicated and tried largest-first. All faces incident to every moved vertex remain validated, including the outer neighbourhood. Ring/edge/face modes are mutually exclusive. Projection may change a group translation into per-vertex projected displacements, as in existing group modes. Eighteen preparation tests pass, including a crossing removed by a common four-vertex ring translation and mode-conflict rejection. The actual four-pass bounded ring trial is running; its result is pending and no anatomical validity is inferred from the fixture.

### Per-setting MCP frame acknowledgement (2026-10-01)

`model_status` now retains body `liveApplied` and adds separate `viewApplied`, `filmSettingsApplied`, `filmEnabled`, `savedView` and `savedFilm`. A fresh receipt's actual view and enabled film settings are compared against current sidecars; unsatisfied comparisons return false. Stale/missing receipts yield null, and disabled film yields `filmEnabled=false` with unknown settings application. `model_measurements` exposes the same acknowledgement fields. This avoids treating body application as proof that recently saved diagnostics or fluid properties have appeared.

Both release MCP tests pass, including fresh match, view/film mismatch with body still matching, disabled film, and stale status. These are receipt/sidecar tests rather than a live native demonstration. The ring-group repair process remains running (session 30552); no result is assumed.

The subsequent release binary build did not complete: concurrently changing `voxy_editor` panel interfaces currently mismatch (`Panels::build` argument count and missing Character/Collider/Physics action variants). The two earlier MCP tests passed against their compile-time tree; that evidence does not certify the later tree or an updated executable. No running MCP process is claimed updated.

### MCP executable protocol verification (2026-10-01)

The editor panel interfaces are now consistent in the current shared tree. `cargo build --release -p voxy_app --bin model_mcp` completed. Exercised the actual binary through initialize/tools-call JSON-RPC on an isolated temporary preset: both `model_status` and `model_measurements` explicitly return null for body/view/film application and film enablement without a live receipt. Proof: `body-mcp-ack-protocol-proof.json` includes executable SHA-256 and actual responses. This supersedes the previous binary-build failure, but proves no live viewer state or screenshot capture. The same ring-search session 30552 was polled and remains running; it was not restarted.

### Ring search result and input validity (2026-10-01)

The pending ring search completed with 25 pairs unchanged, reducing the extent objective from 0.0059636397593398486 to 0.0052408323641101855 m (12.12%). Maximum cumulative movement remains below 3 mm. Full union, closed topology and exact reload pass. Proof: `body-female-ring-direction-proof.json`. The follow-up measure-guided diagonal trial is running on this output; candidate remains unadopted.

The vertex/group search now rejects empty geometry, nonfinite or non-3D source/reference coordinates, empty triangle lists, out-of-range/noninteger/repeated triangle indices before any displacement-bound or intersection checks. This prevents NaN reference values from silently bypassing bounds. Nineteen preparation tests pass, including invalid source and reference data and malformed topology. These guards do not establish anatomical validity or dynamic contact correctness.

The post-ring diagonal trial completed with one extent-reducing flip but retained all 25 pairs; measure 0.005240831541091445 m. Full union, closed topology and reload pass. Proof: `body-female-post-ring-diagonal-proof.json`. The 3 mm bound is an engineering search choice, not a user-prescribed anatomical tolerance or calibrated physiological constraint. A separate 4 mm cumulative-bound vertex trial is now running, preserving original references and all other gates. Increased permissible movement cannot by itself prove acceptable shape; any adoption still requires independent geometry/normal/binding/film/posed/render review.

### Four-millimetre search and refined authored normals (2026-10-01)

The four-pass 4 mm bounded vertex search kept 25 pairs and reduced extent from 0.005240831541091445 to 0.0047080092552517355 m (10.17%). Actual cumulative movement reached 3.309762 mm. Full union, topology and exact reload pass. Proof: `body-female-four-mm-direction-proof.json`. The subsequent diagonal search accepted no flips and produced identical geometry. Proof: `body-female-four-mm-diagonal-proof.json`.

Extended normal reconstruction with paired authored-reference/refinement weights, exact reference provenance SHA checks, interpolation-position consistency, finite positive partition weights and normalized interpolation. Four normal tests pass. Applying repaired faces directly to undeformed positions fails the transport-frame degeneracy guard after topology changes; this is a real incompatibility, not a completed reconstruction. Added explicit `--transport-reference-topology` to transport via the valid original refined neighbourhood while exporting the candidate's actual faces unchanged.

That trial exported 45,266 authored normals with exact geometry preservation and rest error 3.59e-15, but maximum normal change is 173.97 degrees. Proof: `body-female-refined-normal-proof.json`. This severe change requires investigation/visual review and is not acceptable evidence of finished surface appearance. Candidate remains unadopted; twenty-five forbidden intersections remain.

### Normal transport across changed topology (2026-10-01)

Added optional `--vertex-frame`: independently build corner-angle-weighted source and target vertex normals, rotate authored normals by the shortest normal-frame rotation, and explicitly handle opposite directions. This avoids applying target faces to undeformed positions or deformed positions to removed faces. It preserves authored directional detail under that rotation, but does not recover tangential frame twist or validate anatomy. Geometry export remains unchanged. Reports now include opposition to current geometric smoothing and maximum disagreement angle. Six normal tests pass: changed planar diagonals, quarter-turn deformation, exact opposite orientation, interpolation provenance/weights, authored rest preservation and degeneracy.

The actual vertex-frame trial exports with exact geometry preservation and rest error 2.22e-16, but has 25 normals opposing current geometric smoothing, max disagreement 162.26 degrees and max authored change 146.29 degrees. Proof: `body-female-vertex-normal-proof.json`. Inspection of the untouched authored original nipple-refined mesh finds ten opposed normals, with max disagreement 159.35 degrees. Thus baseline authored normals also require investigation; an apparent numerical success is not appearance completion. No candidate adoption is justified. A four-pass 4 mm face-group geometric search is running (session 87675).

### Refined candidate binding reconstruction (2026-10-01)

Extended `rebind_repaired_body.py` to permit appended vertices only with an explicit source/refinement map: exact source SHA and original identity rows, valid finite positive partition weights and original binding identity are checked. Added vertices are closest-projected onto the existing physical shell; their attachment fade is interpolated from parent bindings. Existing unchanged bindings remain identical, and shell topology/positions are preserved. Four binding tests pass, including appended projection, preserved originals, explicit opt-in, inherited fade and invalid-fade rejection.

The actual four-mm vertex-normal candidate now has 45,266 bindings: 45,096 preserved and 170 reprojected, including all 63 added vertices. Maximum new shell distance is 4.259768 mm. Candidate sidecar: `body-four-mm-rebound-skin-candidate.json`, proof beside it as `.audit.json`. The trial retains known opposed normals and intersections and is not adopted. This projection does not build a volumetric anatomical tissue mesh.

The subsequent four-mm face-group geometric search completed with 25 pairs unchanged, extent 0.004468633536795292 m and max cumulative movement 3.315153 mm; full union, topology and reload pass. Proof: `body-female-four-mm-face-direction-proof.json`. The normal/binding outputs correspond to the preceding vertex candidate, not this newer face-group geometry, and must not be mismatched.

### Refined female candidate native-renderer review (2026-10-01)

Added `female_from_assets` and paired `female_render --body-obj / --skin-bindings` support for female candidates through the same constructor as defaults. Candidate source switching remains rejected without rebuilt film correspondence. Stock female asset paths remain unchanged. Release example build passed. The paired vertex-normal candidate loaded all 45,266 render bindings and rendered on Metal. Both reference and candidate full feet frames were inspected, but were too distant to resolve local defects.

Added explicit `--foot-detail` framing for the known positive-x toe residual region (reference-space center 0.175,-0.800,0.100 m). Reference and candidate renders were inspected at this closer scale. Triangular protrusions between toes are visible in both, with some more pronounced in the candidate. This contradicts acceptable final shape/appearance despite reduced static intersection counts. Review: `body-female-candidate-visual-review.json`; images: `body-female-reference-foot-detail.png` and `body-female-candidate-foot-detail.png`. Candidate remains unadopted. Corrective local anatomical/topological work is required; distance/count metrics alone do not certify the surface. CPU times from these single snapshots are not a comparative performance benchmark.

### Local fairing proposal search (2026-10-01)

Added opt-in `--fairing` to vertex/group descent: first propose the normalized average displacement toward each moved vertex's one-ring neighbour mean, then retain existing normal/axis/diagonal directions. Neighbourhood directions are translation invariant and vanish on a symmetric planar fan. The option changes proposals, not the acceptance objective: full local/global intersection gates and cumulative bound still apply. It does not guarantee smoother shape, preserve curvature/volume, remove protrusions or establish anatomical correctness. Twenty preparation tests pass. An eight-pass actual 4 mm bounded trial is running on the latest face-group candidate (session 38840); no result is assumed.

Localized the ten opposed normals on the untouched authored source: five are in feet, two in a hand and three near the head. This reinforces that authored baseline disagreement is not exclusively a new refinement defect. Geometric foot protrusions visible in close-up cannot be declared solved by only changing shading normals.

### Surface quality diagnosis and fairing result (2026-10-01)

Added `audit_body_surface_quality.py`: scale-invariant triangle quality 4*sqrt(3)*area/sum(edge-length squared), explicit region selection, adjacent-face normal angle diagnostics and source SHA. Three tests pass for equilateral triangles across scales, slivers/collapse, known 90-degree face angle and selection boundaries. The tool provides diagnostics rather than an anatomical pass/fail criterion.

For the identical reference-space foot box x=0.15..0.20, y=-0.82..-0.78, z=0.08..0.12 m, original geometry has 2 of 2337 selected triangles below quality 0.05, minimum 0.032951. The previously rendered four-mm candidate has 14 of 2463 below 0.05, minimum 0.002719. These differing triangle counts reflect local refinement; the lower minimum and visible protrusions justify rejecting this candidate's appearance, rather than using intersection-count reduction as a complete quality gate. Reports: `body-female-reference-foot-quality.json`, `body-female-candidate-foot-quality.json`.

The eight-pass fairing-direction trial completed with 25 pairs unchanged and extent reduced from 0.004468633536795292 to 0.004116979767698248 m (7.87%). Maximum displacement is 3.148485 mm. Full union, closed topology and export reload pass. Proof: `body-female-fairing-direction-proof.json`. No adoption is justified; the fairing proposals do not ensure final anatomical shape and local triangle-quality constraints need to accompany further repair.

The fairing output's foot quality audit finds 19 of 2463 triangles below 0.05 and minimum 0.001269 (`body-female-fairing-foot-quality.json`). Thus extent reduction again worsened the sliver diagnostic. This output is unsuitable for adoption; further search must constrain triangle quality as well as intersections. No smoother-surface claim is made.

### Triangle quality gate in geometric repair (2026-10-01)

Added optional `--quality-floor` in vertex/group descent. Each affected face must retain quality at least min(configured floor, its current quality), within 1e-12 comparison tolerance. Existing poor faces cannot materially worsen; good faces cannot fall materially below the floor. Accepted quality values are maintained incrementally and checked against a complete final recomputation and the initial per-face lower bounds; report includes minima, below-floor counts and explicit verification. This is a numerical shape-quality guard, not a physiological or anatomical calibration.

Twenty-one preparation tests pass. A controlled crossing fixture demonstrates the needed tradeoff: unguarded intersection removal produces a quality-0.029 sliver from a quality-0.086 face; the 0.05 guard rejects that outcome while continuing valid proposals. Invalid floor values are rejected. A four-pass guarded 4 mm search is running on the earlier vertex candidate, avoiding use of the more degraded fairing output (session 94613). Known baseline slivers and twenty-five pairs still prevent adoption; result pending.

### Quality-constrained result and sliver objective (2026-10-01)

The guarded four-pass trial completed: forbidden pairs remain 25, extent 0.0045412287794296706 m, max cumulative displacement 3.308100 mm. Across the whole mesh, below-0.05 faces decreased from 46 to 44; minimum quality remained 0.002719. Full quality recomputation, intersection union, topology and reload pass. Proof: `body-female-quality-guarded-proof.json`. These whole-mesh counts must not be compared directly to earlier foot-box counts.

Added opt-in `--repair-slivers` with mandatory positive quality floor, adjacent audit and no-new-pair measure mode. Poor faces join search targets even outside the residual intersection set. Equal-count steps may be accepted for a strictly lower sum of squared quality deficits below the floor, retaining each face's quality lower bound and all geometric/audit gates. Intersection extent may increase in this mode when quality improves, so it must not be described as monotone extent descent or anatomical repair completion. History explicitly records before/after quality deficits.

Twenty-two preparation tests pass. A nonintersecting sliver is improved above 0.05 without adding intersections; ordinary crossing-only mode leaves it unchanged. Invalid mode combinations are rejected. A six-pass actual sliver/geometry trial is running (session identifier returned at launch); result pending. Existing candidate appearance, normals, zero-intersection, volumetric tissue and all remaining original-plan gates are still unfulfilled.

### Sliver removal result and constrained triangulation (2026-10-01)

The six-pass sliver repair completed: all 44 whole-mesh triangles below quality 0.05 were brought above the floor, minimum 0.050132. Maximum cumulative displacement is 3.253493 mm; forbidden-pair union remains 25. Extent increased from 0.0045412287794296706 to 0.004602250404427137 m, an explicitly allowed tradeoff for shape-quality improvement. Full quality recomputation, closed topology, intersection union and exact export reload pass. Proof: `body-female-sliver-repair-proof.json`. Independent foot-box diagnostic finds zero below-floor triangles among 2463, minimum 0.050735 (`body-female-sliver-repaired-foot-quality.json`). Remaining near-180-degree folds and intersections prevent anatomical/appearance completion.

Added optional `--quality-floor` to diagonal repair. Compare sorted old/new quality pairs so an existing poor face cannot worsen and a previously good companion cannot become another sliver; unaffected faces retain their quality. Full final capped-quality order statistics and incremental cache are checked. Twenty-three preparation tests pass, including the important one-poor-to-two-poor rejection and a valid closed-mesh no-change audit. A six-pass actual constrained diagonal trial is running on the sliver-repaired output (session 94084). No candidate is adopted.

The constrained post-sliver diagonal trial completed: three flips reduced the union from 25 to 23 and extent to 0.004439296489244925 m while keeping zero triangles below quality 0.05. Minimum remains 0.050132. Full quality/intersection/topology/reload checks pass. Proof: `body-female-post-sliver-quality-diagonal-proof.json`. A new six-pass quality-constrained vertex trial is running (session 10246).

Separately reconstructed normals/bindings and rendered the preceding sliver-repaired 25-pair geometry on Metal, not the newer 23-pair diagonal geometry. Its close-up `body-female-sliver-repaired-foot-detail.png` was inspected: triangular protrusions remain between toes despite improved triangle-quality scores. Normal report still has 25 opposed normals. Binding count is 45,266, with 202 reprojected and 45,064 preserved, shell unchanged. Therefore triangle quality is a necessary numerical guard, not sufficient appearance or anatomical evidence. No adoption is justified.

### Local fold objective with region restriction (2026-10-01)

The post-sliver guarded vertex trial completed with 23 pairs unchanged, extent 0.00405207379991183 m, no below-0.05 faces and max movement 3.253493 mm. Full intersection/quality/topology/export checks pass. Proof: `body-female-post-sliver-quality-vertex-proof.json`.

Added optional `--fold-threshold-degrees` and `--fold-bounds`: build fixed eligible two-face edges from initial edge-midpoint coordinates, target edges exceeding the requested normal angle, and accept equal-intersection-count steps that strictly decrease the squared angle excess in radians. Every accepted step must not increase the local selected-edge deficit; final total deficit is checked. Positive quality floor, adjacent audit and no-new-pair measure mode are mandatory. Geometry outside the chosen crease region may still move if required by an existing intersection; the region restricts fold targets/objective, not all vertex movement. Fold-angle reduction may increase existing intersection extent.

Twenty-four preparation tests pass, including a known right-angle fold reduced without creating crossings, strict quality protection and a region containing no targets. A four-pass actual trial uses 160 degrees and the previously diagnosed toe region. The threshold is an engineering trial choice, not clinical anatomy, and cannot prove visually correct toes. Result pending; native comparison and complete preparation/adoption gates remain required.

### Local fold trial result and preview source guard (2026-10-01)

The four-pass local-fold run completed: selected-edge angle-excess objective decreased from 1.4268761924212277 to zero across 3688 fixed selected edges. Forbidden pairs remain 23. Minimum triangle quality is 0.050132 with none below 0.05; maximum cumulative movement 3.956928 mm. Extent increased to 0.004818846240057451 m, allowed for fold improvement. Full intersection/quality/topology/export checks pass. Proof: `body-female-local-fold-repair-proof.json`. Independent selected-foot audit finds max adjacent angle 159.872 degrees and no below-floor faces. This engineering threshold is not a clinical or visual completion criterion.

Reconstructed matching normals and bindings for this candidate; 45,266 bindings, 209 reprojected, 45,057 preserved, shell unchanged. Twenty-five shading normals still oppose current geometric smoothing. A close-up is being rendered for visual review; no adoption claim.

Candidate preview now rejects a body preset that switches the candidate's source model, preventing silent replacement by stock geometry through `set_body_parameters`. Release example build passes. Actual executable regression rejects female candidate plus male preset and creates no snapshot (`body-candidate-source-guard-proof.json`, executable SHA included). Existing stock previews retain source switching.

The completed Metal close-up `body-female-local-fold-foot-detail.png` was visually inspected. Triangular protrusions between toes remain visible after the 160-degree fold objective reached zero. This contradicts appearance completion and demonstrates that the selected dihedral threshold is insufficient to repair local anatomical shape. Candidate remains unadopted with 23 intersections and 25 opposed shading normals.

### Authored-source normal frames (2026-10-01)

Added opt-in `--source-vertex-frames` for vertex-frame transport with refinement correspondence. Compute source geometric smoothing on the untouched authored OBJ's own topology, interpolate these frames with verified parent weights, then rotate authored directions into the candidate's independent geometric frames. This avoids using repaired reference connectivity as the authored geometric frame. Frame rotation validates finite, unit source/target arrays and preserves each authored direction's source-frame angle. Seven normal tests pass, including frame-angle preservation and rejection of non-unit frame vectors. Rest-check scope is explicitly unchanged normal-frame rotation, not identity on a differently triangulated target surface.

Actual local-fold candidate: opposed normals decreased from 25 to 15, maximum mismatch 159.35 degrees, exact geometry unchanged. Proof: `body-female-source-normal-proof.json`. The original source has ten opposed normals; remaining disagreement is preserved/introduced by source interpolation and still requires correction, not a completed normal field. Rebuilt matching binding audit yields identical binding content hash to the preceding candidate because positions are unchanged. Rendering is pending. Twenty-three geometric intersection pairs remain and no candidate is adopted.

The new source-frame normal candidate was rendered on Metal and inspected (`body-female-source-normal-foot-detail.png`). Shading changed, but triangular inter-toe protrusions remain visible. This confirms that the remaining defect is not resolved by this normal correction. Geometry and authored baseline issues still prevent adoption.

### Projecting nipple profile cancellation corrected (2026-10-01)

The projecting mode previously subtracted an assumed 3 mm source bump, cancelling its default 3 mm centre displacement. Neither imported source has a measured baseline bump supporting that subtraction. Projecting mode now adds its requested reference-space relief to the imported surface; flat, inverted and reference edits retain their existing approximate baseline subtraction. This is an additive shape control, not absolute measured anatomical height. Stature scaling also scales the relief.

All 12 body-parameter release tests pass, including a new bilateral default-height and cold-height regression with remote/back-surface invariance. The release renderer was rebuilt and four native Metal offscreen chest captures were generated. The inspected comparison `body-nipple-relief-comparison.png` shows visible female neutral relief and a stronger cold profile; male relief remains small at this frontal framing. These images are not a live viewer capture. Temperature/time response, physiological calibration, source-aware flat/inverted geometry and full morphed-surface nonpenetration remain unverified or incomplete. Presets and image hashes: `body-nipple-relief-proof.json`.

### Stronger local fold trial: visual gate still fails (2026-10-01)

The 100-degree fold-objective trial reduced selected squared angular excess from 32.40949 to 0.5127105, but its measured maximum adjacent face angle remains 140.0619 degrees. The static forbidden-pair union remains 23. All triangle qualities remain at least 0.050132; cumulative reference displacement is 3.970245 mm. Intersection extent increased from 0.00481885 to 0.00510374 m, so this is not an intersection improvement. Source-frame normal transport retains 15 opposed normals. Rebinding produced 45,266 complete bindings, with 218 reprojected vertices and maximum shell distance 5.035487 mm.

Actual Metal offscreen closeup `body-female-local-fold-100-foot-detail.png` was inspected: triangular protrusions remain between toes. The model fails the visual adoption gate and remains an isolated candidate; runtime female source is unchanged. Evidence: `body-female-local-fold-100-visual-review.json`. A continuation of the same bounded local objective is running; its result remains pending.

After the projecting-profile correction, the previously built release test executable also passes the male asset/binding/animation test. Rebuilding that test against the later shared tree failed because `voxy_editor/src/import.rs` calls `to_string()` on `voxy_assets::InputError`, which currently lacks Display. The old test executable is evidence for its built revision, not proof that the latest full tree builds.

### Explicit cold-stimulus transition model (2026-10-01)

`body_parameters::ColdResponse` provides normalized target/response state and caller-supplied positive onset/recovery time constants. Exact exponential integration is independent of frame subdivision for constant stimulus. Validation is atomic; zero time is identity and large finite steps converge without overshoot. This is an illustrative response model, with no temperature mapping or physiological constants. `apply` changes only `nipple_cold_response`.

The native renderer supports explicit transition sampling with `--cold-transition-seconds`, `--cold-initial-response`, `--cold-onset-seconds` and `--cold-recovery-seconds`; the preset response is the target. All four options are required for a transition. Four Metal offscreen captures were rendered and inspected: female onset at 0/2/6 seconds and male recovery at 8 seconds. At illustrative time constants 2/8 seconds, responses are 0/0.632121/0.950213 and 0.367879 respectively. Negative elapsed time is rejected by the actual CLI. All 14 body parameter release tests and the example rebuild pass in the current shared tree, resolving the earlier build-blocking observation. Evidence: `body-cold-transition-proof.json`; comparison: `body-cold-transition-comparison.png`. Live viewer advancement/MCP time-state persistence are not integrated yet; these captures sample specified elapsed times.

The continuing local fold geometry trial completed: deficit 0.51271048 to 0.21607097, maximum selected adjacent face angle 126.6317 degrees, zero selected triangles below quality 0.05, unchanged 23 forbidden intersections. `body-local-fold-100-continued-candidate.obj` remains unadopted, with normals/bindings/native visual review still pending for this exact revision.

### Simulation-clock cold response and native entrypoint (2026-10-01)

`FemaleDemo` optionally holds response state; animation mode advances by its consumed dt and physical modes advance only by successful solver substeps. Render morphology and normal transport use current response, while body parameters retain the target. Target/source replacements preserve response state; per-frame response changes do not rebase the physical shell. `SceneApp::with_body_cold_response(initial,onset,recovery)` exposes this behavior, and `female_motion [BODY.json] --cold-response INITIAL ONSET_SECONDS RECOVERY_SECONDS` enables it in the existing native preview. Presentation measurements report enabled/current/target and `solverCoupled:false`. This is dynamic visual shape response, not a thermal constitutive tissue law.

Three focused release tests pass with `--features glam/serde`: actual mesh response at identical pose, binding preservation, unchanged animation-mode shell, recovery after target update, invalid-step/configuration atomicity, and consumed-solver-time behavior. The native example builds with that feature; its ordinary build encountered concurrent camera Vec3 serde edits in voxy_editor. A native launch is running in session 26569; presentation evidence was pending at this entry. This does not claim an observed desktop rendering. Evidence: `body-cold-runtime-proof.json`.

The native viewer emitted a real frame receipt: frame 168, simulation time 8.411258 s, target 1 and current response 0.98508859, with `solverCoupled:false`. Captured receipt: `body-cold-live-receipt.json`. The process then exited successfully (session 26569, exit 0). A subsequent target-file edit produced no recovery receipt because the viewer had exited; live recovery is therefore not claimed. Unit recovery remains verified. No desktop screenshot was captured.

### MCP cold-response acknowledgement (2026-10-01)

Status and measurements expose fresh validated `coldResponse`, `coldTargetApplied` and absolute normalized `coldResponseError` relative to the saved stimulus. Target acknowledgement requires a full matching body-parameter receipt, independently of whether the response has reached its target. `model_get.presentation` now uses the same full status path, including view/film. Missing/stale/invalid response state gives null values. Three MCP release tests and the binary build pass with `glam/serde`, covering target/current separation, body mismatch, invalid flags/numbers, staleness and existing atomic update compatibility.

An actual stdio verification wrapper stopped its child after its 30-second limit, so that run is not a protocol success. A new independent protocol check is running as session 88790 without an internal timeout. Existing live panel server processes are unchanged. Evidence: `body-cold-mcp-proof.json`.

Session 88790 completed with exit 0. Actual stdio initialize/status/measurements/get responses were parsed and validated: no-viewer cold state and acknowledgement fields are present and null at all three entrypoints. Exact responses and executable hash are preserved in `body-cold-mcp-proof.json`.

### Continued fold candidate: exact binding and visual review (2026-10-01)

Source-frame normal reconstruction and rebinding completed for the exact continued fold candidate: geometry preserved, 15 opposed normals, 45,266 bindings with 218 reprojected vertices and max shell distance 5.035487 mm. Metal offscreen rendering of that exact OBJ/sidecar succeeded and was inspected. Toe-webbing triangular protrusions remain, so the model still fails the adoption gate. Verified residual projection contains 23 forbidden pairs across 23 triangles in two local clusters; it depicts involved triangle outlines, not penetration segments. Evidence: `body-female-fold-continued-visual-review.json`, native closeup and `body-female-fold-continued-residuals.png`. A separate quality-guarded diagonal flip trial is running; it moves no vertices, but crease preservation must be audited separately. Runtime female geometry is unchanged.

The post-fold diagonal search completed in 29.07 s with zero accepted flips: 23 pairs and extent 0.00525271234 m are unchanged, candidate/source SHA identical, topology and quality gates pass. It does not improve the model. A grouped face-vertex search with the same 5 mm cumulative bound, quality floor 0.05, no-new-pair condition and 100-degree fold objective is now running (session recorded by tool); no result/adoption claimed.

### Residual patch diagnostics and grouped-search result (2026-10-01)

Added `tools/diagnose_body_residuals.py`: it requires the exact candidate SHA from an uncapped report, checks pair-union count, rechecks each listed pair against current geometry, and clusters involved faces by forbidden pair links/shared vertices. One-ring support and coordinate bounds are reported. This is local patch diagnosis; it does not search omitted pairs or measure penetration depth. Four tests pass, covering component merging/support, duplicate pair normalization, invalid indices, source hash, capped audit and false reported intersections.

Applied to the completed grouped-face candidate, all 23 pairs are reverified. Component 1 has 12 involved faces, 11 pairs and 73 support faces, at x 165.04..166.26 mm, y -801.62..-797.48 mm, z 103.55..105.33 mm. Component 2 has 11 faces, 12 pairs and 63 support faces, at x 176.89..180.24 mm, y -803.23..-799.33 mm, z 95.24..98.25 mm. Exact IDs/bounds: `body-female-post-fold-face-residual-patches.json`.

The grouped search completed with 23 unchanged forbidden pairs. Fold deficit decreased 0.21607097 to 0.21185865, but intersection extent increased 0.00525271 to 0.00529535 m. Min triangle quality remains above 0.05 (0.05001922), no topology gate failures, cumulative displacement 3.970245 mm. These changes do not certify repaired geometry or visual improvement. Candidate SHA `515c6a8db313087d7c81d51131edafd42dada4882ed3103b96b0b8c509ad99d6` remains unadopted; its exact normals/bindings/render have not yet been generated. Earlier visually defective candidates also remain unadopted. This local search cannot establish completion of anatomical geometry or the full original plan.

### Independent nipple radius/projection sides (2026-10-01)

Added `left_nipple_radius_scale`, `right_nipple_radius_scale`, `left_nipple_projection_scale`, `right_nipple_projection_scale`, all default 1 with 0.6..1.6 bounds. They multiply shared radius/projection per model-space side, with smooth central blending. Combined reference radius must remain 3..25 mm and projection 0..12 mm before cold response; inconsistent combinations reject atomically. Cold radius contraction and the existing shared cold extra projection apply afterwards. Type and physiological response remain shared. This is procedural shape parameterization, not complete anatomical morph targets. JSON omission defaults retain compatibility.

MCP schema exposes all four factors; constructor panel source has Russian labels for its schema-generated controls. Existing panel server processes were not restarted, and current panel DOM was not inspected, so live UI application is not claimed. All 15 body parameter tests, four MCP tests and three dynamic/binding tests pass with `glam/serde`; the release renderer rebuild and female/male native offscreen asymmetric captures pass. Both images were inspected and show stronger/broader relief on the configured side. Presets and hashes: `body-nipple-side-controls-proof.json`.

### Current numerical film checkpoint foundation (2026-10-01)

`SurfaceFilm::state` captures current referenced substrate geometry, topology, exact cell volumes, material and precursor wetting. `SurfaceFilm::from_state` validates input and rebuilds geometry operators/contact caches. Vertex slots unused by any film cell have zero coordinates rather than unpreserved body data. No new per-frame geometry copy was added: checkpoint allocation occurs only on explicit capture. External forcing, source drivers and full body state remain outside the core checkpoint.

Two release physics tests pass: capture after deformation and actual advancement reproduces geometry, topology, exact volumes, mass, thickness and driving pressure; five subsequent steps match exactly. Invalid volume/count/geometry/index/wetting states reject without mutating the source. `FilmPreview::state_snapshot` additionally exports JSON with current numerical state, render/solver mappings, canonical geometry, source weights/settings and mass/source/contact history. One preview regression passes with `glam/serde`, confirming deformed substrate/current cell volumes and positive source-added mass. Evidence: `body-film-checkpoint-proof.json`.

This is a numerical-state API foundation. Viewer request handling, MCP delivery and restoring a complete FilmPreview/body from this JSON are not integrated; existing `model_snapshot` remains a fresh offscreen demo capture. It is not an image or whole-body checkpoint and does not complete live liquid-state capture.

### Viewer numerical-film requests and MCP delivery (2026-10-01)

Added `film_state_request` and `film_state_get(requestId)` to MCP. The viewer handles an explicit request in its presentation callback and writes an ID-specific numerical-state JSON with applied body parameters, frame, simulation time, viewer PID and capture timestamp. The saved state contains the actual FilmPreview distribution/mappings/settings/source history. Disabled film is acknowledged explicitly with `filmEnabled:false`. Captures are not screenshots, do not restore a whole body, and retain the numerical solver substrate rather than claiming pixel/pose identity.

Publishing uses a complete temporary file and atomic hard-link creation to prevent replacing a checkpoint already published by another viewer. Status paths validate IDs; superseded/unknown pending requests are distinguished from ready captures. Two presentation tests and one actual-film viewer callback test pass; five MCP tests pass after updating the tool registry from 11 to 13. New renderer/MCP builds pass with `glam/serde`. The callback fixture verifies matching captured mass/frame and immutable repeated acknowledgement; it is not a real GPU presentation.

Actual native protocol verification is running with an isolated /tmp preset (MCP session 48854; native surface_film session 22500). At this entry no reply/frame file had yet appeared, so live capture remains pending. Evidence: `body-film-viewer-request-proof.json`. Existing panel server processes were not restarted.

Native round-trip completed successfully: actual stdio MCP queued ID 51853-1790880263090081000, the Metal viewer captured frame 0 after 0.00947004 s simulation, and actual `film_state_get` returned ready with matching ID/frame and enabled liquid. The saved numerical snapshot contains 90,175 cells and 4.999999999999998e-5 kg liquid. Independently summing cell volumes times density gives 4.9999999999999996e-5 kg. Snapshot size 22,890,580 bytes; SHA 0fc46e9412a4395b0169f726e57fe899e603db2f51aa0fe931a873f8aba7eef6. Preserved data: `body-film-native-state.json`; summary/protocol evidence: `body-film-native-capture-summary.json` and `body-film-viewer-request-proof.json`. The viewer exited successfully; this is a saved capture, not an assertion it remains running. No desktop screenshot was captured. This proves the numerical live-film request/delivery path, not whole-body checkpoint restore, visual frame equivalence, physiologically calibrated flow or completion of the original plan.

### Independent captured-film thickness visualization (2026-10-01)

Added `tools/plot_film_checkpoint.py` to validate captured numerical-state format/units, finite geometry/integer indices/nonnegative volumes and density, then derive thickness from cell volume / 3D triangle area and independently check captured mass. Four tests pass, including a vertical cell whose front projection has zero area, malformed inputs, mass mismatch and density/thickness independence. The PNG was rendered and inspected. It is a static front projection/histogram of captured numerical fields, not a renderer screenshot or proof of flow correctness.

Actual capture: 90,175 cells, 5e-8 m³ (0.05 ml), mass 5e-5 kg, peak thickness 29.3455179 µm at t=0.00947004 s. The explicit display cutoff 0.01 µm selects 58 cells and 99.9956693% of volume. Cell area range [7.056639640324176e-05, 0.00015026791838857405] m²; max selected edge 0.026221715309327678 m. These resolution measurements identify the need for spatial convergence tests rather than imply fluid realism. Source SHA matches the native captured state. Figure/report: `body-film-native-thickness.png`, `body-film-native-thickness-proof.json`. Clinical/physiological calibration, spatial/time convergence, fully realistic optics and full-plan completion remain unproven.

### Time refinement from actual captured film (2026-10-01)

Added runnable `film_checkpoint_convergence` example. It validates capture format/SI units, restores the current numerical film, checks initial mass against captured measurements, and replays original material/wetting on fixed geometry with gravity/capillarity/wetting. Four exact common-duration runs use dt 1/0.5/0.25/0.125 ms. Every run checks finite nonnegative volumes and relative mass error <=1e-12. Negative duration is rejected before replay. No sources, contact exchange or body motion are replayed; the finest run is a numerical reference, not an exact solution.

Actual 90,175-cell capture was replayed for 0.1 and 1 seconds. Both show decreasing temporal differences. At 1 second, normalized volume L1 differences against 0.125 ms are 2.21271489e-8, 9.48299894e-9, 3.16098829e-9; max relative mass error 1.08420217e-15. Total normalized redistribution from initial state is about 0.00279549 (0.2795%), so the transport in this thin, viscous capture is modest. Runtime of the 8,000-step finest 1-second replay was 44.87 s on this run; this is not a full-frame performance measurement.

Reports with source/executable hashes: `body-film-captured-time-convergence.json`, `body-film-captured-time-convergence-1s.json`. Figure `body-film-captured-time-convergence.png` plots measured field differences. This verifies temporal refinement for these two static captured-state replays. Spatial convergence, moving-body/contact/source replay, physiology and the full original plan remain unproven.

### Captured-film spatial refinement replay (2026-10-01)

The release `film_checkpoint_spatial` executable completed on the preserved native capture. Conforming subdivision produces 90,175 / 360,700 / 1,442,800 cells, retaining the original piecewise-planar geometry and material/wetting. Each run advances 0.02 seconds at dt 0.00025 seconds without source, contact or body motion. Child volumes inherit parent thickness and restrict conservatively to original cells. The midpoint-sharing/thickness/restriction unit test passes.

Normalized restricted volume L1 errors against the finest run are 8.4513433e-5 and 5.6489673e-5 (ratio 0.66841). Maximum initial mass error is 6.10e-15; maximum final mass error is 4.07e-16. Measured replay times are 0.241 / 1.418 / 4.764 seconds, excluding construction. This is a decreasing error over three grids, not established asymptotic convergence or moving-body accuracy. Redistribution itself changes with refinement; further resolution and coupled-motion validation remain required. Exact source/capture/executable hashes and measurements: `body-film-captured-space-convergence.json`. No geometry candidate is adopted by this check.

### Temporal control of captured-film spatial refinement (2026-10-01)

The spatial replay now independently restores each grid and repeats its 0.02-second evolution at half dt (0.000125 seconds, 160 steps), retaining the same material and initial thickness. Half-step runs validate finite nonnegative volumes and relative mass error <=1e-12. Release rebuild and midpoint/mass/restriction test pass. Actual captured replay completed on all three grids. Restricted normalized dt-versus-half-dt errors are 6.4721e-11 / 1.9020e-10 / 1.01946e-9. The maximum divided by the fine-versus-finest spatial difference is 1.80468e-5 (0.001805%). Maximum final half-step mass error is 1.36e-15. Thus the observed spatial difference greatly exceeds this particular temporal difference. This is not an asymptotic error bound or moving-body/contact convergence proof. Exact inputs, executable/source hashes and measurements: `body-film-captured-space-time-control.json`.

### Captured-film moving geometry and source replay (2026-10-01)

Added `film_checkpoint_motion` to exercise the existing atomic geometry/source/transport path on the complete captured 90,175-cell substrate with its original material and precursor wetting. A prescribed invertible affine cycle rotates by up to 0.08 radians and stretches x by +/-3%, then returns to its original surface after 0.02 seconds. This is a numerical stress trajectory, not skeletal/physiological motion; there is no acceleration-force coupling. A source at the initially wettest cell supplies 1e-9 m3/s. Four runs at 20/40/80/160 steps complete with finite nonnegative volumes and per-step source-adjusted mass errors <=1.355e-15. Final geometry returns within 2.78e-17 m and expected final mass is 5.002e-5 kg. Normalized distribution errors against the finest step decrease 8.365e-9 / 3.577e-9 / 1.201e-9. Release build and orientation/return test pass. Exact measurements and input/source/executable hashes: `body-film-captured-motion-convergence.json`. Whole-body skeletal animation, self-contact exchange, droplets and two-way inertial coupling remain outside this proof.

### Animated film substrate synchronized with the presented pose (2026-10-01)

Found and corrected an animation-only ordering defect: film geometry/transport previously ran before skeleton time and cold response advanced. The animation branch now stages current time/response first, computes the actual mesh at that time and advances the film. On mesh/film failure, animation clocks and cold state roll back. A film animation dt above 0.1 seconds rejects instead of silently truncating film time while advancing body time. Physical solver modes retain their prior path and need a separate ordering/consumed-time audit.

Two release film capture tests pass: exact equality of every referenced film point to the current skeletal mesh, nontrivial motion, source-adjusted mass, oversized-step rollback of all numerical film state, and existing request-correlated capture. Initial test comparison included unused OBJ slots, which the numerical snapshot intentionally zeros; the corrected test checks the actual referenced substrate. Three cold-response/binding/consumed-solver-time tests pass. A concurrent lighting-cache edit initially failed Debug derivation; adding Debug to SurfaceLightInput restored compilation. Proof: `body-film-animation-synchronization-proof.json`. This is one-step synchronization evidence, not animation/contact convergence or whole-plan completion.

### Film clock and geometry aligned with physical integration (2026-10-01)

Physical modes previously transported film on the old mesh for the input frame dt before the tissue solver consumed a possibly different interval. Film now advances after successful integration, using the current posed/deformed surface and the time actually consumed by the body solver. Frames which only accumulate time do not evolve film or add source volume.

All four release film-state tests pass, including independent secondary-region and full implicit skin cases. A 0.001-second frame leaves body time and full film snapshot unchanged; a following 0.1-second input consumes 0.05 seconds. Source-adjusted film mass matches that consumed duration within relative 1e-12 and every referenced numerical substrate point exactly matches the current mesh. Animation and request-correlated snapshot tests also pass. This does not provide whole-system rollback if fluid fails after tissue advancement, long-duration/convergence/contact validation or two-way force coupling. Proof: `body-film-physical-clock-proof.json`.

### Atomic geometry/source/transport/self-contact film frame (2026-10-01)

Added `SurfaceFilm::advance_on_geometry_with_contact` and switched FilmPreview to it. Optional bridge detection/exchange happens on staged numerical state; any error rejects the entire film frame, including geometry, source and transport. Source and contact receipt counters update only after successful commit. Previously a self-contact error was swallowed after earlier stages had already committed. Existing contact BVH is cloned and refitted in staging, retaining the original cache on failure and avoiding a full rebuild on subsequent successful frames. This adds staging memory/work, whose full-body performance remains to be measured.

Two warmed-BVH core tests pass: late-stage invalid-contact rollback and positive exchange between opposing surfaces, exact agreement with sequential stages and source-adjusted mass preservation. Two preview invalid-frame tests pass, including injected invalid contact parameters and unchanged geometry/volumes/measurement counters. Portable checkpoint compatibility tests are running in session 86508. Initial source test used an exact rounded literal for dt*rate; the corrected assertion compares the same arithmetic product. Proof: `body-film-atomic-contact-proof.json`. Whole-tissue rollback after successful tissue advancement, CCD and calibrated physiological transfer remain incomplete.

Session 86508 completed with exit 0: both warmed-contact tests and both checkpoint compatibility tests pass. Combined with the two preview failure tests, six relevant checks are green.

### Full captured substrate atomic-contact CPU profile (2026-10-01)

Added and release-built `film_checkpoint_contact_profile`. The actual saved full substrate (45,203 vertices, 90,175 cells) advances through 22 prescribed invertible stretch steps, dt 0.001 seconds, with original material/wetting. Two independently restored films run with/without bridge search in alternating order; geometry generation is outside the measured interval, and two warmup frames are excluded. Maximum mass error is 9.49e-16, and final volumes are finite/nonnegative. Steady upper medians are 39.811 ms without contact and 46.184 ms with contact (6.373 ms difference, ratio 1.160). First frames are 38.388/298.827 ms, including initial contact-tree construction. Samples have large scheduling outliers up to 303 ms on the concurrently busy host; these numbers do not establish stable performance or viewer FPS. Gross contact transfer is zero in this captured wet distribution: full-body bridge transfer remains unverified despite the successful opposing-patch unit tests. Exact samples/input/source/executable hashes: `body-film-full-contact-profile.json`.

### Numerical FilmPreview checkpoint restoration (2026-10-01)

Added atomic `FilmPreview::restore_snapshot`, validating SI/version, exact render mapping/representatives/canonical chart/topology, source/settings agreement, physics/settings agreement, finite nonnegative history and mass accounting before committing. Saved source weights are retained exactly; recomputed quadrature is checked in normalized L1 within 128 machine epsilons (observed maximum entry difference 1.39e-17). Enabled serde_json float_roundtrip for exact serialization continuation. The existing renderer now accepts `--film --film-state PATH.json` for a state object or capture envelope; release build passes.

Three release tests pass: serialized exact restoration/continuation, atomic rejection of malformed mapping/topology/volumes/settings/history, and the actual preserved native 90175-cell capture. The latter reconstructs the archived film subset only after verifying every face against the unchanged imported source OBJ, then reproduces the entire snapshot and the next step exactly. Initial relative-path invocation failed; absolute-path test invocation succeeds.

The current runtime facial mask has changed and constructs 90062 film cells. Restoring the old 90175-cell capture into that current chart correctly rejects; the attempted renderer run produced no image. These topology compatibility checks remain strict. Automatic topology migration, viewer/MCP restore requests, whole-body pose/tissue checkpoint and physiology remain incomplete. Proof: `body-film-preview-restore-proof.json`.

### Current-chart checkpoint and rendered roundtrip (2026-10-01)

The renderer now supports `--film-state-output PATH.json`, requires enabled film, and saves numerical state after successful GPU rendering. Release rebuild passed. Two actual native offscreen Metal runs generated `body-film-current-save.png` and `body-film-current-restored.png`, with matching initial/restored state files. Current film has 90062 cells, mass 5e-5 kg. JSON files and PNG files are each byte-identical across save/restore. Both images were inspected. This closes the current-chart renderer roundtrip proof without migrating the older topology.

Visual review finds an obvious flat polygonal film patch on the abdomen, so material realism is still incomplete. Both controlled renders disable surface diffusion and do not prove full optical SSS/refraction. This is numerical film restoration in the renderer, not a desktop viewer screenshot or full-body solver checkpoint. Exact hashes and limitations: `body-film-current-render-roundtrip-proof.json`.

### Film layer composites over shaded skin instead of opaque vertex albedo (2026-10-01)

The flat polygonal patch came from the film shader returning opaque vertex colour, replacing the already shaded skin. Film now uses the existing straight-alpha pipeline to composite dielectric reflection and transmission loss over the shaded substrate. Coverage still derives from calculated thickness. Single-alpha RGB attenuation uses the darkest transmitted channel; this is explicitly an approximation pending a scene-colour optical/refraction pass. Layers with negligible opacity discard.

Release renderer rebuild and three Metal offscreen renders pass. `body-film-transparent.png` was inspected and the former flat patch is absent; numerical checkpoint remains exactly unchanged. Compared to the previous image, 25393 pixels change. Actual zero-absorption/IOR=1 endpoint is pixel-identical to a separate no-film render, proving this endpoint preserves shaded skin. Ordinary water-like film differs from dry skin at 17677 pixels, with maximum 2/255 channel difference at this frontal lighting. This does not prove visually resolved highlights across angles or full optical realism. Exact images/checkpoint/shader hashes and limits: `body-film-transparent-proof.json`.

### Sampled opaque-scene film optical pass (2026-10-02)

Added general renderer methods `create_sampled_color` (filterable colour attachment on the renderer device, validated size/format) and `encode_transparent_over` (load existing colour/depth, depth-tested draws without depth writes). The opt-in `female_render --film --film-scene-optics` splits ordinary film geometry from the opaque body, renders the opaque image into a sampled sRGB attachment, copies it to the final target and draws film using that image. Diagnostic thickness geometry retains its prior path.

The scene-film shader uses per-channel Beer-Lambert transmission, dielectric reflection and a thickness/Snell refraction tangent displacement converted to pixels through the local surface Jacobian. Sampling is linear and clamped to the image edge. This removes darkest-channel attenuation from this opt-in pass; the ordinary single-pass fallback remains approximate. `--film-no-refraction` is an explicit control for validating displacement while keeping geometry/other optical terms fixed.

Release rebuild and five Metal offscreen optical renders pass. Fully transparent IOR=1/zero-absorption pass is pixel-identical to the separately rendered dry body. With only red absorption (10000 inverse metres), 16924 pixels change only in red, maximum reduction 21/255; green/blue/alpha remain exact. A diagnostic 100-times-volume stress fixture with refraction enabled/disabled changes 3443 pixels by at most 1/255; its amplified mass/thickness is not physiological. Ordinary numerical film state remains exactly unchanged. Images were inspected. Proof and source/image hashes: `body-film-scene-optics-proof.json`.

This is a screen-space, single thin-interface approximation in the offscreen example. Live SceneApp integration, explicit foreground occlusion fixture, offscreen visibility, multiple interfaces, true volumetric SSS and performance optimization remain incomplete. Extra mesh splitting/uploading costs roughly 80-100 ms in these concurrent smoke runs; these timings do not establish stable viewer performance. The full anatomy/physics/visual plan is not complete.

### Four-sample optical composition foundation for native viewer (2026-10-02)

Live female SceneApp uses four-sample attachments, whereas the new optical pass initially supported only single-sample targets. Added `SceneRenderer::encode_transparent_over_msaa4`, preserving loaded four-sample colour/depth, depth-testing without depth writes, and resolving to a single-sample final image. It rejects a renderer without MSAA pipelines before encoding. Single/four-sample paths share the same load-pass implementation. The offscreen renderer accepts `--msaa4` and exercises opaque resolve into sampled scene colour followed by the optical four-sample pass.

Release rebuild and three actual Metal renders pass. Shared runtime source changed during work to 60866 vertices/121728 triangles; old snapshots correctly reject mismatched render maps. A fresh compatible film snapshot contains 121388 cells and mass 5e-5 kg. Validation uses a frozen executable copy; fully transparent optical MSAA image is exactly pixel-identical to a separate dry MSAA reference. Current water-like optical render was inspected. Proof/hash files: `body-film-scene-msaa4-proof.json`; image: `body-film-scene-msaa4.png`.

This completes only the MSAA prerequisite. Live SceneApp wiring, resize/suspension/recovery, temporal colour/motion ordering and live frame receipt validation remain outstanding. The full anatomy/physics/visual plan remains incomplete.

### Reusable optical layer split, 2026-10-02

`SceneMesh::split_material_layer` preserves vertex IDs, bind-space coordinates and optical parameters; mixed-tag faces are rejected and absent layers return `None`. The offscreen renderer now uses this helper and clears stale film geometry when the layer disappears. Two focused release tests passed using nondegenerate triangles; the release example built and Metal/MSAA4 smoke rendered successfully on the current 60,866-vertex mesh. The saved full-body image was inspected, but does not establish close-up optical quality. Evidence: `body-film-layer-refactor-proof.json` and `body-film-layer-refactor.png`. Native viewer composition and its current-frame temporal input ordering remain unfinished.

### Shared refractive composition, in progress

Added `SceneRenderer::encode_refractive_layers`: current-frame background pass, destination world pass, depth-tested refractive layers, then UI. Supports single-sample and MSAA4. It avoids COPY_DST requirements on acquired surface images by rendering the world twice; performance has not been established. The offscreen example now calls this same composition entrypoint. Release example build passed; GPU validation is running (session 67070, output `/tmp/voxy-film-common-composition.png`). Native surface and temporal-input integration remain pending.

Shared composition GPU validation completed successfully on Metal/MSAA4. Output was inspected and PNG SHA256 matches the preceding layer-split render (`f93326f3d115764f80f6178cffaac494cff2f5c1639a288a95c0210479dbaab9`), confirming unchanged pixels for this offscreen fixture. This does not yet verify native surface lifecycle or temporal integration.

### Native refractive composition wiring, in progress

`SceneSurface` now owns presentation and temporal sampled backgrounds, invalidated on resize. A new refractive temporal-hook entrypoint preserves the existing API wrapper. Both current-frame presentation and temporal color call the shared compositor; motion-vector generation still uses opaque geometry. `SceneApp` splits film geometry, updates its bindings, clears it when absent, and submits the separate layer. Current checks are pending: voxy_render cargo check session 94223 and voxy_app cargo check session 52411. No native optical frame, resize replay, or temporal GPU validation has yet been observed for this wiring; do not treat it as verified integration.

### Native optical validation, 2026-10-02, incomplete

Renderer cargo check passed. Release surface_film build passed after fixing the import editor borrow conflict. Added `--smoke` and `--motion` example arguments. The first native Metal process exited 0 but emitted no SCENE SMOKE PASS marker, so it does not prove 120 presented frames, resize coverage, or temporal correctness. Added an explicit smoke-run completion guard: premature event-loop exit now reports presented frame/resize counts instead of passing silently. Rebuild session 11589 and editor check session 85021 remain active. The editor fix evaluates required residency against the newly published catalog using disjoint fields rather than calling a method that borrows all of self while the import queue is borrowed.

Native optical replay exited before verification at frame 42 with resize_stages=1; this is a failed/incomplete replay, despite no emitted GPU validation errors. Added event logging for CloseRequested/Escape and a presented-film-layer counter requiring a layer in every smoke frame. Release rebuild passed; replay session 2979 is active. Neither 120-frame coverage nor both resizes have yet been proved.

Native replay session 2979 reached final verification but failed `female skin smoke did not observe deformation`. Inspection showed the secondary-only integrator increments steps but never updates max_displacement; the scalar belonged only to the full skin integrator. Added local cage displacement relative to rest plus root bob and accumulate it after each secondary region step. The existing nonzero-deformation gate remains intact. Release example rebuilt successfully and editor cargo check passed. Focused parameterized-cage test session 29388 remains live; the assertion was moved after integration while compilation was active, so confirm the compiled result before treating coverage as proven. A new native replay is running with the corrected measurement.

### Native optical path smoke passed

Replay session 14241 exited 0 with explicit FILM OPTICAL SMOKE PASS (120 presented layers, temporal=true, resize_stages=3), FEMALE SKIN SMOKE PASS (122 nonlinear steps), and SCENE SMOKE PASS (120 presented frames). This verifies a nonempty film layer for every presented frame, both requested sizes and motion/temporal frame identity/reset checks in the native Metal path. Evidence is `body-film-native-optics-proof.json`. No native screenshot, close-up image quality review, or GPU readback equivalence of temporal color was performed. A one-second CPU sample was retained separately and is not an FPS benchmark. The focused displacement test recompilation remains in progress.

The rebuilt focused cage test (session 97917) passed: unloaded local displacement is zero and becomes nonzero after integration; the rendered unloaded surface remains unchanged.

### Native per-frame mass balance gate, in progress

Added `FilmPreview::verify_mass_balance`, comparing current mass to initial plus source-added mass and rejecting nonfinite/negative totals, invalid tolerance and recorded contact errors. Native smoke now checks every presented film frame with relative tolerance 1e-9 and reports the maximum error; it requires the check count to match presented frames. The small moving/source checkpoint test also verifies conservation and rejects deliberately corrupted source history. Test session 16506 and release build session 68288 are live; no pass or native measured error has yet been observed for this new gate.

### Moving body + source + contact mass replay, in progress

Native mass replay session 59416 ended on an explicit CloseRequested at frame 46 (47 counted by the completion guard), so it is incomplete. Added a two-second 120-frame headless integration replay using the actual FemaleDemo secondary solver, rebuilt substrate, nonzero liquid source and enabled self-contact. Every frame must conserve source-adjusted mass within 1e-9; the final test checks consumed simulation time, nonzero local tissue deformation, exact film/render substrate correspondence and expected 2e-6 kg source addition. It reports maximum observed mass error and actual gross contact transfer, without presuming transfer occurs. Cargo test session 55965 remains active. This does not replace native optical validation or whole-body contact convergence.

Actual moving-body/source replay passed: 120 frames, time 1.9999999999999978 s, maximum relative mass error 1.829656511e-15, zero actual contact transfer. See `body-film-moving-source-mass-proof.json`. Added a separate approaching/separating deforming-surface contact replay requiring positive transfer while near, no transfer while separated, nonnegative volumes and source-adjusted mass conservation over 200 steps. This is a two-triangle contact fixture, not evidence of whole-body contact.

Moving contact suite passed 3/3. Over 200 prescribed-motion steps there were 56 steps with actual transfer and 126 beyond maximum gap with exactly zero transfer; gross transferred volume 4.267646060e-10 m3, maximum source-adjusted mass error 8.687517408e-15. Initial fixture assertion on source addition was changed from bit equality to 1e-25 absolute tolerance to accommodate one-ULP multiplication rounding. Evidence: `body-film-moving-contact-proof.json`. Whole-body contact and CCD remain incomplete.

### Current anatomical source audit, 2026-10-02

Fixed the runtime audit source selection: it now reads the adopted female BODY include and new_male include from female_demo.rs, rather than silently inspecting historical body-nipple-refined.obj. Two source-selection tests pass, including rejection of unsupported declarations without fallback. The actual female body-forehead-refined.obj has 60,866 vertices/121,728 triangles and 484 nonadjacent intersection pairs: head 256, left foot 111, right foot 109, left hand 6, right hand 2. It has no boundary/nonmanifold/inconsistent winding edges or duplicate triangles. Current male source has zero detected nonadjacent intersections. Report: `body-runtime-current-geometry-audit.json`. Adjacent positive-length/area overlap audit session 87089 is still live. These are static surface checks, not internal anatomy, whole-body contact or continuous collision proof. Female geometry preparation is explicitly incomplete.

Adjacent audit completed for the current female source: 759,311 tested adjacent pairs, 50 forbidden overlaps beyond shared vertices/edges. Male adjacent audit remains active in session 87089. Both current adopted meshes have zero degenerate triangles under the nonadjacent audit.

### Current female repair candidate, in progress

Male adjacent audit finished with 546,913 tested pairs and zero forbidden overlap; current male has zero detected adjacent/nonadjacent static intersections. Female current audit has 534 combined forbidden pairs in 17 connected patches, exact pair rechecked against the hash-matched source (`body-current-residual-diagnostics.json`). Launched conservative diagonal-flip repair of body-forehead-refined.obj: four passes, adjacent checks, decreasing intersection measure, triangle quality floor 0.05. Session 56498 remains active. Output is an unadopted candidate; no runtime source/binding/film changes made.

Current source surface quality audit completed: 44 triangles below scale-invariant quality 0.05, minimum 0.013461740897984005, zero collapsed triangles, maximum adjacent angle 179.9760484311918 degrees. See `body-current-surface-quality.json`. These diagnostics locate numerical/shape concerns but do not infer anatomical defects from quality alone. Diagonal repair process remains live in session 56498; no candidate adoption is justified yet.

### Current diagonal candidate completed

Session 56498 exited 0 after 101 accepted flips. Global combined forbidden pairs decreased from 534 to 390; intersection-segment measure decreased from 0.2405245713878746 m to 0.2047109460660613 m. No vertices moved and topology audits remain clean. Triangles below quality 0.05 decreased from 44 to 39; minimum remains 0.013461740897984005. Export/reload identity was verified. The candidate is not valid/adopted because 390 forbidden pairs remain. Report: `body-current-diagonal-candidate-proof.json`. Also passed 35 geometry/binding/normal tests and verified all 60,866 existing binding identities against the current source without reprojection (`body-current-binding-baseline-proof.json`). Residual patch diagnosis is running.

### Current local repair and optical comparison, in progress

Started face-group local descent on the diagonal candidate, two passes with cumulative displacement bounded to 1 mm relative to adopted body-forehead-refined.obj, full adjacent checks, decreasing crossing measure and quality floor 0.05 (session 52748). An initial incompatible group-mode invocation failed before creating files; the corrected face-only invocation is live. Transported authored normals for the diagonal candidate without geometry changes; rest preservation error 2.220446049250313e-16, maximum normal change 99.76121133225288 degrees, 10 normals opposed to geometric smoothing. This candidate is not suitable for adoption yet. A frozen /tmp/voxy-current-repair-renderer is producing original and diagonal face closeups (sessions 80257, 41596). No image has yet been inspected.

Both frozen-renderer face captures completed on Metal and were inspected. Global face proportions look unchanged at this view; eyelid/eyelash raster artifacts remain in both. Pixel difference is recorded in `body-current-diagonal-visual-proof.json`; captures are `body-current-face-original.png` and `body-current-face-diagonal.png`. This view cannot validate foot/hand repair or anatomical correctness of local folds.

Corrected rebind_repaired_body metadata: render_vertices now updates to the new point count along with bindings. Previously the refinement path could retain the old count despite added bindings. Four focused binding tests passed, including unchanged source metadata and matching output binding/vertex counts. Local mesh descent remains live in session 52748. Baseline authored/geometric normal comparison is running separately; no candidate is adopted.

Normal baseline comparison completed: the adopted source already has 10 authored normals opposed to geometric smoothing (maximum angle 159.34886425687154 degrees), matching the diagonal candidate count. These normal disagreements therefore predate the candidate; they remain a geometry/material review item.

Added optional connected-component selection to plot_body_residuals.py for inspectable local geometry. Generated and inspected exact hash-matched diagonal-candidate diagnostics: component 0 has 48 pairs/25 triangles in a left foot patch; component 5 has 74 pairs/74 triangles in a lip-region patch. Files `body-current-foot-residuals.png` and `body-current-lip-residuals.png` are scientific geometry diagnostics, not material renders. Local descent session 52748 is still live; no final result or adoption yet.

### Consistent regional tissue masses, in progress

Found a physical parameterization discontinuity: default Region cages used unit inverse masses (1 kg movable particles), while any nondefault BodyParameters switched to volume-lumped mass at illustrative density 1000 kg/m3. Removed the default-only exception; all cages now calculate nodal mass from tetrahedral rest volume. Preserve fixed particles and expose their actual solver inverse masses for validation. Region stores full lumped mass including pinned nodes for reporting, and native receipts now include cage masses and assumed reference density. Focused tests check mass versus volume and actual solver weights/pins. Test session 41759 is still running and was compiled before the final inverse-mass assertions were added; rerun the updated focused tests after that handle is terminal. Local geometry repair session 52748 remains live. These coarse proxy volumes/masses are not measured anatomical tissue masses.

Updated regional mass checks passed 3/3 in session 35158, including actual inverse-mass/pin assertions, volume/load recovery and height collider regression. Added a tiny-height edit regression requiring continuous mass/inverse-mass changes; session 94927 is live. Replaying the moving body/source film invariant after the mass change in session 73414. Geometry local repair remains live in session 52748. Evidence: `body-cage-mass-consistency-proof.json`; coarse cage mass is not an anatomical measurement.

Post-mass-change moving-film replay passed: 120 frames over 2 seconds, maximum source-adjusted mass error 1.829656511e-15; no actual contact transfer in this trajectory. Tiny-height regression remains live.

### Local candidate completed, still unadopted

Session 52748 exited 0: combined crossings fell from 390 to 289, measure from 0.2047109460660613 m to 0.16286413685950532 m, maximum cumulative displacement 0.0003712888015048343 m. Minimum quality improved to 0.018097731164283242; 36 triangles remain below 0.05. Topology audit is clean; residuals are eight patches in feet/lip areas. Updated 248 surface bindings, preserved 60,618 and kept the 636-node physical shell unchanged; maximum reprojection distance to that shell is 5.619691584518275 mm (not vertex movement). Authored-normal transport session 88907 is live. No adoption because 289 forbidden pairs remain.

Tiny-height inertia test failed despite consistent mass calculation: default cages skipped BodyParameters stature normalization but nondefaults used it. Fixed Region morph to normalize both cases using the same transform; did not relax the continuity assertion. Focused volume_regions test session 57750 is live. Full render-body default normalization remains a separate compatibility consideration; this change normalizes physical cages and subtracts their transformed embedded rest in rendering.

Mass continuity verification session 57750 stopped at a concurrent DropletLifecycle field mismatch. Current source now declares capture_film_immersion; restarted the focused tests from the terminal failed handle rather than treating the failure as a pass. Local candidate authored normals completed with unchanged geometry; maximum transported-normal change 164.5764501267851 degrees. Candidate face preview is running in session 44835, not yet inspected; 289 intersections remain, so large normal rotations and shape need review before any adoption.

Local candidate frontal face render completed and was inspected. Gross proportions remain similar; eyelid/eyelash artifacts persist. Saved `body-current-face-local.png` and quantitative pixel comparison `body-current-local-visual-proof.json`; this does not validate foot repair or hidden folds. Launched single-vertex descent with 26 signed lattice directions, two passes and the same cumulative 1 mm/quality/global-audit limits, session 88890. Output remains an unadopted candidate.

All four updated Region tests passed in session 57011, including the formerly failing tiny-height inertia continuity test. Replaying moving-body/source mass after reference normalization; prior source-conservation evidence was before this final cage transform change and is not automatically current.

Post-normalization moving-body/source replay session 82527 passed 120 frames over 2 seconds; maximum source-adjusted mass relative error 1.829656511e-15, zero contact transfer. Single-vertex repair session 88890 completed: 289 to 284 forbidden pairs, crossing measure 0.16286413685950532 to 0.14800520692733518 m, cumulative displacement remains 0.3712888 mm; 34 triangles below quality 0.05. Clean topology, unadopted because intersections remain.

Ring-group repair session 96776 remains live (PID 43233 confirmed at 100% CPU). Removed exact duplicate proposal directions from future vertex-descent runs, preserving first-seen order and candidate semantics; all 24 prepare_body_geometry tests passed. The already running repair loaded the previous script before this edit and is not claimed to use the optimization.

Prepared the 284-pair vertex candidate for subsequent rendering without adoption: normal transport session 52982 exited 0, preserved geometry and retained 10 opposed normals already present in the baseline; maximum normal rotation 164.5764501267851 degrees remains a visual review concern. Rebinding exited 0: 276 reprojected, 60,590 preserved, original shell unchanged; maximum shell reprojection distance 5.632559370445809 mm is not geometric vertex displacement. Ring-group repair session 96776 is still live. Proof: body-current-vertex-normals-proof.json and candidate skin audit.

Vertex candidate and original right forefoot Metal offscreen renders completed in sessions 6086/60537 and were inspected. Gross toe contours remain similar, but interdigital gaps/angular surfaces persist in both. Frozen renderer predates cage-normalization change, so this is a static geometry/appearance check only. Evidence: body-current-vertex-feet-visual-proof.json. Ring-group repair session 96776 remains active.

Added explicit --foot-view top|sole|outer|inner to female_render --foot-detail. Default camera remains top; invalid direction or direction without foot-detail rejects. Intended to inspect hidden interdigital folds from both sides before accepting repairs. Release example build session 90085 remains live; no rendered validation yet. Ring-group repair session 96776 also remains live.

Foot-view release build session 90085 passed. Metal offscreen sole and inner captures exited 0 (75597/78406) and were inspected: sole reveals polygonal toe pads/interdigital folds, while the inner closeup is occluded by the near toe surface and does not expose the residual patch. Invalid view actual CLI session 95933 correctly exited 1 with allowed directions. Ring repair session 96776 remains active. This is visual diagnosis, not anatomical validity or zero-intersection proof.

Targeted right forefoot quality audit on the 284-pair vertex candidate selected 2,337 triangles: minimum quality 0.08005045520092374, zero below 0.05 and zero zero-area triangles, but maximum adjacent-face angle 177.7834328428636 degrees. Therefore visible defects cannot be attributed merely to sliver triangles; near-reversed folds need direct geometric review/repair. Exact source hash and edge IDs retained in body-current-vertex-foot-quality.json. Ring repair remains active.

Started a distinct right-foot fold candidate in session 60413 from the completed 284-pair vertex candidate. One fairing-assisted single-vertex pass, cumulative 1 mm bound, complete adjacent/nonadjacent audit, quality floor 0.05, local fold objective threshold 150 degrees within [0.15,0.20]x[-0.83,-0.78]x[0.08,0.12] m. Threshold is an engineering search criterion, not anatomy calibration; candidate remains unadopted. Ring-group session 96776 is still live and unchanged.

Fold candidate session 60413 completed: crossing count remains 284; aggregate fold deficit 0.9578197585529742 to 0.6610239157393385, but independent regional audit found worst angle increased 177.7834328428636 to 177.8442239009238 degrees. Candidate not adopted. Tightened fold search to reject increased local maximum angle and verify global selected-edge maximum at completion; all 24 geometry tests pass. Started strict rerun from pre-fold vertex candidate. Native mass/optics smoke running in session 72193 after successful release build 2521; Metal adapter initialized, no final smoke result yet. Ring repair 96776 remains active.

Current post-cage-normalization native Metal smoke session 72193 exited 0: 120 presented film layers, temporal=true, resize stages bitmask 3, 122 nonlinear tissue steps; 120 per-frame source-adjusted mass checks, maximum relative error 1.084202172e-15. Frozen executable hash retained in body-film-native-mass-optics-proof.json. This validates the actual native integration after the mass/reference change, not zero-intersection anatomy, physiological calibration, screenshots or GPU temporal-color readback. Ring repair 96776 and strict fold repair 19317 remain live.

Strict fold rerun 19317 exited 0: forbidden pair count remains 284, fold deficit 0.9578197585529742 to 0.6610029588786728, maximum cumulative displacement 0.3782971064815736 mm, clean topology. Independent right-foot audit verifies maximum adjacent angle improves 177.7834328428636 to 177.13208336088624 degrees; no triangles below quality 0.05 in selected 2,337 triangles. Candidate remains unadopted. Future fold reports now include initial/final maximum angles; 24 geometry tests pass including reported worst-angle monotonicity and empty-region behavior. Ring-group session 96776 remains live.

Started continued strict fold descent from the independently audited improved candidate: session 46707, three passes, same cumulative 1 mm bound, local worst-angle nonincrease and complete crossing/quality gates. Original ring-group repair session 96776 remains active; it has not been restarted. No candidate adoption or zero-intersection claim.

Ring-group session 96776 completed successfully: 284 to 199 forbidden pairs, crossing measure 0.14800520692733518 to 0.113410477627335 m; cumulative displacement 0.45070237188518203 mm. Clean topology, 33 triangles below quality 0.05. Exact residual diagnosis shows six patches: left forefoot two, right forefoot three, left mouth one; previous right mouth patch and one left foot patch eliminated. Candidate remains unadopted. Started a distinct two-pass face-group candidate from this improved ring output; continued strict foot-fold session 46707 remains live on its independent branch.

Ring-candidate normal transport/rebinding passed in sessions 4241/53717: 480 reprojected, 60,386 preserved bindings, unchanged shell. Frontal face original/candidate rendered with the same frozen executable in 96464/46303, both inspected; gross face remains similar, lip contours differ slightly and hair raster artifacts persist. Proof body-current-ring-face-visual-proof.json. Independent continued-fold branch 46707 completed: 284 to 282 pairs, deficit 0.6610029588786728 to 0.323494825428919 and maximum selected angle 177.13208336088624 to 176.98827857426215 degrees; not merged into the better 199-pair ring branch. Post-ring repair 50396 remains active.

Future vertex-descent proposals now audit each candidate triangle pair once in canonical face-index order, matching the full audit rather than repeating reverse-order checks when both faces move. All 31 geometry/quality/residual diagnostic tests pass; no whole-model timing improvement claimed. Already-running post-ring session 50396 loaded its previous implementation and is not claimed to use this optimization.

Generated and inspected exact ring-candidate component-2 frontal/side geometry plot body-current-ring-left-lip-residuals.png: 74 forbidden pairs/74 participating triangles localized to left mouth edge. Scientific geometric projections, not material render or interior anatomy. Current face-group repair session 50396 remains live. Existing canonical-pair optimization is covered by the ring-neighbourhood and shared-face fixtures in the passed geometry suite.

Post-ring face-group repair session 50396 completed: 199 to 193 forbidden pairs, measure 0.113410477627335 to 0.10166656115150004 m, maximum cumulative displacement 0.7399756866638202 mm; clean topology and unchanged 33 below-quality-0.05 count. Ring-candidate left-mouth regional quality selected 624 triangles, minimum quality 0.11080274411364403 and maximum adjacent angle 130.793421980477 degrees, no degenerate/sliver triangles. Started edge-group candidate from latest 193-pair mesh with cumulative 1 mm bound projection; uses deduplicated pair/proposal audits. Candidate remains unadopted.

Independent post-ring right-foot quality audit confirms near-reversed folds remain: maximum adjacent angle 177.70477149069643 degrees, 2,337 selected triangles, minimum quality 0.07983469141346555, no zero-area or below-0.05 triangles. Crossing-only reduction has not resolved the visible fold defect. Edge-group session 23720 remains live; future acceptance must include fold/appearance review rather than crossing count alone.

Edge-group session 23720 completed: count stalled at 193 while measure improved 0.10166656115150004 to 0.09578981742045119 m, cumulative displacement 0.811840559328981 mm. Paired local refinement of the pre-edge 193-pair candidate completed in 32718: marked 336 edges, added 647 vertices, now 61,513 vertices/123,022 triangles; surface area preserved within tolerance and clean topology. Parent cell/vertex interpolation manifest retained with the candidate. Refinement itself does not eliminate crossings, and fragment-pair counts are not directly comparable across topology changes. Started bounded face-group repair on the refined candidate/reference pair. Neither branch adopted.

Refined-candidate binding session 40871 exited 0: 61,513 bindings, including all 647 added vertices, 1,127 reprojected and 60,386 preserved; original physical shell unchanged. Added attachment fades interpolated through validated parent-vertex provenance. Maximum shell reprojection distance 5.953954569744546 mm, not source vertex displacement. Refined repair session 94953 remains live; no adoption or correspondence claim for restored pre-refinement film checkpoints.

Refined normal transport session 11762 passed and preserved geometry: 61,513 vertices/123,022 triangles, source-authored normal/frame interpolation via parent correspondence. Rest rotation error 2.220446049250313e-16, maximum normal change 164.86526783784922 degrees; 22 opposed vertex normals versus 10 on the lower-resolution baseline cannot be treated as a directly comparable defect count without area/local review. Started refined frontal-face visual check. Refined repair session 94953 remains live.

Refined frontal face capture 25833 exited 0 on Metal and was inspected: gross face preserved, hair raster artifacts persist; actual runtime loaded 61,513 vertices and matching bindings. Proof body-current-residual-refined-visual-proof.json. Refinement changes automatic rendered mouth-seal triangles 340 to 348; this is not proof of internal anatomical validity. Refined repair 94953 remains live.

Refinement liquid-volume map session 68424 passed full model geometry coverage: 121,728 source to 123,022 target triangles, maximum parent chart area error 2.0777823905859805e-13. Synthetic nonuniform/zero extensive-volume forward/reverse transfer verified source-adjusted mass and per-source-cell roundtrip below 1e-12 with nonnegative outputs. This is a render-triangle material map, not yet native film canonical-cell checkpoint integration or adopted-source correspondence. Refined repair 94953 remains live.

Added native FilmPreview refinement/seam regression: one parent cell to two child cells with a duplicated OBJ midpoint, verifies canonical seam welding, mass/source-history preservation and reverse coarsening without mutating original film. Release test session 30403 is compiling; not yet a passing check. Refined geometry repair 94953 remains live.

Native refinement regression session 30403 compiled but failed before remapping: synthetic 2 m patch radius exceeds preview settings bounds. Corrected fixture to a 1 cm triangle and 2 cm patch radius, preserving real application bounds; rerunning focused test. Refined repair 94953 remains live.

Corrected native refinement/seam regression 16529 passed: two child canonical cells, four welded solver points, positive running source input retained, forward/reverse mass preserved and original film unchanged. Proof body-current-refinement-film-map-proof.json. Started existing body-remap compatibility suite; full refined body checkpoint application remains unverified. Refined repair 94953 remains live.

Native body-remap compatibility suite 68254 passed 4/4, including source counters, canonical refinement/seam case and existing actual-solver body correspondences. Observed mass errors remain approximately 1e-14 or below. Existing reference-map assets are not the newly refined current anatomy, so this does not prove full candidate checkpoint application. Refined repair 94953 remains live.

Added opt-in full generated-candidate FilmPreview check current_refined_body_transfer_preserves_wet_foot_source_history. Uses exact current post-ring/refined OBJ assets and refinement distribution, wet right-foot patch and positive running source; verifies increased canonical cell count, source/mass preservation, subsequent solver step and unchanged donor state. Test session 53468 is compiling; no passing result yet. Default test suite excludes this generated-unadopted-asset check. Refined repair 94953 remains live.

Opt-in full current-candidate native transfer test 53468 passed: 121,728 to 123,022 canonical film cells, wet right-foot patch mass relative error 9.48676881951278e-16, positive running source history preserved, next solver step and source increment verified, donor unchanged. This uses raw static OBJ surfaces, not posed FemaleDemo mouth edits or installation of a saved viewer checkpoint. Proof body-current-refinement-film-map-proof.json. Refined geometry repair 94953 remains live.
