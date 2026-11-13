# Инвентаризация workspace (волна 3)

Срез: `main` @ `fc07bb3ff`, 2026-11-09. Цифры получены read-only сканированием (скрипты и grep),
без сборки; точность счётчиков ±5%. Метрики по каждой функции воспроизводятся командой
`cargo run -p voxy_depgraph --bin refactor_metrics` (см. `BASELINE.md`).

**Состояние на 2026-11-12.** Срез `fc07bb3ff` после переписывания истории в репозитории не находится. Числа таблицы §1
пересчитаны 2026-11-12 сканером на рабочем дереве (с незакоммиченными правками), после разделения `physics`;
строки новых крейтов стоят в конце таблицы, колонка «итоги волны 3» не переписывалась. §2 и §3 это снимок **до** рефакторинга
(исходные наблюдения), сохранённый как есть; текущее состояние по каждому пункту ведёт `PLAN.md`.

Статусы: `todo`, `wip`, `done`, `отложено` (с причиной). Приоритет: P1 (высокий churn и сложность),
P2 (ядро), P3 (периферия).

## 1. Крейты

Прод-строки и функции без тестов (`#[cfg(test)]`, `tests.rs`, `*_tests.rs`, `tests/`, `benches/`, `examples/`).

| Крейт | Прод. строк | Функций | > 60 | > 100 | > 200 | Главные запахи / итоги волны 3 | Приоритет | Статус |
|---|---:|---:|---:|---:|---:|---|---|---|
| physics | 75 497 | 2101 | 262 | 98 | 21 | 9-слойный ациклический DAG (`KNOWN_VIOLATIONS = &[]`), единый `physics::math`, все файлы <= 1500 строк | P1 | done |
| voxy_render | 62 371 | 2051 | 188 | 73 | 15 | декомпозированы `ArchiveSceneRenderer`, `SceneRenderer`, `Renderer`, `skinned`; вынесены 34 контактных GPU-решателя | P1 | done |
| voxy_editor | 38 344 | 1022 | 125 | 50 | 23 | вынесен `foot_placement`, сняты модульные `allow(dead_code)`, `PanelCache` по ревизии, все файлы <= 1500 строк | P1 | done |
| voxy_animation | 28 436 | 1090 | 63 | 23 | 0 | разделён `lib.rs` (skeleton/clip/pose/animator/math), zero-alloc `*_into` + keyframe cursor | P1 | done |
| voxy_demos | 29 222 | 812 | 97 | 57 | 15 | снято ребро `voxy_demos -> voxy_editor`, троттлинг `fs::metadata`, декомпозированы крупные модули | P2 | done |
| voxy_gameplay | 19 520 | 570 | 62 | 24 | 7 | перенесён `foot_placement`, выделен `character_physics.rs`, кеш `StaticWorld` по ревизии, `ActionId` | P1 | done |
| voxy_ray_probe | 11 792 | 94 | 45 | 31 | 13 | диагностический бинарь разделён по протоколам (<= 1500 строк), документированы `SAFETY` | P3 | done |
| voxy_scene | 7 935 | 345 | 13 | 3 | 1 | плотный индекс `component_slots` по `TypeId` ($O(|T|)$, ускорение до 4 324x на 100k узлах) | P2 | done |
| voxy_model | 8 105 | 220 | 27 | 14 | 7 | разделён на `gltf_{validate,material,animation}.rs` + `pose.rs`, топосорт $O(N^2)$ | P2 | done |
| voxy_gpu_core | 6 753 | 304 | 12 | 2 | 0 | приняты GPU контактные решатели, `wgsl.rs` (`#include`), `BindGroup*Builder` | P2 | done |
| voxy_app | 6 644 | 148 | 28 | 13 | 5 | разделён на `bootstrap.rs` и `app_tick.rs`, вынесены `controls`/`frame_profile`, сняты `allow(dead_code)` | P2 | done |
| voxy_gpu | 5 748 | 156 | 20 | 7 | 0 | все дескрипторы на `BindGroup*Builder`, реэкспорт GPU-решателей | P2 | done |
| voxy_web | 7 137 | 204 | 29 | 12 | 1 | вычищены clippy-линты, файлы <= 1500 строк | P3 | done |
| voxy_shell | 2 749 | 95 | 6 | 4 | 1 | приняты `controls` и `frame_profile`, `SceneApp` разделён на `init.rs` и `events.rs` | P1 | done |
| voxy_assets | 4 054 | 191 | 5 | 0 | 0 | `JsonDocument` подключён к `Manifest` + golden round-trip тесты | P2 | done |
| physics_voxel | 3 729 | 107 | 10 | 5 | 0 | чисто | P3 | done |
| voxy_cuda | 3 383 | 96 | 11 | 2 | 0 | 28 `unsafe`, Windows-only части непроверяемы на macOS | P3 | отложено: нет Windows |
| voxy_streamline | 3 168 | 135 | 2 | 0 | 0 | 127 `unsafe`, Windows-only DX12 FFI | P3 | отложено: нет Windows |
| voxy_water | 2 372 | 99 | 6 | 0 | 0 | чисто | P3 | done |
| voxy_core | 2 264 | 172 | 1 | 0 | 0 | `ByteReader`/`ByteWriter`, табличный CRC32C, `CheckpointEnvelope`, 8-байтовый `FastHasher` | P3 | done |
| voxy_world | 2 163 | 85 | 2 | 1 | 0 | `palette_indices_into`, пословная упаковка и `from_canonical_palette_slots` | P2 | done |
| voxy_xr | 2 052 | 84 | 3 | 0 | 0 | все `unsafe` снабжены `// SAFETY:` | P3 | done |
| voxy_storage | 2 043 | 58 | 5 | 1 | 0 | табличный CRC32, `u64` bit-codec + fast-path `Uniform` + zero-dense распаковка | P2 | done |
| voxy_audio | 1 449 | 54 | 4 | 1 | 0 | чисто | P3 | done |
| voxy_audio_device | 1 369 | 77 | 1 | 0 | 0 | чисто | P3 | done |
| voxy_mesher | 1 445 | 43 | 4 | 0 | 0 | стековые маски граней, модульная структура | P3 | done |
| voxy_vulkan | 1 037 | 37 | 3 | 1 | 0 | 68 `unsafe`, Windows-only D3D12 interop | P3 | отложено: нет Windows |
| voxy_rush | 963 | 40 | 1 | 1 | 0 | чисто | P3 | done |
| voxy_runtime | 1 567 | 59 | 6 | 2 | 0 | переиспользование `LightingScratch` при пересборке чанков | P3 | done |
| voxy_depgraph | 816 | 38 | 0 | 0 | 0 | крейт-страж (`depgraph`, `sources`, `physics_modules`, `refactor_metrics`): 12 ratchet-тестов в `tests/` и 12 модульных в `src/source.rs` | P1 | done |
| voxy_lighting | 953 | 33 | 2 | 1 | 0 | `LightingScratch` + `build_light_with_scratch` + табличный BFS | P2 | done |
| voxy_text | 737 | 29 | 0 | 0 | 0 | чисто | P3 | done |
| voxy_ui | 733 | 40 | 1 | 0 | 0 | чисто | P3 | done |
| voxy_input | 559 | 23 | 1 | 0 | 0 | целочисленные `ActionId` для безаллокационного опроса на тике | P2 | done |
| voxy_navigation | 595 | 22 | 0 | 0 | 0 | чисто | P3 | done |
| voxy_time | 364 | 24 | 0 | 0 | 0 | $O(1)$ окно задержек `FrameLatencyWindow` | P3 | done |
| voxy_ui_winit | 341 | 17 | 1 | 0 | 0 | чисто | P3 | done |
| voxy_mobile | 136 | 6 | 0 | 0 | 0 | чисто | P3 | done |
| physics_bio | 26 381 | 770 | 83 | 33 | 6 | вынесен из `physics` в `4f18c0c3`: biomechanics, skin, tissue, lymph, osmotic, circulation, respiration | P2 | done |
| voxy_scene | 8 481 | 363 | 15 | 4 | 1 |  | P2 | done |
| physics_astro | 7 920 | 195 | 23 | 9 | 2 | вынесен из `physics` в `4f18c0c3`: astrophysics | P2 | done |
| voxy_player | 261 | 10 | 1 | 0 | 0 | новый (2026-11-12, не закоммичен): `GameHost`, `InMemoryPackagedProject`; только библиотека | P2 | done |
| voxy_scene_host | 345 | 19 | 0 | 0 | 0 | новый (2026-11-12): трейт `SceneDemo` между оболочкой и демо | P2 | done |
| **Итого** | **383 998** | **11 793** | **1155** | **469** | **117** | **0 сирот, 0 файлов > 1500 прод-строк, 0 нарушений DAG** | | |

## 2. Workspace (снимок до рефакторинга)

Исправлено волной 3: профили и `[workspace.dependencies]` есть, CI есть (`.github/workflows/ci.yml`), fmt чистый,
сирот нет, `voxy_demos -> voxy_editor` снято, `_skipped.*` удалены. `voxy_shell -> voxy_demos` снято шагом 2.4
(`voxy_scene_host`). Не исправлено: нет `clippy.toml` / `deny.toml`.

- `Cargo.toml`: нет `[profile.*]`; нет `[workspace.dependencies]` (glam x16, thiserror x14, wgpu x13,
  pollster x12, serde_json x12, winit x9, serde x9, bytemuck x7); 41 clippy-линт в `allow` (сейчас 32).
- Нет CI (`.github` пуст), нет `clippy.toml` / `rustfmt.toml` / `deny.toml`.
- `cargo fmt --all --check` на срезе красный: 376 расхождений в 116 файлах (последние коммиты не форматировались).
- Осиротевшие файлы (не входят ни в одно дерево `mod`, компилятор их не видит):
  `crates/physics/src/hair/contact/{closest,grid,mesh_contacts,strand_contacts,strand_responses}.rs` (1378 строк кода),
  `crates/physics/src/hair/contact/{query_tests,surface_tests}.rs` (1395 строк тестов),
  `crates/voxy_render/src/surface_{light,color}_transfer_tests.rs` (421 строка тестов).
- Сомнительные рёбра графа: `voxy_demos -> voxy_editor` (только foot placement), `voxy_shell -> voxy_demos`
  (19 демо-модулей по имени + `include_str!` в чужой крейт, `scene_app.rs:825`), `voxy_gpu -> voxy_animation` (1 функция).
- Корневые `_skipped.face` / `_skipped.node` (TetGen, 227 KB) отслеживаются git, ссылок на них нет.

## 3. physics (снимок до рефакторинга и до разделения на крейты)

### 3.1 Карта модулей (прод. строк)
liquid 24 755 · hair 23 215 · biomechanics 18 914 · plasticity 10 143 · astrophysics 8 657 · surface_film 5 615 ·
skin 2 876 · wear 1 955 · moisture 1 939 · lymph 1 832 · rigid_motion 1 770 · contact 1 541 · debris 1 405 ·
gravity 1 168 · tissue 887 · medium 744 · suspension 736 · circulation 735 · tissue_surface 603 · остальные < 500.

Плоские файлы в `src/`, которым место в каталоге домена: `spin_path`, `mass_properties`, `friction`, `cohesive`,
`planar`, `rotation` (тела и контакт); `respiration`, `circulation`, `lymph` (физиология); `symmetric_eigen`,
`spatial_bounds`, `triangle_index`, `static_triangle_world`, `positive_transport`, `heat_exchange` (математика/геометрия);
`soft_body`, `secondary_motion`, `strand`, `tissue_surface`; `medium`, `suspension`.

### 3.2 Слои
Двусторонние рёбра (число ссылок `crate::X` в не-тестовом коде): biomechanics<->lymph 72/10 ·
moisture<->plasticity 2/62 (все 62 в `plasticity/mesh/quadratic/wet_update.rs`) · moisture<->wear 2/10 ·
liquid<->surface_film 9/5 · liquid<->suspension 7/1 · biomechanics<->circulation 2/1 · rigid_motion<->spin_path 2/2 ·
astrophysics -> rigid_motion -> rotation -> astrophysics.

Сильно связные компоненты (по проектному проходу): `{astrophysics, contact, rigid_motion, rotation, spin_path}`,
`{biomechanics, circulation, friction, liquid, lymph, surface_film, suspension}`, `{moisture, plasticity, wear}`.
`hair` (импортирует только `crate::math`) и `astrophysics` (после выноса spin) это острова.

Точные причины рёбер: `friction.rs:3`, `cohesive.rs:3` берут `biomechanics::Matrix` (`biomechanics/mod.rs:94`);
`circulation.rs:475` берёт `biomechanics::solve_protein` (17-строчная обёртка над `positive_transport::solve_mass`,
`biomechanics/mixed_darcy.rs:994`); `lymph` берёт только `OsmoticPressureLaw` (`biomechanics/vascular_pore.rs:5-38`);
`rotation.rs:6-9` реэкспортирует `astrophysics/spin.rs` (952 строки); `astrophysics/spin.rs:99,156,795` берёт
`rigid_motion::MaterialForceMoment` (`rigid_motion/loads.rs:177`); `moisture/material.rs:15-27` знает про plasticity и wear;
`biomechanics/inertia/film_binding.rs:4` тянет surface_film.

`lib.rs:25-48`: 21 плоский псевдоним (`astrophysics_*`, `gravity_*`), ~200 использований в `src`, ~230 в тестах и
примерах physics, 32 в других крейтах (gameplay 24, demos 5, physics_voxel 3).

### 3.3 Дублирование
- Копии `crate::math`: `static_triangle_world.rs:5-26` (`add/sub/dot/cross` идентичны), `skin/contact.rs:177-194`
  (`vsub/vdot/vcross`), `mul == math::scale` в `static_triangle_world.rs:11`, `hair/math.rs:8`, `skin/shell.rs:12`,
  `tissue/mod.rs:13`, `surface_film/contact.rs:7`; нормы через `sqrt` (не `hypot`): `surface_film/mod.rs:155`,
  `strand.rs:64`, `hair/math.rs:11`.
- `fn finite([f64;3]) -> bool` x8: `contact.rs:33`, `liquid.rs:884`, `mass_properties.rs:24`, `strand.rs:67`,
  `tissue/mod.rs:16`, `skin/shell.rs:15`, `gravity/character.rs:19`, `hair/math.rs:95`.
- 3x3: matmul x4 (`biomechanics/mod.rs:108`, `debris.rs:1323`, `liquid/tensor.rs:222`, `liquid/viscoelastic.rs:17`),
  transpose x4, determinant x4, inverse x2 (`biomechanics/mod.rs:138`, `debris.rs:1332`, разные пороги сингулярности).
- Плотный Cholesky x5: `respiration.rs:363` == `circulation.rs:591` (с точностью до имён), `moisture.rs:195`,
  `plasticity/mesh/quadratic.rs:673`, `biomechanics/mixed_darcy.rs:76,106`.
- Гаусс с выбором главного элемента x5: `astrophysics/equilibrium.rs:107`, `plasticity/mesh.rs:391`,
  `liquid/rheology.rs:261`, `astrophysics/binary.rs:346`, `liquid/droplet_gas_viscosity.rs:126`.
- Тесты: 0 файлов используют `physics::math`; `cross`/`near`/`close` скопированы по 8 раз.

### 3.4 Длинные функции и API
| Строк | Функция | Где | Параметров |
|---:|---|---|---:|
| 1069 | `advance_implicit_surface_quadrature` | `biomechanics/inertia/implicit.rs:312` | 6 |
| 886 | `solve_contact` | `liquid/multi_body.rs:826` | 14 |
| 885 | `reconcile_positions_attempt` | `hair/reconcile.rs:312` | |
| 602 | `step_impl` | `hair/step.rs:41` | |
| 583 | `impact_events_impl` | `liquid/film_impact_events.rs:363` | 12 |
| 318 | `implicit_vascular_pore_ports_step_with_osmotic_law_and_observer` | `biomechanics/vascular_pore.rs:319` | 14 |
| 308 | `step_implicit_supports_controlled` | `biomechanics/inertia/implicit.rs:1394` | |
| 288 | `prepare_affine_motion_impl` | `rigid_motion.rs:317` | |
| 286 | `advance_supports_with_contact_motion` | `biomechanics/inertia/supports.rs:193` | |
| 285 | `advance_mode` | `hair/contact_safe_motion.rs:1218` | |
| 281 | `gather_mesh_contacts` | `hair/contact.rs:1267` | |
| 277 | `solve_normal_network_constraints_with_bounds` | `contact.rs:310` | |

68 функций с > 7 параметрами (biomechanics 21, hair 13, plasticity 13, liquid 8), максимум 15
(`biomechanics/cell_poroelastic.rs:1026`, `plasticity/mesh/quadratic/wet_update.rs:580,630`).
255 `pub fn step*/advance*/implicit*`, 152 `pub fn *_with_*`: комбинации опций закодированы в именах.
1542 `pub fn`, 646 `pub struct`; 401 из 1816 pub-имён не встречается вне `physics/src`.
Ошибки: 1277 `Result<_, &'static str>`, 25 `pub enum *Error` (17 в astrophysics) с повторяющимися вариантами.
`unwrap` 84, `expect` 16, `unreachable` 10 в библиотечном коде (hair 32/9/7, biomechanics 26/1, liquid 14/1).
`#[allow]` 107, из них 94 дублируют workspace-allow.

### 3.5 Производительность (не-тестовый код)
479 аллокаций внутри циклов (270 в step/solve-функциях): `.collect()` 146, `.clone()` 125, `vec![` 95, `Vec::new` 54;
по модулям: hair 171, biomechanics 81, plasticity 53, astrophysics 38, surface_film 35. 206 `self.clone()` на пробный шаг
(liquid 65, biomechanics 40, plasticity 25). 125 `Vec<Vec<` (83 в hair).

| Место | Что |
|---|---|
| `hair/mod.rs:444` | `HairBackend::solve -> Vec<Vec<f64>>`, `solve_responses -> Vec<Vec<Vec<f64>>>`: аллокация в самом интерфейсе |
| `hair/reconcile.rs:345-360` | 28 аллокаций внутри `while work.next_attempt()` |
| `hair/step.rs:192` | `strand_responses = Vec::new()` на каждый подшаг |
| `hair/contact_response_system.rs:1537` | `vec![Vec::new(); n]` на сустав в batch-решении |
| `hair/velocity_contacts.rs:268` | `rod.clone()` + 3 `collect` |
| `hair/contact_square_root_projection.rs:538` | `vec![None; n]` + 4 `collect` на остров |
| `surface_film/squeeze.rs:75,352`, `squeeze_body.rs:106` | векторы и валидация на каждый вызов |
| `liquid/multi_body.rs:865-889,926,1022` | `states`, `zero_rates`, `forces.to_vec()`, клоны снапшотов на каждый интервал |
| `biomechanics/inertia/implicit.rs:564,1590` | 10 и 4 `collect` внутри Newton-итерации |
| `biomechanics/inertia/viscous.rs:378,582`, `vascular_pore.rs:461`, `lymph.rs:591,720,1376` | полный клон состояния на пробный шаг |
| `astrophysics/equilibrium.rs:357` | якобиан `vec![vec![0.0; 2n]; 2n]` каждую итерацию + Гаусс на `Vec<Vec>` |
| `respiration.rs:288`, `circulation.rs:535` | `collect` + новый `Vec` из решателя на каждую Newton-итерацию |

Образцы правильного приёма уже в коде: `NativeLoadCache` (`hair/contact_response_system.rs:1949`), `FractionCache`
(`hair/contact_fraction_cache.rs:40`), `SweptPairCache` (`hair/contact_continuous.rs:126`), `Strand::clone_from`.

Бенчи есть только для surface_film (`benches/sphere_contact.rs`) и strand (`examples/strand_step_bench.rs`).
Без бенча: hair step/reconcile/contact, `multi_body::solve_contact`, biomechanics implicit, plasticity dynamics,
lymph, `contact::solve_normal_network_constraints`, `rigid_motion::prepare_affine_motion`, astrophysics, debris, squeeze.

### 3.6 Тесты
224 integration-бинаря (`tests/*.rs`, 74 160 строк, 1594 `#[test]`, 4 `#[ignore]`), 10 используют `support/`.
`tests/surface_film.rs:421,856,899` пишут `docs/film-*.csv` в отслеживаемое дерево.

## 4. render / GPU / voxel

### 4.1 God-типы
| Тип | Где | Полей | impl, строк |
|---|---|---:|---:|
| `ArchiveSceneRenderer` | `voxy_render/src/archive_scene.rs:322` | 85 | 2814 |
| `SceneRenderer` | `scene.rs:722` | | 2332 |
| `Renderer` | `renderer.rs:204` | 60 | 1628 |
| `WorldRayQueryJob` | `world_nearest_job.rs:171` | 40 | 1501 |
| `SceneSurface` | `surface.rs:84` | | 1416 |
| `ScreenSpaceFluidRenderer` | `fluid_screen.rs:108` | 33 | 1151 |
| `ModelAsset` | `voxy_model/src/lib.rs:92` | | 1077 |
| `OceanRenderer` | `ocean/mod.rs:243` | 27 | 956 |

Длинные функции: `new_with_environment` 1080 (`archive_scene.rs:619`), `with_options` 822 (`world_nearest_job.rs:240`),
`draw` 580 (`archive_scene.rs:2803`), `NativeVegetation::new` 464, `OceanRenderer::new` 450, `new_with_shared_budget` 450
(`fluid_screen.rs:384`), `ArchiveRayStages::new` 427, `parse_internal` 375 (`voxy_model/src/lib.rs:548`),
`propagate` 314 (`voxy_gpu/src/lighting.rs:171`). Почти все это GPU-конструкторы, собирающие layout, pipeline и target inline.

Естественные швы:
- `archive_scene.rs`: Layouts (`:728-760`), Pipelines (12 полей), Targets (пересоздаются на resize), Post, Gi, Environment
  (чистые функции `:3734-3807`); `draw`: cull+upload `:2809-2865`, camera `:2866-2930`, shadow, depth, opaque/transparent
  `:3209-3290`, post, temporal `:3313-3383`; 66 `std::env::var` и `archive_shader_source` (`:3663`) в конфиг.
- `scene.rs`: environment / pipelines / ~20 `upload_*` / deformation / 21 `encode_*` (`:2156-2989`, отличаются MSAA,
  viewport, overlay, x-ray, числом видов) -> один `EncodeOptions`; `SceneTransform` (`:440-622`) пишет по сырым смещениям.
- `renderer.rs`: residency (`:653-777`, `:1983-2085`), camera, skinned motion/LOD (`:843-1131`), frame (`:1132`),
  bodycam (`:1290-1712`), capture (`:1523-1665`).
- `skinned.rs`: CPU-математика и admission (`:866-1330`, без wgpu) вперемешку с GPU upload (`:449-700`).
- `voxy_model/src/lib.rs`: validate / texture / animation (`:394-547`) / parse (`:548-930`) / pose (`:930-1007`).

### 4.2 Архитектура кадра
Нет абстракции прохода. Независимые точки входа: `Renderer::render` (`renderer.rs:1132`),
`ArchiveSceneRenderer::draw`, 21 `SceneRenderer::encode*`, `SceneSurface::encode_presentation`, `OceanRenderer::encode`.
Три цепочки пост-обработки: archive (`archive_post.wgsl` + `archive_temporal` + `bloom.rs`), bodycam (`bodycam_post.wgsl`),
ocean (`ocean/bloom_{down,up}.wgsl`, `ocean/composite.wgsl`); AgX в трёх шейдерах.
Archive-шейдер (2400 строк, 145 KB) компилируется трижды: `archive_scene.rs:773`, `world_ray_producer.rs:110`,
`archive_world_ray_resolve.rs:413`. 89 pipeline с `layout: None`, 85 `get_bind_group_layout`.
Shotgun surgery: новый пост-эффект в archive = 9 мест; новое поле `Camera` = Rust-структура (`archive_scene.rs:262`),
заполнение (`:2884`), три WGSL-копии (`archive_scene.wgsl:15`, `archive_water.wgsl:8`, `archive_post.wgsl:4`).

### 4.3 DRY GPU и шейдеры
`voxy_gpu_core/src/builders.rs` (181 строка) используется в 3 файлах. Сырые дескрипторы (render / ray_probe / gpu):
BGL 40/2/8, bind group 120/25/10, render pipeline 36/1/1, compute pipeline 72/22/11, shader module 100/21/8,
`create_buffer_init` 95/17/17. Локальные дубли билдера: `archive_water.rs:56`, `native_vegetation.rs:126,138,154`,
замыкания в `fluid_screen.rs:484,494,501`, `archive_ray_stages.rs:162,186`, `world_ray_producer.rs:77`, `bloom.rs:50`,
`voxy_gpu/src/medium_solver.rs:65`, `voxy_gpu/src/pose_eval.rs:51`.

WGSL без include: конкатенация `include_str!` + ~27 `.replace()` (например `archive_scene.rs:3669-3673`).
Копии: double-double арифметика (`rounded_mul`, `add`, `divide`, `finite_pair`, ...) в 13 файлах
(`liquid_{density,pair_forces,normals,accumulation}.wgsl`, 8 `joint_contact_*.wgsl`, `hair_banded_compensated.wgsl`);
`struct Camera` 20 определений / 10 раскладок; 26 fullscreen-вершинных шейдеров; AgX x3; константы Rec.709 в 9 файлах.
48 `Pod`-uniform (render 40, gpu 8), проверок размера 3 (`archive_scene.rs:3593`), проверок раскладки через naga 0.

Interop: сопоставление LUID адаптера реализовано трижды (`voxy_cuda/src/d3d12.rs:11`, `voxy_vulkan/src/d3d12.rs:13,271`,
`voxy_vulkan/src/lib.rs:60`); `as_hal::<Dx12>` в `voxy_vulkan/src/d3d12.rs:109,156,160` и `voxy_streamline`.

### 4.4 Производительность
Кадр GPU-bound: street 13.95 мс, без GI 6.49 мс (`docs/architecture-performance/2026-10-26-street-120`);
цель p99 8.33 мс не достигнута, ни один кандидат не принят.

| Место | Что |
|---|---|
| `archive_temporal.rs:322,422,430,456` | `create_bind_group` каждый кадр + `collect` |
| `bloom.rs:180-189` | `create_view` на mip + bind group на проход каждый кадр |
| `shadow_map.rs:161-166` | uniform buffer и bind group на каждый снимок теней |
| `native_vegetation.rs:849`, `blit.rs:500,522` | bind group в `encode` |
| `scene.rs:2166`, `fluid_screen.rs:1140`, `surface.rs:1110` | один и тот же filter-collect в `Vec<SceneDraw>`, скопирован трижды |
| `archive_scene.rs:3209-3230,2843` | `collect` + сортировка прозрачных каждый кадр |
| `archive_scene.rs:2884` | camera uniform переписывается безусловно, включая `inverse()` |
| `scene.rs:462-622` | 13 мелких `write_buffer` без dirty-флага |
| 84 сайта `PollType::Wait` | блокирующие readback в библиотечных GPU-задачах (частота вызова не проверена) |

Voxel: `PalettedBlocks::palette_indices()` (`voxy_world/src/chunk.rs:132`) аллоцирует `Vec<u16>` на 32 768 элементов на
каждый вызов, даже для `Uniform`; потребители: `voxy_mesher/src/greedy.rs:72`, `voxy_lighting/src/padded.rs:47`,
`voxy_storage/src/codec.rs:45`. `voxy_lighting/src/propagate.rs:84,94,207`: новые `Vec` и `VecDeque` на сборку.
`voxy_storage/src/codec.rs`: плотный `Vec` + поиндексная распаковка с проверкой границ (`:201,301-316`), побайтовый CRC32
(`:319`), `BTreeSet`/`BTreeMap` клонированных `ResourceKey` (`:47-73`).

Бенч в области один: `voxy_runtime/benches/voxel_pipeline.rs` (mesh, light, palette, commit; storage не покрыт).

### 4.5 Поверхность API и безопасность
`voxy_render/src/lib.rs`: 553 строки, ~512 реэкспортированных имён, 6 `pub mod`; 19 `Experimental*` GPU-решателей контактов
в API рендера; `#[doc(hidden)]` 57 раз; `test_support` (`lib.rs:463`). 43 `Result<_, &'static str | String>` в render.
`unsafe` без `// SAFETY:` (82): `voxy_vulkan/src/export.rs` 28, `voxy_vulkan/src/d3d12.rs` 13, `voxy_streamline/src/dx12.rs` 10,
`voxy_streamline/src/lib.rs` 9, `scene_dx12.rs` 8, `voxy_xr/src/session.rs` 4, остальные по 1-2.
`#[allow]`: `too_many_arguments` 47, `cast_possible_truncation` 23, `too_many_lines` 22 дублируют workspace-allow.

## 5. app-слой

### 5.1 God-структуры
| Полей | Структура | Где |
|---:|---|---|
| 80 | `VoxyApp` | `voxy_app/src/main.rs:47` |
| 75 | `App` | `voxy_editor/src/lib.rs:457` (52 `impl App` в 36 файлах) |
| 73 | `FemaleDemo` | `voxy_demos/src/female_demo.rs:40` |
| 67 | `GpuHairLinearSolver` | `voxy_demos/src/gpu_hair_solver.rs:58` |
| 64 | `App` | `voxy_app/examples/street_walk/main.rs:52` |
| 46 + 34 | `SceneApp` + `Resources` | `voxy_shell/src/scene_app.rs:119`, `:76` |
| 44 | `BodyParameters` | `voxy_demos/src/body_parameters.rs:104` |
| 33 / 28 | `HdrScene` / `WebEngine` | `voxy_web/src/animated_scene.rs:31`, `browser.rs:45` |

Editor `App`, использование полей по кластерам (166 функций):
| Кластер | Использований | Функций | Только этот кластер |
|---|---:|---:|---:|
| ViewportCameras | 97 | 22 | 2 |
| ModelRelink | 38 | 16 | 2 |
| SmokeHarness | 49 | 12 | 0 |
| AssetResidency | 150 | 41 | 1 |
| Persistence | 95 | 38 | 8 |
| InspectorState | 177 | 45 | 6 |
| WindowHost | 239 | 59 | 6 |

Чистого разреза нет: `publish_editor_checkpoint_install` (`app/restore_barrier/editor.rs:434`) трогает 45 полей в 8 кластерах.
Модальные поля не взаимоисключающие (`app/restore_barrier/save.rs:50-63`, `component_fields.rs:1099`).

Что держат `lib.rs`: `voxy_animation` (2266 строк): `Skeleton` `:135-276`, `AnimationClip` impl `:345-1071`, `Pose`
`:1072-1262`, `Animator` `:1283-1832`, 7 несвязанных функций `:2092-2214`; `voxy_gameplay` (1972): `CharacterPhysics`
impl `:551-1511`; `voxy_scene` (1304): весь `SceneGraph` `:208-898`; `voxy_editor` (1986): `App`, `ApplicationHandler`
`:1073-1424`, экспортеры `:1486-1687`, 7 `run_model_viewport*`.

### 5.2 Длинные функции
`Panels::build` 1314 (`voxy_editor/src/panels.rs:226`, 12 параметров) · `draw_frame_inner` 839 (`app/viewport.rs:28`) ·
`SceneApp::draw` 789 (`voxy_shell/src/scene_app.rs:1200`) · `initialize` 546 (`:653`) · `female_pelvic::new` 544 ·
`FemaleDemo::advance_impl` 479 (`female_demo.rs:1545`) · `poll_editor_file_load` 431 (`app/restore_barrier/load.rs:195`) ·
`fixed_step_with_paths` 421 (`voxy_gameplay/src/lib.rs:1090`) · `VoxyApp::window_event` 417 (`main.rs:387`) ·
`advance_animation` 405 (`main.rs:983`) · `SceneApp::window_event` 360 (`:2275`) · `import::load` 354 ·
`sweep_rigid_path_with_bounds` 330 (`angular_sweep.rs:1126`, 14 параметров) · `edit_key` 320 (`app/inspector.rs:161`) ·
`correct_at_phase_inner` 310 (`foot_placement.rs:644`, 13 параметров) · `animated_models::update` 250 (`:549`, 15 параметров).
Примеры: `villa.rs:227 build` 1526, `original.rs:1100 geometry` 1354 (геометрия как код), `female_render.rs:15 main` 1104.
71 функция с > 7 параметрами: gameplay 29, editor 20, animation 12, demos 7.

### 5.3 Дублирование
- Orbit/fly-камеры x5: `voxy_editor/src/camera.rs:24`, `voxy_app/src/controls.rs:177,313`,
  `voxy_shell/src/camera_motion.rs:5`, `voxy_gameplay/src/follow_camera.rs:9`, поля в `female_demo.rs:83`;
  формула сферического глаза дословно в `labia_tube_insertion.rs:1229` и `anal_tube_insertion.rs:1025`.
- `KeyCode -> Control`: `voxy_editor/src/lib.rs:173`, `voxy_shell/src/scene_input.rs:13`; focus-навигация в
  `voxy_editor/src/game_ui_input.rs:76-93` и `voxy_ui_winit/src/lib.rs:179-187`.
- JSON: `JsonDocument` (`voxy_assets/src/json_document.rs:65`) без реализаций; 77 рукописных `to_json/from_json` в 38 файлах
  (editor 12, animation 9, gameplay 7, scene 5, demos 4); 20 файлов повторяют проверку `text.len() > max_bytes`.
- Checkpoint: 49 файлов, 23 577 строк (editor 23 / 13 506, gameplay 10 / 3 813, animation 10 / 3 652, scene 6 / 2 606).
- `labia_tube_insertion.rs` и `anal_tube_insertion.rs`: 725 общих строк; `cam.rs`/`camcorder.rs` 187, `cam.rs`/`bodycam.rs` 177.
- 12 `impl ApplicationHandler` (3 продакшн, 9 в примерах), 34 файла создают event loop.
- `examples/street_walk/main.rs:4,7`: `#[path = "../../src/controls.rs"]` и `frame_profile.rs` (нарушение ADR 0004, правило 5).

### 5.4 Слои
- Foot placement IK в `voxy_editor` (`foot_placement.rs`, `foot_placement/contact_events.rs`, `foot_runtime_checkpoint.rs`,
  1561 прод. строк), единственная причина ребра `voxy_demos -> voxy_editor`.
- Чистая геометрия в `voxy_gameplay` без `SceneGraph`: `angular_sweep.rs` + `angular_sweep/`, `convex.rs` (1173),
  `liquid_runtime/rigid_support.rs`. Перенос в physics не делается: physics без glam.
- Решатели в `voxy_demos`: `gpu_hair_solver.rs`, `gpu_hair_joint.rs`, `tissue_demo.rs:1648`.
- ~1900 строк `voxy_editor/src/*_smoke.rs` собираются в продакшн-крейт (`lib.rs:56-95`).
- 15 из 41 `pub mod` в `voxy_demos` без внешних ссылок.

### 5.5 Производительность на кадре и тике
| Место | Что |
|---|---|
| `voxy_demos/src/female_demo.rs:1553-1612` | 4 `std::fs::metadata` на каждый шаг физики (hot reload) |
| `female_demo.rs:1641,1684,1726` | клоны `grounded_playback`, `secondary`, `attachments` на шаг |
| `voxy_editor/src/app/viewport.rs:931-1034` | клон всего `SceneDocument` (`inspector.rs:21`) + глубокое сравнение каждый кадр |
| `viewport.rs:394,397,541,563,595,775,789` | покадровые `Vec`, offscreen и onscreen код продублирован |
| `viewport.rs:1090` | клон `animated_model` на экземпляр на кадр |
| `voxy_shell/src/scene_app.rs:1366-1383` | 2-3 `format!` заголовка окна каждый кадр до проверки изменения |
| `voxy_gameplay/src/lib.rs:1185-1195` | `HashSet` живых тел, клон `states`, пересборка `StaticWorld`, поиск ввода по строке каждый тик |
| `voxy_scene/src/lib.rs:418-445` | `active_components<T>`: обход всех слотов + `HashMap<TypeId>`-проба; 43 вызова в gameplay, 50 в editor |
| `voxy_animation/src/lib.rs:867,1254,1700` | `sample`/`blend`/`advance_wall` возвращают новые `Vec` |
| `viewport.rs:803-830` | `println!` трассировки на пути кадра |

Бенчи: `voxy_scene/benches/{scene_scaling,processor_requirements}.rs`, `voxy_gameplay/benches/angular_motion.rs`.
Без бенча: Animator, retarget, root motion, `CharacterPhysics::fixed_step`, liquid_runtime, editor frame, `FemaleDemo::advance`.

### 5.6 Ошибки и мёртвый код
`Box<dyn Error>`: editor 227, demos 103, shell 33, app 15. `Result<_, String>`: editor 46, app 39, assets 25, rush 18,
audio_device 16. `#[allow(dead_code)]` 171, из них 18 модульных в editor (gpu_model_restore, gpu_source_bindings,
gpu_model_batch, animation_sources, app/{authoring_checkpoint,resource_checkpoint,restore_barrier,restore_guard},
ui_live/{checkpoint,publications,restore,reload_checkpoint}, animated_models/checkpoint, import/*_checkpoint) и 4 в voxy_app.
`voxy_demos::destruction_demo` (309 строк) используется только своими тестами.
