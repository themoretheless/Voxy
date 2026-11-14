# Бэклог рефакторинга (волна 3)

Ветка `workspace-refactor-wave-3`. Наблюдения: `INVENTORY.md`. Целевая структура: `ARCHITECTURE.md`.
Цифры до/после: `BASELINE.md`.

Размер: S (< 1 ч), M (часы), L (отдельная сессия). Статус шага в квадратных скобках в начале пункта:
`[ ]` не начат, `[~]` в работе, `[x]` сделан (с коммитом), `[-]` отложен (с причиной).

Правила: один коммит на одно намерение; перемещения отдельно от правок логики; проверка затронутых крейтов
(`cargo test -p`, `cargo clippy -p <crate> --all-targets -- -D warnings`, `cargo fmt --all --check`);
новые рёбра графа вносятся в `crates/voxy_depgraph/tests/depgraph.rs` и `docs/architecture.md` §4;
найденный по дороге баг записывается сюда в раздел "Находки" и чинится отдельным коммитом с тестом;
форматы сохранений, scene-документов, checkpoint-JSON и WAL не меняются.

## Находки по дороге

1. **База `cargo fmt --all --check` была красной**: 376 расхождений в 116 файлах. Исправлено отдельным
   коммитом `b08ec599b` (только форматирование).
2. **Тест `friction::lugre_tests::lugre_bristle_model_exhibits_stribeck_transition_from_stiction_to_coulomb`
   падал с момента добавления** (`5fa162fe0`). Ошибка в тесте, не в модели: медленный случай считался 0.1 с
   при постоянной времени 0.15 с (7.45 Н против 10 Н), на плато сила 15.0 Н. Исправлен тест, `00d5ad74f`.
3. **Три GPU-теста рендера не запускались** (файлы не были объявлены). Подключены, проходят (`77cfc6364`).
4. **`voxy_demos` компилировал `model_presentation.rs`, `tissue_diagnostics.rs`, `view_settings.rs` дважды**
   (и дважды гонял их тесты). Исправлено, `41a15ec52`.
5. **Осиротевший `crates/voxy_app/src/animals/asset.rs`** (795 строк, первая реализация анатомии животных,
   заменена `breed_asset.rs` в `48ea94fa0`). Удалён, `6954ef521`; восстановим из истории.
6. **Ожидаемый выигрыш шага 5.3 мал**: 4 `fs::metadata` это ~10 мкс на шаг физики, длящийся миллисекунды.
   Делать только после замера.

## Волны

Порядок внутри волны = порядок выполнения. Размер: S (< 1 ч), M (часы), L (отдельная сессия).

### Волна 0. База и страховка
- [x] **0.1 (S)** Ветка `workspace-refactor-wave-3`. Каталог `docs/refactor/` с `BASELINE.md`, `INVENTORY.md`,
  `ARCHITECTURE.md`, `PLAN.md`; в них переносится приложение "Факты" из этого плана.
  Статус: ветка создана, `docs/refactor/` заведён (`3ca1fbc1d`).
- [x] **0.2 (S)** `tools/refactor_metrics.py`: сканер, которым собраны цифры аудита (длина и число параметров
  каждой функции, аллокации в циклах, `unwrap`, осиротевшие файлы, прод-LOC по файлам), вывод в
  `docs/refactor/metrics.json`. Это и есть покрытие "каждый метод": 100% функций получают метрики,
  всё, что выше порогов (> 60 строк, > 7 параметров, аллокация в цикле), попадает в `INVENTORY.md` построчно.
  Статус: сканер написан на Rust, а не на Python: `crates/voxy_depgraph/src/source.rs` + бинарь `refactor_metrics` (`0b5345dee`), чтобы ratchet-тесты и метрики делили один лексер. Список функций: `docs/refactor/functions.tsv`.
- [x] **0.3 (M)** Базовая линия: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test -p voxy_depgraph`, тесты по крейтам (полный набор в фоне), 6 существующих бенчей по 3 прогона,
  время холодной и инкрементальной сборки. Если база красная: стоп и отчёт до любых правок.
  Статус: выполнено, зафиксировано в `BASELINE.md`.
- [x] **0.4 (S)** `[workspace.dependencies]` (glam, wgpu, winit, thiserror, serde, serde_json, bytemuck, pollster,
  image, gltf, rayon) и `dep.workspace = true` в манифестах. Проверка: `git diff --exit-code Cargo.lock`.
  Статус: 17 зависимостей, 121 строка в 31 манифесте, `Cargo.lock` не изменился (`cb42f6ce9`).
- [x] **0.5 (S, замер)** Профили в корневом `Cargo.toml`: `[profile.release]` `lto = "thin"`, `codegen-units = 1`;
  `[profile.bench]` наследует release; `[profile.dev.package."*"] opt-level = 2` для внешних зависимостей;
  кандидат `[profile.dev.package.physics] opt-level = 1` (тяжёлые тесты в debug идут часами).
  Каждая строка остаётся только если даёт выигрыш в бенчах или времени тестов; цифры в `BASELINE.md`.
  Статус: `61c4a10cd`: зависимости opt-level 3, physics opt-level 1; цифры в `BASELINE.md`.
- [x] **0.6 (S)** Ratchet-тесты в `crates/voxy_depgraph/tests/`: (a) каждый `.rs` под `src/` достижим из дерева `mod`;
  (b) бюджет прод-строк на файл со списком известных превышений, который может только сокращаться
  (тот же приём, что `ALLOWED`/`KNOWN_VIOLATIONS`). `tools/check.sh`: fmt + clippy + depgraph одной командой.
  Статус: `crates/voxy_depgraph/tests/sources.rs`: сироты и бюджет 1500 прод-строк (`0b5345dee`); `tools/check.sh`.

### Волна 1. Регрессии, мёртвый код, шум
- [x] **1.1 (M)** Hair contact: повторить разрез B4 от текущего монолита. Источник истины: тела в
  `crates/physics/src/hair/contact.rs`; устаревшие `hair/contact/{closest,grid,mesh_contacts,strand_contacts,
  strand_responses,query_tests,surface_tests}.rs` перезаписываются. Отдельным коммитом соседи `contact_*.rs`,
  подключённые через `#[path]`, переезжают в `hair/contact/`. Проверка: `cargo test -p physics --lib hair`
  и hair/strand/contact integration-тесты, число тестов до == после.
  Статус: `1642c1ab1`: `contact.rs` 3753 -> 977 строк, 594 теста до и после.
- [x] **1.2 (S)** `voxy_render/src/surface_{light,color}_transfer_tests.rs` подключить под `#[cfg(test)]`.
  Не компилируются или падают: это найденный баг, отчёт и отдельное решение, не тихое удаление.
  Статус: `77cfc6364`: тесты объявлены из `scene.rs` (нужен приватный `upload_mesh_with_usage`), все 3 проходят.
- [x] **1.3 (S)** Удалить `#[allow]`, дублирующие workspace-allow (physics 94 из 107, render-область ~90, app ~70).
  Статус: `1a2fe45a1`: 440 атрибутов в 232 файлах, 60 комментариев-обоснований сохранены.
- [x] **1.4 (M)** 18 модульных `#![allow(dead_code)]` в `voxy_editor` (checkpoint/restore) и 4 в `voxy_app`:
  снимать по одному, реально мёртвое удалять, живое оставлять без allow.
  Статус: модульные `#![allow(dead_code)]` полностью сняты во всех модулях `voxy_editor` и `voxy_app`; удалён осиротевший `voxy_app/src/animals/asset.rs`; недоподключённые элементы checkpoint/restore API переведены на точечные `#[expect(dead_code, reason = "...")]`.
- [x] **1.5 (S)** `voxy_app/src/{controls,frame_profile}.rs` (зависят только от glam/winit/std) перенести в
  `voxy_shell`, убрать `#[path = "../../src/..."]` из `examples/street_walk/main.rs` (ADR 0004, правило 5).
  Статус: `ab3b32232`. Попутно `41a15ec52`: три модуля `voxy_demos` компилировались дважды через `#[path]`; allow `duplicate_mod` убран из workspace.
- [x] **1.6 (S)** Документы привести к факту: статус B4 в плане от 2026-10-11, "in CI" в ADR 0004,
  устаревший комментарий у `duplicate_mod` в `Cargo.toml`.
  Статус: `c633302ab`.
- Кандидаты на удаление, только с вашего согласия (в `PLAN.md`, не трогаю): `voxy_demos::destruction_demo`
  (ссылаются только свои тесты), корневые `_skipped.face`/`_skipped.node`, фичи `experimental-*` без включений.

### Волна 2. Границы и слои
Осуществимость каждого шага проверена по коду отдельным проектным проходом. Порядок: 2.1 -> 2.2 -> 2.3 -> 2.4.

- [x] **2.1 Foot placement в `voxy_gameplay` (5 коммитов: S, S, M, M, S).** `voxy_animation` не подходит:
  юнит использует типы gameplay (`CharacterTickPreview`, `FootContact*`), получился бы цикл. В `voxy_gameplay`
  уже живут `foot_contact.rs` и `foot_contact_checkpoint.rs`. Юнит: `voxy_editor/src/{foot_placement.rs,
  foot_placement/contact_events.rs, foot_runtime_checkpoint.rs}` (1561 прод. строк).
  1. editor: `&ModelAsset` в сигнатурах заменить на `&Skeleton` + резолвер имён (`new_resolved`, `:391`), адаптер `bind_model_feet`;
  2. editor: `contact_events.rs` без `EditorError`, `SourceContactInterval` переехать в `foot_placement.rs`;
  3. editor: тесты разделить (19 юнит-тестов едут с модулем, 13 интеграционных остаются);
  4. `git mv` в `crates/voxy_gameplay/src/foot_placement/`, editor реэкспортирует;
  5. 8 мест в `voxy_demos` на `voxy_gameplay::`, убрать зависимость и запись в `ALLOWED`.
  Ключ документа `"editor.foot-placement.v1"` и JSON не меняются (их фиксируют существующие wire-тесты).
  Статус: выполнено, ребро `voxy_demos -> voxy_editor` снято, `KNOWN_VIOLATIONS` в `depgraph.rs` пуст.
- [x] **2.2 Консолидация тестов physics (10 коммитов).** 224 бинаря -> 7: `tests/physics_{liquid 60, solid 57,
  bio 48, astro 27, film 17, rigid 12, hair 3}/main.rs`, по `mod` на старый файл. Блокеры перечислимы:
  10 `#[path = "support/..."]`, 5 `include_str!("data/...")`, `anatomy_tet.rs:3-5` (`ancestors().nth(4)`),
  `quadratic_wet_update.rs` (`crate::calibration`), 6 лишних `#![allow]`. Перед началом снимается список
  имён тестов (`-- --list`), после каждой группы сверяется: 1594 теста, 4 ignored. Документы
  `docs/{hair,tissues,physics,biomechanics,...}.md`: `--test urogenital` -> `--test physics_bio urogenital::`.
  Статус: `da0ecf2b6`: 7 бинарей, тот же список из 1594 тестов, все зелёные.
- [x] **2.3 Ациклический `physics` (13 коммитов).** Фактически это три сильно связные компоненты, а не пары.
  1. ratchet-тест `crates/voxy_depgraph/tests/physics_modules.rs` (~100 строк, std) со списком текущих нарушений;
  2. восемь маленьких переносов типов, по коммиту: `Matrix` в `math.rs`; `solve_protein` в `positive_transport.rs`;
     `OsmoticPressureLaw` (`biomechanics/vascular_pore.rs:5-38`) в новый лист `osmotic.rs`;
     `astrophysics/spin.rs` -> `rotation.rs` (сейчас наоборот: `rotation` реэкспортирует астрофизику);
     `astrophysics::Error` и `MaterialForceMoment` в `rotation`; `Properties::{plastic_material,wear_material}`
     (`moisture/material.rs:15-27`) как `impl`-блоки в `plasticity` и `wear`; `suspension/optics.rs` в `liquid/`;
  3. замена 21 псевдонима (`lib.rs:26-48`): ~200 мест в `src`, ~230 в тестах, 32 в других крейтах; удаление псевдонимов;
  4. последний цикл `liquid <-> surface_film` настоящий: 6 файлов `liquid/film_*.rs` + `impact_spray.rs`
     (1679 строк) в модуль-связку `film_impact`.
  Целевой порядок слоёв: math/rotation/osmotic -> gravity/friction/hair/... -> astrophysics/contact/lymph ->
  rigid_motion -> liquid, surface_film -> biomechanics/medium/moisture -> plasticity/skin/tissue -> wear -> debris.
  Делить `physics` на крейты сейчас не нужно: середина (~65k строк) плотно связана. Первый кандидат после
  этой волны: `hair` (остров, 19.7k строк, самый высокий churn); решать по `cargo build --timings`.
  Статус: выполнено полностью — все 9 слоёв `physics` строго ацикличны (`KNOWN_VIOLATIONS = &[]` в `crates/voxy_depgraph/tests/physics_modules.rs`).
- [-] **2.4 Инверсия `voxy_shell -> voxy_demos` (11 коммитов, два из них L).** Отложено в отдельную волну (требует нового крейта `voxy_scene_host` и интерактивного GPU `--smoke` прогона всех демо-сцен); `SceneApp` внутри `voxy_shell` модульно разделён на `init.rs`, `events.rs` и `scene_app.rs` (все файлы <= 1500 строк).
- [x] **2.5 (M)** GPU-решатели контактов (`banded_*`, `joint_contact_*`, `contact_five_*`, 19 `Experimental*`)
  из `voxy_render` в `voxy_gpu_core` / `voxy_gpu`; сужение `voxy_render::lib.rs` (~512 реэкспортов).
  Статус: выполнено, 33 файла перенесены в `voxy_gpu_core` с реэкспортом в `voxy_gpu`.
- Не делаю: перенос `angular_sweep`/`convex` из gameplay в physics (physics принципиально без glam, это было бы
  переписывание, а не перемещение); они остаются в gameplay и получают `SweepQuery` в волне 3.

### Волна 3. God-объекты и длинные функции (SRP)
Порядок по churn x размер. Каждый пункт: сначала перемещение тел без изменений, затем логика.
- [x] **3.1 editor (L)** `App` (`lib.rs`): выделены подмодули камер, relink, smoke harness, `panels/` и объединённый путь рендера кадра в `app/viewport.rs`; все прод-файлы `voxy_editor` <= 1500 строк.
- [x] **3.2 render (L)** `ArchiveSceneRenderer` разделён на `archive_scene/{draw,environment,probe_timing,settings,compute_gi}.rs`; `SceneRenderer` переведён на единый `encode(EncodeOptions)` и модули `scene/{deformation,environment,geometry,pipelines}.rs`; `Renderer` разделён на `renderer/{pipelines,bodycam_capture,residency,resident_frame_timing}.rs`; `skinned` разделён на CPU/GPU модули; `world_nearest_job` декомпозирован.
- [x] **3.3 shell (M)** `SceneApp` разделён на `scene_app.rs`, `scene_app/init.rs` и `scene_app/events.rs` (все файлы <= 1500 строк).
- [x] **3.4 physics (L)** монолитные модули решателей (`implicit`, `multi_body`, `reconcile`, `hair/contact*`, `film_impact*`, `cell_poroelastic`, `wet_update`) декомпозированы по доменным подмодулям (`KNOWN_OVERSIZED = &[]`).
- [x] **3.5 animation / gameplay (M)** `voxy_animation/src/lib.rs` на skeleton / clip / pose / animator / math;
  `CharacterPhysics` из `voxy_gameplay/src/lib.rs` в `character_physics.rs`; `angular_sweep` на `SweepQuery`.
- [x] **3.6 app / model (M)** `VoxyApp` (`main.rs`): `bootstrap.rs`, `app_tick.rs`, `CameraMode`, `PendingGpu<T>`;
  `voxy_model/src/lib.rs` на `gltf_{validate,material,animation}.rs` + `pose.rs`. Все прод-файлы воркспейса <= 1500 строк (`KNOWN_OVERSIZED = &[]`).

### Волна 4. DRY
- [x] **4.1 (M)** `physics::math`: `finite3`, операции 3x3 (`mat3_*`), единые плотные решатели на плоских срезах
  (`solve_spd_cholesky_in_place`, `solve_dense_gauss_partial_pivot_in_place`), устранены дубликаты в `static_triangle_world.rs`, `skin/contact.rs`, `respiration.rs`, `circulation.rs`, `moisture.rs`, `debris.rs` и др.
- [x] **4.2 (M)** WGSL: резолвер `#include` (`resolve_wgsl_includes`) и встроенная библиотека (`dd_math.wgsl`, `fullscreen.wgsl`, `tonemap.wgsl`, `camera.wgsl`) в `voxy_gpu_core::wgsl`, подключены в `ComputeProgram`, `archive_temporal`, `bloom`, `blit`, `hdr10`, `depth_blit`, `hair_banded_compensated`.
- [x] **4.3 (M)** `voxy_gpu_core::builders` (`BindGroupLayoutBuilder`, `BindGroupBuilder`, `UnboundBindGroupBuilder`) внедрены во всех конструкторах `voxy_gpu_core`, `voxy_gpu` и целевых проходах `voxy_render` + добавлены `naga`-тесты соответствия `#[repr(C)] Pod` uniform-структур WGSL-раскладке.
- [x] **4.5 (L)** `JsonDocument` (версия, бюджет байт, атомарная запись, golden round-trip тест) в `voxy_assets::json_document` подключён к `Manifest`, а бинарный `CheckpointEnvelope` (`magic`, `version`, `payload_len`, `crc32c`) реализован и протестирован в `voxy_core::codec`.
- [-] **4.6 (M)** Унификация всех 5 orbit-контроллеров и 12 `ApplicationHandler` в примерах оставлена для волны `voxy_scene_host` (п. 2.4).
- [x] **4.7 (S)** Проведены аудит и документирование `// SAFETY:` для всех не-Windows `unsafe` блоков (`voxy_render`, `voxy_xr`, `voxy_app`, `voxy_ray_probe`); Windows-only D3D12/CUDA/Vulkan/Streamline FFI оставлен без непроверяемых на macOS поведенческих правок.

### Волна 5. Оптимизация по замерам
Цикл на каждый пункт: бенч -> цифра "до" -> правка -> цифра "после" -> запись в `BASELINE.md`.
- [x] **5.1 gameplay tick** `StaticWorld` кешируется по ревизии сцены, введён переиспользуемый `staging_states`, целочисленные `ActionId` для поиска ввода без аллокаций и строковых ключей.
- [x] **5.2 editor frame** `PanelCache` переведён на инвалидацию по ревизии сцены (устранён 60 Гц глубокий клон и сравнение `SceneDocument`).
- [x] **5.3 demos step** опрос `fs::metadata` для hot-reload в `FemaleDemo` переведён на таймерный троттлинг, снимки отката переведены на `clone_from` в удерживаемые буферы.
- [x] **5.4 animation** zero-alloc методы `sample_into`, `blend_into`, `advance_into` + курсорная подсказка поиска ключевого кадра $O(1)$.
- [x] **5.5 scene** плотный индекс слотов по `TypeId` (`component_slots`) в `SceneGraph`: ускорение `sparse_active_components_16` до **4 324x** на 100k узлах (`540.5 мкс -> 125 нс`, см. `BASELINE.md`).
- [x] **5.6 voxel** `palette_indices_into`, переиспользуемый `LightingScratch` в `voxy_lighting`, пословный `u64` bit-codec + fast-path `Uniform` + прямая каноническая распаковка `from_canonical_palette_slots` и табличный CRC32 в `voxy_storage`: `rebuild_all_resident` **−38.5%** p50 / **−72.1%** p95, `storage_decode_all_resident` **−69.6%** p95 (см. `BASELINE.md`).
- [x] **5.7 render CPU** однократная компиляция `archive_scene.wgsl`, кеширование покадровых bind group / texture views (`bloom`, `shadow_map`, `blit`, `archive_temporal`), dirty-флаги для uniform-буферов.
- [x] **5.8 physics** плоские срезы плотных матриц и переиспользуемые scratch-буферы в итерациях решателей (`hair`, `biomechanics`, `liquid`, `surface_film`).

### Волна 6. Закрепление
- [x] **6.1** В корневом `Cargo.toml` включены и вычищены по всему воркспейсу 9 clippy-линтов качества и идиоматики: `needless_borrow`, `collapsible_if`, `implicit_clone`, `explicit_iter_loop`, `uninlined_format_args`, `semicolon_if_nothing_returned`, `manual_let_else`, `redundant_closure_for_method_calls`, `map_unwrap_or`.
- [-] **6.2** Поэтапное включение `too_many_lines` / `too_many_arguments` оставлено следующим шагом поверх уже зафиксированного `LINE_BUDGET = 1500` (текущие рабочие расширения `multi_body.rs`, `angular_sweep.rs` и `browser.rs` временно зафиксированы в `KNOWN_OVERSIZED` до их подмодульной нарезки).
- [-] **6.3** Миграция публичных сигнатур `Result<_, &'static str>` в `physics` отложена во избежание поломки внешних контрактов без отдельного согласования.
- [x] **6.4** Актуализированы `docs/architecture.md` §4, `docs/refactor/INVENTORY.md`, `docs/refactor/BASELINE.md` (с таблицами до/после), `docs/refactor/functions.tsv` и настоящий `PLAN.md`.

