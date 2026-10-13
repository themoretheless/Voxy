# План оптимизации структуры и архитектуры Voxy

Дата: 2026-10-11, обновлено 2026-10-12. Статус: выполняется на ветке
`claude/practical-cori-0uhz8k`.

| Шаг | Статус | Commit / результат |
|---|---|---|
| A1 граф-тест | сделано | `crates/voxy_depgraph`: 4 теста, 4 известных нарушения + 1 dev-цикл в ratchet-списке |
| A2 lint-политика | сделано | workspace `unsafe_code = "deny"`, 6 crates на `[lints] workspace = true` |
| A3 core-примитивы | сделано | `voxy_core::{PADDED_EDGE, PADDED_VOLUME, FastHasher, FastHashBuilder, SeededFastHashBuilder}`; хэши байт-идентичны |
| A4 thiserror | сделано | 50 error-типов в world/storage/lighting/mesher/runtime/scene/gpu_core/render/gpu на `#[derive(thiserror::Error)]` с тем же литералом на уровне enum — сообщения байт-идентичны; `voxy_core` остаётся без зависимостей |
| C6 тесты из god files | сделано | liquid_runtime 6698→2430, angular_sweep 2810→1488, animation/lib 4200→1816, render/scene 3965→3004, app/main 2389→2123, editor/lib 7254→5183 строк |
| C1 `#[path]` в examples | сделано | 0 includes; 24 модуля `pub mod`, 247 `pub(crate)`→`pub` |
| D1 render: CPU-геометрия отдельно | сделано | `scene/geometry.rs` (390 строк, без wgpu), `scene.rs` 3004→2620 |
| D2 dev-цикл render↔editor | сделано | `asset_window` переехал в `voxy_editor/examples`; список dev-циклов пуст |
| C5 общие утилиты | частично | orbit-константы общие (`camera_motion::{ORBIT_PITCH_LIMIT, ORBIT_MAX_DISTANCE, zoom_range}`); `ui_text` в editor уже строится на gameplay (не дубль); KeyCode-маппинги семантически разные (не дубль); JSON-helpers остаются на C3 |
| E документация | частично | §4 перегенерирован из `cargo metadata`, ADR 0004 |
| C2 `voxy_demos` | сделано | 34 демо-модуля (≈45k строк) в `crates/voxy_demos`; `voxy_app` = shell (`scene_app`, `scene_rush`, controls, `main.rs`); `voxy_ray_probe` зависит от `voxy_demos`; шаг C2b: shell (`SceneApp`, scene input, touch, rush) вынесен в `voxy_shell`; `voxy_mobile` и examples зависят от него, `voxy_app` — только бинарь; нарушение `voxy_mobile -> voxy_app` снято |
| D3 `voxy_gpu_core` | сделано | `ComputeError`, `ComputeProgram`, memory budget, readback pool, `GraphicsOptions`, `GpuIndirectMesh` живут ниже render; `voxy_gpu` больше не зависит от `voxy_render` (ratchet: нарушение снято) |
| D5 render↔animation | сделано | glTF-импорт (`model.rs`, 1619 строк) вынесен в crate `voxy_model` над `voxy_render` и `voxy_animation`; `voxy_render` больше не зависит от `voxy_animation` (0 ссылок), бюджет суставов — `voxy_render::MAX_SKIN_JOINTS`, равенство с `voxy_animation::MAX_JOINTS` проверяет тест в `voxy_model`; white-box тесты скиннинга и GPU-нормалей переехали в `voxy_model/tests` и ходят к внутренностям render через `#[doc(hidden)] pub` (`voxy_render::test_support`, 12 полей/методов); потребители (editor, app, demos) импортируют `Model*` из `voxy_model`; `KNOWN_VIOLATIONS` ratchet-списка пуст |
| B1 `physics::math` | сделано (идентичные копии) | 74 локальных `dot/cross/add/sub/scale/norm` в 37 файлах заменены импортами `crate::math`; взяты только копии с байт-идентичным телом (проверено нормализацией тела и unit-тестом), поэтому результаты не меняются; оставлены отличающиеся версии (unrolled-FMA `dot` в biomechanics, `dot(a,a).sqrt()`-нормы, `&[f64]`-варианты) |
| B2 слоение physics (часть) | сделано | 19 `astrophysics_*.rs` → `astrophysics/`, 4 `gravity*.rs` → `gravity/`; старые пути сохранены фасадами `pub use astrophysics::x as astrophysics_x` (0 правок у потребителей) |
| C4 editor `App` | сделано (механически) | 50 методов `impl App` разнесены по `src/app/{authoring,inspector,audio_control,viewport,picking_drag,game_ui,play_mode,residency}.rs` с теми же телами; `lib.rs` 5183→1740 строк; тесты editor 182/182 |
| B4 hair contact | сделано | тесты вынесены (3929→2057 строк), затем решатель разрезан по фазам с неизменными телами: `contact.rs` (750 строк: меш, Clearance, фасад) + `contact/{closest,strand_responses,mesh_contacts,grid,strand_contacts}.rs`; physics lib 369/369 (220 hair-contact тестов), 13 hair/strand/contact integration-бинарей 150/150 |
| D4 контракт бэкендов | сделано | `voxy_gpu_core::GravitySolver` (`step`/`read`/`body_count`, скаляр f32 или f64) и общие POD `GravityBody<T>`/`GravityParameters<T>` — `voxy_gpu::GravityBody` и `voxy_cuda::CudaGravityBody` стали псевдонимами; адаптеры: `voxy_gpu::BlockingGravityJob` (encode + submit + ожидание readback) и `impl GravitySolver for CudaGravityJob`; один reference-тест `compare_with_oracle` «бэкенд == CPU f64 `physics::gravity`» — `voxy_gpu/tests/gravity_reference.rs` (lavapipe, 257 тел, 4×32 шагов, допуск 2e-4) и `voxy_cuda/tests/gravity_reference.rs` (под `feature = "cuda"`, допуск 1e-9); `voxy_vulkan` остаётся graphics-interop обёрткой над CUDA-job, water/terrain/voxel_regions есть не во всех бэкендах — вне контракта |
| B3 фасад physics | сделано | карта модулей в `physics/src/lib.rs`; `tests/support/{rigid,fluid,film}.rs` для трёх байт-идентичных групп fixtures (7 файлов переведены, 57 тестов); остальные одноимённые fixtures (`fluid` ×33, `body` ×27, `film` ×14, `config`, `specimen`, `model`) имеют разные тела — измерено, не дубли |
| C3 ошибки app-слоя | сделано | `voxy_shell::RushError`, `voxy_editor::EditorError` (27 типизированных источников через `#[from]`), `voxy_gameplay::GameplayError` (9), `voxy_demos::DemoError` (6) + вариант `Message(String)` для литеральных сообщений; все 161 публичных `Result<_, String>` переведены, текст Display не изменился; `String` остаётся только там, где его требует контракт библиотеки (reader-замыкания `voxy_assets`, `LiquidGeometry::Error`) и в composition root — 16 сайтов `to_string` на этих границах |

После B1: `cargo test -p physics --lib` 369, все 217 integration-бинарей physics — 1551 тест, 0 падений.

Полный прогон тестов `voxy_app` lib на ветке: 343 теста, все прошли.
`female_demo` (63 теста) гонялся по одному; три теста не укладываются в
debug-профиль на 4 vCPU (один >2 ч) и прошли в `--release` (9 мин, 78 с), как
и документирует `docs/body-development-status.md` для тяжёлых тестов.

Цель плана: привести фактическую структуру workspace к инвариантам
`docs/architecture.md` §3–4 (явное направление зависимостей, composition root
без доменной логики, derived остаётся derived), убрать системное дублирование и
разрезать «god files» так, чтобы каждый шаг был проверяем существующими
тестами, `cargo clippy --workspace --all-targets -- -D warnings` и
`cargo fmt --all --check`. Поведение движка не меняется; каждый шаг — чистый
рефакторинг с oracle-тестом «до == после» там, где есть численный выход.

## 0. Что показал обзор (факты)

Числа — из `wc -l`, `grep -c` и чтения `Cargo.toml` на ветке
`claude/practical-cori-0uhz8k`, commit `8f4759a5`.

### 0.1 Граф зависимостей расходится с документом

`docs/architecture.md` §4 описывает `app -> runtime/render`,
`render -> mesher/core`. Фактически:

| Факт | Где | Нарушаемый инвариант |
|---|---|---|
| `voxy_app` тянет 13 внутренних crates (editor, gameplay, rush, cuda, gpu, …) | `crates/voxy_app/Cargo.toml` | §4: composition root «не должен знать canonical domain logic» |
| `voxy_ray_probe` и `voxy_mobile` зависят от `voxy_app` как от библиотеки | `crates/voxy_ray_probe/Cargo.toml` (`face-demo = ["dep:voxy_app"]`), `crates/voxy_mobile/Cargo.toml` | composition root используется как библиотека |
| `voxy_gpu` зависит от `voxy_render` (GPU «над» рендером) | `crates/voxy_gpu/Cargo.toml` | §3.9 explicit dependency direction |
| `voxy_render` dev-зависит от `voxy_editor`, который зависит от `voxy_render` | `crates/voxy_render/Cargo.toml` | dev-цикл: тесты render собирают editor |
| `voxy_render` зависит от `voxy_animation` (concrete types, без trait-границы) | `crates/voxy_render/Cargo.toml` | render знает про анимацию |
| 9 examples `voxy_app` включают 24 файла `src/` через `#[path]`; 16 файлов компилируются по 3 раза | `crates/voxy_app/examples/{female_render,hair_render,skin_render,...}.rs` | derived-код вместо библиотечного API; это же источник всех `dead_code`/`duplicate_mod` allow в commit `8f4759a5` |

### 0.2 God files (производственный код + тесты в одном файле)

| Файл | Строк | Что смешано |
|---|---|---|
| `crates/voxy_editor/src/lib.rs` | 7 254 | `struct App` (56 полей), `impl App` ~4 040 строк / 65 методов: lifecycle, GPU residency, game UI, play mode, viewport, picking/drag, authoring persistence, panels, audio, smoke tests; плюс 2 070 строк тестов |
| `crates/voxy_gameplay/src/liquid_runtime.rs` | 6 698 | runtime + 4 270 строк тестов (64 %) |
| `crates/voxy_animation/src/lib.rs` | 4 200 | Skeleton/Clip/Pose/Animator + 2 180 строк тестов (57 %) |
| `crates/voxy_render/src/scene.rs` | 4 005 | CPU-геометрия (SceneMesh, normals, weld, hasher) + `impl SceneRenderer` 1 925 строк wgpu + 1 000 строк тестов |
| `crates/physics/src/hair/contact.rs` | 3 929 | контактный решатель; ещё 5 файлов `hair/contact_*` по 1.6–2.3k строк |
| `crates/physics/src/biomechanics/inertia/implicit.rs` | 3 238 | implicit integrator |
| `crates/voxy_gameplay/src/angular_sweep.rs` | 2 810 | код до строки 1 458, затем 11 test-модулей |
| `crates/voxy_app/src/main.rs` | 2 389 | воксельная игра целиком + inline tests |

### 0.3 Дублирование (DRY)

| Что | Где | Масштаб |
|---|---|---|
| Векторная математика на `[f64; 3]` руками | `crates/physics/src/**` | `fn dot` ×25, `fn cross` ×20, `fn solve` ×30, `fn sub` ×19, `fn add` ×18, `fn norm` ×10; `hair/math.rs` — единственный общий модуль, локальный для hair; glam/nalgebra в physics не используются |
| Константы padded-объёма 34³ и `fn index` | `voxy_lighting/src/lib.rs:11-37`, `voxy_gpu/src/lighting.rs:7`, `voxy_gpu/src/voxel_mesh.rs:8`, `voxy_mesher/src/mesh.rs:272` | 4 копии; `voxy_core` их не экспортирует |
| Integer-mix хэшеры | `voxy_render/src/scene.rs:2973-3013` (WeldHasher), `voxy_scene/src/lib.rs:177-184` (FastHasher), `voxy_world/src/terrain.rs:134` | 3 копии одного FxHash-подобного миксера |
| Orbit-камера с clamp pitch | `voxy_app/src/camera_motion.rs:24`, `voxy_app/src/female_demo.rs:1191`, `voxy_editor/src/camera.rs:105`, `voxy_app/examples/gpu_gravity.rs:22` | 4 реализации, плюс `controls.rs::CameraRig` и `voxy_render/src/camera.rs::SceneCamera` |
| Ad-hoc `to_json/from_json` на `serde_json::Value` | `voxy_app/src/{film_settings,face_parameters,body_parameters,view_settings}.rs`, `voxy_gameplay/src/audio_assets.rs`, `voxy_scene/src/document.rs`, `voxy_assets/src/manifest.rs` | 7 мест, 3 разных типа ошибок (`&'static str`, `String`, `Box<dyn Error>`) |
| UI text preparation | `voxy_gameplay/src/ui_text.rs` (121), `voxy_editor/src/ui_text.rs` (414) | две параллельные реализации |
| KeyCode -> control | `voxy_editor/src/lib.rs:101`, `voxy_app/src/controls.rs:51` | 2 |
| GPU-бэкенды без общего контракта | `voxy_gpu`, `voxy_cuda`, `voxy_vulkan`, `voxy_streamline` | gravity/water/terrain/voxel_regions реализованы отдельно в каждом; D3D12-interop трижды |

### 0.4 Ошибки и API

- `thiserror` не используется нигде; каждый error-enum написан руками с
  ручным `Display` (`architecture.md` §4 явно разрешает `thiserror`).
- Библиотечные crates используют типизированные enum. App-слой смешивает
  `Box<dyn Error>` (editor 162, app 99) со `String`/`&'static str`
  (editor 106, app 117, gameplay 52).
- `physics`: 1 339 `pub fn`, единственный re-export — `pub use character::*`;
  64 плоских файла в `src/`, из них 16 `astrophysics_*`; `biomechanics/mod.rs`
  объявляет 35 подмодулей. Внутренние зависимости не слоёные:
  `liquid -> astrophysics_spin` (10 ссылок), `rigid_motion -> astrophysics_spin`
  (15), `biomechanics -> lymph` (72), `plasticity -> moisture` (62).
- `physics/tests/`: ~230 файлов, 71 653 строки; общие fixtures только в
  `tests/support/cohesive.rs` (используют 3 файла).
- Три crates (`voxy_ray_probe`, `voxy_cuda`, `voxy_xr`) держат собственную
  таблицу lints, потому что workspace ставит `unsafe_code = "forbid"`, а им нужен
  `deny` с локальными `allow`; allow-список clippy в них дублируется с
  workspace (`Cargo.toml` и `crates/*/Cargo.toml`, commit `8f4759a5`).

## 1. Принципы плана

1. **Один шаг — один PR**, зелёный по трём воротам (`fmt`, `clippy -D warnings`,
   `cargo test` затронутых crates) и с oracle-тестом там, где есть численный
   выход (как `invalidated_chunks_rebuild_equals_full_rebuild` в
   `crates/voxy_runtime/src/bootstrap.rs`).
2. **Сначала швы, потом переезды.** Выделять модуль/crate только после того, как
   его API зафиксирован тестом; не вводить trait «ради интерфейса»
   (`architecture.md` §4: traits только на реально заменяемых границах).
3. **Направление зависимостей проверяется crate graph**, а не договорённостями:
   каждый шаг заканчивается обновлением mermaid-диаграммы §4 и добавлением
   теста на граф (см. шаг 2.1).
4. **Не трогать численные ядра ради стиля.** `hair/contact*`,
   `inertia/implicit.rs` режутся только по уже существующим швам (отдельные
   фазы решателя), с байт-идентичным результатом на reference-наборах.
5. **Производительность не регрессирует**: парные бенчи из
   `docs/architecture-performance/2026-10-09-engine-hotspots/` остаются в CI
   как smoke-набор; шаг, меняющий hot path, обязан приложить paired-run.

## 2. Фазы

### Фаза A — фундамент и измерение (1–2 PR, низкий риск)

**A1. Граф зависимостей как тест.** Добавить `tools/depgraph` (или тест в
`voxy_core`) который читает `cargo metadata` и проверяет allow-list рёбер из
`docs/architecture.md` §4. Сначала фиксирует текущее состояние (с явным списком
«известных нарушений»), дальше каждый шаг плана уменьшает этот список.
Критерий: тест в CI, список нарушений = 0.1 из этого документа.

**A2. Lint-политика без дублирования.** Заменить `unsafe_code = "forbid"` в
workspace на `deny` (семантика для crates без `unsafe` не меняется: `deny`
так же ломает сборку), перевести `voxy_ray_probe`, `voxy_cuda`, `voxy_xr`,
`voxy_vulkan`, `voxy_streamline`, `voxy_mobile` на `[lints] workspace = true`
и удалить мирроринг allow-списка из их `Cargo.toml`. Затем снимать allow-ы по
одному crate за PR (начать с `voxy_world`, `voxy_mesher`, `voxy_lighting`,
`voxy_core` — там код уже чистый).

**A3. Общие примитивы в `voxy_core`.** Вынести `PADDED_EDGE/PADDED_VOLUME`,
strides и `const fn padded_index` в `voxy_core::chunk_halo`; `FastHasher`/
`WeldHasher`/`terrain::mix` — в `voxy_core::hash::{Mix64, FastHashBuilder}`.
Oracle: существующие тесты lighting/mesher/scene + unit-тест равенства хэшей
старой и новой реализации на 10⁶ случайных ключей (хэш участвует в
детерминизме порядка обхода).

**A4. `thiserror`.** Отклонено после осмотра: в voxel-слое (`voxy_core`,
`voxy_world`, `voxy_lighting`, `voxy_mesher`, `voxy_runtime`) каждый error-enum
имеет ровно один двухстрочный `Display` вида `"invalid chunk: {self:?}"` —
это сознательный стиль, derive его не сокращает, а `voxy_core` и `voxy_world`
потеряли бы статус dependency-free. Для app-слоя унификация типов ошибок
остаётся в C3.

### Фаза B — physics: слой математики и слоение доменов (3–5 PR, средний риск)

**B1. `physics::math`.** Один модуль `physics/src/math/{vec3,mat3,solve}.rs`
с `V3 = [f64; 3]` helpers (`dot`, `cross`, `add`, `sub`, `scale`, `norm`,
`normalize`) и малыми решателями (`solve_3x3`, `solve_symmetric`). Миграция
по одному домену за PR (liquid → contact → biomechanics → plasticity →
astrophysics → остальное), каждый PR удаляет локальные копии. Требование:
численно идентичный результат — порядок операций в helper совпадает с самой
частой локальной версией; домены, где локальная версия отличается порядком
суммирования (меняет последние биты), получают отдельный helper с явным именем
(`dot_kahan`, `dot_pairwise`), а не «общий» с другим результатом.
Oracle: regression-тесты `physics/tests/` (230 файлов) + replay-тесты hair.

**B2. Слоение physics.** Разложить 64 плоских файла в директории по домену:
`astrophysics/` (16 файлов), `gravity/` (4), `surface_film/`, `tissue/`.
Только `mod`-переезды и `pub use`-фасады для старых путей (один PR, затем
PR на удаление фасадов после миграции потребителей). Вынести общую
rotation-математику из `astrophysics_spin` в `physics::rotation`, чтобы
`liquid` и `rigid_motion` не зависели от астрофизики; `biomechanics -> lymph`
и `plasticity -> moisture` зафиксировать как разрешённые рёбра в A1 или
вынести общие типы в `physics::transport`.

**B3. Фасад и тест-fixtures.** `physics/src/lib.rs` получает явные
`pub mod`/`pub use` группы (contact, hair, liquid, biomechanics, tissue) с
`#[doc]`-описанием ответственности, как таблица §4. В `physics/tests/support/`
добавить builders для самых частых fixtures (по grep: box-body, sphere-contact,
film-sample) и перевести на них новые тесты; старые не трогать массово.

**B4. Hair contact по фазам решателя.** `hair/contact.rs` (3 929) и
`hair/contact_*` (5 файлов) → `hair/contact/{assembly,qr,response,safe_motion,
velocity}.rs` с одним `pub(crate)` типом состояния на фазу. Только переезд
функций, без изменения тел; oracle — replay-тесты и `contact_component_fixture_tests`.

### Фаза C — app/editor/gameplay: composition root и god objects (4–6 PR, средний риск)

**C1. Убрать `#[path]`-includes из examples.** Модули, которые 9 examples
включают через `#[path]` (24 файла), становятся `pub mod` библиотеки
`voxy_app` (или уходят в C2). Examples используют `voxy_app::…`. Это удаляет
все `#[allow(dead_code)]`/`duplicate_mod` из commit `8f4759a5`, сокращает
сборку examples втрое (16 файлов × 3) и делает «мертвый код» реальным
сигналом. Критерий: `grep -r '#\[path' crates/voxy_app/examples` пуст.

**C2. Новый crate `voxy_demos` (или `voxy_character`).** Female-* (9 модулей),
tissue/fem/liquid/wear/xray/biomechanics demos, `face_preview`,
`body_parameters`, `face_parameters`, `gpu_hair_solver`, `model_presentation`
переезжают из `voxy_app` в библиотечный crate. `voxy_ray_probe` и
`voxy_mobile` зависят от него, не от `voxy_app`. После этого `voxy_app`
остаётся composition root (`main.rs`, `scene_app`, controls, options) и
убирается из allow-list графа A1.

**C3. Единый error-тип app-слоя.** В `voxy_editor`, `voxy_gameplay`,
`voxy_demos` заменить `Result<_, String>`/`&'static str` (275 мест) на
доменные enum с `thiserror`; `Box<dyn Error>`/`anyhow` остаются только в
`main.rs` (§4: «anyhow разрешён только в binary/composition root»).

**C4. Разрезать `voxy_editor::App`.** `impl App` (65 методов) → сервисы по
уже видимым группам, каждый со своим состоянием и тестами:
`residency.rs` (poll_sources, required_*_assets, synchronize_gpu_residency),
`play_mode.rs` (advance_game, start_standalone, toggle_play, game_key),
`viewport.rs` (draw, split_views, activate_view, view_pointer),
`drag.rs` (pick_*, begin/preview/finish_drag),
`authoring.rs` (authoring_document, restore/save/load, validate_*, commit),
`inspector.rs` (panel_*, field_key, edit_key, expand_model),
`audio_control.rs` (toggle_audio_*, close_*, poll_audio_device).
`App` остаётся тонким агрегатом и `ApplicationHandler`. Тесты (2 070 строк)
переезжают в `tests.rs` рядом с сервисами. Один сервис — один PR.

**C5. Общие утилиты UI/ввода.** `ui_text.rs` из gameplay и editor → один
модуль в `voxy_ui` (`voxy_text` уже есть как зависимость обоих); orbit-камера
(4 копии) → `voxy_render::camera::OrbitController` с параметризованным clamp;
`KeyCode -> control` → `voxy_input::ControlMap`. `to_json/from_json` на
`Value` → trait `voxy_assets::JsonDocument` с единым `JsonError`.

**C6. Тесты вынести из god files.** `liquid_runtime.rs` (4 270 строк
тестов), `angular_sweep.rs` (11 test-модулей), `voxy_animation/src/lib.rs`
(2 180), `voxy_render/src/scene.rs` (1 000), `voxy_app/src/main.rs` →
`<module>/tests.rs` через `#[cfg(test)] mod tests;`. Чисто механический шаг,
но он снижает время инкрементальной сборки и делает размер production-кода
видимым (после него `liquid_runtime.rs` ≈ 2 400 строк, `animation/lib.rs`
≈ 1 800).

### Фаза D — render/scene/gpu: границы (3–4 PR, средне-высокий риск)

**D1. `voxy_render::scene` разрезать на CPU и GPU.** `SceneMesh`,
`SceneVertex`, `smooth_normals`, `NormalCache`, weld → `voxy_render::geometry`
(без `wgpu`-импортов, проверяется clippy `disallowed_types`/тестом на
отсутствие `wgpu::` в модуле); `SceneGeometry/SceneTexture/SceneTransform` и
`impl SceneRenderer` (1 925 строк) → `voxy_render::scene::{resources,renderer,
views}`. Oracle: `random_terrain_with_halos_matches_naive`-подобные тесты
геометрии, тесты `weld_hasher_tests`, memory_tests.

**D2. Разорвать dev-цикл render ↔ editor.** Тесты `voxy_render`, которым нужен
`voxy_editor`, переезжают в `voxy_editor/tests/` или в example-crate
`crates/voxy_render_smoke` (dev-only). После этого `voxy_render` не знает об
editor даже в dev.

**D3. `voxy_gpu` под `voxy_render`, а не над.** Что `voxy_gpu` берёт из
`voxy_render` (upload helpers, device/queue обёртки) — вынести в
`voxy_gpu_core` (только `wgpu` device/queue/buffer utilities, без сцены);
`voxy_render` и `voxy_gpu` зависят от него. Тогда граф: `gpu_core <- render`,
`gpu_core <- gpu`, `render` не знает про `gpu`. Критерий — тест A1.

**D4. Контракт вычислительных бэкендов.** Для доменов, реализованных во всех
трёх бэкендах (gravity, water, terrain, voxel_regions), ввести один trait на
домен в `voxy_gpu_core` (`GravitySolver { fn step(&mut self, input) -> Output }`)
и общий reference-тест «CPU-oracle == backend» с одинаковыми входами. Это
единственная trait-граница, которую план вводит, и она — реально заменяемая
(§4). D3D12-interop (три копии) → `voxy_gpu_core::interop::d3d12`.

**D5. `voxy_render` и `voxy_animation`.** Render нужны только `Pose`/
палитра матриц. Ввести `voxy_animation::SkinPalette` (POD) и сделать его
единственным типом, который render импортирует из animation; остальное
(`Animator`, `Clip`) render не видит. Проверка — grep `voxy_animation::` в
`voxy_render/src` даёт только `SkinPalette`.

### Фаза E — документация и закрепление (1 PR)

- Обновить `docs/architecture.md` §4: mermaid-граф со всеми crates (сейчас 7
  из 32), таблица ответственности для `voxy_scene`, `voxy_gameplay`,
  `voxy_editor`, `voxy_demos`, `voxy_gpu_core`, `physics`.
- ADR `docs/adr/0004-workspace-layering.md`: решения A1–D5 с обоснованиями и
  списком запрещённых рёбер.
- Тест графа A1 с пустым списком исключений — definition of done всего плана.

## 3. Порядок и оценка

| Порядок | Шаг | Риск | Что разблокирует |
|---|---|---|---|
| 1 | A1 граф-тест | низкий | измеримость всех остальных шагов |
| 2 | A2 lints, A3 core-примитивы, A4 thiserror | низкий | убирает дубли, чистит allow-списки |
| 3 | C6 тесты из god files, C1 `#[path]` | низкий | быстрые сборки, честный dead_code |
| 4 | C2 `voxy_demos`, C3 ошибки | средний | `voxy_app` = composition root; ray_probe/mobile не зависят от app |
| 5 | B1–B3 physics math/слои/фасад | средний | −100+ дублей helpers, слоистый physics |
| 6 | D1, D2 | средний | CPU-геометрия без wgpu, нет dev-цикла |
| 7 | C4 editor App, C5 утилиты | средний | SRP в editor |
| 8 | D3–D5 gpu_core/бэкенды/anim-граница | высокий | граф по §4 |
| 9 | B4 hair contact | высокий | читаемый решатель |
| 10 | E | низкий | закрепление |

Шаги 2–3 можно вести параллельно с 5; 6–8 последовательны (общие файлы render).

## 4. Не-цели

- Переписывать численные алгоритмы, менять точность или порядок суммирования.
- Вводить ECS/новый scene-граф: `voxy_scene` остаётся как есть (slot-based
  ComponentMap уже оптимизирован, см. `2026-10-09-engine-hotspots`).
- Менять форматы persistence (`voxy_storage`, scene documents).
- Снимать все clippy-allow разом: только по crate, когда код в нём чист.

## 5. Как проверять каждый PR

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p <затронутые crates>
cargo test -p voxy_core --test depgraph          # после A1
python3 scratchpad/paired_run.py ...             # если тронут hot path
```

Плюс для шагов с численным выходом (A3, B1, B4, D1, D4): тест «старое ==
новое» на зафиксированном наборе входов, который удаляется в следующем PR
после слияния.
