# Native frame profiling

[Актуальный план на 21 октября 2026](#актуальный-план--21-октября-2026).

Both native clients accept `--profile PATH.csv`, `--profile-scenario NAME`,
`--profile-warmup SECONDS` (default 10), and `--profile-seconds SECONDS` (default 60).
The main app supports `idle`, `motion`, and `water`; `street_walk` supports `street`.
Profiling uses a physical 1920×1080 window and automatic fixed-tick input. Normal
launch settings are unchanged. Water releases a natural support block at tick 720
and fires periodically. The street follows a repeating 20-second forward/back path
with its fixed default seed; this is traversal of the resident scene, **not an asset
streaming benchmark**. It freezes the day cycle. Keep quality settings/environment
identical between versions.

```sh
cargo build --release -p voxy_app --bin voxy_app --example street_walk
target/release/voxy_app --backend metal --present-mode immediate --profile /tmp/idle.csv --profile-scenario idle
target/release/voxy_app --backend metal --present-mode immediate --profile /tmp/motion.csv --profile-scenario motion
target/release/voxy_app --backend metal --present-mode immediate --profile /tmp/water.csv --profile-scenario water
target/release/examples/street_walk --profile /tmp/street.csv --profile-scenario street
```

Reports contain attempted-frame intervals, CPU timings, surface acquisition,
presentation/skip observations and nearest-rank p50/p95/p99. A JSON summary sits
beside the CSV. A separate `.gpu.csv` records asynchronously collected GPU samples;
these belong to earlier submissions, not the CPU frame collecting them. Main-app
GPU measurements cover the primary voxel pass only. Street measurements cover the
existing scene timestamp span plus opaque/transparent passes; external postprocessing
is excluded. Unsupported metrics remain empty/null. Upload byte counts, peak memory,
full GPU frame time, display scanout and actual missed refreshes are not measured.
Main-app simulation time includes pose publication and its GPU upload calls.

Warmup starts after initialization; intervals crossing its boundary are excluded.
Closing the window early fails the profiling run. Surface skips are included and
prevent qualification. Keep the window visible, disable capture/validation modes,
and do not run benchmarks concurrently. A software adapter cannot qualify hardware
performance. `frame_budget_met` only describes measured CPU frame intervals; it is
not complete 120 FPS qualification.

## Paired comparisons

Save separate instrumented executables for both versions, then run:

```sh
python3 tools/performance/paired_frames.py --before /tmp/before/voxy_app --after /tmp/after/voxy_app --street-before /tmp/before/street_walk --street-after /tmp/after/street_walk --focus water --output /tmp/voxy-pairs
```

The runner executes five alternating before/after pairs for each scenario, hashes
executables, retains raw reports/logs, and compares median run p99. Acceptance needs
at least 5% improvement in the chosen scenario and no regression above 3% elsewhere.
The complete matrix takes about 47 minutes plus initialization. A partial/missing
report, process failure or timeout stops the comparison.

No performance optimization is qualified merely by adding this harness. Large-scene
streaming, bounded incremental chunk publication, full GPU pass coverage, memory
telemetry and visual regression captures still require separate implementation and
measurements before claiming the complete 120 FPS plan achieved.

## Observed baseline and verification (15 October 2026)

One complete `motion` run on Apple M4 Max / Metal, 10 s warmup + 60 s measurement:

| Metric | p50 | p99 |
| --- | ---: | ---: |
| Attempted frame interval | 46.67 ms | 80.55 ms |
| Simulation including pose publication | 35.74 ms | 61.65 ms |
| CPU render including presentation | 10.90 ms | 18.78 ms |
| Surface acquisition | 0.034 ms | 0.049 ms |
| Primary voxel GPU pass (56 delayed samples) | 1.25 ms | 6.56 ms |

Raw data is in `baseline/`. This is one observation in a dirty, concurrently edited
checkout, not a paired result or an accepted optimization. The other attempted
scenarios did not produce complete reports. The user subsequently requested no
long native GPU runs. The 120 FPS goal remains unachieved; the first measured
bottleneck is CPU simulation/publication, whose finer stage counters were added
after this observation. The draw/upload cost of incremental chunk publication
has not yet been isolated. No unmeasured performance rewrite is enabled.

The main client suite passed 37 tests before the final extra report test. The
street suite passed 18 tests, ignored one, and failed its existing geometry
assertion (`every frontage owns a distinct facade`: 48 versus expected 24).
Three stale test initializations were updated for the existing `Ageing` API;
the geometry assertion was preserved. Python comparison tests passed three cases.
Workspace strict Clippy fails in existing renderer code (14 diagnostics), and
workspace formatting fails on existing files. Full workspace tests and complete
GPU/visual qualification are not claimed.

## Актуальный план — 21 октября 2026

Цель сохраняется: физический GPU, 1920×1080, прежнее качество изображения и
точность физики, p99 времени кадра ≤ 8,33 мс во всех четырёх сценариях.
**Цель пока не достигнута доказательно:** полной актуальной матрицы нет.
Микрозамеры разных дат/фикстур не складываются в выигрыш времени кадра.

Ближайший этап выполняется **без окон и длинных GPU-прогонов**. Полная матрица
10 с прогрева + 60 с измерений возвращается отдельным последующим этапом, когда
пользователь разрешит такие запуски. Короткие CPU-тесты её не заменяют.

### Текущее состояние

База интеграции: `a8c57d98b`, локальный HEAD совпадает с известным `origin/main`
на момент сверки; новая сетевая проверка в рамках актуализации не выполнялась.
CPU-оптимизации, вода/вилла и исходная Apple shared-upload работа объединены и
отправлены. В рабочей копии продолжаются отдельные изменения рендера, ray/exact
вычислений, ripple clock и Apple frame inputs. Они не считаются частью проверенного
коммита и не должны попадать в сравнение незаметно.

| Направление | Статус и подтверждение | Что ещё требуется |
| --- | --- | --- |
| Воспроизводимые сценарии/отчёты | Есть параметры запуска, фиксированное разрешение/ввод, CSV/JSON, процентили, warmup, отдельное ожидание поверхности и асинхронные GPU-выборки | Полное покрытие проходов, байты загрузок, пиковая память; корректная привязка GPU-выборок к измеряемым submissions |
| CPU-симуляция | Переиспользование scratch прядей и атомарного кандидата Companion; checksum/oracle и парные замеры | Новый сквозной профиль реальной симуляции; шерсть, столкновения, extraction ранжировать заново |
| Подготовка палитры | Сохранён общий буфер, прямое добавление матриц, inverse один раз за tick | Замерить долю в реальном кадре; повторно не оптимизировать микросекундный участок без профиля |
| Проверка render pose | Кэш одинаковых влияний, параллельный скан и консервативная сертификация групп; точный fallback | Актуальная стоимость на текущей геометрии и сложных/необычных позах, частота fallback и стоимость потоков |
| Temporal pose | Повторная принятая поза переиспользует admission; меняющаяся — матрицы соседних вершин | Проверить долю в основном пути и p99 текущего приложения; synthetic temporal-тест не равен всей публикации render pose |
| Apple shared uploads | Базовый ring/rigid-путь в main; отдельный отчёт содержит парные аппаратные проверки | Frame inputs ещё в рабочей копии; проверить точный кандидат, переполнение ring, бюджет и fallback перед включением в базу |
| Перестройка/публикация чанков | Завершение исходного пункта не подтверждено собранными отчётами | Измерить очередь, подготовку/загрузку/публикацию и проверить согласованность версий |
| GPU, culling, depth bounds, ray | Есть реализация и продолжающиеся исследования | Нет общей квалификации проходов, изображения и четырёх сценариев; исследовательские режимы не считать автоматически production-базой |
| 120 FPS | Не квалифицировано | Свежая аппаратная матрица, p99, skips, загрузочные задержки и изображение |

Ссылки на доказательства:

- [CPU pose и scratch прядей](../2026-10-15-cpu-pose/README.md).
- [Атомарная симуляция и политика кэша](../2026-10-16-cpu-round-2/README.md):
  около −24% CPU tick Companion на той фикстуре.
- [Буфер палитры и неизменная поза](../2026-10-16-cpu-round-3/README.md):
  около −40/44% подготовки палитры для одного/двух спутников.
- [Меняющаяся temporal pose](../2026-10-16-cpu-round-4/README.md):
  −81,9% для длинных повторов и −23,6% для смешанных влияний; контроль +2,95%,
  шумные измерения, это p50 CPU, а не p99 кадра.
- [Параллельный admission и сертификация групп](../2026-10-16-parallel-pose-admission/README.md):
  отдельный отчёт о реальной геометрии; его времена не переносить на текущую
  геометрию без повторного измерения.
- [Базовые shared uploads](../2026-10-20-apple-shared-upload/README.md) и
  [frame inputs в рабочей копии](../2026-10-20-apple-frame-inputs/README.md):
  аппаратные результаты отдельных потребителей, не доказательство FPS.

Кэш rest orientation шерсти из round 4 отклонён: выигрыш 0–1% вместо требуемых
5%, производственный кандидат удалён. Повторять его без новой причины не нужно.
Проверки интеграции `a8c57d98b`: сборка всех целей workspace, 74 Rust-теста и
7 Python-тестов прошли. Это историческая проверка коммита, не проверка нынешних
незакоммиченных изменений, не полный workspace test и не warning-free Clippy.

### Следующий этап: порядок работ

1. **Зафиксировать измеряемую базу и снять короткую CPU-трассу.** Использовать
   неизменяемый снимок исходников и отдельный сохранённый бинарник. Записать SHA,
   настройки, seed, хэш геометрии/палитры, backend и режим потока ресурсов.
   Headless replay должен выполнять реальную симуляцию и подготовку ресурсов,
   а не только тестовый математический kernel. Раздельно измерить simulation,
   collision/gait/fur, extraction, render/temporal admission и CPU publication;
   вывести p50/p95/p99/max и число fixed ticks на кадр. GPU-работу/ожидания,
   которых в replay нет, явно отметить отсутствующими. Результат этапа — рейтинг
   стоимости и хвостов с абсолютными временами, а не заявление о FPS.
2. **Выбрать крупнейшую измеренную CPU-причину.** Для admission проверить
   сертифицированные и fallback-позы, повторение влияний, размер палитры и
   накладные расходы потоков. Для симуляции — solver шерсти и контакты, затем
   обходы мира и extraction, если они попадают в профиль. По одному изменению,
   без изменения timestep, числа итераций или публичного lifecycle ADR 0002.
   Каждому кандидату — oracle/checksum и минимум пять чередующихся пар на одной
   фикстуре; при шуме повторять всю серию, сохранять и неблагоприятные результаты.
3. **Измерить и ограничить публикацию чанков.** Добавить счётчики backlog,
   размера изменённых мешей, байтов загрузки и времени commit. Стресс-сценарии:
   границы/отрицательные координаты, разрушение во время перестройки, устаревший
   результат и смена ресурсов. Ограничивать работу кадра, публиковать согласованную
   версию, сохранять отбрасывание устаревшего результата. Полный набор мешей не
   перезагружать, если изменена только часть.
4. **Закрыть пробелы измерений и сценариев.** Street сейчас проходит resident
   сцену; добавить настоящий вариант появления/загрузки ресурсов. Текущий water
   сценарий не считать автоматически покрытием новой FFT-воды и перестройки
   чанков: определить отдельные события и ожидаемые состояния. Расширить GPU
   timestamps на реальные проходы и postprocessing, отдельно submission/acquire;
   заполнить upload/memory telemetry без синхронного ожидания GPU. Проверить
   fixed-step catch-up, паузу/возврат и интерполяцию под CPU-нагрузкой, сохранив
   частоту и результаты физики. Смена содержимого сцены требует новой базы.
5. **Квалифицировать текущие GPU-кандидаты отдельно.** После согласования
   аппаратных запусков ранжировать проходы, измерить стоимость culling/depth bounds
   и сэкономленную отрисовку, проверить консервативность и сохранение прозрачности.
   Apple frame inputs проверять по их capability gate, fallback, memory budget,
   переполнению очереди и изображению, не распространяя результаты M4 Max на
   непроверенные устройства. Снижение качества/разрешения в этот этап не входит.
6. **Финальная приёмка.** На точном снимке — тесты затронутых подсистем,
   форматирование/Clippy/workspace checks с отдельным учётом существующих проблем;
   визуальное сравнение одинакового кадра, глубины, прозрачности и движения,
   resize/возврат после паузы. Затем четыре сценария, 1080p, 10+60 с, пять
   чередующихся аппаратных пар. Для отдельной оптимизации: ≥5% целевой метрики,
   ухудшение p99 остальных сценариев ≤3%. CPU microbench допускает локального
   кандидата, но не закрывает этот критерий. Общий успех: p99 ≤8,33 мс во всех
   сценариях; skips и загрузочные задержки отражены отдельно. Если не выполнено —
   измеренный остаточный bottleneck и бюджет следующего этапа, без подмены средним FPS.

Ближайшее конкретное действие — пункт 1, затем один кандидат из пункта 2.
До нового профиля приоритет публикации чанков относительно admission остаётся
гипотезой; численный бюджет для них нельзя честно вывести из старых микрозамеров.

### Результат следующего CPU-этапа — 21 октября 2026

[Сквозной CPU replay и замеры](../2026-10-21-cpu-replay/README.md) реализован
для цепочки персонаж → анимация/спутники → палитра → render/temporal admission
→ CPU-публикация и извлечение теневых сфер. Окон и GPU-устройств не создаёт.
Это частичное покрытие пункта 1: вода, чанки, vehicle/LOD и GPU-операции остаются
вне replay. На снимке текущих исходников temporal admission занимал около 79%
измеренного времени; его заменяет достаточная граница конечности позиций с точным
fallback. Первый кандидат с дополнительными потоками отклонён из-за p99.

Пять пар дают медиану run CPU p99 для движения 10,248 → 2,108 мс (−79,4%).
Все хэши геометрии и накопленного состояния совпадают. После полного повторения
стресс-сценария суммарно десять пар дают 9,379 → 2,213 мс по медиане run p99.
Однако объединённый стресс-p99 по всем CPU-кадрам **12,888 мс против 11,861 мс**,
а максимум кандидата — 34,162 мс. Поэтому стабильность хвостов и полная приёмка
по условию отсутствия регрессии p99 остаются открытыми. Эти результаты не равны
p99 показанных кадров и не подтверждают 120 FPS.

Следующий измеренный приоритет в покрытой CPU-цепочке — render admission
(около 97% оставшегося времени); отдельно проверить стоимость запуска/планирования
рабочих потоков и ограниченное переиспользование workers. До изменений заранее
зафиксировать способ агрегации p99: показывать медиану run p99, pooled p99,
максимум и все неблагоприятные пары. Затем вернуться к очередям публикации чанков
и пробелам телеметрии из исходного плана; их стоимость этим replay не установлена.

Два варианта переиспользуемого пула workers затем проверены отдельно на пяти
парах каждого CPU-сценария; оба ухудшили хвосты и удалены вместе с новой
зависимостью. Это проверенная отклонённая гипотеза. Следующая работа должна
разделить стоимость запуска/планирования и арифметики сертификатов под
контролируемой нагрузкой, а не автоматически заменять threads на pool.

### 22 October: pooled CPU tail remains open

[The pooled-tail investigation](../2026-10-22-pooled-tail/README.md) localized the
historical worst 1% CPU frames to render-pose admission (median 96.7% of elapsed
frame time), without a repeatable problematic trajectory frame. Four-worker,
sequential and dynamic-block candidates were rejected; no new renderer change is
retained. The complete dynamic-block matrix improved median run p99 but regressed
pooled p99 in motion and stress. The collector now requires **both** metrics, so
this failure can no longer pass the median-only gate. External CPU contention
(load 34–47 on 16 logical CPUs) prevents a clean causal qualification; next collect
worker/process CPU versus elapsed time during an uncontended CPU control interval.
The original pooled stress regression and physical 120-FPS qualification stay open.

### 22 October: CPU versus worker waiting, retained family certificate

[The follow-up CPU investigation](../2026-10-22-pose-cpu-wait/README.md) measured
thread/process CPU separately from spawn, work and join elapsed time. Baseline
render admission certified 518,549 weight groups per pose; monitored worst-frame
join time was predominantly off-CPU. A conservative convex joint-family certificate
now avoids per-weight work and temporary workers for certified poses, retaining
original exact fallback and current-pose checks. Five initial AB/BA pairs per scenario plus full five-pair repeats of the
disputed idle/stress series, with profiling clocks disabled, passed both median
and pooled p99 gates. All 50 phases are included: idle 20.733 → 0.514 ms (10 pairs),
motion 61.448 → 2.058 ms (5 pairs), catchup 28.627 → 4.357 ms (10 pairs) pooled
CPU-replay p99. State/geometry checksums agree. CPU correctness, clock and collector
tests plus release workspace check passed; strict Clippy remains blocked by unrelated
`world_nearest_wide.rs` dead code. Rare long frames and the uncertain-pose fallback
remain, including an optimized idle maximum of 234.778 ms. External CPU load was
high; physical displayed-frame 120-FPS/1080p qualification remains open.
