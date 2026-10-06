# physics и physics_voxel

## Орбитальная механика

`physics::astrophysics` добавляет относительную задачу двух тел с
`mu = G * (m1 + m2)`. `Conic::state` преобразует наклонённую эллиптическую,
параболическую или гиперболическую орбиту в положение и скорость. `orbit`
возвращает удельную энергию, вектор углового импульса, вектор эксцентриситета,
перицентр, а для связанных орбит — апоцентр и период.

`propagate` решает универсальное уравнение Кеплера с функциями Стампфа и
ограниченным методом Ньютона внутри найденного интервала. Поддерживается
отрицательное время. Проверки охватывают круговую орбиту на 100 оборотов,
эксцентричный эллипс, околопараболическую/параболическую и гиперболическую
траектории, сохранение энергии/углового импульса и обращение времени.
Источник метода: [NASA/JPL SPICE prop2b](https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/cspice/prop2b_c.html).

Это изолированная ньютоновская задача: возмущения других масс моделирует
`gravity`; столкновения, прямолинейное падение на центр,
эволюция звёзд, газ и перенос излучения не входят в этот орбитальный решатель.
Численный диапазон ограничен f64; переполнение и отсутствие сходимости возвращают
ошибку. Астрофизическая подсистема пока находится в разработке.

`NewtonianField::tidal_tensor` возвращает производную ускорения по положению.
Она согласована с внешним полем, линейным полем внутри однородной сферы и
сглаживанием Пламмера. Вне несглаженной точечной массы тензор имеет нулевой след:
радиальное растяжение сопровождается сжатием в двух поперечных направлениях.
На границе однородной сферы производная разрывна; метод возвращает внешнюю ветвь.

`astrophysics::gravity_gradient_torque` вычисляет ведущий момент приливных сил
для заданного симметричного тензора инерции в той же системе координат.
Приливное поле должно мало меняться на размере тела: это линейное приближение,
без деформации, диссипации или самопроизвольного приливного захвата.
Источник: [NASA, gravity-gradient torque](https://ntrs.nasa.gov/api/citations/19800005912/downloads/19800005912.pdf).

`astrophysics_spin::Spin` хранит principal moments, инерционный угловой импульс
и quaternion ориентации тела. Метод `step` интегрирует постоянный мировой момент
на шаге с неявной midpoint-ориентацией; `world_inertia` связывает вращение с
приливным моментом. В отсутствие момента угловой импульс сохраняется точно;
энергия для асимметричного тела сохраняется приближённо и проверяется на длинном
прогоне. При отсутствии сходимости следует уменьшить шаг. Ошибки не меняют состояние.

`astrophysics_binary::Binary` связывает относительную орбиту пары с вращением
обоих тел и их теплом. Постоянная фигура описана потенциалом Маккаллага до
квадрупольного порядка; сила и моменты берутся из одного потенциала и сохраняют
суммарный угловой импульс. Love number и постоянная временная задержка задают
равновесный прилив и его диссипативную часть. Сила действует на орбиту с
равным обратным моментом на вращение; потерянная механическая энергия
депонируется в `Body::heat` конкретного тела.

Диссипативный шаг решает связанную линейную систему для относительной скорости
и двух угловых импульсов методом midpoint; тепловая мощность неотрицательна.
Консервативный шаг использует kick-drift-kick и свободное вращение. Контакт
проверяется по всему линейному drift, а не только в его конечных точках.
Превышение бюджета, контакт или численная ошибка откатывают всю пару.

`cargo run -p physics --example tidal_binary` выводит замедление вращения,
тепло и ошибки полной энергии/углового импульса за 20 единиц времени.
Тесты проверяют обратную реакцию несферической фигуры, приливный обмен и тепло
на 10 000 шагах, отсутствие начального нагрева при синхронном круговом вращении,
пролётный контакт и атомарность ошибки после успешных подшагов.

Модель требует хорошо разделённых тел, маленького lag и заданных постоянных
параметров отклика. Это изолированная пара с квадрупольным приближением,
без динамического расчёта реологии, резонансных мод, деформации или слияний.
Тепло пока не связано с излучением и структурой тела.
Источники: [потенциал Маккаллага](https://farside.ph.utexas.edu/teaching/celestial/Celestialhtml/node73.html),
[constant-time-lag tidal model](https://www.aanda.org/articles/aa/pdf/2010/08/aa14337-10.pdf).

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

### Ньютоновская гравитация

`physics::gravity` рассчитывает взаимное притяжение положительных точечных масс
в трёх измерениях. `Gravity::constant` задаёт G в единицах приложения; значение
по умолчанию — 6.67430e-11 для метров, секунд и килограммов. `uniform_acceleration`
добавляет постоянное внешнее поле. Масса влияет на притяжение других тел, но
не меняет ускорение свободного падения в одинаковом внешнем поле.

`step` использует velocity Verlet; приложение должно вызывать его с постоянным
шагом, достаточно малым для самой быстрой орбиты. Все пары вычисляются напрямую,
за O(N²); тела движутся взаимно, без закреплённого центрального тела.
`softening` задаёт длину сглаживания Пламмера: ноль соответствует точному закону
обратных квадратов, положительное значение ограничивает близкие взаимодействия.
Совпадающие несглаженные массы возвращают ошибку. Некорректные входы и переполнение
не публикуют частично обновлённое состояние.

Тесты проверяют закон обратных квадратов, равенство противоположных сил,
аналитическое свободное падение и 100 000 шагов круговой двойной орбиты с контролем
энергии, импульса и центра масс.
`Gravity::diagnostics` возвращает кинетическую и потенциальную энергию,
линейный и орбитальный угловой импульс и суммарную массу. Потенциал согласован
со сглаживанием Пламмера и постоянным внешним полем; вращательная энергия сфер
не входит в эту диагностику точечных масс.

`physics::gravity_spheres` добавляет конечные однородные сферы: аналитический
поиск первого контакта в линейном drift-подшаге, обмен импульсом с коэффициентом
упругости, кулоновское трение и угловые скорости с I = 2mr²/5. Гравитационные
kick-подшаги используют тот же закон притяжения; шаг должен разрешать изменение
ускорения. Начальные пересечения исправляются с сохранением центра масс.
Лимиты подшагов и контактов ограничивают работу; их превышение откатывает весь шаг.
Это сферическая модель: произвольные формы и их гравитационные мультиполи здесь
не поддерживаются. Одновременные контакты решаются совместной итерацией импульсов:
все пары читают одно состояние, а цели упругого отскока фиксируются до итераций.
Учитываются и неподвижные касающиеся соседи. Нормальный импульс неотрицателен,
касательный ограничен кулоновским трением. Бюджет max_contacts учитывает каждую
обработку пары, включая итерации; несходимость исчерпывает бюджет и откатывает шаг.
Допуск итерации равен 1e-12 масштаба начальной относительной скорости контактов.
Остаточное сближение уже касающихся тел ниже 1e-12 начального масштаба скорости
drift-подшага не порождает повторные удары в нулевое время. Регрессии проверяют
перестановки трёх тел, симметричный отскок, импульс и момент при трении,
диссипацию и полностью неупругий удар. Начальные пересечения исправляются
совместными позиционными итерациями с сохранением центра масс; отклик скорости
вычисляется один раз после исправления геометрии. Проверены все перестановки
пересекающейся группы с разными массами и откат при нехватке бюджета. Для точно
совпадающих центров направление задаёт относительная скорость; если она нулевая,
геометрия не задаёт направления разделения и шаг возвращает InvalidInput атомарно.

`cargo run -p voxy_app --example gravity` показывает объёмные сферы и следы
движения через штатный SceneRenderer. 1 — двойная орбита, 2 — столкновение тел
разных масс, 3 — ходьба по планете, J — радиальный прыжок, R — сброс, Space — пауза.
`--planet` запускает третий режим сразу. Персонаж автоматически идёт по касательной
и периодически прыгает; направление вверх следует полю, в том числе под планетой.
Камера отдаляется при разлёте тел.
`--smoke` проверяет показанные кадры, resize, реальное столкновение, перевёрнутую
ходьбу, прыжок и приземление; окно должно
оставаться видимым. Физика работает с фиксированным шагом 1/240 с, изображение
интерполируется между состояниями. Это подключение к объектам демонстрационной
сцены. `physics::gravity_field::NewtonianField` задаёт список точечных или
однородных сферических источников с целочисленным anchor и локальной позицией.
Внутри несглаженной сферы ускорение линейно зависит от расстояния до центра,
снаружи действует закон обратных квадратов. Разности anchor вычисляются в i128
до преобразования в f64, сохраняя локальную точность рядом с дальним источником.

`step_character_in_field` и `step_projectile_in_field` в `physics_voxel`
принимают это поле либо постоянный вектор ускорения. Основное приложение теперь
использует эти вызовы с прежним постоянным полем: управление, приземление и
баллистика сохраняются. Персонаж остаётся кинематическим Y-up контроллером:
его мотор задаёт X/Z-скорость на каждом шаге; прыжок, ступени и ground detection
ориентированы по Y. Свободные орбитальные тела следует моделировать через
`gravity`/`gravity_spheres`, а не контроллер персонажа. Источники внешнего поля
задаются вызывающим кодом и не получают обратную реакцию от тестовых объектов;
для взаимного движения масс используется N-body симуляция.

`physics::gravity_character` — отдельный контроллер сферического персонажа с
произвольным направлением вверх и непрерывными контактами с вещественными
нормалями. Поле задаёт вверх как -normalize(acceleration), при нулевой гравитации
сохраняется предыдущий unit-вектор. Желаемая мировая скорость проектируется на
касательную плоскость; `tangent_velocity: None` сохраняет инерционное движение.
Прыжок направлен вверх и отключает snap на весь шаг. Ground snap следует новому
направлению поля после перемещения. Origin ребазируется с проверкой переполнения;
ошибки поля, коллизий и бюджета не меняют состояние.

`SphereWorld` реализует контракт `gravity_character::CollisionWorld` для статических
сферических поверхностей. Другие формы подключаются через этот контракт; он требует
ближайший контакт, единичную нормаль и явную ошибку при отсутствующей геометрии.
Начальные пересечения возвращают ошибку, а не неявное выталкивание. Проверки
охватывают полный обход планеты, прыжки/приземления по ±X/±Y/±Z, нулевую гравитацию,
дальние координаты и быстрое движение без пролёта сквозь поверхность.

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

## Grass and hair strands

`physics::strand::Strand` is a dependency-free CPU Verlet/PBD strand with a pinned
root, segment-length constraints, damping per second, and acceleration toward a
root-relative rest shape. Stiffness is in s^-2: high values keep grass upright;
low values let hair hang and lag behind a moving attachment. This rest-shape spring
is a simplified bending model, not an angular rod or XPBD solver.

Call `step` with a fixed timestep (demo: 1/240 s), world acceleration including wind,
a new root position, and sphere colliders. Steps above 1/30 s are rejected. Invalid
inputs and nonfinite results leave state unchanged. Collisions project free particles
outside sphere radius plus strand radius; the pinned root is never projected.
Finite constraint iterations leave length error, especially during contact. Contacts
are discrete: fast obstacles can tunnel; segment collision, friction, self-collision,
and strand-to-strand collision are not implemented. Use local coordinates.

Run `cargo run -p voxy_app --example strands`: left, 80 blades bend in wind and a
moving sphere presses them aside; right, 28 strands follow a moving head. Geometry
is updated in the existing scene renderer. Space pauses; Esc exits. `--smoke` checks
120 presented frames and surface resize recovery. The demo caps accumulated frame
time at 100 ms and slows simulation under sustained overload.

`cargo test -p physics --test strands` checks wind deflection and recovery, root
attachment inertia, length error, sphere exclusion over 3000 steps, and invalid-step
atomicity. This is a small CPU demonstration, not a GPU hair system for large scenes.

## Volumetric soft-body prototype

`physics::soft_body` uses tetrahedra, XPBD edge-length and signed-volume
constraints, inverse-mass pins, and discrete sphere/vertex projection. Rest edge
lengths provide elastic shape recovery. `step` runs 24 iterations, applies
exponential velocity damping, and accepts fixed steps up to 1/120 s. Invalid
inputs, nonfinite output, or inverted tetrahedra leave the body unchanged.
Compliance is a numerical parameter, not a calibrated tissue modulus; the same
parameter is used for length and volume constraints despite their different units.

`cargo run -p physics --example soft_body` runs a numerical compression/release
example with two sphere obstacles and three pinned vertices; it prints positions
and volume ratios. It does not open a rendering window. This is a small CPU
foundation, not an anatomically validated tissue model. There is no surface
embedding, friction, self-contact, triangle contact, CCD, or moving bone
attachment yet. Vertices outside an obstacle do not guarantee triangles outside
it. Highly compressed steps can fail rather than recover from inversion.

## Soft tissues

The independent `physics::tissue` module supplies compliant skin, volume constraints, moving skeleton attachments and active muscle links. See [model, native example and limitations](tissues.md). Run `cargo run -p voxy_app --example tissues`.

## Body secondary motion

`cargo run -p voxy_app --example body_motion` displays a clothed schematic mannequin
from the front and rear. Four volumetric `Tissue` samples drive the visible breast
and buttock surface vertices through barycentric attachments. A twelve-second cycle
contains gait-like vertical excitation, jumps, and rest; it is not a walking
skeletal animation. Moving pins follow the torso; free particles lag under inertia
and gravity, with compliant length/volume constraints and damping. Space pauses,
Esc exits. Parameters are illustrative, not calibrated anatomical measurements.
The body mesh is schematic and has no continuous skin or garment simulation.

The `body_walk_jump_and_settle` test verifies relative secondary motion and settling
in the rest phase. `--smoke` verifies presented frames and resize recovery, not visual
or anatomical accuracy.

## Embedded render surfaces

`physics::tissue_surface::EmbeddedSurface::bind(rest_positions, tetrahedra,
render_positions)` stores barycentric attachments in rest space. After each
simulation step, `deform(current_positions)` returns render vertices following
local tetrahedral deformation, including shear and rotation. Use
`Tissue::tetrahedra()` to obtain the simulation topology; preserve simulation
vertex order/count for the lifetime of a binding. Render topology stays owned by
the application; recalculate surface normals after deformation.

Binding searches all cells (O(vertices * cells)); per-frame deformation is linear
in render vertices. Vertices must be inside the tetrahedral domain; exterior
vertices, degenerate cells, invalid indices and nonfinite input are rejected.
Boundary ties choose the first cell. There is no nearest-cell extrapolation,
spatial acceleration, inversion correction, collision guarantee, or skin material
model in this attachment layer. A binding can also be reused with `SoftBody` or
an external solver when vertex indexing is stable.

```rust
use physics::tissue::{sample, TissueKind};
use physics::tissue_surface::EmbeddedSurface;
let mut body = sample(TissueKind::Buttock, [0.0; 3])?;
let cells: Vec<_> = body.tetrahedra().collect();
// Replace these coarse vertices with an authored render mesh inside the domain.
let surface = EmbeddedSurface::bind(body.positions(), &cells, body.positions())?;
body.step(1.0 / 240.0, [0.0, -9.81, 0.0], &[], 24)?;
let render_positions = surface.deform(body.positions())?;
# Ok::<(), &'static str>(())
```

## Nonlinear skin

`physics::skin` adds a layered GOH-type shell, signed-dihedral bending, objective Maxwell relaxation and implicit dynamics. [Model and verification](skin.md). The native `female` example binds a simulated anatomical skin patch to Blender Studio's CC0 realistic female mesh.

The body-motion demo now subdivides each simulation boundary face and binds those
render vertices through `EmbeddedSurface`. Each vertex follows its own cell;
axis-extent fitting is no longer used. The coarse octahedral simulation domain
remains visibly faceted; subdivision does not create additional physical detail.
`EmbeddedSurface::deform_into` updates a caller-owned buffer without allocations.
It validates all results before publishing any output and requires an exactly
matching buffer length. Its two evaluation passes trade arithmetic for atomicity.

## Strength diagnostics in continuum FEM

`biomechanics::Material::from_young_poisson(E, nu)` specifies the isotropic
matrix using small-strain Young's modulus in pascals and Poisson's ratio
(-1 < nu < 0.5). It converts these to shear and bulk moduli; the existing
large-deformation neo-Hookean law remains in use. This does not model linear
elasticity at arbitrary large strains, plasticity or material failure.

`Material::stress(F, activation)` computes spatial Cauchy stress
`sigma = P F^T / det(F)`. `Body::stresses_at(positions)` returns that stress
for each tetrahedron, with node indices, reference volume and volume ratio.
`Stress` reports principal stresses in descending order, pressure (positive
in compression), von Mises stress and maximum shear stress, all in pascals.
The stress measure follows the current configuration; see
[COMSOL's stress definitions](https://doc.comsol.com/6.4/doc/com.comsol.help.sme/sme_ug_theory.06.017.html).
Normalized cyclic Jacobi diagonalization avoids the repeated-eigenvalue loss
of precision of an acos-based cubic formula. Nonfinite inputs, asymmetric
external stress tensors and unrepresentable diagnostics fail explicitly.

`yield_utilization(yield_pa)` returns von Mises stress divided by supplied
yield strength. A value >= 1 diagnoses onset of yielding for isotropic ductile
materials. It does not change the constitutive response, remove elements or
predict fracture. This criterion is not a calibrated failure law for anisotropic
soft tissues, concrete, wood or brittle solids. Strength values must come from
the material and experiment being modelled.

Run `cargo run -p physics --example strength` for a numerical load-controlled
coupon (no window). A 0.1 m tetrahedron uses E = 2 MPa, nu = 0.3 and a 1 N axial
load; three pins constrain transverse strain. Its effective axial area is V/L,
not a rectangular section. The example compares extension with the constrained
small-strain modulus K + 4 mu/3 and checks axial stress against force/area.
These are illustrative constants. The equilibrium tolerance is 5e-6 N; tighter
1e-8 N convergence was not attained in this small-strain case, and small-strain energy evaluation subtracts nearly equal quantities.
The precise cause of this convergence floor has not been isolated. The example fails if equilibrium
does not meet its stated tolerance.

`cargo test -p physics --test stress` verifies uniaxial, pure shear and
hydrostatic stress, invariance under rotation, zero stress at rigid rotation,
Young/Poisson small-strain response, an affine tetrahedral patch, actual loaded
equilibrium, force balance, inversion rejection and extreme numerical scales.
These are constitutive/numerical checks, not experimental material validation.
There is no stress visualization, fracture evolution, voxel coupling or mesh
convergence study in this addition. Existing explosion resistance remains a
gameplay parameter, not a continuum strength calculation.

## Plastic deformation of solids

`physics::plasticity` adds small-strain J2 plasticity with linear isotropic
hardening, distinct from the large-deformation soft-tissue constitutive law.
`Material::new(E, nu, yield_pa, hardening_pa)` takes pascal-valued constants.
Tensorial strain is symmetric; off-diagonal components are half engineering
shear. Plastic volume change is zero. The von Mises flow stress is
`yield_pa + hardening_pa * equivalent_plastic_strain`. A backward-Euler radial
return computes the plastic increment `(q_trial - flow_stress)/(3G + H)` when
outside the yield surface. See the
[MOOSE isotropic plasticity formulation](https://mooseframework.inl.gov/moose/source/materials/IsotropicPlasticityStressUpdate.html).

The returned `State` records irreversible plastic strain, equivalent plastic
strain and dissipated energy density. Hardening energy is separately reported
as `H alpha²/2`; irreversible dissipation is `yield_pa * alpha`. A nonmutating
response returns a candidate state, so rejected global trials cannot accumulate
plastic history. `tangent_action` supplies the consistent algorithmic tangent.
A roundoff-sized yield tolerance prevents repeated holding increments and
incorrect loading tangents at an accepted yield surface during unloading.

`plasticity::mesh::Body` assembles constant-strain tetrahedra and solves static
force balance with dense Newton steps and residual backtracking. Nodal loads
are in newtons; prescribed displacement components are in metres relative to
rest. Components marked `None` are free. It reports free-DOF residual and
support reactions. Material state and positions commit together only when the
requested tolerance is attained. Unconverged solves return `converged=false`
and preserve the last accepted body. Singular supports/tangents, invalid inputs,
inversion and numerical overflow return errors without committing changes.
The current CPU implementation limits shared-node meshes to 512 vertices and 8192 cells.
Use local coordinates; it is not a scalable production sparse solver.

Run `cargo run -p physics --example plastic_coupon` for a numerical loading and
unloading demonstration (no rendering window). Illustrative E = 210 GPa,
nu = 0.3, yield = 250 MPa and H = 1 GPa are not a measured material calibration.
At axial strain 0.004, the analytical uniaxial solution gives equivalent plastic
strain 0.00279620853 and axial stress 252.7962085 MPa. Removing the applied force
leaves a 0.279620853 mm residual extension on the 0.1 m coupon. The example prints
stress, history, dissipated energy and actual force residual at each accepted step.

Tests `plasticity` and `plastic_mesh` cover return mapping, monotonic work/energy
balance, reverse loading, elastic unloading, hydrostatic loading, tensor basis
covariance, the tangent derivative, rejection atomicity, missing supports,
load/displacement-controlled analytical uniaxial solutions and a homogeneous
patch on 6 versus 48 tetrahedra. The patch refinement check does not establish
convergence of bending, stress concentrations or fracture.

The model assumes infinitesimal strains and a fixed material frame. Coordinate
basis covariance does not imply objectivity under finite physical rotation.
The mesh inversion check does not extend that validity range. There is no rate
dependence, kinematic hardening/Bauschinger effect, temperature coupling,
bulk softening, bulk crack-path insertion or voxel coupling yet. Large rotations
require a finite-strain plastic formulation. The cohesive interface extension
below adds preinserted interfacial fracture and frictionless crack closure.
The original gameplay blast planner is unchanged.

`astrophysics_thermal::ThermalBody` моделирует однородную температуру тела в SI:
постоянная полная теплоёмкость J/K, площадь m² и серая излучательная способность.
`step` принимает выделенное тепло J, температуру изотропного окружения K и время s.
Неявный шаг решает C(Tnew-Theated) = -dt ε A σ(Tnew⁴-Tbath⁴) бисекцией;
температура остаётся между нагретым телом и окружением даже при жёстком охлаждении.
Счётчик net radiated energy позволяет проверять баланс энергии; он отрицателен при
нагреве окружением. Приливное приращение `Body::heat` можно передать как deposited_heat;
накопленный tidal heat остаётся диагностикой диссипации, а не внутренней энергией.
Это модель теплового резервуара: пока без переноса внутри тела, спектров, оптической
толщины, фазовых переходов, звёздной структуры и термоядерных реакций.
Константа и закон излучения: [NIST](https://physics.nist.gov/cgi-bin/cuu/Value?sigma).

`astrophysics_gas::Gas` решает одномерные уравнения Эйлера идеального газа на
равномерной сетке: плотность, плотность импульса и полная плотность энергии.
Давление p=(γ−1)(E−ρu²/2). Консервативный поток Rusanov и адаптивный CFL 0.4
обрабатывают ударные волны без искусственного обрезания плотности/давления.
Периодические границы сохраняют все три интеграла; отражающие — массу и энергию,
но передают импульс стенке; открытые допускают выход через границы.
`step` ограничен бюджетом подшагов, ошибка полностью откатывает эволюцию.
Проверки включают периодическое сохранение, равномерное течение, стенки и
Sod shock tube с сравнением давления, скорости и плотности в star region.
Это первый порядок по времени/пространству: разрывы численно размываются.
Пока нет вакуума, самогравитации газа, трёхмерной сетки, MHD и радиационного переноса.
Источник уравнений и эталонной задачи: [Clawpack Euler shock tube](https://www.clawpack.org/gallery/pyclaw/gallery/shocktube.html).

Internal-organ geometry and orthotropic myocardial FEM: see [organs.md](organs.md).

`astrophysics_star` добавляет ньютоновскую сферическую гидростатическую модель
политропы: dP/dr = −GM(r)ρ/r², dM/dr = 4πr²ρ, P ∝ ρ^(1+1/n).
`lane_emden` решает безразмерные уравнения RK4 с регулярным центральным рядом
и уточняет первую нулевую температуру бисекцией последнего шага. Поддержаны
индексы 0..5; n=0 — несжимаемый предел, n=5 имеет бесконечный радиус.
`surface=None` означает, что первая нулевая точка не достигнута в заданном
max_xi, а не доказательство бесконечного радиуса. Шаг ограничен 0.1, бюджет
подшагов ограничивает время и память; точность оценивается сгущением шага.
`Scaling` переводит профиль в физические радиусы, плотность, давление и массу
по центральным плотности/давлению и G. При n=0 плотность на поверхности остаётся
центральной, давление обращается в ноль. Проверены аналитические n=0,1,5,
численная n=1.5, сходимость, масса однородной сферы и гидростатический баланс.
Запуск профиля: `cargo run -p physics --example polytrope` (CSV).
Это равновесное строение без вращения, термоядерного энерговыделения, переноса
энергии, эволюции состава и релятивистских поправок.
Вывод уравнений и масштабов: [Princeton, Polytropes](https://www.astro.princeton.edu/~gk/A403/polytrop.pdf).

`astrophysics_radiation` выполняет серый стационарный перенос в LTE вдоль луча:
Iout = Iin exp(−τ) + B(T)(1−exp(−τ)), τ=absorption×length, B=σT⁴/π.
Каждый слой имеет постоянную температуру и коэффициент истинного поглощения.
Интенсивность интегрирована по частоте (W/m²/sr), length в m, absorption в 1/m.
`trace` проходит слои в порядке луча; `deposited` содержит разность входящей и
выходящей интенсивности каждого слоя, отрицательную при чистом испускании.
Для энергии обмена нужно умножить её на телесный угол, проекционную площадь
луча и время; это не полная мощность охлаждения слоя без углового интегрирования.
`expm1` сохраняет точность малой оптической глубины. Проверены закон поглощения,
тонкий/толстый пределы, LTE, разбиение слоя и суммарный баланс вдоль луча.
Пример: `cargo run -p physics --example radiation` выводит CSV для источника
6000 K за газом 3000 K. Температура газа здесь задана: обратный нагрев газа,
рассеяние, частотные линии, доплеровские эффекты и нестационарность ещё отсутствуют.
Источник: [NASA, Notes on Continuum Radiative Transfer](https://hesperia.gsfc.nasa.gov/summerschool/lectures/bastian/Notes_on_Rad_Trans.pdf).

`astrophysics_column::Column` связывает серый LTE-перенос с изменением температуры
плоского газового столба. Слои заданы снизу вверх, толщиной m, поглощением 1/m,
температурой K и теплоёмкостью на площадь J/m²/K. Два полусферических потока
используют μ=1/2, веса 2π: поток изотропного чёрного тела равен πB=σT⁴.
Путь луча через слой равен thickness/μ; потоки в двух направлениях вычисляются
формальным решением. Их дивергенция даёт нагрев каждого слоя W/m².
Адаптивный явный тепловой шаг ограничивает производную охлаждения и падение
температуры; при исчерпании бюджета весь столб откатывается. `escaped_energy`
считает чистую энергию на площадь, вышедшую через обе границы (может быть
отрицательной при внешнем нагреве). Внутренняя энергия + escaped_energy сохраняется.
Проверены LTE-равновесие, прогрев, охлаждение, прозрачность и откат.
Пример: `cargo run -p physics --example radiative_column` (прогрев снизу, CSV).
Это полусферическое приближение, не точное угловое решение: без рассеяния,
радиационного давления, конечной скорости распространения и газодинамического
движения. Геометрия, теплоёмкости и коэффициенты поглощения фиксированы.
Обзор двухпоточных методов: [Heng et al.](https://arxiv.org/abs/1405.0026).

`astrophysics_radhydro::RadiatingGas` соединяет газовую динамику и радиационный
нагрев в SI: сначала полшага Эйлера, затем полный тепловой шаг, затем полшага
Эйлера. Упорядочивание симметричное, но компонентные схемы остаются первого
порядка; для точности расщепления нужно уменьшать внешний dt. Из плотности
вычисляются absorption=κρ и теплоёмкость слоя=ρ cv dx, из внутренней энергии
вычисляется T=e/(ρ cv). После нагрева сохраняются плотность и импульс, меняются
внутренняя энергия, давление и последующее движение. Коэффициенты κ и cv постоянны.
Газовые бюджеты общие для двух полушагов; тепловой бюджет отдельный. Отказ
любой компоненты откатывает всё состояние. Отражающие стенки сохраняют массу
и общую энергию газа + net escaped radiation; открытые границы дополнительно
уносят газовую энергию, и escaped_energy считает только радиационную часть.
Проверены движение при облучении, баланс энергии, LTE и поздний откат.
Пример: `cargo run -p physics --example radiation_hydro` выводит профиль через 1 s.
В этой модели пока нет самогравитации газа, радиационного давления, рассеяния,
термоядерного горения и связи с эволюцией звёздной структуры.

## Energy-regularized interfacial fracture

`physics::cohesive` implements a bilinear traction-separation law with irreversible
scalar damage. Inputs are interface stiffness K and closure stiffness Kc in Pa/m,
peak traction T in Pa, and fracture energy Gc in J/m². The characteristic jumps
are `delta0 = T/K` and `deltaf = 2 Gc/T`; deltaf must exceed delta0. These are
separations in metres, not element strains. The area under the full opening
traction curve is Gc. The law uses equal normal/shear stiffness and equal
mode-I/II toughness; it is not a fitted mixed-mode B-K criterion. See
[cohesive-zone damage and crack closure](https://doc.comsol.com/6.4/doc/com.comsol.help.sme/sme_ug_theory.06.092.html)
for the underlying traction/damage approach.

Effective jump combines positive normal opening and tangential slip. Maximum
accepted effective jump drives damage; unloading follows the degraded elastic
stiffness and cannot heal. Stored energy and fracture dissipation are reported
separately in J/m². Complete opening dissipates exactly Gc. Compression retains
an independent penalty normal stiffness even after complete rupture. By default this
closure model is frictionless and permits finite penetration set by Kc; it is
not an exact nonpenetration constraint.

`plasticity::mesh::Body::add_interface(minus, plus, material)` pairs two boundary
triangles with distinct node indices and matching reference positions. Winding
on the minus side points into the plus solid. Validation rejects shared-node,
nonmatching, duplicate, degenerate and nonboundary pairs. Insert before loading.
The two sides remain distinct in mesh topology and can separate. Three
quadrature points have independent damage histories. Interface tractions and
consistent tangents assemble into the same bulk Newton solve. All interface
histories commit atomically with the bulk histories and nodal positions.
`interface_reports()` integrates energy over reference area into joules.

Run `cargo run -p physics --example fracture_coupon`. Two 0.1 m cubes, joined
across a 0.01 m² plane, load through peak force and complete separation. The
illustrative constants are E = 1 GPa, nu = 0, bulk yield = 100 MPa (the bulk stays
elastic), K = 1e9 Pa/m, Kc = 1e10 Pa/m, T = 100 kPa and Gc = 10 J/m². Loading
ends at a 0.2 mm opening and a total work of 0.1 J. After reclosure by a prescribed
-0.01 mm end displacement, the reaction is -333.333333 N; the crack remains
fully damaged. The example prints the full reaction/opening/energy curve and
fails on missed equilibrium or incorrect fracture work. It has no window.

Tests `cohesive` and `cohesive_mesh` cover irreversible history, no tensile
healing, compression after rupture, mixed-mode tangent derivatives, fracture
work, force/energy derivatives under nonuniform opening, independent quadrature
history and rejection atomicity. The full load-to-break/reclosure curve and
work/energy balance match on 12- and 96-tetrahedron meshes with 2 and 8 crack
triangles. This verifies a homogeneous preselected plane, not arbitrary crack
path convergence or experimental fracture toughness.

The implementation assumes small strain/sliding and fixed reference normals;
paired vertices do not search for new contact partners after large sliding.
It does not insert new cracks, branch them, grow cracks through bulk elements,
provide finite-rotation fragment dynamics. The small-strain dynamic wrapper
below moves disconnected solid components. Optional Coulomb friction is described below.
Only preinserted planes
can fracture. The static Newton solver checks residual, not stability: unstable
softening/snap-back needs an appropriate arc-length/dynamic continuation scheme.
The demonstration uses a stiffness ratio with a stable displacement-controlled
branch. Arbitrary material/mesh/load combinations are not thereby validated.

`astrophysics_gas_gravity` добавляет самогравитацию плоского газового столба с
изолированным полем: φ''=4πGρ. Горизонтальные слои бесконечны; ускорение ячейки
равно 2πG(масса справа − масса слева) на единицу площади, собственная средняя
сила равна нулю. `field` за O(N) вычисляет поле и точный интеграл потенциальной
энергии кусочно-постоянной плотности, включая собственный вклад ячейки.
Используется положительный Green kernel 2πG|x−x'|: ноль потенциала на бесконечности
для бесконечной плоскости невозможен. Периодическая граница отклоняется.
`SelfGravitatingGas::step` выполняет kick–Euler drift–kick, сохраняет внутреннюю
энергию при гравитационном импульсе и пересчитывает поле после переноса газа.
Внешний шаг ограничен также гравитационным временем; бюджеты откатываются атомарно.
С отражающими стенками гравитационная работа определяется переносом массы
через границы ячеек: −Δmass × Δpotential, со средним потенциалом до/после шага.
Она заменяет энергетический вклад импульсов-предикторов. Потенциал включает
собственный вклад ячейки, а его производная совпадает с производной энергии.
Это сохраняет газовую + потенциальную энергию до округления; глобальной
нормировки и обрезания температуры нет. Открытые границы используют фактический
перенос массы и возвращают гравитационный обмен через границу. Пространственная точность остаётся
первого порядка и требует сгущения сетки. Тесты баланса охватывают 100→200→400.
Также проверены однородный аналитический слой, масса, симметрия и поздний откат.
Пример: `cargo run -p physics --example gas_collapse` (безразмерные согласованные
единицы). Здесь пока нет сферической геометрии и общего шага с радиационным модулем.
Источники: [FLASH gravity](https://flash.rochester.edu/site/flashcode/user_support/flash_ug_devel/node130.html),
[USF infinite slab](https://gscommunitycodes.usf.edu/geoscicommunitycodes/public/geophysics/Gravity/Bouguer_slab.php).

`RadiatingGas::step_with_gravity` выполняет общий шаг самогравитации,
газодинамики и радиационного нагрева: gravity half-kick → radiation hydro →
gravity half-kick. `CoupledBudget` ограничивает размер внешнего подшага и
суммарное число гравитационных, газовых и тепловых шагов. Бюджеты общие для
всего вызова; ошибки откатывают и газ, и накопленный радиационный обмен.
`total_energy_with_gravity` считает газовую, потенциальную и вышедшую
радиационную энергию на площадь. С отражающими стенками это инвариант до
округления: тесты баланса охватывают сетки 40→80→160 и длинное облучённое
сжатие длительностью 1 s. Для открытых границ к нему добавляется накопленный
`CoupledWork::escaped_gas_energy`, включающий газовую энергию и гравитационный
обмен через границу.
Запуск единого примера: `cargo run -p physics --example radiation_hydro -- --gravity`.
Демонстрационный G=1000 выбран для заметной гравитации в небольшом столбе;
это не параметры реальной звезды. Сферическая геометрия, ядерные реакции и
эволюция химического состава остаются отдельной незавершённой частью.

Консервативная работа через перенос массы следует дискретному тождеству для
квадратичной энергии: ΔU = average potential · Δmass. Общий контекст методов
консервативной самогравитации: [Jiang et al.](https://arxiv.org/abs/1208.1795).
Реализация здесь использует изолированный плоский Green kernel, а не трёхмерную
схему Athena из статьи.

## Inertial secondary motion

`physics::secondary_motion::SecondaryMotion` is a reusable three-axis linear
spring attached to an accelerating support. Configure natural frequency in Hz
and damping ratio (0: undamped, 1: critical). `step(dt, anchor_acceleration,
external_acceleration)` integrates the damped oscillator analytically while
acceleration is constant over that step. Frequency is independent of particle
mass because it represents stiffness divided by mass. Gravity supplied as
external acceleration produces static sag; zero external acceleration centres
motion on the authored rest shape.

`offset()` and `velocity()` are relative to the attachment. `configure` preserves
state when changing response, and `reset` clears displacement and velocity.
Invalid configuration/steps and nonfinite results preserve state. Steps must be
in (0, 0.1] seconds. Caller acceleration must use consistent world axes; rotating
frames, contact, nonlinear tissue response, and surface-volume preservation are
not supplied by this oscillator. Exact integration does not make rapidly varying
input acceleration timestep-independent.

The actual anatomical `female_motion` example now uses four library oscillators
with smooth local surface weights. This is a secondary-motion approximation,
separate from the nonlinear skin shell and tetrahedral `Tissue` solver.

## Взаимная гравитация на GPU

`voxy_gpu::GravityProgram` выполняет взаимную ньютоновскую гравитацию с Plummer
softening, однородным ускорением и интегратором velocity-Verlet. Это явно выбранный
формат `f32`: исходный CPU-решатель `physics::gravity` с `f64` не переключается
автоматически. Начальные `GravityBody` загружаются один раз; `encode_steps`
записывает несколько шагов над резидентными данными, без обмена с CPU между ними.

Три отдельных прохода вычисляют предсказанное состояние из неизменяемого снимка,
корректируют скорости по новым позициям и публикуют результат. Общий атомарный
флаг ошибки запрещает публикацию всей системы при сингулярной паре или
переполнении. Ошибка остаётся в задаче; для восстановления нужно создать новую
задачу. Проверка и получение результатов асинхронны через `encode_readback`,
`begin_read` и `try_read`. Результат выдаётся один раз. GPU-буфер доступен для
следующих проходов; записи тела начинаются с байта 32 и состоят из двух `vec4`:
позиция/масса и скорость/нулевой padding.

Бюджет по умолчанию: до 4096 тел и 256 шагов на один вызов `encode_steps`.
Размеры и пределы устройства проверяются перед выделением памяти. Алгоритм
попарный, O(n²); ускорение относительно CPU не измерено. Контакты сфер,
характеры, жидкости и деформируемые тела пока не интегрированы с этим GPU-путём.

`cargo run -p voxy_gpu --example gravity_smoke` проверен на Apple M4 Max Metal:
257 тел за 128 шагов сравнивались с CPU `f64` с допуском 0.0002; максимальное
расхождение составило 0.0000259126064. Орбитальный тест из 2560 GPU-шагов дал
относительный дрейф энергии 8.93e-7, при допуске 0.001; траектория совпала с CPU
в пределах 0.001, импульс — 1e-6. Ошибки после предсказания и переполнение
сохранили исходные данные побитно. Проверены бюджеты и диапазоны readback.
Подключение этого решателя к игровому циклу и выполнение на остальных
аппаратных бэкендах остаются обязательной незавершённой работой.

`Gas::advance` возвращает интегральный перенос массы через каждую грань и
подписанные выходящие массу/импульс/энергию. `step` сохраняет прежний интерфейс.
`SelfGravitatingGas::advance` дополнительно возвращает `escaped_mass` и
`escaped_energy`, включая average potential × граничный перенос массы.
Внутренняя гравитационная работа распределяется между соседями по фактическому
переносу массы; граничная часть возвращается отдельно. `CoupledWork` аналогично
отчитывается о газовой массе и энергии, отдельно от радиационного счётчика.
Проверены уход расширяющегося газа, сквозной поток и облучённый открытый столб:
остаток + подписанный выход сохраняет массу и дискретную полную энергию.
Выходящий газ удаляется из моделируемого поля: его дальнейшая динамика и
воздействие снаружи домена не вычисляются. Граничный энергетический обмен
использует дискретный средний потенциал ячейки; это баланс численной модели,
а не доказательство точности движения вне домена.

`astrophysics_spherical::Sphere` решает радиальные уравнения Эйлера на неподвижных
сферических оболочках. Объём = 4π(b³−a³)/3, площадь граней = 4πr²; геометрический
источник импульса p(Aout−Ain)/V точно компенсирует поток постоянного давления.
Центральная грань имеет нулевую площадь и не переносит массу/энергию; наружная
граница отражающая или открытая, периодическая отклоняется.
Самогравитация использует заключённую массу и точное усреднение −GM(r)/r² по
объёму ячейки. Потенциал и энергия связи включают собственную энергию каждой
кусочно-однородной оболочки; ноль потенциала выбран на бесконечности.
Полиномы толщины оболочек избегают вычитания близких степеней радиуса.
Гравитационная работа следует фактическому переносу массы через сферические
грани, с учётом среднего потенциала и открытого граничного обмена.
`step` адаптивно ограничивает CFL и гравитационный шаг, возвращает вышедшие
массу/энергию; при ошибке или бюджете всё состояние откатывается. Масса и полная
энергия + граничный обмен сохраняются до округления, но радиальный скалярный
импульс не является сохраняющимся декартовым векторным импульсом.
Проверены однородная сфера (U=−3GM²/(5R)), среднее поле, постоянное давление,
сжатие, открытая граница и атомарность. Пример:
`cargo run -p physics --example spherical_collapse` выводит CSV после сжатия.
Это первый порядок и полностью положительный газ без вакуумной поверхности;
пока без вращения, ядерных реакций и специально сбалансированной
дискретизации гидростатических политроп.
Контекст уравнений: [FLASH hydrodynamics](https://flash.rochester.edu/site/flashcode/user_support/flash_ug_devel/node104.html),
[FLASH self-gravity](https://flash.rochester.edu/site/flashcode/user_support/flash_ug_devel/node132.html).

## Coulomb friction after fracture

`physics::friction::Material::new(Kn, Kt, mu)` implements fixed-normal,
small-sliding penalty contact with stick/slip return mapping. Stiffnesses are
in Pa/m; mu is nonnegative. Pressure is `Kn * max(-normal_gap, 0)`. A tangential
elastic trial is projected onto the Coulomb disk `|traction_t| <= mu * pressure`.
The returned slip is stateful. A nonsymmetric consistent tangent includes the
normal-gap derivative of the friction limit. The implementation follows the
[penalty/backward-Euler friction formulation](https://doc.comsol.com/6.3/doc/com.comsol.help.sme/sme_ug_theory.06.088.html).
There is no different static/dynamic coefficient or velocity-dependent law.

`cohesive::Material::with_friction(mu, Kt)` enables the same law on its paired
faces once complete fracture has been accepted. Until that point the cohesive
shear law transmits traction and the inactive friction reference follows the
jump. This avoids adding a second tangential spring to an intact interface or
creating frictional preload at activation. This is a fully-broken-face contact
model, not gradual friction transfer on partly damaged interfaces. Default
cohesive interfaces retain their original frictionless behavior.

Closed contact stores tangential penalty spring energy. Sliding increments
accumulate nonnegative work `mu * pressure * slip_increment`, separately from
Gc-based fracture dissipation. On opening, contact carries no traction, its
slip reference follows free relative motion, and released tangential spring
energy is recorded separately. Recontact does not inherit a spring stretched
by motion while the faces were apart. A dynamics integrator must account for
released energy; this static material does not turn it into fragment velocity.

For a closed-contact update, endpoint traction work differs from elastic-energy
change plus friction work by `Kt/2 * |delta elastic_gap|²`. This backward-Euler
endpoint-work defect is reported as numerical dissipation, not physical friction
heat. The defect tends to zero with refinement. Under constant pressure,
proportional sliding, trapezoidal work with a resolved stick/slip transition
matches elastic energy plus physical friction work. Large pressure/history steps
are not exact integrations of a continuous loading path. Energy fields in
`InterfaceReport` keep fracture work, friction work, numerical defect and released
energy separate; stored energy includes the active friction spring.

Run `cargo run -p physics --example friction_coupon`. It fractures the same two
cubes as `fracture_coupon`, closes the plane, prescribes uniform normal
compression and tangentially loads the outer faces. Normal displacements are
constrained throughout shear loading; this isolates a homogeneous shear/contact
experiment. Pressure is 33.333333 kPa, mu = 0.5, area = 0.01 m². The shear reaction
saturates at 166.666667 N. At 0.05 mm outer-face slide the physical friction work
is 0.004444444 J. The numerical defect is printed independently. The example
fails if equilibrium or the analytical force relation is missed; no window opens.
Constants are illustrative, not a calibrated friction pair.

Tests `friction` and `friction_mesh` cover stick/slip saturation, constant-pressure
work, the endpoint-work identity, pressure unloading, reversal, opening/recontact,
mu = 0, nonsymmetric tangent derivatives, tensor-frame covariance, history
atomicity and diminishing numerical defect. The slab force/work/history checks
match on 12 and 96 tetrahedra. A further slab test leaves normal and shear
interface DOFs free, reaches actual sliding, and verifies the coupled global
force residual and integrated Coulomb bound. These tests do not validate large sliding,
changing normals, general contact search, dynamic impact, temperature, wear,
frictional heating or finite-rotation detached-fragment motion. Those requirements remain open.

`astrophysics_spherical_radiation` выполняет серый LTE-перенос по полным хордам
сферы. Постоянные оболочки задаются наружным радиусом, поглощением и температурой;
луч проходит дальнюю и ближнюю половины каждой пересечённой оболочки. В каждом
радиальном кольце диска выбираются midpoint-сэмплы по impact parameter²; поэтому
центральная область не теряется даже при малом числе лучей. Вес = площадь кольца
× 4π, даёт полную светимость для сферически симметричного изотропного окружения.
`transfer` возвращает чистую светимость W и тепловую мощность каждой оболочки W;
их сумма с противоположным знаком совпадает до округления. Угловая точность
зависит от числа лучей на кольцо; стоимость O(N²Q), общий бюджет сегментов
проверяется до трассировки. Проверены аналитическая однородная сфера, толстый
предел, LTE, прозрачность, центральное покрытие и угловая сходимость.
`Sphere::radiation_rates` выводит температуры и поглощение из состояния газа;
`Sphere::radiate` меняет только тепловую энергию, сохраняя плотность и импульс.
Явный адаптивный шаг ограничивает изменение тепловой энергии и производную
испускания; общий бюджет лучевых сегментов действует на все тепловые подшаги.
При любой ошибке сфера откатывается. Остаток газовой энергии + возвращённая
`escaped_energy` сохраняется; изотропное внешнее облучение может делать её отрицательной.
Пример: `cargo run -p physics --example spherical_radiation` (охлаждение за 1 s).
Пока нет рассеяния, частотной зависимости, доплеровских поправок, радиационного
давления и ядерного горения.
Геометрия p-rays: [Puls, stellar atmospheres](https://www.usm.uni-muenchen.de/~puls/stellar_at/stellaratm_imprs25_lecture_compressed.pdf).

`astrophysics_evolution::RadiatingSphere` связывает сферическую гидродинамику,
самогравитацию и лучевой радиационный нагрев: hydro/gravity half-step →
radiation → hydro/gravity half-step. Температура, плотность и поглощение
пересчитываются из актуального газа на каждом шаге. Симметричное расщепление
не повышает первый порядок компонентных схем; точность требует уменьшения
внешнего шага, сгущения радиальной сетки и угловой квадратуры.
Внутренний бюджет общий для всего вызова: внешние, гидродинамические и тепловые
шаги плюс точное число лучевых сегментов. Ошибка возвращает весь объект к
начальному состоянию, включая накопленный выход массы/газа/излучения.
`energy()` включает газовую и гравитационную энергии плюс signed cumulative
выходы газа и излучения. Проверены совместный баланс с отражающей/открытой
границей, поздний отказ бюджета и точность подсчёта сегментов.
Пример `cargo run -p physics --example spherical_evolution` моделирует 0.01 s
движения и охлаждения демонстрационной сферы. G=1000 демонстрационный,
модель не калибрована на реальную звезду. Это динамическая тепловая эволюция,
но пока без ядерных реакций, изменения химического состава, переноса конвекцией,
вакуумной поверхности и физически зависимых от состояния опакностей/EOS.

`astrophysics_eos::Mixture` описывает полностью ионизованный невырожденный
одноатомный газ и захваченное LTE-излучение. Состав — массовые доли отдельных
ядер с A и Z; inverse μ = sum X(1+Z)/A, inverse μe = sum XZ/A.
Массы ядер приближены A×atomic mass constant, без массового дефекта и энергии
ионизации. Pgas=ρkT/(μmu), Prad=aT⁴/3, internal energy density=1.5Pgas+aT⁴.
Возвращаются удельная теплоёмкость и ньютоновская адиабатическая производная
скорости звука с радиацией; инерция излучения не включена, поэтому релятивистский
предел не поддержан. `temperature` инвертирует энергию монотонной бисекцией.
Проверены чистые H/He и смесь, обращение энергии в T, производная cv, газовый
и радиационный пределы звука, некорректный состав/overflow.
Пример: `cargo run -p physics --example stellar_eos` (таблица при ρ=1 kg/m³).
EOS подключён к `Sphere::step_ionized`: давление и скорость звука используют
газовую + радиационную термодинамику. `RadiatingSphere::step_ionized` также
использует этот EOS для температуры и радиационного нагрева. Режим `step`
с постоянными γ/cv сохранён. Радиационная энергия хранится в полной внутренней
энергии ровно один раз.
Источники: [Princeton EOS](https://www.astro.princeton.edu/~burrows/classes/403/eos.opac.pdf),
[NIST CODATA 2022](https://physics.nist.gov/cuu/pdf/all.pdf).


### CUDA f64 N-body (2026-10-01)

`voxy_cuda::CudaGravityJob` реализует velocity-Verlet с f64 и постоянными
буферами устройства. Три фазы predict/correct/commit сохраняют исходное состояние
при сингулярности или переполнении; ошибка остаётся до пересоздания задания.
`step` не копирует данные на CPU, `read` возвращает только опубликованные тела.
По умолчанию: до 4096 тел и 256 шагов за вызов, дополнительно действует бюджет
памяти `CudaCompute`. NVRTC отключает fast math и свёртку FMA.

Проверка арифметики исходника дала точное совпадение с CPU f64 для 257 тел
за 128 шагов; прошли орбита и откат ошибочного состояния. Настоящие NVRTC 12.6
и PTXAS проверили компиляцию для sm_52/sm_75/sm_89. Выполнение на NVIDIA
не проверено: локально `DriverUnavailable`. Команда аппаратной проверки:
`cargo run -p voxy_cuda --features cuda --example gravity_probe`.

`Sphere::step_ionized(dt,max_step,budget,mixture)` принимает фиксированный
ионизованный состав. Для каждого состояния E−kinetic содержит газовую и
захваченную радиационную энергию, T восстанавливается через EOS, а поток и
геометрический источник используют Pgas+Prad. CFL использует адиабатическую
скорость звука того же EOS. Газовый предел воспроизводит γ=5/3; постоянное
радиационное давление не вызывает ложного движения, градиент создаёт движение.
Самогравитационная работа и открытый граничный обмен сохраняют прежний точный
дискретный энергетический баланс. Ошибки EOS/бюджета полностью откатывают сферу.
Значения звука ≥c отклоняются, но это ньютоновская схема без инерции излучения;
физическая применимость требует существенно меньших скоростей и невырожденного
полностью ионизованного газа с захваченным LTE-излучением. `gamma` остаётся
валидным параметром старого интерфейса и диагностической проверки положительности,
но не определяет давление/скорость в новом режиме.
Пример: `cargo run -p physics --example radiation_pressure` (демонстрационные
параметры и искусственный G, не физическая калибровка конкретной звезды).
Для радиационного нагрева нового режима нужно использовать
`radiate_ionized`, а не fixed-cv `radiate`.

`Sphere::radiation_rates_ionized` и `radiate_ionized` восстанавливают температуру
из газовой + захваченной радиационной энергии через тот же EOS, что гидродинамика.
Локальная мощность ray transfer изменяет полную внутреннюю энергию, отдельного
дублирующего запаса radiation energy нет. Адаптивный шаг использует положительную
газовую теплоёмкость смеси как консервативную нижнюю оценку; radiation cv уже
учтена в нелинейном обращении энергии в T. Полная энергия остаётся инвариантом
с подписанным выходом через поверхность, ошибки/бюджеты полностью атомарны.
`RadiatingSphere::step_ionized(dt,budget,mixture)` соединяет новый EOS во всех
частях общего шага; состав фиксирован. Старое поле `specific_heat` не определяет
физику этого режима, но остаётся валидным для совместимости с `step`.
Проверены охлаждение без двойной энергии, масса/полная энергия, LTE-равновесие,
независимость от старого cv и поздний откат бюджета.
Пример: `cargo run -p physics --example spherical_evolution -- --ionized`.
Параметры демонстрационные: физическая применимость полного LTE-EOS требует
захваченного излучения, а лучевой перенос остаётся квазистатическим и серым;
это не универсальная модель optically thin плазмы или калиброванная звезда.
Ядерное горение, перенос состава, ионизация, вырождение и реалистичные опакности
в этот шаг пока не входят.

Интерактивная GPU-орбита: `cargo run -p voxy_app --example gpu_gravity`.
Физика и vertex shader используют один постоянный буфер, без копирования тел
через CPU в игровом цикле. Space — пауза, R — сброс, Escape — выход.
Metal smoke проверяет 60 кадров, resize, паузу, сброс и 200 шагов. Контакты
сфер и планетарный персонаж остаются в существующем CPU-демо `gravity`.

## Inertial solid dynamics and detached components

`plasticity::mesh::DynamicBody` wraps the same tetrahedral body and material
histories with lumped nodal masses and velocities. Constructor densities are
per reference tetrahedron in kg/m³; each cell mass is divided equally among
its four nodes. Velocities use m/s. Negative/zero/nonfinite densities,
unrepresentable masses and invalid counts fail before constructing a body.

The dynamic step solves

```
m (v_new - v_old)/dt + (f_internal_old + f_internal_new)/2 = f_external_mid
x_new - x_old = dt (v_old + v_new)/2
```

The implicit tangent contains `2m/dt² + K_endpoint/2`. External forces and
uniform acceleration are sampled at the midpoint; prescribed displacements
are endpoint values relative to reference geometry. Initial acceleration is not
required. The linear elastic scheme is the trapezoidal/average-acceleration
method, related to [Newmark beta=0.25, gamma=0.5](https://mooseframework.inl.gov/moose/source/timeintegrators/NewmarkBeta.html).
It preserves unforced linear elastic energy with fixed supports and has no
artificial damping in that regime. Large steps can still have large phase error;
unconditional linear stability does not guarantee temporal accuracy.

`step(dt, loads, acceleration, prescribed, max_iterations, tolerance_n,
energy_tolerance_j)` requires both force convergence and an energy ledger
within the requested tolerance. The ledger includes kinetic, bulk elastic and
hardening energy, plastic/fracture/friction work, interface stored energy and
unresolved contact spring release. It subtracts external and constraint work.
Friction's numerical endpoint-work diagnostic is not counted as physical heat.
A force-converged candidate that violates the energy tolerance returns
`converged=false` with `energy_defect_j`; it does not publish geometry, velocity
or any material/contact history. Invalid, inverted, nonfinite or singular
trials also leave the accepted state unchanged. The caller must retry with a
smaller step; there is no hidden timestep enlargement or unbounded work.
Prescribed motion must be smooth and initial velocities consistent with its
constraints; abruptly activating a pin is not an impact model.

`diagnostics()` reports mass, momentum and separate energy terms.
`fragments()` returns connected node groups with mass, centre of mass, momentum
and centre velocity. Tetrahedra join their own nodes; any remaining cohesive
bond joins paired faces. Fully broken contact pairs remain separate fragments
even when closed. These are deformable components, not a rigid-body replacement.

Run `cargo run -p physics --example dynamic_fracture`. Two free 1 kg cubes start
at velocities -0.5 and +0.5 m/s with 0.25 J total kinetic energy. Their preinserted
0.01 m² plane costs 0.1 J to fracture. At dt = 5 microseconds and t = 1.6 ms,
the cubes are separate components travelling at approximately -0.380047 and
+0.380047 m/s. Kinetic energy is 0.145367432 J and bulk elastic energy is
0.004643381 J; total ledger defect is 1.081262e-5 J. Total momentum stays near
zero. Parameters are illustrative; the example is numerical and opens no window.

The demonstration first rejects a 0.2 ms step that jumps directly from zero
traction to complete failure: force residual is small, but missing the peak
would create 0.1 J of unaccounted fracture work. The energy guard detects that
failure and preserves the intact initial state.

`solid_dynamics` tests cover exact uniform free fall and gravity work, lumped
mass, 1000 undamped elastic-oscillator steps, second-order linear phase refinement,
invalid/inverted/unconverged/energy-rejected step atomicity, fragment connectivity,
fracture work and momentum. The free-fracture energy defects at 20, 10 and
5 microseconds are respectively 2.357577e-4, 2.151727e-5 and 1.081262e-5 J.
This is timestep evidence on one coarse mesh, not fracture trajectory mesh
convergence or experimental validation.

The bulk law still assumes small strain in a fixed material frame. Finite
rotations/tumbling require a corotational or finite-strain formulation. Normals
and contact pairing remain fixed; there is no CCD, general fragment collision
search, adaptive crack insertion or voxel-world coupling. Released friction
spring energy is recorded as an unresolved reservoir rather than converted into
fragment velocity. Nonlinear plastic/friction/fracture trajectories require
both timestep and mesh checks; the force/energy guard alone is not a proof of
physical accuracy. The dense solver currently retains a 512-vertex bound; intrinsic cohesive expansion has its separate 32-cell cap.

`astrophysics_nuclear` добавляет реакционную сеть с REACLIB-параметризацией:
сумма экспонент семи коэффициентов, T9=T/1e9. Для реакции порядка k поток
пропорционален ρ_cgs^(k−1) × произведение (X/A)^count / произведение count!.
Публичная плотность в kg/m³ переводится в g/cm³; энергорезультат возвращается
в J/kg через Avogadro×1000. Заданные диапазоны температуры обязательны;
экстраполяция не выполняется. Коэффициенты и подходящий диапазон задаёт вызывающий.
Сеть проверяет сохранение A и Z в каждой реакции; это сильные реакции, без β-процессов.
Гидродинамические массы — baryon mass без mass defect. Отдельный отрицательный
binding reservoir вместе с теплом и выходом нейтрино сохраняет энергию;
Q выводится из разности заданных binding energies, не независимая произвольная
энергия реакции. Endothermic реакция отнимает тепловую энергию. Неотрицательная
neutrino_fraction относится только к положительному release и должна соответствовать
физическим данным реакции (для triple-alpha в примере равна нулю).
`burn` удерживает температуру и плотность: явный совместный шаг ограничивает
gross расход каждого топлива до 5%, общий бюджет шагов/fit evaluations действует
на весь вызов. Любая ошибка откатывает состав, без обрезания или нормировки долей.
Тесты используют синтетические скорости для аналитического закона расхода,
проверяют фактор плотности/symmetry, energy reservoir, диапазон и поздний откат.
`cargo run -p physics --example helium_burn` использует три физических fy05
компоненты из распространяемого pynucastro REACLIB и Q=7.275 MeV. Состав He4→C12,
ρ=1e8 kg/m³, T=2e8 K удержаны; в примере разрешена только эта температура,
что не заявляет универсальный диапазон пригодности fit. Binding reference He4=0,
C12=Q — сдвиг нуля энергии, согласованный с этой реакцией. Это не самосогласованная
звёздная эволюция: тепловую обратную реакцию, перенос состава и EOS ещё нужно связать.
Источники: [JINA format](https://reaclib.jinaweb.org/help.php?intCurrentNum=0&topic=rate_fitting),
[pynucastro REACLIB data](https://github.com/pynucastro/pynucastro/blob/main/pynucastro/data/reaclib_default2_20250330).

`Network::burn_isochoric` couples the supplied reaction network to the fully
ionized gas plus trapped-radiation EOS at fixed density. It updates mass
fractions, deposits reaction heat into specific internal energy (J/kg), rebuilds
molecular weights, and inverts the EOS for the next temperature. The complete
call is transactional, including late budget failures. The shared budget counts
all underlying reaction steps and coefficient-set evaluations.

This is explicit one-zone thermal feedback: rates are held during each outer
interval bounded by `Budget::max_step`. Stiff burning requires timestep
convergence; this method is not an implicit stellar burning solver. Neutral
nuclei remain unsupported by the ionized EOS. Nuclear tests cover energy plus
binding reservoir plus neutrino escape, temperature-sensitive rate feedback,
step refinement, and rollback. Spatial composition transport and coupling this
burn to spherical hydrodynamics remain pending.

`Sphere::step_composition` transports passive species mass fractions using the
actual integrated radial mass flux at each hydrodynamic substep. A face takes
its composition from the donor shell; outflow uses the last shell composition.
The returned vector records signed escaping mass for each species. Species
masses are conservative, including boundary exchange, without fraction floors
or renormalization. A negative transported species is an error and rolls back
both gas and composition. This API currently uses the fixed-gamma gas EOS;
nuclear burning coupling remains pending.
The spherical tests check a composition discontinuity, nonnegative unit-sum
fractions, species conservation for reflecting and open boundaries, and late
budget rollback.

`DynamicBody::advance_free` advances a whole interval with constant external
forces and acceleration, without prescribed supports. `AdvanceLimits` bounds
Newton iterations, attempted substeps and the minimum timestep. Force- or
energy-rejected candidates are bisected; absolute accepted energy defects consume
one interval-wide budget. All substeps remain private until the whole interval
succeeds, so a work/minimum-step limit or constitutive error rolls back the entire
interval. Constitutive errors are reported directly rather than silently retried.
This is an energy-ledger guard, not an error estimator for phase or trajectory;
time refinement remains required. Moving supports and time-varying loads need
explicit midpoint sampling through `step`.


`Sphere::step_composition_ionized` uses each shell's transported fractions to
rebuild its fully ionized gas plus trapped-radiation EOS on every hydrodynamic
substep. The network supplies mass numbers and charges; this method does not
execute reactions. The same local pressure and sound speed set geometric
pressure forces, interface fluxes, and CFL limits. Final local states are
validated before gas and composition are committed together. The test checks
exact agreement with the uniform-mixture API, composition-driven pressure
forces at initially equal density and internal energy, closed-domain energy
conservation, and rollback. Neutral species remain unsupported by this EOS.

For a maximum-normal-stress onset envelope,
`Stress::normal_strength_utilization(tensile_pa, compressive_pa)` returns
`max(max(sigma_1,0)/tensile_pa, max(-sigma_3,0)/compressive_pa)`, where the
principal Cauchy stresses are ordered descending and tension is positive.
Unlike von Mises yielding, this detects hydrostatic tension or compression;
independent strengths permit different tensile and compressive thresholds.
The diagnostic is rotationally invariant and does not itself create damage,
change plastic history, or insert cracks. Multiaxial material calibration is
still necessary before using this envelope for a particular brittle material.
The `strength` example now prints both illustrative onset estimates.

`Stress::principal_directions` contains orthonormal spatial eigenvectors in
columns, ordered with descending `principal_pa`. The stress tensor reconstructs
as `Q diag(principal_pa) Q^T`; each column gives a plane normal with zero shear
traction. The Jacobi rotations accumulate the directions along with the values,
so directions rotate with the body. Eigenvector signs are arbitrary. Repeated
principal stresses select a subspace, not a unique crack normal: hydrostatic
stress must not be assigned a physically preferred fracture orientation from
these axes alone. Tests reconstruct general, shear, repeated and zero tensors
across stress scales from 1e-200 to 1e200, and verify rotational covariance for
distinct eigenvalues. The `strength` example prints the axes.

`Sphere::step_reactive` couples local composition-dependent spherical
hydrodynamics and gravity to the nuclear network: half a hydrodynamic interval,
a full isochoric burn in every shell, then the second hydrodynamic half.
Composition advection uses the same mass flux as gas transport. Nuclear heat
changes internal energy without changing momentum; subsequent fluxes use the
new local EOS. Burn steps and coefficient evaluations share one budget across
all shells, and both hydrodynamic halves share their step budget. Gas and all
fractions commit together only after every phase succeeds.

`reactive_energy` includes gas, gravitational binding and the nuclear binding
reservoir. `ReactiveExchange` returns escaping gas/gravitational energy, signed
nuclear binding energy carried by escaping species, deposited heat and escaping
neutrino energy in joules. Deposited heat is already in the gas and must not be
added again to the conserved total. Add the three escaping-energy terms to the
remaining `reactive_energy` to close its ledger. The synthetic triple-alpha test
checks this balance for closed and open spheres and verifies late burn-budget
rollback. It is an algorithm test, not a stellar rate calibration.

The split interval `dt` and nuclear `max_step` both need convergence studies
for a chosen model; the method is explicit and does not promise second-order
accuracy because the constituent solvers are first-order. The standalone reactive method excludes escaping photon radiation; use
`step_reactive_radiating` below to include it.

`plasticity::mesh::Body::with_cohesive_faces` converts a shared-node tetrahedral
mesh into an intrinsic cohesive mesh: each cell receives four independent solver
vertices and every two-sided interior face receives a matched zero-thickness
interface. Boundary faces remain free. Source node and cell mappings are returned
in `CohesiveMesh`; replicate prescribed constraints onto copies, but partition
original nodal loads rather than applying the same force to each copy. Density
stays per cell, so duplication does not multiply total mass. Face winding is
chosen from the incident cell geometry; nonmanifold and overlapping neighbors
are rejected. The current dense limit permits at most 32 source cells after
expansion. Finite interface stiffness adds initial compliance, and cracks follow
mesh facets; this does not yet represent arbitrary within-cell crack paths.
A two-tet test verifies automatic bonding, zero bulk strain under separate rigid
translations, fracture work `Gc*area = 5 J`, and two independent fragments after
failure, each retaining its original reference mass.


`Sphere::radiate_composition` derives each shell temperature from its current
network composition and total gas plus trapped-radiation internal energy.
The minimum gas heat capacity across the shells supplies a conservative thermal
step bound. It uses the existing grey LTE full-chord transfer with constant
mass opacity, without assuming a uniform molecular weight.

`Sphere::step_reactive_radiating` composes half a radiative thermal interval,
a full reactive hydrodynamic interval, and the second radiative half. The
second half uses the changed composition after burning and transport. Radiation
steps and ray segments are shared across both halves; all gas and fractions
roll back if any phase fails. Add `result.radiation.escaped_energy` and the
three escaping-energy terms in `result.dynamics` to `reactive_energy` for the
full ledger. The test verifies this balance and a ray-budget failure after
burning has already succeeded internally.

This remains a first-order, explicit, Newtonian spherical model with supplied
reaction rates and constant grey opacity. Its transfer has no scattering,
frequency dependence, finite photon propagation time or radiation momentum
force. This coupled interface is not a calibrated stellar evolution model.

Run `cargo run -p physics --example reactive_star` for the complete coupled
model, or pass `-- --dt 0.005` to halve its outer interval. It evolves sixteen
shells with Newtonian gravity, local ionized gas/radiation EOS, composition
advection, synthetic temperature-dependent triple-alpha burning, grey photon
transfer and an open gas boundary. CSV includes time, central density,
temperature and carbon fraction, accumulated photon/neutrino losses and the
complete relative energy residual, including gas and nuclear boundary energy.
The output includes the initial and final samples.

The numerical demonstration deliberately uses an artificial reaction rate and
binding release; it is not a calibrated helium star. A one-second run with
0.01 s intervals reached central T=201199536.6 K and carbon fraction
0.0074612565. Halving the interval changed these to 201199532.6 K and
0.0074612395; the energy residual was at roundoff in both runs. This single
refinement comparison is evidence for this demonstration, not a general
convergence guarantee. The example's CSV goes to stdout and a concise energy
residual to stderr.

For source-node input, `CohesiveMesh::split_nodal_forces` partitions each nodal
force equally among its coincident solver copies, preserving total reference
force and moment; `expand_constraints` replicates prescribed displacements.
Both validate input counts, finiteness and mapping indices. Equal force
partition is a stated loading convention, not a face-traction integration rule.
Integrate actual boundary tractions per face, and use density-weighted uniform
acceleration for gravity instead of copying a source nodal force to every cell.
A load-controlled two-tet test keeps interface nodes free, transmits 100 N
through the automatically inserted bond, and checks Clapeyron's linear energy
identity including interface compliance. An apex-loaded tetrahedron has
nonuniform face traction, so a uniform-bar series compliance is not its reference
solution.

`astrophysics_reaclib::parse` reads a selected reaction in fixed-width REACLIB
text, including repeated chapter markers and additive coefficient sets. It
preserves reactant/product names, six-character labels, reverse flag and Q in
MeV. Coefficients use thirteen-character fields; Fortran D exponents are
accepted. Input is ASCII, finite and bounded by an explicit set budget. Chapters
with more than three incoming or outgoing nuclei are rejected, as are weak
rates and mixed reaction/source/Q records. This is a selected-rate reader, not
an importer of a complete multi-reaction library. Callers still map names to
network species and supply consistent binding energies and neutrino treatment.

Temperature validity is deliberately supplied by the caller rather than
inferred from this file format. `helium_burn` now loads three original fy05
records from the test fixture rather than maintaining duplicate coefficients;
its domain remains restricted to its held 2e8 K demonstration point.
The fixture was extracted from the
[pynucastro REACLIB dataset](https://raw.githubusercontent.com/pynucastro/pynucastro/main/pynucastro/data/reaclib_default2_20250330).
The fixed-width field layout and chapter interpretation follow the
[pynucastro reader](https://github.com/pynucastro/pynucastro/blob/main/pynucastro/rates/reaclib_rate.py).
Tests verify original coefficients, summed rates, stoichiometric names and Q,
plus malformed, truncated, weak, mixed, non-ASCII and over-budget input.

`Body::exposed_faces` extracts current outward-oriented tetrahedral boundary
triangles. Shared-node internal faces and paired interfaces with any remaining
bond are hidden; a fully fractured pair contributes both separate faces.
`SurfaceFace` reports owning cell, solver vertices, unit normal and current area.
These are topological surfaces, not a fluid-access or contact-occlusion model.
`Body::pressure_loads` integrates one constant pressure per exposed triangle in
that deterministic order, using area/3 nodal shape-function weights. Positive
pressure points inward and negative pressure is outward suction. Force counts,
nonfinite input and overflow are rejected. Pressure geometry is sampled at the
accepted configuration: follower-load derivatives are not included in Newton,
and callers must resample when geometry changes. Tests verify zero resultant
and moment for uniform closed-surface pressure, the analytical `-p*dV/dx` forces
of an axis-aligned tetrahedron, face center-of-pressure moments, and six exterior
faces becoming eight after the automatically bonded two-tet mesh fractures.

`Rate::reaction` connects a parsed rate to a network using an explicit list of
species names in network order. It counts repeated reactants (including all
three helium nuclei), rejects unknown or duplicate mappings, and validates
baryon number and charge through the existing network validator. It compares
file Q against products-minus-reactants binding energy with a caller-specified
absolute tolerance in MeV. This prevents mixing a rate with an inconsistent
energy reservoir. Reverse records retain the supplied direction and Q; no
rate is synthesized from detailed balance. The method returns a reaction
without mutating the network. `helium_burn` now uses this checked mapping.
Tests cover reordered species, unknown names, duplicate names, inconsistent
binding energy and nonconserved charge.

The composition-dependent radiation regression also checks hydrogen, helium
and a mixed shell at equal temperature, despite their different internal
energies. With incident `blackbody(T)` intensity, they remain in radiative
thermal equilibrium. Invalid local fractions roll back the sphere. The
`Heating.ambient` parameter is an isotropic intensity in W/m²/sr, not a
Kelvin temperature; use `astrophysics_radiation::blackbody` to convert one.

### Finite-deformation elastic inertia

`biomechanics::InertialBody` wraps the existing objective finite-deformation FEM
body with reference-density lumped mass and velocity-Verlet dynamics. It accepts
only free nodes and time-independent constitutive potentials, rejecting pinned
bodies and viscoelastic history. Existing constant cavity-pressure and nodal
force potentials remain in `potential_j`; they must not be counted twice as
additional external work. Diagnostics report mass, linear momentum, angular
momentum about the world origin, kinetic and potential energy. `step(dt,
energy_tolerance_j)` evaluates old/new gradients, performs two half velocity
kicks around the position drift, and publishes all state only if the candidate
is valid and its absolute total-energy change meets the per-step guard. Explicit
stability and temporal accuracy still require a small timestep; the guard does
not establish global trajectory accuracy.

A freely spinning tetrahedron with E=100 kPa, nu=0.3, density=1000 kg/m³ and
initial angular velocity 10 rad/s rotates by more than 0.5 rad over 0.1 s while
conserving linear and angular momentum to the tested absolute 1e-10 tolerance.
Maximum relative energy deviations for 0.4 ms and 0.2 ms steps are respectively
3.4094e-6 and 8.5734e-7, consistent with second-order refinement. Reversing all
velocities retraces the discrete trajectory; a separate rotated-and-translated
coordinate-frame test compares full trajectories. This elastic wrapper is not
yet connected to the small-strain J2/cohesive fracture solver: large-rotation
plasticity, evolving crack normals and collisions between arbitrary fragments
remain outstanding.

`InertialBody::set_uniform_acceleration` adds a constant mass-weighted
acceleration (e.g. gravity) alongside existing nodal forces, and includes its
potential `-sum m*g·(x-X)` in the energy ledger. Changing the parameter returns
the potential-energy change at fixed geometry; account for that external
parameter work separately. Invalid/nonfinite or overflowing changes are atomic.
A zero-spin free-fall test reproduces ballistic positions and velocities over
100 steps, conserves kinetic-plus-gravitational potential energy, and checks
parameter work when gravity is removed. Run `cargo run -p physics --example
finite_fall` for a rotating elastic tetrahedron with simultaneous gravity and
CSV diagnostics of its center, velocity and energy.

`astrophysics_opacity::Table` stores positive mass-opacity values on a
rectangular density (kg/m³) and temperature (K) grid, with opacity in m²/kg.
Its immutable validated grid uses log-bilinear interpolation and returns
opacity plus local logarithmic density and temperature derivatives. Input
values are density-major. Boundaries are inclusive; queries outside the grid
return `OutsideDomain` rather than extrapolating. Zero opacity cannot be
represented by this logarithmic table. Tests recover arbitrary power laws
and derivatives at interior points and nodes, and reject invalid domains,
axes and table shape.

Each table identifies `GreyAbsorption`, `PlanckAbsorption` or `RosselandTotal`.
These are metadata, not conversions between opacity means. This module is
not yet wired into spherical thermal evolution and contains no calibrated
opacity dataset or composition interpolation. In particular, MESA opacity
tables use Rosseland means and a log-R/log-T coordinate convention; they
cannot be passed to this SI rho/T grid without conversion and a physically
appropriate transport closure. See the official
[MESA opacity overview](https://docs.mesastar.org/en/latest/kap/overview.html).

`Sphere::radiate_tabulated` now connects a `GreyAbsorption` table to the
composition-dependent thermal solver. It evaluates every shell's density and
EOS temperature on each thermal substep, then uses rho times tabulated opacity
as the ray absorption coefficient. The supplied scalar `Heating.opacity` is
superseded. The table-wide maximum opacity supplies the existing emission
step bound; the energy-change bound remains active. This is still an explicit
method and steep opacity variation requires timestep refinement. Initial and
final states must lie in the table domain; any domain/budget/thermal failure
rolls back the sphere. Rosseland and Planck tables are rejected by this grey
closure. Tests compare a constant table with the scalar path, check its energy
ledger, and verify domain-exit and wrong-kind rollback.

`Sphere::step_reactive_tabulated` also uses the table in both radiation halves
of the reactive dynamics wrapper.

`biomechanics::PlaneContact` adds a stationary frictionless halfspace to
`InertialBody` via `set_plane_contact`. The validated unit normal points into the
allowed halfspace `n·x >= offset_m`. A boundary node penetrating the plane stores
`0.5*k*gap²` and receives outward force `-k*gap*n`; interior nodes do not receive
contact springs. Stiffness is N/m per boundary node, so it is a mesh-dependent
penalty parameter, not a calibrated interface modulus. Scale it with refinement
and verify both timestep and penetration convergence. `contact_j` reports spring
energy already included in the total potential. Installing/removing the plane
returns parameter work at fixed positions and leaves state unchanged on overflow.
The fixed plane absorbs the normal impulse; tangential momentum is unchanged.

A 1 m/s tetrahedral impact with 10 µs steps rebounds conservatively. Raising
nodal penalty stiffness from 20 kN/m to 80 kN/m reduces peak penetration from
1.4550 mm to 0.72314 mm; maximum relative energy deviations are 8.71e-6 and
3.57e-5 respectively. Tests also compare each momentum increment with the
trapezoidal plane reaction impulse and verify timestep refinement. This is
frictionless elastic penalty contact against one infinite stationary plane;
there is no contact damping, adhesion, moving environment, general body/body
collision or continuous collision detection in this wrapper.


The coupled tabulated step shares the same budgets and energy ledger as
`step_reactive_radiating`. The second radiation half evaluates the density and
temperature after composition advection and nuclear heating. A table-domain
failure after burning rolls back all gas and composition changes; a regression
test verifies this and comparison against a constant-opacity table.
The table is fixed for the call and does not interpolate chemical composition.
Its data must therefore represent the intended composition approximation.

Run `cargo run -p physics --example reactive_star -- --tabulated` to use a
synthetic rho*T^(-3.5) opacity table in the complete demonstration. This is an
algorithm demonstration with artificial opacity and reaction rates, not a
calibrated stellar model; `--dt` remains available for interval refinement.

Run `cargo run -p physics --example finite_impact` for a coupled spinning
finite-deformation elastic impact, with 1 m/s initial center velocity and
10 rad/s spin. The example emits CSV samples of center motion, minimum gap,
plane reaction, contact energy and von Mises stress. It independently checks
cumulative reaction impulse against momentum change and bounds the observed
energy drift. At 80 kN/m nodal stiffness and 10 µs steps, this run reaches peak
penetration 0.89878 mm, maximum relative energy deviation 1.8372e-5, and reaction
impulse error 1.11e-16 kg m/s. By 60 ms the body has separated from the plane and
its center rises at 0.68485 m/s; elastic vibration remains, so this speed is not
an imposed restitution coefficient. This demonstration uses one coarse elastic
tetrahedron and does not establish experimental material fidelity.

Tabulated thermal steps additionally limit the estimated fractional temperature
change by `0.05 / (1 + maximum_temperature_slope)`. The bound is the maximum
absolute logarithmic temperature slope over every table interval and density
row, so crossing an interior table knot does not silently discard a steeper
nearby slope. Gas-only heat capacity supplies a lower bound on the full EOS
heat capacity. This remains a first-order explicit limiter, not an implicit
stability guarantee. Tests verify the global slope bound, increased substep
count for a steep table, and conservation of the resulting thermal energy.

### Spatial accuracy: unresolved bending stiffness

`cargo test -p physics --test beam_accuracy -- --nocapture` checks a 1 m
cantilever with a 0.1 m square section, E=10 MPa, nu=0 and 0.01 N transverse
end-face load. Nodal forces integrate a constant end traction; the reported tip
displacement is work-conjugate to that traction, and clamp reactions recover
the total force. Euler-Bernoulli bending plus the rectangular Timoshenko shear
term gives 4.024e-5 m. Shared-node linear-tet meshes of 4x1x1, 8x2x2 and 12x2x2
bricks give respectively 3.5271e-6, 1.0815e-5 and 1.6321e-5 m. The finest case
has 117 vertices and 288 tetrahedra but still reaches only 40.56% of the beam
reference displacement. The reference is a slender-beam approximation, not an
exact 3D boundary-value solution; that distinction does not justify the observed
large stiffness discrepancy. The test verifies force balance and a spatial
convergence trend, not adequate bending accuracy. Linear-tet bending stiffness
and the current dense vertex cap are material outstanding limitations for
realistic structural mechanics. Energy/momentum checks alone do not establish
spatial fidelity. A higher-order formulation and scalable assembly/solve need
validation against this benchmark and a converged 3D reference.

`Table::from_csv` loads external SI opacity grids with the exact header
`density_kg_m3,temperature_K,opacity_m2_kg`. Rows can be unordered; every
Cartesian density/temperature node must appear once. Missing, duplicate,
nonpositive and nonfinite nodes are errors. An explicit row budget bounds
accepted records. The caller specifies the opacity kind; a CSV header alone
cannot certify that data are grey absorption rather than a different mean.

Pass `--opacity-table /absolute/path/table.csv` to `reactive_star` to replace
the demonstration opacity with an external grey-absorption table. The table
must cover every queried density and temperature; an out-of-domain state
reports a failure instead of extrapolating. This option does not calibrate the
synthetic nuclear rate or certify the supplied physical dataset. Loader tests
cover unordered data, exact nodes, wrong-unit headers, missing/duplicate records
and row budgets. The coupled example has also been run using a CSV version of
its synthetic demonstration grid.

`reactive_star` accepts `--rate-file PATH --rate-min K --rate-max K` to replace
its synthetic nuclear fit with a selected REACLIB forward triple-alpha rate.
Both temperature limits are mandatory caller-supplied bounds, not an inferred
validity claim. The example checks the named reactants/products and maps them
through `Rate::reaction`; the file Q supplies the carbon-minus-three-helium
binding release. This capture uses zero neutrino escape. Missing bounds,
wrong reactions or a temperature-domain exit return errors. The nuclear option
can be combined with `--opacity-table`, but supplying coefficients alone does
not calibrate the initial stellar structure, opacity or equation of state.


The shared-node plastic FEM capacity has since been raised from 128 to 512
vertices. Gaussian elimination skips rows whose current pivot-column entry is
exactly structurally zero, preserving pivot selection and singularity checks.
Assembly/storage remains dense and quadratic in free DOFs; this change is not a
sparse or unbounded solver. The intrinsic cohesive builder keeps its separate
32-source-cell limit for now. Extending the same cantilever convergence test to
30x3x3 bricks (496 vertices, 1620 tetrahedra) gives 2.92885e-5 m, or 72.78% of the
4.024e-5 m beam reference, improving on 40.56% at 117 vertices. The remaining
roughly 27% stiffness discrepancy is still unacceptable as proof of accurate
bending. This evidence motivates higher-order interpolation and genuinely
scalable sparse assembly/solves rather than interpreting conservation tests as
spatial accuracy.

The imported-reaction coupled regression loads original fy05 text, maps the
triple-alpha reaction through the checked Q/stoichiometry interface, reads a
nonconstant synthetic opacity CSV, and evolves an open gravitating sphere for
0.2 seconds. It checks the cumulative energy ledger after every interval,
positive carbon production and zero prescribed neutrino loss for this capture.
Runs with 0.01 and 0.005 second intervals agree within 1e-6 relative central
temperature and 1e-3 relative central carbon abundance. These are regression
tolerances for the numerical scenario, not observational calibration or a
claim that the chosen temperature-domain bounds are certified by the rate source.

`optical_depth_radius(shells, target_depth)` integrates radial absorption
inward from the model's outer boundary and locates the requested depth inside
a constant-opacity shell. It returns `None` if the central radial depth never
reaches the threshold, rather than inventing a photosphere for an optically
thin model. Transparent outer shells and a crossing at the centre are handled.
This radial diagnostic is distinct from the full-chord image formation problem.
Tests verify uniform-sphere and layered analytic radii, thin models and invalid
opacity. A target of 2/3 corresponds to the Eddington grey convention where
T equals effective temperature; this convention is explained in the official
[MESA atmosphere documentation](https://docs.mesastar.org/en/latest/atm/t-tau.html).
No atmospheric hydrostatic boundary condition is imposed by this diagnostic.

`Sphere::composition_optical_depth_radius` builds the diagnostic directly from
the current gas and composition. It accepts constant mass opacity or a grey
absorption table evaluated at local density and EOS temperature. It is read-only
and does not solve transfer. Tests compare both paths against the analytic
radius and verify the optically thin case without changing gas state.
`reactive_star` now includes `radial_tau_two_thirds_radius_m` in its CSV. An
empty field means the radial depth never reaches 2/3; the demonstration's low
opacity often has no such surface. This diagnostic does not infer an observed
image radius or impose an atmospheric boundary condition.

`composition_radiation_rates` exposes instantaneous shell heating and signed
net luminosity for local composition and optional grey opacity tables, without
advancing gas. Its ray work is explicitly budgeted and reported. Tests compare
constant-table and scalar luminosities and the sum of heating against net
escaping power. `effective_temperature(L, R)` defines a blackbody-equivalent
T through L=4*pi*R²*sigma*T⁴ using logarithms to avoid unnecessary intermediate
radius-square overflow. It accepts nonnegative outward emission, with zero
mapping to zero; signed net irradiation is not an emitted luminosity. Analytic
blackbody and extreme-radius tests cover the conversion. These diagnostics
do not establish a stellar spectrum, atmospheric structure or calibration.

### Quadratic tetrahedral structural mechanics

`plasticity::mesh::QuadraticBody` adds straight-reference ten-node tetrahedra
using the existing J2 material, dense linear solver and transactional Newton
contract. Node order is corners 0,1,2,3 then edge midpoints 01,12,02,03,13,23.
Shared edges must share midpoint indices. Four positive-weight tetrahedral
quadrature points integrate elastic quadratic-element stiffness exactly;
nonlinear plastic responses need their own spatial/quadrature convergence.
Each point has independent plastic history; all histories publish together only
on convergence. `responses_at` provides all four point stresses per cell without
committing history. Reference curvature, unused vertices and inconsistent edge
connectivity are rejected. Deformation inversion is checked at integration
points, not certified everywhere inside a curved deformed cell. The model is
still small-strain/fixed-frame and has no quadratic dynamics or cohesive adapter.

The same cantilever benchmark with 2x1x1, 4x1x1 and 8x1x1 bricks elevated to
conforming quadratic tetrahedra gives 3.80718e-5, 3.98264e-5 and 4.01527e-5 m.
The finest reaches 99.783% of the 4.024e-5 m beam approximation, compared with
72.78% from the much larger linear-tet mesh. End-face transverse traction uses
consistent quadratic face weights: zero at the corners and area/3 on each of
the three edge midpoints. Tests verify clamp resultant, work-conjugate tip
motion, exact quadratic pure-bending stress distribution, and atomic plastic
history commit/rollback. This fixes the demonstrated elastic bending stiffness
problem for this benchmark; it does not establish general experimental fidelity
or complete coupling of accurate bending with large rotations and fracture.

The coupled example CSV now includes instantaneous `net_luminosity_W` and
`photosphere_effective_temperature_K`. With the example's zero ambient
irradiation, net luminosity is outward emission and can be used in the
blackbody-equivalent temperature definition. The temperature field remains
empty when no radial tau=2/3 surface is found. This equivalent temperature
need not equal the local material temperature in this dynamic grey model.
Diagnostic transfer evaluations are read-only and have their own per-output
ray budget; their work is not reported as evolution work.
`--duration SECONDS` controls total simulated time (default one second), while
`--dt` controls intervals. Thin and thicker synthetic-opacity runs verified
both optional-photo-sphere output branches and positive instantaneous
luminosity, with energy residuals at roundoff.

`Sphere::hydrostatic_residual` reports static pressure-plus-gravity acceleration
per shell in m/s² using the local composition-dependent EOS. Face pressures
are arithmetic averages, and geometric pressure forces match the existing
zero-velocity momentum update. It is a read-only equilibrium diagnostic,
excluding velocity advection and radiation momentum forces. Tests check zero
residual for uniform pressure without gravity and mesh refinement toward the
analytic constant-density pressure profile in interior shells. The final face
uses the current boundary closure; it is not a hydrostatic atmosphere boundary.
A small momentum residual alone does not make the Rusanov mass transport
well-balanced for a stratified density profile.

`QuadraticBody::consistent_mass(densities)` now assembles the exact scalar
consistent mass matrix `integral rho*N_i*N_j dV`, shared independently by the
three velocity components. Its degree-four integrand must not be integrated
with the four-point stiffness rule. The element entries use exact barycentric
moments; tests independently reproduce them with a 4x4x4 Gauss/Duffy volume
integration, verify positive definiteness, total reference mass, affine-velocity
momentum and kinetic energy. The corner row sums are negative even though the
full mass matrix is positive definite, so diagonal row-sum lumping is not a
valid point-mass model for this element. This API prepares consistent inertia;
a dynamic integrator/factorization for quadratic cells is still outstanding.

`reactive_star` additionally reports `max_hydrostatic_residual_m_s2`, the
maximum absolute static pressure/gravity residual over its shells. The initial
uniform sphere is intentionally not hydrostatic; this diagnostic makes that
startup imbalance explicit. A regression compares the residual with the
initial momentum time derivative from the actual composition-dependent solver,
and the discrepancy decreases when the timestep is halved. This verifies the
diagnostic against the momentum update rather than only an analytic formula.
A dynamic model can also have velocity-advection forces, so this residual
should not be interpreted as its complete instantaneous acceleration.

`QuadraticBody::consistent_inertia` factors the exact consistent mass matrix
once with scaled Cholesky; the returned `ConsistentInertia::accelerations` solves
`M*a = force` for three force components using that cached factor. Normalization
separates the dimensional mass scale from the SPD factorization. Invalid force
counts, nonfinite input, solution overflow and numerically singular mass are
reported explicitly. Tests recover both uniform gravity and nonuniform affine
acceleration fields from consistent nodal loads and verify repeated solves leave
the factor unchanged. This supplies the acceleration operator needed by a
quadratic dynamic integrator; it does not itself advance geometry/history.

`Mixture::temperature_from_pressure` inverts total gas plus trapped-radiation
pressure at fixed density, allowing a specified hydrostatic pressure profile
to initialize thermodynamically consistent temperatures and internal energies.
It uses monotonic bisection with gas/radiation brackets computed logarithmically,
without replacing total pressure by a gas-only expression. Tests recover known
states across gas- and radiation-dominated limits and reject invalid input.
This inversion alone does not construct a hydrostatic stellar structure or
supply its atmospheric boundary condition.

`Sphere::initialize_uniform_hydrostatic` initializes the analytic continuum
constant-density profile P=P_surface+(2*pi/3)*G*rho²*(R²-r²), using volume-averaged
pressure for each shell. It obtains local temperature from total EOS pressure,
sets corresponding gas plus trapped-radiation internal energy, and resets
momentum to zero. Uniform positive density and positive surface pressure are
required. All composition/EOS failures roll back the entire initialization.
Tests verify inward-increasing energy, atomic failure and decreasing interior
force residual under mesh refinement.

Use `--hydrostatic` in `reactive_star` to initialize this profile. It supplies
an analytic interior pressure structure rather than a uniform pressure; its
outer face still uses the existing hydrodynamic boundary condition. Therefore
it is not an exactly balanced discrete star or an atmosphere model. A general
stratified stellar structure solver and consistent atmosphere boundary remain
pending; chemical composition, radiation transport and nuclear data still
need physical calibration for any intended star.

`plasticity::mesh::QuadraticDynamics` now advances a free quadratic body with
velocity Verlet using the exact consistent mass matrix and cached factorization.
Uniform acceleration is converted consistently: gravity work uses mass-matrix
row weights, not fictitious positive nodal point masses. Diagnostics evaluate
`0.5*v^T*M*v`, `1^T*M*v`, elastic energy, hardening energy and irreversible
plastic dissipation. Endpoint material states, geometry and velocities commit
together only if the signed full-energy change minus acceleration work meets
the per-step tolerance. Invalid/inverted/energy-rejected candidates leave all
accepted state unchanged. Explicit stability, small-strain kinematics and
plastic quadrature/temporal convergence limits still apply. Prescribed supports,
nonuniform time-dependent loads, contact and cohesive adapters are outstanding.

Tests reproduce analytic free fall, consistent momentum, and elastic energy
refinement: maximum relative deviations at 0.2 ms and 0.1 ms steps are 1.0781e-4
and 2.6857e-5. A separate low-yield plastic loading case accumulates independent
quadrature plastic histories, positive hardening energy and physical dissipation;
its total ledger error reduces from 6.5789e-9 to 3.6398e-10 with temporal
refinement. These measured bounds apply to the tested configurations rather than
a universal error guarantee. The previously documented absence of quadratic
dynamics is superseded by this free-body adapter; large-rotation coupling and
quadratic fracture/contact remain incomplete.

`Sphere::virial` returns the separate joule-valued terms 2K, 3 integral(P dV),
gravitational binding W, and -3 P_surface V, with `balance()` giving their sum.
Pressure includes gas and trapped radiation through each local EOS. Surface
pressure is explicit input. The constant-density initializer passes the global
static balance test as well as its local-force refinement test. For an open
flow this is not the complete moment-of-inertia evolution equation: boundary
mass and momentum flux terms are excluded. The surface-pressure contribution
and uniform-sphere gravitational term are derived in these
[Princeton astrophysical fluid dynamics notes](https://www.astro.princeton.edu/~rt3504/ewExternalFiles/course_notes.pdf).

`astrophysics_atmosphere::Atmosphere` provides a static plane-parallel
Eddington-grey model with constant gravity and grey mass opacity. It uses
T⁴=(3/4)Teff⁴(tau+2/3) and dP_total/dtau=g/kappa, as in the
[MESA grey atmosphere description](https://docs.mesastar.org/en/latest/atm/t-tau.html).
The supplied top gas pressure is distinct from the radiation pressure at
zero optical depth. Density follows the fully ionized gas EOS; radiation
pressure is included exactly once. Nonpositive gas-pressure gradients return
`NoStaticAtmosphere`, rather than generating negative density. Tests verify
the temperature convention, hydrostatic pressure increment, EOS consistency,
and rejection of nonstatic or invalid cases. The numerical 6000 K test is an
algebraic check, not a realistic partially ionized stellar atmosphere.
This module has no spherical curvature, variable-opacity integration, partial
ionization or time-dependent boundary coupling yet.

`Atmosphere::gas_cell` converts a nonvacuum atmospheric depth and supplied
radial velocity into the Euler conserved state. It includes gas internal,
trapped-radiation and kinetic energy exactly once. Vacuum density, invalid
velocity, overflowing energy and relativistic EOS sound speed are rejected.
A test recovers the atmosphere temperature from the resulting conserved
energy after subtracting kinetic energy. The method prepares a state but does
not attach it to a spherical boundary or evolve an atmosphere.
Run `cargo run -p physics --example grey_atmosphere` for an SI CSV profile
of temperature, density, gas/radiation pressure and energy versus optical depth.
This is a constant-gravity grey demonstration, not a calibrated atmosphere.

`QuadraticDynamics::new_supported` adds stationary whole-node supports at the
initial accepted positions, requiring zero initial velocity on supported nodes.
It factors the free-node principal submatrix of consistent mass, not a diagonal
approximation. `step_loaded` accepts constant nodal forces during the step and
includes their work in the energy guard. Uniform-acceleration forces use the
full mass row weights before restriction to free nodes: simply adding gravity
to a constrained inverse-mass solution would be incorrect. Fixed supports have
zero work but exchange impulse with the body. Fully fixed bodies, malformed
support arrays and nonzero support velocities are rejected. Moving supports,
per-axis supports and contact/cohesive coupling remain outstanding.

Two independent supported-oscillator tests isolate a quadratic corner-node
mode with effective consistent mass `rho*V/70` and stiffness
`0.6*V*(K+4G/3)` for the axis-aligned reference tetrahedron. Its unforced frequency
and constant-force displacement agree with the analytical discrete Verlet
solution; pinned positions and velocities stay exactly unchanged. Existing
free-fall and plastic history tests continue to pass. This supersedes the prior
free-body-only limitation for stationary whole-node supports and constant
nodal loads, while retaining small-strain and explicit-stability limits.

`Sphere::step_ionized_exterior` connects a fixed exterior reservoir cell and
its EOS to the outer Rusanov interface of an open sphere. An atmospheric
`gas_cell` can supply this reservoir. Exterior wave speed participates in the
CFL bound, and signed mass and gas/gravitational energy exchange retain the
existing conservative boundary ledger. The exterior mass is excluded from
self-gravity and its state remains fixed, rather than evolving an atmospheric
mesh. Reflecting boundaries are rejected. Tests construct the reservoir from
the grey atmosphere, verify mass/energy exchange and late-budget rollback.
This path currently supports a uniform interior mixture; reactive composition
transport needs an explicit exterior composition for inflow and is not yet
connected to this reservoir boundary.

`Sphere::step_composition_exterior` extends the fixed reservoir boundary with
explicit exterior fractions. Exterior thermodynamics are derived through the
same network as the interior. Incoming species use reservoir fractions;
outgoing species use the last interior shell. Signed per-species escaping
mass remains conservative and all failures roll back gas and fractions.
The inflow regression introduces helium into initially hydrogen gas, verifies
the mass ledger separately for both species and checks late-budget rollback.
The coupled burning/radiation wrapper has not yet been extended with this
reservoir; this method advances composition-dependent hydrodynamics only.

`QuadraticDynamics::support_reactions(loads, acceleration)` evaluates constraint
forces `M*a + f_internal - f_external` at the accepted configuration. It solves
free accelerations with the restricted consistent mass and sets pinned
accelerations to zero, retaining the off-diagonal inertia terms in pinned rows.
Returned free-node entries are force residuals near zero; support entries are
forces exerted by the constraints on the body (opposite force acts on supports).
A 500-step supported/loaded/gravity test checks every momentum increment against
the trapezoidal sum of external and constraint impulses, including the full
consistent-mass coupling. Malformed/nonfinite loads and overflow are rejected
without modifying the accepted body.

`Sphere::step_reactive_exterior` couples both hydrodynamic halves of a reactive
interval to a fixed composition reservoir. `ReactiveBudget` shares hydrodynamic
steps and nuclear work across the phases. Incoming material carries both gas
energy and nuclear binding energy through the existing signed boundary ledger;
no incoming binding release is mistaken for deposited nuclear heat. The test
uses incoming helium/carbon mixture, verifies the full ledger including its
binding reservoir, and rolls back a late burning-budget failure. The radiating
reactive wrapper still needs the exterior reservoir option threaded through it.

`QuadraticBody::from_linear` now elevates an existing shared-node tetrahedral
mesh directly. Original corner indices are retained; one midpoint is inserted
per distinct source edge and shared by adjacent cells. `edge_midpoints` exposes
the sorted edge-to-midpoint mapping for boundary-condition/traction adaptation.
Source mesh validation precedes expansion, and meshes exceeding the 512-node
limit after expansion are rejected. The production constructor is now used by
the quadratic cantilever convergence test, preserving its <1% beam-reference
accuracy. A separate two-cell test verifies shared-face midpoint connectivity,
corner preservation, and constant affine strain/stress after elevation. Original
linear nodal forces cannot simply be copied unchanged when representing face
tractions: use consistent quadratic face weights, as in the bending test.

### Radiating reactive sphere with an external matter reservoir

`Sphere::step_reactive_radiating_exterior` combines the fixed
`CompositionExterior` boundary with nuclear burning and both radiation halves.
It accepts an optional grey absorption opacity table. The exterior supplies
matter and its composition; incoming photon intensity remains the explicit
`ReactiveSettings::radiation.ambient` value. The reservoir is fixed and excluded
from the sphere's self-gravity, so this is not an evolving atmospheric model.

The complete energy ledger includes signed escaped gas/gravitational energy,
escaped nuclear binding energy, neutrinos, and net photon energy. The sphere and
all composition rows commit together only after the whole split step succeeds.
A regression checks an inward carbon flux, the complete energy ledger, and a
ray-budget failure in the second radiation half after matter transport and
burning have already succeeded on the temporary state.

`QuadraticBody::reference_faces` exposes straight reference boundary triangles
with six-node topology (three outward-wound corners, then midpoints 01,12,02),
reference area and outward normal. Shared internal faces are omitted and
nonmanifold faces are rejected. `traction_loads` integrates constant nominal
traction vectors in N/m² per returned face. These are fixed spatial force vectors
integrated over reference area, not follower pressure or curved-current-surface
tractions. Exact quadratic shape-function integration gives zero corner loads
and area/3 times traction at each midpoint. The cantilever benchmark now uses
this production loading API rather than test-local load assembly. Independent
tests verify resultant force, center-of-pressure moment, balanced closed-surface
reference pressure and affine displacement work. Nonfinite/malformed loads and
force overflow are rejected.

The same exterior regression also compares a constant opacity table with the
scalar-opacity path (density, energy, composition, and photon ledger). A narrow
density table accepts the initial radiation half but rejects the density raised
by inflow before the second half; this restores both the original sphere and its
composition, and reports `Opacity::OutsideDomain` through the radiation error.

The `reactive_star` example accepts `--exterior-density` (kg/m³), with optional
`--exterior-temperature` (K, default 2e8), `--exterior-carbon` (mass fraction,
default 0), and `--exterior-velocity` (m/s, outward positive, default 0).
The remaining exterior mass fraction is helium. These options describe a fixed
matter reservoir and work with scalar or tabulated opacity. Composition,
temperature, density, and velocity are validated before evolution; subordinate
options require `--exterior-density`. CSV output includes signed escaped helium
and carbon masses (negative values mean net inflow).

Reproduce a short inflow demonstration with:

```sh
cargo run -p physics --example reactive_star -- --dt 0.001 --duration 0.002 --exterior-density 200000 --exterior-carbon 0.5 --tabulated
```

This synthetic run has net inflow of both species and a relative complete-energy
residual approximately -4.13e-16 on the checked run. It remains a numerical
example, not a calibrated accreting star or a static atmosphere.

Quadratic construction now validates shared-face ownership before publishing
the body: one owner is a boundary; two owners must lie on opposite sides of the
face; three or more owners are nonmanifold and rejected. Duplicate cells and
same-side overlapping neighbors therefore cannot silently double structural
mass/stiffness. Tests cover duplicate, same-side and nonmanifold meshes while
retaining valid reversed local corner ordering. This is shared-face topology
validation, not a general intersection test between unrelated tetrahedra.

### Prescribed stratified hydrostatic initialization

`Sphere::initialize_hydrostatic` accepts arbitrary positive piecewise constant
shell densities and local composition, integrating `dP/dr = -G M(r) rho/r²`
inward from a positive supplied surface pressure. This is the Newtonian stellar
structure equation described in the [Princeton stellar interior notes](https://www.astro.princeton.edu/~burrows/classes/204/stars.interior.pdf).
Each shell's volume-averaged pressure is integrated analytically with positive
thickness polynomials; the existing gas plus trapped radiation EOS determines
its temperature and internal energy. Density and composition stay prescribed,
while radial momentum is reset to zero. All changes commit atomically.

The initializer matches the independent constant-density pressure profile and
closes the scalar virial relation for a three-layer density profile. A late
invalid central composition rolls back all already initialized outer shells.
This initializes mechanical continuum balance only: it does not solve nuclear
heating versus energy transport or make the Rusanov evolution well balanced.
The numerical outer boundary is unchanged.

Run `cargo run -p physics --example quadratic_cantilever` for an integrated
ordinary-mesh elevation, consistent reference traction, static bending and
supported consistent-mass load-release simulation. The default 8x1x1 brick mesh
has 153 quadratic nodes; static work-conjugate end displacement is 4.01527e-5 m
against the 4.024e-5 m beam approximation (0.217% difference). The constant load
is then removed and the initially stationary deformed beam evolves for 20 ms
with 10 µs explicit steps. CSV samples show tip displacement, kinetic/elastic
energy and signed energy drift. The observed maximum relative total-energy
change is 3.1044e-8. This short transient is not a full-period/modal-frequency
validation. The example verifies the energy bound and uses illustrative E,
Poisson ratio and density, not an experimentally calibrated material.

The cantilever example accepts an optional step count (1..=200000, default
2000, fixed dt=10 µs). Run `cargo run --release -p physics --example
quadratic_cantilever -- 70000` for 0.7 s of supported vibration. This longer
153-node run spans more than one fundamental period; maximum observed relative
energy drift is 3.50399e-8. A least-squares fit of `A*cos(2*pi*f*t) +
B*sin(2*pi*f*t) + C` to its 701 uniformly sampled tip-displacement outputs,
searching 1.4..1.8 Hz in 0.0001 Hz increments, gives 1.6092 Hz. The slender
Euler-Bernoulli first-mode reference `1.8751040687²/(2*pi)*sqrt(E*I/(rho*A*L⁴))`
is 1.615401 Hz, a -0.384% difference. The fitted single-component RMS residual
is 1.816% of its amplitude, so this is a dominant-frequency estimate rather than
a full eigenspectrum/individual-mode proof. Shear and 3D clamp effects also
separate the beam approximation from the full continuum boundary problem.

`QuadraticBody::finite_elastic_at` adds objective finite-deformation elastic
energy, internal energy-gradient forces and spatial Cauchy stress at all four
integration points using a supplied `biomechanics::Material` per cell. It
reuses quadratic interpolation/reference quadrature and evaluates activation
zero without mutating geometry or histories. Existing J2 plastic history is
rejected rather than reinterpreted or erased. Pure rigid rotation plus
translation gives negligible energy/forces; deformed configurations preserve
energy and rotate force vectors with the frame. Independent central energy
differences recover every nodal force component. Input counts, nonfinite data,
inverted integration points and constitutive overflow fail explicitly.
This supplies the finite-elastic quadratic force kernel. `QuadraticDynamics`
uses its small-strain J2 law; the distinct finite-elastic dynamic adapter is
described below. Finite-strain plasticity and quadratic fracture remain outstanding.

`plasticity::mesh::FiniteQuadraticDynamics` now connects the objective
finite-elastic force kernel to the existing consistent-inertia Verlet engine.
It is a distinct public mode from J2 `QuadraticDynamics`, constructed with an
explicit finite-elastic material per cell and optional stationary whole-node
supports. Its positions, velocities, energy, support reactions and `stresses`
use the actual finite-elastic law; it does not expose a misleading small-strain
response accessor. Existing plastic histories are rejected. Constitutive mode
is selected before initial diagnostics, without evaluating the unused J2 law.
The same transactional step and external-load work/energy guard apply.

Consistent angular momentum diagnostics evaluate `sum M_ij*x_i cross v_j`.
A 10-node tetrahedron with 4 rad/s initial spin evolves for 0.5 s, rotating more
than one radian while preserving linear and angular momentum within the tested
absolute 1e-6 tolerance. Relative energy envelopes at 0.2 ms and 0.1 ms steps
are 1.38536e-7 and 3.46675e-8. An unstable/energy-rejected step preserves positions
and velocities. Four-point integration is approximate for nonlinear hyperelastic
strain variations, so quadrature/spatial convergence and interior inversion
certification remain necessary. This supersedes the missing finite-elastic
quadratic dynamic adapter; finite-strain plasticity and fracture are
still not coupled into this mode.

`reactive_star --density-contrast C` sets the initial density to shell averages
of `rho(r) = 1e5 * [1 + C * (1 - r²/R²)]` kg/m³. `C` must be finite and
nonnegative; zero retains the uniform profile. With `--hydrostatic`, the general
initializer integrates pressure for these prescribed shell densities. Without
that option, the initial temperature remains 2e8 K, so the profile is not assumed
to be in mechanical equilibrium.

```sh
cargo run -p physics --example reactive_star -- --hydrostatic --density-contrast 4 --dt 0.001 --duration 0.002 --tabulated
```

The checked run begins with central density 499062.5 kg/m³ and central temperature
about 1.39547e8 K. The complete energy ledger closes to reported floating-point
precision over this short interval. The reported initial maximum hydrodynamic
force residual is about 322.45 m/s²: continuum pressure initialization does not
remove the evolution scheme's discretization and outer-boundary residuals.


`QuadraticPlaneContact` adds a stationary frictionless plane to both quadratic
dynamic modes via `set_plane_contact`. The stiffness density is N/m³ (Pa/m),
integrated over reference boundary area, rather than a spring stiffness per
node. Six positive triangle quadrature points distribute forces through the
quadratic shape functions. `contact_j` enters the transactional energy ledger;
`contact_force_n` reports the resultant on the body. Installing/removing the
plane returns the change of potential at fixed geometry as parameter work.

Tests compare every nodal contact force against central energy differences and
check the analytic integral of a fully active linear gap. A 0.1 m T10 tetrahedron
impacting at 1 m/s with E=100 kPa, density 1000 kg/m³ and penalty 1e6 N/m³ rebounds;
the wall impulse balances body momentum to 1e-9 kg m/s. At a 10 microsecond step
the original depth-zero relative energy envelope was 5.8114e-7; temporal refinement is checked. Invalid
contact parameters/overflow and rejected dynamic steps preserve accepted state.
This is a conservative sampled penalty, with finite penetration. Partially
active faces need surface/quadrature convergence; minimum sampled gap does not
certify geometric nonpenetration. Continuous collision detection, friction,
body-body contact and quadratic cohesive fracture remain outstanding.

`reactive_star --profile-out PATH` writes the final shell structure as CSV after
successful evolution. Each row includes the time, inner and outer radii, density,
local EOS temperature, separate gas/radiation pressure, radial velocity, helium
and carbon mass fractions, enclosed mass at the outer face, and shell-averaged
self-gravity acceleration and potential. The potential uses zero at infinity.
Units are explicit in the header. This supplements the time-series output on
stdout; the exported profile is the evolved final state, not the initial one.

The exported 16-shell stratified example was checked for finite entries,
contiguous radii, normalized composition, independently integrated cumulative
mass, inward gravity, negative potential, and agreement with the central
state reported by the time series.


Contact diagnostics now include `minimum_surface_gap_m`: the quadratic gap
polynomial is minimized over each complete boundary triangle, checking vertices,
edge stationary points and an interior positive-definite stationary point.
Scaling the nodal gaps avoids overflow/underflow in the quadratic coefficients.
Singular convex minima reach an edge; concave/saddle interiors cannot supply
a strict minimum. This is analytic in exact arithmetic, with ordinary floating
point evaluation, not interval certification or continuous collision detection.

Tests cover edge/interior/flat/singular extrema, 1e-200-scaled gaps, frame changes
and 100 polynomial cases against an independent dense triangular search. A
curved-face example has an actual minimum gap of -1 mm while every force
quadrature sample remains outside. The diagnostic detects this missed patch;
the depth-zero six-point contact force rule misses it. The adaptive integration
below resolves the tested patch; spatial/time collision handling remains necessary
for reliable contact with arbitrary curved faces.

Plot an exported profile with `python tools/plot_star_profile.py PROFILE.csv
OUTPUT.png` (requires Matplotlib; PNG, PDF and SVG are supported). The four
panels show shell density, local temperature, separate gas/radiation pressure,
and carbon mass fraction versus shell midpoint radius. The plot labels the
synthetic model explicitly. The input must contain one finite snapshot with
contiguous shell radii and positive density, temperature and pressures.

### Nonlinear response and contact

`SecondaryMotion::step_nonlinear` uses implicit Euler for a cubic hardening spring
and attachment-relative `ContactPlane` half-spaces. A monotone scalar solve gives
the unconstrained displacement; discrete projection enforces contact and applies
inelastic normal response and Coulomb friction. Inconsistent half-spaces roll the
step back. This is a lumped motion model, not a continuum material.

`Tissue::set_hardening` adds quartic edge energy using a dimensionless strain
coefficient. `step_with_contact` adds Coulomb velocity friction to existing sphere
particle projection. The original `step` API remains available with zero friction.
Inverted tetrahedra now fail without publishing state. Contacts remain discrete; the tetrahedral boundary now also resolves closest-point
triangle/sphere contact with barycentric normal and friction impulses. It rejects
unresolved penetration rather than publishing an intersecting triangle.

The actual `female_motion` model now also embeds four coarse volumetric tissue
cages. Pins follow the moving support, edge hardening and signed-volume constraints
shape the response, and torso-side sphere proxies limit inward deformation.
The render displacement fades at cage boundaries. These are illustrative cages
and proxies, not measured anatomy, a conforming body tetrahedralization, or a
complete tissue-to-tissue collision system. Both the cage deformation and bounded
secondary support motion are shown; the expensive full-body shell is a separate
example mode.

`astrophysics_star::Profile::sample(xi)` interpolates the stored Lane–Emden
potential and enclosed dimensionless mass at arbitrary radii inside its domain.
It uses linear interpolation, with exact stored points returned unchanged, and
never extrapolates beyond the final point. Publicly mutable profiles are checked
for finite nonnegative values, increasing radii, decreasing theta, and increasing
mass before sampling. This provides a checked sampling step for future transfer
of a polytropic structure onto a finite-volume radial mesh; it does not yet
perform shell averaging or initialize the evolved gas sphere.

The interpolator is checked between integration nodes against the analytic
`n=1` solution `theta=sin(xi)/xi` and `m=sin(xi)-xi*cos(xi)`, including centre,
surface, outside-domain queries and malformed point arrays. The analytic
solution and scaling relations are described in the
[Princeton polytrope notes](https://www.astro.princeton.edu/~gk/A403/polytrop.pdf).


Partially active contact triangles now subdivide into four children, using
analytic quadratic minimum/maximum gaps to refine only mixed regions. Entirely
active leaves retain the degree-four six-point rule, exact for this quadratic
penalty integrand in exact arithmetic. `with_refinement_depth` selects a bounded
depth 0–8 (default 4). The same subdivisions integrate potential and nodal force;
changing the refinement tree can introduce quadrature variation, so the existing
dynamic energy acceptance guard remains necessary.

A circular active patch missed by the original rule now has positive force and
energy. Independent polar-coordinate integrals give the reference energy and
resultant; relative energy errors at depths 4/6/8 are 1.1243e-2, 2.3948e-4 and
6.9142e-7. `partial_energy_error_bound_j` sums the maximum possible penalty
energy on mixed terminal triangles: both true and positive-weight approximate
energy lie between zero and this local cap. It bounds absolute quadrature error
in exact arithmetic, excludes roundoff, and is deliberately conservative.
Finite depth can still miss smaller active patches, but now reports their
energy cap; this is not continuous collision detection or exact contact clipping.

`Profile::shell_average(inner, outer, intervals)` computes volume averages of
`theta^n` and `theta^(n+1)`, returning density and pressure relative to central
values. Composite two-point Gaussian quadrature uses the interpolated profile;
its accuracy depends on both the source integration step and the requested
interval count. It rejects zero work budgets and shells outside the profile.
Normalized radii avoid forming large dimensionless radius cubes in quadrature.
The method adds no density floor, atmosphere, or pressure correction.

Checks cover the constant-density analytic pressure average, the analytic `n=1`
shell mass integral, central and surface shells, and mass integrated over a
16-shell partition (relative agreement better than 1e-7 in the tested mesh).

`Sphere::initialize_polytrope` transfers a sampled Lane–Emden profile onto the
existing radial grid through volume-averaged density and total pressure. It
checks that the supplied physical scaling agrees with the sphere's gravitational
constant. Every shell must lie within the source profile; no extrapolation or
atmospheric density floor is added. The local composition-dependent EOS supplies
temperature and internal energy, radial momentum is reset, and the entire
operation commits atomically. Stellar sampling errors are exposed as
`astrophysics_spherical::Error::Stellar`.

A scaled `n=1` test verifies outward-decreasing temperature, enclosed mass to
relative 1e-6, late-composition rollback, and rejection of inconsistent length
scaling. This imports a mechanical polytropic model; nuclear/radiative thermal
balance and exactly stationary numerical evolution still require separate work.

With default adaptive depth 4, the impact regression has relative energy
envelopes 2.57473e-6 and 6.16174e-7 at 20 and 10 microseconds respectively;
the peak contact resultant is about 22.12 N. Momentum/wall-impulse balance
and atomic failure rollback still pass. Adaptive integration changes the
partially active face forces; it does not add friction or body-body collision.

Run the coupled example from a polytropic profile using
`--polytrope-index N`. Central reference density is 1e5 kg/m³ and central
reference temperature is 2e8 K; their EOS pressure and G determine the physical
Lane–Emden length scale. `--polytrope-radius-fraction F` (default 0.9, strictly
between zero and one) places the computational outer face inside the surface.
This is an explicitly truncated positive-density model; it does not attach an
atmosphere. The example requires a surface within xi=20. Polytropic initialization
cannot be combined with `--hydrostatic` or a nonzero `--density-contrast`.

```sh
cargo run -p physics --example reactive_star -- --polytrope-index 1.5 --dt 0.0001 --duration 0.0002 --profile-out /tmp/polytrope.csv
```

The checked short run has outer radius about 1.30483e9 m, outward-decreasing
temperature, and final relative energy residual about 8.9e-16. This remains a
synthetic rate/opacity demonstration: importing a polytrope supplies mechanical
initial structure, not a calibrated thermally equilibrated star.

`reactive_star --shells N` selects 2–256 uniform radial shells (default 16).
The nonpolytropic radius remains 1e7 m; a polytrope keeps its scaled radius as
resolution changes. Existing CFL, nuclear and radiation budgets remain enforced,
so a permitted count is not a guarantee that arbitrary durations fit the budget.

A short synthetic `n=1.5` evolution with dt=1e-4 s and duration=2e-4 s was checked
at three resolutions:

| Shells | Central temperature (K) | Central carbon mass fraction | Relative energy error |
| --- | --- | --- | --- |
| 16 | 199450678.15 | 1.4729882e-6 | 8.90e-16 |
| 32 | 199862618.64 | 1.4931970e-6 | -1.76e-15 |
| 64 | 199965814.93 | 1.4982947e-6 | -1.75e-15 |

Successive differences in these central quantities decrease by approximately a
factor of four, while total mass agrees to better than 1e-10 relatively. This
short-run result does not establish convergence of long-term stellar evolution.
The maximum initial force residual instead rises from about 2581 to 3318 to
3711 m/s²; the initializer and current hydrodynamic boundary/discretization do
not provide stationary well-balanced evolution.

The exported profile also includes `hydrostatic_residual_m_s2` per shell,
using the same zero-velocity force diagnostic as the time-series maximum. It
includes the current standard outer closure, not an imposed reservoir's pressure
or advective forces. This distinction matters when exterior matter is enabled.

A near-initial snapshot (1e-12 s) locates the `n=1.5` refinement residual:

| Shells | Outer-shell residual (m/s²) | Centre residual (m/s²) | Interior maximum excluding two shells at each end (m/s²) |
| --- | --- | --- | --- |
| 16 | -2580.56 | 1735.94 | 271.51 |
| 32 | -3317.70 | 894.75 | 202.39 |
| 64 | -3710.84 | 450.78 | 110.28 |

The absolute maximum is in the outermost shell at every checked resolution.
Interior and central residuals decrease, while the truncated outer boundary
remains inconsistent with the continuous pressure gradient. This identifies a
boundary limitation; it does not demonstrate a well-balanced solver.


Both `QuadraticDynamics` and `FiniteQuadraticDynamics` now provide
`advance_loaded(interval_s, loads, acceleration, QuadraticAdvanceLimits)` for
constant loads and stationary supports/contact. Dyadic subdivision retries
energy-rejected or inverted-quadrature-point trials. Other constitutive/input
errors propagate directly; invalid initial states are checked before retries.
Maximum dt separately caps sampling, minimum dt and attempt count bound work,
and accepted substeps publish only after the complete interval succeeds.

Each substep receives the interval energy budget multiplied by its duration
fraction. `QuadraticAdvance` records accepted dt/defect pairs, total attempts
and the sum of absolute defects, so signed errors cannot cancel to hide drift.
This controls the energy ledger; it is not a phase accuracy estimate, stability
proof, contact quadrature tolerance or continuous collision detection.

In the 20 ms impact regression, a rejected single 20 ms step is replaced by
1501 accepted substeps (3001 attempts) with absolute defect 2.38084e-8 J within
a 1e-7 J interval budget. Against a fixed 10 microsecond reference, maximum
coordinate and velocity differences are 2.85168e-9 m and 2.26568e-6 m/s.
A large compressive trial that inverts an integration point recovers by
subdivision. Minimum-dt failure and attempt exhaustion preserve the complete
original state, including plastic histories after an internally accepted
plastic substep. A free-fall check verifies the independent maximum-step cap.

`Sphere::hydrostatic_residual_exterior` evaluates the zero-velocity force
stencil using the supplied exterior's thermal pressure on the outer interface.
It requires an open boundary and valid exterior composition/EOS. The diagnostic
still excludes advective momentum flux and exterior gravity. Its outer pressure
force agrees with an analytic zero-velocity interface calculation and the
initial momentum derivative of `step_composition_exterior` in the regression.
`reactive_star` now selects this diagnostic for both time-series and profile
output whenever an external matter reservoir is supplied, replacing the earlier
standard-closure-only diagnostic in that case.

`reactive_star --polytrope-index 1.5 --polytrope-exterior` constructs a fixed
matter reservoir from the volume-average of the next shell of the same
Lane–Emden model. Its composition matches the initial outer shell and its
velocity is zero. The extra shell must fit inside the source profile; this is
checked rather than extrapolated. Explicit exterior options cannot be combined
with this option. Ambient radiation remains separately prescribed (zero in this
example). The external shell is fixed and excluded from self-gravity, so this
boundary is not an evolving stellar envelope.

Near-initial outer force residuals with this boundary are approximately 419.04,
126.03 and 34.72 m/s² for 16, 32 and 64 shells respectively. The maximum residual
moves to the centre and decreases approximately by half with each refinement.
A dt=1e-4 s, duration=2e-4 s run also preserves the complete boundary/nuclear/
photon energy ledger to relative 1e-12. These checks support improved boundary
consistency, not exact hydrostatic stationarity or thermal equilibrium.


`QuadraticBody::plane_friction_at` now evaluates reference-area kinetic friction
using `QuadraticPlaneFriction(mu, regularization_m_s)`. At each active sample,
pressure comes from the normal penalty and velocity is interpolated through the
quadratic surface functions. Traction is `-mu*p*v_t/sqrt(|v_t|²+v_reg²)`, then
distributed with those same shape functions. It shares the normal-contact active
region subdivision. Resultant, nodal forces and instantaneous mechanical power
are returned; nonfinite force/power or positive power is rejected.

Uniform sliding checks recover the analytic resultant and power, with force
magnitude below mu times the normal resultant. Nonuniform velocity checks show
integrated power equals nodal force/velocity work and is negative. A rotated and
translated plane/mesh/velocity preserves power and rotates every nodal force.
No contact, pure normal motion and zero coefficient give zero friction.
This is a velocity-regularized kinetic law, not static stick/slip friction.
The evaluator supplies kinetic traction; dynamic integration and cumulative
dissipation are described below.

`Network::rates` evaluates held-state instantaneous nuclear composition
changes (dX/dt in s^-1), thermal deposition and escaping neutrino power (W/kg),
without mutating composition or energy. It shares the reaction calculation with
`burn`, including density conversion, symmetry factors, fit domains, binding
energy release and neutrino convention. The caller supplies a coefficient-set
evaluation budget; errors return no partial rates. Endothermic deposition can
be negative, while neutrino loss is applied only to positive release.

A synthetic triple-alpha regression checks the exact dX/dt, analytic SI power,
one-step burn agreement, evaluation-budget rejection and endothermic sign.
This supplies a read-only diagnostic for comparing nuclear heating with
radiative cooling; it does not itself solve thermal equilibrium.


Both quadratic dynamic modes now accept `set_plane_friction`, requiring an
installed stationary plane. Two explicit dissipative half kicks surround the
conservative Verlet update. Each freezes the integrated nodal friction force,
solves the consistent free-node inertia, and records force work on the average
kick velocity. Positive work rejects the step and adaptive advancement can
subdivide it. This is a timestep-dependent dissipative integration, not an
implicit Coulomb projection or static sticking constraint.

`friction_dissipated_j` and cumulative `friction_impulse_n_s` publish atomically
with geometry, velocity and material history. The energy guard includes the
actual cumulative dissipation increment, including its floating-point rounding.
Support reactions include friction forces and consistent-mass coupling. Removing
the plane clears the enabled friction law; changing/disabling friction preserves
its accumulated dissipation and impulse.

A freely sliding 0.1 m tetrahedron initially penetrating the plane by 1 mm
(with E=100 kPa, density 1000 kg/m³, penalty 1e6 N/m³, mu=0.4 and regularization
0.01 m/s) loses tangential momentum and dissipates about 0.0035834 J over 2 ms.
Tangential momentum change balances recorded wall friction impulse within
1e-10 kg m/s. Relative to a 2.5 microsecond trajectory, maximum velocity errors
at 10 and 5 microseconds are 5.8451e-6 and 1.1685e-6 m/s. Absolute full-ledger
energy envelopes are 6.5200e-9 and 1.6300e-9 J. Rejection after a successful
friction half kick preserves original diagnostics; excessive energy-injecting
kicks and failed adaptive intervals roll back. Supported-node reactions and
retention of dissipation history on disabling friction are also tested.
Static friction, body-body contact and finite-strain plasticity remain outstanding.


`Tissue::surface_triangles()` exposes oriented tetrahedral boundary faces. Shared
interior faces are removed; nonmanifold faces are rejected at construction.
Triangle/sphere contact covers face interiors and edges, including a sphere that
misses all particles. Contact conflicts with pinned faces and nonconvergence
roll back the step. This is discrete external sphere contact, not swept collision,
mesh-to-mesh self-contact, or detection of a sphere wholly enclosed inside a body.
The contact cage need not coincide with every detailed render vertex after fade
blending, so its exclusion guarantee applies to the simulation boundary.


`QuadraticBody::coulomb_plane_impulse_at` adds an unregularized Coulomb impulse
solver at fixed geometry/normal pressure. `QuadraticPlaneCoulomb` specifies mu,
velocity residual tolerance, iteration limit and sample limit. Each surface
quadrature impulse lies in a tangential disk of radius mu times sampled normal
force times dt. Projected coordinate descent couples all samples through the
restricted consistent mass, solving the convex maximum-dissipation problem.
Velocity residuals are checked after a complete sweep, not only at each local
update. Sample/iteration exhaustion returns an error without modifying mesh
or caller velocities. Pressure and adaptive surface sampling are shared with
normal contact.

Sufficient friction capacity brings the sampled contact velocity to zero;
insufficient capacity saturates the impulse bound and permits sliding. Tests
cover analytic sliding resultant, consistent-mass momentum balance, supported
reactions, normal-velocity preservation, frame covariance, zero coefficient/no
contact and explicit work-limit failure. The sticking example converges in
15 sweeps to residual 3.6538e-10 m/s, with energy defect about 2.22e-16 J.

The report separates endpoint traction work loss from backward-impulse
numerical dissipation (half the consistent-mass norm of the velocity change).
Their sum balances kinetic change; small negative endpoint losses from solve
tolerance are clamped and retained in `energy_defect_j`. This is a fixed-geometry
impulse kernel. Its dynamic adapter is described below; the kernel by itself
does not prove long-term static support. The kinetic regularized dynamic mode remains available.

### Contact-resolving flux foundation

`astrophysics_gas::contact_flux` provides an ideal-gas HLLC interface flux with
bounding acoustic wave speeds and a Rusanov fallback for degenerate/nonphysical
star states. It exactly preserves stationary equal-pressure density contacts
and selects the upwind flux for supersonic flow. Regression checks include a
wide density contrast and reflection symmetry of the Sod interface.

This API is not yet used by the evolution solvers. It is a building block for
removing static-contact diffusion while developing the spherical hydrostatic
reconstruction and matching gravity source. A contact-resolving flux alone does
not preserve a pressure-stratified hydrostatic star. The need to combine contact
resolution, hydrostatic reconstruction and source discretization is discussed
in [Käppeli and Mishra's second-order scheme](https://epubs.siam.org/doi/10.1137/140984373).

`Gas::advance_contact` now evolves the planar ideal-gas system with HLLC and
returns its actual mass and boundary transport. It retains the same CFL=0.4,
validation and whole-call rollback as the Rusanov path. Existing `advance` and
`step` keep their original flux. A periodic equal-pressure, zero-velocity density
contact remains exactly unchanged through many substeps, with zero transported
mass. Dynamic checks include reflective mass/energy conservation, exhausted
budget rollback, and the analytic Sod star region. This validates the flux in
evolution; it does not yet supply the spherical hydrostatic reconstruction.

`contact_flux_ionized` uses independently supplied left/right fully ionized
mixtures to derive gas plus trapped-radiation pressure and adiabatic sound
speed from the conserved states. It shares the HLLC construction and thermal
Rusanov fallback with the ideal-gas flux. Radiation energy is already in the
cell energy and is not added again; relativistic sound speed is rejected.
EOS failures are reported as `astrophysics_gas::Error::Eos`.

Tests include an equal-total-pressure stationary hydrogen/helium interface with
substantial radiation pressure, and agreement with gamma=5/3 in the gas-dominated
limit. This is an interface API; the spherical equilibrium reconstruction and
its source discretization are still pending.

`Sphere::step_composition_contact` adds an opt-in contact-resolving spherical
transport path with local gas+radiation thermodynamics and an optional fixed
composition exterior. The existing gravity-work calculation uses the actual
contact/fallback mass flux; composition uses that same flux, and state plus
composition commit atomically. Original spherical APIs retain Rusanov.

The regression preserves a static hydrogen/helium density interface without
G to EOS inversion roundoff (fraction changes below 1e-16), verifies mass and
energy with self-gravity and an exterior, and checks whole-call rollback after
budget exhaustion. This mode alone is not hydrostatically well balanced:
pressure reconstruction and a matching gravitational source are still required.


`set_plane_coulomb` now selects the impulse law in either quadratic dynamic
mode, replacing velocity-regularized friction. After each conservative half
kick, the Coulomb solve acts before drift (first half) or publication (second
half). This allows a feasible static friction impulse to cancel the applied
traction before position changes. A feasible common pressure-weighted impulse
initialization minimizes the same convex objective over a shared traction
direction, accelerating uniform traction loads before pointwise KKT sweeps.

Dynamic `friction_dissipated_j` records impulse work on the average velocity
of the complete force-plus-friction half kick. `friction_numerical_j` records
the isolated projection's backward-impulse loss as a diagnostic; it must not be
added again to the physical energy ledger. Under static loading, projecting
the temporary force-generated velocity back to zero does not dissipate physical
energy when the complete half-kick has zero displacement/velocity. The energy
guard checks actual mechanical change plus recorded friction work minus applied
load work. Solver nonconvergence remains atomic and adaptive advancement may
subdivide it; fixed sample exhaustion propagates directly.

A normally balanced T10 fixture loaded tangentially at 0.25 times the integrated
pressure remains stationary with mu=0.5: maximum movement over twenty 1 ms steps
is 4.2639e-18 m. The pressure-distributed normal load balances the penalty force;
this is a static-contact fixture, not a free impact stability test. Loading at
0.75 times pressure slides. Tests check wall/load momentum balance, whole-interval
energy, no false static dissipation and full rollback of failed solves.
`last_coulomb_impulse` exposes the last accepted half-kick and support impulses.
Instantaneous `support_reactions` rejects this mode because its velocity-only
force evaluator cannot represent a static traction reaction; use accepted impulse
diagnostics. Body-body contact, arbitrary moving supports, contact CCD and
quadratic cohesive fracture remain outstanding.

`Sphere::hydrostatic_pressures(surface_pressure)` returns both volume-averaged
cell pressures and all face pressures from the same analytic integration of
piecewise-constant density self-gravity. The outer face equals the prescribed
surface pressure, and the centre face is finite. `initialize_hydrostatic` now
uses these cell pressures rather than a separate integration. Face pressures
are checked against the constant-density analytic solution; each cell average
lies between its bounding face pressures. This supplies a consistent reference
for the forthcoming equilibrium-preserving reconstruction/source discretization.

### Equilibrium-preserving confined spherical evolution

`Sphere::initialize_balanced` initializes the piecewise-constant-density
hydrostatic model and returns an immutable `HydrostaticReference`.
`step_composition_balanced` reconstructs pressure perturbations about that
reference to each face, uses HLLC, and combines the reference pressure divergence
with departures of the geometric and self-gravitational force. Density and
composition remain live: this is a perturbation evolution, not a state freeze.
Gas-plus-binding energy work uses actual transported mass and midpoint
potentials. Nonphysical face pressure or evolution fails atomically; no floors
or pressure clipping are applied. Network nuclei and grid/G must match the
reference. This first path requires a reflecting, impermeable outer wall.

A nonuniform hydrogen/helium sphere remains bitwise unchanged for at least
1000 substeps. A 0.1% local energy perturbation drives motion while mass and
complete gas/gravitational energy remain conserved to relative 1e-12. A late
budget failure restores state and fractions. Nuclear burning, radiative exchange
and open-reservoir evolution have not yet been connected to this balanced path.


A threshold-transition regression now holds the same pressure-balanced body at
load/normal-force ratio 0.49, then increases it to 0.51 with mu=0.5. The first
phase has displacement below 1e-9 m and negligible mechanical friction loss;
the second has positive tangential momentum and dissipation. Its load/wall
impulse balance is checked within 1e-9 kg m/s. Relative to a 2.5 microsecond
trajectory, velocity errors at 10 and 5 microseconds are 4.76286e-9 and
9.77241e-10 m/s. These checks verify the threshold in this controlled fixture,
not general contact geometry or an experimental friction coefficient.

`cargo run -p physics --example quadratic_stick_slip` emits a CSV trajectory
for hold, slip and unload phases. It reports mean tangential velocity, actual
friction work loss, the isolated projection-loss diagnostic and the complete
energy defect after applied-load work. The example uses adaptive interval
advancement so near-stop solver iteration failures can retry with smaller
substeps without changing the 1e-10 m/s KKT tolerance. Near-stop subdivision
can be expensive with the current pointwise solver; performance is not yet
a validated real-time property.

The balanced path now also supports an open fixed-composition reservoir.
`initialize_balanced_exterior` requires a stationary initial reservoir matching
the supplied surface pressure to relative 1e-12. It records the exterior thermal
pressure reference. `step_composition_balanced_exterior` reconstructs current
reservoir pressure departures at the boundary and returns signed gas/gravity
energy and mass exchange plus escaped species masses. The reservoir is not part
of self-gravity and is not evolved; it can be changed explicitly between steps.
Its acoustic speed participates in CFL limiting.

The open-boundary regression keeps a nonuniform sphere exactly unchanged for
1000+ substeps with zero exchanges, then raises reservoir energy by 1% and checks
inward matter/composition transport and mass/energy ledgers to relative 1e-12.
Late budget exhaustion restores the entire sphere and composition. Burning and
radiation remain to be attached to this balanced evolution path.

`Sphere::step_reactive_balanced` connects nuclear burning to the immutable
hydrostatic reference, using balanced transport in both hydrodynamic halves.
It supports a reflecting sphere or a matching open exterior, and retains shared
hydrodynamic/burning budgets and complete atomicity. Nuclear heating and changed
composition create pressure perturbations; the reference is not reinitialized
after burning. Closed/open regressions verify nonzero heat and carbon production,
the gas/gravity/nuclear/neutrino energy ledger to relative 1e-12, and rollback
when a later cell exhausts the burning budget.

`step_reactive_radiating_balanced` completes the radiation/burning/transport
split around the immutable hydrostatic reference. It supports scalar or grey
absorption table opacity, a reflecting boundary or the balanced open reservoir,
and the existing shared ray/burning/hydrodynamic budgets. A reference is never
rebuilt between radiation or burning halves: physical heating/cooling creates
live perturbations. Closed/scalar and open/tabulated regressions check the full
gas/gravity/binding/neutrino/photon energy ledger to relative 1e-12 and restore
all state on a ray-budget failure after burning and transport.


The Coulomb solver now spends at most half its iteration budget on coordinate
sweeps, then can use FISTA projected updates on the same disk-constrained dual.
The trace of the scalar contact compliance matrix bounds the gradient Lipschitz
constant; no dense matrix over contact samples is assembled. Every accepted
iterate remains within its Coulomb disk, and the original per-point KKT velocity
residual and total iteration budget still determine success. A coupled analytic
two-contact test checks one sticking and one saturated sliding solution,
including endpoint and numerical loss (0.07 J and 0.19 J respectively).
This improves the difficult near-stop fixture without weakening solver tolerance.

The runnable example accepts `--balanced`, selecting the complete balanced
transport/burning/radiation path. It requires either an explicit stationary
exterior or `--polytrope-exterior`; the reservoir pressure sets the surface
pressure used to initialize the discrete hydrostatic reference. Existing shell
densities are retained, while pressure/temperature are recomputed consistently.

```sh
cargo run -p physics --example reactive_star -- --balanced --polytrope-index 1.5 --polytrope-exterior --dt 0.0001 --duration 0.001 --profile-out /tmp/balanced-star.csv
```

The checked ten-step run has final complete-energy residual about -1.48e-15.
The `hydrostatic_residual` columns still describe the standard pressure stencil,
not the balanced momentum operator; a stderr notice makes this explicit.
The equilibrium-preservation tests, rather than that diagnostic column, establish
stationarity of the balanced unheated/unirradiated sphere.

The runnable hold/slide/unload example completed all 120 samples through 1.2 ms.
Reference normal force is 5.19418 N. End-of-phase mean tangential velocities
are -2.64e-18, 1.24660e-4 and 1.55494e-7 m/s respectively. Absolute full-ledger
energy deviation is at most 1.47086e-12 J. Unloading leaves small elastic body
motion; this example does not assert that every internal node becomes stationary.

### Instantaneous stellar thermal balance

`Sphere::thermal_rates` evaluates deposited nuclear power, neutrino loss,
net radiative deposition and their local sum in W for each shell, without
changing gas or composition. Reaction-fit and ray-segment budgets are explicit.
Compression, advection and gravity work are excluded from this thermal diagnostic.
`ThermalRates::relative_imbalance` measures the largest local absolute net power
relative to the sum of absolute nuclear and radiative powers. A globally zero
sum does not establish equilibrium: opposite shell residuals may cancel.
This diagnostic does not yet construct a thermally equilibrated stellar model.
Regression checks cover local/global power closure, unchanged inputs, fit-budget
failure, cancellation between imbalanced shells and finite-power overflow scaling.


`QuadraticCohesiveFace` now supplies a matched T6 cohesive-face kernel with
six fixed reference-area integration points and independent irreversible
cohesive/contact histories. Corner/edge ordering is 0,1,2,01,12,02; paired
faces must use twelve separate indices at matching straight reference positions.
`trial_at` returns forces, energies, responses and a candidate history without
changing the accepted face. The containing solve must publish the candidate
only on acceptance and validate solid ownership/boundary winding.

Each integration point builds its material frame from the mean current quadratic
surface tangents. Constitutive jumps and crack-closure friction history live
in that frame. Assembled forces include the derivative of the moving frame,
not just opposite interpolated tractions. This supplies frame covariance and
zero total force/moment for curved surfaces, including compression and history-
dependent friction. Reference area remains fixed. Singular current frames fail
explicitly; finite nonlinear damage variations require quadrature/spatial
convergence. The mesh adapter below supplies a numerical cohesive Newton tangent;
the face kernel does not return an analytic global tangent.

Independent central stored-energy differences verify all 36 force components
in an elastic curved/compressed/sheared example. Force and torque sum to zero
within 1e-9 N / N m; rotating/translating the configuration preserves energy
and rotates forces. Uniform separation to full failure spends Gc*A=5 J for
Gc=10 J/m² and A=0.5 m², with no healing on closure. Fractured compression and
friction histories remain objective under rigid rotation. Invalid reference
pairing, shared nodes and degenerate current frames are rejected.
Its T10 mesh assembly, dynamic adapter and fragment diagnostics are described
below. Intrinsic insertion along existing mesh faces is available; arbitrary
within-cell crack insertion remains outstanding.

`Sphere::equilibrate_thermal` now searches for this local balance at fixed
shell density, momentum and composition. `ThermalSearch` supplies a temperature
interval, relative residual tolerance, sweep/evaluation limits and a cumulative
reaction-fit budget; `Heating::max_segments` bounds cumulative ray work.
Sequential shell solves bisect log temperature within sign-changing brackets,
then recheck the whole profile because changing one shell affects its neighbours.
A successful report includes the final powers and all work counters. Invalid
physics, missing brackets, exhausted budgets or nonconvergence leave the entire
sphere unchanged. This solver may miss multiple roots and does not prove thermal
stability. Its changed pressure is not automatically in hydrostatic balance;
it is a thermal structure solve, not yet a joint stellar equilibrium initializer.

### Coupled spherical stellar structure

`Sphere::equilibrate_stellar` (`astrophysics_equilibrium`) simultaneously solves
for density and temperature in every shell, with local nuclear/radiative balance
and EOS pressure equal to `hydrostatic_pressures(surface_pressure)`. A damped
Newton iteration uses a numerical Jacobian in logarithmic primitive variables;
line searches require a smaller maximum coupled residual. Density/temperature
bounds and cumulative evaluation, ray and reaction-fit budgets are explicit.
Failed solves preserve the complete original sphere. The grid/radius,
composition and surface pressure are prescribed; total mass is an output rather
than a conserved input or an additional constraint. Initial momentum must vanish.
The EOS must remain nonrelativistic. A converged root does not establish stability,
and fuel evolution will generally move the star away from its initial balance.
This is still the existing fully ionized grey absorption model, with no convection.

### Physical free-free absorption

`astrophysics_opacity::FreeFree` supplies spectral mass absorption in SI and an
analytic Planck mean for a fully ionized, nondegenerate, nonrelativistic plasma.
It sums the electron abundance and each ion's Z²-weighted abundance. The constant
Gaunt factor and valid temperature interval are explicit caller inputs.
The frequency-dependent cgs coefficient and stimulated-emission factor follow
[McGill PHYS 642 notes, Eq. (2.61)](https://www.physics.mcgill.ca/~cumming/teaching/642/phys642_all_notes.pdf).
The implemented Planck mean is our analytic integration of that formula with a
frequency-independent Gaunt factor, not an imported opacity dataset.
`planck_table` retains `Kind::PlanckAbsorption`; the existing grey chord solver
rejects it rather than silently treating this mean as grey or total extinction.
No scattering, partial ionization, bound absorption or frequency-dependent Gaunt
factors are included. Tests check an independent cgs spectral value, numerical
frequency integration, temperature/density scaling, H/He composition dependence,
log-table interpolation and explicit domain errors.


`QuadraticBody::add_cohesive_interface` now attaches matched T6 faces to the
actual mesh before loading. It checks boundary membership, separate matching
nodes, outward minus winding, opposite-side solid ownership and duplicate
pairing. Bulk plastic and six-point cohesive histories commit together after
quasistatic convergence or dynamic energy acceptance. Constitutive trials and
rejected solves cannot publish partial fracture history.

Both quadratic dynamic modes assemble corotated interface forces and report
`cohesive_stored_j`, `fracture_dissipated_j` and the separate crack-friction
loss/released-energy diagnostics. Actual fracture/friction/released increments
enter the energy guard; the crack-friction numerical diagnostic is not added
as another physical loss. The quasistatic Newton adapter differentiates only
the interface forces by central differences at fixed accepted history, with
step size based on material onset separation and coordinate roundoff. This is
a numerical tangent and needs resolution checks near nonsmooth damage branches.

`exposed_faces_at` excludes bonded paired faces and reveals both sides once all
six integration points fail. Plane penalty, regularized friction and Coulomb
impulse sampling query these faces at trial geometry instead of retaining a
stale boundary cache. This is an integration-point exposure rule, not a
continuous crack-front or fluid-access model.

A two-tetrahedron coupon has six exposed faces while bonded, then eight after
full fracture. Prescribed opening spends exactly Gc*A=5 J without healing on
closure. A free finite-elastic dynamic separation over 30 ms preserves momentum
and angular momentum within 1e-7 kg m/s and kg m²/s. Energy envelopes at 20 and
10 microsecond timesteps are 1.93253e-5 and 6.52790e-6 J, with final fracture work
5 J at both resolutions. An energy-rejected peak-skipping step retains original
geometry, velocities and cohesive history. Numerical-tangent equilibrium,
failed-Newton rollback and reversed-winding rejection are covered. Existing
contact, friction, plasticity and finite-quadratic dynamic regressions pass.
Arbitrary body-body contact and crack-front/spatial convergence remain outstanding.

`QuadraticBody::fragment_nodes` groups connected T10 cells and cohesive faces
using accepted irreversible damage history. A face connects its two sides until
all six integration points reach full damage; closing a fully broken interface
does not reconnect fragments. `QuadraticDynamics::fragments` and its finite
adapter report each component's mass, center, momentum, center velocity, angular
momentum about the world origin and kinetic energy without changing the body.
The diagnostics retain the full consistent mass matrix. Negative corner row
weights are valid for mass and first moments, but cannot replace that matrix
when computing kinetic energy.

Tests cover partial damage, full separation and subsequent closure. Fragment
totals agree with global mass, momentum, angular momentum and kinetic energy.
Rigid translation/rotation recover analytic tetrahedron moments; a quadratic
velocity field v_x=x² recovers momentum m/10 and kinetic energy m/70 for the
unit reference tetrahedron, distinguishing consistent inertia from row-sum
lumping. Components remain deformable parts of the same solver; automatic
conversion into separate bodies and general fragment recontact are outstanding.

`Sphere::free_free_radiation_rates` integrates the frequency-dependent absorption
law and Planck emission over an explicit finite frequency band, using logarithmic
midpoint bins and the existing spherical chord quadrature. Exterior irradiation
is a blackbody with explicit temperature. `trace_sources` carries each bin's
source intensity directly, without converting it into a fictitious temperature.
The result is finite-band power; users must check band, frequency and angular
convergence. No opacity-mean substitution, scattering or uncomputed spectral-tail
correction is performed. Ray budgets include all frequency bins.
`thermal_rates_free_free` combines those powers with the network's heating;
`equilibrate_stellar_free_free` uses them throughout the joint structure solve.
Tests show frequency refinement toward analytic optically thin Planck power
(final relative error below 3e-4), local/global energy closure, blackbody LTE
balance, and joint pressure/thermal residuals below 1e-7 with shared work budgets
and atomic failure. The joint regression deliberately uses synthetic constant
nuclear heating to isolate the structure solver; it is not a calibrated star.
Frequency-dependent radiative evolution and a physical reference star remain
separate verification work.

`Sphere::radiate_free_free` now evolves thermal energy with the finite-band
frequency-dependent powers at fixed density/composition. It limits internal-energy
and EOS-temperature changes per explicit step, rechecks the absorption domain,
and measures escaped photon energy from the actual step powers. Frequency bins,
angular sampling, cumulative ray/step budgets and maximum thermal timestep are
explicit. As with other explicit thermal updates, stability/convergence still
require timestep refinement. `step_reactive_free_free` composes two thermal
halves around burning and spherical dynamics, optionally using the mechanical
reference and a composition reservoir. Both halves share radiation budgets;
composition-dependent absorption is recomputed after burning and transport.
Tests with the original REACLIB triple-alpha fixture and physical free-free
absorption close the full gas/gravity/binding/photon energy ledger to 1e-12 and
verify complete rollback when the ray budget runs out after the first thermal
half and reactive dynamics.


`QuadraticBody::with_cohesive_faces` builds an intrinsic fracture discretization
from a conforming straight-reference T10 mesh: it duplicates all ten nodes per
cell and bonds every two-sided interior face with matched six-node interfaces.
The returned `QuadraticCohesiveMesh` retains source-node and cell-node mappings,
replicates prescribed displacements and splits source nodal forces equally
among coincident copies, preserving reference resultant and moment. Integrate
face tractions or heterogeneous body forces per cell instead; equal splitting
is not a replacement for their physical load distribution. Consistent mass
retains each cell's original density and volume. The dense solver currently
limits this construction to 51 cells / 510 duplicated nodes. Nonmanifold and
overlapping neighbors are rejected before returning a body.

A two-cell regression checks one automatically inserted interface, six exposed
outer faces, preserved mass with heterogeneous densities, source force/moment
and constraint mapping, and full fracture into two components with Gc*A=5 J.
Intrinsic interfaces add compliance before damage and restrict crack paths to
mesh faces; arbitrary within-cell cracks and spatial convergence remain open.

`QuadraticBody::from_linear_with_cohesive_faces` supplies the same intrinsic
construction directly from a conforming T4 source. It first elevates shared
edges, then duplicates the T10 cells. `source_reference_positions` returns the
complete elevated reference order: original corner indices followed by edge
midpoints. Nodal-force and prescribed-displacement mapping requires values for
all these source DOFs; corner-only arrays are rejected rather than silently
inventing midpoint forces or supports. Source cell order and material assignment
are retained.

A rigid-cell opening of 0.1 mm checks pre-damage interface force F=A*K*delta
and stored energy U=A*K*delta²/2 at K=1e6 and 4e6 N/m³, with zero fracture
work and equal/opposite resultants. This explicitly checks the additional
interface compliance: increasing K reduces it, while damage onset sigma_peak/K
and the explicit stable timestep also change. The constructor does not calibrate
K from bulk material stiffness or claim a mesh-independent fracture response.

### Opaque spectral transport and reference probe

Frequency-dependent transfer now reconstructs continuous source intensities at
shell faces and integrates a source linear along each chord segment exactly.
This avoids treating jumps between opaque constant-temperature cells as resolved
physical gradients. The outer face retains the last cell's source (zero radial
source gradient there); this is still a finite-grid boundary approximation.
`trace_linear_sources` has stable thin-limit weights and the correct inverse
absorption response in the opaque limit. Regression checks cover segment
splitting, power closure and a stratified opaque core: multiplying absorption by
ten reduces its transported power by approximately ten. The public grey
constant-shell transfer remains piecewise constant.
The structure solver freezes each shell's thermal normalization during its
Newton iteration; differentiating a residual saturated near ±1 can otherwise
hide the actual net-power derivative when heating and cooling differ greatly.
Convergence still requires the original local relative imbalance criterion.

`stellar_reference` is currently an experimental physical REACLIB/free-free
structure probe, not a completed calibration. A four-shell R=1e8 m,
initial rho=1e6 kg/m³ run with a hydrostatic seed did not converge. Thus no mass,
radius/luminosity calibration or long-term equilibrium claim follows from this
probe. Temperature bounds in this example are numerical search bounds rather
than a newly established reaction-fit calibration domain.

`QuadraticFace::closest_point_at` searches the complete current T6 parameter
triangle, including edges and corners, as geometry groundwork for moving
fragment contact. A feasible projected Gauss-Newton candidate supplies an upper
distance bound. Best-first subdivision uses the quadratic Bezier control hull:
AABB distance and separating support planes supply lower bounds. Both support
directions and the local candidate are heuristics for tightening bounds; their
global optimality is not assumed. Returned convergence requires the global
upper/lower distance gap to meet the requested tolerance. Patch/depth exhaustion
returns an unconverged result with that gap. Bounds are subject to floating-point
roundoff and are not interval certified. A small distance gap alone does not
certify a small position error or a unique minimizer.

Tests cover planar interior/edge/corner projections and observer rotation, a
curved minimum, two competing curved minima with an independently analytic
minimum distance, explicit budget exhaustion and invalid geometry. All eighteen
surface-node derivatives of distance at a unique curved interior minimum agree
with finite differences and the virtual-work expression N_i*(x-q)/|x-q|. The
query reports shape weights and a tangent normal without committing history.
It does not yet assemble moving-surface contact forces, friction, collision
pairs or continuous collision detection; those remain necessary for general
fragment recontact.

`QuadraticSurfaceContact` evaluates a symmetric frictionless proximity potential
for two separately indexed current T6 surfaces with a positive effective layer
thickness h. Its two reference-area passes each have weight one half and default to
six positive quadrature points; configurable subdivision is described below. At each source point q, a globally checked
closest-point query finds x on the target. The potential density is
0.5*K*max(h-|q-x|,0)² with K in N/m³. Source and target nodal forces use their
respective quadratic shape weights and opposite radial resultants. For a unique
resolved projection, the envelope derivative supplies the energy gradient;
closest-point parameter derivatives do not add a separate force. This also
preserves the pair's moment because the opposite resultants act along q-x.

Parallel unit reference triangles (area 0.5 m²), h=0.02 m, separation 0.01 m
and K=1000 N/m³ give exactly 0.025 J and opposing 5 N resultants. Beyond the
layer range, both force and energy vanish. An independently perturbed curved
pair checks all 36 nodal force components against energy differences, force and
moment balance, side-swap symmetry and rigid observer rotation. Unresolved
closest-point searches, shared-node pairs and active distances no larger than
search tolerance return errors rather than inventing a contact direction.

`search_energy_error_bound_j` bounds only the distance-search contribution to
the sampled energy error, subject to the floating-point qualification of the
closest-point bounds. It does not bound the fixed surface quadrature error or
force error; curved or partly active contact requires quadrature convergence.
This two-sided finite-thickness layer is suitable as a surface contact kernel,
but it is not a signed solid-penetration law: crossed surfaces and coincident
samples require further treatment. Automatic finite-layer component selection is described below; signed solid
contact, friction and continuous collision detection remain open.

The physical probe now has an explicit `SpectralSurface::EddingtonApproximation`
option. It uses the grey Eddington relation as a per-frequency numerical surface
closure: with center optical depth alpha*dr/2, the source contribution at the
surface is divided by (1+0.75*alpha*dr), blended with external irradiation.
[The grey relation is documented by MESA](https://docs.mesastar.org/en/latest/atm/t-tau.html);
our per-bin extension is an approximation, not MESA's atmosphere implementation
or a calibrated non-grey atmosphere. `CellSource` retains the previous surface
choice. Regression checks establish inverse-opacity escaping-flux scaling and
irradiated LTE balance for the approximation.

`Search::absolute_power_tolerance` makes the structure stopping criterion explicit:
for each shell, |net power| must be at most max(absolute tolerance, relative
tolerance times the sum of absolute nuclear and radiative powers). Hydrostatic
pressure retains its relative/log-ratio criterion. Zero absolute tolerance keeps
the old purely relative condition. Reports expose the scaled thermal residual
and maximum raw net power. This permits meaningful balance checks in outer cells
whose reaction rate is almost zero; their relative local residual alone is
ill-conditioned near numerical cancellation.

The earlier four-shell root was superseded by the source reconstruction that
preserves cell-center values: face-only interpolation admitted alternating
cell-temperature modes. Its reported mass and luminosity must not be used as a
reference. The physical eight-shell regression checks a newly solved profile
with independent fresh local balance and global luminosity/heating calculations.
Spatial convergence and long-term stability remain unverified; the experimental
`stellar_reference` example is not an observed-star calibration.

`QuadraticDynamics::set_surface_contact` and the finite-quadratic adapter now
install explicitly selected moving-surface pairs. Node sets resolve to the
body's authoritative reference boundary faces and areas; duplicate/reversed
pairs, shared-node pairs and missing boundaries are rejected transactionally.
Only currently exposed faces interact, using the same trial damage criterion
as other contact sampling. Thus an intact cohesive pair stays inactive and
fully failed faces can interact without reconnecting fragment topology.

Surface forces enter both dynamic force evaluations, including support reaction
assembly. `surface_contact_j` enters the accepted-step energy-defect ledger;
`surface_contact_search_error_bound_j` is a separate numerical diagnostic and
is not physical energy. Enabling, changing or removing the law returns the
fixed-geometry potential change as parameter work. Failure preserves the
previous law and accepted positions, velocities and material/interface history.

Two free unit T10 cells with density 1000 kg/m³, E=100 kPa and an initial
0.01 m face gap exchange approximately opposing 0.01 N s impulses over 2 ms
under the h=0.02 m, K=1000 N/m³ layer. Total momentum and angular momentum remain
within 1e-9 in their respective SI units. Energy envelopes at 200 and 100
microsecond steps are 2.68870e-10 and 6.72145e-11 J. Tests also verify explicit
parameter work, failed-setting/failed-step rollback and cohesive exposure in
both the small-strain and finite dynamic paths. Automatic component selection is described below. Signed penetration treatment,
friction and continuous collision detection are still outstanding. Energy refinement alone
does not prove that a fast crossing cannot skip contact.

`set_automatic_surface_contact` now enables nearest-surface finite-layer contact
between distinct trial fracture components in both T10 dynamic adapters. At each
force/energy evaluation it uses cohesive trial histories to form components and
current trial exposure to collect their boundary faces. Existing selected-pair
mode remains available. Same-component self-contact is not added. Automatic mode
requires no user-authored face pairs and returns fixed-geometry parameter work;
failed configuration/evaluation preserves the previous accepted state.

`QuadraticSurfaceContact::evaluate_surfaces` integrates each source sample
against the nearest point on the complete target surface. It retains the two
half-weight reference-area passes, selecting a single closest target face per
sample rather than adding pressures from every nearby face. Quadratic Bezier
control-point AABBs discard target faces whose distance lower bound is outside
the layer range; remaining projections must satisfy their requested distance
accuracy. The minimum of target distance lower bounds supplies the search-energy
error diagnostic. Ambiguous equal-distance projections can have a nonsmooth
force; fixed surface quadrature still needs separate convergence checks.

A planar target split into two T6 subtriangles retains the original 0.025 J
energy and 5 N source resultant, demonstrating no doubled pressure from target
face subdivision. Automatic two-cell dynamics matches explicit contact-pair
velocities within 1e-10 m/s, preserves removal parameter work, and leaves intact
bonded cells inactive while recognizing completely fractured components. This
provides automatic finite-layer fragment interaction, not signed solid contact
or a continuous collision detector. Fast surface crossing, same-component
self-contact, friction and spatial/contact-quadrature convergence remain open.

Accepted T10 steps with a surface-contact law now require a conservative
relative surface-motion bound no larger than one quarter of the layer thickness.
For a T6 triangle the sum of absolute shape weights is at most 7/4: each of its
three potentially negative corner weights has magnitude at most 1/8 and all
weights sum to one. Multiplying maximum nodal movement by 7/4 bounds movement
of every material point on the quadratic face during the linear position drift.
A common moving anchor is subtracted before combining both surfaces' bounds,
so a uniform translation of the two bodies does not limit the timestep.

Selected pairs use their face nodes; automatic mode uses trial fragment node
sets, including components that split during this candidate step. The check
runs after candidate drift and before final force evaluation or state commit.
Excess motion returns `quadratic surface motion limit reached` atomically;
`advance_loaded` bisects such a step and respects its existing minimum timestep,
maximum attempts and interval energy budget. It checks distant configured
components too, making this a conservative sampling cap rather than a spatially
optimized continuous collision detector.

Tests reject a fast crossing step even with a permissive 1e6 J energy tolerance,
for both selected and automatic modes. Adaptive separating motion refines into
accepted smaller steps and exactly matches replay of those steps. A common
1000 m/s translation leaves relative velocities consistent within 1e-9 m/s.
The cap prevents traversal of an entire finite contact layer in one accepted
linear drift; it does not enforce nonpenetration or establish a time of impact.
A weak penalty can still let bodies penetrate over multiple small steps, and
fixed surface quadrature can miss a small active patch. Signed contact,
continuous collision detection and quadrature refinement remain necessary.

`QuadraticSurfaceContact::with_integration_depth` now controls uniform four-way
subdivision of each source reference triangle, independently of closest-point
search depth/tolerance. Depth zero retains the original six positive samples;
depths 1 through 5 apply the same degree-four rule on 4^depth subtriangles.
Reference weights sum to the original half-pass face area. Depth five has
6144 samples per source face/pass; larger depths are rejected. The chosen depth
is carried through selected and automatic dynamic contact, and changing the law
still returns its fixed-geometry potential change as parameter work.

An independent planar-triangle reference integrates finite-triangle closest
points with a regular barycentric midpoint grid, separately refined from 512
to 1024 subdivisions per edge. A sloping face z=0.007+0.031*u opposite z=0
with h=0.02 m, K=1000 N/m³ has a partly active contact boundary that does not
align with the source subdivision. The reference energy is 0.0105796678231 J
and the lower-side z resultant is -2.34524449655 N. At depths 0 through 5,
absolute energy errors are 3.01015e-4, 1.81574e-5, 3.59580e-6, 6.49902e-7,
2.45427e-8 and 1.75215e-8 J. Force errors are 0.150317, 4.47277e-4,
5.86173e-3, 7.54975e-4, 4.21331e-4 and 8.73420e-5 N: force convergence is
not monotonic. A common translation of the refined target surface also checks
the resultant against the finite-difference energy derivative within 1e-6 N.

Uniform refinement does not certify detection of every tiny contact patch and
does not bound quadrature error a priori. The distance-search energy bound
continues to exclude surface integration error. Compare both force and energy
across integration depths; timestep refinement alone cannot establish spatial
contact convergence. Adaptive spatial contact integration, signed penetration
and continuous collision detection remain outstanding.

`QuadraticFace::swept_point_clearance_at` now queries continuous point-to-T6
clearance over normalized time [0,1], with a linear trajectory for the query
point and every surface node. Relative endpoint coordinates cancel a common
uniform observer translation. Their maximum nodal motion times 7/4 bounds
surface motion and therefore supplies a Lipschitz bound L for minimum distance.
A midpoint closest-distance lower bound minus L times the interval half-width
bounds the distance throughout that interval. Only intervals whose bound exceeds
the requested clearance are discarded; safe endpoint samples alone are
insufficient. Remaining intervals are bisected in chronological order with
explicit interval-count and minimum-time-fraction limits.

`QuadraticSweep::Separated` means all time intervals have been discarded by
these bounds. `WithinClearance` returns a feasible surface point and time
fraction within the requested distance, and does not claim first time of impact.
`Unresolved` preserves the undecided interval when work or temporal resolution
is exhausted. Spatial closest-point bounds and time arithmetic remain subject
to floating-point roundoff rather than interval certification. Zero clearance
may require unresolved output instead of an exact intersection witness.

Tests detect a planar crossing at fraction 0.25 between endpoints whose
distances exceed 0.4 m, detect a moving face crossing a stationary point, and
locate a curved crossing at analytic fraction 0.445 within 3e-5. A static
rotation combined with common 1000 m endpoint translation preserves that
curved witness. Separate tests cover full-interval separation, interval-budget
exhaustion and invalid inputs. This is a continuous geometric point-to-face
query; its optional dynamic rejection guard is described below. It does not
cover edge-to-edge or all curved surface-to-surface intersections.

`QuadraticSurfaceContact::with_sweep_guard` now optionally installs continuous
node-to-T6 clearance checks in both T10 dynamic adapters. Its positive clearance
must be smaller than the contact layer thickness. The existing surface-motion
cap runs first; then the candidate linear drift is checked before final force
assembly or accepted-state commit. Both directions are checked, using unique
source surface nodes and each opposing exposed face. Automatic mode forms trial
fracture components; selected mode retains explicitly paired faces. This guard
checks faces exposed at the start of the drift, and does not claim coverage of
newly exposed faces during the same step or curved edge-edge intersections.

A clearance witness returns `quadratic surface sweep clearance reached`;
undecided temporal/spatial search returns `quadratic surface sweep unresolved`.
Both trigger bisection in `advance_loaded`, retaining its minimum timestep,
attempt and energy limits. They never publish a partly applied step or interval.
Safe trajectories retain the original conservative surface forces; the guard
adds no impulse or artificial energy loss. Search bounds retain their existing
floating-point, non-interval-certified qualification.

Tests start two faces only 1 micrometre apart with opposing 0.01 m/s velocities.
The 100-microsecond drift is small enough for the previous layer-motion cap but
crosses the opposing face. Continuous guarding rejects it even with a permissive
1e6 J energy tolerance, in selected and automatic modes. A one-interval search
budget produces an explicit unresolved rejection. Safe guarded motion agrees
with unguarded velocities within 1e-9 m/s. When the penalty is too weak to stop
approach, the adaptive interval reaches its minimum timestep and rolls back
positions and velocities completely. This is a rejection guard; constraint-based
collision response, edge-edge coverage and a full nonpenetration guarantee
remain outstanding.

`ConsistentInertia::normal_impact` resolves one fixed-normal restitution
constraint using the full (or supported-node-restricted) consistent inertia.
Dimensionless signed nodal weights define the separating speed g=G*v. A
point-to-face contact uses positive source shape weights and negative target
shape weights. The unit impulse response is M^-1*G^T and its scalar compliance
is G*M^-1*G^T. An approaching contact receives J=-(1+e)*g/compliance for
restitution e in [0,1]; separating contacts receive no impulse. Reported
velocities and nodal impulses remain separate from the input state.

The effective mass is 1/compliance, and restitution loss is
0.5*(1-e²)*g²/compliance. The diagnostic compares that loss with the consistent
kinetic impulse work J*g+0.5*J²*compliance. Tests use two unit T10 tetrahedra
with densities 1000 and 2000 kg/m³ and corner contact. The independently
analytic inverse corner mass coefficient is 100/(rho*V), unlike a diagonal
nodal mass assumption. Tests cover e=0,0.5,1, the resulting separating speed,
full-matrix kinetic loss, M*delta_v=nodal_impulse, and total linear/angular
momentum conservation. Noncontact node velocities also change through the
consistent matrix. Invalid normals, dimensions, zero constraints and restitution
are rejected. Coupled simultaneous impacts and automatic impulse application
at detected contact times remain outstanding; this kernel alone does not
replace continuous collision response.

The expanded material goal additionally requires abrasion/removal and wetting
of materials. Existing `surface_film` provides conservative liquid-film
transport and a precursor wetting potential, while biomechanics poroelastic
modules provide fluid-storage/pressure coupling. These do not yet establish a
validated bulk moisture uptake/drying law, moisture-dependent strength or
abrasive geometry loss for T10/voxel solids. Those require explicit material
parameters, conserved water/debris accounting and mechanical coupling rather
than a visual surface effect.

`wear::Material` and `wear::Layer` now represent calibrated Archard sliding
wear and a finite uniform removable surface layer. The convention is
Delta V = k*F*Delta s/H, with indentation hardness H in Pa, accepted normal
load F in N and relative tangential sliding distance s in m. The dimensionless
coefficient k absorbs convention-specific geometric factors; it has no measured
default. This load/distance/hardness relation is discussed in the [Fouvry and
Proudhon tribology lecture](https://cmet.yastrebov.fr/Lectures/CMET_7_2025.pdf)
with the original Archard reference. It is an empirical wear law, not a material
fracture or cutting simulation.

Layer area, initial thickness and density define its available volume and mass.
Each accepted update reduces remaining thickness and accumulates debris mass;
reported removal is the actual representable change of its volume inventory.
No load, no sliding or zero wear coefficient causes no loss. Updates reject
invalid/overflowing/unrepresentable inputs before mutation. At exhaustion the
report returns the distance consumed by this layer and the unprocessed distance
that must be re-evaluated against the exposed substrate. Debris is an explicit
inventory rather than silently destroyed material. The model does not infer
frictional dissipation or heat from hardness times removed volume.

Tests recover 1e-8 m³, 1 micrometre depth and 2.5e-5 kg removal for a 0.01 m²
patch with H=1e8 Pa, k=1e-3, F=100 N, s=10 m and density 2500 kg/m³. They
verify load scaling, equal depth at fixed pressure across patch areas, one-step
versus 100-step distance partitioning, conservation of solid plus debris mass,
layer exhaustion with substrate path remainder, no-wear cases and rejection
rollback. This is physical layer accounting; feeding accepted contact sliding
into it, removing voxel/T10 geometry, transferring debris momentum, recalculating
mass/inertia and moisture-dependent wear parameters remain outstanding.

`moisture::Body` now stores finite water capacities and inventories per material
cell, with symmetric nonnegative conductances between cells. Saturation is
water mass divided by capacity. The model solves
C_i*ds_i/dt=sum_j G_ij*(s_j-s_i) using backward Euler and a scaled Cholesky
factorization, bounded to 128 cells. Capacities are kg and conductances kg/s per
unit saturation difference. No measured defaults or universal material
sorption curve are implied. Geometry/material adapters must determine capacity
from accessible pore volume and conductance from the relevant transport law.

Optional prescribed-saturation reservoirs model uptake or drying. Reservoir
saturation is an equilibrium material value, not automatically relative air
humidity. Each boundary's signed transferred water mass is returned explicitly;
reservoir inventory is external/infinite and a finite supply must be budgeted
by the caller. Internal exchanges cancel exactly in the mathematical system.
The discrete maximum principle bounds saturation between zero and one, and
accepted updates verify the actual inventory change against boundary transfers.
Invalid, ill-conditioned, overflowing or nonconservative updates leave state
unchanged. Tiny roundoff excursions are clamped within 1e-12 saturation before
checking the mass balance.

Independent tests verify the one-step two-cell saturation-difference factor
1/(1+dt*G*(1/C0+1/C1)), conservation and eventual common saturation with unequal
capacities. A dry 2 kg-capacity cell with bath saturation one and G=1 kg/s
converges under timestep refinement toward water uptake 2*(1-exp(-t/2)) kg.
Drying from saturation one with dt=100 s gives 2/51 kg water, with the removed
water recorded as negative reservoir transfer; subsequent wetting remains below
capacity. Invalid/overflowing steps preserve the accepted inventory. This is
bulk water transport accounting; coupling absorbed water to mechanical mass,
stiffness, strength, swelling, wear and surface-film depletion remains open.

`moisture::Calibration` now samples explicitly supplied dry and saturated
mechanical/wear properties at actual cell saturation. The current interpolation
is linear in saturation for Young's modulus, Poisson ratio, yield stress,
isotropic hardening, indentation hardness and Archard coefficient. Endpoint and
intermediate properties are validated by the existing plasticity and wear
constructors. Both strengthening and weakening are allowed; none of these
changes is a universal prediction for wet materials or a measured default.
Humidity-to-saturation sorption, hysteresis and material-specific nonlinear
property curves remain separate calibration requirements.

`Cell::properties` supplies compatible plastic and wear materials from accepted
water inventory. `Cell::wet_density_kg_m3` adds the actual absorbed water mass to
the supplied dry skeleton mass per reference volume; it does not multiply dry
mass by saturation or assume swelling. Applying that density or changed moduli
to a live solver still requires incoming-water momentum and constitutive
parameter-work accounting; this sampling adapter does not mutate solver history.

Synthetic regression parameters E_dry=100 MPa, E_sat=50 MPa,
yield_dry=1 MPa, yield_sat=0.1 MPa and hardness_dry=100 MPa,
hardness_sat=50 MPa verify transport-to-property coupling. A 1 kg-capacity cell
with a unit-saturation bath and 1 kg/s conductance reaches saturation 0.5 after
a backward-Euler one-second step, giving E=75 MPa. Its density rises from
2000 to 2500 kg/m³ for 2 kg dry skeleton mass and 0.001 m³ reference volume.
Further uptake changes a previously elastic test strain into plastic flow.
At full saturation the supplied half-hardness gives twice the wear volume at
unchanged coefficient/load/distance; reversed endpoints test wet strengthening.
Invalid water, calibration and density inputs are rejected. Live inertia,
cohesive-fracture energy, swelling and water-carrying debris coupling remain open.

`FiniteQuadraticDynamics::apply_moisture` now applies accepted per-cell water
inventories, dry skeleton masses and property calibrations to a live T10 body.
It updates reference density from dry plus water mass, rebuilds/factors the full
and supported-node-restricted consistent inertia, and changes finite-elastic
Young's modulus/Poisson ratio at fixed geometry. Cohesive histories and contact
laws are retained. This finite adapter remains elastic; wet yield/hardening
parameters are not implicitly turned into a finite-plasticity model.

Each cell's signed mass change contributes its consistent matrix Delta M.
Incoming water carries the supplied nodal velocity field u; outgoing water
carries the previous solid velocity. The new velocity solves
M_new*v_new=M_old*v_old+sum_cell Delta M_cell*u_cell on free nodes. Pinned
velocities remain zero and their required reaction impulses are returned.
Full-matrix incoming/outgoing kinetic energy is explicit, including signed
energy carried out during drying. The kinetic transfer decrement includes
inelastic mixing and constrained-node losses; changing elastic energy at fixed
geometry is reported separately as constitutive parameter work. Neither is
silently added to unrelated friction diagnostics. The caller must account for
reservoir/water momentum and energy and any eventual thermal conversion.

Tests entrain 1 kg of water into a 1 kg dry unit-reference tetrahedron initially
moving at 1 m/s. Still incoming water gives a 0.5 m/s mixture and 0.25 J kinetic
loss; water moving at 3 m/s gives 2 m/s and 1 J loss. Drying removes the absorbed
mass and its actual momentum without changing the retained material's speed.
A deformed-body calibration that halves E halves finite-elastic stored energy
and reports the signed difference as parameter work. Supported-node tests
verify the external momentum balance including support impulse. Invalid dry
mass remapping preserves positions, velocities and mass. Calibration, momentum
and energy checks finish before publishing any change.

This provides live finite-elastic wet mass/property updating, without swelling
or automatic surface-film/reservoir depletion. Coupled water transport velocities,
wet cohesive toughness, finite plasticity, heat evolution and water/debris
transfer during geometric abrasion still require integration and validation.

`moisture::Body::advance_with_supplies` now accepts finite pure-liquid sources
with explicit remaining water inventory. Each source drives toward saturation
one while available, with backward-Euler transfer
min(available_mass, dt*G*(1-s_new)). The capped flux is solved inside the same
network: a binding supply contributes its available mass to the right-hand side
instead of retaining a Robin bath term and clamping an already-computed state.
Starting from uncapped uptake, binding budgets can only lower saturation, so
further caps are identified by a monotonically growing active set. No exact
exhaustion time within the timestep is claimed. At most one additional solve
per newly binding supply is needed; sources are bounded to 4096.

Material and source water inventories commit together after bounds and actual
mass balance pass. Empty sources contribute no additional water; zero-conductance
sources retain their inventory. Unrepresentable source decrements are rejected
rather than increasing material water while leaving the floating-point supply
unchanged. A 0.1 kg source feeding a dry 1 kg-capacity cell with G=1 kg/s over
10 s gives exactly 0.1 kg uptake and zero remaining supply. Linked-cell tests
verify the independent capped/uncapped matrix equations, parallel sources and
combined material/source water conservation. Invalid updates preserve both.

`FiniteQuadraticDynamics::advance_moisture_supplies` now joins finite-source
uptake and live wet mass/property remapping transactionally. It first verifies
that moisture and mechanical cell order/inventory agree, advances cloned water
and supply states, then applies the momentum/energy-aware mechanical update.
Only complete success publishes all three states. Tests entrain a finite 0.1 kg
still-water supply into a 1 kg body moving at 1 m/s, producing 1.1 kg and
1/1.1 m/s. A mechanical mapping failure after tentative water uptake leaves the
source, material water and body unchanged. Source velocity/momentum accounting
remains an explicit caller input/report; actual surface-film or liquid-grid
routing, geometry wear removal, swelling and wet cohesive fracture remain open.

### Layered abrasive surface recession

`wear::Column` represents a finite planar patch with a fixed area and ordered
material strata. Accepted load and sliding distance drive Archard removal; an
exhausted stratum passes its unprocessed sliding path to the next material.
The cumulative recession is a geometric inward surface offset. Removed mass
is retained as debris inventory separately for each stratum, with an atomic
update of geometry and inventories. A non-wearing exposed stratum shields
its substrate. Work is bounded to 4096 strata.

Analytic two-material tests verify the hardness change at exposure, recession,
material-specific debris, total mass conservation, path subdivision, exhaustion
and invalid-input rollback. This planar representation does not yet modify
world voxels or remesh FEM solids, launch debris particles, transport their
momentum/water, or change a collision surface automatically.

`Column::mass_properties(width_m)` additionally integrates the remaining
rectangular strata exactly: mass, inward center depth measured from the original
surface, and the diagonal centroidal inertia tensor. Different densities are
combined by the parallel-axis theorem; an exhausted column returns no solid.
Tests compare homogeneous box inertia before/after removal and an independent
two-density centroid/inertia calculation. The caller must still apply these
properties to a mechanical body and transfer removed momentum to debris; the
geometry query alone does not perform that dynamic transfer.

`Column::advance_rigid` partitions the remaining body and removed material into
finite rectangular slabs with inherited rigid velocity and angular velocity.
Each debris inventory includes mass, centroidal inertia, linear momentum,
angular momentum about the original footprint center, and kinetic energy.
Keeping the slab spin avoids discarding rotational energy as a point-particle
approximation would. Analytic heterogeneous-column tests verify conservation of
both momenta and kinetic energy, plus atomic rollback on overflowing motion.
This models separation without an ejection impulse; friction work, heat,
particle fragmentation and world collision integration remain separate work.

`Column::advance_wet` uses each stratum's current saturation calibration to
select hardness and wear coefficient, then partitions uniformly distributed
pore water in proportion to removed volume. Remaining capacity shrinks with
geometry; removed water is returned separately from dry debris mass. Geometry
and water commit atomically. Exhausted strata have zero capacity and water and
must be removed/reindexed before constructing a diffusion network, which
requires positive capacities. Tests verify wet hardness, dry/water mass
balances, exhaustion and invalid-inventory rollback. The removal-only adapter reports dry and water inventories separately.
`advance_wet_rigid` additionally carries their combined mechanical inventory.


`Column::wet_mass_properties` includes uniformly distributed pore water in
stratum density, center of mass and centroidal inertia. `advance_wet_rigid`
atomically combines calibrated wet wear, water/capacity partition and inherited
rigid motion for the remaining body and debris. Water is entrained with the
skeleton; relative pore-fluid velocity is not modeled. Tests verify combined
mass, linear/angular momentum and kinetic energy conservation under simultaneous
translation and rotation, and rollback of geometry and water on late mechanical
overflow. These inventories still require a world adapter and collision update.

### Voxel wear world adapter

`physics_voxel::VoxelWear` binds a finite Archard layer to a loaded solid voxel
and its chunk revision, using an explicit SI side length, density, hardness and
wear coefficient. Partial removal accumulates thickness and debris inventory;
full exhaustion commits an AIR write through `World::commit`, producing the
normal dirty geometry bounds, inverse edit and durability receipt. World commit
precedes wear inventory mutation; intervening chunk changes reject the stale
binding without losing mass. Tests exercise partial wear, actual world removal,
remaining sliding path, receipt/inverse and competing-edit rollback.

This coarse adapter keeps the original voxel collision/render shape until full
exhaustion. It is not yet wired into automatic contact loading or the game tick,
and products remain inventory rather than spawned world objects. Wet rigid
column coupling is not implicitly applied by this single-voxel adapter.

`sweep_worn_voxels` provides collision geometry for partial +Y-facing full-face
wear: the remaining slab occupies local y=[0, remaining_thickness/side]. It uses
`sweep_aabb_with_bounds`, preserving the existing sweep broad phase, continuous
box contact, deterministic ordering and conservative unloaded-cell behavior.
Bindings must match current chunk revisions; duplicates and stale geometry are
rejected. A falling-box test independently checks contact fractions 0.5 for a
full voxel and 0.75 after removing its top quarter. All 36 physics_voxel tests
pass. This is an explicit query adapter: character/game-tick callers still need
to select it, the renderer remains full-cube, and arbitrary wear-face orientations
are not yet represented by this +Y-specific geometry.

`WornVoxelCollisionWorld` implements the shared `physics::CollisionWorld`
backend, and `step_character_with_wear` routes the ordinary bounded character
controller through the worn geometry. A landing test checks grounding, vertical
velocity cancellation and 0.75-voxel downward travel onto a quarter-worn surface.
Binding validation also runs before a stationary character step, so stale wear
cannot silently pass when no sweep is requested; failure preserves character
state. Game tick callers must choose this entrypoint while maintaining the wear
inventory. Automatic normal-load/sliding-distance extraction remains unfinished.

`VoxelWear::new_on_face` generalizes partial slab recession to any of the six
axis-aligned outward normals. Positive-face wear lowers the corresponding
maximum bound; negative-face wear raises its minimum. The other bounds remain
unchanged. The default constructor remains +Y. Six-face geometry tests verify
inward recession, fixed opposite faces and invalid-normal rejection; all 37
physics_voxel tests pass. A voxel still has one uniformly wearing face per
binding; duplicate bindings are rejected to avoid double-counting material.

`VoxelWear::surface` exposes a closed outward-wound 12-triangle surface in
normalized local coordinates from the same bounds used by worn collision.
Exhausted voxels return no surface. Tests integrate signed tetrahedral volume
for all six wear orientations and verify it equals remaining normalized volume
(0.75 after quarter removal). This geometry can feed the renderer's existing
floating-point scene mesh path, but is not yet submitted there. The primary
chunk mesher/GpuQuad path stores integer origins and extents and therefore
cannot represent partial recession without a geometry-format extension.
Neighbor occlusion/material/UV updates and old-mesh replacement remain required.

`cargo run -p voxy_render --example wear_surface` uploads the actual
`VoxelWear::surface` output to the existing floating-point SceneRenderer path,
renders original and 40%-worn blocks, and saves `/tmp/voxy-wear-surface.png`
(or a supplied output path). The example ran successfully on Metal; readback
checks that both geometries draw and the worn geometry covers fewer pixels.
The saved image was visually inspected. Material inventory reports 1200 kg
remaining and 800 kg debris for the explicit 2000 kg fixture. This proves the
geometry/render adapter in an isolated scene; automatic chunk replacement,
neighbor visibility and game-loop contact-driven wear remain unfinished.

`advance_voxel_wear_batch` advances up to 4096 wear bindings against one world
revision set and commits exhausted blocks in one transaction. Surviving bindings
in edited chunks adopt the actual receipt revisions; external edits still fail
validation. Partial inventories only publish after the commit succeeds. Removed
entries can remain in the list with zero loading. Tests check rollback when a
later loading entry is invalid, removal alongside partial neighbor wear, updated
neighbor revision validity, continued removal next tick and total debris mass.
All 38 physics_voxel tests pass. Callers must include all affected wear bindings
when refreshing same-chunk revisions; automatic registry ownership is pending.

Additional world-level wear regressions exercise actual transaction rejection:
a one-write budget rejects two exhausted blocks without changing either block,
chunk revision, remaining mass, debris or surface; a subsequent permissible
single removal succeeds. A two-chunk batch returns one receipt containing both
chunk deltas/inverse writes and conserves all dry mass, with independent
half-meter geometry checking volume and unprocessed path. All 40 physics_voxel
tests pass. SI side length must match the caller's world-unit conversion;
collision/surface output remains normalized to the unit voxel coordinate frame.

`moisture::Body::partition_material` removes uniformly distributed pore water
and capacity in proportion to retained volume, drops exhausted cells and returns
an explicit old-to-new index map. The caller supplies transport conductances for
the changed geometry, indexed by old cells; links touching exhausted cells fail
rather than silently transporting through absent material. An empty remainder
is represented by None. Analytic tests verify retained saturation, removed water,
new graph backward-Euler diffusion and closed water conservation, complete
exhaustion, and invalid-link/input rejection without modifying the original body.
Reservoir/supply/mechanical cell indices must be rebuilt with the returned map.

`Column::advance_wet_network` jointly commits calibrated layered wear, pore-water
partition, network compaction and the cell-to-original-stratum map. It validates
complete nonduplicated coverage of surviving strata, requires changed-geometry
conductances, drops exhausted cells, and represents complete exhaustion with
an absent network. A failed transport graph rolls back geometry, water and
mapping together, including when wear would have exhausted the last stratum.
Tests verify partial water/debris accounting, late graph failure rollback,
complete removal and subsequent no-material steps. This integrates wear with
moisture inventory; live FEM remeshing and world-liquid source routing remain
separate outstanding integrations.

Wet-network substrate regressions now independently verify crossing a saturated
H=10 top layer into a half-saturated H=15 substrate, preserving the original
stratum index after network compaction and using substrate hardness on the next
step. Analytic recession and removed/remaining pore-water mass agree across both
steps. Partial wet removal additionally rejects a floating-point underflow that
would otherwise remove all of an extremely small water inventory while retaining
material; the rejected operation preserves geometry and water together.

`moisture::CohesiveCalibration` supplies validated empirical dry/saturated
cohesive stiffness, closure stiffness, peak traction and fracture toughness,
linearly interpolated at initial saturation. Tests independently check peak
traction, complete-fracture work and irreversible unloading for dry, half-wet
and wet initialized interfaces. It intentionally does not replace material on an
accepted damaged interface: damage currently depends on maximum opening and law
parameters, so naive reparameterization could heal or change fracture history.
Live moisture-dependent cohesive updates still require history migration and
explicit parameter-work accounting; this initial-law calibration is not that
integration and does not establish complete wet fracture behavior in the world.

`cohesive::Material::migrate_history` now permits a fixed-pose frictionless
material change when it cannot reduce accepted damage. A stored fracture-history
offset preserves accumulated fracture work despite a changed toughness; returned
signed parameter work equals the change in stored energy per reference area.
This prevents previously spent fracture work from disappearing when moisture
changes material parameters. New loading and healing parameter changes reject;
friction-history migration remains unsupported. Dry-to-wet tests verify full
damage transition, unchanged accumulated fracture work, stored-energy balance,
irreversible reclosure and rejection of attempted healing/new loading. Existing
linear/quadratic cohesive tests also pass (14 tests total). Integrating the
migration into face quadrature and the dynamic energy ledger remains required;
it is not yet a live wet-FEM update or a wet chemistry energy model.

`QuadraticCohesiveFace::update_material_at` now migrates all six accepted
quadrature histories at fixed corotated geometry, rejecting healing/new loading
or unsupported friction migration before committing anything. It integrates
signed external parameter work with the same reference-area quadrature used by
force/energy evaluation. Tests verify a softened wet law can fully break a
previously damaged interface while preserving integrated fracture work and
balancing stored-energy change; attempted healing leaves all histories unchanged,
and subsequent closure remains broken. Existing objective force/friction tests
pass. The dynamics-level update still needs to publish this work in its ledger
and couple face saturation to the moisture network.

`FiniteQuadraticDynamics::apply_cohesive_moisture` applies explicit per-interface
saturation/calibration at the accepted pose, with a cloned-body transaction. It
returns integrated cohesive parameter work, any change in newly exposed surface
contact potential, their total external parameter work, and before/after fragment
counts. Stored cohesive energy and accumulated fracture work are checked before
publishing. The dynamic two-tetrahedron regression changes one damaged fragment
into two under a wet law, preserves fracture work, balances parameter energy and
rejects attempted drying-induced healing without changing the body. Twelve
quadratic wet/cohesive tests pass. Cell-to-face saturation transport and live
friction-history migration are still not supplied by this interface.

`FiniteQuadraticDynamics::cohesive_cell_saturations` resolves both interface
owners from the six face nodes and requires one unambiguous tetrahedral owner
per side. Explicit minus-side weights select a convex mixture of cell saturation.
`apply_moisture_with_cohesion` then atomically combines bulk mass/momentum/modulus
updates with history-preserving cohesive updates and returns both energy reports.
Tests exercise asymmetric cell saturation/weights, wet fracture into two dynamic
fragments, water mass uptake, and complete mechanical rollback when a subsequent
drying update would heal damage. The mixing law is an empirical uniform-cell
choice, not a resolved pore-fluid gradient. Transport/source inventories are
still updated separately unless the caller wraps the full moisture solve.

`advance_moisture_supplies_with_cohesion` wraps finite-source uptake, material
moisture, consistent-mass momentum/modulus remapping, interface saturation and
cohesive fracture in one cloned transaction. It returns source-transfer, bulk
wet-mechanics and cohesive parameter-work reports. The dynamic fracture test
now also exercises uptake from two finite sources, total source-plus-material
water conservation, mechanical mass gain, wet fragmentation, and rollback of
all three inventories when a late face-law validation fails. Ten supply/wet
mechanical/cohesive tests pass. These finite sources are explicit SI inventories;
world-liquid voxel extraction and contact-driven source routing remain pending.

`physics_voxel::advance_voxel_moisture` stages finite-source wet bulk/cohesive
mechanics from explicitly requested eighth-voxel liquid units, then commits the
world liquid-level decrease before publishing solid/material-water changes.
Unabsorbed withdrawn water is returned as `remaining_source`, a continuous owned
inventory that callers must retain and route; it is no longer in the voxel.
The regression checks analytic uptake, world-plus-pore-plus-surplus water balance,
mechanical mass gain and the inverse world edit. Both invalid mechanics and an
actual world write-budget rejection leave world, pore water and solid unchanged.
All 41 physics_voxel tests pass. Source discovery/contact routing and surplus
inventory ownership in the game loop remain explicit outstanding integration.

`return_liquid_surplus` returns only whole liquid units that fit the destination
voxel, retaining fractional and capacity-excess mass in the caller's continuous
source. It accepts matching liquid or air, checks the actual returned mass against
inventory even at rounded quotient boundaries, and changes the source only after
world commit succeeds. Tests cover a sub-unit remainder, capped refill, full-cell
no-op and real world-budget rejection with unchanged bank/world. The withdrawal
report is now must-use because dropping its remaining source would lose water.
All 41 physics_voxel tests pass. Game-loop ownership/persistence of these surplus
banks remains to be wired; the API does not auto-detect contact or recycle banks.

`VoxelMoistureBank` owns a validated continuous source between ticks with private
mass inventory, read-only mass access and explicit target-cell changes. Its
`advance` reuses the conservative finite-source bulk/cohesive transaction without
withdrawing world water again; `return_to_world` returns whole units and retains
fractions. The world regression now checks a second analytic uptake step
(material water 12/121 kg), unchanged bank on failed mechanics, total
world+pore+bank mass across ticks, and sub-unit return preservation. All 41
physics_voxel tests pass. The bank is a simulation-state object; persistence and
automatic ownership in the main game loop remain to be connected.

`FiniteQuadraticDynamics::advance_wet_loaded` integrates one atomic wet-motion
interval: implicit finite-source uptake and material updates first, followed by
adaptive mechanical motion with the updated mass/laws. This is first-order
operator splitting, not a monolithic moisture/mechanical solve. Reports retain
source transfer, bulk/cohesive parameter work and motion energy defects separately.
Tests verify that a later motion failure restores source/material water and body
pose, and that successful free-body motion has the correct updated-mass velocity
and interval duration. All six quadratic wet-update tests pass.

The expanded realism goal additionally requires drying/evaporation, crumbling,
dust, smoke, fog, water suspension and density-dependent transport. Current pore
network supports externally prescribed drying reservoirs and mechanical water
outflow accounting, but no evaporation/latent heat or gas/vapor inventory coupling.
The surface-film model explicitly excludes evaporation. Aerosol/suspension
particle transport and erosion-to-particle routing are not established by the
current layered debris inventories. These are outstanding requirements, not
implicit consequences of the existing wet/solid tests.

### Dilute suspension transport

`suspension::Particle` stores physical spherical radius, material density, mass,
position and velocity. `advance` solves linear Stokes drag and buoyancy in a
constant prescribed carrier with integrated displacement and reports the carrier
reaction impulse (drag plus buoyancy), maximum Reynolds number and particle
kinetic change. Density determines whether particles settle, remain neutrally
buoyant or rise. Re>0.1 rejects before mutation; endpoint relative-speed bounds
cover the whole exponential interval. Analytic tests verify Stokes terminal
settling, neutral/light particles, one versus 100 time intervals, gravitational
momentum balance and rejected-regime rollback.

The continuum incompressible low-Re sphere law follows the regime discussion in
[NASA particle-drag assessment](https://ntrs.nasa.gov/api/citations/20220007039/downloads/FAR_2022_Palmer_Particle_Drag_v3.pdf).
The caller must also ensure continuum/low-Mach conditions; Knudsen/slip correction
is not included, so very small airborne smoke particles are not validated by the
water test. This is a dilute prescribed-carrier model: it reports reaction but
has not coupled it to a gas/liquid solver. Particle generation, aggregation,
turbulence, Brownian motion, evaporation/condensation and rendering remain needed
for complete dust/smoke/fog/suspension behavior.

Suspension steps now report viscous heat from the integrated squared relative
velocity, effective gravity/buoyancy work, signed carrier-translation work and
an independently checked particle energy defect. The exponential integral uses
a positive Gram decomposition and a small-relaxation series to reduce cancellation
near zero initial relative speed. Energy changes use velocity differences rather
than subtracting large squared speeds. Tests verify lost relative kinetic energy
becomes heat and the same heat is obtained under common carrier/particle velocity
translation; settling and momentum regressions continue to pass (three tests).
Carrier heat/impulse deposition still needs a finite carrier-field integration.

`Particle::exchange_drag` couples one sphere to a finite homogeneous carrier
volume, integrating the two-mass relative exponential exactly with reduced mass.
Both velocities change, total momentum is checked from actual stored velocity
increments, and lost relative kinetic energy increases the carrier's private
heat inventory. Unrepresentable impulses/heat increments reject atomically.
A small-relaxation series stabilizes displacement. Tests check momentum,
kinetic-plus-heat conservation, carrier acceleration and one versus 100 exchange
intervals; all four suspension tests pass. This local substep excludes external
forces, requires Re<=0.1 and particle volume fraction<=0.01, and does not include
added-mass/history forces. Multi-particle carrier fields, spatial transport and
phase changes are still outstanding; deposited heat has not been converted to
temperature through an EOS.

`exchange_drag_cloud` solves a bounded (4096-particle) shared-carrier
backward-Euler drag exchange simultaneously rather than sequentially changing
fluid velocity per particle. It checks aggregate dilute volume, endpoint Re,
actual stored momentum increments, and energy. Physical drag heat and BE numerical
loss have distinct carrier inventories; the latter is not labeled physical heat.
Particle drift is first order, and the continuum/low-Mach assumptions remain.
Tests check reversal of particle order, momentum and kinetic+heat+numerical-loss
balance. Independent one-particle analytical two-mass relaxation verifies time
refinement reduces velocity error and numerical loss; all six suspension tests
pass. This remains a local homogeneous interaction, not a spatial gas/liquid
field, turbulence model or full dust/fog simulation.

`Liquid::exchange_suspension_cell` now connects this local cloud to one explicitly
selected SPH carrier particle, using its current thermal/phase material and the
reference volume `mass / rest_density`. Momentum feedback updates fluid velocity;
physical drag heat enters existing transport enthalpy, including the latent-heat
plateau. Numerical damping remains a returned energy ledger, never thermalized.
Liquid and cloud commit together only after successful heat deposition; missing
transport, invalid regime or unrepresentable heat rolls back both. Newtonian
carriers only; shear-thinning/yield-stress carriers are rejected. Tests compare
actual momentum, kinetic energy plus enthalpy plus numerical loss, latent fraction
and rollback. Spatial assignment, kernel-weighted deposition, gas-field transport
and evaporation/condensation mass exchange are still outstanding. Fluid motion
uses the existing SPH step separately; this local coupling is operator splitting.

`Liquid::advance_suspension` provides bounded spatial cloud assignment to nearest
fluid particles within the smoothing radius. Assignments are frozen for one
interval, then recomputed after suspended drift on the next call. This is a
piecewise-constant Voronoi coupling, not kernel-weighted SPH interpolation, and
rejects unsupported particles instead of silently deleting their mass. Each cell
uses simultaneous cloud feedback; the whole collection commits atomically,
including failure in a later cell after earlier cells have exchanged momentum and
heat. Tests cover carrier changes across a cell boundary, whole-step rollback and
global momentum/kinetic/enthalpy/numerical-loss balance across two carriers.
Fluid positions still advance through the existing SPH solver separately; no
claim of spatial convergence, wall deposition or airborne gas transport follows.

`Liquid::step_suspension_free` now owns the complete joint free-flight interval:
local drag/enthalpy exchange plus suspended drift, followed by the existing
symmetric fluid advance on each caller-selected coupling subinterval. Assignment
is refreshed each subinterval, and the complete fluid/cloud state rolls back on
any failure, including aggregate fluid substep budget exhaustion. The outer
coupling remains first order; symmetric fluid stepping does not raise its order.
Heated viscosity and pressure work are mandatory. Nonzero gravity, sampled/finite
boundaries and reflecting walls are explicitly rejected until shared-cloud
buoyancy, particle gravity and wall deposition have consistent implementations.
Tests verify common translation advances both phases and a later fluid budget
failure preserves both complete inventories. This is an API-level integration;
the game loop still does not own a suspended-particle field.

`exchange_forced_cloud` extends the shared-carrier implicit solve to constant body
accelerations, reporting endpoint force work separately from heat and numerical
loss. Its actual stored momentum increments balance prescribed force impulses;
kinetic change + physical heat + numerical loss equals force work. Existing pure
drag calls retain zero force work. `exchange_gravity_cloud` provides gravitational
weight plus local hydrostatic Archimedes force with equal opposite carrier
reaction. Tests independently verify both implicit momentum equations, total
weight impulse and work balance, plus invalid-force rollback. This hydrostatic
approximation must not be added on top of another particle pressure-gradient
force. It is not yet wired into the joint SPH advance: freely falling fluid is
not generally hydrostatic, so that API still rejects gravity until pressure
coupling supplies the correct particle force. Carrier position remains owned by
its spatial solver.

`Liquid::exchange_suspension_cell_forced` connects independently prescribed grain
and carrier accelerations to SPH momentum and latent-heat transport without a
hydrostatic assumption. Its body-force work remains a separate signed ledger;
only drag heat is deposited into carrier enthalpy. Force dimensions and the entire
thermal/mechanical candidate are validated before either phase commits. Tests
verify actual force impulse and kinetic/enthalpy/numerical-loss/work balance, plus
common free fall: equal acceleration causes no relative slip, drag heat or
artificial Archimedes force. Pressure-force interpolation and its reaction/work
must still be supplied by a coupled pressure provider, and must not be duplicated
in a subsequent fluid step. The joint free-flight API still rejects gravity.

`Liquid::suspension_inventory` reports solid mass and volume separately from liquid
mass/reference volume, using the same nearest-carrier assignment as drag. Total
mixture density is `(liquid_mass + solid_mass) / (liquid_volume + solid_volume)`;
solid volume fraction uses that total volume, not the dissolved-solute field.
Carrier reference volume follows current thermal/phase density, so phase expansion
changes concentration while preserving both phase masses and solid volume. The
same dilute limit as drag is enforced. Tests verify the independent mixture
formula, phase-dependent volume, mass reassignment after drift, and rejection of
nondilute clouds without state changes. This diagnostic reference density does not
replace kernel-estimated SPH density, alter the EOS or constitute volume-displacing
two-phase pressure coupling; those remain separate work.

`Layer::advance_dust` now commits wear removal and creation of real suspended
spherical grains together. Equal grain volume derives from the accepted removed
volume and caller-selected emission count (bounded to 4096); density derives from
accepted removed mass/volume. Summed emitted mass/volume and inherited translation
are checked before committing the layer. Invalid positions, missing grains or
kinetic-energy overflow preserve the layer. `DustRemoval.wear.mass_kg` is already
owned by its particles and must not be emitted again. Tests verify remaining-layer
plus particle mass, emitted volume, momentum and translation energy, rollback, and
actual emitted grains exchanging momentum/heat with liquid and appearing in its
mixture inventory. Positions, velocity and grain count remain caller inputs; this
is not a calibrated size-distribution/fracture model, surface-energy budget,
rotating-fragment conversion or wet debris evaporation. Cumulative layer debris
mass is historical accounting, not a second live mass inventory.

`Layer::advance_dust_with_surface_energy` now requires a finite caller-owned
`SurfaceEnergy` bank for the all-fresh-surface model. It charges specific single-
surface energy times the actual total spherical surface area; inherited
translation stays with the removed mass and is not an ejection energy purchase.
Layer removal, grain creation and bank debit commit together, with insufficient or
unrepresentable energy rolling back all. Returned surface energy must remain in
the energy inventory rather than also being booked as heat. Equal total volume
split into 64 equal grains costs four times the surface energy of one grain;
tests check that analytical size scaling, accepted energy debit, mass invariance
and rollback followed by successful coarser emission. No existing free-surface
credit is modeled, and this bank is not automatically funded from friction work.
The original `advance_dust` remains a kinematic partition API with no such energy
closure; callers needing energetic formation must use the budgeted variant.

`Layer::advance_dust_friction` funds fresh-grain surface creation from prescribed
accepted tangential force times sliding distance consumed by this layer. It returns
surface energy and the remaining friction heat separately; insufficient work for
the selected grain size rolls back removal. Distance beyond layer exhaustion is
left for the substrate and contributes no work to the exhausted layer. Empty
layers now consistently return the entire unprocessed distance, including zero
load/wear coefficient cases, instead of consuming distance through nonexistent
material. Tests verify exhaustion work, surface+heat balance, no work on an empty
layer and zero-work rollback; column/wet wear regressions cover the distance
change. Contact mechanics must actually remove the prescribed work, and the
returned heat must be deposited into an owned thermal state. This method does
not itself apply tangential impulses or select a heat partition between surfaces.

`Layer::advance_contact_dust` connects the existing fixed-normal Coulomb return
mapping directly to energetic wear emission. Contact pressure times patch area
sets normal load, accepted plastic slip sets Archard distance, and the actual
increment of physical friction dissipation times area funds fresh surfaces. It
excludes tangential spring storage, backward-Euler numerical loss and opening
release from that funding. Contact history and layer commit together only after
emission/energy validation. A held pose generates no new wear or work; insufficient
surface energy preserves both. Contact increments crossing layer exhaustion are
rejected for subdivision rather than committing a history for nonexistent
material. Returned friction heat is physical dissipation minus surface energy;
mechanics still must apply the returned contact traction, and thermal coupling
must deposit that heat. Tests use actual Coulomb slip, independently expected
wear volume, physical work partition, held-pose behavior and failed-formation
history rollback. This is a local fixed-normal contact integration, not automatic
mesh/voxel contact discovery or moving-frame remeshing.

`Layer::advance_contact_dust_heated` closes the local physical friction-energy
partition into owned state: fresh-particle surface energy plus residual heat
actually deposited through existing liquid enthalpy/latent-heat transport. An
explicit per-particle normalized weight map selects the thermal destination; no
implicit solid/liquid contact mapping is assumed. Contact history, remaining
layer and liquid enthalpy commit together. It checks actual accepted heat against
the friction remainder and rejects deposits lost to floating-point precision.
Returned friction heat is already deposited and must not be booked again. Tests
verify a real Coulomb-slip update drives a latent plateau, surface+enthalpy equals
physical contact dissipation, and late unrepresentable thermal deposition rolls
back all three states. The sink receives all residual heat by caller choice;
solid thermal fields, physical heat partition/effusivity and application of
returned contact tractions in global dynamics remain outstanding.

Run `cargo run -p physics --example wear_suspension` for a complete local prescribed-
sliding-contact demonstration: Coulomb return mapping, energetic dry-grain wear,
latent enthalpy deposition, accumulated cloud drag and joint fluid/cloud movement.
It prints CSV and checks, on every one of 20 increments, remaining layer plus
particle mass, emitted-plus-carrier momentum and the complete energy ledger:
prescribed endpoint tangential work plus inherited emitted kinetic energy equals
contact spring storage + contact numerical loss + created surface energy + liquid
enthalpy gain + current phase kinetic energy + cloud numerical loss. Thus physical
friction dissipation is not counted as both heat and surface energy. The observed
run emits 240 grains totaling 3.75e-9 kg, stores about 3.9380574e-5 J in grain
surfaces, deposits about 0.0749606194 J in enthalpy, and reaches high-phase fraction
0.749606194. Mixture reference density rises to about 1000.224966 kg/m³. Constants
are illustrative, not a measured material calibration. This is driven local
sliding and a freely translating homogeneous fluid carrier; it does not prove a
coupled deformable-body contact solve, spatial convergence or airborne smoke/fog.

`WearSuspension` is now an owned local system containing layer geometry/history,
Coulomb history, liquid thermal/mechanical state and the sole live dust vector.
`step_contact` stages the complete contact/wear/surface/heat/cloud-motion update
and commits once, with a bounded total cloud. Diagnostics return counts/mass/work
rather than another owned copy of emitted grains. A late fluid-step failure
restores old layer/contact/enthalpy and both existing and newly emitted grains;
the test then accepts the same contact increment with valid time and reaches 32
grains. The runnable `wear_suspension` example now uses this owner for every full
step rather than publishing contact formation before fluid advancement. It retains
the same 20-step mass/momentum/energy checks. Contact poses and inherited emission
velocity are still externally prescribed; global body dynamics, contact discovery
and gas/voxel integration are not established by this transaction owner.

The owned system now retains `WearSuspensionEnergy`: cumulative created surface
energy, drag numerical loss, physical friction input and inherited emission
kinetic input, plus actual current kinetic energy and liquid enthalpy. These
stores persist even when step reports are discarded. Positive inventory increments
must be representable; the full candidate is rejected on overflow/precision loss.
Before commit each step checks actual kinetic/enthalpy change plus created surface
energy plus numerical drag loss against physical friction input plus emitted
kinetic input. Reports expose the resulting defect; tests independently verify the
owned ledger and unchanged accounting after late rollback. The example compares
its external ledger to the owner on every increment. Additional fluid potential
energies are not accounted in this ledger; configurations introducing such work
can fail its balance gate and need an extended physical energy model, not a looser
acceptance tolerance.

Whole-step acceptance now also checks actual emitted grain mass against accepted
wear mass, and actual fluid plus existing/new grain momentum increments against
the prescribed incoming emission impulse in all three components. It uses stored
velocity differences rather than subtracting two large total momenta. Reports
include emitted mass and momentum defects; a failed gate discards the complete
candidate. A moving-carrier test independently checks global mass, each momentum
component and the accumulated energy ledger through ten emissions (160 grains).
The example's independent checks remain in place alongside these internal gates.
The prescribed emission momentum is still supplied by an external driver; a
future dynamic parent body must lose that same momentum when grains detach.

`WearSuspension::new_translating` adds a finite parent with prescribed constant
translation and no spin/ejection velocity. Emitted grains must inherit that same
velocity. Parent remaining mass determines its momentum and kinetic energy, which
transfer to detached grains; `emission_kinetic_input_j` stays zero in this mode.
The whole-step energy gate includes actual parent kinetic change, and momentum
acceptance includes parent mass-loss impulse with an explicit parent-inventory
roundoff allowance. Positive removal/energy transfers disappearing in parent
floating-point state are rejected. Tests independently verify parent + fluid +
cloud mass/momentum/energy, unchanged external emission input, and rollback for
wrong inherited velocity or late fluid failure. This is finite mass detachment
under prescribed translation, not a force-driven rigid-body contact integrator:
contact-driver work remains external, rotation/ejection impulse and changing
parent center/inertia are not modeled here. Existing `new` retains explicit
external-emission accounting for kinematic fixtures.

`WearSuspension::apply_parent_impulse` now changes finite-parent velocity using
remaining mass and an explicitly supplied external impulse. It validates actual
stored momentum increments, computes accepted signed kinetic work and retains it
in `parent_impulse_work_j`; acceleration and braking therefore have distinct
signed energy effects. Invalid/unrepresentable impulse or work preserves all
state. Detached grains retain their own velocity, while subsequent emissions must
inherit the updated parent velocity. Tests apply a three-axis impulse, verify
parent momentum and whole-system energy including impulse work through dust
formation, then brake the remaining parent without altering detached grains and
check invalid-impulse rollback. This is a force-integration building block; contact
traction is not automatically integrated here, and the user of the API must not
both prescribe contact-driver work and charge the same work again as a parent
impulse. Rotation, body pose integration and the coupled contact velocity solve
remain outstanding.

`step_contact_with_parent_impulse` stages an explicit external parent kick and the
complete contact/wear/heat/fluid/cloud frame in one candidate. Emission velocity
is derived from the kicked finite parent; invalid later fluid advancement restores
pre-kick velocity and impulse-work accounting as well as layer, contact and cloud.
Tests reject an invalid-time frame after otherwise valid force/formation candidates,
then accept the same kick with valid time and independently verify total parent +
fluid + cloud impulse and energy. Impulses whose nonzero component work underflows
to zero are also rejected atomically. The kick remains externally supplied and
must not duplicate contact-driver work; coupled contact-force/velocity solution,
body pose evolution and rotation are still separate requirements.

`friction::Material::advance_slider` now solves a finite mass's tangential motion
and fixed-normal Coulomb contact implicitly together. The contact is already
closed, normal gap constrained and target surface stationary; normal reaction
performs no work. A closed radial elastic/Coulomb projection gives accepted
velocity and contact pose from initial velocity, mass, area and dt, rather than
requiring a prescribed contact impulse. It checks actual stored momentum and
kinetic change + tangential spring-energy change + physical friction heat +
contact return numerical loss + integration numerical loss = 0. Trial results
include all ledgers and accepted history without mutating the input. Tests verify
the analytical one-step elastic mass/spring solution, constant Coulomb impulse,
100 repeated steps' energy balance/fixed gap, and time refinement toward the
independent undamped sticking oscillator with decreasing numerical loss. This is
backward Euler, not an energy-preserving oscillation scheme. Open/normal-motion
contact and inconsistent accepted pose are rejected; body rotation, normal impact
and integration with wear/mass changes are not yet handled by this slider solve.

`WearSuspension::initialize_slider` and `step_sliding` now integrate the implicit
fixed-normal finite-parent contact solve with energetic wear, inherited grain
mass/momentum, latent heat deposition and fluid/cloud motion in one transaction.
The contact pose is owned and advances from solved velocity. Pre-removal mass is
used for the contact solve, then grains detach at accepted body velocity (first-
order splitting). Physical friction is now an internal conversion rather than
external `friction_input_j`. The owned ledger retains contact spring energy,
contact return numerical loss and parent integration numerical loss in addition
to previous stores. Whole-frame kinetic/enthalpy/spring/surface/numerical balance
must close without external work. A ten-step test creates 160 grains while the
body slows and liquid heats, verifies mass and energy, includes opposite stationary-
plane impulse in total momentum, and confirms late invalid fluid-time rollback
restores dynamic pose/history/velocity/energy/cloud. The normal gap is clamped;
normal reaction performs no work. Contact updates crossing layer exhaustion are
still rejected for subdivision. Rotation, changing contact geometry/normal impact,
spatial plane discovery, solid thermal fields and game-loop integration remain
outstanding; illustrative tiny-friction test coefficients are not calibration.

Run `cargo run -p physics --example wear_slider` for the dynamic fixed-normal
counterpart of the prescribed wear demo. Twenty implicit contact/formation/fluid
increments reduce parent speed from 0.001 to about 0.000600 m/s, create 320 grains
with mass about 3.949975e-10 kg and increase liquid enthalpy by about 1.483549e-8 J.
Surface energy is about 9.644146e-10 J and total numerical loss about 2.000501e-10 J.
Every increment independently checks complete parent/fluid/cloud energy and mass,
and momentum including stationary-plane reaction. External friction/impulse work
stays zero; the final observed cumulative energy defect is about 7.52e-20 J.
These are illustrative solver inputs, including a deliberately small friction
coefficient; they are not experimental validation or continuum convergence proof.

`wear_slider` accepts an optional step count (1..200) for longer runs. A 100-step
probe exposed sub-ULP drag heat rejection at step 34. Liquid now owns a per-carrier
`suspension_heat_buffer`: if `add_heat` produces no representable enthalpy change,
physical drag heat stays in this buffer while momentum/drift commit. Later drag
calls attempt to deposit accumulated heat through the existing enthalpy/phase
model. The owned wear energy ledger and dynamic energy gate include this buffer;
it is not numerical damping. Tests verify small heat remains owned beside a large
enthalpy, rollback preserves it, and lowering enthalpy allows buffered heat plus
new heat to deposit and clears the buffer. Sixteen liquid/owned-system tests pass.
The longer run now accepts 47 steps before the separate contact-friction thermal
precision gate rejects step 48. Full stopping has therefore not been proved;
contact thermal buffering/precision and energy-limited grain formation remain
required. Positive deposited increments still have ordinary enthalpy rounding
error, bounded by the existing checks; this is not exact arbitrary-precision energy.

`Liquid::deposit_dissipation_heat` now applies nonnegative contact heat through the
same owned precision buffer, with bounded repeated phase/property validation and
atomic state update. `advance_contact_dust_heated` counts enthalpy plus buffer
increments, and allows ordinary enthalpy rounding at its actual energy scale
instead of imposing relative accuracy tighter than representation allows. Tests
verify real contact heat below a huge enthalpy remains owned, plus invalid heat
partition rollback. Twenty-three contact/liquid/owned-system tests pass. The long
slider probe now completes 49 increments, then fails the existing cloud drag
balance gate on increment 50. Full stopping remains unverified; that new failure
must be diagnosed without weakening the conservation gates indiscriminately.

The increment-50 cloud failure was split into precise overflow/energy/momentum/
inventory diagnostics. Its energy check now uses kinetic changes and body-force
work in the old carrier's moving frame, avoiding cancellation of large common
translation terms; reported body-force work remains in the laboratory frame.
Actual stored momentum checking includes an explicit velocity-rounding bound
`2 epsilon sum(m (abs(v_old)+abs(v_new)))` per component in addition to the original
relative impulse tolerance. A nearly comoving regression independently checks
relative kinetic+heat+numerical balance and stored momentum against that bound;
all eight suspension tests pass, and liquid coupling regressions also passed.
The long slider now passes those cloud gates but increment 50 rejects an
unrepresentably small positive update to cumulative owner energy inventory.
Compensated accumulation there is still required before claiming full stopping.

Owned cumulative positive energy inventories now use normalized two-component
TwoSum accumulation. `accumulation_correction_j` retains low components for surface,
drag loss, friction input, emission input, contact loss and parent integration loss;
each physical store is its reported high component plus that correction. Tests
verify eight quarter-ULP additions to 1.0 survive and produce exactly two ULPs,
and overflow preserves both components. This finite two-component precision is
not arbitrary precision. Whole-step kinetic subtraction also uses an explicit
8-epsilon bound from stored kinetic/buffer magnitudes alongside the existing
relative gate. The long slider now accepts increment 50, reaching speed about
4.02e-14 m/s with cumulative energy defect about 7.55e-20 J. Increment 51 trips the
example's independent momentum assertion during spring recoil; settled rest is
still unverified and that force/pose-rounding discrepancy needs diagnosis. No
claim of complete stopping follows from the small speed at increment 50.

### Contact history and sub-ULP suspension heat (2026-10-01)

The fixed-normal dynamic slider now evaluates the constitutive return map in a
local tangential frame, using the stored elastic gap rather than subtracting
large accumulated coordinates. The accepted global pose and plastic reference
are restored together. A translated-history regression compares impulses,
velocities and spring energy over twenty steps with a reference offset of 1e6.

Deferred liquid dissipation uses a compensated high/low heat inventory per
carrier. A representable enthalpy deposit consumes the high part and retains the
signed rounding residue; wear accounting includes both parts. The regression
adds eight quarter-ULP increments to a unit heat buffer and checks their exact
accumulated value and atomic rejection of invalid input.

The focused friction-slider, liquid-suspension, wear-dust and wear-suspension
suites pass (28 tests). The 100-step dynamic wear example accepts 52 steps, then
rejects step 53 with `cloud drag energy balance failure` as relative velocities
approach zero. This is an unresolved numerical gate, not a successful long-run
validation or evidence of complete game integration.

The cloud energy gate now includes a velocity-storage roundoff bound weighted
by relative velocity changes and applied acceleration. This bound vanishes with
those changes rather than scaling with total laboratory kinetic energy. Nine
suspension tests pass, including 200 unforced relaxation intervals checked
against independently accumulated relative energy and momentum. The integrated
slider now accepts step 53, but step 54 rejects a positive increment below the
resolution of the two-component cumulative energy inventory. That limit remains
unresolved; the full 200-step slider run has not passed.

### Fixed-normal dynamic wear relaxation: 200 accepted steps

Cumulative owner energy inventories now retain an ordered floating-point
expansion below the reported high/low components. Error-free TwoSum growth keeps
nonzero residuals; nonfinite arithmetic and the 2048-component budget fail
atomically. `energy_accumulation_tail()` exposes the remaining components for
independent accounting. Unit tests retain increments at 2^-60 through 2^-240
and check invalid-input rollback. This does not add precision to velocity or
enthalpy storage and does not prevent underflow of products.

The slider computes physical slip work and quadratic contact damping directly
from the accepted increment, rather than subtracting cumulative dissipations.
Spring energy change uses the difference-times-sum identity. The owner energy
gate includes an explicit eight-epsilon bound on stored kinetic, spring and
buffer magnitudes. The stationary-plane example completes 200 steps with mass,
plane-reaction momentum and energy assertions on each step, including spring
recoil and decay to |v| below 1e-20 m/s. At step 200 its independent energy defect
is 7.551238754178e-20 J and it owns 784 grains. Demonstration coefficients remain
uncalibrated; this fixed-normal local result is not full 3D contact validation.

### Voxel wear / suspension bridge

`physics_voxel::VoxelWearSuspension` binds a fresh receding voxel to the existing
owned contact/liquid/grain system. A candidate physics step updates its layer,
then full exhaustion commits a revision-checked world transaction before the
owner is replaced. Partial recession leaves block storage intact; consumers use
`binding()` for the existing worn collision adapter and `WearSurface`, while
`suspension()` supplies the sole live emitted grains and liquid state. Contact
normal must match the selected receding face. Late fluid failures and stale
chunk revisions reject the whole owned update. Contact motion/work and emission
positions remain caller inputs; this bridge does not implement deformable-body
forces or install itself into the main application's update/render loop.

### Native Voxy wear scene

`cargo run -p voxy_app --example voxel_wear` selects the owned voxel/suspension
bridge in the existing `SceneApp` fixed update and scene-mesh upload pipeline.
Space pauses; R recreates the scene; Esc exits. After 80 prescribed abrasion
increments the scene holds 1280 grains and a receding solid. The same accepted
binding supplies rendered surface and swept collision; verification checks the
actual downward sweep against the rendered top and total solid/grain mass.
All physical lengths, positions and grain radii use one 100x presentation scale.
The explicit injection sites form a separated cloud to avoid hiding different
steps in coincident particles; they are caller inputs, not simulated fragment
ejection trajectories. The demo stops after 80 increments instead of continuing
to force layer exhaustion. It is a diagnostic abrasion scene, not the full
load/deformation/crack/fragment pipeline. Coefficients are illustrative.

`cargo run -p voxy_app --example wear_snapshot -- /tmp/voxy-wear.png` uses Metal
GPU readback for a before/after image and checks that solid pixels remain,
emitted grain pixels appear, and both frames differ. The native `--smoke` mode
also exercises the window pipeline and validates 120 presented frames. The
initial native and snapshot runs both passed; source-global warnings from other
scene modules remain.

### Incremental J2 work partition

`plasticity::Material::response_with_work` extends the existing fixed-frame,
small-strain J2 radial return with a candidate `WorkStep`. Input includes the
previous accepted total strain and new total strain. The old history must admit
that previous strain without a further plastic return. Every nonlinear trial
still uses the last accepted history; this method never commits it.

Endpoint work `sigma_new : delta_epsilon` is partitioned into elastic storage,
isotropic-hardening storage, physical plastic dissipation `yield_initial *
delta_alpha`, and a separate backward-Euler numerical loss. The latter is the
elastic quadratic increment plus `0.5 * H * delta_alpha^2`; it must not be heated
into a solid or fluid. Energy values are reference-volume densities (J/m^3).
Storage changes use difference-times-sum forms and physical increments rather
than subtraction of cumulative dissipations. Nonfinite arithmetic, incompatible
starting history and a failed relative work balance reject the candidate.
This does not supply finite-rotation plasticity, a solid temperature field or
automatic FEM thermal coupling.

Validation: five incremental-work tests and all six prior plasticity tests pass.
The new checks cover analytic elastic and plastic shear, hydrostatic compression,
invalid-history rejection and cyclic reverse flow. Independent accumulated work
balances final elastic/hardening energy, plastic heat and numerical loss; 16/64/256
subdivision runs verify decreasing numerical loss and converging physical heat.

### T10 bulk work / accepted equilibrium

`QuadraticBody::bulk_work_at` integrates the J2 incremental partition at the four
reference-volume quadrature points of each straight-sided ten-node tetrahedron.
`QuadraticWork` reports body totals, candidate point histories and a per-cell
physical-heat vector in joules. It is a bulk report; cohesive-face work is not
included. A thermal consumer must deposit only accepted `cell_plastic_heat_j`,
not numerical damping or hardening storage.

`equilibrate_with_work` solves an uncracked body on a clone. On convergence it
checks volume-integrated work against `(external loads + constrained reactions)
 dot (accepted nodal displacement increment)`, and checks point histories against
the accepted solve before publishing geometry/history together. Failure rolls
back; nonconvergence returns no work report and commits nothing. Bodies with
cohesive interfaces are rejected by this combined acceptance API until the
interface-work partition is integrated. Existing cohesive equilibrium remains
available through its original API. No temperature field is installed yet.

Validation: all three new quadratic-work tests and five existing quadratic patch
and topology/history tests pass. Affine shear loading, unloading and reversal
agree with an independent material-point reference multiplied by tetrahedral
volume. Inversion/invalid-solver input and a nonconverged solve preserve accepted
positions and point histories; nonconvergence exposes no consumable heat report.

### Coupled bulk / cohesive endpoint-work audit

`QuadraticBody::coupled_work_at` combines the existing bulk partition with each
T6 interface's stored-energy change, fracture work, physical friction heat,
friction numerical damping and released contact energy. Fracture work remains
separate from thermal heat. The interface endpoint integration error is signed:
softening can make endpoint force times displacement smaller than the energy
spent creating a crack. Such error is reported explicitly, never converted to
positive damping or heat.

`equilibrate_with_interface_work` solves on a clone and requires an explicit
nonnegative finite per-step error budget in joules. It gates the sum of absolute
interface errors, not their signed sum, checks total bulk+interface work against
nodal loads/reactions, and verifies bulk and interface candidates match the
converged histories before accepting all state together. Over-budget/error
steps roll back; nonconvergence supplies no consumable work report. Callers
must reduce load increments to meet their selected accuracy. The budget is not
a physical energy source and reporting the signed error is not a claim of exact
energy conservation for arbitrary fracture increments. The previous uncracked
work API remains unchanged.

The nodal/quadrature comparison also includes an explicit coordinate-storage
roundoff bound: `16 epsilon sum((abs(load)+abs(reaction)) *
(abs(x_new)+abs(x_old)+abs(delta_x)))`. This handles nearly zero work after
traction vanishes; nonfinite bounds are rejected. It does not enlarge the
explicit interface integration-error budget.

Validation: two new interface-work tests and three bulk-work tests pass after
the roundoff fix; all five existing cohesive tests passed in the preceding run.
The prescribed coupon fracture spends 5 J (=10 J/m^2 times 0.5 m^2), remains
broken upon closure, and rejects an oversized opening before any geometry or
history commit. Refinement from 128 to 512 opening increments reduces the sum
of absolute interface endpoint errors to less than 30% of the coarse value.
These are prescribed equilibrium tests, not a dynamic fragment-impact proof.

### Accepted dynamic fracture work and deformable fragments

`QuadraticDynamics::step_loaded_with_fracture_work` wraps the existing guarded
small-strain dynamic step with a candidate bulk/cohesive work audit and accepted
fragment partition. It rejects finite-elastic dynamics rather than interpreting
a finite-strain state as J2 history. Actual mechanical integration energy defect
and signed constitutive endpoint error remain separate; the latter uses an
explicit sum-of-absolute-interface-error budget. All audits operate on a clone,
and late failures preserve pose, velocity and constitutive/contact histories.

The returned `QuadraticFractureStep` identifies newly fully broken interfaces,
reports previous fragment count and consistent-mass kinematics for all accepted
pieces. Before publishing it checks fragment mass, linear/angular momentum and
kinetic-energy sums against the containing dynamic state. A fragment remains a
deformable connected component; there is no lossy rigid fit or independent
copy owning its mass. No separate fragment collision world is installed by this
method. Reported bulk numerical work is not added a second time to the existing
Verlet energy ledger or deposited as thermal heat.

Dynamic fragment momentum audits expose linear and angular summation roundoff
bounds computed from absolute consistent-mass terms and the reduction count.
They handle cancellation when total momentum is nearly zero; nonfinite bounds
reject the step. The relative partition checks remain unchanged.

`advance_loaded_with_fracture_work` adds atomic dyadic subdivision, with a
maximum sample interval and attempt/minimum-dt limits. Each accepted substep
receives its time-proportional share of the mechanical energy and interface
endpoint-error budgets. Energy/geometry/contact guard failures and interface
budget failures bisect; other constitutive/audit errors propagate. The complete
interval publishes only after every substep passes. Rejected trials emit no
consumable heat or crack events. Summaries retain fracture work, plastic heat,
energy/error diagnostics and transitions; final fragments use the accepted
consistent-mass state. Per-cell heat accumulation rejects overflow or a lost
positive increment rather than silently dropping it.

Validation: three new fracture-work tests and all four prior adaptive dynamics
tests pass. A freely separating, translating coupon fractures over 0.04 s into
two deformable pieces, emits one fully-broken-interface event, and spends 5 J
of fracture energy. Tests independently compare total mass, nonzero initial
linear/angular momentum and final mechanical energy against accumulated signed
integration defects. The run stays within 0.01 J of summed absolute mechanical
defects and 0.05 J of summed absolute interface endpoint errors. Late single-step
audit rejection and a whole-interval attempt limit after a verified valid prefix
both preserve original positions, velocities and material/interface histories.
These are small-strain coupon dynamics; arbitrary finite-rotation fragment
contact and full game-world integration remain incomplete.

### Consistent fragment inertia and spin projection

`QuadraticDynamics::fragment_rotations` and the finite-elastic wrapper expose a
nonmutating mass-metric projection for each accepted connected component.
`QuadraticFragmentRotation` includes center-based inertia and spin (excluding
orbital angular momentum), angular velocity, translation/rotation kinetic energy
and residual deformation kinetic energy. Inertia uses the full consistent T10
matrix, including valid negative corner row sums, not point-lumped corner mass.
The symmetric three-axis inertia solve rejects singular/unrepresentable output.

Residual energy is integrated from the actual residual velocity field rather
than obtained by subtracting two large energies. The translation+rotation+residual
partition is checked against the original fragment kinetic inventory. Actual
FEM velocities, geometry and history remain unchanged; the report is not a
second owner of mass and not a conversion to independent rigid bodies. The
finite-elastic API delegates the same kinematic calculation without applying a
small-strain plastic constitutive interpretation. Spin is computed from centered
positions/velocities, avoiding subtraction of world-origin orbital momenta.

Validation: all three rotation tests pass. A uniform reference tetrahedron of
mass M reproduces centered inertia diagonal `3M/40` and off-diagonal `M/80`;
arbitrary three-axis rigid spin is recovered with negligible residual energy.
A nonrigid midpoint perturbation retains positive residual kinetic energy and
checks orbital+spin angular momentum against the original fragment inventory.
A 90-degree rotated and translated finite-elastic reference reproduces rotated
spin/angular velocity and invariant rotational kinetic energy. These are
kinematic projection checks, not arbitrary rotating-fragment collision proof.

### Owned single-contact fragment impact

`QuadraticDynamics::impact_fragments` (also exposed by finite-elastic dynamics)
uses the existing consistent-inertia normal impulse kernel on the owner's live
velocities. Two normalized interpolation fields must each belong to a different
connected component and identify coincident points. Signed quadratic weights
are supported. The normal points from second to first. Fully free bodies are
required; pinned impacts need an explicit support-reaction ledger.

The candidate updates live velocities only after restitution, actual kinetic
energy, linear and angular momentum checks pass. `QuadraticEnergy` now exposes
`impact_dissipated_j`, an owned irreversible restitution-loss inventory, separate
from friction/constitutive work. A lost positive increment or overflow rejects
atomically. It is not automatically deposited as heat. Pose, mass, strain and
crack histories do not change during the instantaneous impulse. Existing
mechanical-step energy differences are unchanged because this inventory stays
constant during subsequent conservative steps.

This is one caller-identified fixed-normal collision, not contact discovery,
CCD, simultaneous-contact iteration or a frictional impulse solve. Coincidence
and distinct-component checks protect force/moment exchange but do not prove
that arbitrary supplied fields describe actual exposed T6 surface points.

Validation: all three `fragment_impact` tests pass. They cover restitution
coefficients 0, 0.5 and 1, off-center spin, linear/angular momentum and kinetic
energy plus retained impact loss, rotated/translated finite geometry, and
atomic rejection of noncoincident or same-fragment point pairs. Both existing
`quadratic_normal_impact` kernel tests also pass. Focused rustfmt checks and
`git diff --check` pass; this does not validate automatic world collision handling.

### Coulomb friction during fragment impact

`ConsistentInertia::frictional_impact` and the owned
`impact_fragments_with_friction` APIs add a tangential impulse opposing the
pre-impact relative slip. The scalar consistent FEM mass matrix acts identically
on all spatial axes, so normal and tangent responses decouple: the stopping
impulse is effective mass times slip speed, capped by the supplied Coulomb
coefficient times normal impulse. Sticking stops slip without reversing it;
separating contact receives no friction impulse. Tangential loss is computed
from impulse work and retained together with restitution loss. Nonfinite inputs,
overflow and unrepresentable positive loss reject; owned updates remain atomic.
This is an instantaneous impact model, not persistent static contact friction.
Automatic contact discovery, CCD and coupled simultaneous contacts remain open.

Validation: all four owned `fragment_impact` tests and all three
`quadratic_normal_impact` tests pass. The Coulomb test checks the analytic
impulse cap in frictionless, sliding and sticking cases, nonreversal of slip,
and actual consistent-mass kinetic energy closure. The owned friction test
checks global linear/angular momentum, retained loss and atomic rejection of
invalid friction. Focused rustfmt checks and `git diff --check` pass.

### Automatic fragment node-to-face sweeps

`QuadraticBody::fragment_sweeps_at` builds exposed boundary-node/face pairs
between distinct accepted connected components and applies the existing bounded
continuous T6 clearance search to supplied linear nodal trajectories. It returns
both feasible witnesses and unresolved intervals; separated queries are omitted.
A caller-supplied total query limit is checked before geometry searches. No
body state changes. This avoids relying solely on end-step overlap for these
node-to-face pairs. It does not cover edge-edge crossings, prove first impact
time, apply collision impulses, or change topology during a trajectory. A
clearance witness is not a coincident point: it cannot be passed directly to the
owned impact API without advancing/refining the contact geometry.

Validation: both `fragment_sweep` tests pass. They verify detection during a
crossing with separated endpoint geometry, no false witness for static separated
surfaces, distinct-component pairing, query-budget rejection without mutation,
and invalid-limit rejection even when there are no interfragment queries.
Focused rustfmt checks and `git diff --check` pass.

### Earliest node-to-face clearance bracket

`QuadraticFace::first_point_clearance_at` refines a feasible clearance witness
with bounded searches over trajectory prefixes. The returned lower time bound
has a separated preceding prefix; when a witness exists, the upper bound is its
feasible time. `converged` requires both a witness and the requested time-bracket
width. Unresolved searches preserve uncertainty and are never treated as
separation. An initially unresolved search returns its earliest unresolved
interval with no witness and `converged=false`. Prefix-search and per-search
budgets are explicit. None indicates separation for the complete query.
The query uses prescribed linear trajectories and floating-point T6 bounds; it
is not signed-solid penetration, an edge-edge test, an impulse application, or
an automatically advanced mechanical step.

Validation: all four `quadratic_sweep` tests pass. The new first-clearance test
brackets analytic planar entry at t=(0.5-0.01)/2 within a 1e-6 normalized-time
interval and checks a separating trajectory. Existing tests retain curved
crossing, moving-target/common-observer motion, budget exhaustion and invalid
query coverage. Focused rustfmt and `git diff --check` pass.

### Earliest clearance across accepted fragments

`QuadraticBody::first_fragment_clearance_at` enumerates exposed node/face pairs
and refines their possible events. Its global lower bound is the minimum
separated-prefix bound across every nonseparated candidate, including unresolved
ones. Its upper bound is the earliest feasible witness (or 1 with no witness).
The witness index identifies the associated component pair and surface weights.
A later converged pair cannot conceal an earlier unresolved pair. Global
`converged` requires a witness and the requested global bracket width. Per-pair
query/refinement budgets remain explicit. This read-only query does not advance
constitutive history or position, and retains the node-face coverage limits.

Validation: both `fragment_sweep` tests pass with the added global-query checks.
The crossing bracket contains analytic entry time; a one-interval search budget
retains unresolved candidates and does not report global convergence. Accepted
positions remain unchanged. Focused rustfmt and `git diff --check` pass.

### Transactional mechanical prefix before fragment clearance

`step_loaded_before_fragment_clearance` on small-strain and finite-elastic
quadratic dynamics tries the actual loaded Verlet step on a clone, checks the
linear drift of its first-kicked velocities against exposed accepted-component
node/face surfaces, and halves the duration if any query finds clearance or is
unresolved. Only a separated prefix is committed. Its energy allowance scales
by accepted/requested duration. Attempt exhaustion and minimum-step rejection
retain all original positions, velocities and histories. The report identifies
the actual accepted duration; callers must consume remaining time explicitly.
This is conservative dyadic stopping, not advancement to the exact impact time
or automatic restitution. The same recoverable dynamic energy, inversion, friction-iteration and surface
guard errors as the main adaptive integrator also trigger halving. Other
constitutive or input errors reject immediately. Edge-edge crossings and newly created crack surfaces during
the step remain outside this guard's coverage.

Validation: all three `fragment_sweep` tests pass. The mechanical test starts
with two separated tetrahedra and a 1 s crossing drift, accepts only its 0.25 s
separated prefix, checks actual translated position and energy defect, and
checks complete position/velocity rollback when only one attempt is allowed.
Focused rustfmt and `git diff --check` pass.

Validation of recoverable-error refinement: all four `fragment_sweep` tests
pass. A single-tetrahedron nonrigid velocity case first rejects a 0.01 s ordinary
step under a 1e-8 J energy allowance, then accepts a shorter guarded step within
its duration-scaled allowance. Existing collision stopping and rollback checks
still pass. Focused rustfmt and `git diff --check` pass. Birth-time-aware crack
surface contact is still incomplete: sweeping newly exposed faces backwards
through their bonded coincident state would incorrectly prevent fracture.

### Bounded interval progress toward fragment clearance

`QuadraticDynamics::advance_loaded_until_fragment_clearance` consumes separated
prefixes and returns advanced time, remaining time, absolute energy defects,
accepted step reports and an explicit stop reason. A minimum-step, per-step
attempt, interval-step or residual-time limit is a partial-progress outcome;
only accepted separated prefixes are committed. Other errors roll back the
whole interval. The interval energy allowance is apportioned by duration and
summed over accepted steps. Unrepresentable time increments reject atomically.
This API allows a world loop to hand an approaching contact to its collision
handler rather than repeat shrinking steps indefinitely. It does not itself
resolve contact; a positive-clearance guard can also stop separating motion
that starts inside its layer, requiring appropriate collision/contact handling.

Validation: all five `fragment_sweep` tests pass. The interval crossing case
commits positive separated progress before analytic contact entry, reports its
remaining time and guard stop, preserves consistent-mass fragment centers and
momenta, and stays within the cumulative energy allowance. A static separated
single tetra consumes the full interval with no stop reason. Individual FEM
nodes are not required to remain rigid during a sequence of deformable steps;
center-of-mass transport is the verified invariant. Focused rustfmt and
`git diff --check` pass.

### Automatic single proximity impact

`QuadraticDynamics::impact_nearest_fragment_node` searches exposed accepted
component node/face pairs with the bounded T6 closest-point solver, then selects
the nearest approaching pair inside a caller-supplied capture distance. It
constructs authoritative source-node and target-face shape weights and invokes
`impact_fragments_along_gap`. This frictionless impulse lies along the actual
point separation, so the two nodal resultants have zero combined torque despite
a positive gap. The same owned mass/energy/momentum gates and irreversible loss
inventory apply before commit. Query-budget or projection failure is atomic.

This is an opt-in finite capture-distance, two-sided proximity impact, not exact
zero-gap signed solid collision or a simultaneous-contact solver. Only one pair
is updated per call; repeatedly applying it is not a coupled-contact solution.
A proximity impulse with tangent friction would introduce an uncancelled couple
between separated points, so the existing frictional API continues to require
coincident contact points. The caller must choose an appropriate capture layer
and integrate with the trajectory guard and persistent contact treatment.

Validation: all four `fragment_impact` and all six `fragment_sweep` tests pass.
The automatic proximity test discovers a 0.01 m gap, produces no impact with
0.005 m capture distance, rejects an insufficient pair budget without velocity
changes, and verifies linear/angular momentum plus kinetic energy and retained
loss after capture. Existing coincident frictional and rotated finite-elastic
impact tests still pass. Focused rustfmt and `git diff --check` pass.

### Coupled perfectly inelastic normal impacts

`ConsistentInertia::inelastic_impacts` assembles full contact compliance
J M^-1 J^T and solves nonnegative impulses with nonnegative final relative
normal velocities and complementary active constraints. Projected Gauss-Seidel
is bounded by explicit sweep count and absolute m/s residual tolerance, for up
to 64 caller-defined constraints. Initially separating contacts remain in the
system and can activate through other impulses. Actual returned velocities are
checked against residual tolerances; impulse work must be finite; positive work exceeding the explicit floating-point
summation bound rejects.
The reported loss is minus actual midpoint impulse work. Failure returns no
updated state. The kernel is frictionless and perfectly inelastic; it does not
perform contact discovery, owned-state commit, geometry/torque validation,
restitution coupling or persistent contact. These remain integration work.

Validation: all four `quadratic_normal_impact` tests pass. The new three-body
two-constraint case matches analytic impulses (7/3 and 2/3 times inverse-corner
effective mass), activates an initially separating constraint, rejects a single
iteration budget, retains the same velocities under reversed constraint order
within tolerance, conserves summed linear momentum and closes actual consistent
kinetic energy plus loss. Focused rustfmt and `git diff --check` pass.

### Atomic owned coupled fragment impact

`impact_fragment_contacts` on small-strain and finite-elastic quadratic dynamics
validates each supplied pair of normalized interpolation fields against distinct
accepted components. Coincident points or bounded positive gaps parallel to the
normal are accepted. It then solves the full perfectly inelastic normal contact
system and performs actual owner kinetic-energy/loss and linear/angular momentum
checks before one commit. Single-contact and coupled APIs share geometry and
commit checks. The owned irreversible impact loss is updated once, with overflow
and lost-positive-increment rejection. Failure preserves the whole owner.
The caller still identifies contacts; this is free-body, frictionless contact
with zero restitution, not automatic manifold discovery or frictional support
reaction. Arbitrary caller fields are not proof of exposed surface membership.

Validation: all five `fragment_impact`, six `fragment_sweep` and four
`quadratic_normal_impact` tests pass after shared-check extraction. The owned
three-fragment off-center contact case retains loss, conserves total linear and
angular momentum, and closes actual kinetic energy plus loss. A failed one-sweep
solve and an invalid later contact leave velocities and loss unchanged. Existing
single frictional, finite rotated and automatic proximity checks still pass.
Focused rustfmt and `git diff --check` pass.

### Automatic coupled node/surface impact manifold

`QuadraticDynamics::impact_fragment_nodes` collects one nearest exposed target
surface per exposed source node and target accepted component within the capture
distance, including initially separating constraints. It sends the complete
collected set to the owned coupled perfectly inelastic normal solver. Surface
adjacency therefore does not insert several constraints for the same source
node/component pair. Reverse-direction source queries can yield redundant
constraints; the solver retains them. More than 64 constraints rejects before
commit, rather than truncating the manifold. The report exposes source/target
components, selected faces, gaps and solved impulses. Single-nearest impact now
shares this nearest-component-surface collection before choosing an approaching
pair. The query remains two-sided, node-based, frictionless and capture-distance
based; exact impact timing, edge-edge coverage and persistent friction remain
separate work.

Validation: all five `fragment_impact` and six `fragment_sweep` tests pass.
The expanded automatic test verifies multiple discovered contacts, unique
source-node/target-component keys, nonnegative final normal velocities within
1e-8 m/s, total linear/angular momentum and actual kinetic energy plus owned
loss. The same automatic test passes again after restoring explicit nonfinite
relative-velocity rejection in single-nearest selection. Focused rustfmt and
`git diff --check` pass.

### Guarded coupled Newton restitution

`ConsistentInertia::restitution_impacts` and the owned
`impact_fragment_contacts_with_restitution` extend the coupled normal system
with a shared coefficient e in [0,1]. Approaching constraints target -e*g_before;
initially separating constraints retain zero target and may activate. The
actual returned velocities must meet complementarity against these targets.
Actual midpoint impulse work must remain nonpositive within its explicit
floating-point summation allowance; energetic incompatibility
rejects instead of turning added kinetic energy into a fabricated negative loss.
The inelastic API delegates with e=0. This is a guarded Newton target model,
not a universally admissible restitution law for arbitrary contact networks:
activating initially separating contacts can make even e=1 targets inject energy.
Such networks require a different coupled restitution/expansion model. Owned
updates retain the existing atomic momentum and loss-inventory gates.

Validation: all five `quadratic_normal_impact` and five `fragment_impact` tests
pass. A single coupled constraint matches the original analytic restitution
kernel at e=0,0.5,1. The three-body network accepts e=0.5 with energy closure,
but rejects its energy-injecting e=1 targets. The owned rejection preserves
velocities and loss, and the admissible owned rebound closes kinetic energy plus
retained loss. Focused rustfmt and `git diff --check` pass.

### Coupled impulse-work rounding ledger

`QuadraticMultiImpact` now reports `energy_defect_j` and
`energy_roundoff_bound_j`. Midpoint impulse work uses an explicit summation
allowance based on absolute impulse/old/new velocity products and node count.
Positive work above that allowance rejects; positive work within it remains an
explicit defect with zero retained loss, rather than becoming negative heat.
Negative work is retained as impact loss as before. This floating-point bound
is a numerical allowance, not an interval-arithmetic proof or calibration error
budget. Owned actual-energy/momentum checks remain in force independently.

Validation: all six `quadratic_normal_impact` and five `fragment_impact` tests
pass. Five oblique elastic normals under common observer motion preserve the
restitution condition, retain only negligible loss and report positive rounding
defect separately within the explicit bound. Actual consistent kinetic closure
is checked; the previous genuinely energy-injecting coupled e=1 case still
rejects. Focused rustfmt and `git diff --check` pass.

### Automatic coupled restitution entrypoint

`impact_fragment_nodes_with_restitution` now passes the entire automatically
collected nearest-component-surface manifold to the guarded Newton restitution
solver and the owned acceptance checks. The old automatic API delegates with
zero restitution. Both APIs are also available on finite-elastic dynamics.
Invalid restitution rejects before geometry work; incompatible energy targets
preserve the original owner. The contact discovery and capture-layer limits
remain unchanged. This connects automatic manifold discovery to admissible
coupled rebound, while friction and continuous impact advancement remain open.

Validation: all six `fragment_sweep` tests pass. The expanded automatic test
rejects NaN restitution without velocity changes, discovers multiple contacts
at e=0.5, checks every final normal velocity against its restitution target,
and verifies total linear/angular momentum and actual kinetic energy plus
retained loss. Focused rustfmt and `git diff --check` pass. Finite-elastic API
forwarding compiles; a separate finite automatic-manifold scenario is not proved
by this small-strain fixture.

### Finite automatic-manifold covariance evidence

The dedicated `finite_automatic_rebound_is_covariant_on_prestrained_rotated_translated_geometry`
test passes. Both bodies are prestrained by an affine stretch before finite
elastic dynamics construction, then one scenario is rotated about two axes and
translated. Automatic e=0.5 manifold rebound preserves mass, total linear and
angular momentum, actual kinetic energy plus owned loss, and unchanged stored
elastic energy. Returned velocities match the rotated baseline within 1e-7 m/s
with 1e-10 m closest-search tolerance. This replaces the earlier missing finite
automatic-manifold scenario evidence; it does not prove arbitrary curved contact,
friction, edge-edge CCD or long-run constitutive/contact coupling. Focused rustfmt
and `git diff --check` pass.

### Current FEM render-surface extraction

`QuadraticBody::surface_triangles` returns uniformly subdivided exposed T6
surface triangles with current SI positions, analytic current normals,
barycentric coordinates, reference-face identity and accepted component IDs.
It uses the same exposure rule and T6 interpolation as the contact queries;
fully bonded interface sides stay hidden and fully broken sides become visible.
A caller-specified triangle budget is checked before output allocation, with
bounded subdivision depth. Singular sampled tangents or nonfinite geometry
reject without mutating the body. This supplies a renderer-facing surface
snapshot; it is not yet uploaded into the Voxy renderer, and its polygonal
approximation does not replace the curved collision/search surface.

Validation: both `quadratic_render_surface` tests pass. The fracture coupon
changes from six exposed faces on one component to eight on two components,
matching the contact exposure query. The analytic quadratic shear z'=z+0.2*x²
matches tessellated positions and inverse-transpose normals at subdivision depth
two (64 triangles), and a triangle-budget failure leaves positions unchanged.
Focused rustfmt and `git diff --check` pass. GPU upload and native visual proof
are not established by these geometry tests.

### FEM surface bridge into the scene renderer

`voxy_app::fem_surface::fem_surface_scene_mesh` converts current exposed T6
surface triangles into the existing validated `SceneMesh` vertex/index format.
A caller-supplied color function receives component identity and analytic current
normal; barycentric UVs and an explicit SI origin/display scale are preserved.
Invalid display transforms or unrepresentable f32 geometry reject. Physics state
is read-only. The new `fem_snapshot` example uploads before/after accepted
prescribed cohesive-fracture geometry to the existing GPU scene renderer and
reads back pixels. Its colors and simple normal-based shading are diagnostics;
it is not yet a native continuously simulated FEM world or calibrated material
appearance. The prescribed fracture fixture does not prove dynamic fracture
energy closure, which has separate tests.

GPU validation: `cargo run -p voxy_app --example fem_snapshot -- /tmp/voxy-fem.png`
passes with validation-error scope and readback. Frame 0 has six exposed faces,
one component and 96 triangles; frame 1 has eight exposed faces, two components
and 128 triangles. Readback contains 41,253 first-component pixels before,
33,503 after, and 15,504 second-component pixels after fracture. The saved
before/after image was visually inspected: the separated blue fragment and gap
are visible. Focused rustfmt and `git diff --check` pass. This proves the existing
renderer consumes the current FEM snapshot, not continuous main-world dynamics.

### Native dynamic FEM scene

`SceneApp::with_fem_fracture` and `cargo run -p voxy_app --example fem_fracture`
run an owned cohesive-fracture coupon in the existing native update/render loop.
Initial separating velocities drive fracture under elastic/cohesive forces;
there is no prescribed post-break pose. Accepted dynamic work/fracture steps
feed `fem_surface_scene_mesh` each frame. Space pauses and R restores the initial
state. The diagnostic simulation plays 0.04 physical seconds at 0.03 physical
seconds per wall second, then holds the final state; parameters/colors are
illustrative. It does not include dust, wear, water or full fragment contact.

Validation: `--smoke` passes on Apple M4 Max/Metal with 120 presented frames and
884 accepted physical substeps, mass and both momenta preserved, 5 J accepted
fracture loss, and separate mechanical/interface work budgets satisfied.
`dynamic_fem_snapshot` passes GPU validation/readback: six faces/one component/
96 triangles become eight faces/two components/128 triangles, with 10,225 visible
second-component pixels after fracture. `/tmp/voxy-fem-dynamic.png` was viewed
and confirms the changed surface and second fragment. Focused new-file rustfmt
and `git diff --check` pass. These checks prove the diagnostic native scene,
not full voxel-world integration or calibrated game-frame performance.

### Moisture-driven fracture render preview

`wet_fem_snapshot` uses the existing finite-supply moisture/cohesive update to
weaken a preloaded interface, then renders the same accepted finite-elastic
body through the FEM scene adapter. `FiniteQuadraticDynamics::body` exposes its
immutable current mesh for shared geometry/render queries. The preview owns
the body, cell water and source inventories; water uptake, density/modulus and
cohesive updates are staged on a clone and publish together after verification.
It checks total water across sources/material, added solid mass, transfer energy
defect and the one-to-two component transition. Material-parameter work remains
explicit. Capacities, dry density and dry/wet strengths here are illustrative,
not measured material calibration. This fixed-pose wetting preview does not
prove evaporation, drying, thermal-water closure or subsequent free motion.

GPU validation: `wet_fem_snapshot` passes readback/validation. The material
retains 0.1998001998001998 kg water and sources retain 0.0001998001998069765 kg,
with total initially 0.2 kg within tolerance. Solid mass increases by retained
water; combined reported material/cohesive parameter work is -1.2335526315789456 J.
Exposure changes from six faces/one component to eight faces/two components,
with 9,316 second-component pixels in the wet frame. `/tmp/voxy-fem-wet.png`
was inspected; the newly separated component is visible. Focused new-file
rustfmt and `git diff --check` pass.

### Closed isothermal material–vapor exchange

`moisture::Body::advance_vapor` couples the existing implicit cell network to a
finite `VaporReservoir`. Material and vapor water publish together; evaporation
withdraws latent heat from an explicit thermal inventory and condensation
returns it. Invalid links, insufficient heat and failed balance checks leave
both owners unchanged. The transfer report records material/vapor mass changes,
signed latent exchange and water/energy defects.

Vapor capacity is saturated mass at a caller-specified fixed temperature and
volume. Material saturation is assumed to represent calibrated equilibrium
activity. This is an isothermal inventory foundation, not a universal sorption
law: it does not calculate changing temperature, sensible heat, pressure,
supersaturation or flowing gas, and is not yet coupled to the FEM wetting preview.
The combined network limit is 128 cells, including the vapor cell.

Validation: `cargo test -p physics --test moisture_vapor --test moisture` passes
all six tests. New checks cover the analytic two-capacity implicit solution in
both evaporation and condensation directions, atomic rejection on exhausted
heat/duplicate links, and 1,000 closed steps approaching activity equilibrium
while preserving water and accounted latent/thermal energy within tolerance.

`FiniteQuadraticDynamics::advance_moisture_vapor_with_cohesion` stages this
exchange with the existing wet inertia/modulus and cohesive update, then
publishes solid, material water and vapor together. It validates the previous
solid/material inventory mapping before transport. Bulk reports retain outgoing
water momentum/kinetic energy and material parameter work; the vapor inventory
does not own gas momentum, sensible heat or a temperature field. Consequently
this bridge closes water/latent inventory and updates solid density/properties,
but does not yet close the complete gas–solid mechanical/thermal system.

Bridge validation: the focused vapor and quadratic wet update suites pass all
10 tests, including drying-induced solid mass reduction, exported water momentum
and rollback of all three owners after a mechanical failure. An additional
condensation regression checks analytic incoming momentum and kinetic mixing
loss and passes in the nine-test wet update suite. Passing these checks does
not establish dynamic gas momentum closure.

`advance_vapor_loaded` extends the vapor bridge to a complete atomic interval:
isothermal activity exchange and wet material/cohesive updates precede adaptive
loaded motion. A later integration failure restores all three owners. This is
first-order operator splitting; gas momentum and thermal evolution retain the
limitations above. New regressions check condensation with analytic entrainment
momentum/mixing loss, and moving-body translation after condensation with a
whole-interval rollback on invalid dynamic loads. The focused quadratic wet update suite passes all nine tests, including both
new regressions. No game-world integration or timestep convergence claim is
implied by the wrapper.

### Temperature-dependent finite vapor inventory

`moisture::ThermalVapor` uses the existing liquid `SaturationCurve` and ideal
vapor capacity p_sat(T) V/(R_v T) in a fixed volume. Its effective constant heat
capacity owns sensible energy C T, with total accounted energy C T + L m_v.
`advance_thermal_vapor` freezes capacity for the implicit mass exchange, then
recovers temperature from the accepted latent withdrawal and recalculates
capacity. Evaporation cools this store; condensation into material heats it.
Both owners commit atomically. Out-of-domain temperatures or supersaturation
reject; this does not yet create mist/condensate droplets. This is first-order
splitting with constant L/C, not a full solid–gas enthalpy model: sensible heat
carried by exchanged water, separate solid temperature and gas mechanics remain
absent. The new regression checks cooling/heating, mass/energy inventories and
rollback at the saturation-curve domain boundary. All four vapor tests pass, including this temperature-dependent regression;
parameters are illustrative rather than calibrated water data.

`MaterialThermalStore` supplies a separate lumped material temperature with
constant effective heat capacity. `ThermalVapor::exchange_material_heat` uses
the exact two-store conduction solution at fixed W/K conductance, preserving
combined sensible/latent energy and updating saturation capacity at the new gas
temperature. Invalid rates, saturation-domain violations, supersaturation and
unrepresentable heat updates reject without mutating either store. The analytic
regression uses unequal heat capacities and checks transferred heat, both
temperatures, total energy and rollback; the six-test vapor suite passes.
This operation transfers no water. Coupling the material sensible enthalpy to
mass exchange, spatial solid conduction, FEM temperature-dependent constitutive
laws and gas transport remains unfinished.

`Body::advance_enthalpy_vapor` extends the lumped model to closed mass/enthalpy
transfer. Current effective heat capacities must include retained water. A
constant water specific heat shared by both phases changes the store capacities
by ±c_w Δm. Sensible enthalpy uses the donor's initial temperature; vapor also
carries latent energy L Δm. Evaporation consumes material latent heat and
condensation returns it to material. The combined stores own C_m T_m + C_g T_g
+ L m_v, with atomic water/energy/domain validation before publication. Activity
transport still freezes the initial saturation capacity, so this is first-order
splitting, not simultaneous thermal/chemical equilibrium. Distinct liquid/vapor
specific heats, spatial temperature fields, gas motion, supersaturation droplets
and coupling of these temperatures to FEM material laws remain unfinished.
An analytic evaporation/condensation regression checks updated temperatures,
water and combined energy, plus rollback on overflow; the six-test vapor suite
passes. Sensible energy now uses each accepted gas-link flux and its donor
temperature separately, including opposing evaporation/condensation with zero
net gas mass. A new two-cell counterflow regression checks the nonzero heat
transfer in that case; all eight vapor tests pass.

`advance_heated_vapor` composes half-interval conduction, donor-enthalpy water
transfer and another half conduction using updated heat capacities. All three
owners publish only after the whole interval passes its energy check; even a
failure after the initial heat exchange restores the original stores. The mass
solver is still first order, despite symmetric placement of conduction. New
verification exercises 1,024 coupled steps, cumulative water/energy accounting,
whole-interval rollback and time refinement (8/16/32 steps versus 1,024 over the
same second). All eight focused vapor tests pass; time refinement reduces
errors in vapor mass and both temperatures at each halving (by more than 30%).
This does not establish full spatial thermal transport, arbitrary condensation
droplets or FEM integration.
The rollback assertion added to the coupled long-run test after the first
binary was built passes in its separate focused rerun.

`FiniteQuadraticDynamics::advance_heated_vapor_loaded` stages the closed lumped
heat/water interval, wet inertia/material/cohesive updates and adaptive loaded
motion as one transaction. Previous water/solid mass mapping is checked before
transport; solid, material water, gas and material thermal store publish together.
Its new regression checks condensation-driven mass change, thermal energy,
analytic free translation and rollback of all four owners on invalid dynamic
loads. The combined vapor/wet suite passes all 19 tests before mixing-heat
conversion. Mechanical kinetic mixing losses, fracture
losses and signed material-parameter work remain separate report terms; they
are not automatically deposited into the thermal stores. Constitutive laws
still use moisture calibration, without temperature response. This is therefore
an atomic thermo-moisture/mechanics bridge, not completed total energy closure
or full spatial thermomechanics.

Closed donor-enthalpy exchange now also requires positive dry/background heat
capacities after subtracting c_w times the current retained water in each store.
This enforces the documented effective-capacity convention before transport and
rejects hidden negative background capacity without changing any owner. The
new regression covers invalid capacities in either material or vapor. The full
vapor/wet-mechanics rerun passes all 19 tests including this constraint.

The heated mechanical interval now deposits free-body water entrainment/mixing
kinetic loss into its material thermal store before motion. All of that heat is
assigned to material as an explicit lumped partition assumption. Supported-body
loss can include support work and remains an external report term; fracture
surface energy and other mechanical losses are not converted by this change.
`deposit_heat` rejects invalid/unrepresentable additions atomically. The combined
regression now checks thermal gain against reported mixing loss and total
kinetic-plus-thermal energy for zero-load free translation. The newest heat conversion passes the focused combined vapor/wet suite
(20 tests), including total kinetic-plus-thermal closure in free translation.

Before mechanical motion, the heated wet bridge now checks kinetic-plus-thermal
exchange against the signed kinetic energy carried by water. Supported-body
unconverted loss remains explicit in that balance. Material/cohesive parameter
work stays separately accounted by the existing constitutive update checks.
The gate stages all owners and rejects nonfinite or excessive defects before
publication. New heat-deposition tests check analytic energy gain and atomic
rejection of negative, infinite or unrepresentably small positive additions.
The focused vapor/wet suites including this gate pass all 20 tests.
Validation command: `cargo test -p physics --test moisture_vapor --test quadratic_wet_update`.
Focused rustfmt and `git diff --check` also pass. These checks validate the
lumped heat/water and finite-elastic bridge; spatial thermomechanics, gas flow,
condensate droplets and calibrated full-world behavior remain unproven.

### Explicit temperature/moisture material calibration

`moisture::ThermalCalibration` interpolates user-provided cold/hot dry/saturated
property endpoints over a positive bounded Kelvin interval. It validates the
result through existing plastic/wear material constructors and rejects any
extrapolation. This is an empirical bilinear calibration, not a universal
weakening law, phase transition or thermal expansion model. Measured endpoints
are still required for realistic material-specific predictions.

`FiniteQuadraticDynamics::apply_thermal_moisture` applies per-cell temperatures
through the existing atomic wet inertia/elastic update. Accepted modulus changes
retain explicit elastic parameter work; they are not implicitly drawn from the
thermal store. The finite model still applies only E/nu; yield/hardness/wear data
are available to their own constitutive consumers. Thermal expansion, a thermal
free-energy law, temperature-dependent plastic history and temperature-dependent
cohesive laws are not implemented here. The new prestrained-body regression
checks the analytic elastic energy ratio, parameter work, unchanged mass,
bilinear midpoint and rollback outside the calibration domain. The focused
`temperature_changes_elastic_response_with_explicit_parameter_work` test passes.

`ThermalCohesiveCalibration` adds bounded cold/hot dry/saturated cohesive data.
`FiniteQuadraticDynamics::apply_thermal_cohesive_moisture` resolves supplied
interface temperatures before the existing atomic cohesive/history migration.
Temperature and face saturation remain explicit caller inputs. No automatic
cell-to-interface temperature reconstruction is assumed. Parameter work remains
separate, fracture work cannot be erased, and healing migrations reject. New
verification heats an already damaged cohesive state to complete failure,
checks stored-energy work and preserved fracture energy, then rejects healing
and out-of-domain temperatures. The focused cohesive/wet suite is pending.
Neither thermal expansion nor a coupled temperature-dependent plastic/free-
energy law is established by this empirical calibration.

The existing `wet_fem_snapshot` also accepts `--heated`:

```sh
cargo run -p voxy_app --release --example wet_fem_snapshot -- /tmp/heated-fem.png --heated
```

This mode transfers heat from finite vapor to a material thermal store, evaluates
calibrated temperature-dependent bulk/cohesive properties, and advances loaded
motion through `advance_heated_vapor_loaded_calibrated`. Both thermal owners
remain in the preview's accepted state. Water stays zero in this heat-only test;
parameter work remains an explicit mechanical report. The fixture uses synthetic
calibration endpoints, not measured properties of a named material.

Current Apple M4 Max/Metal GPU readback passes both wet and heated modes. In
heated mode, material reaches 572.7227177179662 K; the accepted FEM state changes
from 1 fragment/6 exposed faces to 2 fragments/8 faces, and the image contains
9316 pixels of the second fragment. The snapshot is visually inspected. CPU
thermal/FEM physics is rendered by the existing GPU renderer; this does not
qualify GPU physics, CUDA, other adapters, or full-scene fracture. Evidence and
two-frame images: `artifacts/heated-fem-gpu-2026-10-06/`.

Adding `--motion` to either wet or heated `wet_fem_snapshot` produces a third
frame of loaded motion after accepted fracture. The diagnostic applies opposite
constant accelerations through the exact consistent-mass nodal loads, rather
than changing display coordinates. For the two fixed-volume fixture cells,
density includes accepted water mass. Loads and resulting impulse are explicit;
the separation is externally driven, not spontaneous fracture energy release.

On the current Metal adapter, both wet and heated paths pass GPU readback. In
the heated path, 0.25 s takes 128 accepted substeps: fragment centers move by
+0.125 and -0.125 m, matching 4 m/s² opposite accelerations. Position/velocity
checks use 1e-8 tolerance; momentum is checked against the applied impulse.
The reported energy defect is 7.468718352110493e-12 J. The third image differs
from the fractured frame, confirming accepted physical coordinates reach the
renderer. A CPU test also verifies failure before fracture leaves state intact
and mechanical motion preserves water, supply and thermal owners. Images and
logs: `artifacts/fem-fragment-motion-gpu-2026-10-06/`.

Finite moving sources can now feed the existing impact/spray/film scenario.
`LiquidDemo::new_finite_impacts` and `SceneApp::with_finite_liquid_impacts` share
the existing liquid, finite gas and film owners; restart preserves the combined
mode. Both `liquids` and `liquid_snapshot` accept `--finite-source --impacts`.

```sh
cargo run -p voxy_app --release --example liquids -- --finite-source --impacts
cargo run -p voxy_app --release --example liquid_snapshot -- /tmp/finite-impacts.png --finite-source --impacts --optical
```

The finite source contains 10 kg of liquid plus 90 kg dry source mass. Its
energy reserve includes the configured 300 K liquid sensible energy and 1000 J
mechanical reserve. Species composition is supplied to the existing emission
transaction. Energy tolerance accounts for f64 evaluation at this energy scale;
cumulative emitted momentum/energy and remaining source inventories are checked
independently. No replenishment or energy clipping is introduced. Later fluid,
gas and film transfers retain their existing ledgers. This source-boundary
ledger does not alone close all gravity/wall energy work of the full scene.

Actual current Metal GPU readback passes diagnostic and optical rendering.
Source masses end at 95/96 kg after emitting 5/4 kg, with zero reported source
energy defects. Six CPU tests pass, one unrelated timing test is ignored; a
second-source energy exhaustion restores the entire candidate frame. All app
targets compile; native window presented-frame acceptance is not run here.
Images, initial failure, logs and source digests:
`artifacts/finite-source-impact-2026-10-06/`.

Scene liquid authoring increment: `voxy_gameplay::LiquidSource` is registered as
`game.liquid-source.v1`. Pulses retain explicit SI units, density, particle volume,
aperture and world-space direction. Material identity is a durable asset name;
the liquid owner supplies a resolved ephemeral material slot when preparing the
existing `PulsedEmitter`. No runtime mass, energy or elapsed clock is serialized.
Preparation bounds pulses to 4096, rejects invalid geometry/pulse values and
empty material identity. Thermal/species fields are admitted at attachment.

Registry component capture and JSON roundtrip preserve this configuration. An
independent physics emission check verifies 1 kg emitted with 2 kg m/s momentum.
The full gameplay library has 70 passing tests. The generic scene liquid runtime
is still missing: shared authoring/play preflight returns an explicit attachment
error for this component instead of silently running a scene without its source.
This is a preparatory authoring contract, not completed editor/game fluid
integration. Evidence: `artifacts/liquid-source-authoring-2026-10-06/`.

### Scene liquid owner (2026-10-06)

`voxy_gameplay::SceneLiquidRuntime` binds `game.liquid-source.v1` material names
against an explicit resolved material catalog and owns one shared existing
`physics::liquid::Liquid`. Hosts supply fixed intervals and an optional static
container. Sources emit in deterministic NodeId order, follow world origins,
and add measured nozzle displacement / interval to relative exhaust velocity.
Inactive sources pause pulse clocks while previously emitted fluid keeps moving.
The whole world, source positions and clocks commit together after successful
emission and simulation; a late source budget failure or invalid container rolls
back every owner. Descriptor edits, source addition/removal and foreign scenes
require explicit rebind rather than resetting inventories silently.

`validate_game_descriptors_with_liquid_runtime` admits a scene only with matching
runtime bindings. The ordinary preflight still rejects liquid descriptors:
editor PlaySession attachment, liquid rendering, finite-body recoil, thermal and
species configurations are not yet connected through this authored scene path.
Existing standalone demos continue to own those richer coupled configurations.

Validation: 74 gameplay library tests passed, including shared-world mass and
trajectory, inactive clock/continued motion, moving-nozzle velocity, late emission
and physics rollback, foreign-scene/edit/material/budget rejection. App/editor
all-target release checks passed. Logs: `artifacts/scene-liquid-runtime-2026-10-06`.

### Editor liquid PlaySession attachment (2026-10-06)

Editor document preflight and Play now resolve each liquid material through
`AuthoringProject` using the existing project manifest/path resolver and bounded,
revalidated `ImportInputs`. Material JSON has exactly `rest_density`,
`sound_speed` and `viscosity`; missing/malformed inputs and invalid physics
materials reject admission. Material inputs are pinned for the session, with no
live material reload yet. All resolved sources bind again after detaching the
runtime scene, so editor handles never leak into the playing world.

PlaySession owns the liquid runtime and clears it on Stop. The character-step
schedule declares write access to `liquid.world`; a staged liquid tick observes
world transforms before character motion, then publishes only after that
character/animation step succeeds. This is a source-world transaction, not a
claim of whole-frame rollback across all scheduled scene systems. Sources use
externally powered emission, default fluid configuration/gravity and no authored
container in this editor path. Rendering fluid particles/films in the editor,
solid-fluid collisions/recoil and authored thermal/species settings remain open.

The actual headless editor Play path was tested with a persisted source and
on-disk material: one 1/60-second tick emits 1/60 kg, Stop restores the document,
restart clears particles, and missing material prevents Play without altering
the authoring document. Full editor library regression: 169 passed, 12 ignored;
app/editor all-target release checks passed. These results do not prove a
presented GPU fluid view. Evidence: `artifacts/editor-liquid-play-2026-10-06`.

### Scene liquid affine static collision (2026-10-06)

The scene owner now extracts active `BoxCollider` geometry each fixed tick and
uses exact existing affine-box SAT sweeps, including rotated/scaled/sheared
hierarchies. `physics::liquid::LiquidGeometry` accepts arbitrary unit contact
normals without changing the legacy axis-normal `CollisionWorld` contract.
`Liquid::step_with_geometry` shares the existing adaptive SPH integration,
validates backend normals/fractions, responds in normal/tangent coordinates,
and rolls back all fluid state on overlap, malformed hits or budgets. Scene
source clocks and emitted particles participate in that same outer transaction.
Particle collision support remains an axis-aligned box of configured radius.

Current scene settings use zero restitution/friction. Geometry is frozen during
each tick: moving collider velocity, two-way body recoil, SPH wall-pressure
samples and contact-loss heat deposition are not included. Dissipated normal
kinetic energy is not claimed as conserved thermal energy. Existing dedicated
thermal/dynamic-world physics paths remain separate until authored integration.

Evidence: a 45-degree thin wall deflects a 2 m/s stream to (1,-1,0) m/s with
unchanged mass, without the false initial overlap its broad AABB would create.
Increasing the wall to engulf the stream rejects and restores all runtime state.
Independent oblique restitution tests check expected kinetic loss for e=0,0.5,1;
malformed backend contact after prior advection restores the original fluid.
75 gameplay tests and the editor Play lifecycle test passed; legacy collision
compatibility and app/editor all-target release checks passed. Logs live in
`artifacts/scene-liquid-collision-2026-10-06`. No GPU fluid view is proven here.

### Editor accepted-liquid draw bridge (2026-10-06)

Editor world views and standalone views now draw the accepted PlaySession liquid
state through the existing SceneRenderer and editor material shader. Particle
meshes use octahedra with volume 4r^3/3 = mass / effective rest density; their
centroids track the solver positions. Material slots have diagnostic colors.
This is opaque particle geometry, not the existing standalone optical surface
pipeline, and does not claim refraction, connected fluid surfaces or films.

The bridge is bounded to 16384 particles and uploads only when scene identity or
accepted simulation tick changes. GPU geometry accounting includes fluid and
checks the existing geometry budget including old/new transient storage before
publication. Per-view transforms use each editor view projection; empty or
stopped runtimes clear the draw. No physical state is changed by rendering.

Tests independently check mesh volume/centroid and run actual Metal GPU readback
with the editor shader from a scene emitter's accepted 100 kg emission: 364 blue
pixels, with zero before/after the draw. GPU validation scope is clean. This is
an offscreen GPU qualification, not a presented native editor-window recording.
170 editor library tests passed, 13 ignored; the new GPU test was separately run
successfully. Editor all-target release check passed. Evidence and PNG:
`artifacts/editor-liquid-draw-2026-10-06`. Initial output-path failure is retained.

### Editor optical liquid composition (2026-10-06)

Liquid material JSON now accepts optional `optics: [absorption_r, absorption_g,
absorption_b, ior]`, with absorption per metre and finite nonnegative channels,
finite IOR >= 1. Physical and optical values are admitted from the same bounded,
revalidated project input snapshot. PlaySession retains the resolved slot-order
optics and clears them on Stop. Missing optics selects diagnostic particles;
there is no implicit water IOR assigned to arbitrary materials.

For a single perspective view without MSAA, the editor now uploads equivalent
sphere radii derived from accepted mass / effective density and composes the
existing ScreenSpaceFluidRenderer through SceneSurface::render_custom, including
world background and UI. Targets recreate on resize; split views, orthographic
views and MSAA retain particle geometry. This is an integration increment:
multiview optical composition, optical targets in residency eviction accounting,
authored film cells, material live reload and presented native-window acceptance
remain open. Optical updates are read-only with respect to physics.

Independent CPU checks verify represented sphere volume, explicit material
identity and rejection of negative absorption. The editor Play lifecycle test
now persists optical metadata. Actual M4 Max/Metal readback compares the same
checker background with and without a scene emitter's accepted particles: 408
pixels differ; removing optical particles restores the baseline. Validation
scope is clean. Full editor regression: 171 passed, 13 ignored; separate physical
GPU test and editor all-target release check passed. Evidence:
`artifacts/editor-liquid-optical-2026-10-06`; early API compile mismatch and
black-background fixture threshold failure are retained. No native presentation
or cross-hardware optical support is claimed by this fixture.

### Optical liquid logical GPU residency (2026-10-06)

`ScreenSpaceFluidRenderer::required_allocation_bytes` now computes checked logical
storage before creation: nine single-sample targets (output format texel bytes +
50 intermediate bytes per pixel), both particle/film vertex buffers and camera
uniform. `allocation_bytes` records this exact requested storage. It excludes
driver padding, opaque pipeline/bind-group storage and compositor/surface buffers;
it is not a hardware VRAM measurement. Invalid dimensions/formats and overflow
are rejected. Existing device/adapter capability admission remains in force.

`new_with_adapter_budget` admits other live resources plus the candidate before
any fluid resource creation. Editor geometry residency now includes optical
resources so model, animation and fluid admission share its existing budget.
Replacement admission includes old plus new targets; rejection preserves the old
renderer. Resize failure is reported to the caller, not claimed as successful
low-resolution rendering. Global image-cache budget remains separately owned.

Actual Metal tests compare the estimator against every created target and buffer:
64x32 capacity 8 consumes 111648 logical bytes. Exactly sufficient budget admits;
one byte less rejects. A 128x64 candidate needs 443424 bytes; old+new minus one
rejects without replacing the old size. Integer overflow also rejects and the
GPU validation scope remains clean. Full renderer library qualification,
including physical GPU tests: 164 passed, zero ignored. Editor regression: 171
passed, 13 ignored. Editor/render all-target release checks passed. Evidence:
`artifacts/liquid-gpu-residency-2026-10-06`. Multiview optics, dynamic-body coupling
and full hardware qualification remain open.

### Disjoint editor optical view composition (2026-10-06)

`ScreenSpaceFluidRenderer::encode_viewport` composes into a checked subregion of
an existing target, loading its prior contents and restricting viewport/scissor.
Optical reconstruction now derives local pixel coordinates from interpolated UV,
so output offsets do not change rays or sample another view's textures. Bad
sizes, bounds and integer overflow reject before encoding. Per-view overlays
use the full-window depth attachment with the same region; global UI follows
all optical views.

`SceneSurface::render_scene_views_with_fluids` uses one acquisition/submission,
ordinary scene views first, then independent fluid composition and UI. Duplicate
fluid indices and mismatched sizes reject; MSAA is still unsupported in this
path. Editor owns one budgeted optical renderer per perspective view and prunes
unused views, with transient replacement bytes included in residency. Split
orthographic views retain volume-preserving diagnostic particles. All views
observe one accepted fluid state; rendering does not advance simulation.

Actual M4 Max/Metal GPU readback places optical fluid in the right 64x64 region
of a 128x64 target: 408 pixels differ from the same background without fluid;
all 4096 left-region pixels remain exactly unchanged. An out-of-bounds region
is rejected before the valid render, and GPU validation remains clean. The
existing whole-frame optical baseline/clear tests also pass. Full renderer
qualification: 164 passed, no ignored tests; editor: 171 passed, 13 ignored.
Editor/render all-target release checks passed. Snapshot/logs:
`artifacts/liquid-optical-viewport-2026-10-06`. Native split-window presentation,
orthographic optical reconstruction, MSAA fluid resolve and moving-body coupling
remain unproven/open; these results cover offscreen composition and compiled
editor attachment.

### Finite translating geometry with oblique liquid contact (2026-10-06)

`Liquid::step_with_dynamic_geometry` now accepts the arbitrary-unit-normal
`LiquidGeometry` contract for one finite-mass translating collision template.
Particle sweeps are evaluated in the body's instantaneous translating frame.
The existing event loop chooses the earliest particle/body contact, advances
all participants to that time, applies equal opposite normal/tangent impulses,
and re-queries after body recoil. Rotation remains constrained. The axis-world
and thermal impact paths share this event loop; there is no parallel contact
engine. Global event/query budgets still bound work.

Contact loss is computed in relative normal/tangent coordinates with reduced
mass, restitution and tangential damping. It is reported separately, not
silently converted into heat. Fluid and finite-body state are staged together,
including empty-fluid gravity/drift. Invalid hits, overlaps, backend errors and
exhausted budgets restore both owners.

Tests cover 18 restitution/friction/Galilean-boost combinations with independent
momentum and energy-plus-loss balances. A real scene affine-box template at
45 degrees receives a 1 kg particle moving at (2,0,0) m/s: a 3 kg finite body
recoils to approximately (0.25,0.25,0), particle velocity becomes
(1.25,-0.75,0), reported loss is 0.75 J. Both body axes translate; the immutable
scene template remains unchanged. This proves the geometry bridge and physical
recoil, not authored PlaySession dynamic-body ownership/publication. Multiple
independent bodies, rotational inertia, combined static/dynamic environment
contacts and their scene/editor attachment remain required work.

Evidence: `artifacts/finite-affine-liquid-contact-2026-10-06`, including legacy
axis/thermal contact compatibility, analytic oblique contact and full regressions.

Final finite-contact qualification after the separation correction: 1490 physics
tests passed, 11 ignored; 76 gameplay tests passed. The corrected separation
scale weights only coordinates participating in the contact normal. An independent
1e12-metre orthogonal translation fixture preserves both transverse trajectories
and all velocities exactly. App/editor all-target release checks, formatting and
diff checks passed. `result.json` pins the final source hashes.

### Authored liquid body and static surroundings (2026-10-06)

`DynamicLiquidEnvironment` supplies read-only particle/static and exact
body-template/static sweeps. The existing earliest-event contact loop now chooses
between fluid/body, fluid/static and body/static hits on one timeline, re-querying
all participants after recoil. `DynamicEnvironmentReport` separately records the
impulse received by fixed surroundings, so the moving-system momentum balance
includes its external boundary. All contact kinetic losses remain reported,
without implicit thermal deposition. Empty fluid still advances and sweeps the
body. Late backend failure restores fluid and body together.

Authored `game.liquid-body.v1` contains `mass_kg` and
`initial_velocity_m_s`. SceneLiquidRuntime owns its mechanical state. Current
admission is one root body with one BoxCollider; parented/compound bodies,
multiple bodies and competing CharacterBody/AngularMotion transform owners
reject explicitly. Authored rotation and scale remain fixed. Active static
scene colliders are the environment, and affine SAT sweeps preserve oblique
normals. Initial body/environment overlap rejects preflight. Inactive body state
pauses, and removed/edited ownership requires explicit rebind.

`tick_and_publish` stages source clocks, fluid/body state and representable scene
pose before committing. Editor prepares the candidate at character.step and
publishes the body pose after successful character/animation preparation. Stop
restores the authoring document and clears runtime state; restart uses authored
initial velocity. The body uses f64 mechanical translation and separately records
its published f32 pose, preserving the physical template independent of narrowing.
An unrepresentable publication rejects without changing scene or runtime.
Body-only worlds carry an unused reference fluid material; it emits no particles
and does not supply body mass or density.

Tests prove a finite body striking a fixed wall after fluid impact, reversed event
ordering where a particle/static hit happens first, empty-fluid body/static
collision, late backend rollback and an inelastic body pressed against a wall.
Momentum includes external impulse and kinetic energy includes reported losses.
The actual headless editor Play path runs a persisted source/body, receives recoil,
publishes translation while retaining rotation, restores both descriptors on Stop
and resets velocity on restart. No native/GPU body presentation is claimed here.

Qualification: 1493 full physics tests passed (11 ignored); the subsequently added
resting-wall check also passed, with all 9 focused geometry tests passing.
78 gameplay and 172 editor tests passed (13 editor tests ignored). App/editor
all-target release checks passed. Evidence and final source hashes:
`artifacts/liquid-dynamic-environment-2026-10-06`. Multiple bodies, rotational
inertia, compound templates, deformation, authored heat exchange, contact with
characters and moving kinematic environment velocity remain required work.

### Shared multiple-body contact kernel (2026-10-06)

`LiquidBodyWorld` and `Liquid::step_with_body_world` advance a bounded slice of
finite translating bodies together with fluid and static surroundings. Particle/body,
body/body and static contacts share one earliest-event timeline; the single-body
API delegates to this kernel. Admission or late geometry failure restores all
owners. Three focused tests cover an empty-fluid three-body impulse chain,
fluid recoil followed by a body pair and wall impact, and transactional rollback.
The authored scene runtime still admits only one body; multiple authored bodies,
rotation and compound colliders remain unfinished.
