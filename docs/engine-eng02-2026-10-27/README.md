# ENG-02: общий lifecycle редактора и standalone

Дата: 27 октября 2026. Статус: завершён для общего headless lifecycle
Play/Stop/project/package. Native event loop, окна, GPU и аудиоустройства
не запускались; их lifecycle остаётся отдельным платформенным барьером.

## Исправления и контракт

Play и standalone используют существующие App/PlaySession, GameHost,
SceneSimulation и один animation/gameplay phase schedule. Новый runtime рядом
с ними не создаётся; публичные behavior callbacks и fixed-step частота сохранены.

1. Все fallible стадии запуска после отделения runtime graph теперь имеют общий
   rollback: закрытие play audio, Stop simulation, очистка runtime owners и
   восстановление authoring/history. Simulation переходит во владение PlaySession
   до host setup, поэтому ошибка более поздней стадии не теряет `on_destroy`.
2. Stop очищает playing flag, tick count, input, animation, physics, liquid,
   angular motion, UI handlers и follow state. Очистка owners выполняется и при
   возвращённой ошибке simulation Stop. Authoring восстанавливается до возврата
   ошибки Stop. Pending commands и fractional clock не переходят в новый Play.
3. Повторный `start_standalone` на активной/ожидающей сессии возвращает ошибку,
   сохраняя текущую игру, вместо вызова toggle и неожиданного Stop. Ошибка первого
   standalone-start восстанавливает прежний standalone flag.
4. Проверка неотрицательного конечного elapsed выполняется в общем advance_game
   до UI dispatch, synchronization и simulation. HeadlessGame использует её же.

Успешный Play создаёт свежие generational handles. Stop возвращает авторские
данные, не runtime edits, и инвалидирует runtime handles. Behavior awake/start/
fixed/destroy вызываются по существующему контракту; явный shutdown и последующий
Drop не дублируют destroy. Последующие Play получают новый clock и callbacks.

## Воспроизведение найденного дефекта

[Неуспешный тест до исправления](late-start-before.log): host успешно создаёт
behavior и затем добавляет некорректный AngularMotion. Следующая стадия запуска
отклоняет motion, но прежняя реализация теряет локальную simulation, оставляет
runtime graph и playing flag; чтение authoring возвращает InvalidTransform.

Новая регрессия проверяет восстановление исходного документа, отсутствие runtime
owners, ровно один destroy и успешный повторный Play после исправления входа.
Отдельные проверки покрывают ошибку UI setup и повторный standalone-start.

## Финальные результаты

[Контрольный повтор](repeat-01/checks.json): все 8 команд успешны; хеши
Rust/TOML/WGSL исходников до и после совпали. Это dirty checkout с зафиксированными
хешами, а не заявление о чистом HEAD. Первый прогон [сохранён](checks.json):
все команды прошли, но параллельно добавился `ui_text_checkpoint.rs`; он не выбран
финальным доказательством неизменности исходников.

| Набор | Результат | Лог |
| --- | --- | --- |
| GameHost lifecycle и новые регрессии | 9 passed | [host](repeat-01/host-qualified.log) |
| Создание/open/save/Play/package проекта | 14 passed | [project](repeat-01/project-qualified.log) |
| Headless input/UI/PCM и package replay | 3 passed | [headless](repeat-01/headless-qualified.log) |
| Follow camera и rollback | 3 passed | [camera](repeat-01/follow-camera-qualified.log) |
| Отмена pending Play без native device | 1 passed | [pending](repeat-01/pending-qualified.log) |
| Scene library и integration suites | 122 passed | [scene](repeat-01/scene-qualified.log) |
| Strict editor lib/bin Clippy | passed | [Clippy](repeat-01/clippy-qualified.log) |
| Strict editor lib/tests/bin Clippy | passed | [test Clippy](clippy-tests.log) |
| Основной CLI binary | built | [build](repeat-01/build-qualified.log) |
| Реальные CLI project/package проверки | 6 вызовов passed | [CLI](cli.json) |

Всего 152 Rust-теста. Host parity проверяет одинаковые сцены, fixed ticks и
полную последовательность callbacks при zero-step, дробных шагах и catch-up
перегрузке. После удаления source tree packaged replay даёт тот же результат.
Проверяются также некорректный elapsed, stale handles, двойной cleanup, pending
commands/input и отсутствие переноса дробного времени после Stop.

CLI выполняет create, два project game-check, build и два source-free package
game-check из cwd `/`. Четыре game-check дают одинаковые документы после 120 ticks;
конечный поворот проверяется независимо по аналитическому углу 4 радиана.
Авторский scene-файл не изменяется. SHA-256 CLI binary сохранён и перепроверен.

[Scoped rustfmt](scoped-fmt.log) и diff-check проходят. Общий
[workspace fmt](workspace-fmt.log) возвращает exit 1 на изменениях вне ENG-02,
начиная с physics. Чужие файлы не форматировались. Полные workspace тесты не
запускались; здесь выполняются только явно выбранные сценарии без устройств.

## Воспроизведение

Использовать pinned toolchain из rust-toolchain.toml и изолированный target:

```sh
CARGO_TARGET_DIR=/tmp/voxy-eng01-20261027-target CARGO_BUILD_JOBS=2 cargo test --locked --offline -p voxy_editor --lib game_host_tests:: -- --test-threads=1
CARGO_TARGET_DIR=/tmp/voxy-eng01-20261027-target CARGO_BUILD_JOBS=2 cargo test --locked --offline -p voxy_editor --lib project::tests:: -- --test-threads=1
CARGO_TARGET_DIR=/tmp/voxy-eng01-20261027-target CARGO_BUILD_JOBS=2 cargo test --locked --offline -p voxy_editor --lib headless_game::tests:: -- --test-threads=1
CARGO_TARGET_DIR=/tmp/voxy-eng01-20261027-target CARGO_BUILD_JOBS=2 cargo test --locked --offline -p voxy_editor --lib follow_camera_tests:: -- --test-threads=1
CARGO_TARGET_DIR=/tmp/voxy-eng01-20261027-target CARGO_BUILD_JOBS=2 cargo test --locked --offline -p voxy_scene --lib --tests
CARGO_TARGET_DIR=/tmp/voxy-eng01-20261027-target CARGO_BUILD_JOBS=2 cargo clippy --locked --offline -p voxy_editor --lib --tests --bin voxy_editor -- -D warnings
CARGO_TARGET_DIR=/tmp/voxy-eng01-20261027-target CARGO_BUILD_JOBS=2 cargo build --locked --offline -p voxy_editor --bin voxy_editor
python3 tools/qualify_lifecycle_headless.py --binary /tmp/voxy-eng01-20261027-target/debug/voxy_editor --output /tmp/voxy-eng02-fresh-cli
```

CLI qualifier допускает только create/build и project/package с обязательным
`--game-check`; обычный native game/editor launch не допускается. Результаты
команд сохраняются до assertions, неудачная квалификация остаётся в отчёте.

## Границы и следующий этап

Это завершение headless ENG-02: общий запуск, lifecycle, изоляция Play/Stop,
возвращаемые ошибки setup и повторный запуск. Восстановление после произвольной
panic в пользовательском callback, native suspend/resume/device loss, динамическая
смена сцен и shipping game-code toolchain этим набором не квалифицируются.
CPU-проверки не доказывают кадры изображения или FPS. Native аудио отключено,
PCM/UI/physics/animation проверяются через существующие headless owners.

Следующий ENG-03: зависимости ресурсов, безопасный reload, бюджеты и packaging
при изменениях/ошибках импорта. Общие контракты scene transitions и платформенного
lifecycle расширять отдельными проверяемыми задачами, сохраняя этот срез.
