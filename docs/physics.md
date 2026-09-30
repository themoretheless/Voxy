# physics и physics_voxel

Зависимости направлены в одну сторону: приложение → physics_voxel → physics.
Пинбол использует physics напрямую, без воксельного адаптера.
Общий physics не имеет зависимостей, включая voxy_core и voxy_world.

## physics

Общая кинематическая физика: независимые Origin/AnchoredAabb, непрерывная проверка
столкновения двух коробок, контроллер персонажа с гравитацией, прыжком, скольжением
и подъёмом на ступени. Целочисленный origin сохраняет локальную точность вдали от
начала координат; единицы выбирает приложение, они не означают блоки.

CollisionWorld задаёт запрос движения AABB. Backend возвращает ближайший контакт
и собственные типы препятствия и ошибки; общий алгоритм не знает о блоках, чанках,
registry, окне или ECS. Лимит max_candidates_per_sweep ограничивает работу backend.
Непрогруженную геометрию нельзя выдавать за свободное пространство.

AABB-контроллер не является полноценным rigid-body solver: вращающиеся 3D-тела,
капсулы персонажа, произвольные нормали склонов и суставы не реализованы.
Отдельный planar-модуль ниже решает ограниченную задачу плоских контактов.
sweep_box принимает проверенные конечные упорядоченные границы в одной локальной
системе координат. Начальное пересечение возвращает нулевую нормаль; отдельного
решателя выталкивания из пересечений пока нет.

## physics_voxel

- VoxelCollisionWorld реализует общий контракт через VoxelView и BlockRegistry.
- collision выбирает воксельные кандидаты, проверяет бюджет и вызывает общий sweep_box.
- character — тонкий адаптер координат и состояния к общему контроллеру, без копии solver.
- raycast выполняет обход воксельной сетки.
- water и destruction возвращают планы изменений, не меняя мир самостоятельно.
- projectile связывает полёт с воксельными попаданиями и взрывами.
- vehicle пока содержит игровой контроллер транспорта и правила трассы; столкновения
  транспорта проходят через общий контроллер персонажа.

Существующий API приложения доступен из physics_voxel. CharacterConfig и
CharacterInput переэкспортированы из physics; прежний параметр
max_candidate_voxels_per_sweep называется max_candidates_per_sweep.

Мир по-прежнему владеет блоками, транзакциями и каноническим состоянием.
Приложение задаёт ввод и фиксированный шаг и применяет готовые планы.
Обратных зависимостей из physics на адаптер и из voxy_world на физику нет.

## Проверка независимости

crates/physics/tests/boxes.rs использует обычные коробки с дробными размерами,
без воксельных зависимостей: приземление, прыжок, скольжение и атомарность
состояния при ошибке backend/переполнении координат. Старые воксельные тесты
сохранены в physics_voxel и вызывают тот же общий алгоритм.

Проверки: cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings,
cargo fmt --all -- --check. cargo tree -p physics должен показывать только сам crate.

## physics::planar и пример пинбола

`Vec2`, `Ball`, `Capsule`, `resolve_contact` не зависят от Voxy или сторонних crate.
Капсула — округлый отрезок; при совпадении концов получается круг. Решатель
выталкивает шар из пересечения и отражает нормальную составляющую относительной
скорости с заданной упругостью. Скорость поверхности включает перенос и вращение
вокруг первого конца капсулы: `v + omega * perpendicular(contact - pivot)`.
Поэтому удар концом поднимающейся лопатки сильнее удара у оси. Разделяющийся контакт
исправляет пересечение без повторного импульса. При нулевом расстоянии используется
детерминированная запасная нормаль. Входы должны быть конечными, радиусы положительными.

Это дискретный решатель для одного круга и препятствий бесконечной массы, не CCD
и не общий решатель динамических тел. Вращение самого шара, трение контакта,
взаимодействия нескольких шаров и объёмные рампы не моделируются.

Пример `crates/voxy_app/examples/pinball` задаёт единицы, поле и игровой цикл:

- фиксированный шаг 1/240 секунды с четырьмя подшагами;
- радиус шара 0.22, скорость ограничена 45 единицами/с перед перемещением и после ударов;
- перемещение шара за подшаг не более 0.047, конца лопатки — примерно 0.034;
- гравитация 10 единиц/с², слабое затухание скорости, бамперы добавляют импульс;
- счёт бампера имеет защиту от повторного начисления в течение 0.15 с;
- накопитель реального времени ограничивает задержку кадра до 0.1 с, чтобы не
  выполнять неограниченный догоняющий расчёт; при больших задержках игра замедляется;
- статические и динамические элементы отображаются общей GPU-моделью, лопатки и
  шар получают отдельные матрицы; форма бортов и лопаток соответствует коллайдерам.

Физические тесты проверяют упругость, сохранение касательной скорости, круговые
концы, разделяющиеся контакты и передачу скорости вращения. Тесты примера проверяют
запуск, три потери шара, быстрый удар о борт, бампер и 30 000 шагов автопилота.
Они включены в `cargo test --workspace`. `--smoke` дополнительно требует реальные
показанные кадры и попадания в бамперы; это проверка запуска, а не оценка игрового баланса.

## Smooth simulation time

`voxy_runtime::SimulationClock` advances from real elapsed seconds, even when
simulation time is paused. It integrates the scale into a fixed-step accumulator;
physics always receives the same step. Decreasing the target uses critical damping.
Increasing it uses a damped spring (frequency 12 rad/s, damping ratio 0.55), with
roughly 12% overshoot of the speed change before settling. Retargeting preserves
both current scale and its velocity. The scale cannot become negative.

`advance(real_dt, step, max_steps)` returns a step count, interpolation fraction,
and overload flag. Real frame time is capped at 100 ms; excess whole simulation
steps beyond the budget are dropped, so sustained overload slows effective time.
Targets must be finite and between 0 and 4. `freeze()` stops immediately for focus
loss; normal pauses set the target to zero and brake smoothly.

Pinball uses a 240 Hz physics step, a 128-step frame budget, and interpolated ball
and flipper transforms. Controls: S = 0.1x, 1/2/4 = target speed, P = smooth
pause/resume, N = one step after fully pausing, R = reset. The window title shows
current and target speed and overload. Focus loss freezes immediately.
Run `cargo run -p voxy_app --example pinball`; `--smoke` also exercises slow motion,
spring recovery, and 4x speed during the automated collision run.
