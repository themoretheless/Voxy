# ENG-00: реестр возможностей и результаты аудита

Дата: 27 октября 2026. **ENG-00 завершён в пределах аудита**, не реализации остальных этапов.

Проверенная основа: `8acd97edce5fe2024fe173a2ba792dad27e087fe` плюс незакоммиченные изменения.
Рабочее дерево активно меняется другими задачами. Полный список состояния и хеши
исходников до/после проверок сохранены в [отчёте](checks-01/report.json).
Хеши Rust/TOML/WGSL исходников не изменились в течение контрольного прогона.

## Метод и границы

Для каждой области указаны владелец, код контракта, реальный потребитель, исходник
проверки и оставшаяся интеграция. Ссылки на тест означают наличие проверочного
сценария, а не его свежий успешный запуск. Все продуктовые gates в JSON оставлены
`not_requalified`; состояние области — `partial`. Это намеренное различие между
аудитом и квалификацией продукта. Готовность в процентах не вычисляется.

Код проверен чтением интерфейсов и их потребителей. Это не полный семантический
аудит всех алгоритмов. Для networking отсутствие общего владельца означает,
что он не установлен этим аудитом; отсутствие любого прототипа не утверждается.

## Матрица

| Область / этап | Владелец | Контракт / потребитель / проверка | Следующая проверка и пробел |
| --- | --- | --- | --- |
| project / ENG-01 | ProjectDescriptor / editor project | [api](../../crates/voxy_editor/src/project.rs); [consumer](../../crates/voxy_editor/src/main.rs); [test](../../crates/voxy_editor/src/project.rs) | Create/open/relocate and refuse overwrite. Далее: Native project creation UI and clean-machine delivery |
| scene / ENG-02 | SceneGraph / SceneDocument | [api](../../crates/voxy_scene/src/document.rs); [consumer](../../crates/voxy_editor/src/authoring_session.rs); [test](../../crates/voxy_scene/tests/associated_data.rs) | Transactional loading, IDs, references and component codecs. Далее: Project migrations and scene transitions |
| prefab / ENG-08 | PrefabSceneDocument / authoring session | [api](../../crates/voxy_scene/src/prefab_document.rs); [consumer](../../crates/voxy_editor/src/prefab_authoring.rs); [test](../../crates/voxy_scene/tests/persistent_prefabs.rs) | Nested instances, overrides and history roundtrip. Далее: Combined authoring UX and background dependency recovery |
| runtime / ENG-02 | SceneSimulation / domain stores | [api](../../crates/voxy_scene/src/simulation.rs); [consumer](../../crates/voxy_editor/src/project_runtime.rs); [test](../../crates/voxy_scene/tests/scheduled.rs) | Declared access, command barriers, fixed steps. Далее: Cross-domain scheduling and transitions with active jobs |
| game-code / ENG-06 | GameHost / BehaviorRunner | [api](../../crates/voxy_editor/src/game_host.rs); [consumer](../../crates/voxy_editor/examples/project_game.rs); [test](../../crates/voxy_editor/src/game_host_tests.rs) | Lifecycle parity across Play and source-free package. Далее: Project code build workflow and runtime/editor layering |
| assets / ENG-03 | AssetCatalog / observed inputs | [api](../../crates/voxy_assets/src/lib.rs); [consumer](../../crates/voxy_editor/src/import.rs); [test](../../crates/voxy_assets/src/import_worker.rs) | Revision observation and last-good resource publication. Далее: All resource classes, cache quotas and dependency-wide reload |
| package / ENG-03 | ResourcePackage / project exporter | [api](../../crates/voxy_assets/src/package.rs); [consumer](../../crates/voxy_editor/src/packaged_game.rs); [test](../../crates/voxy_editor/src/project.rs) | Source-free package and rejected corrupt/overwrite output. Далее: Distributable executable, user game code and platform gates |
| render / ENG-05 | SceneRenderer / graphics owners | [api](../../crates/voxy_render/src/scene.rs); [consumer](../../crates/voxy_editor/src/gpu_model.rs); [test](../../crates/voxy_render/examples/scene_smoke.rs) | Combined materials/light/shadow/transparency image oracle. Далее: GPU tests intentionally not run; full scene/resize/device qualification |
| physics / ENG-04 | physics stores / gameplay staged tick | [api](../../crates/physics/src/lib.rs); [consumer](../../crates/voxy_gameplay/src/staged_tick.rs); [test](../../crates/voxy_gameplay/tests/integration.rs) | Bodies, contacts, queries and scene synchronization. Далее: Audit general joints/triggers/CCD and test actual editor exposure |
| animation / ENG-06 | Animator / AnimationRuntime | [api](../../crates/voxy_animation/src/lib.rs); [consumer](../../crates/voxy_editor/src/animation_runtime.rs); [test](../../crates/voxy_animation/src/tests.rs) | Clips/fades/root motion/IK and rejected pose rollback. Далее: Complete authoring and combined physical motion qualification |
| input / ENG-07 | InputMap | [api](../../crates/voxy_input/src/lib.rs); [consumer](../../crates/voxy_editor/src/headless_game.rs); [test](../../crates/voxy_input/src/checkpoint.rs) | Quick taps, focus and pending edge checkpoint. Далее: Native input/gamepad and project rebinding UI |
| ui / ENG-07 | UI runtime / focus owner | [api](../../crates/voxy_ui/src/lib.rs); [consumer](../../crates/voxy_editor/src/game_ui_input.rs); [test](../../crates/voxy_editor/src/headless_game_tests.rs) | Saved layout, focus and callbacks in packaged replay. Далее: Native controls, complete layout and accessibility policy |
| text / ENG-07 | Text shaping / atlas | [api](../../crates/voxy_text/src/lib.rs); [consumer](../../crates/voxy_editor/src/ui_text.rs); [test](../../crates/voxy_text/src/shape.rs) | Font import, Unicode layout and bounded atlas. Далее: Coverage/fallback and native DPI/layout qualification |
| audio / ENG-07 | AudioRuntime / output worker | [api](../../crates/voxy_audio/src/lib.rs); [consumer](../../crates/voxy_editor/src/audio_session.rs); [test](../../crates/voxy_gameplay/src/audio_runtime.rs) | PCM spatial sources, buses and reload retention. Далее: Device tests intentionally excluded; native lifecycle and authoring |
| navigation / ENG-07 | NavigationGrid / bounded jobs | [api](../../crates/voxy_navigation/src/lib.rs); [consumer](../../crates/voxy_scene/examples/navigation_gameplay.rs); [test](../../crates/voxy_navigation/src/lib.rs) | Revision-safe sliced four-connected grid path. Далее: 3D navigation, local avoidance and real moving agents |
| saves / ENG-07 | Scene/runtime participant owners | [api](../../crates/voxy_scene/src/scene_simulation_checkpoint.rs); [consumer](../../crates/voxy_editor/src/play_session/checkpoint.rs); [test](../../crates/voxy_gameplay/examples/gameplay_save.rs) | Bounded versioned restore and continuation parity. Далее: Audit complete session transaction, revision archive and disk slots |
| world / ENG-09 | Voxel world / rebuild publication | [api](../../crates/voxy_world/src/lib.rs); [consumer](../../crates/voxy_runtime/src/lib.rs); [test](../../crates/voxy_runtime/src/lib.rs) | Stale jobs, bounded barriers and rebuild/full oracle. Далее: Moving-world integration, atomic changed mesh upload and budgets |
| clock / ENG-02 | FrameLoop / time owner | [api](../../crates/voxy_time/src/lib.rs); [consumer](../../crates/voxy_runtime/src/frame_loop.rs); [test](../../crates/voxy_time/src/lib.rs) | Pause, overload, checkpoints and percentile telemetry. Далее: Actual frame pacing and domain timing attribution |
| editor / ENG-08 | AuthoringSession / history | [api](../../crates/voxy_editor/src/lib.rs); [consumer](../../crates/voxy_editor/src/panels.rs); [test](../../crates/voxy_editor/src/project_prefab_tests.rs) | Create/edit/save/undo and isolated Play. Далее: Native combined workflow, recovery and authoring completeness |
| platforms / ENG-11 | Platform shell / XR session | [api](../../crates/voxy_xr/src/lib.rs); [consumer](../../crates/voxy_mobile/src/lib.rs); [test](../../mobile/android/test_build.py) | Package validation and target-specific lifecycle. Далее: Actual Windows/Linux/Web/mobile/XR deployment and device matrix |
| network / ENG-12 | No common owner established by audit | [api](../../docs/adr/0002-general-engine-architecture.md); [consumer](../../Cargo.toml); [test](../../docs/engine-roadmap-2026-10-25.md) | Authority, replication, loss/reconnect and server test required. Далее: Locate any prototype, then define runtime contract; not qualified |
| scripting / ENG-13 | Behavior command/event adapter | [api](../../crates/voxy_scene/src/behavior.rs); [consumer](../../crates/voxy_editor/src/game_host.rs); [test](../../crates/voxy_editor/src/game_host_tests.rs) | Rust lifecycle is present; additional language not qualified. Далее: Language ADR, bounded execution and adapter compatibility |

Машиночитаемый источник: [inventory.json](inventory.json). Для каждой области
отдельные gates: core, runtime, editor, persistence, standalone, platform.

## Свежие результаты без устройств

| Проверка | Результат | Доказательство |
| --- | --- | --- |
| Транзитивные production-зависимости | 4 владельца, 39 пакетов; нарушений заданных границ нет | [лог](checks-01/check-1.log) |
| Регрессии dependency checker | 7 passed | [лог](checks-01/check-2.log) |
| Rust library tests | core 17, time 8, input 8, navigation 5, runtime 16; всего 54 passed | [лог](checks-01/check-3.log) |
| Strict Clippy | time/input/navigation, library targets; passed | [лог](checks-01/check-4.log) |

Тесты самого ENG-00 валидатора: [7 passed](validator-tests.log); проверяются дубли, отсутствующие пути,
выход за корень через абсолютный/относительный путь и symlink, отсутствие gates,
а также запрет превращать source pointers в заявление о готовности standalone.

Окна, GPU, audio devices и native input не запускались. Полные editor/physics/
animation suites и workspace Clippy/tests в этом этапе не запускались. Прежние
отчёты production-readiness остаются историческими свидетельствами своих ревизий.
CPU тесты не доказывают 120 FPS, потоковую загрузку большого мира или native UX.

## Уточнения к старому плану и следующий этап

1. В текущем main.rs уже есть create/validate/project/build/package команды.
   Старую строку «вход только от модели» считать историческим состоянием 25 октября.
2. project.rs уже содержит создание, перенос и отказ от перезаписи. project_runtime,
   GameHost, headless_game и source-free package тесты тоже существуют. ENG-01–03
   нельзя начинать с их повторной реализации.
3. Навигация — bounded четырёхсвязный grid, не подтверждённая общая 3D-навигация.
4. Общая физика требует проверки scene binding и экспозиции joints/triggers/CCD.
   Наличие сложных физических модулей не закрывает пользовательский workflow.
5. Сохранения имеют много участников; нужно проверить одну атомарную транзакцию
   всей активной сессии, совместимость ресурсов и реальные disk slots.

**Следующий ENG-01:** перечитать текущий project API и выполнить девять существующих
project tests из project.rs на зафиксированном дереве, затем реальный CLI create/
validate/relocate/overwrite из другого cwd. Проверить поддерживаемые input/build
настройки и схему миграции descriptor. Исправлять выявленные пробелы в существующих
владельцах. Native UI-проверку вынести за ограничение текущей headless-сессии.

## Воспроизведение

Из корня репозитория (Python 3.11+, установленный pinned Rust toolchain):

```sh
python3 tools/audit_engine_readiness.py
python3 tools/test_audit_engine_readiness.py
python3 tools/audit_engine_readiness.py --run-checks --output /tmp/voxy-eng00-fresh-report
```

Без `--run-checks` выполняется только валидация реестра. С флагом запускается
фиксированный список CPU-проверок; команды из JSON не исполняются. Cargo использует
offline/locked, изолированный target и два build jobs. Существующий report.json
не перезаписывается. Неизменность хешей проверяется до/после; изменение исходников
или неуспешная проверка возвращают ошибку. Пограничный snapshot не является
транзакционной блокировкой общего checkout; при конкурирующих изменениях повторить
проверку на замороженной копии. Full workspace gates остаются отдельной работой.
