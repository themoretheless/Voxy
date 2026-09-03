# ADR 0001: фундамент Voxy

- Статус: принято для reference profile V1
- Дата: 2026-09-02
- Область: greenfield-архитектура

## Контекст

Репозиторий не содержит legacy-кода и совместимых форматов. Требуется архитектура Rust-движка, которая даёт быстрый проверяемый вертикальный срез и не закрывает путь к большому редактируемому миру, headless runtime и сетевой репликации.

До появления продуктовой спецификации принят такой reference profile:

- блочные осево-ориентированные воксели;
- нативные macOS, Windows и Linux;
- процедурный редактируемый мир с сохранением;
- single-player сначала, server-authoritative multiplayer позже;
- целевая плавность клиента — 60 кадров/с, но это критерий будущих измерений, а не текущая гарантия.

## Решения

1. **Rust workspace вместо монолитного crate.** Границы crates следуют потокам владения: core, world, storage, mesher, runtime, render и desktop app.
2. **`wgpu` + `winit` для desktop shell.** Прямые Vulkan/Metal/D3D12 backends и собственный render graph не входят в V1.
3. **Один authoritative writer.** Мир и simulation изменяются только на tick-thread. CPU workers получают immutable snapshots и возвращают результаты с версиями.
4. **Chunk edge равен 32 в формате V1.** Глобальные voxel и chunk coordinates — `i64`, local coordinates — `u8`. Деление отрицательных координат только евклидово, а chunk-to-voxel arithmetic всегда checked.
5. **Palette-compressed chunks.** Каноническое содержимое — `Uniform`, bit-packed local palette или direct `u32`; mesh, lighting и GPU handles являются производными данными. Persistent world IDs отделены от плотных runtime IDs.
6. **CPU greedy meshing сначала.** Mesher выдаёт quads по слоям opaque/cutout/translucent. GPU meshing, multi-draw, Hi-Z и LOD добавляются только по профилю.
7. **Bounded pipelines.** Generation, meshing, disk I/O, completion apply и GPU upload имеют отдельные очереди, лимиты in-flight и memory budgets.
8. **WAL + append-only region checkpoints.** Отдельный I/O actor — единственный владелец файлов. Committed multi-chunk edits попадают в checksum-protected journal transactions; regions являются отстающими checkpoints, а compaction использует recoverable replacement.
9. **Fixed-step simulation.** Клиент симулирует с фиксированным tick и интерполирует render state. Voxels/chunks не являются ECS entities; ECS требует отдельного ADR после появления реальных entity-query workloads.
10. **Никаких native dynamic plugins в V1.** Сначала data packs. Если потребуется исполняемый modding, предпочтителен versioned sandboxed WebAssembly host API.

## Обязательные инварианты

- `Unloaded` никогда не превращается в `Air` неявно. Render, collision и raycast выбирают политику явно.
- Любой background result содержит ticket и source versions; устаревший результат можно только отбросить.
- Commit edit transaction атомарен на уровне затронутых chunks и строго увеличивает их revisions.
- Journal/checkpoint acknowledgment относится к конкретной revision; более новая dirty revision после него остаётся dirty.
- Chunk с неподтверждённой journal revision нельзя evict; committed WAL replay всегда идемпотентен.
- Renderer не читает mutable world и не публикует GPU objects за пределы `voxy_render`.
- Ни одна рабочая очередь не может расти без верхней границы.
- Disk/wire DTO не является дампом Rust struct и не зависит от ABI или `serde` layout внутренних типов.
- Преобразование глобальных координат в `f32` происходит только после camera-relative rebasing.

## Последствия

Плюсы:

- гонки world mutation исключаются моделью владения;
- generation и meshing становятся pure, детерминированными и хорошо тестируются;
- headless runtime не зависит от window/GPU;
- backpressure является частью API, а не аварийной оптимизацией;
- смена backend-реализации storage или mesher не затрагивает application shell.

Цена:

- snapshots и version stamps требуют явной bookkeeping;
- устаревшая работа иногда будет вычислена и отброшена;
- CPU meshing и per-chunk draws раньше упрутся в предел, чем полностью GPU-driven pipeline;
- WAL и append-only checkpoints требуют recovery/fault-injection tests, compaction и явных durability watermarks;
- limited translucent path V1 имеет визуальные ограничения.

## Триггеры пересмотра

- Требуется smooth/SDF terrain: пересмотреть sample type, chunk halo, mesher и collision до начала world implementation.
- Профиль показывает CPU meshing bottleneck: рассмотреть compute meshing, сохранив stamped job contract.
- Draw submission становится bottleneck: добавить indirect/multi-draw по negotiated GPU capabilities.
- Нужны тысячи динамических actors и сложные component queries: выбрать ECS отдельным ADR.
- Нужен недоверенный исполняемый modding: определить sandbox, capability model и versioned host ABI.
- Нужен competitive deterministic lockstep: текущая server-authoritative модель и floating-point physics недостаточны.

Полная модель модулей, API, потоков и этапов реализации находится в [архитектурном документе](../architecture.md).
