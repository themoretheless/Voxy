# MCP сцен Voxy

`scene_mcp` — проектный companion существующего `model_mcp`.
Оба используют общий `tools/mcp_stdio.rs`: локальный newline-delimited
JSON-RPC по stdin/stdout, протоколы 2024-11-05, 2025-03-26, 2025-06-18.
stdout содержит только MCP; сообщения рендера идут в stderr.
Существующие 13 инструментов параметрической модели сохранены.

## Запуск и подключение

```sh
cargo build -p voxy_editor --example scene_mcp
target/debug/examples/scene_mcp /absolute/path/to/project.voxy
```

Проектная `.codex/config.toml` подключает `voxy_scene` к сцене AirPods.
`codex mcp get voxy_scene --json` проверяет чтение конфигурации.
После добавления конфигурации перезапустите подключение MCP в клиенте.
Если текущий чат уже зафиксировал инструменты при старте, понадобится новый
сеанс. Сервер не может сам добавить свои инструменты в уже работающий чат.
Для другого проекта поменяйте единственный аргумент запуска сервера.

Пример клиентской конфигурации:

```toml
[mcp_servers.voxy_scene]
command = "/absolute/path/to/voxy/target/debug/examples/scene_mcp"
args = ["/absolute/path/to/project.voxy"]
startup_timeout_sec = 20
tool_timeout_sec = 90
```

Конфигурация stdio и проектная область:
https://learn.chatgpt.com/docs/extend/mcp .
Протокол и inline PNG:
https://modelcontextprotocol.io/specification/2025-06-18/basic/transports ,
https://modelcontextprotocol.io/specification/2025-06-18/server/tools .

## Команды

| Инструмент | Действие |
| --- | --- |
| `scene_get` | Сцена, монотонная revision, BLAKE3 sceneHash, dirty |
| `scene_apply` | Атомарный пакет 1..128 правок |
| `scene_undo` | Отменить один пакет |
| `scene_redo` | Повторить один пакет |
| `scene_save` | Проверить внешние изменения и атомарно сохранить startup scene |
| `scene_render` | Вернуть настоящий PNG Voxy размером 1024×1024 прямо в MCP |

Все команды, кроме `scene_get`, принимают `expected_revision` из последнего
ответа. Каждая фактическая правка/undo/redo повышает revision, включая возврат
к прежнему содержимому. Устаревшая revision отклоняется до изменения состояния.
`scene_save` и `scene_render` revision не меняют.

Пример аргументов `scene_apply`:

```json
{
  "expected_revision": 0,
  "edits": [
    {"op":"transform","node_id":"right-earbud","translation":[0.041,0.08,0]},
    {"op":"material","node_id":"case-body","tint":[0.18,0.35,0.9,1],"lit":true},
    {"op":"camera","node_id":"reconstruction-camera","target":[0,0.046,0],"distance":0.16,"yaw":0.75,"pitch":0.23,"perspective":true}
  ]
}
```

Остальные операции: `create` с полным нативным `SceneObject`, `remove` с
`node_id` (удаляет поддерево), `reparent` с `node_id` и `parent` (ID либо null),
`active` с `node_id` и boolean `active`. Transform может менять translation,
rotation XYZW и scale по отдельности. Размеры в метрах; повороты камеры
в радианах. Материал — непрозрачный RGB tint в [0,1], alpha строго 1,
`lit` включает освещение Lambert. Roughness/metalness этот API не обещает.

Один невалидный элемент отклоняет весь пакет. Нативные `SceneDocument`,
`SceneGraph` и `SceneHistory` проверяют иерархию, компоненты и трансформации.
Дополнительно проверяются композиция мировых матриц, ссылки на модели,
диапазоны цветов и камеры. История ограничена 32 версиями и 64 MiB.

## Владение сценой и сохранение

Сервер держит самостоятельную сессию авторинга в памяти. Изменения сразу
влияют на следующий `scene_render`, в том числе до сохранения. Они не являются
командами уже открытому окну редактора. `liveEditorApplied` всегда null.
Для загрузки сохранённых изменений в отдельном редакторе переоткройте проект.
Открытый редактор с несохранённой сценой автоматически не синхронизируется.

Выход из сервера теряет несохранённые правки: перед завершением вызовите
`scene_save` и проверьте `dirty=false`. `scene_save` пишет только startup scene
выбранного проекта. Prefab-композиции отклоняются, чтобы не сохранить
развёрнутую сцену поверх исходной композиции. Изменения descriptor/manifest
требуют переоткрытия сервера. Внешняя правка scene-файла блокирует сохранение.
Проверка является обнаружением наблюдаемых внешних изменений, не межпроцессным
CAS: конкурирующие писатели в один файл не поддерживаются. Атомарное сохранение
использует существующий `save_scene_file`, включая его отчёт `committed=true`
при ошибке проверки durability после rename.

## Проверяемый рендер

Рендер использует `SceneRenderer` и `voxy_editor/src/material.wgsl` — общий
код с `procedural_preview`. Поддерживаются локальные OBJ, иерархия активности,
трансформации, opaque tint, lit/unlit и перспективная/ортографическая камера.
Требуется ровно одна активная камера без legacy-режима. Другие компоненты
или форматы рендера явно отклоняются; glTF, skinning, fog, UI, PBR, анимации и
полноценный кадр игрового runtime этим путём пока не поддержаны.

Для каждого кадра создаётся уникальный временный каталог со своей копией сцены,
мешей, manifest и descriptor. С этого момента рендер не перечитывает исходные
файлы проекта. MCP-ответ содержит inline PNG, локальный путь, revision,
sceneHash и meshHashes. В каталоге сохранены PNG, сцена и receipt. Это offscreen
кадр, не снимок окна/монитора. Снимки сохраняются до явной уборки пользователем.
Один запуск обслуживает один проект; сервер не открывает сетевых портов.

Лимиты: 2 MiB на входной MCP-запрос, 8 MiB на документ/OBJ, 512 узлов,
64 MiB суммарных мешей одного снимка. Обработка синхронная; отмена уже
выполняющегося GPU-рендера не реализована. Неполный каталог не считается
успешным снимком.

## Проверка

```sh
cargo test -p voxy_editor --example scene_mcp
cargo test -p voxy_app --bin model_mcp
cargo clippy -p voxy_editor --example scene_mcp --example procedural_preview --no-deps -- -D warnings
python3 tools/test_scene_mcp.py --output /tmp/voxy-mcp-new-evidence
```

Испытание 2026-10-27 на Apple M4 Max / Metal:
`scene-mcp-2026-10-27/summary.json` и `transcript.json`.
Пакет правок изменил цвет корпуса, положение наушника и камеру. Полученный
PNG изменился. Undo восстановил исходный PNG байт-в-байт; redo восстановил
изменённый PNG. Проверены создание узла, reparent, удаление поддерева,
сохранение, отклонение невалидного пакета и защита наблюдаемой внешней правки.
Использовалась отдельная копия проекта; исходная модель AirPods не изменена.
