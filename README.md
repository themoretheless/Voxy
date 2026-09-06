# Voxy

Voxy — воксельный Tamagotchi-движок на Rust с нативным desktop-клиентом.

Cargo-workspace повторяет границы архитектуры: `voxy_core` содержит координатные и временные контракты, `voxy_world` — registry блоков, palette chunks, детерминированные проходимые горы и атомарные edit-транзакции, `voxy_mesher` — naive oracle и production greedy meshing, `voxy_render` — `wgpu` surface lifecycle, ортографическую камеру, skeletal GPU skinning, 32-уровневый чёрно-серый LCD shader и alpha blending, `physics` — независимую кинематическую физику, `physics_voxel` — её воксельный адаптер, воду и разрушения, а `voxy_runtime` — fixed-step simulation и renderer-neutral bootstrap сцены.

`voxy_app` реализует игровой Tamagotchi-цикл: объёмный питомец ходит и прыгает по миру, стареет, испытывает голод, теряет энергию, настроение, здоровье и чистоту, спит и реагирует на уход. Окно движка остаётся скрытым до готовности мира и GPU-ресурсов; после запуска в заголовке показываются состояние питомца и измеренное время загрузки.

## Управление

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
