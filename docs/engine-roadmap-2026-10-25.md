# Voxy: план реализации полноценного 3D-движка

## Актуальный checkpoint: 27 октября 2026

ENG-00 завершён как этап аудита: [реестр 22 областей, пробелы и свежие CPU-проверки](engine-audit-2026-10-27/README.md).
Машиночитаемый реестр, валидатор, логи и исходные хеши находятся рядом с отчётом.
ENG-01 завершён для headless создания, открытия и сохранения проекта:
[контракт, ограничения и результаты](engine-eng01-2026-10-27/README.md).
Проверено 14 project-тестов, 2 prefab-теста и 19 реальных CLI-вызовов из cwd `/`;
strict editor lib/bin Clippy и scoped rustfmt проходят. Общий workspace fmt
пока не проходит на изменениях вне ENG-01; полный лог сохранён в отчёте.
ENG-02 завершён для общего headless lifecycle Play/Stop/project/package:
[исправления rollback, повторного запуска и 152 теста](engine-eng02-2026-10-27/README.md).
На неизменных исходниках прошли CPU suites, strict Clippy (включая тесты)
и 6 реальных CLI-вызовов. Native lifecycle и произвольные callback panic этим
этапом не квалифицированы. Native авторский UX остаётся частью ENG-08.
ENG-03–13 не закрыты.
Ниже сохранён исходный план 25 октября;
его утверждение об отсутствии project entrypoint уже устарело: в текущем дереве
существуют create/validate/project/build/package, GameHost и headless game.
Следующий шаг — ENG-03: import/reload, зависимости, бюджеты и упаковка ресурсов.
В текущем этапе окна и GPU-прогоны не запускались.

Дата: 25 октября 2026. Статус: план реализации, без заявления о готовности.
Исходная ревизия при аудите: `0a318afc2`; рабочее дерево содержит параллельные
изменения рендера и street_walk. Они не входят в результат этого аудита.

## Цель и границы выпуска

Движок готов к первому выпуску, когда пользователь может создать проект,
импортировать ресурсы, собрать сцену, написать игровое поведение, отладить игру
в редакторе и выпустить самостоятельное приложение. Доказательство — несколько
разных проектов на общих публичных API, без изменения исходников движка.

Первый выпуск: настольные 3D-игры, редактор, Rust-код игры, обычная и voxel-геометрия.
Локальная проверка начинается с macOS; Windows/Linux требуют собственных запусков.
Web, Android/iOS, XR, сеть и дополнительный scripting остаются обязательными
направлениями программы, но не блокируют первый настольный вертикальный срез.
Сроки назначать после декомпозиции и оценки первого среза, не по числу crate/API.

Архитектурная основа — [ADR 0002](adr/0002-general-engine-architecture.md)
и [ADR 0004](adr/0004-workspace-layering.md). Существующие рабочие реализации
сохраняются; новый общий слой вводится только вместе с реальным потребителем.

## Что подтверждено текущим чтением кода

Это аудит интерфейсов, а не свежая проверка исполнения. Старые отчёты сохраняют
силу только для указанных в них ревизий и сценариев.

| Область | Основание в репозитории | Что ещё нужно подтвердить или закончить |
| --- | --- | --- |
| Вход в редактор | `crates/voxy_editor/src/main.rs`: файл модели или manifest + asset ID, опциональная сцена | Общее создание/открытие проекта без обязательного bootstrap через модель |
| Сцены и prefab | `voxy_scene`: document, prefab_document, history, transaction; редактор: authoring_session, prefab_authoring | Полный пользовательский путь, миграции, повторное открытие и работа с конфликтами |
| Runtime | `voxy_scene`: simulation, table, commands, schedule, behavior; `voxy_gameplay` | Общая композиция проекта, идентичные контракты Play и standalone, смена сцен |
| Ресурсы и сборка | `voxy_assets/src/package.rs`, editor `export_game_package`, `packaged_game.rs`, `tools/macos/package-game.sh` | Общий build workflow проекта, игровая логика, чистая машина, другие платформы |
| UI и звук | `voxy_gameplay`: ui_runtime, ui_actions, audio_runtime; editor: ui_live, audio_session | Законченная авторская настройка и совместная проверка в собранной игре |
| Графика/физика/анимация | Существующие отдельные домены, editor adapters и примеры | Матрица реальных функций и общая сцена; наличие модулей не закрывает область |

Подробные прежние доказательства и ограничения: [цель от 2 октября](engine-goal-2026-10-02.md).
Не переносить автоматически в новый реестр старые строки «remaining»: часть
возможностей была реализована в последующих разделах того же документа.

## Правила учёта готовности

Каждая задача получает ID, зависимости, владельца данных, публичный контракт,
критерий проверки и ссылку на результат с ревизией. Статусы: запланировано,
реализовано, проверено, частично, заблокировано. Сейчас этапы ниже запланированы.
Отдельно фиксировать: core API, runtime, редактор, сохранение, standalone и
платформенную проверку. Один успешный столбец не закрывает остальные.

## Этапы и порядок зависимостей

| ID | Результат | Зависимости | Критерий завершения |
| --- | --- | --- | --- |
| ENG-00 | Реестр возможностей и воспроизводимые проверки | Нет | Для каждого домена указаны реальные потребители, тесты, ограничения и незакрытые интеграции |
| ENG-01 | Создание и открытие проекта | ENG-00 | Пустой проект создаётся без перезаписи чужих файлов, открывается по project-файлу, сохраняет стартовую сцену и настройки |
| ENG-02 | Общий запуск проекта | ENG-01 | Play и standalone используют одинаковые регистрации компонентов, actions и порядок фаз; Stop восстанавливает authoring |
| ENG-03 | Надёжный import/reload/build ресурсов | ENG-01, ENG-02 | Зависимости импортируются фоново, ошибки сохраняют последнюю рабочую версию, пакет запускается после удаления исходных ресурсов |
| ENG-04 | Общая 3D-физика в сцене | ENG-02 | Динамические/кинематические тела, контакты, queries, triggers, joints и character controller доступны в редакторе и игре |
| ENG-05 | Совместный графический pipeline | ENG-02, ENG-03 | Материалы, свет, тени, прозрачность, skinning, instancing, LOD и culling работают вместе с проверкой изображения |
| ENG-06 | Анимация и пользовательская игровая логика | ENG-02, ENG-03, базовая ENG-04 | Сохранённые state machines/смешивание и Rust-компоненты проекта работают одинаково в обоих режимах |
| ENG-07 | Игровые UI, звук, навигация и сохранения | ENG-02–04, ENG-06 | Меню/HUD, spatial audio, NPC и save/load совместно работают в standalone |
| ENG-08 | Полный авторский workflow редактора | Развивается вместе с ENG-01–07 | Сцену и её компоненты можно создать и настроить без ручной правки документов |
| ENG-09 | Большой мир и voxel-интеграция | ENG-03–06 | Движение, загрузки, разрушение и перестройка не публикуют смешанные версии и не накапливают неограниченную работу |
| ENG-10 | Контрольные игры и настольный выпуск | ENG-01–09 | Три проекта собираются и запускаются отдельно; проверены поддерживаемые desktop-платформы |
| ENG-11 | Web, mobile и XR | Стабильные ENG-02/03/05 | Реальная упаковка и запуск, input, suspend/resume и восстановление ресурсов на каждом целевом устройстве |
| ENG-12 | Сетевой runtime | Версионированные ENG-02/04/06/07 | Dedicated server, authority, replication, prediction/reconciliation проверены при задержках, потерях и reconnect |
| ENG-13 | Дополнительный scripting и расширения | Стабильный API ENG-06 | Версионированный lifecycle, команды/события, диагностика, лимиты и корректная отмена; язык выбирается отдельным ADR |

## Ближайший исполнимый срез: ENG-01–03

1. Проверить существующие project/manifest/import контракты и выбрать их владельца.
   Не создавать второй asset catalog или второй формат сцены.
2. Определить версионированный project descriptor: стартовая сцена, manifest,
   настройки приложения, input и build profiles. Пути разрешать относительно
   проекта; ошибки версии и отсутствующих зависимостей должны быть явными.
3. Добавить общий create/open/validate путь и CLI без графической инициализации.
   Создание атомарно, с отказом при конфликте файлов. Нужна настоящая пустая
   сцена без фиктивной обязательной модели.
4. Подключить descriptor к существующему редактору, сохранив старый запуск от модели.
5. Вынести общие регистрации и запуск игры из редакторского владения в runtime/
   gameplay слой по ADR 0004. Переносить по одному реальному потребителю.
6. Подключить существующий экспорт пакета и standalone loader к проекту.
   Пакет должен включать объявленные зависимости и не читать исходники в обход.
7. Создать минимальный проект с собственным Rust-поведением, кнопкой UI и
   изменением сцены. Проверить сохранение, повторное открытие, Play/Stop и сборку.

Автоматические проверки среза: missing/unknown schema, повреждённый manifest,
битые ссылки, конфликт создания, перенос проекта, смена cwd, отсутствие исходников
после сборки, незарегистрированный компонент/action, старые handles после Stop,
неудачный reload, повторный запуск и ошибки доступа. Формат проекта фиксировать
после проверки существующих контрактов, не заранее отдельным несовместимым API.

## Обязательная детализация остальных этапов

- ENG-04: audit существующих solvers; единый scene binding; collision layers,
  friction/restitution, sleeping, CCD, compound/static triangle collision;
  события через барьеры; регрессии стопок, быстрых тел, склонов и платформ.
- ENG-05: корректный цветовой pipeline, материалы/окружение, несколько камер,
  resize/device recovery, terrain, частицы и decals. Продвинутые эффекты — после
  устойчивой общей сцены. Culling сверять с консервативным oracle.
- ENG-06: clips, переходы, blend trees, layers/masks, events, root motion,
  retargeting, IK; компоненты проекта с сериализацией/инспектором; gameplay API
  не должен требовать правки движка для каждого нового поведения.
- ENG-07: layout/fonts/focus/gamepad; buses/listeners/streaming audio;
  динамические препятствия и bounded navigation jobs; версии сохранений и
  восстановление ссылок. Проверять общую игру, а не только отдельные примеры.
- ENG-08: hierarchy, поиск, multiselect, copy/duplicate, gizmos/snapping,
  prefab create/instantiate/revert/apply, настройки импорта, визуальные редакторы
  материалов/анимации, ошибки, profiler, autosave и восстановление после сбоя.
- ENG-09: отмена stale jobs, согласованная публикация affected chunks,
  загрузка только изменившихся мешей, лимиты очередей и old+new памяти,
  потоковая загрузка мира, разрушение во время rebuild и отрицательные координаты.
- ENG-10: 3D action, разрушаемый voxel-мир и невоксельная интерактивная сцена.
  Общие API; инструкция с чистого checkout; релизные пакеты и лицензии ресурсов.
- ENG-11–13: отдельные acceptance-матрицы; компиляция без запуска не считается
  платформенной квалификацией, сетевым доказательством или готовым scripting.

## Проверки, производительность и выпуск

Каждый этап: focused regressions → интеграционный сценарий → необходимые workspace
fmt/Clippy/tests. Точный набор и ограничения сохранять в отчёте; не выдавать
исторический зелёный результат за проверку текущего дерева. Не запускать тяжёлые
проверки одновременно с чужими performance-замерами.

Производительность измерять сквозным CPU replay и, отдельно, GPU/frame pacing.
Учитывать scheduled CPU, ожидания и очереди раздельно. Для принятия оптимизации:
не менее пяти чередующихся A/B-пар, выигрыш целевой метрики ≥5%, ухудшение p99
контрольных сценариев ≤3%; сохранять raw данные, выбросы и отклонённые кандидаты.
Качество изображения и точность физики не снижать ради результата.

Цель 120 FPS: physical GPU, 1920×1080, p99 ≤8,33 мс в четырёх контрольных сценариях
после прогрева; CPU replay и offscreen throughput сами по себе этого не доказывают.
До отдельного согласования не открывать окна и не запускать GPU-прогоны.
Native audio/device проверки также выделять отдельно от headless PCM проверок.

Выпуск блокируют потеря данных, расхождение Play/standalone, повреждённая сборка,
неограниченные очереди/утечки и отсутствие воспроизводимого основного workflow.
Остальные ограничения публиковать явно по платформам и возможностям.

## Первый следующий результат

ENG-00: заполнить функциональную матрицу по исходникам и актуальным проверкам.
Затем ENG-01: headless create/open/validate проекта с реальным пустым scene-файлом
и регрессиями безопасности файлов. Текущий документ — согласованный порядок
реализации и критерии, а не доказательство выполнения этих этапов.

## Обновление 26 октября: первый реализованный шаг

[Реестр текущих доказательств](production-readiness-2026-10-26.md) фиксирует
ENG-00/ENG-01 как частичные. Добавлены headless CLI `--create-project` и
`--validate-project`, существующие форматы manifest/scene и пустой шаблон
`examples/empty-project/project.voxy`. Четыре focused-теста, проверка реального CLI
после переноса/смены cwd и Clippy с `-D warnings` прошли. Добавлен вход `--project` в существующий
редактор без обязательной модели. Шесть тестов проекта включают реального владельца
редактирования: создание объекта, undo/redo, сохранение и открытие после переноса.
Полный набор редактора: 198 прошли, 17 пропущены; визуальная проверка окна пока
не проводилась. Общий Play/standalone и сборка из project descriptor остаются
незавершёнными; ENG-01 и вертикальный срез не закрыты.


## Обновление 26 октября: общий runtime и пакет проекта

Добавлены `--project ... --game-check/--game`, `--build-project` и
`--package ... --game-check/--game`. Новый и прежний входы используют общую
инициализацию standalone; Play работает с тем же владельцем сцены и симуляцией.
Проверены совпадение движения после 120 фиксированных шагов, восстановление Stop,
инвалидация старых handles и пользовательские codecs в editor/standalone/package.

Реальный CLI из cwd `/` собрал пакет вращающейся модели, отказался перезаписать
результат и запустил его после удаления исходного каталога; итоговые документы
сцены совпали. Регрессия glTF отдельно проверяет внешний бинарный буфер в пакете.
201 тест редактора прошёл, 17 пропущены; Clippy всех targets чистый. Логи, хэши,
воспроизводимый скрипт и границы проверки — в [реестре](production-readiness-2026-10-26.md).

ENG-02/ENG-03 остаются частичными: общий интерактивный проект с input, физикой,
UI actions, звуком, пользовательской логикой, prefab/reload и нативной сборкой
на чистой машине ещё не квалифицирован. 120 шагов симуляции не являются 120 FPS.


## Обновление: исполняемая игровая логика проекта

Добавлен общий `GameHost` для codecs, behaviors и UI setup. Один контракт
используется в editor Play, standalone, build и проектном пакете. Два новых теста
проверяют повторный Play, реальное движение, lifecycle, остановку частичного
подключения с ошибкой и повторный запуск. 203 теста редактора прошли, 17 пропущены;
Clippy всех targets с `-D warnings` прошёл.

Отдельный Rust-пример `project_game` использует публичный API. Реальный бинарник
из cwd `/` подтвердил совпадение движения после 120 fixed ticks в проекте и
пакете после удаления исходников; неверная конфигурация не публикует пакет.
Полные доказательства и ограничения — в production-readiness-2026-10-26.md.
ENG-02/ENG-06 продвинуты, но общий интерактивный сценарий и полный план не закрыты.


## Обновление: совместный интерактивный сценарий

Добавлен публичный `HeadlessGame` поверх того же игрового владельца и общий
владелец извлечённого пакета. Проект `examples/interactive-project` объединяет
персонажа, пол, стену, UI callback и пространственный звук. Одинаковый ввод
проверен в editor Play, standalone и пакете после удаления исходного каталога:
прыжок/приземление, отсутствие проникновений в пол/стену, UI command barrier,
сброс ввода/захвата при потере фокуса и офлайн PCM совпадают. Неверные временные
и координатные значения отклоняются без изменения состояния.

205 тестов редактора прошли, 17 пропущены; Clippy всех targets чистый. Отдельный
бинарник `interactive_project` через публичный API подтвердил source-free package
replay. Его 180 fixed ticks и 144000 PCM frames не являются кадрами GPU или
проверкой аудиоустройства. Полные доказательства — interactive-qualification.json
и interactive-cli-proof.json в production-readiness-evidence-2026-10-26/.

Это частичная квалификация ENG-02/03/04/07, не закрытие этих этапов целиком.
Нативная визуальная проверка, анимация, плавная игровая камера, prefab/reload,
полная физика, выпуск на платформах и целевой FPS остаются в плане.


## Обновление: вложенные префабы через project descriptor

Исправлен вход проекта: он использует существующий PrefabSceneDocument и core
expansion вместо отклонения поля instances плоским SceneDocument. Наблюдаемые
входы ограничены и проверяются перед публикацией, лимиты topology объединены с
редакторским load/save/place/revert. OpenProject.scene возвращает expanded view;
сохранение редактора оставляет instance links и overrides.

Два новых проектных теста проверяют override/inheritance, undo/redo, стабильные ID,
отказ с сохранением рабочего состояния, восстановление зависимости, перенос,
Play/Stop и source-free package. Проверены отказы cycle/unknown/stale/version/depth.
Реальные CLI валидировали и проиграли вложенный интерактивный проект после
переноса и пакет после удаления исходников. 207 тестов прошли, 17 пропущены;
Clippy всех targets чистый. Доказательства: prefab-qualification.json и
prefab-cli-proof.json в production-readiness-evidence-2026-10-26/.

ENG-01/02/03/08 остаются частичными. Нативный prefab workflow, все разновидности
переопределений/конфликтов, камера/анимация, платформы и целевой FPS не закрыты.


## Обновление: плавная follow-камера

Добавлен gameplay descriptor `game.follow-camera.v1` со стабильной ссылкой на
цель и prefab remapping. Камера использует тот же интерполированный world pose,
что и отрисовка, с отдельной f64-историей сглаживания; физика и сохранённые
компоненты не изменяются. Проверены движение между fixed ticks, аналитический
ответ, сохранение малых остатков, source-free package trace, Stop/reset,
недоступная цель и отказ при нескольких активных follow-камерах.

209 тестов редактора прошли, 17 пропущены; 3 математических теста прошли,
Clippy всех targets чистый, CLI проекта и пакета совпали. Первый полный gameplay
прогон дал 180 passed / 1 failed из-за неверного ожидания нуля в граничном тесте;
исправленный focused-тест проходит, полный повтор ещё идёт (см. camera-qualification.json).
Это не доказательство нативной визуальной плавности или FPS. Inspector/target
picker, orbit smoothing, camera collision/blends и анимация остаются в плане.


## Обновление: follow-камера настраивается через редактор

Поля half-life и offset используют существующий generic inspector. Добавлены
Add/Remove follow camera и Pick у поля target; выбор проходит через hierarchy
со стабильными ID и эпохой запроса. Старый клик после закрытия/повторного открытия
picker не меняет новый запрос. Добавление view+follow атомарно по history;
remove сохраняет view. Проверены numeric edit, ссылки, undo/redo, save/reopen,
вложенный target override, reset/undo и отмена при переходе в Play.

Валидация subtree deletion теперь выполняется до изменения живого графа; ссылка
на удаляемую цель отклоняет действие без инвалидации текущих handles. Старый путь
тоже откатывал failed commit, но уже после изменения графа. Исправлен учёт
object-ID counter после повторного открытия обычных объектов; G не удаляет view
другой follow-камеры. 212 тестов редактора прошли, 17 пропущены, Clippy чистый,
редактор собран. Полный gameplay-повтор завершился: 181 passed / 0 failed.

Нативная визуальная проверка UI, orbit smoothing, camera collision/blends,
анимация персонажа и остальная программа остаются незакрытыми. Доказательства:
camera-authoring-qualification.json и production-readiness-2026-10-26.md.


## Обновление: locomotion requests и реальный пробел импорта

Добавлен `editor.model-locomotion.v1`: запросы idle/move/rise/fall используют
предыдущее принятое состояние CharacterPhysics и pending jump edge. Визуальная
модель может быть дочерней по отношению к физическому телу. Это адаптер запросов
к существующим ModelPlayback/crossfade и AnimationRuntime, не новый проигрыватель
или замена GraphPlayer. При failed tick восстанавливаются изменённые запросы;
при reload проверяются все связанные клипы, включая сейчас не проигрываемые.

214 тестов редактора прошли, 17 пропущены; Clippy чистый, бинарники собраны.
Synthetic regression проверяет четыре отдельных clip names, short jump,
crossfade, accepted skeletal matrices, rollback/retention, Stop и точное
совпадение кадров пакета после удаления исходников. Public CLI это подтверждает
на явно регрессионном rig; реализм и готовая анимация персонажа этим не закрыты.

Оригинальный RobotExpressive получен из официального three.js с pinned commit,
CC0 README и SHA-256. Импорт воспроизводимо отклоняется: несколько mesh instances,
два skin, morph primitives и weight channels сейчас не поддержаны ModelAsset.
Ничего из модели не удалено ради прохождения. Следующий приоритет — полный путь
multi-mesh/skin + morph geometry/animation/CPU/GPU, затем приёмка реального
персонажа. Доказательства: locomotion-qualification.json, locomotion-cli-proof.json,
locomotion-upstream-import-gap.json в production-readiness-evidence-2026-10-26/.
ENG-06, визуальное качество, FPS и полный план остаются незавершёнными.


## Обновление: исходный morph-контент в явном CPU-потребителе

ENG-03/ENG-06 продвинуты: общий glTF-импортер получил сохранение morph-дельт,
начальных весов узла/меша и каналов STEP/LINEAR/CUBICSPLINE. Явный
`MorphModelAsset` позволяет проверить результат на CPU, сохраняя базовый ассет
приватным; обычный импорт пока строго отклоняет morph, чтобы существующий
GPU/runtime-путь не начал молча его игнорировать.

Неизменённый RobotExpressive прошёл 854 CPU-выборки всех 14 клипов. Его
собственные каналы весов постоянные нулевые; три исходные формы отдельно
проверены явными весами без правки GLB. Это не визуальная приёмка и не FPS.
Подробные границы и доказательства — `morph-import-proof.json` и раздел glTF morph
в `production-readiness-2026-10-26.md`. Следующий обязательный шаг — GPU-данные и
принятый снимок весов, связанный с существующими часами и прерыванием переходов.
Общий план остаётся незавершённым.


## Обновление: morph-свойства в едином снимке Animator

ENG-06 продвинут: `AnimatorFrame` содержит свойства, вычисленные по тем же f64
часам и весу перехода, что и скелет. Прерывание сохраняет снимок свойств; ошибки
оставляют прежние часы и значения. Подготовленные rigid-prefix кадры и проверка
их владельца включают эти свойства. Импортированный MorphModelAsset подключает
кривые к этому Animator; CPU-потребитель использует готовый кадр без пересэмплинга.

335 тестов анимации и 215 редактора прошли (17 editor-тестов пропущены), строгий
Clippy animation/model/editor/demos/gameplay прошёл. Неизменённый исходный GLB
дополнительно прошёл 112 кадров Animator с прерываемыми переходами. GPU morph и
обычный editor/runtime-путь ещё не подключены; визуальная приёмка и FPS открыты.
Доказательства: `morph-frame-qualification.json` и `morph-frame-proof.json`.


## Обновление: точная GPU morph-деформация

ENG-05/ENG-06 продвинуты: SceneSkinner получил отдельный ленивый morph pipeline,
общие неизменяемые дельты и независимые веса экземпляров. Сумма исходных f32
произведений вычисляется целочисленно с одним округлением перед существующим
skinning; приёмка ресурсов/позы предшествует записи в GPU. Обычный shader/layout
моделей без morph сохранён.

Физическая GPU-проверка на Apple M4 Max прошла: независимые экземпляры,
компенсация больших чисел, нормали, субнормальные значения и 256 целей.
41 численный случай совпал с независимым рациональным oracle. Все три
morph-примитива неизменённого RobotExpressive прошли 12 кадров с явными весами;
проверены 20 904 вершины. Полного изображения модели или результата FPS это
не устанавливает. Обычный skeletal GPU-контроль также прошёл.

358 тестов рендера, 6 morph-регрессий модели и 215 тестов редактора прошли;
строгий Clippy всех целей render/model/editor чистый. Доказательства и границы:
`morph-gpu-qualification.json` и `morph-gpu-proof.json`. Далее — обычный
editor/runtime import и принятые GPU-кадры, reload/package/rollback, статическая
morph-привязка, LOD/culling с morph и визуальная/производительная приёмка.
Все этапы программы ещё не завершены.


## Обновление: morph в обычном проектном пути

ENG-03/ENG-06 продвинуты до общего ModelAsset, обычного импорта редактора,
ModelPlayback, принятого кадра, GPU-владельца, безопасного reload и пакета.
Оболочка MorphModelAsset использует того же владельца данных. Старые pose-only
API строго отклоняют morph; редактор использует frame-aware импорт/вывод.

Неизменённый RobotExpressive прошёл обычный проект, контроллер, сборку и запуск
пакета после удаления исходников: сцена и принятые кадры совпали после 240 тиков.
Его полный GPU-владелец проверен для всех 19 примитивов: 21 642 вершины за три
кадра. Меняющиеся веса и reload отдельно проверены на явном синтетическом риге;
исходные weight-каналы RobotExpressive нулевые. Это не приёмка качества движения
или изображения.

336 тестов анимации и 218 редактора прошли; два новых GPU-теста владельцев
отдельно прошли на Metal. Clippy всех целей animation/render/model/editor/demos/
gameplay чистый. Доказательства: `morph-ordinary-qualification.json`,
`morph-runtime-proof.json`, `morph-ordinary-owner-proof.json`.

Открыты редакторские morph-контролы/переопределения, явное сопоставление форм
для retarget, rigid GPU-привязка статического morph, LOD/culling/picking с
деформацией, нативная визуальная приёмка и FPS. Общая готовность движка и
минимум 120 FPS пока не доказаны; ENG-03/05/06 остаются частичными.


## Update: rigid morph GPU binding, 26 October

Single unskinned morph imports now use their existing authored node through the
common GPU owner. Focused transform/normal tests, dynamic source-free replay,
no-clip/no-playback-component default preservation and physical GPU/CPU image
parity are reproduced by `production-readiness-evidence-2026-10-26/rigid-morph-proof.py`.
The unchanged upstream multi-skin/morph owner regression also passed. This is a
bounded ENG-03/05/06 integration gate, not completion of those stages. Authoring,
morph-aware LOD, cross-rig mapping, native workflow and FPS gates remain open.


## Update: persistent morph overrides and existing inspector history

The ordinary animation component now serializes named absolute morph overrides.
The common frame preparation path applies them for runtime/timeline preview;
the generic component inspector edits complete maps and scalar values through
existing undo/redo. A synthetic panel-edit/save/reopen/locomotion/package regression
passed, including exact source-free replay. Editor suite: 221 passed, 20 ignored;
strict editor/model all-targets Clippy passed. Dedicated sliders/target discovery,
native authoring review, reload identity changes and remaining ENG gates stay open.


## Update: morph source identity and authoring defaults/resume

Named overrides now resolve original morph nodes independently of palette order.
The existing inspector can discover/import named owners' defaults and resume the
original animation, with stale-action rejection and undo. Ten focused model tests
and 221 editor tests passed (20 ignored); strict model/editor Clippy passed.
Native UI review, sliders/target labels, unnamed-node support and reload identity
UX remain open; see morph-discovery-qualification.json. ENG stages remain partial.


## Update: ordinary authoring viewport overrides

Saved morph overrides now produce bind-pose deformation requests in the ordinary
viewport without requiring Play or a timeline seek. The shared owner caches frames
by object/model/settings, preserves admitted state on invalid edits and releases
inactive entries. The new CPU regression and full editor suite passed: 222 passed,
20 ignored; strict editor Clippy passed. Native image/GPU/FPS gates and deformation-
aware picking/bounds remain unqualified. See morph-viewport-qualification.json.


## Update: deformed owner triangle picking

Picking now uses the accepted rendered model/frame pair, or the current runtime/
preview/authored override frame before GPU presentation. All complete primitives
participate in the existing triangle/depth test. Synthetic App tests prove hits
outside imported geometry, absence of old-position hits, both projections,
mirrored nonuniform scale, timeline/runtime frames and inactive filtering.
Editor: 223 passed, 20 ignored; strict Clippy passed. Bounds/culling, morph LOD,
native visual checks and performance remain open; see morph-picking-qualification.json.


## Update: focus on displayed deformation bounds

F now centers on current displayed geometry bounds using the same accepted-frame
provider as picking, with f64 world-bound accumulation and unchanged distance.
The App regression verifies displaced morph geometry and mirrored nonuniform scale
in both projections. Editor: 223 passed, 20 ignored; strict Clippy passed.
No static-AABB reject was found in the animated draw branch; conservative GPU
culling remains unimplemented/unqualified, rather than marked complete by CPU
bounds. Native focus visuals and zoom-to-fit remain open. See morph-focus-qualification.json.


## Update: F camera distance fit

F now fits displayed model bounds, replacing the previous center-only behavior.
Camera orientation/projection are preserved, active rendered view aspect is used,
and every bound corner must project inside before publication. Both projections,
multiple aspects, rotated cameras, point bounds, failure rollback and deformed App
vertices passed. Editor: 225 passed, 20 ignored; strict Clippy passed. Native UI,
panel safe areas, extreme precision, GPU culling and FPS gates remain open; see
focus-fit-qualification.json.


## Update: focus inside panel-safe viewport

Focus fitting now targets the visible subrectangle between side panels in the
active split view, with shared panel dimensions and physical/logical DPI handling.
Off-center projection/corner tests and odd-split 1x/2x layout tests passed; full
editor: 227 passed, 20 ignored; strict Clippy passed. Native/fractional-DPI execution,
extreme coordinates and remaining rendering/performance gates stay open. See
focus-safe-qualification.json.


## Update: physical GPU panel image and overlap fix

A stronger control-region check reproduced animation timeline/morph/lower-action
overlap. Lower actions and keyboard hints were moved; clipped labels shortened.
The actual SceneRenderer panel image on Apple M4 Max/Metal passed glyph/hit checks
and visual inspection. Final PNG and failed/passing logs are retained under
production-readiness-evidence-2026-10-26/animation-panel-*. This is one offscreen
panel gate; native input, broader layouts, engine visuals and FPS remain open.


## Update: current original-asset integration requalified

Reran unchanged RobotExpressive through the current ordinary project/controller,
build, source-free package and physical GPU owner. Project/package states match
after 240 ticks; all 19 GPU primitives/21,642 vertices pass the existing reference
limit across three frames. Weight-only pixel parity and failure retention also
passed. New packages are content-addressed and old packages/reports retained.
See current-original-regression-qualification.json. Native workflow/images,
realistic motion, platform delivery and scene FPS remain unqualified.


## Update: exact controller checkpoints and bounded JSON codec

Added versioned controller snapshots with durable ObjectId rebinding and exact
float-bit serialization, plus candidate-only restore and byte/body-bounded JSON
admission. A new scene restored from JSON follows 180 subsequent falling/landing
steps exactly. Corruption, duplicate/missing identities, versions, byte/body
limits and unknown fields are rejected. Full gameplay baseline: 183 passed;
editor: 227 passed, 20 ignored; final codec-focused: 3 passed; strict Clippy passed.
The full save-session envelope, disk slots and other runtime participants are
still open; see character-checkpoint-qualification.json and the ENG-07 inventory.


## Update: exact clock participant for saves

Added clock bit snapshots and SceneSimulation restore with fixed-step/catch-up
configuration guards. 720 direct continuation frames and 120 JSON/new-scene
schedule frames match exactly. The time suite also exposed/fixed short-window
p95/p99 nearest-rank telemetry. Time: 8 passed; scene: 75; editor: 227 with 20
ignored; strict four-package Clippy passed. Whole-session scene/pose history,
behaviors, input/audio/liquid/animation and disk slots are still required. See
clock-checkpoint-qualification.json; this is an ENG-07 participant, not full save/load.


## Update: interpolation-history save participant

Added durable-ID/bit-exact SceneSimulation presentation snapshots, including clock,
alpha and both pose histories. Candidate-only restore validates composed worlds
before commit. JSON/new-scene first-frame interpolation and 120 subsequent frames
match exactly; dead-parent/zero-tick and corruption/overflow rollback regressions
passed. Scene: 78 passed; editor: 227 with 20 ignored; strict Clippy passed. Whole
session participants and disk slots remain open; see presentation-checkpoint-qualification.json.


## Update: input save participant and runtime consumer

Added exact InputMap snapshots including bindings, controls, focus and pending
quick-tap/logical edges. Candidate restore checks capacities and derived state
before commit. HeadlessGame exposes the participant API; quick jump and 120
subsequent scene/input steps match. Input: 8 passed; editor: 228 with 20 ignored;
strict Clippy passed. Outer serialization, adapter identity, UI capture/focus and
whole-session atomic load remain open. See input-checkpoint-qualification.json.


## Update: behavior state prerequisite for whole-session saves

Added explicit opt-in behavior codecs, opaque payloads, durable owner/reference
rebinding and ordered lifecycle flags. Registered factories prepare a fresh runner
without awake/start replay; nonempty live targets and unsupported states reject.
120 updates, activation transitions, raw/custom-reference codecs and failure gates
passed. Scene: 82; editor: 228 with 20 ignored; strict Clippy passed. GameHost/full
save envelope, commands and remaining participants/slots stay open. See
behavior-checkpoint-qualification.json.


## Update: behavior codecs reach normal GameHost/package consumers

GameHost now retains its shared behavior codec registry across project/editor,
headless, build and packaged project launch. A source-free package test restores
JSON behavior state with installed host factories and matches 120 further scene/
behavior steps; missing codecs and nonempty live targets reject atomically.
The public project_game example adds an opt-in codec and headless --behavior-check.
Scene: 82 passed; editor: 229 with 20 ignored; strict Clippy passed. Whole save
file/session transaction and remaining participants stay open; see
host-checkpoint-qualification.json.


## Update: pending command participant and exact component JSON

Added durable pending SceneCommands snapshots, including registered typed component
operations and the spawn journal. Original apply order, stale-handle/invalid-pose
errors and slot-reuse behavior are retained; failed restore preserves the queue.
Explicit Insert/Remove tags preserve null components. A real component f64
roundtrip regression exposed/fixed parser low-bit loss via float_roundtrip.
Scene: 86 passed; editor: 229 with 20 ignored; strict Clippy passed. The common
save envelope, remaining participants and disk slots are still open; see
command-checkpoint-qualification.json.


## Update: unified scene/SceneSimulation save file

Added one versioned scene/simulation file with privately staged load and reuse of
the existing atomic writer. It includes exact allocation/free/sibling/retired-ID
metadata, clock/interpolation, behaviors and pending commands. A real disk roundtrip
and 120 following frames match world bits, node order, state and command results;
old handles stay invalid and rejected candidates retain the old file/pair.
Scene: 89 passed; editor: 229 with 20 ignored; strict Clippy passed. This closes
one owner composition gate. Full App/game participants, project identity, native
slots and performance remain open; see scene-save-qualification.json.

## Update: staged gameplay save participants

The gameplay layer now stages a single owned scene/simulation/character/input/tick
unit from one bounded JSON envelope. Mid-fall, landing and subsequent jump match
for 240 frames; the runnable `gameplay_save` example also compares the complete
supported checkpoint for 240 frames. Compatibility keys are caller supplied.
App integration and the other active runtime owners remain required for whole-game
save/load. See the readiness register's staged gameplay checkpoint section.

## Update: exact Animator save participant

The core Animator now has a bounded JSON checkpoint with revision-resolved clips,
exact clocks, ordinary and interrupted fades, frozen poses and animated properties.
Fresh-asset continuation tests pass for 240 frames; animation 339 tests, editor
229 passed/20 ignored and strict Clippy are recorded in the readiness register.
Higher animation owner state and complete App/game save publication remain open.

## Update: model playback events and contact history

ModelPlayback checkpoints now retain pending events and their tracks/windows,
playing-source contact intervals and frozen-source pose/tick fractions alongside
Animator. Fresh-model restore and 240 continuation frames match, with exactly-once
event draining. Animation 339/editor 231 tests passed (20 editor ignored); strict
Clippy passed. The complete AnimationRuntime/App save integration remains open.

## Update: durable foot contact and runtime history

Support/contact checkpoints preserve ObjectId bindings and hysteresis. FootRuntime
history now preserves pending pivots, geometry, contact weights and shared frozen
pose keys. Imported models and native rigs retain their original name-resolution
paths. Four new tests and 180 moving-platform/history steps passed; animation
339/editor 233 passed (20 ignored), focused contact/support tests and strict Clippy
passed. The shared snapshot archive, full AnimationRuntime and App save remain open.

## Update: shared snapshot identity in animation saves

A bounded pose archive now retains allocation identity across Animator,
ModelPlayback and FootRuntime. A combined JSON/fresh-model restore verifies one
shared Arc and 240 subsequent frames with repeated fade interruptions and event
delivery. Equal-valued independent snapshots remain distinct. Animation 340/editor
234 tests passed (20 ignored), with strict Clippy. The remaining AnimationRuntime
owner state and App publication are still required before full game-save readiness.

## Update: accepted animation presentation frame

The accepted AnimatorFrame can now be checkpointed after IK/retarget owner
corrections, preserving pose/property/palette/root metadata bits. Preparation
rejects even a one-bit inconsistent palette. The physical foot-placement test
verifies a saved solved frame against a fresh model. Animation 342/editor 234
tests passed (20 ignored), strict Clippy passed. Root reference interval proofs,
remaining AnimationRuntime owner state and full App save integration remain open.

## Update: exact root-reference interval persistence

Root reference scale/body/authored enclosure endpoints now survive JSON loading
without widening or normalization. Exact integer unit-sphere admission detects
subnormal norm excess lost by f64 arithmetic. Tests verify 180 subsequent transports
and real accepted-physics retarget/reflected-parent references. Animation 344/editor
235 tests passed (20 ignored), strict Clippy passed. Pending rotation certificates,
complete AnimationRuntime composition and full App save/load remain open.

## Update: replay-verified root paths

Root trajectories now support bounded construction-program checkpoints verified
against every stored field's exact bits after native replay. Cubic/STEP curves,
explicit screw clocks, retiming, coordinate changes and append are covered.
Animation 346/editor 235 tests passed (20 ignored), strict Clippy passed. Automatic
playback recipe capture, AnimationRuntime composition and full App save remain open.

## Update: automatic root trajectory source receipts

ModelPlayback now retains exact single-clip root trajectory input receipts only
after successful frame publication and saves them with replay-verified paths.
Fresh-model loading and 120 continuation ticks match; late rejection retains the
previous receipt. Animation 346/editor 237 tests passed (20 ignored), strict Clippy
passed. Certified fades, full AnimationRuntime composition and App save remain open.

## Update: certified original-source fade persistence

Original-source fade checkpoints now reconstruct the native certificate and compare
every path/error/domain bit. They can share the staged owner's animator bindings.
The physical fade request tests consume a JSON-restored motion, retaining collision
prefix/event and rollback behavior. Animation 347/editor 237 tests passed (20 ignored),
strict Clippy passed. Full AnimationRuntime composition, additional compiler adapters
and complete App/game-save publication remain open.

## ENG-07 update: animation owner checkpoint integration

The current AnimationRuntime participant is implemented and tested together with
scene, simulation, physics and input for 120 continuation frames, including solved
foot history and interrupted fade snapshot identity. ModelPlayback v2 additionally
preserves event-source revisions across rebind. See the current readiness register
and animation-owner-qualification.json for retained logs and explicit limits.

Next save milestone remains a single App/PlaySession transaction with resource
revision archives and all active UI/audio/camera/angular/liquid owners. Separate
participant tests do not close whole-game saves, native workflow or ENG-07.


## 27 October: restored watcher policy admission

ENG-03 now rejects a restored model-resource watcher whose saved source-count
policy cannot cover the retained dependency graph and mandatory manifest, or whose
per-file byte policy cannot cover accepted watched model inputs. Admission runs
before provider access and producer creation. Exact-boundary policies are retained;
explicit offline owners remain supported. Existing worker restoration already
preserved saved source-count limits; the new regression confirms that behavior.

Live model completion also checks actual watcher byte capacity before publication.
A real-file regression grows a valid model beyond policy, retains the old accepted
model, then automatically recovers when a changed valid source fits again. Native
GPU refusal coverage retains original CPU/GPU publication and cache ownership.

Validation: assets 67 passed; editor 349 passed, 32 ignored; one explicit M4 Max /
Metal GPU transaction test passed; strict all-target Clippy and scoped format/diff
checks passed. Evidence: [qualification and preserved logs](production-readiness-evidence-2026-10-27/restored-watcher-policy-admission/qualification.json).
This does not qualify rendered-image parity, frame rate, complete App session
installation, or native audio devices. ENG-03 through ENG-13 remain open.


## 27 October: explicit model import recovery in native panels

Failed selected model imports now expose a Retry import panel action. The existing
catalog invalidation and background import queue perform the retry; accepted model
resources and scene documents remain intact. The action is disabled during Play,
checkpoint/restore fences, relink and retarget edits, and after its first queueing.
Automatic reload after source changes remains supported.

The regression injects a transient failure through native catalog completion, then
reimports the unchanged real OBJ through the normal worker and compares original
input observations and authoring document. Native M4 Max/Metal panel rendering
checks retry glyphs and presented-frame hit acknowledgement. Editor suite: 350
passed, 32 ignored; explicit GPU test: 1 passed; strict all-target Clippy and scoped
format/diff pass. The initial help-text regression and corrected run are retained.
Evidence: [qualification and logs](production-readiness-evidence-2026-10-27/explicit-model-import-retry/qualification.json).
No native window interaction, physical frame-rate or complete engine qualification
follows; ENG-03 through ENG-13 remain open.


## 27 October: stale Retry panel action admission

A reproduced panel dispatch defect cleared a live relink path draft before Retry
eligibility was checked. A stale retry action could then queue an import and discard
user input. Dispatch now checks admission before finishing drags, resetting trace
state or clearing authoring drafts; queue-time admission remains unchanged.

The regression first fails on the previous behavior, then confirms path text,
source change token, accepted model Arc, authoring document and manifest bytes
remain intact on rejection. The original path entry subsequently completes normal
background relink and publication. Editor suite: 351 passed, 32 ignored; strict
all-target Clippy and scoped format/diff pass. Evidence: [qualification and logs](production-readiness-evidence-2026-10-27/retry-action-preflight/qualification.json).
No new GPU/window/frame-rate or native-device-audio qualification is claimed.
ENG-03 through ENG-13 remain open.


## 27 October: declared deferred model dependencies in project packages

A reproduced build defect copied every declared manifest primary file but only
collected transitive model inputs for models referenced by the startup scene. An
uninstanced glTF consequently exported successfully even with its external BIN
missing. Project build now uses the same InputRecipe decoder and original import
quotas/revalidation for declared models outside the required scene set, drops each
decoded model after validation and feeds captured source bytes into the existing
package source-consistency/no-clobber publication path. Existing export cleanup
still closes original worker owners on success and failure.

The regression rejects missing and truncated deferred BIN inputs without output,
then builds after repair. After deleting the entire original project it imports
the declared animated model through the normal manifest worker from a materialized
package and completes source-free GameCheck. Editor suite: 352 passed, 32 ignored;
strict all-target Clippy and scoped format/diff pass. Evidence: [qualification and logs](production-readiness-evidence-2026-10-27/declared-model-package-closure/qualification.json).
The fixture proves deferred external BIN closure; native renderer/FPS, arbitrary
uninstanced prefab dependency graphs and new importer types are not qualified.
ENG-03 through ENG-13 remain open.


## 27 October: deferred declared prefab validation during build

Declared prefab files absent from the startup scene were copied without native
parse/expansion, allowing malformed and cyclic future prefab assets into packages.
Build now loads every declared prefab choice through the original AuthoringProject
loader, registry, bounded expander and source revalidation. Logical prefab and
existing scene paths share the original source-reading helper. Captured model and
prefab inputs share consistency admission and existing final package byte checks.

The regression reproduces acceptance before the fix, rejects a broken leaf and a
cyclic deferred prefab without publishing output, then exports corrected nested
content. After deletion of the source project, the materialized package retains
an empty startup scene and successfully loads the declared nested prefab with
manifest/leaf observations and original model component. Source-free GameCheck
also passes. Editor: 353 passed, 32 ignored; strict all-target Clippy and scoped
format/diff pass. Evidence: [qualification and logs](production-readiness-evidence-2026-10-27/deferred-prefab-package-validation/qualification.json).
This validates native prefab composition/source closure, not arbitrary future
gameplay composition, runtime spawning API, native device audio or frame rate.
ENG-03 through ENG-13 remain open.


## 27 October: bounded retained deferred source accumulation

The declared model/prefab dependency collector now admits each complete input group
against the existing 4096-source / 64 MiB package payload ceiling before retaining
its observations. Shared input identities count once and preserve original shared
payload allocations. Count/byte excess, conflicting revisions and failed reads
reject without partially changing the previously captured set. Original model and
prefab import quotas and final package revalidation remain unchanged.

Two regressions check exact-boundary admission, shared-source deduplication/Arc
retention and atomic refusal for count, byte, conflict and read-failure cases.
Editor suite: 355 passed, 32 ignored; strict all-target Clippy and scoped format
pass. Evidence: [qualification and logs](production-readiness-evidence-2026-10-27/package-retained-source-budget/qualification.json).
This bounds retained deferred source payloads, not total exporter heap/RSS, decoded
model output, physical GPU memory or frame rate. ENG-03 through ENG-13 remain open.


## 27 October: project manifest admission matches editor/build policy

Project validation previously accepted 100000 bindings / 8 MiB manifests while
editor bootstrap and build only admitted 128 bindings / 64 KiB. A reproduced
129-binding project therefore validated but could not open. The existing actual
editor/build policy now has one native manifest decoder shared by project
open/editor-open/save/create, model inventory/recipe and project package build.
This corrects validation claims; it does not increase supported asset capacity.

The regression accepts exactly 128 bindings and 65536 bytes in project/editor
opening, rejects count+1 and byte+1, preserves descriptor bytes on failed saves,
and reopens after manifest repair. Editor: 356 passed, 32 ignored; final strict
all-target Clippy and scoped format/diff pass. Initial Clippy unused-import failure
and cleanup are retained. Evidence: [qualification and logs](production-readiness-evidence-2026-10-27/project-manifest-policy/qualification.json).
Scalable configurable resource policies, full native lifecycle, device audio and
frame-rate qualification remain open, along with ENG-03 through ENG-13.


## 27 October: sampler-only reload through source-free project build

Existing sampler decode behavior now has an ordinary project lifecycle regression.
A real watcher observes a GLB sampler-only change from linear/mipmapped/repeat to
nearest/clamp/mirror without mipmaps. Accepted CPU publication updates while
position/UV/color f32 words, index arrays, texture pixels and authored scene remain
unchanged. The project is saved and exported, its entire source tree removed, and
the materialized package restores the new sampler plus geometry/pixels/document.
Source-free GameCheck completes 120 ticks. No production sampler change was needed.

Editor suite: 357 passed, 32 ignored; strict all-target Clippy and scoped format/diff
pass. Initial test dependency compilation failure is retained; direct f32 word
comparison fixed the test without adding dependencies. Evidence: [qualification and logs](production-readiness-evidence-2026-10-27/sampler-reload-source-free-package/qualification.json).
GPU rendered sampler parity and physical frame-rate qualification remain separate.
ENG-03 through ENG-13 remain open.


## 27 October: native GPU sampler views and exact rendered reuse

An explicit M4 Max/Metal regression now exercises the original model_texture cache
and SceneRenderer bindings. Nearest-clamp, linear-clamp, nearest-repeat and nearest
with mip visibility retain distinct material-view Arcs while sharing one uploaded
image texture and original mip-chain residency bytes. Repeated exact policies
return the original view allocations. Native 64x64 offscreen readback distinguishes
filtering and wrap modes; nearest has four colors, linear more than 100, and the
original view reproduces its prior pixels exactly after the other draws. Native
validation error scope is clean. No production cache behavior change was needed.

Explicit GPU test: 1 passed; strict all-target Clippy and scoped format/diff pass.
The existing full CPU suite was not repeated for this test-only module addition.
Evidence: [qualification and logs](production-readiness-evidence-2026-10-27/gpu-material-sampler-views/qualification.json).
This is separate from the CPU watcher/reload/source-free-package regression and
does not establish one complete installed App GPU reload, native presentation
pacing, physical memory or FPS. ENG-03 through ENG-13 remain open.


## 27 October: editor resource device access separated from presentation

Inspection found that App GPU resource paths obtained device/queue through its
mandatory SceneSurface, preventing window-free construction of the original full
Graphics owner. Graphics now explicitly retains clones of the same initialized
wgpu Device/Queue handles. Model reload/residency, UI preparation/restore and
viewport resource uploads use these handles directly. The surface-only model
upload wrapper is removed; normal residency uses the same native upload_on as
offscreen GPU qualification. No second device or alternate publication path exists.

Editor CPU suite: 357 passed, 33 ignored. Explicit M4 Max/Metal native model reload
transaction and sampler rendering tests: 2 passed. Strict all-target Clippy and
scoped format/diff pass. Evidence: [qualification and logs](production-readiness-evidence-2026-10-27/editor-gpu-device-access/qualification.json).
This is preparation for optional presentation ownership. Graphics construction
still requires SceneSurface; full-App offscreen sampler reload, native presentation
pacing, physical memory/FPS and full ENG-03 through ENG-13 remain open.


## 27 October: native App model reload with real GPU and no surface

Graphics now constructs its original renderer/material shader, animation/gizmo/UI
resources and residency owners through one shared constructor. SceneSurface is
optional; normal window initialization supplies it and explicit offscreen native
App qualification does not. Presentation calls require a surface while resource
import and GPU publication are independent. No alternate model runtime is added.

The M4 Max/Metal regression uses original App.tick, real source watcher/import
workers and deferred GPU admission. During preparation, old CPU/GPU publications
remain; commit installs the same new imported Arc in both owners. A sampler-only
change reuses image storage/residency and changes diagnostic-quad pixels drawn
through the App renderer. Corrupt source retains publication and pixels; source
repair automatically reloads and reproduces accepted pixels exactly. Authoring
document remains unchanged, workers stop, and GPU validation scope is clean.

Editor suite: 357 passed, 34 ignored. Explicit native App GPU test: 1 passed;
strict all-target Clippy and scoped format/diff pass. Evidence: [qualification and logs](production-readiness-evidence-2026-10-27/offscreen-native-app-reload/qualification.json).
This closes the direct-model offscreen App reload integration gap. Complete
viewport-scene rendering, OS presentation/input, native device audio, full editor
session installation, combined package/GPU lifecycle and physical frame-rate
qualification remain open, along with full ENG-03 through ENG-13.


## 27 October: actual App GPU budget refusal and unchanged-source Retry

The original offscreen App hardware regression now also changes a valid watched
GLB while the native geometry budget is zero. Normal GPU staging refuses the
candidate and settles the catalog failure without pending work. Prior CPU/GPU
publication Arcs, geometry allocation, material binding, cache state/identity and
pixels remain. Native model checkpoint completes and resumes after the refusal.

The test restores the original budget and dispatches the existing RetryModel panel
action. The unchanged source file is reimported through the original worker/GPU
publication path; CPU/GPU share the new accepted Arc and expected original pixels
match exactly. Authoring document is unchanged and GPU error scope is clean.
Explicit M4 Max/Metal App GPU test: 1 passed; strict all-target Clippy and scoped
format pass. Production code already supported the behavior; this extends combined
qualification rather than adding fallback or lowering quality. Evidence: [qualification and logs](production-readiness-evidence-2026-10-27/offscreen-app-gpu-budget-retry/qualification.json).
No budget editor UI, complete viewport image, native presentation/audio-device or
physical frame-rate qualification follows. ENG-03 through ENG-13 remain open.


## 27 October: joined project reload, GPU refusal/Retry and source-free package

The native M4 Max/Metal App regression now starts from a real project/manifest,
creates a model instance and saves a transform edit. It retains the combined
watcher/sampler reload, corrupt-source recovery, GPU-budget refusal, checkpoint
and unchanged-source Retry scenarios. After repair the ordinary project exporter
builds a package and the entire original project directory is deleted.

A fresh materialized-package App imports through original manifest/worker/GPU
pipelines using independent catalog publication, GPU cache identity and image
allocation on the same device. Captured manifest/GLB observations match accepted
source inputs; native model-material readback matches original pixels exactly and
the saved scene edit remains. Source-free GameCheck completes 120 ticks and the
GPU validation scope is clean. Explicit joined project GPU test: 1 passed; strict
all-target Clippy and scoped format pass. Evidence: [qualification and logs](production-readiness-evidence-2026-10-27/project-gpu-source-free-package/qualification.json).
This joins previous separate package/GPU checks for this model-material fixture.
It does not qualify a separate process/device, complete viewport rendering, native
presentation/input/audio devices, physical memory or FPS. ENG-03 through ENG-13
remain open.


## 27 October: original scene viewport frame to texture and source-free parity

App.draw now delegates to draw_frame with optional native SceneViewTargets. This
shares the original extraction, camera, animation/LOD, materials, transforms and
draw-list preparation; offscreen output encodes those same views/overlays and
returns after submission without native presentation/UI ACK or frame increment.
Sample policy must match; nonempty fog/optical-fluid offscreen paths reject until
their native targets and adapter ownership are connected, rather than omitting
effects. Normal surface rendering retains its existing branch and ACK behavior.

The native project regression expands the imported hierarchy to four objects,
saves a transform edit and renders actual scene geometry in standalone App mode
into 128x128 targets. After export and deletion of the complete source project,
a fresh package App renders identical RGBA bytes through the same frame path.
The frame is nonempty, native presentation counters/flags remain unchanged,
source-free GameCheck completes 120 ticks with four objects and the GPU scope is
clean. Editor: 357 passed, 34 ignored; explicit M4 Max/Metal project GPU test:
1 passed; strict all-target Clippy and scoped format/diff pass. Evidence:
[qualification and logs](production-readiness-evidence-2026-10-27/native-scene-frame-source-free/qualification.json).

This qualifies this static hierarchical viewport fixture, not all rendering
features, photorealistic street quality, OS input/presentation, device audio,
physical memory or frame rate. ENG-03 through ENG-13 remain open.


## 27 October: original fog transport in offscreen App scene frames

Offscreen App frames now use the original FogDraw target preparation, transport
encoding and submitted-resource lifetime, deriving size/format from native output
targets. Surface fog shares the same preparation; failed encoding discards pending
fog resources and no offscreen frame acknowledges OS/UI presentation. Native fog
parameters, integration samples, source validation and budgets are retained.

The project fixture authors a non-legacy perspective camera through the native
editor command and a uniform fog volume with 32 samples. Disabling fog changes
pixels and reenabling it restores the exact image. Build plus complete source
deletion and a fresh package App reproduce the same fogged scene RGBA bytes.
Source-free GameCheck completes 120 ticks with five objects. The initial legacy
camera correctly rejected and is retained as evidence. Editor: 357 passed,
34 ignored; explicit M4 Max/Metal joined App GPU test: 1 passed; strict all-target
Clippy and scoped format/diff pass. Evidence: [qualification and logs](production-readiness-evidence-2026-10-27/offscreen-fog-native-app/qualification.json).

This supersedes the blanket offscreen-fog rejection for this supported fixture.
Overlapping fog/MSAA/merged fog-liquid restrictions, optical-fluid adapter/output
ownership, dynamic/shadow qualification, native OS/audio devices, physical memory
and FPS remain open. ENG-03 through ENG-13 remain incomplete.


## 27 October: opaque scene shadow contribution in packaged fog frames

The joined native App GPU fixture now authors directional light/shadows and opaque
caster markers on its imported hierarchy. Native defaults remain 1024x1024 shadow
map, PCF3x3 filtering, original bias/extent and 32 fog samples. Shadow-disabled and
caster-disabled controls independently change pixels; restoring either reproduces
the exact accepted frame. Fog on/off controls also remain. No production shadow
algorithm change was required.

Light/shadow/caster/camera/fog data survive project build and deletion of every
source file; a fresh package App renders identical RGBA bytes and source-free
GameCheck completes 120 ticks with five objects. Explicit M4 Max/Metal joined GPU
test: 1 passed; strict all-target Clippy and scoped format pass. Evidence:
[qualification and logs](production-readiness-evidence-2026-10-27/native-fog-shadow-package/qualification.json).
This qualifies static opaque shadow influence on native fog transport, not all
surface receivers, animated/deformed casters, cascades, transparency, native OS
input/presentation/audio, physical memory or FPS. ENG-03 through ENG-13 remain open.


## 27 October: offscreen snapshot clock and current moving shadow geometry

A hardware regression reproduced six unintended gameplay ticks from one offscreen
frame because draw_frame consumed wall time. Offscreen output now renders current
simulation state without modifying its timestamp/tick; explicit advance_game owns
headless stepping. Ordinary surface draw retains its previous real-time update.
Resource polling/extraction still use original owners and are not frozen.

The joined project fixture saves native AngularMotion and advances exactly four
fixed steps. World transform and rendered fog/shadow scene change; rendering adds
no extra ticks. Source-free package App reproduces both initial and moved frames.
A third fresh App first allocates FogDraw only after stepping to the moved pose
and matches that frame exactly, ruling out reuse of the earlier shadow map. Native
fog/shadow/caster controls and prior reload/refusal/Retry/package assertions remain.
Editor: 357 passed, 34 ignored; explicit M4 Max/Metal joined GPU test: 1 passed;
strict all-target Clippy and scoped format/diff pass. Evidence:
[qualification and logs](production-readiness-evidence-2026-10-27/offscreen-clock-dynamic-shadow/qualification.json).
This covers rigid object motion, not full skeletal/morph cycles, OS presentation
or audio devices, physical memory or FPS. ENG-03 through ENG-13 remain open.


## 27 October: original optical fluid renderer in native offscreen App frames

Graphics now retains its original adapter alongside device/queue, so optical format
and budget preflight no longer depends on a window. SceneRenderer exposes its
actual attachment format. Existing scene/fluid/UI composition is shared through
encode_view_frame_with_fluids by surface multi-view and offscreen rendering; native
ScreenSpaceFluidRenderer, input conversion, capacity 16384 and budget policy are
retained. No simplified optical renderer is introduced.

A native authored LiquidSource emits after one explicit fixed step. Its optical
frame differs from the empty frame, and rendering preserves particle f64 words,
game tick and native presentation counter. After project build and full source
deletion, a fresh package App reproduces empty/optical RGBA and particle state bits
exactly. Source-free GameCheck completes 120 ticks with two objects; GPU scope is
clean. The joined moving fog/shadow project regression also still passes.
Editor: 357 passed, 35 ignored; renderer: 374 passed, 86 ignored; two explicit
M4 Max/Metal GPU tests passed; strict renderer-lib/editor-all-target Clippy and
scoped format/diff passed. Evidence: [qualification and logs](production-readiness-evidence-2026-10-27/offscreen-optical-fluid-native-app/qualification.json).

This supersedes blanket offscreen optical-fluid rejection for the supported
single-sample fixture. Merged fog/fluid optical depth, full dynamics/performance
qualification, native OS/input/audio devices, physical memory and FPS remain open.
ENG-03 through ENG-13 remain incomplete.


## 27 October: liquid material provenance checked before package publication

A public GameHost setup reproducer changed liquid material bytes after native
runtime preparation and the old exporter still published the unvalidated payload.
Native preparation now optionally returns original validated ImportInputs alongside
unchanged runtime/optics; existing runtime consumers retain their tuple contract.
Build captures those original bytes before host startup and uses existing final
package consistency checks to reject changed content before atomic publication.

Both corrupt and valid-but-different material replacements now reject specifically
as changed liquid.json input without creating output. Restoring original content
allows normal export and source-free GameCheck after deleting the entire project.
Editor: 358 passed, 35 ignored; explicit native optical-liquid M4 Max/Metal App
GPU/source-free test: 1 passed; strict all-target Clippy and scoped format/diff
pass. Evidence: [qualification and logs](production-readiness-evidence-2026-10-27/liquid-material-build-observation/qualification.json).
This is build provenance, not a filesystem lock, new live material watcher or new
session/checkpoint participant. Native coefficients, solver, optical precision
and import quotas remain unchanged. ENG-03 through ENG-13 remain open.


## 27 October: deferred prefab model bindings after nested overrides

A regression reproduced successful packaging of an unused declared prefab whose
model binding was absent. Build now checks original expanded prefab objects after
nested overrides against native model inventory. Empty/missing IDs and declared
non-model assets reject without output; authoring unresolved-reference repair
policy is unchanged and no separate catalog/gameplay runtime is introduced.

The nested override case is first confirmed structurally valid by native expansion,
then rejected for its unresolved model. After repair, export succeeds; deletion
of the source project still permits native prefab placement from the materialized
package, standalone model import and one fixed AngularMotion step changing world
transform. The original empty-startup package separately passes 120 GameCheck
ticks. Editor: 359 passed, 35 ignored; final focused regression, strict all-target
Clippy and scoped format/diff pass. Evidence: [qualification and logs](production-readiness-evidence-2026-10-27/deferred-prefab-model-bindings/qualification.json).
Retarget sources, clip/part/profile semantics, other deferred asset links, native
runtime prefab spawn API and complete ENG-03 through ENG-13 remain open. No new
GPU/native-device/FPS qualification follows.


## 27 October: native retarget compilation for deferred prefab build

Unused declared prefab retarget profiles previously exported with unresolved source
models. Build now checks original expanded/overridden profile values using native
ModelRetarget.validate and compile_models against original imported model assets.
Existing InputRecipe/quotas/revalidation and bounded package input observations
are reused; identical source/target identity shares the imported owner. Generic
decode scaffolding preserves dropping unused payloads before source revalidation.

The reproduced missing-source export now rejects. Eight negative cases cover
missing/non-model/static source, missing source/target bones, invalid rotation
calibration, static target and missing source hidden by nested override. Correct
hip mapping builds and, after deletion of the source project, native prefab
placement from the materialized package starts standalone with animated rig and
one game tick. Editor: 360 passed, 35 ignored; strict all-target Clippy and scoped
format/diff pass. Evidence: [qualification and logs](production-readiness-evidence-2026-10-27/deferred-prefab-retarget-admission/qualification.json).
Clip/part/locomotion and morph-shape semantics, persistent import caching, full
GPU deformation/animation-cycle coverage, native devices and FPS remain open.
ENG-03 through ENG-13 remain incomplete.


## 27 October: deferred prefab imported-part index admission

A regression reproduced export of an unused prefab selecting OBJ ModelPart zero
when that imported model has no node hierarchy. Build now checks native expanded
model-part descriptors after nested overrides against actual node count from the
original InputRecipe decoder/revalidation. Native group nodes and u32::MAX
containers remain valid; source observations join existing final package checks.

Negative cases cover empty OBJ node hierarchy, glTF count-boundary/large indices
and invalid nested override. Positive group/container cases build. A valid textured
part also builds; after full source deletion native prefab placement and standalone
startup load a drawable mesh_for(part) and complete one game tick. Editor:
361 passed, 35 ignored; strict all-target Clippy and scoped format/diff pass.
Evidence: [qualification and logs](production-readiness-evidence-2026-10-27/deferred-model-part-admission/qualification.json).
Same-count hierarchy semantic changes still need stable part identity; clip and
locomotion admission, full GPU/deformation/native devices and FPS remain open.
ENG-03 through ENG-13 remain incomplete.


## 27 октября: приёмка анимаций отложенных префабов при сборке

Build проверяет настройки анимации на expanded objects через существующий
ModelPlayback: неизвестные имена и индексы клипов, неверные скорость и кости
отклоняются до публикации пакета; учитываются nested overrides. Для retarget
клип разрешается на source-риге. Регрессия использует target без собственных
клипов, удаляет исходный проект и проверяет кадр runtime из восстановленного
пакета. Именованный клип сохраняет приоритет над устаревшим индексом.

Свежие проверки: 362 editor lib tests passed, 35 ignored; strict Clippy всех
targets прошёл. [Логи, хеши и границы](production-readiness-evidence-2026-10-27/deferred-prefab-animation-admission/qualification.json).
Это приёмка build/runtime, без GPU/full-cycle/FPS квалификации; ENG-03, ENG-06
и общая программа ENG-00–13 остаются открыты.


## 27 октября: morph-настройки отложенных префабов

Сборка ранее принимала отсутствующий morph-узел. Теперь настройки expanded
префаба проверяются на целевой геометрии штатным apply_morph_weights и
validate_morph_frame; retarget source используется отдельно для клипов.
Регрессии отклоняют отсутствующий узел и неверное число весов. Корректные
веса [1.25, -0.75] без clamping проверены в runtime-кадре восстановленного
пакета после удаления исходников. Отдельный source/target retarget+morph
положительный сценарий этой фикстурой не квалифицирован.

Проверки: 6 focused passed; 363 editor tests passed, 35 ignored; strict
all-target Clippy прошёл. [Доказательства и ограничения](production-readiness-evidence-2026-10-27/deferred-prefab-morph-admission/qualification.json).
GPU, полный цикл и FPS этим шагом не проверяются. ENG-03/06 и общая цель
движка остаются открыты.


## 27 October: deferred prefab locomotion admission

Build now uses native ModelLocomotion.validate_model on the playback source
to check all four clip names and transition settings. A missing ModelAnimation
component is rejected before publication. The baseline regression demonstrated
that a missing falling clip was previously packaged successfully. Character-body
locomotion with retargeting and a clipless target was restored and stepped from
a package after deletion of the original project. The positive fixture uses the
same source clip for all states and does not qualify a full locomotion cycle.

Fresh checks: 363 editor tests passed, 35 ignored; strict all-target Clippy passed.
[Logs and hashes](production-readiness-evidence-2026-10-27/deferred-prefab-locomotion-admission/qualification.json).
Deferred prefab character ancestry and ModelPart admission remain open, along
with native GPU/FPS qualification, ENG-03/06 and the full engine roadmap.


## 27 October: nested prefab locomotion full CPU movement cycles

A new integration regression composes the character and skeletal/morph visual
through two prefab levels. It replays four 180-tick cycles (720 fixed ticks)
in the source project and then in the package after deletion of the original
project. Each cycle covers Idle, Walking, Jump and Fall, blended transitions,
actual body displacement, ascent and landing, and returns to Idle. Every native
AnimationSnapshot and the final serialized scene match exactly between runs.
Morph weights vary and retain negative values without clamping.

Fresh full editor suite: 364 passed, 35 ignored; strict all-target Clippy passed.
[Evidence and source hashes](production-readiness-evidence-2026-10-27/nested-prefab-locomotion-cycle/qualification.json).
This is a small-fixture CPU integration proof, not a production character visual
qualification, GPU cycle, native presentation or FPS result. ENG-03/06 and the
full engine roadmap remain open.


## 27 October: native GPU locomotion cycle and CPU animation ownership

A real GPU App test exposed lost animation sources after an inactive model
released GPU residency. Simulation now starts from CPU model owners and prefers
the resident GPU publication when available, preserving displayed revision
consistency without making gameplay depend on visibility. The regression checks
actual GPU eviction, retained CPU publication, and successful subsequent ticks.

On Apple M4 Max/Metal, four nested-prefab locomotion cycles rendered through the
original scene pipeline: 720 fixed ticks and RGBA readbacks, then 720 more after
source-free package restoration. Every CPU debug-state and 128x128 RGBA frame
matched. Visible/hidden frames differ; animated frames vary; all four clips run.
Native watcher initial inventory/reloads are settled before replay. Earlier
failures from missing sources and unsettled publications are preserved.

Current checks: three native GPU App tests passed (reload, optical liquid,
locomotion); 364 editor tests passed, 36 ignored; strict all-target Clippy passed.
[Logs, hashes and limitations](production-readiness-evidence-2026-10-27/native-gpu-locomotion-cycle/qualification.json).
This small-fixture offscreen test does not qualify realistic character quality,
independent CPU pixel correctness, arbitrary concurrent reloads, native window
presentation, 1080p frame pacing or 120/160 FPS. ENG-03/05/06 and full parity stay open.


## 27 October: independent CPU deformation pixel oracle for native animation

The full GPU movement-cycle regression now temporarily exchanges the animation
geometry owner for a test-only instance of the existing CPU fallback. Its
ModelAsset.scene_meshes_frame evaluator computes skin/morph geometry independently
from the compute shader, while both paths retain the original raster/material
pipeline, camera, world transform and accepted simulation frame. Assertions prove
that GPU geometry and CPU-baked geometry are actually selected respectively.

All 1440 frames (720 project ticks plus 720 source-free package ticks) produce
exactly equal RGBA8 images on M4 Max/Metal. Two stationary-world frames differ in
pixels, demonstrating visible deformation rather than just body translation.
Offscreen oracle draws preserve simulation and presentation counters. GPU
validation scope is clean. Focused hardware test, strict all-target Clippy, scoped
rustfmt and diff checks pass. Full editor suite was not rerun for test-only edits.
[Logs, hashes and limits](production-readiness-evidence-2026-10-27/native-animation-cpu-pixel-oracle/qualification.json).
This establishes deformation pixel parity for the small 128x128 fixture only;
shared rasterizer/material correctness, all production rigs, native presentation
and 120/160 FPS are not qualified. Full ENG-05/06 and engine readiness stay open.


## 27 October: observed liquid materials in deferred prefab build admission

Build previously copied a declared but unused prefab's liquid material without
decoding it. The native material loader now separates observed material decoding
and physical-coefficient admission from scene-specific LiquidRuntime creation.
Both ordinary runtime preparation and expanded declared-prefab build admission
use this same loader. Prefab materials do not require a complete body/collider
context before instantiation. Physical coefficients reuse native Liquid admission.

Regression cases reject malformed JSON, negative density and invalid optics.
Two deterministic BehaviorSetup mutations after admission (corrupt data and valid
changed coefficients) are rejected by final package observation checks, with no
published package. A repaired package restores the nested prefab and emits one
particle after original project deletion.

Fresh checks: 365 editor tests passed, 36 ignored; three native GPU App tests
passed; strict all-target Clippy and scoped diff check passed.
[Evidence and hashes](production-readiness-evidence-2026-10-27/deferred-prefab-liquid-material/qualification.json).
This does not qualify deferred material hot reload, all audio/font dependencies,
PlaySession liquid watcher ownership, production physical accuracy or target FPS.
ENG-03 and the complete engine roadmap remain open.


## 27 October: deferred prefab audio and import-settings admission

Declared expanded prefabs now pass AudioSource descriptors through native
validation and import_audio at the ordinary offline mix rate of 48000 Hz. WAV
and settings observations join the existing bounded retained-source closure.
Conflicting settings for the same logical audio asset in one expanded prefab
are rejected, matching the runtime policy. No second decoder/catalog is added.

Regression proves the original corrupt-WAV acceptance, then verifies rejection
of malformed audio/settings, frame-limit violation, nested conflicting settings,
and both waveform corruption and valid settings mutation after admission. The
repaired source-free package restores the nested prefab and mixes nonzero PCM.

Fresh checks: 366 editor tests passed, 36 ignored; strict all-target Clippy,
scoped rustfmt and diff check passed.
[Evidence and source hashes](production-readiness-evidence-2026-10-27/deferred-prefab-audio-admission/qualification.json).
Native audio devices, other sample rates, complete voice/bus/listener admission
and all remaining dependency classes are unqualified; ENG-03/07 and the full
engine roadmap remain open.


## 27 October: deferred prefab UI text and observed font admission

Build now runs declared expanded prefab UI through the same bounded
UiTextPreparation.prepare_import path and 1280x720 build viewport as the startup
scene. This includes native font decoding, glyph/layout admission and source
observations, rather than only copying manifest files. The loaded prefab graph
is shared with liquid-material admission when both consumers are present.

The regression demonstrates corrupt-font acceptance before the fix, then rejects
corrupt fonts and deterministic font mutation after admission. A repaired package
restores the nested prefab after original project deletion, prepares Unicode
glyph runs and advances standalone by one fixed tick. No second text decoder or
font catalog was introduced.

Fresh checks: 367 editor tests passed, 36 ignored; strict all-target Clippy,
scoped rustfmt and diff checks passed.
[Evidence and source hashes](production-readiness-evidence-2026-10-27/deferred-prefab-ui-font-admission/qualification.json).
This test uses a local system Unicode font. It does not qualify all scripts, GPU
text pixels, native focus/input, interactive authoring, release build throughput
or FPS. Full ENG-03/07/08 and the complete engine roadmap remain open.


## 27 October: deferred locomotion requires a logical model owner

Build now rejects ModelLocomotion combined with ModelPart in an expanded
declared prefab, matching the existing native runtime prohibition. This includes
imported hierarchy containers with node=u32::MAX. Direct descriptors and nested
overrides are both covered; the error identifies prefab and object.

The regression first demonstrated that the incompatible owner was packaged.
Both malformed variants now fail without publication. Removing the incompatible
part permits export and native retarget locomotion after source deletion.
Partial prefabs are not required to have a local character-body ancestor because
that context can be supplied at instantiation; this change checks an unconditional
owner incompatibility only.

Fresh checks: 367 editor tests passed, 36 ignored; strict all-target Clippy,
scoped rustfmt and diff checks passed.
[Evidence and hashes](production-readiness-evidence-2026-10-27/deferred-locomotion-logical-owner/qualification.json).
Full semantic component admission, ENG-03/06, GPU/FPS and engine readiness remain open.


## 27 October: preview checkpoint validates target morph frame and source role

Before including preview in a whole-editor transaction, two native restore gaps
were reproduced: a frame missing required target morph properties was accepted,
and a distinct source rig was accepted without a retarget profile. Preview capture
and restore now validate the frame through ModelAsset.validate_morph_frame and
require shared source/target allocation when no retarget profile is present.
No pose is resampled and no clock advances during admission.

Regression preserves exact valid accepted-frame checkpoint JSON, phase words
and source/target aliasing, rejects both malformed candidates, and verifies capture
also refuses invalid native participants. Existing retarget/model-graph tests pass.
Fresh checks: four preview tests passed; 368 editor tests passed, 36 ignored;
strict all-target Clippy, scoped rustfmt and diff checks passed.
[Evidence and source hashes](production-readiness-evidence-2026-10-27/preview-restore-native-admission/qualification.json).
Preview still needs the outer model-root union and shared frame pool; this does
not complete atomic whole-App publication, native save/load menus, GPU preview
restore or ENG-07/08. The full engine roadmap remains open.


## 27 October: accepted GPU frame pool validates target morph schema

Review of the existing shared accepted-frame pool exposed another restore gap:
a generic skeletal frame without the target model's required morph properties
was accepted. Capture now validates every owner's model/frame pair before assigning
a pool role. Restore applies the same native ModelAsset.validate_morph_frame
admission to both legacy v1 and pooled v2 frames. Archive formats are unchanged.

The regression reproduces the original admission, rejects invalid legacy and
pooled frames, preserves complete valid properties, shares restored Arc frames
only for original shared roles, and keeps equal independent frames separate.
Fresh checks: two checkpoint tests passed; 369 editor tests passed, 36 ignored;
strict all-target Clippy, scoped rustfmt and diff checks passed. Existing native
GPU restore late-rejection/cancellation test passed on Apple M4 Max/Metal.
[Evidence and hashes](production-readiness-evidence-2026-10-27/accepted-frame-target-morph-admission/qualification.json).
The GPU frame pool is still separate from runtime/preview frame ownership; the
whole-editor root/frame graph and atomic App publication remain unfinished.
No whole-App GPU restore, production rig or FPS qualification follows.


## 27 October: runtime/GPU shared accepted-frame roles in the native session file

Play resource capture now records explicit durable owner joins where original
runtime and accepted GPU frames share one Arc. Equal independent frames are not
joined. Existing participant codecs remain unchanged; optional role metadata is
part of the existing bounded/checksummed session envelope. Missing metadata in
older files leaves participants independent rather than guessing from values.

Detached prepare checks role uniqueness and existence, validates the target morph
frame and exact complete native AnimatorFrameCheckpoint JSON, and rebinds runtime
frames to the already restored GPU frame pool. This changes allocation sharing
only: clocks, events, accepted pose bits and render tick metadata are preserved.

Three native M4 Max/Metal full-session variants pass after source deletion and
GPU staging: prior accepted frame, shared current frame, and equal independent
current frame. Pixel references remain exact. Malformed roles and joins between
different states reject. A controlled fault omitting the final shared Arc
publication failed at the intended identity assertion; the correct source was
restored and all three variants rerun successfully.

Fresh checks: 369 editor tests passed, 38 ignored; strict all-target Clippy, scoped
rustfmt and diff checks passed.
[Logs, hashes and negative control](production-readiness-evidence-2026-10-27/shared-runtime-gpu-frame-roles/qualification.json).
Preview and runtime-only frame classes without GPU roles still require outer graph
composition. Whole-App atomic publication, native save/load UI/input handoff,
production character quality and target FPS remain open with the full roadmap.


## 27 October: runtime-only accepted frame classes and transitive GPU joins

Native AnimationRuntimeCheckpoint now records original shared frame classes even
when no GPU owner exists. Optional durable owner roles reference the first
canonical member; values alone never merge independent frames. Prepare validates
bounded canonical topology and exact native accepted-frame data before reuse.
Legacy files without metadata retain independent frames. Per-owner native frame
encoders remain authoritative; this is alias topology, not frame compression.

Regression reproduces the previous loss, retains an inactive owner's shared frame,
keeps an equal independent frame separate, preserves exact frame JSON/serial, and
rejects duplicate, missing, forward/chained roles and conflicting frame contents.
A second regression exposed cross-participant joins splitting existing runtime
classes. The join now validates every class member before transferring all shared
owners together; no partial class is mutated on admission failure.

Fresh checks: 370 editor tests passed, 38 ignored; all three native full-session
GPU variants passed on M4 Max/Metal after source deletion; strict all-target
Clippy, scoped rustfmt and diff checks passed.
[Evidence, both baselines and hashes](production-readiness-evidence-2026-10-27/runtime-only-frame-aliases/qualification.json).
Preview/outer model-root composition, whole-App atomic publication, native
save/load UI/input handoff, physical memory and target FPS remain open.


## 27 October: preview roots join the original resource model graph

Resource capture now unions native runtime roots with preview target/source roots,
keeping runtime indices stable and appending only distinct Arc allocations. The
existing catalog/model encoder remains authoritative. Optional preview root-role
indices are bounded, unique and resolved before workers start; runtime validation
accounts for this same union instead of treating preview-only roots as orphans.

The baseline regression demonstrated a missing preview-only root. Resource and
combined model/animation files now restore it after deleting the original source
directory, with fresh model allocations and exact original catalog/root aliasing.
Invalid indices, duplicate roles and a combined orphan root reject. The real
retarget-preview fixture also verifies two distinct target/source allocations and
both catalog aliases through the resource graph against an empty provider root.

Fresh checks: 371 editor tests passed, 38 ignored; three native full-session GPU
variants passed; strict all-target Clippy, scoped rustfmt and diff checks passed.
[Evidence and source hashes](production-readiness-evidence-2026-10-27/preview-resource-model-root-union/qualification.json).
This is preview model-root ownership, not yet accepted preview frame/phase
composition in the editor file. Preview/runtime/GPU frame joining, whole-App
atomic publication, native save/load UI and target FPS remain open.


## 27 October: accepted preview state in the existing resource snapshot file

Resource snapshots now contain the original PreviewCheckpoint, not just model
roots: durable owner, exact f64 phase words, settings/profile and accepted native
frame. The same codec separates model/frame admission from scene binding; no pose
is resampled, no dummy scene is introduced, and no gameplay/event clock advances.
Root-role identity and native frame limits are checked before workers start.
Model/animation and Play-resource preparation retain the bound preview through
the existing resource owner into detached GPU-ready candidates.

Original preview/GPU Arc sharing is recorded as an explicit owner role. Prepare
adopts the existing restored GPU pool only after exact native frame comparison;
equal independent frames stay independent. CPU source-free regression preserves
0.25 plus one f64 mantissa bit and complete checkpoint JSON, and rejects missing
participants/owners, root mismatches and shared roles without accepted GPU frames.

On M4 Max/Metal, shared and independent variants survive source deletion and GPU
staging with exact phase/frame JSON, correct Arc identity and identical nonblank
64x64 geometry pixels. A controlled fault omitting Arc publication failed at the
intended identity assertion; final source was restored byte-for-byte and retested.
Fresh checks: 371 editor tests passed, 39 ignored; four native GPU App tests and
three full-session GPU tests passed; strict all-target Clippy, scoped rustfmt and
diff checks passed.
[Evidence, hashes and negative control](production-readiness-evidence-2026-10-27/preview-state-resource-file/qualification.json).
Direct preview/runtime sharing without a GPU bridge, idle whole-editor lifecycle,
remaining authoring/Graphics ownership, whole-App atomic publication and native
save/load UI/first-presented-frame handoff remain open. Geometry pixel parity on
the same renderer does not qualify native window presentation or target FPS.


## 27 October: preview/runtime frame sharing without GPU and three-way joins

The existing resource envelope now records original preview/runtime frame Arc
sharing as a durable runtime-owner role. Source identity determines the relation;
equal independent frames are not joined. Prepare resolves the native runtime
frame and validates exact complete frame state before adopting it for preview.
When a GPU role is also present, the existing runtime class is joined to that
accepted GPU pool; Play preview binding follows runtime/GPU class restoration.
A compact capture descriptor carries the original preview codec and both roles.

Regression first reproduced lost sharing after source deletion. Shared and equal
independent bind-pose variants now retain correct identities, including an inactive
source model, exact f64 phase/frame JSON and runtime serial. Missing roles, nodes
without runtime frames and conflicting native frame contents reject. The native
M4 Max/Metal fixture now additionally verifies preview/runtime/GPU three-way Arc
identity, with exact source-free checkpoint and 64x64 geometry pixel parity.
Existing active-clip GPU-only shared/independent variants also pass.

Checks: 372 editor tests passed, 39 ignored; four GPU App and three full-session
GPU tests passed; strict all-target Clippy, scoped rustfmt and diff checks passed.
The preview GPU test was rerun after lint-only Copy callback argument cleanup.
[Evidence and source hashes](production-readiness-evidence-2026-10-27/preview-runtime-frame-joins/qualification.json).
Whole-App atomic publication, idle whole-editor lifecycle, native save/load menus
and first-presented-frame handoff remain open. New three-way identity proof uses
bind pose and does not qualify all animation modes, production rigs or target FPS.


## 27 October: idle editor snapshots through the existing session codec

The original PlaySessionCheckpoint now represents both running Play (legacy v1)
and idle authoring (v2). Idle capture saves the native SceneDocument without
starting a SceneSimulation or manufacturing a Stop document. Running snapshots
retain their original field representation. Strict envelope admission rejects
mixed lifecycle participants, absent idle scenes and incompatible mode versions.
All animation, input, physics and other existing participants retain their native
codecs, budgets and detached preparation.

A regression first failed with `Play checkpoint requires running simulation`.
It now restores an inactive session and re-encodes the same complete JSON. Five
malformed lifecycle combinations reject both encoding and decoding. An additional
native editor-file test creates a project, edits and undoes, reaches the shared
resource/UI/audio checkpoint barrier while idle, writes and reads the original
checksummed editor file, and prepares its scene and history against one fresh
scene identity. Full serialized undo/redo history matches after five operations;
the restored game stays idle and detached candidate workers are explicitly joined.

Checks: 374 editor library tests passed, 39 ignored. No GPU/window/FPS or native
audio-device qualification was performed. Whole-App atomic publication, native
save/load menus, preview-bearing idle editor integration and first-presented-frame
handoff remain open; ENG-03 through ENG-13 are not complete.
[Evidence](production-readiness-evidence-2026-10-27/idle-editor-checkpoint/qualification.json).


## 27 October: preview-bearing idle editor file after source deletion

The idle editor integration qualification now covers both an empty-resource project
and a project with an imported animated GLB, authored animation component and an
accepted preview at f64 phase 0.25 plus one low mantissa bit. The project is edited,
undone and frozen through the shared model/UI/audio barrier. The original editor
file is saved, read and re-encoded, then the entire original source directory is
deleted before detached preparation against an empty destination root.

The prepared candidate remains idle, binds its preview to the same fresh scene
as authoring history, and reproduces the original complete native preview JSON
including phase and accepted animation frame. Five undo/redo operations compare
full serialized history. Candidate workers and source workers are closed explicitly.
The model-bearing case requires the existing opt-in runtime model binding codec;
authoring-only component registries intentionally do not encode ModelInstance.
The initial qualification exposed this missing registry opt-in in the new test;
no production codec or import behavior was changed to bypass admission.

This is a full-file detached prepare proof, not whole-App installation or native
presentation. Guarded atomic publication, menus, first-presented-frame handoff,
production assets and FPS remain open, along with ENG-03 through ENG-13.
[Evidence](production-readiness-evidence-2026-10-27/idle-editor-preview-file/qualification.json).

Checks for idle preview file qualification: 375 editor library tests passed, 39
ignored; strict all-target Clippy, scoped rustfmt and diff checks passed.


## 27 October: coordinated GPU staging retains original outer owners

PreparedCoordinatedPlay can now stage the original PlayResources GPU restoration
without separating its prepared project, UI, audio and historical UI input. Pending
and ready coordinated candidates retain those outer owners. The existing immutable
PlayResourceGuard is shared by Arc between the coordinated source observation and
native GPU candidate; existing standalone Play staging keeps its original API.
Before admission the ready coordinated candidate validates all native source owners
and the destination scene/device/cache. The outer source guard already validates
the game state, so destination validation does not redundantly serialize it again.
Explicit discard returns all model, watcher, UI and audio worker handles for joining.

All three original full-session physical M4 Max/Metal GPU variants now exercise
this coordinated path after source-directory deletion: previous accepted frame,
shared runtime/GPU frame and equal independent frames. Unguarded candidates and
candidates missing historical UI input reject before upload. Pending cancellation
and ready discard leave the live upload cache unchanged. Altering native audio
output policy or pending gameplay input after GPU staging rejects the ready
candidate; restoring those values permits validation again. Existing exact accepted
frame, pixel, runtime clock and source-free continuation checks still run.

This joins the detached GPU and outer-owner preparation paths. It does not install
them into App, transfer native presentation/audio output devices, or close the
remaining authoring-history publication, menus or first-presented-frame gates.
ENG-03 through ENG-13, production content and FPS remain open.
[Evidence](production-readiness-evidence-2026-10-27/coordinated-gpu-staging/qualification.json).

Coordinated staging checks: three physical GPU variants passed, including direct
new-candidate pixel/Arc assertions; 375 editor library tests passed, 39 ignored
before those final ignored-test assertions; strict all-target Clippy, scoped
rustfmt and diff checks passed.


## 27 October: editor history accompanies coordinated GPU preparation

PreparedEditorCheckpoint now stages its original PreparedAuthoringHistory with
PendingCoordinatedPlay and carries it through GPU validation into a ready editor
candidate. Scene identity is checked before uploads, after validation and during
final source/destination admission. Pending cancellation and ready discard use
the original coordinated cleanup path and return worker handles for joining.
No new history, scene, resource or animation representation was introduced.

A physical M4 Max/Metal test creates an actual project and animated model through
App, authors animation, duplicates and undoes to retain redo, previews a phase with
a low f64 mantissa bit and captures the full ordered model/UI/audio barrier. It
writes and reads the original editor file, deletes the original source directory,
and prepares against an empty destination. The ready candidate's history order,
selection, node bindings, preview and accepted GPU geometry belong to one fresh
scene. Complete preview JSON and nonblank rendered pixels match the source. Five
undo/redo steps compare full serialized source and restored histories.

Wrong prepared scene identity rejects before upload; wrong ready history identity
rejects admission. An otherwise valid ready candidate rejects after source history
changes. Pending cancellation and
ready discard leave the live destination cache unchanged. This is detached
candidate preparation and admission, not atomic App installation, native menus,
first-presented-frame handoff, production-rig or FPS qualification. Full ENG-03
through ENG-13 remain open.
[Evidence](production-readiness-evidence-2026-10-27/editor-history-gpu-staging/qualification.json).

Editor history staging checks: five GPU App tests and 375 editor library tests
passed (40 ignored). Strict Clippy required boxing the 27 KiB inner GPU future;
Clippy and the native editor-history GPU scenario passed again after that change.
Scoped rustfmt and diff checks passed.


## 27 October: exclusive App installation of the prepared editor snapshot

App now owns an asynchronous installation boundary for ReadyEditorCheckpoint.
It validates the native source observation and history scene, prepares all outer
owners and scan timing, validates exact live/replaced GPU inventory, then acquires
the existing audio and UI publication permits. The original UI-after-models GPU
transaction settles before any scene/history/resource owner is replaced. The
remaining publication has no fallible operations or awaits: combined GPU cache,
model/UI owners, audio, native scene/Play/camera, authoring metadata/history/order,
model catalog/dependencies/import workers and historical UI input are installed
under one exclusive App borrow. Derived viewport caches and stale interactions
are invalidated, and the old restore fences are released.

The installed authoring project keeps the source authoring component registry;
checkpoint-only ModelInstance codecs do not leak into ordinary authored scene
files. Source model/watch workers and old UI/audio workers transfer to an explicit
retirement owner. It retains the old scene with its old PlaySession, dispatches
native stop callbacks against that scene after publication, catches retirement
callback panics, and still joins all closed workers. Retirement errors are
separate from installation admission errors and cannot imply a rollback.

The physical M4 Max/Metal editor-file test now stages against actual App graphics
and installs the source-free snapshot. A wrong target history identity preserves
the original scene/history tokens and GPU cache while retaining the source fence.
Success installs the prepared scene/history, preserves preview phase and exact
nonblank geometry pixels, releases both fences, retains authoring registry identity
and excludes runtime model descriptors from authoring documents. The ordinary
App offscreen scene render path executes after installation. A CPU retirement test
checks old-scene callback identity and worker joining with and without callback
panic.

Qualification remains bounded: idle App installation and native offscreen output;
full running-session App installation, nonempty UI/audio installation integration,
native output-device transfer, native surface presentation, menus, project Save As,
production assets and FPS are still open. This does not close ENG-03 through ENG-13.
[Evidence](production-readiness-evidence-2026-10-27/editor-app-installation/qualification.json).

App installation checks: 376 editor library tests passed, 40 ignored; five physical
GPU App tests passed; strict all-target Clippy, scoped rustfmt and diff checks passed.


## 27 October: running App installation with accepted UI and offline audio

The original editor-file/App installation qualification now has idle and running
variants through one shared test workflow. The running case declares its tone in
the project manifest, authors an AudioSource and visible UI rectangle, starts Play
through the ordinary audio preparation path and waits for an accepted UI GPU
publication before saving. Native simulation/UI/physics and accepted input-policy
limits are retained; the initial small empty-participant test policies correctly
rejected this real content and were replaced by matching bounded policies.

The complete editor file is written and read, then the entire source directory
is deleted. The existing prepared/guarded GPU pipeline installs the running scene,
original history and accepted render frame into actual App. Native UI publication
bytes match the saved source exactly. The installed audio playback checkpoint
matches the source including imported inputs, accepted clips, runtime state,
catalog and clock. A separately prepared native audio owner supplies an independent
continuation reference. Both owners run their original reload polling, preserving
last-good accepted clips after missing-source failures; a reference without reload
correctly differed in catalog failure state and is retained as a rejected control.

After installation the normal App game schedule executes 120 additional 1/60-second
steps and 120 ordinary offscreen scene renders. Simulation advances 120 ticks;
audio advances 96,000 stereo PCM frames, with exact frames/peak at every step and full native
playback checkpoint equality against the independent owner at the end. Stop then
restores authoring and releases simulation/audio owners. Existing wrong-scene
rejection, source-history guards, cancellation, worker retirement, exact accepted
geometry pixels and authoring schema checks run in both modes.

This closes the previously unqualified running/nonempty-rectangle/offline-audio
App installation scenario. It does not qualify all gameplay systems/content,
text-bearing App installations, native surface/input presentation, native audio
output queues, menu commands, Save As, production scenes or target FPS. ENG-03
through ENG-13 and the complete engine goal remain open.
[Evidence](production-readiness-evidence-2026-10-27/running-editor-ui-audio-installation/qualification.json).

Running UI/audio installation checks: 376 editor library tests passed, 41 ignored;
six physical GPU App tests passed; strict all-target Clippy, scoped rustfmt and
diff checks passed.


## 27 October: text/font atlas through actual running App installation

The original running editor-file qualification now has a Unicode-text variant
alongside idle and rectangle-only running cases. It copies a local TrueType font
into the temporary project, declares it in the manifest and authors a UI label
containing Latin, Cyrillic and accented characters. The ordinary asynchronous UI
importer publishes actual glyph-atlas GPU draws before the full checkpoint barrier.
The existing bounded font/run/pixel policies retain the source capacities; no
rendering, shaping, precision or quality path is replaced for the qualification.

The full editor file is saved and read, and the entire original project directory,
including the font, is removed before preparation and App installation. Native UI
publication bytes compare exactly after installation, including accepted font data,
glyph metrics and atlas pixels. The existing native UI restoration pixel harness
is shared through test-only visibility rather than introducing another renderer.
Separate 128x128 UI-only GPU readbacks match exactly before and after installation;
a nonblank white-glyph check and actual atlas-texture draw verify the test is not
just comparing rectangle backgrounds. The same pixels still match after the 120
normal App game/render cycles and failed rereads of the deleted source files.

The existing exact audio playback reference, 120 simulation ticks/96,000 stereo
PCM frames, guarded failure/cancellation, authoring history and Stop checks remain
in this variant. The chosen font byte count and BLAKE3 digest are logged; system
font bytes are not added to the repository or evidence folder.

This is source-free text-bearing App installation and offscreen GPU parity. Native
surface/input presentation, real output-device queues, menu integration, Save As,
all languages/fonts, production content and 120/160 FPS remain open, along with
full ENG-03 through ENG-13 and the complete engine objective.
[Evidence](production-readiness-evidence-2026-10-27/text-font-app-installation/qualification.json).

Text/font installation checks: 376 editor library tests passed, 42 ignored; seven
physical GPU App tests passed; strict all-target Clippy, scoped rustfmt and diff
checks passed.


## 27 October: custom-codec registry fork and background editor-file save command

ComponentRegistry now clones its schema metadata while retaining immutable typed
reference-discovery/remapping callbacks through Arc. Extending a checkpoint
registry no longer requires rebuilding the built-in registry and losing project
component codecs. A native scene regression verifies custom reference remapping,
independent registrations and independent collection defaults on both registry
views. No live component or scene data is cloned by this operation.

App now accepts an explicit editor-file save request with bounded participant/file
policies. It progresses the original ordered model/UI/audio fence, captures through
the original editor codec using a fork of the actual authoring registry, then moves
the immutable archive to a named worker thread. Serialization, atomic file writing
and file sync run on that thread. The normal viewport polls only completed worker
handles; it never joins an unfinished writer. Both success and failure resume all
original fences before returning the final receipt. Shutdown finishes an already
started write outside frame-critical polling and reports cancellation if capture
has not begun.

Headless regressions verify a project-defined component with integer value
9,007,199,254,740,993 survives file read and detached preparation; duplicate requests
reject; capture-budget and writer-budget failures preserve the old file and native
scene token while releasing all fences; shutdown completes and reads a started
atomic write. A channel-blocked writer proves frame polling returns before the
worker can finish and later resumes correctly. Existing source-free GPU/App
installation scenarios still pass after the native registry representation change.

This is the native save command/state machine and frame/shutdown integration.
Buttons/keybindings, file selection/default policies, the nonblocking load command,
native output-device barriers and native surface presentation remain open. Snapshot
capture itself is synchronous under the source fence; these tests do not establish
capture latency, frame pacing or 120/160 FPS. Full ENG-03 through ENG-13 remain open.
[Evidence](production-readiness-evidence-2026-10-27/background-editor-save/qualification.json).

Background-save checks: 380 editor library tests passed, 42 ignored; 99 scene tests
and seven physical GPU App tests passed; strict all-target Clippy for both crates,
scoped rustfmt and diff checks passed.


## 27 October: owned UI/GPU installation phases without an App borrow across checks

The native combined model/UI transaction can now release its source publication
borrow after staging and retain an owned pending GPU candidate. The original
error-scope checks, uploaded draws, combined cache, prepared UI and model owners
are preserved. Rebinding requires the original held UI fence, unchanged native UI
observation, matching prepared scene and revalidated destination device/queue/cache
and geometry inventory. The enclosing coordinator also revalidates its original
source epoch. This adds no durable UI or scene format and no alternative renderer.

App installation is split into owned pending/ready installation phases and a
synchronous final publication. The source observation is checked before staging
and again immediately before reacquiring UI/audio publication permits. The prior
exclusive asynchronous helper delegates to these same phases. Final owner swaps
and retirement remain unchanged and infallible after admission. Pending and ready
candidates provide explicit worker-retirement paths for cancellation.

Source UI observation includes native layout values, original accepted text Arc
identities and the awaiting-first-presentation flag. A CPU regression models an
acknowledgment that changes only that flag while published/presented layouts stay
equal; the new observation rejects it. This closes a gap that a layout-only or
serialized-publication-only check would not detect after releasing the borrow.

All idle, running rectangle/audio and running text/font GPU App installation cases
now stage the owned UI phase, modify native source input while App remains
available, settle GPU validation and reject final publication with the specific
native Play-runtime-change error. Restoring input preserves source scene/history
and cache tokens. Explicit cancellation after UI uploads also preserves those
owners; subsequent successful installation retains exact pixels, archives, clocks,
120-step continuation, Stop and retirement checks.

The owned phases are ready for event-loop polling, but the nonblocking file-read/
prepare command, its frame scheduling, UI actions and native presentation are not
wired or qualified by this milestone. Source observation must be scheduled after
the source frame's accepted publication work; later source/GPU changes correctly
reject rather than publish a mixed version. Frame pacing and FPS remain open,
as do full ENG-03 through ENG-13 and the complete engine goal.
[Evidence](production-readiness-evidence-2026-10-27/owned-ui-installation-phases/qualification.json).

Owned installation checks: 381 editor library tests passed, 42 ignored; seven
physical GPU App tests passed; strict all-target Clippy, scoped rustfmt and diff
checks passed.


## 27 October: background file/prepare and frame-polled full editor load command

App now accepts a bounded snapshot-file load request. The original file read,
checksummed decoding and detached CPU preparation execute on named worker threads.
Only a successfully decoded file acquires the original model/UI/audio barrier;
corrupt and oversized files leave the running source unfrozen. Preparation uses a
fork of the actual project component registry and its existing Send/Sync behavior
and UI-action restore factories. Save/load requests exclude each other.

The frame owner advances original model GPU checks, owned UI GPU checks and the
final guarded publication. Device polling uses Poll and futures use ordinary
nonblocking polling; unfinished IO threads are never joined by the frame poll.
Old simulation callbacks execute against the old graph at the publication
boundary, while already-closed worker handles retire across subsequent polls.
Shutdown joins file/prepare threads outside frame processing and discards detached
candidates. Failed preparation resumes the held source fences before reporting.
Interactive cancellation and menu/file-chooser policy remain separate open work.

The load poll lives in a common successful-frame wrapper, after source resource
adoption and presentation bookkeeping. A genuine first integration run remained
in Reading because the offscreen return bypassed the initial tail-only hook. The
wrapper now covers offscreen and skipped-presentation success paths as well as the
normal native-window branch; no old-frame outcome can acknowledge a newly installed
UI. Native surface/occlusion/device-loss behavior is still not hardware-qualified.

The host pacing Instant is now rebased at actual installation and after old stop
callbacks in the polled command, rather than at potentially much earlier CPU
preparation. Serialized simulation accumulators and audio clocks remain unchanged.
A controlled removal of the publication rebase fails the specific monotonic epoch
assertion; restoring the exact source passes. Preparation/GPU wait time is therefore
not treated as elapsed game time by the published host epoch.

Headless controls verify corrupt/oversized reads never freeze or mutate source,
duplicate/save requests reject, a delayed reader does not block frame polling,
and incompatible preparation preserves file and scene while releasing both fences.
The original idle, running rectangle/audio and running text/font GPU App scenarios
now additionally request the file command and drive it solely through ordinary
128x128 offscreen frames after source-directory deletion. Each command settles,
installs a fresh scene, releases fences, restores exact simulation/audio counters,
and matches accepted geometry and text UI pixels. Existing 120-step continuation,
independent audio reference, Stop, rollback and cancellation-layer proofs still run.

This qualifies the internal frame-polled load command on the existing offline-audio
and offscreen GPU path. User actions/default policies/file selection, interactive
cancel, failing-source-frame recovery, native audio output queues, real window/input
presentation, physical memory, load/capture latency and target FPS remain open.
Full ENG-03 through ENG-13 and the complete engine goal are not closed.
[Evidence](production-readiness-evidence-2026-10-27/frame-polled-editor-load/qualification.json).

Frame-polled load checks: 384 editor library tests passed, 42 ignored; seven
physical GPU App tests passed; strict all-target Clippy, scoped rustfmt and diff
checks passed. Controlled host-epoch regression failed as expected; source was
restored byte-for-byte and its native GPU test passed again.


## Дополнение: отменяемая загрузка редактора, 27 октября 2026

Frame-polled загрузка получила кооперативную отмену до публикации сцены.
Отменённый запрос освобождает только свой барьер (идентичность `Arc` epoch),
сохраняет текущую сцену и дожидается завершения CPU/GPU-подготовки без join
незавершённых потоков в кадре. Кандидаты закрываются без установки; завершившиеся
потоки собираются отдельно. После публикации отмена не принимается.
Освобождение после ошибки также привязано к исходному epoch.

Проверено: 5 load-тестов; полный editor lib — 386 passed, 42 ignored;
7 offscreen GPU App-тестов на Apple M4 Max/Metal; strict all-target Clippy,
scoped rustfmt и diff check. Три настоящих GPU file-install сценария дополнены
отменой в Models и Ui с проверкой сохранения исходной сцены, затем обычной
успешной загрузкой. Native window/audio-device lifecycle и FPS не проверялись.
Кнопка/команда UI для отмены ещё не подключена; ENG-03–13 остаются открытыми.
[Логи, границы проверки и хеши исходников](production-readiness-evidence-2026-10-27/cancellable-editor-load/qualification.json).


## Дополнение: прогресс загрузки и отмена в панели, 27 октября 2026

Существующая нативная панель показывает реальные стадии frame-polled загрузки:
чтение, ожидание кадра, подготовка сцены, модели, UI, восстановление/завершение.
Пока загрузка активна, её статус заменяет Save, а Cancel load (Esc) — Load;
неактивные стадии не публикуют hit targets. Изменения стадии инвалидируют кэш
и прежнюю презентацию панели. Кнопка проходит через общий panel_action и
проверяет текущее состояние; запоздалое нажатие не меняет сцену. Escape
перехватывается до game UI и не теряется при работающей игре.

Проверены load-тесты с реальным построением панели, отменой через action adapter,
неактивными targets и stale action; четыре GPU panel-теста на Apple M4 Max/Metal
проверили glyph pixels и presented hit target, включая новый статус/Cancel.
Полный editor lib и strict all-target Clippy прошли, логи сохранены.
Окно не запускалось. Подключение команд открытия/сохранения полного checkpoint,
выбора пути и production-политики бюджетов остаётся следующим шагом; прежние
Save/Load авторской сцены сохраняют свой формат и поведение.
[Доказательства и хеши](production-readiness-evidence-2026-10-27/editor-load-progress-controls/qualification.json).


## Дополнение: команды полного снимка и desktop-профиль, 27 октября 2026

Полный `.voxyedr` снимок теперь доступен через Ctrl/Cmd+Shift+S (сохранить)
и Ctrl/Cmd+Shift+O (открыть). Ввод пути показан в существующей панели и заголовке
окна; Enter подтверждает, Escape отменяет. Относительный путь разрешается от
корня текущего проекта; абсолютный поддерживается. Имя по умолчанию выделено
для замены при первом вводе; Ctrl/Cmd+A повторно выбирает весь путь, Backspace
обрабатывает Unicode. Чужое расширение, управляющие символы и путь свыше 4096
байт отклоняются до запроса операции. Замена сцены делает старый draft неактивным.
Обычные F5/F9 продолжают работать с авторской сценой в прежнем формате.
Стадии сохранения теперь также отображаются в панели; незавершённая атомарная
запись не предлагает ложную возможность отмены.

Введён отдельный desktop admission profile, не использующий тестовые helpers.
Play и профиль используют общий SimulationLimits (60 Hz, 8 steps, 128 behaviors
и commands); бюджеты допускают текущие рабочие physics/UI, текст/шрифты, модели,
историю, звук и native liquid. Превышение отклоняется без обрезки или изменения
точности. Политика задаётся хостом, а не принимается из загружаемого файла.
Пользовательская настройка бюджетов проекта пока не подключена.

Настоящий GPU-сценарий выявил ошибочное применение per-source 16 MiB к исходному
aggregate input budget 32 MiB. Исправлено общей константой импортера и политики;
сохранён как исходный падающий лог, так и успешный повтор. Через реальные path
commands сохранены и открыты model/font/UI/offline-audio состояние: geometry/UI
pixels, ticks и audio frames совпали. Финальные проверки: 388 editor tests passed,
43 ignored; 7 GPU App tests passed на M4 Max/Metal; strict all-target Clippy и
scoped format/diff checks прошли. Native window и audio-device lifecycle остаются
неподтверждёнными; окно не открывалось. ENG-03–13 не закрыты.
[Логи, ревизия и хеши](production-readiness-evidence-2026-10-27/desktop-snapshot-commands/qualification.json).


## Дополнение: восстановимые ошибки снимков, 27 октября 2026

Ошибки frame-polled сохранения и загрузки больше не записываются в фатальный
`App.error`. Общие адаптеры кадровых команд публикуют отдельную диагностику в
панели/заголовке и оставляют основной runtime доступным. Native path-command
ошибки используют тот же маршрут. Escape закрывает сообщение, новая принятая
операция очищает прежнюю ошибку. Ошибки draw/runtime сохраняют исходный фатальный
маршрут; сообщение о снимке не утверждает rollback после уже выполненной
публикации и ошибки retirement.

Проверен полный отказоустойчивый сценарий: corrupt read → редактирование сцены →
writer IO failure → успешное сохранение → независимая новая ошибка. Исходные
scene tokens и файл сохраняются, барьеры освобождаются, fatal error не появляется.
В настоящем offscreen GPU App через path commands проверены corrupt read и
ошибка записи, последующие кадры текущей сцены и успешное повторное открытие
model/font/UI/offline-audio снимка с исходными ticks/audio frames. Отдельный GPU
panel-тест использует реальное App-сообщение и проверяет glyph pixels всех трёх
строк, отсутствие overlap с controls.

Проверки: 389 editor tests passed, 44 ignored; 7 GPU App и 5 GPU panel tests passed
на M4 Max/Metal; strict all-target Clippy, scoped rustfmt и diff check прошли.
Native window и audio-device lifecycle, FPS и весь ENG-03–13 остаются открытыми.
[Доказательства, логи и хеши](production-readiness-evidence-2026-10-27/recoverable-snapshot-errors/qualification.json).


## Дополнение: кадровый цикл активного окна, 27 октября 2026

Убран unconditional `WaitUntil(now + 16 ms)` в editor about_to_wait. Активное
неперекрытое окно ненулевого размера теперь использует `ControlFlow::Poll` и
исходный request_redraw; pacing по умолчанию остаётся за VSync GPU-презентации.
Для occluded/нулевого размера сохранена прежняя фоновая политика 16 ms; при
восстановлении запрашивается redraw. Без окна выбирается Wait. Это изменение
не меняет fixed-step simulation и не вводит пропусков качества/точности.

`VOXY_EDITOR_PRESENTATION=vsync` (по умолчанию) или `unthrottled` задаётся при
создании App. Неверное значение отклоняется. Unthrottled использует исходный
SceneSurface::enable_unthrottled_presentation: поддерживаемый mode выбирает
renderer, выбранное значение логируется; отсутствие поддержки — ошибка, без
скрытого fallback. Новый encoder, renderer или отдельная система презентации
не создавались. Рекомендация Poll+VSync и поведение macOS таймера проверены по
локальным исходникам используемого winit 0.30.13; выдержки сохранены в evidence.

390 editor tests и 7 offscreen GPU App regressions прошли; strict all-target
Clippy, scoped rustfmt и diff check прошли. GPU regressions проверяют прежние
сцены/восстановление ресурсов, а не native scheduling. Настоящее окно не открывали:
ни 120/160 FPS, ни native frame pacing/latency/power этим этапом не подтверждены.
Требуется контролируемый оконный before/after на заданных сцене, качестве,
разрешении, presentation mode и устройстве. Полный ENG-03–13 остаётся открытым.
[Границы проверки, логи и хеши](production-readiness-evidence-2026-10-27/native-frame-cadence/qualification.json).


## Дополнение: асинхронное управление native audio, 27 октября 2026

Исходный OutputDeviceWorker получил request_pause/request_resume и poll_paused.
Platform stream pause/play выполняет исходный поток-владелец; кадровый поток
только отправляет команду и неблокирующе проверяет подтверждение. Одновременно
допускается один переход, одинаковый pending request идемпотентен, противоположный
отклоняется. После ошибки native API состояние неизвестно: прежний ack не выдаётся
как текущее состояние, recovery pause/resume действительно доходит до владельца.
Закрытие не ждёт зависший native API и подавляет поздний ack; join оставлен для
shutdown. Паника owner не превращается в успешное подтверждение.

Проверено 10 audio-device тестов, включая блокировку, поздний ack, failed pause,
failed resume, recovery, panic, owner-thread destruction и существующее точное
сохранение неподанного PCM в PcmPump; полный editor lib — 391 passed, 44 ignored.
Strict Clippy для audio_device/editor all-targets, scoped rustfmt/diff check прошли.
Controlled owner использует исходный worker protocol; физический CPAL stream и
окно не открывались. Это реализация prerequisite API, а не native audio checkpoint.

Native guard AudioSession::checkpoint_boundary намеренно сохранён и дополнительно
проверен регрессией: Native admission/request отклоняются без захвата barrier,
Offline после этого работает. Queue PCM и позиция hardware output пока не входят
в снимок; новый pause/resume ещё не подключён к AudioSession snapshot/UI Pause.
Следующий необходимый шаг — точный захват/восстановление исходной device queue,
композиция с PcmPump suffix и clock/rate, rollback/cancel через исходный owner и
затем реальная hardware qualification. ENG-03–13 и общий выпуск остаются открытыми.
[Логи, границы проверки и хеши](production-readiness-evidence-2026-10-27/async-native-audio-control/qualification.json).


## Дополнение: точная очередь PCM и пакетный PcmPump, 27 октября 2026

В исходный CPAL callback подключена bounded lock-free ArrayQueue (готовый
crossbeam-queue 0.3.14). Колбэк не получает mutex или новые allocations. Уникальный
RAII freeze token запрещает новые операции и неблокирующе ждёт уже допущенные;
закрытие токена размораживает ту же очередь. Захват сохраняет исходные биты PCM,
порядок и capacity; prepared-кандидат проверяет ограничения и нормализованные
finite samples. Публикация в исходную замороженную очередь возвращает её старое
состояние для rollback. Неуспешный/mismatched кандидат не меняет исходную очередь.
Физический stream и queued host buffers остаются отдельными владельцами.

Скалярный путь с новым freeze учётом оказался дороже прежнего std channel
reference, поэтому production PcmPump переведён на submit_frames с единым
admission на блок. С исходным single-frame pump алгоритмом побитово совпали
renderer clock/cursor/PCM на 720 irregular callbacks при capacities 1/5/128.
Отдельно прошли 720 циклов queued PCM + оригинальный PcmPump unsent suffix,
отмена in-flight freeze, rollback, невалидные budgets/PCM и закрытие.
100 повторов release concurrent stress: 2 млн stereo frames и 10 тыс. freezes,
с сохранением каждого ожидаемого сэмпла в исходном порядке.

CPU microbenchmark: девять троек с ротацией порядка и 1,048,576 frames на вариант,
последний повтор после завершения наших build/test/clippy. Пакетный путь быстрее
reference прежнего канала; scalar остаётся дороже. Точные числа/отношения, все
контрольные прогоны и ограничения приведены в cpu-pairs.json. Это сравнение
алгоритмов очереди на CPU, не before/after FPS или замер физического устройства.

17 audio-device tests passed в debug и release (1 benchmark ignored), 391 editor
tests passed (44 ignored); strict all-target Clippy для обоих crates и scoped
format/diff checks прошли. Native audio guard сохранён: typed queue record ещё
нужно соединить с persistent AudioSession archive, device pause ack, PcmPump и
host clock/rate, затем квалифицировать на реальном устройстве. Во время
изолированного freeze callback выдаёт намеренную тишину, поэтому один token не
заменяет pause и не доказывает audible continuity. Окно/устройство не открывались.
ENG-03–13 и весь выпуск остаются открытыми.
[Логи, CPU-пары, ограничения и хеши](production-readiness-evidence-2026-10-27/checkpointable-pcm-queue/qualification.json).


## Дополнение: бинарный output participant в playback archive, 27 октября 2026

Добавлен VOXYOUT1: ограниченный binary transport для sample rate, stereo f32 bits,
queue capacity/order, полного исходного PcmPump block и cursor. Header+payload
проверяются BLAKE3. Header sizes/counts, aggregate bytes, rate, capacity и cursor
проверяются до декодирующих allocations; PCM проверяется на finite/normalized без
пересчёта сохранённых битов. Capture проверяет идентичность connection/freeze.
Prepared publication меняет исходную замороженную queue и pump вместе, возвращает
точный старый participant для rollback, отклоняет повторно consumed candidate.
Source pump limits проверяются до копирования заведомо over-budget блока.

Исходный AudioPlaybackCheckpoint получил optional output field (None не пишется,
старый offline JSON остаётся тем же), логическую capture-композицию и отдельную
prepare_output. Обычный offline prepare явно отклоняет output-bearing archive,
чтобы не потерять queued PCM при незавершённом native handoff. Clock/remainder,
состояние микшера и playback counters остаются в исходных outer records.

Проверено: binary roundtrip → original queue/pump publication → 720 irregular
callbacks с идентичными PCM bits, cursor и renderer clock; rollback; чужой freeze;
неприемлемое прежнее состояние; truncation на каждом offset, corruption каждого
байта, trailing bytes, quotas/rate; семантически плохой PCM/counts с пересчитанным
checksum. В исходном playback archive проверены optional JSON roundtrip, staging,
corrupt embedded output и отказ от тихого offline fallback.
20 audio-device tests passed в debug/release (1 benchmark ignored), 391 editor
tests passed (44 ignored); strict all-target Clippy и scoped format/diff checks
прошли. Физический native AudioPlay capture/helper, устройство и окно не проверяли.
Native AudioOwner/AudioSession guards сохранены: нужна композиция с device pause
ack, original reload barriers и native publication, затем hardware qualification.
Host buffers/playback position этим record не сохраняются. FPS и весь ENG-03–13
остаются открытыми.
[Доказательства, логи и хеши](production-readiness-evidence-2026-10-27/pcm-output-archive/qualification.json).


## Дополнение: audio resume после остальных владельцев, 27 октября 2026

Перед подключением native queue обнаружен неверный release order: исходный
resume_restore_barrier освобождал audio прежде UI/models. При поздней ошибке
model watcher игра оставалась frozen, но audio save owner уже был отпущен.
Теперь сначала возобновляются UI и models; audio остаётся held, пока любой из
этих владельцев pending или возвращает ошибку. Audio возобновляется последним,
после чего сохраняется исходный финальный rebase simulation_time.

Настоящая регрессия на прежнем коде упала на audio-held assertion. Она вызывает
реальный watcher-disappearance failure: удерживает прежние watcher/ready record,
временно моделирует их потерю, проверяет held audio checkpoint и неизменные
scene token/ticks/host epoch. Восстановление исходного watcher/record позволяет
завершить release, очистить fences и продолжить игровой цикл. Negative-control
и успешный повтор сохранены. Проверки: 392 editor tests passed, 44 ignored;
7 offscreen GPU App tests passed на M4 Max/Metal, strict all-target Clippy,
scoped format/diff checks прошли.

Это необходимое исправление общего release protocol, а не завершённый native
AudioSession barrier. Native admission guards сохранены; реальное устройство,
окно и timing resume не проверялись. Составление pause ack + queue token +
AudioOwner reload/архив остаётся следующим шагом. ENG-03–13 не закрыты.
[Доказательства, negative control и хеши](production-readiness-evidence-2026-10-27/audio-resumes-after-resource-fences/qualification.json).


## Дополнение: native source capture в AudioSession, 27 октября 2026

Native capture подключён к исходным AudioSession/AudioOwner: admission требует
живую connection исходного worker и того же playback output. Worker хранит
идентичность опубликованной connection; NativeCheckpoint удерживает её до конца.
Reload получает исходный Draining fence, затем запрашивается pause; при отказе
request reload откатывается без ожидания IO. Только после native pause ACK
захватывается queue token и архивируются original playback/clock/reload и PCM
output. Resume/cancel ждёт pending pause, возобновляет original reload, отправляет
native resume и удерживает queue token до ACK. Ошибка resume допускает явный retry.
Пропавший/подменённый владелец отклоняется до вмешательства в чужой output.

Это меняет прежний save gate только для активной owned native connection.
Native replacement/load по-прежнему явно закрыт в publication и prepare;
offline fallback с потерей PCM не допускается. Native idle без активного owned
playback пока также отклоняется. Архив AudioSession теперь реально может содержать
native output, но native restore/published device handoff ещё не реализован.

На действительном default system output через CPAL/CoreAudio (48 kHz stereo f32)
прошли две ignored hardware-проверки: worker pause/freeze/capture/resume и целый
AudioSession source save. Использован только нулевой PCM/gain=0, окна не было.
Session проверяет раннюю отмену до capture, два стабильных захвата, JSON write/read,
сохранение held queue/native pause при потере текущей connection, восстановление
исходного владельца и resume, дальнейшее увеличение playback frames, errors=0
и joined shutdown. Модель/маршрут системного endpoint не записывался; физическая
слышимая непрерывность и host buffer position этим не доказаны.

20 audio-device tests passed (2 ignored), 392 editor tests passed (45 ignored),
оба native tests явно запущены и прошли; 7 offscreen GPU App regressions на
M4 Max/Metal прошли; strict all-target Clippy обоих crates и scoped format/diff
checks прошли. Native окно, полный editor file workflow, native target publication,
reconnect/host-failure recovery и FPS остаются открытыми; ENG-03–13 не закрыты.
[Логи native device/session, границы проверки и хеши](production-readiness-evidence-2026-10-27/native-audio-session-capture/qualification.json).


## Дополнение: подготовленный native AudioSession, 27 октября 2026

Добавлен отдельный PreparedNativeAudioSession: исходная подготовка playback,
clock/catalog/runtime и reload используется повторно, PCM output готовится
отдельным participant. Native entrypoint требует output archive, до открытия
staged workers проверяет sample rate и capacity destination queue. Кандидат
удерживает scene change token, project observation identity и конкретную
connection; повторная destination validation отклоняет изменённые scene/project/
connection. Cleanup использует исходный PreparedAudioSession discard/Drop.
Обычный offline publication API не принимает native candidate: подготовленные
output данные нельзя случайно потерять через offline fallback. При выделении
общих helpers устранена повторная сериализация полного playback JSON.

В реальном native Session-тесте (CPAL/CoreAudio 48 kHz, gain=0, без окна) source
WAV удалён до native preparation; кандидат успешно подготовлен из JSON archive.
Проверены unresolved object references, другой project с тем же root, изменение
scene после staging, fresh prepare и discard с joined staged workers. Held live
AudioSession JSON после этих операций остался идентичным; исходные save/cancel/
resume также работают. Core admission tests отклоняют rate/capacity mismatch и
consumed output, сохраняя текущую очередь.

21 audio-device tests passed (2 ignored), 392 editor tests passed (45 ignored),
реальный native session test явно запущен и прошёл; 7 offscreen GPU App regressions
на M4 Max/Metal, strict all-target Clippy и scoped format/diff checks прошли.
Это подготовка, не native restore completion. Destination validation ещё не
является проверкой native ACK/source epoch. Финальная native publication/load
остаётся закрытой: нужны owned readiness/commit, согласование queue gate с
остальными App owners и реальная проверка восстановления. Native окно, слышимая
непрерывность и FPS не квалифицированы; ENG-03–13 остаются открытыми.
[Логи, границы проверки и хеши](production-readiness-evidence-2026-10-27/prepared-native-audio-session/qualification.json).


## Дополнение: owned native audio publication, 27 октября 2026

Добавлены poll_native_checkpoint_ready/publish_native_checkpoint для исходного
AudioSession. Кандидат привязывается к уникальному source checkpoint epoch и
точному source audio record; scene/project/connection и известные device errors
проверяются повторно. Native API возобновляется при всё ещё held PCM queue;
после ACK выполняется новая проверка source/destination и audio-owner commit
очереди/pump/playback. Старые source workers возвращаются без join в методе.
Очередь после commit остаётся frozen до явного resume enclosing caller: внешний
координатор должен сначала опубликовать остальные уже проверенные владельцы.

Реальный CPAL/CoreAudio Session-тест (48 kHz, gain=0, без окна) дважды восстановил
native output из сохранённого JSON после удаления WAV. Отказ до readiness,
изменение source playback, поздняя scene mutation и старый epoch после cancel/
нового checkpoint отклонены с сохранением source. После успешного commit
native API уже работает, но PCM остаётся удержанным 20 ms; queue/pump record
побитово равен архиву, playback frames восстановлены. Duplicate commit отклонён,
retired workers joined вне метода, после resume playback снова продвигается.

21 audio-device tests passed (2 ignored), 392 editor tests passed (45 ignored),
реальный native session test явно запущен и прошёл; 7 offscreen GPU regressions
на M4 Max/Metal, strict all-target Clippy и scoped format/diff checks прошли.
Это native AudioSession publication, а не завершённая загрузка App: обычные
native file load/publication entrypoints пока закрыты. Требуется связать owned
readiness/commit и окончательное release queue с общей scene/GPU/UI/input
транзакцией. Ошибки остальных владельцев должны проверяться до audio commit;
общего rollback уже committed audio здесь нет. Native окно, слышимая непрерывность,
host playback position, reconnect и FPS не квалифицированы. ENG-03–13 открыты.
[Логи, сценарии и хеши](production-readiness-evidence-2026-10-27/native-audio-owned-publication/qualification.json).


## Дополнение: финальный native audio permit, 27 октября 2026

Core OutputCheckpoint получил owned ReadyOutputPublication: всё admission и
подготовка allocations завершаются до переноса unique queue freeze. Cancel
возвращает gate и пригодный к повтору prepared output без изменения source;
логический publish больше не делает native API calls или проверок admission.

NativeAudioPublicationPermit заимствует session/candidate и неизменные scene/
project до audio commit. Drop до publish возвращает original held source и
кандидат. Publish возвращает CommittedNativeAudio: он сохраняет exclusive session
borrow и queue gate, пока caller меняет остальные уже проверенные host slots.
finish() без Result/native API/poll/wait очищает checkpoint и освобождает PCM.
Legacy audio-only wrapper использует тот же permit через hold; прежняя дублирующая
реализация удалена. После publish нет общего rollback: остальные fallible
проверки обязаны завершиться до commit; drop committed receipt освобождает уже
committed PCM, а не отменяет его.

Core тест с nonzero/negative-zero/subnormal PCM проверил cancel/source identity и
точный commit. Реальный CPAL/CoreAudio 48 kHz/gain=0 Session-тест проверил отмену
финального permit (моделируя отказ другого владельца), идентичный source JSON,
повторную acquisition/commit, удержание PCM 20 ms при уже активном backend,
побитовое совпадение с архивом и изменение настоящего scene owner до finish.
finish очистил source checkpoint/gate без backend resume; retirement joined
отдельно. Прежние epoch/late scene/source guards также прошли.

22 audio-device tests passed (2 ignored), 392 editor tests passed (45 ignored),
реальный native session test явно запущен и прошёл; 7 offscreen GPU App regressions
на M4 Max/Metal, strict all-target Clippy и scoped format/diff checks прошли.
Это готовый commit/release механизм, но полный App native scene/GPU/UI/input load
ещё не подключён, обычный native load gate сохранён. Native окно, слышимая
непрерывность, reconnect, FPS/latency/power и весь ENG-03–13 остаются открытыми.
[Логи, границы проверки и хеши](production-readiness-evidence-2026-10-27/native-audio-commit-permit/qualification.json).


## Сохранение: возобновление игры до завершения записи файла

После получения независимого EditorCheckpoint сериализация, sync и atomic rename
больше не продлевают паузу сцены/звука. Frame-polled Writing сначала возобновляет
исходных владельцев, затем наблюдает worker; незавершённый поток не join-ится.
Capturing, Writing и Resuming удерживают точный epoch своего restore boundary:
поздняя операция не снимает более новый barrier. Ошибка release сохраняет
worker/fence для повторной попытки; capture при смене boundary отклоняется.

Регрессия намеренно блокирует file worker при настоящем running Play с desktop
policy: после восстановления model watcher игра снимает паузу и увеличивает
simulation_ticks до разрешения записи. Отдельная проверка всех трёх save stages
сохраняет новый restore/model barrier. Ошибки capture/write, прежний файл,
точный custom u64 и завершение уже запущенной записи также проверены.
393 editor tests passed (45 ignored), 7 offscreen GPU App tests passed; strict
all-target editor Clippy и scoped rustfmt прошли. В первом варианте running
тест использовал слишком маленькую generic UI policy; исправлен на реальную
desktop policy, исходный отказ сохранён в evidence. Это проверка save workflow,
без native window/FPS или нового native audio device прогона. Полный native
scene/GPU/UI/input load и ENG-03–13 остаются открытыми.
[Проверки, логи и исходный хеш](production-readiness-evidence-2026-10-27/save-resumes-before-file-write/qualification.json).


## Native-аудио подключено к общей загрузке App

Прежний запрет полного App native load для активного, принадлежащего session
аудиовыхода снят: существующий PreparedCoordinatedPlay теперь хранит original
offline либо native candidate. В CPU worker передаётся только клонированный
OutputConnection уже замороженного владельца; сам CPAL stream не переносится
и архив не выбирает/запускает устройство. Форматы файлов и original owners сохранены.

После model/UI GPU validation frame-polled load хранит отменяемую Audio stage.
Device resume ACK проверяется с исходной PCM queue под gate. После source/target
и UI admission native permit публикует audio без fallible операций; committed
receipt удерживает очередь до замены scene/GPU/UI/input/project/history/Play и
освобождает её последней без CPAL вызова, ожидания или повторного admission.
Отмена использует точный original barrier epoch и закрывает candidate workers.
Exclusive async install также ожидает audio readiness.

Новый совместный GPU/native тест реально использует M4 Max/Metal и default
CoreAudio output 48 kHz с gain=0, без окна. Проверены desktop save, отмена
после native resume request, две загрузки после удаления rig.glb/tone.wav,
точное совпадение historical input JSON, animated-model/UI pixels и восстановленных
simulation/audio counters. После каждой загрузки выполнены 120 кадров игры/рендера.
Missing-source watcher diagnostics ожидаемы: valid prior model/UI/audio остаются
активными. Native session permit/cancel/queue/pump/epoch тест отдельно повторён.

393 editor tests passed (46 ignored), 8 offscreen GPU App tests passed,
1 явно запущенный native Session test passed; strict all-target Clippy и scoped
format/diff checks прошли. Это квалификация активного native output при совпадающих
rate/capacity. Native idle, cross-mode loads, reconnect, настоящее окно, слышимая
непрерывность, FPS/latency/power и весь ENG-03–13 остаются открытыми.
[Evidence, логи и исходные хеши](production-readiness-evidence-2026-10-27/native-audio-app-load/qualification.json).


## Native idle и UI lifecycle после Stop

Выбранная Native policy теперь отличается от фактического владельца output.
При отсутствии device/connection/native playback logical checkpoint разрешён
без создания устройства и без изменения policy. Partial ownership и незавершённые
переходы по-прежнему отклоняются. Unsaved output mode включён в source fingerprint,
поэтому смена режима даже при idle отклоняет подготовленный кандидат. Непустой
offline playback не публикуется молча под native policy: требуется настоящее
подготовленное устройство, cross-mode переход остаётся отдельной задачей.

Реальный native Play → Stop → save выявил dangling UI ownership: restore_authoring
менял scene handles, а UiLive и input runtime сохраняли accepted layouts старой
игровой сцены. Первоначальный GPU suite упал с unidentified accepted UI owner.
Play/Stop теперь закрывает original UI workers без join, очищает GPU UI/focus и
создаёт input runtime для нового графа. Обычная отмена фокуса сохраняет своё
прежнее поведение; сброс применяется к смене сцены в Play lifecycle.

Проверены пустой проект до/во время Play без устройства, native-idle GPU
model/history/source-free workflow, прежние mode-change guards и реальный
CoreAudio 48 kHz/gain=0 цикл с двумя source-free loads, продолжениями 120 кадров,
Stop, асинхронным завершением output owner и последующими idle save/read/load.
394 editor tests passed (47 ignored), 9 offscreen GPU App tests passed; строгий
all-target Clippy и scoped format/diff прошли. Исходный Stop/UI отказ сохранён.
Незавершённый device Stop требует обычного polling до admission; reconnect,
cross-mode playback, окно, слышимая непрерывность, FPS и ENG-03–13 не закрыты.
[Логи, ограничения и хеши](production-readiness-evidence-2026-10-27/native-idle-and-stop-ui/qualification.json).


## 28 октября: активный native output → сцена без воспроизведения

Общий native candidate теперь готовит и архив с play=null. Используются прежние
OutputCheckpoint/PcmPump: пустая очередь и полностью потреблённый блок нулей,
без новых форматов, второго runtime, изменения rate или преобразования PCM.
Destination guard требует ровно один active playback либо silent transport.
До commit исходный native owner остаётся неизменным; discard и отмена permit
возвращают исходную очередь. Infallible commit ставит пустую очередь под held
gate, а final receipt после смены всех host owners посылает неблокирующий close
оригинальному device worker. Его завершение/join остаётся у Session polling/
shutdown. Старый active audio-only hold не изменён; новый silent audio-only
путь завершает receipt и закрывает output без дополнительного resume.

Отдельный M4 Max/Metal + CoreAudio 48 kHz/gain=0 GPU тест проверил source-free
active→idle load, отмену на native readiness, сохранение исходных scene token/
audio frames и последующие idle save/load. Прежний native active/Stop сценарий
сохранён отдельной проверкой. Session тест подтвердил exact source JSON после
discard/cancel, пустую очередь до снятия gate, pending pump frames=0, удержание
connection до finish, закрытие/завершение worker и native policy после commit.
394 editor tests passed (48 ignored), 10 offscreen GPU App tests passed,
1 явно запущенный native Session test passed; all-target strict Clippy и scoped
format/diff прошли. Уже переданные host/analog buffers не отменяются задним числом.
Idle→native startup/load, остальные cross-mode переходы, reconnect, окно, FPS
и весь ENG-03–13 остаются открытыми.
[Evidence и исходные хеши](production-readiness-evidence-2026-10-28/native-active-to-idle/qualification.json).


## 28 октября: idle → native archive через общий restore

Нативный архив теперь загружается из idle/no-device сцены. Original reader worker
проверяет вложенный PCM archive до source fences/device startup; на frame thread
остаётся O(1) lookup rate. Отменяемая Device stage через original Session/device
worker открывает временный пустой output, ждёт pause ACK и замораживает queue.
Требуется host Native policy либо native window; файл сам не разрешает output.
Есть 20-second observation timeout; несовпадающая rate отклоняется без resampling.

Исходный checkpoint остаётся logical play=null, provisional output mode защищён
от смены. После original CPU/GPU/UI/native checks final receipt принимает device
вместе со scene/Play/PCM/pump. Отмена или ошибка закрывает только временный output
неблокирующей командой; Session хранит worker до обычного finished polling/join.
Новый runtime, второй каталог, другой формат или PCM conversion не введены.

Реальные M4 Max/Metal + CoreAudio 48 kHz/gain=0 проверки прошли полный
source-free active→idle→native цикл и 120 кадров продолжения, отмену в Device
и Audio stages, повреждённый вложенный PCM до acquisition и source-admission
отказ уже после device ready. Session тест подтвердил rate/mode rejection,
неизменность idle JSON, точный archived queued PCM/pump до gate release и
сохранение принятого output после receipt.
395 editor tests passed (48 ignored), 10 GPU App tests passed, 1 explicitly-run
native Session test passed; strict all-target Clippy и scoped format/diff прошли.
Cancel/timeout cleanup остаётся асинхронным до окончания native API. Пустые
zero-filled callbacks возможны до pause; слышимая/аналоговая непрерывность
не проверена. Nonempty offline/native conversion, active offline→native,
reconnect/rate changes, настоящее окно, FPS и весь ENG-03–13 остаются открытыми.
[Логи, ограничения и хеши](production-readiness-evidence-2026-10-28/idle-to-native-restore/qualification.json).


## 28 октября: offline archive → native output

Original playback decoder теперь принимает offline archive в native prepare при
точном совпадении rate. Mixer/clock/clip publications восстанавливаются прежним
кодеком; новый transport получает пустую queue и consumed pump, потому что
offline PCM уже был потреблён. Shared empty-output builder также используется
прежним silent target. Resampling, обрезание данных и второй audio runtime не
введены. Host Native policy/window выбирает native prepare по playback metadata.

Provisional device startup теперь может удерживать активного offline владельца
под original reload checkpoint. Source capture остаётся offline codec с
исходными ObjectId bindings; новая queue проверяется пустой/quiescent. Cancel
сначала возобновляет original offline reload owner, затем закрывает временное
устройство неблокирующей командой. Это сохраняет возможность повторного save/
load после отмены, а не оставляет старый audio reload fence захваченным.

Новый M4 Max/Metal + CoreAudio 48 kHz/gain=0 сценарий использует настоящий offline
file после удаления model/WAV: отмена в Device/Audio сохраняет runtime/clock/PCM,
переход из active offline создаёт output, повторная загрузка с active native
сохраняет connection identity, а после Stop создаёт новый output. После каждого
из трёх переходов выполнены 120 кадров. Session отдельно подтвердил exact
runtime/clock/raw PCM/imported data и empty queue/pump до release, а также
повторный checkpoint исходного offline owner после cancel.
395 editor tests passed (49 ignored), 11 GPU App tests passed, 1 явно запущенный
native Session test passed; strict all-target Clippy и scoped format/diff прошли.
Native→offline outstanding PCM, reconnect/rate changes, настоящее окно,
слышимая непрерывность, FPS и ENG-03–13 остаются открытыми.
[Логи, границы и исходные хеши](production-readiness-evidence-2026-10-28/offline-to-native-restore/qualification.json).


## 28 октября: native → offline с сохранением ожидающего PCM

Original OutputCheckpoint теперь готовит bounded OfflineOutput без устройства:
canonical queue/pump samples сохраняются как u32 bits, partial consumption
сохраняется прежним binary output codec с точными capacity/block/cursor.
AudioPlay offline сначала потребляет уже смешанный prefix и только затем
рендерит остаток sample budget прежним mixer. Счётчик frames отражает новые
смешанные samples, поэтому prefix, уже учтённый в native mixer cursor, не
смешивается/не считается второй раз. Обычный offline wire без output не изменён.

Host policy/window выбирает реальный sink. Native archive теперь загружается
offline без открытия hardware; если source ещё native, original final permit
очищает прежнюю device queue под gate, публикует logical owner с полным prefix
и закрывает device через receipt последним. Второй mixer/runtime и resampling
не введены. OfflineOutput.render_prefix копирует точные samples, AudioPlay
использует прежний discard sink offline-режима.

Проверены negative-zero/subnormal/nonzero bits и partial binary roundtrip;
бюджет 1050 prefix → 800 consumed/0 mixed → 250 consumed/550 mixed, включая
межпромежуточное сохранение. Реальный M4 Max/Metal + CoreAudio 48 kHz/gain=0
сценарий проверил cancel с сохранением прежнего connection, точный PCM wire,
120 offline кадров с frames=archived+96000-prefix_count, logical reload без
устройства и возврат в native. Session подтвердил весь playback record при
native→logical decode. Прежний отрицательный decoder test заменён точной
positive проверкой новой функции, corrupt-output rejection сохранён.
396 editor tests passed (50 ignored), 23 audio-device tests passed (2 ignored),
12 GPU App tests passed, 1 явно запущенный native Session test passed; строгий
all-target Clippy двух crates и scoped format/diff прошли. Host/analog buffers
и слышимая непрерывность не квалифицированы; reconnect/rate changes, окно,
FPS и весь ENG-03–13 остаются открытыми.
[Evidence, логи и хеши](production-readiness-evidence-2026-10-28/native-to-offline-prefix/qualification.json).
