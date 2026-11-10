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
- [ ] **0.3 (M)** Базовая линия: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test -p voxy_depgraph`, тесты по крейтам (полный набор в фоне), 6 существующих бенчей по 3 прогона,
  время холодной и инкрементальной сборки. Если база красная: стоп и отчёт до любых правок.
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
- [~] **1.4 (M)** 18 модульных `#![allow(dead_code)]` в `voxy_editor` (checkpoint/restore) и 4 в `voxy_app`:
  снимать по одному, реально мёртвое удалять, живое оставлять без allow.
  Статус: `6954ef521`: allow снят с 9 модулей editor, где он ничего не скрывал; удалён осиротевший `voxy_app/src/animals/asset.rs`. Остаются 13 модулей с 67 реально неиспользуемыми элементами checkpoint API (prepare / validate_origin / discard / stage_gpu / capture): это недоподключённая функциональность, удалять её без решения владельца нельзя. Следующий шаг: заменить модульный allow на точечные `#[expect(dead_code, reason)]`.
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

- [ ] **2.1 Foot placement в `voxy_gameplay` (5 коммитов: S, S, M, M, S).** `voxy_animation` не подходит:
  юнит использует типы gameplay (`CharacterTickPreview`, `FootContact*`), получился бы цикл. В `voxy_gameplay`
  уже живут `foot_contact.rs` и `foot_contact_checkpoint.rs`. Юнит: `voxy_editor/src/{foot_placement.rs,
  foot_placement/contact_events.rs, foot_runtime_checkpoint.rs}` (1561 прод. строк).
  1. editor: `&ModelAsset` в сигнатурах заменить на `&Skeleton` + резолвер имён (`new_resolved`, `:391`), адаптер `bind_model_feet`;
  2. editor: `contact_events.rs` без `EditorError`, `SourceContactInterval` переехать в `foot_placement.rs`;
  3. editor: тесты разделить (19 юнит-тестов едут с модулем, 13 интеграционных остаются);
  4. `git mv` в `crates/voxy_gameplay/src/foot_placement/`, editor реэкспортирует;
  5. 8 мест в `voxy_demos` на `voxy_gameplay::`, убрать зависимость и запись в `ALLOWED`.
  Ключ документа `"editor.foot-placement.v1"` и JSON не меняются (их фиксируют существующие wire-тесты).
- [x] **2.2 Консолидация тестов physics (10 коммитов).** 224 бинаря -> 7: `tests/physics_{liquid 60, solid 57,
  bio 48, astro 27, film 17, rigid 12, hair 3}/main.rs`, по `mod` на старый файл. Блокеры перечислимы:
  10 `#[path = "support/..."]`, 5 `include_str!("data/...")`, `anatomy_tet.rs:3-5` (`ancestors().nth(4)`),
  `quadratic_wet_update.rs` (`crate::calibration`), 6 лишних `#![allow]`. Перед началом снимается список
  имён тестов (`-- --list`), после каждой группы сверяется: 1594 теста, 4 ignored. Документы
  `docs/{hair,tissues,physics,biomechanics,...}.md`: `--test urogenital` -> `--test physics_bio urogenital::`.
  Статус: `da0ecf2b6`: 7 бинарей, тот же список из 1594 тестов, все зелёные.
- [ ] **2.3 Ациклический `physics` (13 коммитов).** Фактически это три сильно связные компоненты, а не пары.
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
- [ ] **2.4 Инверсия `voxy_shell -> voxy_demos` (11 коммитов, два из них L).** Демо взаимоисключающие
  (каждый `with_*` сбрасывает остальные, `scene_app.rs:222-360`), значит честная форма: один слот
  `Option<Box<dyn SceneDemo>>`. Trait нельзя объявить в `voxy_shell` при миграции по одному демо (получится
  cargo-цикл), поэтому новый крейт-лист `voxy_scene_host` (~200 строк: trait `SceneDemo` из 12 методов-фаз,
  6 обязательных, контекстные типы, хелпер `MeshStage`). Это единственный новый крейт в плане; заодно
  `voxy_demos` остаётся без winit, как требует `architecture.md` §4.
  Порядок: слот и хуки в shell -> `strands` -> `tissues` -> `gravity` -> `fem`+`wear` -> `xray` -> `liquids` ->
  `female` в два шага (GPU-резидентность в `voxy_demos::female_scene::FemaleScene`, затем impl) -> снять ребро.
  Скрытая связь, которую это убирает: `scene_app.rs:825` делает `include_str!("../../voxy_demos/src/...wgsl")`.
  Проверка требует запуска примеров с `--smoke` (GPU), одного `cargo test` недостаточно.
- [ ] **2.5 (M)** GPU-решатели контактов (`banded_*`, `joint_contact_*`, `contact_five_*`, 19 `Experimental*`)
  из `voxy_render` в `voxy_gpu`; сужение `voxy_render::lib.rs` (~512 реэкспортов). Осуществимость проверяется
  перед началом (что они берут из render).
- Не делаю: перенос `angular_sweep`/`convex` из gameplay в physics (physics принципиально без glam, это было бы
  переписывание, а не перемещение); они остаются в gameplay и получают `SweepQuery` в волне 3.

### Волна 3. God-объекты и длинные функции (SRP)
Порядок по churn x размер. Каждый пункт: сначала перемещение тел без изменений, затем логика.
- [ ] **3.1 editor (L)** `App` (`lib.rs:457`). Чистого разреза нет ни у одного кластера (например,
  `publish_editor_checkpoint_install` трогает 45 полей), поэтому порядок от самого изолированного, образец:
  существующий `AudioSession` (приватные поля + методы), а не `AuthoringSession` (мешок полей).
  `ViewportCameras` (camera, secondary_camera, active_view, camera_drag, cursor) -> `ModelRelink` (4 поля) ->
  `SmokeHarness` (enum `SmokeScenario`, сценарии взаимоисключающие) -> предикаты занятости
  (`file_operation_active()`, `fences_held()`, `draft_active()`) -> asset pipeline (10 полей, `catalog` остаётся
  в `App`) -> состояние файловых операций -> panel UI (`invalidate()` вместо 28 `self.panel_cache = None`).
  Не извлекаю: `WindowHost` и `InspectorState` (сборные кластеры, инкапсуляции не дают). Единый enum
  `ModalOperation` не делаю: поля не взаимоисключающие (сохранение держит `editor_save`, `restore_barrier`
  и `resource_checkpoint` одновременно, `app/restore_barrier/save.rs:50-63`).
  Отдельно: `Panels::build` 1314 строк (`panels.rs:226`) по режимам инспектора; `draw_frame_inner` 839
  (`app/viewport.rs:28`) по фазам, дубль offscreen/onscreen (`:563-604` и `:775-800`) в одну функцию.
- [ ] **3.2 render (L)** `ArchiveSceneRenderer` (`archive_scene.rs:322`): под-структуры Layouts, Pipelines,
  Targets, Post, Gi, Environment; `new_with_environment` 1080 и `draw` 580 по ним; 66 `std::env::var`
  в структуру конфигурации; 21 `SceneRenderer::encode_*` в `encode(EncodeOptions)`; `Renderer` (60 полей)
  на residency / camera / skinned / bodycam / capture; `skinned.rs` на cpu / admission / gpu.
- [ ] **3.3 shell (M)** `SceneApp::{draw 789, initialize 546, window_event 360}` после 2.4.
- [ ] **3.4 physics (L)** пять функций по 580-1070 строк (`advance_implicit_surface_quadrature`,
  `multi_body::solve_contact`, `reconcile_positions_attempt`, `hair::step_impl`, `impact_events_impl`):
  фазы в отдельные функции над структурой состояния попытки/итерации; вместе с 5.8.
  68 функций с > 7 параметрами и 152 `*_with_*`: структуры параметров по доменам.
- [ ] **3.5 animation / gameplay (M)** `voxy_animation/src/lib.rs` на skeleton / clip / pose / animator / math;
  `CharacterPhysics` (impl 960 строк) из `voxy_gameplay/src/lib.rs` в модуль; `angular_sweep` (11 вариантов
  по 10-14 параметров) на `SweepQuery`.
- [ ] **3.6 app / model (M)** `VoxyApp` (80 полей, `main.rs:47`): CameraMode, `PendingGpu<T>`, vehicle;
  `voxy_model/src/lib.rs` на `gltf/{validate,material,animation,parse}.rs` + `pose.rs`.
- `voxy_ray_probe` (диагностический бинарь, функции до 1100 строк): только механический разрез по протоколам, последним.

### Волна 4. DRY
- [ ] **4.1 (M)** `physics::math`: `finite`, операции 3x3, один плотный решатель на плоском хранилище
  (Cholesky x5, Гаусс x5 сводятся к двум функциям), идентичные копии в `static_triangle_world.rs`,
  `skin/contact.rs`. Отличающиеся варианты теперь тоже можно сводить; каждое изменение ожидаемых
  значений в тестах фиксируется в коммите и `BASELINE.md`.
- [ ] **4.2 (M)** WGSL: маленький резолвер `#include` в `voxy_gpu_core` вместо конкатенаций и `.replace()`;
  общие `dd_math.wgsl` (13 копий), `fullscreen.wgsl` (26), `tonemap.wgsl` (3), `camera.wgsl` (20 определений).
- [ ] **4.3 (M)** `voxy_gpu_core::builders` во всех конструкторах render/gpu; удалить ~22 локальных хелпера.
- [ ] **4.4 (S)** Тест раскладки каждого `Pod`-uniform против WGSL через naga; `SceneTransform` как структура.
- [ ] **4.5 (L)** `JsonDocument` и `Checkpoint<T>` (версия, бюджет байт, prepare/publish) для 77 функций и
  49 checkpoint-файлов. Сначала golden round-trip тест на каждый формат, затем миграция по одному файлу.
- [ ] **4.6 (M)** Один orbit-контроллер (5 реализаций), одно отображение `KeyCode -> Control`, общий
  winit frame-host для 12 `ApplicationHandler`; `labia_tube_insertion` и `anal_tube_insertion` (725 общих строк).
- [ ] **4.7 (S)** `// SAFETY:` на 82 `unsafe`, затем `clippy::undocumented_unsafe_blocks`. Ограничение: D3D12/CUDA/
  Streamline-код собирается только под Windows, на этой машине (macOS) непроверяем; в нём только комментарии.

### Волна 5. Оптимизация по замерам
Цикл на каждый пункт: бенч (в стиле существующих `benches/*.rs`) -> цифра "до" -> правка -> цифра "после"
-> запись в `BASELINE.md` и `docs/architecture-performance/2026-11-10-wave-3/`. Нет выигрыша: откат.
- [ ] **5.1 gameplay tick** `lib.rs:1185-1195`: `StaticWorld` пересобирается и `states` клонируется каждый тик,
  ввод ищется по строке в `BTreeMap`. Кэш по ревизии сцены, staging-буфер, заранее разрешённые действия.
- [ ] **5.2 editor frame** `app/viewport.rs:931-1034`: клон всего `SceneDocument` и глубокое сравнение каждый кадр.
  Кэш панелей по ревизии.
- [ ] **5.3 demos step** `female_demo.rs:1553-1612`: 4 `fs::metadata` на каждый шаг физики. Опрос по таймеру.
  `:1641-1726`: клоны состояния для отката через `clone_from` в удерживаемые буферы.
- [ ] **5.4 animation** `sample_into` / `blend_into` / `advance_into(&mut AnimatorFrame)`: сейчас 2+ `Vec` на
  персонажа на кадр (`lib.rs:867,1254,1700`).
- [ ] **5.5 scene** `active_components<T>` (`voxy_scene/src/lib.rs:418-445`) обходит все слоты с хэш-пробой;
  индекс по типу. Бенч `scene_scaling` уже есть.
- [~] **5.6 voxel** `palette_indices()` (`voxy_world/src/chunk.rs:132`, 32 768 `u16` на вызов, 3 потребителя):
  scratch-API; scratch в `voxy_lighting/src/propagate.rs`; пословная распаковка и табличный CRC32 в
  `voxy_storage/src/codec.rs` (формат тот же). Бенч `voxel_pipeline` расширить на storage.
  Статус: буферы `palette_indices_into` сделаны (`aab725583`, меш −11%); остались пословный codec и CRC32 в `voxy_storage`.
- [ ] **5.7 render CPU** кэш покадровых bind group / view / buffer (`archive_temporal.rs:322-456`, `bloom.rs:180-189`,
  `shadow_map.rs:161-166`, `native_vegetation.rs:849`, `blit.rs:500-522`); archive-шейдер 145 KB
  компилировать один раз вместо трёх.
- [ ] **5.8 physics** `HairBackend::solve` (`hair/mod.rs:444`, `Vec<Vec<f64>>` на каждый solve) в `solve_into`;
  scratch на попытку в `reconcile.rs:345-360` и `step.rs:192`; `surface_film/squeeze.rs:75,352`;
  `multi_body.rs:865-1022`; 206 `self.clone()` пробных шагов через `clone_from`; `Vec<Vec<f64>>`-матрицы в плоские.
  Сначала бенчи: hair step/reconcile, multi_body, implicit, squeeze, lymph (сейчас их нет).
- Вне волны: GI даёт половину GPU-кадра (13.95 -> 6.49 мс без GI), но это отдельный протокол квалификации
  изображения (`docs/architecture-performance/2026-10-27-gi-*`), в бэклог с пометкой "самый большой выигрыш кадра".

### Волна 6. Закрепление
- [ ] **6.1** Автоисправляемые clippy-линты из allow-списка по одному на коммит (`cargo clippy --fix`):
  `needless_borrow`, `collapsible_if`, `implicit_clone`, `explicit_iter_loop`, `uninlined_format_args`,
  `semicolon_if_nothing_returned`, `manual_let_else`, `redundant_closure_for_method_calls`, `map_unwrap_or`.
- [ ] **6.2** `too_many_lines` (порог 200) и `too_many_arguments` (порог 10) включить; оставшиеся нарушители
  получают `#[expect(..., reason = "docs/refactor/PLAN.md")]`: новая длинная функция ломает clippy, а
  сокращённая заставляет убрать `expect`.
- [ ] **6.3** Типизированные ошибки: physics (1277 `Result<_, &'static str>`) начиная с функций на границе крейта;
  `Box<dyn Error>` в editor/demos/shell (363). Текст `Display` сохраняется.
- [ ] **6.4** `docs/architecture.md` §4 из `cargo metadata`, ADR 0005 (решения этой волны), `INVENTORY.md` со
  статусами, `BASELINE.md` до/после, остаточный бэклог в `PLAN.md`.

