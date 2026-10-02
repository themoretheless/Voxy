# Параметры телосложения

`voxy_app::body_parameters::BodyParameters` задаёт непрерывные пропорции импортированной модели. `SceneApp::with_body_parameters` применяет параметры после выбора `with_female`, `with_female_animation` или `with_female_secondary`.

```sh
cargo run --release -p voxy_app --example female_motion -- assets/characters/blender-female/presets/tall.json
```

| Параметр | Единицы / диапазон |
| --- | --- |
| height_cm | Рост, 130–210 см |
| weight_kg | Вес, 35–180 кг |
| breast_size, buttock_size | Размер груди и ягодиц, 0.6–1.6 |
| leg_length, arm_length, torso_length | Длина ног, рук и туловища, 0.6–1.6 |
| shoulder_width, hip_width, waist_width | Ширина плеч, бёдер и талии, 0.6–1.6 |
| head_size, hand_size, foot_size | Размер головы, кистей и стоп, 0.6–1.6 |

Коэффициент 1 означает исходную модель (около 164 см, условный исходный вес 60 кг). Пропущенные поля JSON сохраняют значения по умолчанию; неизвестные поля и значения вне диапазонов отклоняются. После изменения длины сегментов модель нормализуется к заданному росту, подошвы сохраняют высоту. Глаза, детали лица и волосы получают тот же пространственный морфинг.

Вес влияет на толщину через приблизительную зависимость от массы и роста. Это художественная параметризация, а не оценка состава тела, окружностей или медицински откалиброванной массы. Размер груди и ягодиц также меняет частоту вторичного движения (больший размер — меньшая частота). Деформации решаются в исходных координатах и отображаются через тот же морфинг, поэтому области движения следуют новым пропорциям. Перестройка четырёх вторичных областей и панель описаны ниже; полная физическая перестройка кожи, скелета и волос остаётся следующим этапом.

## Отдельные сегменты

Дополнительно доступны коэффициенты 0.6–1.6:

| Параметры | Назначение |
| --- | --- |
| thigh_length, shin_length | Длина бедра и голени |
| upper_arm_length, forearm_length | Длина плеча и предплечья |
| thigh_width, calf_width | Толщина бедра и голени |
| upper_arm_width, forearm_width | Толщина плеча и предплечья |
| torso_depth | Глубина туловища |
| neck_length | Длина шеи |

`leg_length` и `arm_length` остаются общими множителями. Изменения длины сегментов складываются вдоль конечности; колено и локоть сохраняют непрерывность. Общий рост нормализуется после изменения сегментов и шеи. Толщина ног меняется относительно центра каждой ноги. Эти параметры автоматически доступны в `model_schema` и `model_update`; старые JSON-файлы получают для новых полей коэффициент 1.

## Пупок и соски

| Параметр | Значения |
| --- | --- |
| nipple_type | reference, projecting, flat, inverted |
| nipple_radius_mm | Радиус локального профиля, 3–25 мм |
| nipple_projection_mm | Величина выступа или втяжения, 0–12 мм |
| navel_shape | reference, round, vertical, horizontal |
| navel_width_mm, navel_height_mm | Размер профиля, 8–40 мм |
| navel_depth_mm | Глубина, −8–12 мм; отрицательная означает выступ |

`reference` с исходными размерами сохраняет исходную геометрию. Другие варианты задают компактные гладкие локальные профили на передней поверхности, затем применяются общие пропорции тела. Это условные морфы существующего меша с номинальным исходным рельефом, а не реконструкция индивидуальной анатомии. Размеры указаны в исходных координатах: общий масштаб тела меняет итоговый размер. Разрешение мелких деталей ограничено плотностью исходного меша. Настройка симметрична для двух сосков; независимых параметров сторон пока нет.

Пример частичного обновления через MCP:

```json
{"parameters":{"nipple_type":"projecting","nipple_radius_mm":12,"nipple_projection_mm":6,"navel_shape":"vertical","navel_width_mm":18,"navel_height_mm":26,"navel_depth_mm":5}}
```

## Перестройка областей и независимые стороны

`left_breast_size`, `right_breast_size`, `left_buttock_size`, `right_buttock_size` — дополнительные множители сторон (левая сторона: отрицательная X исходного меша). Общий размер умножается на размер стороны; произведение должно оставаться в 0.6–1.6. Через среднюю линию используется гладкий переход.

При изменении параметров через `SceneApp::with_body_parameters` или файл теперь атомарно перестраиваются четыре объёмных клетки вторичного движения: физическая исходная форма, барицентрические привязки, расчётные узловые массы и сферические прокси контакта. Для изменённой геометрии используется условная плотность 1000 кг/м³; это не распределение всего заданного веса тела. Состояние колебаний сбрасывается при перестройке. Нормали освещения рассчитываются по изменённой геометрии.

Полноценный кожный решатель, скелет и волосы пока не перестраиваются по новым физическим метрикам. Сферические прокси остаются приближением. Сгущённая геометрия подготовки не заменяет отображаемый исходный меш автоматически.

Панель: `python3 tools/body-constructor/server.py --binary target/release/model_mcp --parameters assets/characters/blender-female/presets/body-panel.json --port 8774`. Затем открыть http://127.0.0.1:8774/ и запустить `female_motion` с тем же абсолютным путём JSON.

The live body setter now rebuilds the full skin shell from an immutable canonical rest surface, resetting dynamic state and recalculating metrics/masses. Attachment stiffness and viscosity scale with the rebuilt local mass/area. Animation targets are posed in canonical coordinates and morphed into the same space as the shell. Skin displacements are added after morphology, avoiding a second scaling of physical displacement. Existing barycentric topology remains; anatomical re-registration and full rig/hair rebasing are not completed by this change.

`body_model` selects `female` (default) or `male` source anatomy. It is included
in presets, MCP schema/get/update, and offscreen snapshots. The runtime stages a
new mesh/shell/rig with complete bindings before replacing the model; camera,
view mode and parameter-file watches are retained. Simulation state resets for
the new source. Fluid state transfers through verified shared-quad overlap maps. Unsupported
wet surface edits reject the entire switch while preserving the old model and
liquid; sources, optical settings and mass counters are retained.
