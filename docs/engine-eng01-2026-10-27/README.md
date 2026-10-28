# ENG-01: создание, открытие и сохранение проекта

Дата: 27 октября 2026. Статус: ENG-01 завершён в headless-области.
Область: создание, открытие и сохранение проекта. Native UI, GPU,
экспорт приложения и последующие этапы не входят в эту квалификацию.

## Реализованный контракт

Существующие `project::create` и `project::open` сохраняют формат descriptor v1,
пустую сцену, manifest, root-relative пути, реестр компонентов и prefab expansion.
Добавлен `OpenProject::save_descriptor(candidate, registry)`:

- Проверяет имя, версию, размер, manifest, стартовую сцену и prefab-зависимости
  до записи; descriptor, manifest и scene не могут быть одним путём.
- Публикует только descriptor через существующий same-directory atomic writer.
  Scene/manifest не перезаписывает и не разворачивает prefab в плоскую сцену.
- Отклоняет descriptor, изменившийся после открытия; повторное открытие разрешает
  новую запись. Ошибка проверки сохраняет файл и состояние OpenProject.
- После успешного сохранения обновляет наблюдение, поэтому повторный save разрешён.
  Допускается выбор существующей вложенной стартовой сцены.

Это отдельная транзакция descriptor, не атомарный multi-file save или Save As.
Сцену сохраняет существующий authoring owner с историей и source observations.
Его повторные save/undo/redo/reopen и удержание правок при внешнем конфликте
проверяются отдельно. Некорректные изменения нельзя обходить путём сырого overwrite.

Создание теперь проверяет размер сериализованного descriptor до создания каталога:
слишком длинное имя больше не создаёт проект, который невозможно открыть.
Открытие дополнительно перечитывает descriptor после валидации зависимостей,
отклоняя обнаруженное изменение во время загрузки.

## Headless CLI

```sh
voxy_editor --help
voxy_editor --create-project /absolute/new-game "My Game"
voxy_editor --validate-project /absolute/new-game/project.voxy
voxy_editor --rename-project /absolute/new-game/project.voxy "Renamed Game"
```

Эти ветки возвращаются до создания App/EventLoop, GPU и аудиоустройства.
`--rename-project` — реальный потребитель нового API сохранения descriptor.
Обычные `--project PATH` и native game launch продолжают открывать окна;
они здесь не запускаются. CLI работает из cwd `/`, с Unicode и пробелами в путях.

## Проверки и доказательства

- [Project tests](project-tests.log): исходные 9 и новые 5 регрессий.
- [Prefab project tests](prefab-tests.log): композиция, сохранение, перенос,
  ошибки зависимостей и source-free package через существующий owner.
- [Clippy](clippy.log): editor library и основной binary, `-D warnings`.
- [Build](build.log): основной CLI binary; сборка не является GPU-проверкой.
- [CLI report](cli.json): 19 вызовов passed; create/validate/rename, отказ от overwrite/пустого имени,
  повторный save, вложенная сцена, битые версии/зависимости и перенос проекта.
- [Идентичность исходников и команды](checks.json): ревизия, хеши до/после,
  результаты команд; dirty tree не приравнивается к чистому HEAD.

Полный `cargo fmt --all -- --check` проверен отдельно: [лог](workspace-fmt.log).
Результат: exit 1 на изменениях вне ENG-01 (включая physics). Изменённые
project.rs/main.rs проходят [scoped rustfmt](scoped-fmt.log).
Изменения вне ENG-01 не форматируются автоматически. Для project.rs/main.rs
выполняется отдельный rustfmt check. Полные workspace тесты не запускаются:
этот этап использует только явно выбранные headless-сценарии.

Воспроизведение (изолированный target, pinned toolchain из rust-toolchain.toml):

```sh
CARGO_TARGET_DIR=/tmp/voxy-eng01-20261027-target CARGO_BUILD_JOBS=2 cargo test --locked --offline -p voxy_editor --lib project::tests:: -- --test-threads=1
CARGO_TARGET_DIR=/tmp/voxy-eng01-20261027-target CARGO_BUILD_JOBS=2 cargo test --locked --offline -p voxy_editor --lib project_prefab_tests:: -- --test-threads=1
CARGO_TARGET_DIR=/tmp/voxy-eng01-20261027-target CARGO_BUILD_JOBS=2 cargo clippy --locked --offline -p voxy_editor --lib --bin voxy_editor -- -D warnings
CARGO_TARGET_DIR=/tmp/voxy-eng01-20261027-target CARGO_BUILD_JOBS=2 cargo build --locked --offline -p voxy_editor --bin voxy_editor
python3 tools/qualify_project_headless.py --binary /tmp/voxy-eng01-20261027-target/debug/voxy_editor --output /tmp/voxy-eng01-fresh-cli
```

## Границы гарантии и дальнейшая работа

Пути считаются доверенными локальными путями. Наблюдение изменений не является
межпроцессным compare-and-swap: другой writer может попасть в интервал между
проверкой и rename. Directory crash durability для descriptor replacement не
заявляется; сохранение использует существующий writer и его платформенные
ограничения. Создание каталога по-прежнему не является атомарным directory rename.

В рамках ENG-01 завершён headless create/open/save текущего формата проекта.
Native UX создания/открытия/сохранения остаётся ENG-08; game execution и сборка
проектного кода — ENG-02/03. Input bindings/build profiles не добавляются в schema v1
попутно: их проектирование и потребители относятся к соответствующим этапам.
Следующий этап: квалификация общего lifecycle редактора и standalone (ENG-02).
