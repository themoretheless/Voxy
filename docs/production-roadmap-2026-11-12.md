# Отчёт о шести инфраструктурных шагах (2026-11-12)

Документ фиксирует, что именно добавлено в рабочее дерево 2026-11-12 по шести шагам, и что из этого
**не** следует. Исходного «6-фазного плана» в репозитории нет: порядок и критерии этапов задаёт
[`engine-roadmap-2026-10-25.md`](engine-roadmap-2026-10-25.md) (ENG-00…ENG-13), статус ведёт
[`engine-status-2026-11-11/README.md`](engine-status-2026-11-11/README.md).

**Статус: движок не production-ready.** Ни один критерий выхода ENG-этапа этими шагами не закрыт.
Шаги 2-6 добавляют библиотечные примитивы с модульными тестами. К продукту подключены два из них: суставы
(тик Play в редакторе) и кэш проб (`street_walk`); остальные пока не вызываются редактором, игрой или рендерером. Сверка с кодом и замечания: [`plans-review-2026-11-12.md`](plans-review-2026-11-12.md).

Состояние git на момент отчёта: всё перечисленное не закоммичено. Не отслеживаются
`crates/voxy_player/`, `crates/voxy_gameplay/src/joints.rs`, `crates/voxy_scene/src/replication.rs`,
`crates/physics/src/liquid/multi_body/step_entry.rs`, `crates/voxy_gameplay/src/angular_sweep/path_bounds.rs`.
`voxy_player` уже прописан в `members` и в зависимостях редактора, поэтому чистая выгрузка только
отслеживаемых изменений не соберётся: эти файлы нужно коммитить вместе.

---

## 1. Что сделано

### Шаг 1. Бюджет размера файлов, патчи wgpu, CI (ENG-00)

- **Список исключений `KNOWN_OVERSIZED` пуст** ([`crates/voxy_depgraph/tests/sources.rs`](../crates/voxy_depgraph/tests/sources.rs)).
  Бюджет `LINE_BUDGET = 1500` считает только прод-строки достижимых из дерева `mod` файлов, без
  `#[cfg(test)]`-участков. Файлов длиннее 1500 строк по `wc -l` в воркспейсе много (в основном тесты,
  самый большой `crates/voxy_demos/src/gpu_hair_joint.rs`, 6806 строк), и это бюджету не противоречит.
  Три последних исключения сняты так:

  | Файл | Было (HEAD) | Стало | Вынесено |
  |---|---|---|---|
  | [`physics/src/liquid/multi_body.rs`](../crates/physics/src/liquid/multi_body.rs) | 1669 | 1382 | [`multi_body/step_entry.rs`](../crates/physics/src/liquid/multi_body/step_entry.rs), 296 строк (новый) |
  | [`voxy_gameplay/src/angular_sweep.rs`](../crates/voxy_gameplay/src/angular_sweep.rs) | 1575 | 1143 | [`angular_sweep/path_bounds.rs`](../crates/voxy_gameplay/src/angular_sweep/path_bounds.rs), 444 строки (новый) |
  | [`voxy_web/src/browser.rs`](../crates/voxy_web/src/browser.rs) | 1609 | 1434 | в существовавший [`voxel_terrain.rs`](../crates/voxy_web/src/voxel_terrain.rs): 119 -> 324 строки |

- **Регрессионный тест патчей `vendor/wgpu`**: `vendored_wgpu_patches_preserve_core_device_identity_and_metal_surface_autorelease`
  в том же файле проверяет, что в `vendor/wgpu/src/backend/wgpu_core.rs` остались `CoreDevice::identity()` с
  указателем на `Global` и `impl_eq_ord_hash_proxy!(CoreDevice => .identity())`, а в
  `vendor/wgpu-hal/src/metal/surface.rs` осталась защита `autoreleasepool`.
- **`.gitignore`**: `*.vprobes.tmp`, `*_skipped.face`, `*_skipped.node`.
- **CI** ([`.github/workflows/ci.yml`](../.github/workflows/ci.yml)): тулчейн `nightly-2026-05-27` (тот же, что в
  `rust-toolchain.toml`), `cargo clippy --workspace --all-targets -- -D warnings`, матрица
  `macos-latest` / `ubuntu-latest` / `windows-latest`, job `cross-targets`: `cargo check -p voxy_web` для
  `wasm32-unknown-unknown` и `cargo check -p voxy_mobile` для `aarch64-apple-ios` и `aarch64-linux-android`.
  **Workflow ни разу не запускался.** Локально `cargo check -p voxy_web --target wasm32-unknown-unknown`
  проходит. Обе мобильные проверки локально не выполнить: `blake3` собирает C-код, а на машине нет iPhoneOS SDK
  (только Command Line Tools) и нет Android NDK. Для Android в job добавлены `CC_aarch64_linux_android` и
  `AR_aarch64_linux_android` из `ANDROID_NDK_LATEST_HOME`; это не проверено. По правилу роадмапа «компиляция без
  запуска не считается платформенной квалификацией» это не закрывает ни ENG-10, ни ENG-11.

### Шаг 2. Чтение ассетов из `.vpak` в памяти и крейт `voxy_player` (к ENG-03 / ENG-10)

- [`voxy_assets/src/files.rs`](../crates/voxy_assets/src/files.rs): `AssetSourceProvider`
  (`Directory(FileInputs)` | `Package(Arc<ResourcePackage>)`) с методами `read` и `fingerprint`.
- [`voxy_assets/src/package.rs`](../crates/voxy_assets/src/package.rs): `ResourcePackage::fingerprint` и
  `ResourcePackage::open_project_descriptor` (пакет из байтов открывает существовавший `ResourcePackage::from_bytes`).
- Новый крейт [`voxy_player`](../crates/voxy_player/src/lib.rs) (только библиотека, 324 строки, без бинаря,
  окна и цикла кадра): `GameHost`, `BehaviorSetup`, `register_builtin_ui_handlers`,
  `builtin_ui_action_checkpoint_registry` и `InMemoryPackagedProject` (`from_vpak_bytes`, `from_package`).
  `InMemoryPackagedProject` проверяет манифест, сцену, раскрытие префабов, отпечатки привязок манифеста и
  разрешимость ссылок `editor.model.v1`. Меши, материалы, шрифты и аудио он не декодирует.
- [`voxy_editor/src/game_host.rs`](../crates/voxy_editor/src/game_host.rs) и
  [`ui_action_codecs.rs`](../crates/voxy_editor/src/ui_action_codecs.rs) реэкспортируют из `voxy_player`.
  Рёбра графа внесены в [`depgraph.rs`](../crates/voxy_depgraph/tests/depgraph.rs),
  [`tools/check_engine_boundaries.py`](../tools/check_engine_boundaries.py) и [`architecture.md`](architecture.md) §4.
- **Подключено к:** только к собственному тесту. Автономного запускаемого плеера нет; `InMemoryPackagedProject`
  не использует ни один раннер.

### Шаг 3. Кэш запечённых проб и узлы `RenderGraph` (к ENG-05)

- [`voxy_render/src/probe_gi.rs`](../crates/voxy_render/src/probe_gi.rs): `BakedProbeCache`, конверт
  `voxy_core::CheckpointEnvelope` с магией `VPRB`, версией 1 и CRC-32C. Методы: `new`, `to_bytes`, `from_bytes`,
  `save_to_path` (запись через временный файл и `rename`), `load_from_path`, `upload_to_buffer`.
- [`archive_scene/probe_timing.rs`](../crates/voxy_render/src/archive_scene/probe_timing.rs):
  `ArchiveSceneRenderer::read_irradiance_probe_cache(&bake)` считывает коэффициенты завершённого запекания с GPU,
  `restore_irradiance_probe_cache(min, max, spacing, &cache)` загружает их вместо запекания и возвращает `false`,
  если кэш снят для другой сетки.
- [`voxy_render/src/render_graph.rs`](../crates/voxy_render/src/render_graph.rs): `RenderGraphNode::Ocean`
  (`add_ocean`) и `RenderGraphNode::ScreenSpaceFluid` (`add_screen_space_fluid`), вызывающие
  `OceanRenderer::encode` и `ScreenSpaceFluidRenderer::encode`.
- **Подключено к:** кэш подключён к примеру `street_walk` через переменную `VOXY_STREET_PROBE_CACHE=<файл.vprobes>`:
  при наличии подходящего файла запекание пропускается, после запекания файл записывается. Кэш привязан только к
  сетке проб, а не к содержимому сцены: после изменения сцены, света или материалов файл нужно удалить вручную.
  Проверено тестом на Metal (`single_submission_probe_bake_matches_separate_submissions`: считанный и
  восстановленный кэш побайтно совпадает с запеканием). Сам `street_walk` с кэшем не запускался, выигрыш во
  времени старта (запекание занимало 1310 с по журналу статуса) не измерен.
- Узлы `Ocean` и `ScreenSpaceFluid` не создаёт ни один рендерер. Оба рендерера рисуют в собственные цели без
  общего depth, а `RenderGraph::encode` требует depth-цель; подключение осмысленно только вместе с общей
  графической сценой (ENG-05).

### Шаг 4. Компонент `PhysicsJoint` (к ENG-04)

- [`voxy_gameplay/src/joints.rs`](../crates/voxy_gameplay/src/joints.rs): сериализуемый компонент
  `game.physics-joint.v1` с видами `Fixed`, `Hinge`, `DistanceSpring`, валидацией параметров, переназначением
  `target` при инстанцировании префабов, функциями `solve_distance_spring_position`, `collect_physics_joints`,
  `step_physics_joints`, `validate_physics_joints` и `step_scene_physics_joints` (тело системы `joint.step`).
- Решатель кинематический: `Fixed` и `Hinge` переставляют владельца относительно цели, без масс, скоростей и
  импульсов; пружинным является только `DistanceSpring`. Ограничения решаются в мировых координатах, владелец и
  цель могут иметь разных родителей. Пять тестов: пружина, `Fixed` и `Hinge` под разными родителями, сбор привязок,
  проверка перед стартом, совместимость с фазой движущихся платформ.
- **Подключено к:** система `joint.step` добавлена в `voxy_gameplay::gameplay_schedule` (после `kinematic.step`,
  до персонажей) и исполняется в Play редактора ([`app/play_mode.rs`](../crates/voxy_editor/src/app/play_mode.rs)).
  Standalone и пакет идут через тот же `App::advance_game` (`HeadlessGame`), отдельной проводки не потребовалось.
- **Правка в редакторе** ([`joint_authoring.rs`](../crates/voxy_editor/src/joint_authoring.rs)): на странице
  компонентов инспектора сустав добавляется и удаляется кнопкой, цель выбирается в иерархии тем же механизмом, что у
  следящей камеры; «Pick» меняет цель, «Change» переключает вид, параметры и якоря правятся как поля компонента.
  Новый сустав `Fixed` удерживает авторскую относительную позу. Недопустимые значения (`PhysicsJoint::valid`), цель
  на самого себя и удаление объекта-цели отклоняются без изменения сцены; `validate_game_descriptors` и
  `validate_physics_joints` повторяют проверку при каждой правке документа и перед первым тиком Play.
- **Сквозной сценарий** (`authored_joints_hold_in_play_stop_standalone_and_source_free_package`): сварной объект
  следует за движущимся `KinematicBody`, пружина сходится к длине покоя, Stop возвращает авторскую сцену, 120 тиков
  совпадают в Play, standalone и пакете без исходников. До этого сустав, сдвинувший владельца в тике с движущейся
  платформой, отклонял тик (`InvalidMotion`): `joint.step` менял ревизию сцены после фазы платформ. Теперь
  `PlatformMotion::adopt_unrelated_moves` сохраняет фазу, если сустав не двигает платформу, её пассажира или их
  предков; иначе тик отклоняется, как раньше.
- Не сделано: с динамическими телами и контактами суставы не связаны (владелец с коллайдером переставляется без
  учёта столкновений), `voxy_web` суставы не исполняет. Критерий ENG-04 («в редакторе и игре») для суставов закрыт
  только в этом кинематическом объёме.

### Шаг 5. Навигация по карте высот (к ENG-07)

- [`voxy_navigation/src/lib.rs`](../crates/voxy_navigation/src/lib.rs): `HeightfieldNavGrid`. Это 2.5D: одна высота
  и один вертикальный габарит на ячейку, многоуровневых поверхностей (мост над дорогой) он не описывает.
  Лимиты `max_step_height`, `max_slope_tan`, `min_clearance` задаются в `HeightfieldNavGrid::new`; поиск:
  `find_path_3d(&self, start, goal, max_expanded)`, детерминированный 8-связный A* с запретом срезания углов.
- **Подключено к:** только к тестам крейта. Критерий ENG-07 (NPC в автономной тестовой игре) не выполнен.

### Шаг 6. Снимки и дельты сцены в памяти (к ENG-12)

- [`voxy_scene/src/replication.rs`](../crates/voxy_scene/src/replication.rs):
  - `TickSnapshot`: канонический `SceneDocument` с номером тика и CRC-32C состояния;
  - `SceneSnapshotDelta`: изменённые объекты целиком плюс удалённые `ObjectId`; `diff(&base, &target)` и
    `apply(&TickSnapshot)` с проверкой `base_tick` и контрольной суммы результата;
  - `SnapshotRollbackBuffer`: кольцевая история фиксированной ёмкости, `push`, `get`,
    `reconcile_or_truncate(server_tick, crc)` и
    `rollback_and_apply_authoritative(&ComponentRegistry, capacity, &SceneSnapshotDelta) -> LoadedScene`:
    находит базовый снимок, применяет дельту, загружает результат во временную сцену через
    `SceneDocument::load` и оставляет в истории только авторитативный снимок. При ошибке история не меняется.
- Это структуры данных. Сети, транспорта, сервера и побайтового сжатия нет; ни один крейт не имеет сетевых
  зависимостей. Критерий ENG-12 (выделенный сервер, авторитет, проверки под задержкой, потерями и
  переподключением) не выполнен.
- **Подключено к:** только к тестам модуля.

---

## 2. Что остаётся, чтобы шаги что-то значили для этапов

| Этап | Чего не хватает |
|---|---|
| ENG-03 / ENG-10 | Бинарь плеера, который открывает `.vpak` через `InMemoryPackagedProject` и крутит кадр; запуск на чистой машине |
| ENG-04 | Связь суставов с динамическими телами и контактами; исполнение суставов в `voxy_web` |
| ENG-05 | Замер старта `street_walk` с кэшем и без; привязка кэша к содержимому сцены; использование узлов `Ocean` / `ScreenSpaceFluid` рендерером |
| ENG-07 | Построение `HeightfieldNavGrid` из мира и NPC, идущий по пути в тестовой игре |
| ENG-10 / ENG-11 | Первый реальный прогон CI (в том числе iOS и Android, которые локально не собрать); запуск, а не `cargo check`, на целевых платформах |
| ENG-12 | Транспорт, сервер, сериализация дельт в байты, предсказание и повтор ввода, тесты под потерями |

---

## 3. Проверки

Запускались в этой сессии, на незакоммиченном рабочем дереве, macOS:

| Проверка | Команда | Результат |
|---|---|---|
| Форматирование | `cargo fmt --all -- --check` | чисто |
| Границы владельцев | `python3 tools/check_engine_boundaries.py` | `6 owners, 42 packages` |
| Граф, сироты, бюджет файлов, слои physics, патчи wgpu | `cargo test -p voxy_depgraph` | 18 тестов: 6 + 4 + 3 + 5, все прошли |
| Модульные тесты затронутых крейтов | `cargo test -p voxy_scene -p voxy_gameplay -p voxy_navigation -p voxy_player -p voxy_assets -p voxy_render --lib` | `voxy_assets` 70, `voxy_gameplay` 253, `voxy_navigation` 8, `voxy_player` 1, `voxy_render` 365 (69 ignored), `voxy_scene` 109; падений нет |
| Clippy по всему воркспейсу | `cargo clippy --workspace --all-targets -- -D warnings` | чисто после исправления шести ошибок (до исправления падал) |
| Ссылки в изменённых документах | скрипт проверки относительных ссылок | мёртвых нет |
| Полный набор тестов воркспейса | `cargo test --workspace --no-fail-fast` | 124 набора, 4769 прошли, 0 упали, 318 ignored (запущен до правок суставов и кэша проб) |
| Перепрогон крейтов, изменённых после этого | `cargo test -p voxy_gameplay -p voxy_editor -p voxy_scene -p voxy_render -p voxy_player --no-fail-fast` | 24 набора, 1239 прошли, 0 упали, 120 ignored |
| Кэш проб на GPU | `cargo test -p voxy_render --lib single_submission_probe_bake -- --ignored` | прошёл на Metal |
| Web | `cargo check -p voxy_web --target wasm32-unknown-unknown` | собирается |
| Полный набор тестов на итоговом дереве (после `voxy_scene_host` и лимитов функций) | `cargo test --workspace --no-fail-fast` | 126 наборов, 4777 прошли, 0 упали, 318 ignored |
| То же после унификации контроллеров и оконного хоста (4.6) | `cargo test --workspace --no-fail-fast` | 126 наборов, 4786 прошли, 0 упали, 318 ignored |
| То же после авторинга суставов в инспекторе | `cargo test --workspace --no-fail-fast` | 126 наборов, 4797 прошли, 0 упали, 320 ignored |

Не запускалось: workflow GitHub Actions, оконные сценарии, остальные `#[ignore]`-тесты (в основном требуют GPU
или долгие). Сборку под iOS и Android локально выполнить нельзя (нет iPhoneOS SDK и NDK).
