# Целевая архитектура (волна 3)

Дополняет `docs/architecture.md` (инварианты §3, граф §4) и ADR 0004. Здесь зафиксировано, что именно
меняется в структуре и почему; исходные наблюдения лежат в `INVENTORY.md`, порядок работ в `PLAN.md`.

Принцип прежний: направление зависимостей проверяется тестом, а не договорённостью. Каждое решение ниже
имеет механизм закрепления (ratchet), иначе оно откатится так же, как разрез hair contact после слияния
`be884bf72`.

## 1. Что закрепляется тестами

| Правило | Механизм | Где |
|---|---|---|
| Рёбра между крейтами только из списка | `ALLOWED` / `KNOWN_VIOLATIONS` | `crates/voxy_depgraph/tests/depgraph.rs` |
| Каждый `.rs` под `src/` входит в дерево модулей | обход `mod`-дерева от корней крейта | `crates/voxy_depgraph/tests/sources.rs` |
| Прод-файл не длиннее бюджета | список известных превышений, только сокращается | `crates/voxy_depgraph/tests/sources.rs` |
| Слои модулей внутри `physics` | список разрешённых рёбер `crate::X` | `crates/voxy_depgraph/tests/physics_modules.rs` (шаг 2.3) |
| Длина и число параметров функций | clippy `too_many_lines` / `too_many_arguments` + `#[expect]` с причиной | `Cargo.toml`, `clippy.toml` (шаг 6.2) |
| Раскладка `Pod` == WGSL | рефлексия naga в тесте | `voxy_render`, `voxy_gpu` (шаг 4.4) |

## 2. Граф крейтов

Изменения относительно `docs/architecture.md` §4:

- **Снять `voxy_demos -> voxy_editor`.** Причина ребра: foot placement живёт в editor. Юнит
  (`foot_placement.rs`, `foot_placement/contact_events.rs`, `foot_runtime_checkpoint.rs`) переезжает в
  `voxy_gameplay::foot_placement`. `voxy_animation` не подходит: юнит использует `CharacterTickPreview`,
  `FootContactInput/Settings/State`, `SupportQueryBudget` из gameplay, а gameplay зависит от animation.
  Связей с editor три: `model_playback::SourceContactInterval` (структура из типов animation), `EditorError`
  (создаётся из строки и сразу превращается в строку), `ModelAsset` (нужны только `skeleton`, `animations`,
  `resolve_joint_name`; конструктор с резолвером уже есть). Формат `"editor.foot-placement.v1"` не меняется.
- **Снять `voxy_shell -> voxy_demos`.** Демо взаимоисключающие (каждый `with_*` сбрасывает остальные), поэтому
  девять `Option`-полей `SceneApp` это один слот `Option<Box<dyn SceneDemo>>`. Trait объявляется в новом
  крейте-листе `voxy_scene_host` (зависит от `voxy_render`, `glam`, `wgpu`): если объявить его в `voxy_shell`,
  то при миграции по одному демо shell и demos зависели бы друг от друга. Заодно `voxy_demos` остаётся без winit
  (строка таблицы §4: "voxy_demos не должен знать window, event loop").
  Новые рёбра: `voxy_scene_host -> voxy_render`, `voxy_shell -> voxy_scene_host`, `voxy_demos -> voxy_scene_host`.
- **GPU-решатели контактов** (`banded_*`, `joint_contact_*`, `contact_five_*`) не относятся к рендеру и
  переезжают из `voxy_render` в `voxy_gpu`; публичная поверхность `voxy_render::lib.rs` сужается.
- **Не меняется:** `angular_sweep`/`convex` остаются в `voxy_gameplay`. `physics` намеренно без `glam`
  (`[f64; 3]`), перенос был бы переписыванием.

### Trait `SceneDemo` (эскиз)

Методы это фазы кадра, а не ветки текущего `draw`:

```rust
pub trait SceneDemo: std::fmt::Debug {
    fn window(&mut self, env: &DemoEnv) -> WindowRequest;
    fn create(&mut self, gpu: DemoGpu<'_>) -> DemoResult;           // повторяется после потери устройства
    fn release(&mut self) {}
    fn advance(&mut self, tick: DemoTick) -> DemoResult;
    fn camera(&self, view: DemoView) -> voxy_render::SceneCamera;
    fn prepare(&mut self, frame: &mut DemoFrame<'_>) -> DemoResult;
    fn frame<'a>(&'a mut self, shared: SharedDraw<'a>) -> FrameParts<'a>;
    fn presented(&mut self, info: Presented) -> DemoResult { Ok(()) }
    fn input(&mut self, input: DemoInput) -> DemoResult<DemoReply> { Ok(DemoReply::default()) }
    fn title(&self, paused: bool) -> Option<String> { None }
    fn smoke_ready(&self) -> bool { true }
    fn verify_smoke(&self, frames: u32) -> DemoResult<Vec<String>> { Ok(vec![]) }
}
```

`frame()` возвращает и список draw, и замыкание подготовки одним вызовом `&mut self`: сейчас draw-список
держит геометрию по `&`, а prepare меняет compute-состояние (`scene_app.rs:1651-1880`), и два раздельных
метода `draws(&self)` + `encode(&mut self)` не скомпилировались бы.

## 3. Слои внутри `physics`

Модуль импортирует только из строк выше себя:

1. `math`, `symmetric_eigen`, `triangle_index`, `spatial_bounds`, `positive_transport`, `heat_exchange`, `osmotic`, `rotation`
2. `gravity`, `mass_properties`, `planar`, `strand`, `soft_body`, `secondary_motion`, `respiration`, `tissue_surface`,
   `static_triangle_world`, `friction`, `circulation`, `suspension`, `hair`
3. `astrophysics`, `spin_path`, `contact`, `cohesive`, `lymph`, `character`
4. `rigid_motion`
5. `liquid`, `surface_film` (известная пара до появления `film_impact`)
6. `biomechanics`, `medium`, `moisture`
7. `plasticity`, `skin`, `tissue`
8. `wear`
9. `debris`

Минимальные переносы, делающие граф ациклическим:

| Перенос | Снимает |
|---|---|
| псевдоним `Matrix` в `math.rs` | `friction`, `cohesive -> biomechanics` |
| `solve_protein` в `positive_transport.rs` | `circulation -> biomechanics` |
| `OsmoticPressureLaw` в лист `osmotic.rs` | `lymph -> biomechanics` |
| `astrophysics/spin.rs` -> `rotation.rs` | `rotation -> astrophysics` |
| `astrophysics::Error` в `rotation` | `spin_path -> astrophysics` |
| определение `MaterialForceMoment` в `rotation` (impl остаётся) | `rotation -> rigid_motion`, `spin_path -> rigid_motion` |
| `Properties::{plastic_material, wear_material}` как impl-блоки в `plasticity` и `wear` | `moisture -> plasticity`, `moisture -> wear` |
| `suspension/optics.rs` в `liquid/` | `suspension -> liquid` |
| шесть файлов `liquid/film_*.rs` + `impact_spray.rs` в модуль-связку `film_impact` | `liquid <-> surface_film` |

Разбиение `physics` на крейты откладывается: середина (`liquid`, `surface_film`, `biomechanics`, `moisture`,
`plasticity`, `wear`, ~65k строк) плотно связана, общего ядра ошибок и математики как крейта нет. Первый кандидат
после ацикличности: `hair` (остров, 19.7k строк, самый высокий churn, единственный потребитель двух cargo-фич).
Решение принимается по `cargo build --timings`, а не по числу строк.

## 4. Рендер

- **Под-структуры вместо god-типа.** `ArchiveSceneRenderer` (85 полей) делится по времени жизни ресурсов:
  `Layouts` (создаются один раз), `Pipelines` (зависят от формата и MSAA), `Targets` (пересоздаются на resize),
  `Post`, `Gi`, `Environment` (чистые функции). Конструктор и `draw` делятся по тем же границам.
- **Кадр как упорядоченный список проходов.** Не render graph: достаточно функций `encode_<pass>(&mut Frame)` в
  явном порядке. Три цепочки bloom/tonemap (archive, bodycam, ocean) сводятся к одной реализации только после
  этого, отдельным шагом с проверкой изображения.
- **Конфигурация не читается из окружения внутри рендера.** 66 `std::env::var` в `archive_scene.rs` собираются
  в одну структуру, заполняемую на входе.
- **WGSL-модули.** Маленький резолвер `#include "name.wgsl"` в `voxy_gpu_core` заменяет конкатенацию и
  текстовые `.replace()`. Общие модули: `dd_math`, `fullscreen`, `tonemap`, `camera`.
- **CPU-логика отдельно от GPU.** Skinning-математика и admission, валидация fluid, функции окружения archive
  выносятся в файлы без `wgpu`, чтобы тестироваться без устройства.

## 5. App-слой

- **Editor `App`.** Образец: существующий `AudioSession` (приватные поля + методы). Порядок извлечения от самого
  изолированного: `ViewportCameras` -> `ModelRelink` -> `SmokeHarness` -> предикаты занятости -> asset pipeline ->
  состояние файловых операций -> panel UI. `WindowHost` и `InspectorState` не извлекаются (сборные кластеры).
  Единого enum модальности нет: поля не взаимоисключающие. Метод, переезжающий на под-структуру и вызывающий
  `&self`-хелпер `App`, требует превратить хелпер в свободную функцию над явными полями (пример:
  `required_cpu_model_assets`, `app/residency.rs:62`).
- **Сериализация.** `voxy_assets::JsonDocument` + `Checkpoint<T>` (версия, бюджет байт, prepare/publish) вместо
  77 рукописных функций. Формат каждого документа фиксируется golden round-trip тестом до миграции.
- **Ввод и камеры.** Один orbit-контроллер и одно отображение `KeyCode -> Control`; общий winit frame-host для
  примеров.

## 6. Ошибки

Библиотечные крейты: `thiserror`-enum на домен, текст `Display` сохраняется. `Box<dyn Error>` и `anyhow` только
в бинарях и composition root (`architecture.md` §4). В `physics` типизация начинается с функций на границе крейта;
внутренние `&'static str` конвертируются через `From`.

## 7. Производительность

Правило: бенч в стиле существующих (`harness = false`, `Instant`, несколько проходов), цифра "до", правка, цифра
"после", запись в `BASELINE.md` и `docs/architecture-performance/<дата>-wave-3/`. Без выигрыша правка откатывается.
Общие приёмы, уже имеющие образцы в коде: scratch-структура на попытку/итерацию, `clone_from` в удерживаемый
буфер вместо `clone`, API вида `*_into(&mut out)`, кэш по ревизии вместо пересборки на тик.

Кадр GPU-bound, и половину GPU-времени даёт GI. Это отдельный протокол квалификации изображения
(`docs/architecture-performance/2026-10-27-gi-*`), в эту волну он не входит.

## 8. Чего не делаем

ECS, render graph как фреймворк, criterion, trait под одну реализацию, микросервисы, разбиение `physics` на
крейты до ацикличности, правки Windows-only кода (cuda, vulkan, streamline) сверх комментариев: на macOS он не
собирается и не проверяется.
