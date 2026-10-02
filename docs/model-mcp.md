# MCP параметризации модели

Локальный сервер `model_mcp` использует stdio JSON-RPC MCP. Один процесс обслуживает одну импортированную модель `blender-female` и один выбранный JSON-файл. Поддерживаются протоколы 2024-11-05, 2025-03-26 и 2025-06-18. Сервер не требует Python, Node или сетевого порта.

```sh
cargo build --release -p voxy_app --bin model_mcp --example female_motion
```

Пример конфигурации MCP-клиента (пути нужно заменить на свои):

```json
{
  "mcpServers": {
    "voxy-models": {
      "command": "/Users/themoretheless/Documents/ChatGPT/Voxy/target/release/model_mcp",
      "args": ["/Users/themoretheless/Documents/ChatGPT/Voxy/assets/characters/blender-female/presets/live.json"]
    }
  }
}
```

Если файл отсутствует, сервер создаёт его с исходными параметрами. Родительская папка должна существовать. Для просмотра того же файла:

```sh
target/release/examples/female_motion /Users/themoretheless/Documents/ChatGPT/Voxy/assets/characters/blender-female/presets/live.json
```

| Инструмент | Назначение |
| --- | --- |
| model_get | Текущие параметры и путь файла |
| model_schema | Параметры, диапазоны, единицы и исходные значения |
| model_update | Частично изменить параметры |
| model_reset | Вернуть исходные значения |

Аргументы `model_update`:

```json
{"parameters":{"height_cm":182,"weight_kg":78,"breast_size":1.3,"buttock_size":1.25,"leg_length":1.15}}
```

Неуказанные значения сохраняются. Валидация использует тот же Rust API, что визуализатор. Изменение записывается через временный файл и атомарную замену; отклонённый запрос не меняет файл. Визуализатор перечитывает его при следующем продвижении симуляции. Во время паузы применение откладывается до возобновления. Ошибочный внешний JSON сохраняет последнее допустимое состояние. Сервер подтверждает сохранение файла, а не отображение кадра: `liveApplied: null`. Несколько одновременно пишущих серверов для одного файла не поддерживаются. Это файловый мост, не удалённое управление произвольными моделями или экспорт меша.

Конфигурация приведена как пример; сервер автоматически не устанавливается в настройки клиента. Подробности параметризации: [body-parameters.md](body-parameters.md).

Протокол: https://modelcontextprotocol.io/specification/2025-06-18/basic/transports и https://modelcontextprotocol.io/specification/2025-06-18/server/tools.

## Подтверждение представленного кадра

`model_status` читает `<preset>.presented.json`, который создаётся визуализатором после `RenderOutcome::Presented`, только если объект активен. MCP-сохранение не создаёт подтверждение. `model_get` также возвращает поле `presentation`.

`liveApplied: true` означает: параметры свежего подтверждения совпадают с текущим JSON. `false` — кадр подтверждён недавно, но с другими параметрами. `null` — подтверждения нет, оно повреждено или старше пяти секунд. В записи есть ID кадра, время симуляции, размер поверхности и PID визуализатора. Запись обновляется не чаще двух раз в секунду.

Это подтверждение представления кадра рендерером, не свидетельство физического вывода на монитор и не снимок изображения. Пауза/минимизация/закрытие окна могут привести к устареванию записи. Сохранённое испытание настоящего Metal-визуализатора: `docs/body-presentation-live-proof.json` — изменение роста через MCP, получение соответствующего `Presented` и восстановление исходных параметров. Один JSON предназначен для одного активного визуализатора; несколько пишущих экземпляров не поддерживаются.

`model_snapshot` renders the current saved body parameters to a 576×768 PNG
using the sibling `examples/female_render` executable. Build it alongside
`model_mcp` with `cargo build -p voxy_app --release --bin model_mcp --example female_render`.
The tool freezes parameters into a unique temporary directory, renders pose time
zero, and returns a local PNG resource link and the exact parameters used.
It has a 60-second deadline and terminates only its owned rendering process.
This is an offscreen render, not a screenshot of the live viewer. Files remain in
the returned directory for inspection; `model_status` remains the separate live
presentation acknowledgement.

`model_measurements` reads the simulation measurements attached to the latest
presented-frame receipt: frame, simulation time, exact applied body parameters,
freshness/age, fluid volume (m³), mass (kg), maximum cell thickness (m), and four
coarse tissue-cage volumes (m³). These cage volumes are not anatomical organ
measurements. Fluid data is null when film simulation is disabled. No receipt
means unavailable measurements, not zero fluid. Stale or mismatched parameter
receipts remain explicitly marked and must not be treated as current data.

`cargo run -p voxy_app --release --example surface_film -- /absolute/path/to/body.json`
starts the neutral torso film demo with live parameters and measurement receipts.
Native Metal + stdio MCP verification returned the configured 0.05 ml / 0.05 g
and a maximum cell thickness of 30.61 micrometres in frame 0. Evidence is in
`model-measurements-live-proof.json`. This verifies reporting of that frame,
not long-run mass conservation or physiological calibration.

`film_get` reads saved settings from the body's `.film.json` sidecar; `film_update`
atomically patches `density` (kg/m³), `viscosity` (Pa·s), `surface_tension` (N/m),
`wetting` (phenomenological m²/s) and `source_rate_m3_s` (m³/s). Bounds are
engineering limits for the preview, not measured physiology. A running enabled
film demo watches the sidecar and applies validated properties. Density edits
rescale cell volumes to conserve existing mass. This is material replacement,
not fluid mixing. Invalid settings preserve the last valid state.

The independent source follows the cells of the initial deposition brush;
zero rate disables it. `source_x_m`, `source_y_m`, `source_z_m` and
`source_radius_m` now edit its canonical-model-space footprint. The footprint
follows selected cells through deformation; it is not a world-fixed emitter.
`model_measurements` returns the applied film settings, initial mass and source
added mass from presented frames. Saving settings is not application proof;
check those measurements. The body-only snapshot tool does not capture dynamic
film state. Multiple simultaneous sources are supported; optical-material controls remain to be implemented.

Native Metal + MCP control verification is saved in `film-control-live-proof.json`.
Frames 0/3/6 acknowledge the original, density=2000 kg/m³ and active source
states. Density doubling halved volume while retaining initial mass. At frame 6,
source-added mass was 4e-7 kg and total mass 5.04e-5 kg, matching initial plus
injected mass. The original underflow rejection found in this integration test
was corrected: mass balance is checked globally within 1e-12 relative tolerance,
allowing negligible dry-cell subnormal rounding while rejecting bulk mass loss.

Source relocation is staged before material replacement, so a footprint missing
the surface leaves both the existing liquid and properties untouched. The saved
file can be numerically valid but geometrically rejected by the viewer; compare
applied settings in `model_measurements` with the saved settings. Native proof:
`film-source-relocation-live-proof.json`, original frame 0 and moved-source frame
1; applied settings match saved settings and source mass is accounted for.

`film_update.settings.sources` replaces up to 16 additional independent sources.
Each requires a unique `id` (ASCII letters/digits/underscore/hyphen, max 64 bytes;
`primary` is reserved), `x_m/y_m/z_m`, `radius_m`, and `rate_m3_s`. The legacy
scalar source remains independent. An empty list disables additional sources
without deleting their deposited liquid. All sources share the configured fluid
material. Invalid geometry in any source rejects the entire application. Source
positions are canonical model coordinates, not world-fixed emitters.

Native evidence: `film-multiple-sources-live-proof.json` confirms the two-source
settings and subsequent disabling in frames 0/3/6. Source-added mass 6e-7 kg and
total mass 5.06e-5 kg match the initial-plus-input balance. These checks do not
provide biological calibration or long-run whole-model validation.

Optical film controls are now `refractive_index` (1..2.5) and
`absorption_r_per_m`, `absorption_g_per_m`, `absorption_b_per_m` (0..10000 m⁻¹).
The shader uses the applied index in dielectric Fresnel and refracted optical
path length; RGB absorption controls Beer-Lambert transmission. They do not
change physical density/viscosity or liquid mass. Values are explicit engineering
inputs, not physiological measurements. `film-optics-live-proof.json` confirms
a runtime change in frames 0/3 with mass retained.

The native offscreen comparison in `film-optics-render-proof.json` records 30384
changed pixels and maximum channel difference 24/255 for a clear index-1 layer
versus an illustrative absorbing index-1.45 layer. Reproduce with
`female_render --film --torso --film-preset /absolute/film.json --snapshot output.png`.
This remains a substrate-colour coating approximation; full scene refraction,
sorted transparency and skin subsurface scattering are still incomplete.

### Film in reproducible snapshots

`model_snapshot` accepts optional `includeFilm: true` (default false). It
copies body and film settings into immutable temporary presets before
launching the offscreen renderer. The response records the exact saved film
settings, fresh-demo state, initial mass (0.05 g), material-adjusted initial
volume, deposit footprint, and `liveLiquidCapture: false`. Optical settings
are applied to the mesh. It does not capture evolving liquid from a live
viewer; sources have not advanced at pose time zero.

Validation: MCP unit test passes; release MCP and renderer build passes.
A real stdio call using IOR 1.45 and RGB absorption produced the inspected
576x768 `model-film-snapshot.png`; metadata is in
`model-film-snapshot-proof.json`. Full-body framing limits film detail.

### Fluid panel

The existing local body panel now exposes film material, optical parameters,
primary-source position/radius/rate, and JSON download. HTTP `/film` GET/POST
uses MCP validation and persistence; `/measurements` exposes receipt data.
The UI converts millimetres and ml/s to SI. Confirmation compares every saved
setting with settings reported in a fresh presented frame, separately from
saving. Freshness failure or disabled film is shown explicitly. Independent
extra-source editing and preset import remain future work.

Browser validation: saved IOR 1.45 and 0.001 ml/s, verified persisted SI rate
1e-9 m3/s; invalid density rejected without modifying saved values. Screenshot:
`body-film-panel.png`; proof: `body-film-panel-proof.json`. No native viewer
was connected in this panel test, so live confirmation was not claimed.

Fluid presets can now be imported from the panel's file picker (64 KiB limit).
Every current setting must be present; MCP rejects unknown or invalid values
and duplicate source IDs atomically. Import restores independent source arrays
as well as physical and optical values. Saving/import errors and renderer
confirmation use separate status lines so polling cannot hide a rejection.
Browser file-picker tests applied a full two-source preset and rejected a
preset with a duplicate ID; persisted settings remained unchanged.

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
