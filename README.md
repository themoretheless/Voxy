# Voxy

Voxy — воксельный движок на Rust с нативным desktop-клиентом, Tamagotchi и примером пинбола.

Cargo-workspace повторяет границы архитектуры: `voxy_core` содержит координатные и временные контракты, `voxy_world` — registry блоков, palette chunks, детерминированные проходимые горы и атомарные edit-транзакции, `voxy_mesher` — naive oracle и production greedy meshing, `voxy_render` — `wgpu` surface lifecycle, ортографическую камеру, skeletal GPU skinning, 32-уровневый чёрно-серый LCD shader и alpha blending, `physics` — независимую кинематическую физику и плоские контакты круга с капсулами, `physics_voxel` — её воксельный адаптер, воду и разрушения, а `voxy_runtime` — fixed-step simulation и renderer-neutral bootstrap сцены.

`voxy_app` реализует игровой Tamagotchi-цикл: объёмный питомец ходит и прыгает по миру, стареет, испытывает голод, теряет энергию, настроение, здоровье и чистоту, спит и реагирует на уход. Окно движка остаётся скрытым до готовности мира и GPU-ресурсов; после запуска в заголовке показываются состояние питомца и измеренное время загрузки.

## Общий 2D/3D renderer (в разработке)

Помимо вокселей, `voxy_render` предоставляет индексированные меши, RGBA-текстуры,
перспективную и ортографическую камеры, обновляемые MVP-матрицы и 2D overlay.
Проверка реальных GPU-пикселей, глубины и прозрачности:

```sh
cargo run -p voxy_app --example scene_demo
cargo run -p voxy_app --example scene_demo -- --smoke
cargo run -p voxy_render --example scene_smoke
cargo run -p voxy_render --example scene_smoke -- --shader-test
cargo run -p voxy_render --example compute_smoke
cargo run -p voxy_render --example gpu_probe -- metal
```

Целевые платформы: Windows, Linux, macOS, iOS, Android, браузер и VR (OpenXR,
включая Meta Quest). Мобильные и VR-пути ещё требуют реализации и проверки.
Состояние backend’ов и оставшиеся этапы — [docs/engine-development.md](docs/engine-development.md).

## Браузер: WebGPU / WebGL2

Нужны Rust target `wasm32-unknown-unknown` и `wasm-bindgen-cli` версии `0.2.127`.

```sh
web/build.sh
python3 -m http.server 8766 --bind 127.0.0.1 --directory web
```

Открыть `http://127.0.0.1:8766/`: анимированная 3D-сцена и 2D overlay.
Ссылки переключают WebGPU / WebGL2, `Space` ставит анимацию на паузу.
`?backend=webgpu&voxel=1` открывает voxel-мир с генерацией всех 27 чанков на GPU,
персонажем, автомобилем и разрушением земли. `WASD` управляет движением,
`J` — прыжком, `B` — тормозом; кнопка Enter vehicle переключает режим.
Для машины также доступны экранные газ, руль и тормоз.
`?backend=webgpu&voxel=1&worldCheck=1` проверяет все 884736 блоков сохранённого мира
против CPU-генератора. WebGL2 использует CPU-генерацию и физику с GPU-рендером.
В WebGPU поиск препятствий выполняется на GPU, точные контакты — на CPU;
meshing и lighting также остаются CPU. Вода в браузере пока имеет отдельную
GPU-проверку, но не подключена к игровому миру.
Оба backend’а проверены в локальном браузере; мобильная матрица ещё не проверена.

## Пример: пинбол

```sh
cargo run -p voxy_app --example pinball
```

Объёмный стол использует существующий `voxy_render` (wgpu, ортографическая камера,
32 градации серого). Физика шара работает в плоскости: гравитация вдоль стола,
упругие столкновения с округлыми бортами, вращающиеся лопатки с передачей скорости,
три активных бампера по 100 очков. Три шара на игру, счёт и остаток шаров на табло.

- `A` / `←` — левая лопатка; `L` / `→` — правая.
- Удерживать `Space` для заряда, отпустить для запуска. Заряд виден в заголовке окна.
- `P` — пауза; `R` — новая игра; `Esc` — выход.
- Потеря фокуса ставит игру на паузу; `P` продолжает игру.

```sh
cargo test -p voxy_app --example pinball
cargo run -p voxy_app --example pinball -- --smoke
```

`--smoke` выполняет 10 секунд симуляции с автоматическим вводом, проверяет показ
кадров и попадания в бамперы, затем завершает процесс. Нужен доступ к нативному
окну и GPU. Подробности и ограничения решателя — [docs/physics.md](docs/physics.md).

## Управление Tamagotchi

- `WASD` / стрелки — движение;
- `Space` — прыжок;
- мышь — поворот камеры, колесо — масштаб, `Esc` — освободить курсор;
- `E` — кормить, `P` — играть, `Z` — сон/пробуждение;
- `H` — лечить, `C` — убрать;
- `R` — переключить режим транспорта.

- [Архитектура](docs/architecture.md)
- [ADR 0001: фундамент движка](docs/adr/0001-engine-foundation.md)

Проверка текущего инкремента:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p voxy_app
```

Главное раннее решение: V1 — это **block voxels**, а не smooth/SDF terrain. Если нужен Dual Contouring или Marching Cubes, это следует решить до реализации `voxy_world`: формат выборки мира и требования к соседям будут другими, хотя runtime, streaming и большая часть renderer architecture сохранятся.

## Движение снарядов на CUDA

`cargo run -p voxy_app --features cuda -- --cuda-projectiles --cuda-device 1`
подключает пакетную интеграцию движения снарядов к основному приложению.
Кернел использует `f64` и тот же semi-implicit Euler, что CPU-путь; проверки
столкновений и целочисленные якоря координат остаются в воксельной физике.
Флаг можно сочетать с `--gpu-terrain` или `--cuda-terrain`; устройство общее.
Ошибки CUDA завершают запуск/работу с ошибкой, без автоматического CPU fallback.
Аппаратный probe: `cargo run -p voxy_cuda --features cuda --example projectile_probe -- 1`.
На NVIDIA этот путь пока не выполнен. `--cuda-character-motion` также переносит
интеграцию ускорения и перемещения персонажа на тот же f64-кернел; управление
прыжком и ограничение падения остаются CPU. По умолчанию столкновения и ступеньки
рассчитываются на CPU; `--gpu-collisions` включает асинхронный GPU-поиск препятствий
с точными CPU-контактами, а `--cuda-collisions` — CUDA-поиск и f64-контакты.
Оба флага CUDA-движения персонажа и снарядов можно включить вместе. `--cuda-vehicle-motion` использует тот же CUDA-расчёт
для шасси автомобиля; двигатель, торможение и рулевое управление остаются CPU.
Автомобиль также поддерживает `--gpu-collisions` и `--cuda-collisions`.
`--gpu-water` включает GPU-переносы воды с проверкой версий перед commit в мир;
без этого флага используется CPU-путь.

`cargo run -p voxy_app --features cuda --example cuda_character_probe -- 1`
проверяет 240 шагов CUDA-движения персонажа против CPU, включая воксельные
контакты, приземление и прыжки. Этот probe включён в аппаратные Linux/Windows
проверки; выполнение на NVIDIA ещё не подтверждено.
`cargo run -p voxy_app --features cuda --example cuda_vehicle_probe -- 1`
аналогично проверяет 240 шагов автомобиля против CPU; он включён в те же проверки.

## Генерация мира на GPU / CUDA

Основное voxel-приложение принимает `--backend auto|metal|dx12|vulkan|gl`.
Например: `cargo run -p voxy_app -- --backend metal --gpu-terrain`.
Политика применяется к окну и compute-генератору; `--fallback` запрашивает
резервный адаптер явно. Ошибка выбранного backend завершает запуск с ошибкой.

`cargo run -p voxy_app -- --gpu-terrain` переносит хеширование, шум, биомы и
заполнение чанков на compute-шейдер. Результат сохраняет алгоритм CPU-генератора,
все биты seed и точность координат `i64`. Metal проверен на Apple M4 Max.

На NVIDIA: `cargo run -p voxy_app --features cuda -- --cuda-terrain` выбирает
CUDA-устройство 0 и требует драйвер и NVRTC. Ошибки возвращаются без автоматического
перехода на CPU; выполнение CUDA на NVIDIA пока не проверено.
Для другого устройства добавьте `--cuda-device N`, например
`cargo run -p voxy_app --features cuda -- --cuda-terrain --cuda-device 1`.
Отсутствующий, отрицательный, повторный или слишком большой номер отклоняется
до создания окна; `--cuda-device` требует `--cuda-terrain`.
На Linux с NVIDIA проверки CUDA запускаются командой
`sh tools/cuda/hardware-acceptance.sh 1`: буферы, чанки, гравитация и Vulkan↔CUDA.
Нужны CUDA-драйвер, NVRTC, Vulkan и совместимое устройство; логи сохраняются в
`target/cuda-hardware`. Скрипт останавливается на первой ошибке.
На Windows: `powershell -File tools/cuda/hardware-acceptance.ps1 -DeviceOrdinal 0`.
Проверяет CUDA и DX12, включая шейдерные пиксели и окно; логи —
`target/cuda-hardware-windows`. Требуется доступный рабочий стол.
Он также проверяет пиксели wgpu после прямой записи CUDA в Vulkan-память:
`cargo run -p voxy_vulkan --features cuda --example cuda_gravity_render -- 1`.
Этот путь реализован, но его выполнение на NVIDIA ещё не подтверждено.
Vulkan-экспорт и этот пример также собираются для Windows с Win32-дескрипторами;
аппаратный запуск Windows ещё не проверен.
Для shared committed-буфера DirectX 12 добавлен отдельный unsafe-импорт
`CudaCompute::import_d3d12_resource_u32` с CUDA dedicated-флагом. Экспорт из
D3D12/wgpu реализован через `D3d12ExportBuffer`; его обнуление и NT-handle
проверяет Windows-пример `cargo run -p voxy_vulkan --example d3d12_export`.
Синхронизированная публикация CUDA-физики реализована через
`CudaGravityD3d12Graphics`. Для проверки прямой CUDA-записи:
`cargo run -p voxy_vulkan --features cuda --example d3d12_export -- --cuda-device 0`.
Этот DX12/CUDA путь собран для Windows, но аппаратный запуск ещё не проверен.
Пиксели после CUDA-физики через DX12 проверяет
`cargo run -p voxy_vulkan --features cuda --example cuda_gravity_render -- 0 --dx12`.
Адаптер выбирается по LUID CUDA-устройства; проверяются три кадра и 64 шага.
Оконная CUDA-орбита: `cargo run -p voxy_vulkan --features cuda --example cuda_gravity_window -- --cuda-device 1`.
Space — пауза, R — сброс, Escape — выход; `--smoke` проверяет оконный цикл.
На Windows добавьте `--dx12` для общего D3D12-буфера; CUDA-адаптер
выбирается по LUID. Аппаратный DX12/CUDA запуск пока не подтверждён.
Явный `--export-only` проверяет Vulkan-окно с WGSL-физикой, без CUDA-вычислений.
Проверки и ограничения описаны в [генерации мира](docs/procedural-terrain.md).


GPU-физика с рисованием из общего буфера устройства:

```sh
cargo run -p voxy_app --example gpu_gravity
cargo run -p voxy_app --example gpu_gravity -- --smoke
cargo run -p voxy_gpu --example gravity_render -- metal
cargo run -p voxy_gpu --example gravity_smoke -- metal
```

Space — пауза, R — сброс орбиты. CUDA f64-решатель проверяется отдельно:
`cargo run -p voxy_cuda --features cuda --example gravity_probe` требует NVIDIA.

Для отдельной проверки API: `cargo run -p voxy_app --example gpu_gravity --
--smoke --backend metal` (также `vulkan`, `dx12`, `gl`). Явный выбор не допускает
перехода на другой API; `--fallback` запрашивает fallback-адаптер выбранного API.

Linux/Vulkan-проверки окна, численной точности и пикселей:
`sh tools/linux/gpu-gravity-smoke.sh`. Docker-образ `voxy-linux-smoke:latest`
с Mesa llvmpipe проверяет программный Vulkan; это не аппаратный GPU-бенчмарк.
`gravity_smoke` и `gravity_render` принимают один аргумент
`auto|metal|vulkan|dx12|gl`; явный API не заменяется другим.
`sh tools/linux/opengl-smoke.sh` проверяет OpenGL-сцену, замену шейдеров,
окно физики, траектории/орбиту, пиксели физики и точное совпадение чанков с CPU.
Оба Linux-сценария проверяют backend в логах; результаты Mesa — программное исполнение.

Браузерная GPU-орбита: `?backend=webgpu&gravity=1` или ссылка **GPU gravity**.
Space останавливает физику; WebGL явно сообщает отсутствие compute.

Численная проверка WebGPU против CPU f64: `?backend=webgpu&gravityCheck=1`
(257 тел, 128 шагов, ошибка и результат отображаются в статусе страницы).

Обхват кистями: `cargo run -p voxy_app --example female_grasp -- cylinder`.
Камера следует за кистью; клавиши **1**, **2**, **3** выбирают цилиндр, шар и
ручку с утолщениями. **[** и **]** открывают и закрывают кисть на 10 процентных
пунктов, переводя её в ручной режим; **G** возвращает цикл, **Space** ставит
воспроизведение на паузу. Степень закрытия показана в заголовке окна.
Для собственного замкнутого OBJ передайте путь вместо
`cylinder`; модель центрируется у ладони и масштабируется до 75 мм.
Пальцы имеют по три сустава, большой палец использует оппозицию. Боковое
сведение пальцев подстраивается под поверхность предмета. Суставы
останавливаются по контакту с предметом; это кинематическая подгонка,
без расчёта удерживающей силы и трения. OBJ должен иметь согласованную
наружную ориентацию граней; для стенок замкнутой полости нормали направлены
в полость. Загрузчик проверяет замкнутость, вырожденные грани и неманифолдные
соединения, учитывая дублирование вершин на UV-швах. Крупный план со стороны ладони:
`cargo run -p voxy_app --example female_render -- --hands --grasp --palm --sequence`.
Флаг `--grasp-object sphere` или `--grasp-object handle` меняет предмет;
также можно указать путь к OBJ. Кадры сохраняются в `/tmp/voxy-finger-frames`.
Для проверки деформации пальцев без предмета используйте
`cargo run -p voxy_app --example female_render -- --hands --grasp-free --palm --bare`.
Этот режим фиксирует кисть в полностью согнутой позе и сохраняет изображение
в `/tmp/voxy-finger-preview.png`.
Для отдельного кадра с заданной степенью закрытия добавьте
`--grasp-amount 1 --snapshot /tmp/hand-closed.png` к команде с `--hands --grasp`.
Именованный снимок позволяет сравнивать варианты, не перезаписывая общий preview.


GPU-проход воды: `cargo run -p voxy_gpu --example water_smoke -- metal`
(также `vulkan`, `dx12`, `gl`). Целочисленный шейдер сохраняет порядок переносов
CPU-солвера; пример проверяет 16 тиков и 524288 клеток, объём и ошибки бюджетов.
`sh tools/linux/water-smoke.sh` проверяет Vulkan/OpenGL через Mesa llvmpipe.
Основная игра использует этот путь с `cargo run -p voxy_app -- --gpu-water`.
Без флага остаётся CPU-планировщик. GPU захватывает неизменяемые снимки чанков
и формирует транзакции с проверкой ревизий; конвейер использует устройство рендера.
Основная игра отправляет GPU-шаг без ожидания и получает результат в последующих
обновлениях; при конфликте ревизий возвращает исходные ячейки в очередь.
Шейдер сохраняет последовательный порядок CPU-переносов.
Графический автопилот с GPU прошёл на Metal: 7 шагов, 5 водных транзакций
и 119 показанных кадров. Он разрушает существующий блок под естественной
водой, чтобы проверить повторную активацию после изменения мира.
`sh tools/linux/gameplay-water-smoke.sh` запускает тот же игровой сценарий
с явными Vulkan/OpenGL через Mesa; физические GPU требуют отдельной проверки.

Native character movement can use nonblocking GPU collision broadphase with `cargo run -p voxy_app --bin voxy_app -- --gpu-collisions --gpu-water`. Exact contact resolution remains on the CPU; stale world data restarts the pending character tick. Combine it with `--cuda-character-motion` on NVIDIA to retain CUDA-integrated motion during asynchronous collision queries.

On NVIDIA, `cargo run -p voxy_app --features cuda --bin voxy_app -- --cuda-collisions --cuda-character-motion --cuda-device 0` selects CUDA voxel broadphase and f64 box contacts. This path synchronizes CUDA readbacks; world sampling and contact reduction remain on the CPU. It rejects missing drivers and cannot be combined with `--gpu-collisions`. Native tests and Windows cross-compilation pass; physical NVIDIA execution remains unverified. Linux/Vulkan and Windows/DX12 hardware acceptance scripts include a separate CUDA collision gameplay gate.

Browser WebGPU collision validation (`?backend=webgpu&collisionCheck=1`) now compares four continuous sweeps, 240 asynchronous character ticks and 240 vehicle ticks against CPU states and contact reports, including consumed-task rejection and stale-world recovery (485 reported checks). This passed in the in-app browser; see `docs/browser-character-proof.png`. The validation uses a separate fixture world. The interactive voxel entrypoint is `?backend=webgpu&voxel=1`, with character/vehicle movement and world edits; WebGPU voxel gameplay now includes `Pour water`, asynchronous GPU transfer steps, mesh updates after committed world transactions, and wake-up after destruction. WebGL gameplay does not expose compute water. Activation scans the center chunk; dormant water outside it still needs wider wake-up support. Missing transfer destinations now trigger nonblocking GPU terrain generation and a fresh water snapshot; unavailable data is rejected. `?backend=webgpu&voxel=1&streamingCheck=1` verifies the actual browser loader on a cloned boundary fixture against 65536 CPU-reference cells. Additional retained chunks are capped at 64; loaded and edited chunks are published in the draw mesh. `?backend=webgpu&voxel=1&waterBoundary=1` demonstrates this in the actual rendered world (a diagnostic seeded water/floor edit at x=63, GPU neighbor loading, three visible chunks and continued gameplay). The camera frames the visible chunk set. Eviction and broader dormant-water wake-up remain unfinished.

The browser collision gate also edits a captured chunk after the character has completed its first contact query. It verifies stale-result rejection, consumed-task rejection and exact CPU state/contact parity after a fresh retry (245 checks total).

### Live model viewport

Open a bounded OBJ resource in the main application with background content polling
and imports:

```sh
cargo run -p voxy_app -- --model /absolute/path/model.obj
cargo run -p voxy_app -- --model-manifest /absolute/path/assets.json logical-id
```

Manifest paths resolve relative to the manifest directory. Failed reloads keep the
previous geometry visible. Rename a source by updating its manifest binding while
keeping the logical ID. The native viewport lives in `voxy_editor`; the existing
`voxy_render` asset_window example calls the same implementation. This opens a
model viewport mode; it does not instantiate imported models in the terrain world.

In the native model viewport, press **D** to duplicate the selected model,
**Tab** to select the next instance, and **arrow keys** to move it. Instances
share loaded geometry and keep independent transforms through resource reload.
The viewport currently supports up to 128 instances.
Press **Z** to undo a viewport edit and **Y** to redo it. Undo includes
model duplication and movement; resource reload stays independent of history.
Click a model with the **left mouse button** to select its instance. Selection
uses the current model's local bounding box, transformed into the viewport.
**Delete** or **Backspace** removes the selected instance, including the last
one. Undo restores it; **D** adds a model again when the viewport is empty.

The authoring viewport can also run as its own application:

```sh
cargo run -p voxy_editor -- model.obj
cargo run -p voxy_editor -- --manifest assets.json logical-id
```

This entry point uses the editor's production dependencies and does not build
the renderer examples' physics dev-dependencies.
Hold the **left mouse button** on a model and drag to translate it in the
viewport plane. Releasing commits one undo step; **Escape**, focus loss, cursor
exit or window resize cancels the gesture.

Persist native viewport instances with a scene file:

```sh
cargo run -p voxy_editor -- --manifest assets.json logical-id --scene workspace.scene.json
```

An existing scene loads before the window opens; a new path becomes the save
location. **F5** saves and **F9** reloads. The file retains object IDs, names,
activity, translation, rotation, scale and logical model references. Use the same
logical model when reopening; this viewport currently supports one resource per
scene. Loading during editing creates an undo step; opening a scene starts a
fresh history. Invalid file loads retain the current scene.
Selected models now have an amber bounds outline. Drag the **red X** or **green Y**
handle to constrain translation; drag the **blue center** vertically to change
depth. Dragging the model away from the handles moves it in the viewport plane.

Проверка диагоналей voxel-квадов: `cargo run -p voxy_render --example voxel_diagonal -- metal`
(также `vulkan`, `gl`, `dx12`). Она сравнивает 128 пикселей AO с аналитической
интерполяцией через диагностический fragment stage. Проверена на Apple M4 Max Metal
и Mesa llvmpipe Vulkan/OpenGL; NVIDIA-проверки требуют `--require-nvidia`.
Компиляция для Windows не подтверждает аппаратное выполнение DX12.

The native model editor now shows a scrollable scene tree and TRS inspector.
Click a numeric value, type a number, Enter to apply, Escape to cancel. In a
manifest project, click the model row (or R) to cycle the selected object's
logical resource; D duplicates its current resource. Click Parent/detach, then
choose a parent in the tree. Active toggles inherited visibility; Delete removes
a subtree. F6/Play previews a separate rotating runtime scene; Stop restores the
authoring scene. Save/load (`--scene PATH`, F5/F9) includes hierarchy and each
object's model reference. The native panels currently require Arial (macOS/Windows)
or DejaVu Sans (Linux); the demo uses an identity camera and bounded OBJ imports.

Runnable two-resource editor fixture:

```sh
cargo run -p voxy_editor -- --manifest crates/voxy_editor/examples/assets/models.json square --scene crates/voxy_editor/examples/assets/two-models.scene.json
```

F5 saves edits to this fixture; copy its scene file first if you want to keep the
initial layout.

Scene simulation uses the shared `voxy_time` clock through `SceneSimulation`,
with explicit behavior/command/catch-up limits. Native Play runs fixed 60 Hz ticks
and cleans up behaviors on Stop/window close. A backend-independent integration
example is available with `cargo run -p voxy_scene --example fixed_scene`.

CUDA water is selectable with `cargo run -p voxy_app --features cuda --bin voxy_app -- --cuda-water --cuda-device 0`. It shares the selected CUDA context with motion/collision, uses synchronous ordered graph transfers and publishes through the existing revision-checked world plans. `--cuda-water` and `--gpu-water` are mutually exclusive. CPU world storage, snapshot construction and mesh generation remain; this is not GPU-resident water/world state. Compilation and option tests pass; physical NVIDIA execution and gameplay acceptance remain unverified.
