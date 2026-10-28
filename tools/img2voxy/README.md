# img2voxy

Нативный Rust-компилятор процедурных реконструкций для Voxy. Идея поэтапного
восстановления по референсу вдохновлена [img2threejs](https://github.com/img2threejs/img2threejs).
Код реализации написан для Voxy; исходники upstream не скопированы.

Из корня Voxy:

```sh
cargo run --manifest-path tools/img2voxy/Cargo.toml -- validate tools/img2voxy/examples/desk-lamp.json
cargo run --manifest-path tools/img2voxy/Cargo.toml -- build tools/img2voxy/examples/desk-lamp.json /tmp/voxy-desk-lamp
cargo run -p voxy_editor -- --validate-project /tmp/voxy-desk-lamp/project.voxy
cargo run -p voxy_editor -- --project /tmp/voxy-desk-lamp/project.voxy
```

Каталог вывода должен быть новым; повторный запуск не перезаписывает результат.
При ошибке записи неполный каталог сохраняется для диагностики.

## Работа с референсом

1. Агент рассматривает изображение и составляет список видимых деталей,
   пропорций, цветов и предполагаемых скрытых областей. Компилятор сам
   изображение не анализирует и не вызывает AI-сервис.
2. Агент пишет JSON по образцу `examples/desk-lamp.json`. Поле `reference`
   сохраняет путь/URL референса; `inferred_regions` перечисляет предположения.
   Каждая запись `details` связывает наблюдение с видимой частью.
3. Сначала строится силуэт, затем структура, форма, цвет и мелкие детали.
   После каждой итерации создаётся новый каталог проекта.
4. Проект открывается в Voxy, камера подбирается под референс. Агент сравнивает
   полученный рендер с изображением и исправляет спецификацию. Сходство нельзя
   объявлять проверенным по успешной компиляции.

Выход: обычный `project.voxy`, asset manifest, `SceneDocument`, процедурные OBJ,
исходная спецификация и `review.json`. OBJ — генерируемые ресурсы для текущего
импортера Voxy, никаких скачанных моделей и JavaScript. `review.json` всегда
помечает визуальную проверку как `pending`: автоматического визуального gate
в этой версии нет.

## Контракт первой версии

Метры, Y вверх. Геометрия центрирована: box имеет размер 1×1×1, sphere диаметр 1,
cylinder высоту 1 и диаметр 1. `scale` задаёт размер, `rotation` — единичный
кватернион XYZW; `translation` относительно родителя. `geometry: null`
создаёт пустой узел для pivot/socket. Наличие pivot не означает, что анимация
уже реализована или проверена.

Поддержаны box, sphere, cylinder, rounded_box, rounded_box_section,
локальные трансформации, RGB tint, иерархия и необязательная камера `camera`.
Секции скруглённого корпуса сохраняют общий внешний контур и закрывают срезы.
Движок проверяет сцены на циклы и потерянных родителей. Компилятор ограничивает
число частей, subdivision, диапазоны трансформаций и ссылки из списка деталей.
Демонстрационная лампа придумана, не восстановлена с фотографии.

Это первый рабочий срез, а не полный перенос upstream: нет автоматического
camera matching, PBR fitting, проекции фото, произвольных поверхностей,
rigging, экспортируемых animation clips, сравнения пикселей или блокировки
этапов по подписанным render-review evidence. Цвет использует существующий
`SceneMaterial` Voxy; физические roughness/metalness не имитируются.

Проверки:

```sh
cargo test --manifest-path tools/img2voxy/Cargo.toml
cargo clippy --manifest-path tools/img2voxy/Cargo.toml --all-targets -- -D warnings
```

Проверено 2026-10-27: 3 unit-теста и Clippy с `-D warnings` прошли.
Сгенерированный `examples/img2voxy-desk-lamp/project.voxy` прошёл
`--validate-project` (6 узлов) и `--game-check` (120 тактов, 4 опубликованных
меша). Это проверка интеграции и загрузки; визуальный рендер и сходство
с фотографией не квалифицированы.

## AirPods Pro 3

Спецификация: `tools/img2voxy/examples/airpods-pro-3.json`.
Результат после визуальных исправлений:
`examples/img2voxy-airpods-pro-3-final/project.voxy`.

Референс и размеры: https://www.apple.com/airpods-pro/specs/ .
Локальный снимок референса в `references/airpods-pro-3-apple.jpg` принадлежит
Apple и используется для сравнения; он не включён в модель или её материалы.
В этой композиции кейс закрыт, наушники показаны отдельно.

```sh
cargo run -p voxy_editor -- --project examples/img2voxy-airpods-pro-3-final/project.voxy
cargo run -p voxy_editor --example procedural_preview -- examples/img2voxy-airpods-pro-3-final/project.voxy /tmp/airpods.ppm
```

`procedural_preview` загружает локальные OBJ проекта и рендерит их через
`SceneRenderer` и тот же `material.wgsl`, что использует редактор. Это отдельный
offscreen-путь для процедурных OBJ-проектов, не снимок окна и не универсальный
загрузчик моделей. Камера, иерархия и tint берутся из сцены.

На Apple M4 Max / Metal получен `preview.png`, проверенный визуально.
`game-check.log`: 120 тактов, 32 узла, PASS. Четыре теста компилятора,
Clippy компилятора и Clippy примера с `-D warnings` прошли.
Сходство квалифицировано только как приближённая процедурная заготовка:
поверхности наушников упрощены, шов гранёный, нет глянцевого PBR,
точного сенсорного узла и полостей. Анимация не проверена.
Предыдущие варианты `img2voxy-airpods-pro-3` и `img2voxy-airpods-pro-3-v2`
сохранены как отклонённые итерации, с заметными дефектами шва/отверстий.
