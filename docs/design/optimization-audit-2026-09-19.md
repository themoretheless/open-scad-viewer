# Аудит производительности и архитектуры: 2026-09-19

Дата: 2026-09-19. Ветка `claude/optimization-refactoring-a9dfd5` от `fdf7b1a`.
Статус: проверка исходного кода плюс замеры на M-серии Mac (Node 20, own-Rust
ядро в WASM и нативно). Все числа в этом документе получены в этой сессии
командами, перечисленными в разделе 8; одиночный прогон считается
ориентировочным, а не гарантией.

Сопутствующий документ с замерами и правками CSG-ядра:
[csg-scaling-2026-09-19.md](csg-scaling-2026-09-19.md). Реестр долга
обновлён в [recommendation.md](../recommendation.md).

## 0. Главные выводы

1. **Узкое место продукта: масштабирование CSG-ядра, а не рендер и не
   парсер.** Штатные фикстуры `bench:cpu` (`medium-csg`, `dense-sphere`) на
   HEAD не собираются: ядро отвечает «Boolean work/fragment/output budget
   exceeded». Плита с 49 отверстиями при `$fn=32`, union трёх
   непересекающихся сфер при `$fn=64` и union двух пересекающихся сфер при
   `$fn=48` были за пределами возможностей до этой сессии. Причины: предел
   10 000 входных треугольников стоял *раньше* точных быстрых путей
   (разнесённые/вложенные тела), n-арные операции сворачивались
   последовательно, а специализация для «просверленной плиты»
   (`prism_boolean`) требовала одинаковую высоту резака и детали, тогда как
   в OpenSCAD резак всегда длиннее. В этой сессии исправлены три вещи из
   четырёх; квадратичное BSP-построение на кривых телах и слабая 2D-
   триангуляция многоугольника с отверстиями остались (раздел 5).
2. **В TypeScript на горячем пути оставалось то, что должно быть в Rust:**
   нормали с crease-разбиением считались на каждую сборку в
   `services/geometry/module.ts` (adjacency-списки, строковые ключи в `Map`,
   `number[][]` на треугольник). Перенесено в ядро одним ABI-вызовом
   `abi_render_mesh` с побитово тем же результатом (раздел 4).
3. **Код содержит ~5 200 строк мёртвых модулей** и два параллельных
   TS-эвалуатора OpenSCAD с 630 дословно совпадающими строками (раздел 3).
4. **Клиппи не запускается в CI и не чистый** (548 уникальных диагностик),
   а коммит `fdf7b1a` случайно выбросил три крейта из `workspace.members`,
   из-за чего `npm run build:geometry` на HEAD не собирает языковое ядро.
   Членство восстановлено в этой ветке.
5. **Стратегический выбор:** BSP-CSG с бюджетами принципиально не даёт
   устойчивого масштабирования на кривых телах. Либо ядро получает
   arrangement-based булеан (Manifold-подход: точные предикаты уже есть в
   `cad-predicates`), либо принимается зависимость от pure-Rust порта
   Manifold (`manifold-rust` 0.13, Apache-2.0). Это решение уровня ADR, а не
   оптимизация.

## 1. System design: что есть и где трещит

Слои описаны в [architecture.md](../../architecture.md) и в целом
соблюдаются: workspace snapshot → `BuildCoordinator` → `geometry.worker` →
парсер/эвалуатор → WASM-ядро → `MeshData` → App/WebGPU. Границы, которые
реально нарушены:

| Граница | Факт | Где |
| --- | --- | --- |
| Один эвалуатор OpenSCAD | Два независимых эвалуатора над одним AST: mesh-lane (`openscadParser.ts`, 3 974 строки) и B-rep lane (`openscadSemanticLowerer.ts`, 3 467). 43 одноимённые функции, 630 совпадающих содержательных строк; второй не использует общие `openScadStable*Semantics` и сам реализует `makeCylinder`/`makePolyhedron`/матрицы. Класс `StableBuiltinValueError` объявлен дважды. | `src/services/openscadParser.ts:487-520` против `openscadSemanticLowerer.ts:1097-1130`; `:342` против `:217` |
| Rust-фронтенд OpenSCAD | `crates/openscad-core` (7 959 строк без тестов) достижим только из двух тестов через `scadCompileRust`/`scadEvalRust`; ни одна продуктовая поверхность его не вызывает. | `src/services/languages/kernel.ts:80,92` |
| Один владелец состояния сцены | Selection хранится в трёх местах (renderer, `sceneController`, `App.vue:523`), visibility и meshes пишутся в два владельца; `ViewportController` держит вторую `CameraHistory`, чьи методы не вызываются нигде. | `src/App.vue:2073-2094`, `src/services/viewportController.ts:163-200` |
| Валидация публикации один раз | Полная O(V+T) проверка `isGeometryWorkerEvent` выполняется в worker'е и повторно на главном потоке; provenance обходится трижды, BVH дважды. Комментарий «O(mesh count), never O(vertex)» в `geometryWorkerProtocol.ts:528` не соответствует коду. | `src/workers/geometry.worker.ts:220`, `src/services/buildCoordinator.ts:476`, `geometryWorkerProtocol.ts:509-523,567,594` |
| Главный поток не считает геометрию | `inferSurfaceIds` (union-find по всем треугольникам, строковые ключи) выполняется на каждую публикацию и повторяет то, что ядро уже вернуло в `topology`; его `contentCache` никогда не очищается. | `src/services/meshSurfaceGroups.ts:4,9-30`, `src/App.vue:1624` |
| Транспорт больших массивов | `mesh_boolean` (Solid/Mesh workspace) гоняет меши через `Value`-дерево: 32 байта на скаляр в памяти, 9 на проводе, плюс `v[k].clone()` всего поддерева в `field()`. Сырой путь (`abi_import_mesh` + op 9) существует и используется только импортом/экспортом. | `crates/geometry-bridge/src/lib.rs:112-114,668-678`, `src/services/geometry/polygon.ts:48` |

Что сделано хорошо и не должно пострадать при рефакторинге: протокол v6 с
цифрами источника и tombstone-ами job id; identity trio
(`SourceOperationId`/`SceneEntityId`/provenance); handle-only kernel ops с
GC-сессиями; кооперативная отмена между statement'ами; честные ярлыки
(`selfIntersectionStatus`, «approximate sorted alpha»).

## 2. Rust-workspace: горячие пути и структура

Полный отчёт агента лежит в истории сессии; здесь только то, что проверено
чтением и подтверждено замером или воспроизведением.

### 2.1 Сборочная гигиена

- `cargo clippy --workspace --all-targets -- -D warnings` всё ещё не является
  чистым: исходный аудит насчитал 548 уникальных диагностик (brep-core 280,
  geometry-bridge 89, nurbs-core 49). Первый корневой блокер в `osv-math`
  (5 × `needless_range_loop`) устранён без allow-атрибутов, а CI теперь
  запускает строгий Clippy для этого общего математического крейта. Остальные
  крейты следует подключать по мере очистки, не скрывая baseline глобальным
  allow-list.
- `crates/Cargo.toml` на HEAD не содержал `mechanical-core`,
  `languages-bridge`, `languages-wasm` (регрессия коммита `fdf7b1a`);
  `cargo metadata --manifest-path crates/languages-wasm/Cargo.toml` отвечал
  «current package believes it's in a workspace when it's not». Исправлено
  здесь вместе с `Cargo.lock`.
- Ни один rbench-пример не вызывается из npm-скриптов или CI; для булеана,
  `Mesh::inspect`, `simplify` и тесселяции бенчей не было вовсе. Добавлен
  `crates/polygon-core/examples/bench_boolean.rs`.

### 2.2 Горячие пути `polygon-core` (путь браузера)

| Место | Что происходит | Частота |
| --- | --- | --- |
| `Mesh::edges()` `lib.rs:430-440` | `BTreeMap<(usize,usize), Vec<[usize;2]>>`: одна куча на ребро, 3T упорядоченных вставок | на каждый `inspect()`; булеан вызывает `inspect` не меньше 4 раз на полных мешах |
| `primitives::simplify` `:241-372` | `positions.clone()` на каждую группу копланарных граней; `boundary_loops()` (validate + 2 карты рёбер) на группу | после каждого BSP-результата; для плиты с 64 отверстиями G ≈ 2 000 |
| `Bsp::build` `boolean.rs:399-466` | до 4 полных классификаций набора на узел ради выбора плоскости; на кривых телах ни одна плоскость не разделяет без разрезов, глубина ~n → O(n²) | причина «Build A/B: work budget exceeded during BSP construction» уже при 2 × 2 300 треугольниках |
| `merge_coplanar` `:603-689` | стоимость кандидата O(v²), для n-угольной крышки Σk² ≈ n³/3 тиков | 64 × 2 крышки по 32 → ≈1.4M из 8M `max_work` |
| `stitch` `:726-861` | три comparison-сортировки индексов, 6 `partition_point` + линейный скан на ребро | на каждый BSP-результат |
| `planar-geometry::triangulation` | ear clipping O(n³) в худшем случае, бюджет 2 048 вершин, эвристика мостов к отверстиям | крышки extrude/prism-пути и `simplify`; падает на выровненной сетке 4×4 отверстий |
| `Value`-транспорт | `Serialize for [T]` строит `Vec<Value>` на массив; `Number::from_f64(NaN)` молча даёт `Null` | все JSON-ops с мешами |

### 2.3 DRY между крейтами

- `brep-core` не зависит от `math-core` и содержит 71 локальную копию
  векторной математики в 14 файлах (`operations.rs:19-47` через итераторы и
  `from_fn`, без `#[inline]`, под `opt-level = "s"`).
- 24 копии в `geometry-bridge` (`cad_mesh_planes.rs:13-35` и далее), 3 в
  `polygon-core` (`bvh_query.rs:8-20`), 5 копий `dist` в `planar-geometry`.
- Точка-в-многоугольнике реализована 7 раз, знаковая площадь 7 раз,
  `Plane` 3 раза; `cad_predicates::orient3d` не используется ни одной из них.
- Два одинаковых ZIP/CRC32 хелпера: `polygon-core/src/package_3mf.rs:20-40`
  и `gcode-core/src/package_3mf.rs:44-70`.
- Допуски: 15 разных литералов (`1e-11`, `1e-9`, `1e-8`, `2e-6`, `1e-4`…)
  без общего источника на mesh-стороне; B-rep имеет `ToleranceContext`.

### 2.4 `unsafe`

Все `unsafe` в `polygon-core` (BVH/edges), `photogrammetry-ffi`, `*-wasm`
имеют документированный инвариант и `debug_assert!`. Без обоснования только
CUDA-`launch` (11 в `math-core/src/cuda.rs`, по одному в `sdf-core`,
`photogrammetry-core`, `geometry-bridge/lattice_cuda.rs`) и
`bytemuck_u32` в `photogrammetry-core/src/gpu/rectification.rs:244`.
`openscad-core` держит `#![forbid(unsafe_code)]`.

## 3. TypeScript: SOLID, DRY, мёртвый код

### 3.1 Мёртвые модули (ни одного импортёра в `src`, `tests`, `scripts`, `benchmarks`, `tools`)

| Модуль | Строк | Примечание |
| --- | ---: | --- |
| `src/parser/geometry.ts` + `limits.ts` | 2 591 | «Extracted from the openscadParser monolith… later moved to a Web Worker»: переезд не состоялся, примитивы теперь в Rust |
| `src/i18n/index.ts` | 1 623 | ru/en/de/zh; `App.vue` держит свой inline-словарь `L` (ru/en) на строках 132-238 |
| `src/services/threemfExport.ts`, `zipExport.ts`, `stlExport.ts`, `objExport.ts` | 698 | вытеснены Rust-экспортом (`meshExport.ts`, `meshExportFormats.ts`); `vite.config.ts:51` до сих пор объявляет для них чанк `exporters` |
| `src/config/index.ts` | 151 | «Static configuration tables extracted from App.vue» |
| `src/renderer/shaders.ts` | 139 | WGSL инлайнится в `webgpuRenderer.ts:91-265`; модуль сам предупреждает о тройном копипасте структуры |
| `src/services/brepDiagnosticWorkerLane.ts`, `modelGraphTextNurbs.ts` | 29 | |

Удаление безопасно по статическому анализу; перед удалением нужен `vite
build` с анализом бандла (динамических `import()` по этим путям не найдено).
`docs/TOP-50-ISSUES.md` описывает `App.vue` на 15 000 строк и символы,
которых в текущих 3 658 строках нет; документ помечен как устаревший.

### 3.2 Дублирование инструментов между Solid / Mesh / Code

- Undo/redo реализован трижды: `DirectHistory`
  (`directModeling.ts:158-199`), `MeshHistory` (`meshEditing.ts:51-79`,
  сериализует весь стек на каждый commit, то есть тот O(history), который
  `DirectHistory` уже починил), история исходника (`mainSourceEditing.ts`).
- Орбитальная камера: четыре реализации с разной чувствительностью
  (`cameraGestures.ts`, `DirectModeler.vue:1019`, `MeshModeler.vue:607`,
  `directModelingTools.ts:6-17`).
- Picking: BVH в рендерере против перебора всех треугольников на каждый
  `pointermove` в `DirectModeler.vue:544-561` и проекции всей сцены в SVG в
  `MeshModeler.vue:512-540`.
- Сварка вершин по округлённому ключу: шесть копий с четырьмя разными
  допусками; crease-нормали: три реализации (`solidGpuView.ts:291`,
  `DirectModeler.vue:804`, бывшая в `module.ts`).
- `downloadBlob`: девять копий; `clamp01`, `sameTypedArray`/`sameView`,
  FNV-1a: по две.

### 3.3 God-модули и смешанные обязанности

`openscadParser.ts` одновременно mesh-эвалуатор, точка входа stable-профиля
(`parseOpenScadProject`), диспетчер ModelGraph-текста и глобальная очередь
сериализации всей геометрии (`parseQueue`). `webgpuRenderer.ts` (2 759
строк, 133 строки полей класса) держит 12 pipeline'ов, picking, input,
overlays и анимацию, хотя вспомогательные модули (`cameraGestures`,
`selectionCycling`, `nativePickingCache`, `viewFrustum`,
`transparentOrdering`) уже извлечены и импортируются только им. `App.vue`
содержит 20 кластеров ответственности; самые плотные: жизненный цикл
рендерера (947-1231) и `handleGeometryResponse` (1594-1710).

## 4. Что перенесено в Rust в этой сессии

| Было | Стало | Эффект |
| --- | --- | --- |
| Нормали с crease-разбиением в TS (`module.ts` `getMesh()`): adjacency `number[][]`, `Map<string, number>` с ключом `id:r0,r1,r2`, `Math.hypot(...)` на вершину | `geometry-bridge/src/mesh.rs::render_buffers` + `abi_render_mesh(id, cosine)`; JS-`Math.hypot` и `Math.round` воспроизведены побитово (`js_hypot3`, `js_round`) | один вызов ABI и одна копия из линейной памяти вместо снимка позиций f64 → JS-массивы → `Float32Array`; в TS удалено ~50 строк; выход идентичен байт в байт (тест `kernel-built display meshes`) |
| `analyzeSolid` копировал все 10 массивов `Mesh` ещё раз; парсер копировал `vertProperties`, `triVerts`, `faceID` третий раз | массивы, принадлежащие вызову, публикуются напрямую (`isExclusiveView`) | минус две полные копии меша на тело |
| `boolean3('difference', [base, union(cutters)])` | `[base, c1, …, cn]`, где каждый child-statement один операнд; ядро вычитает разнесённые резаки батчами по 8 (`difference_many`) | плита с отверстиями и подобные модели не строят один гигантский резак |
| Последовательное сворачивание n-арного union | `union_many`: union-find по AABB, разнесённые группы `join`-ятся без CSG | union 64 цилиндров: 63 булеана → 0 |

Что оставалось в TS и теперь перенесено в ядро: `inferSurfaceIds`
(`meshSurfaceGroups.ts`) больше не выполняет O(T) обход на главном потоке для
обычных публикаций. `geometry-bridge::mesh::surface_group_ids` строит те же
smooth connected patch ids за границей WASM, `render_buffers`/`export_buffers`
публикуют их сразу, а TS-слой только принимает уже транспортированные `faceIds`
или вызывает raw-buffer fallback для legacy-мешей без ids.

Замер этой правки (`scripts/bench-surface-groups.mts`, Node 22.23.2,
`--expose-gc`, синтетическая connected strip-сетка 64 000 треугольников):

| Путь | Медиана |
| --- | ---: |
| До: `inferSurfaceIds` в TypeScript | 46.529 мс |
| После: warmed raw-buffer Rust fallback для legacy-мешей без ids | 16.236 мс |
| После: обычная публикация с уже транспортированными `faceIds` | 0.000250 мс |

Первые два числа измеряют сам алгоритм группировки; третье — продуктовый путь
`withSelectionSurfaces` для нормальных публикаций после переноса, где O(T)
обход больше не выполняется.

Что осталось в TS и просится в ядро следующим: `measureMeshBounds` в `setMeshes`
(`webgpuRenderer.ts:981-1040`), три отдельные загрузки одного меша в WASM
(BVH, рёбра, экспорт) вместо одного `analyze_solid`, глубокая валидация
протокола на главном потоке.

## 5. Узкие места с числами

Полная таблица «до/после» в [csg-scaling-2026-09-19.md](csg-scaling-2026-09-19.md).
Кратко (Node 20, WASM, полное качество, медианы):

| Сценарий | HEAD | Эта ветка |
| --- | --- | --- |
| Плита минус 36 цилиндров, `$fn=32` | 284 мс | 235 мс |
| Плита минус 49/64 цилиндра | ошибка бюджета | см. CSG-документ |
| Union 3 разнесённых сфер, `$fn=64` / `128` | ошибка (12k > 10k) | 24 мс / 103 мс |
| Union 3 разнесённых сфер, `$fn=192` | ошибка | ошибка: 100k-предел на меш (`MAX_MESH_TRIANGLES`), честно |
| Union 2 пересекающихся сфер, `$fn=48` | ошибка (BSP build) | ошибка, без изменений |

Не исправлено и требует алгоритмической работы:

1. **BSP-построение на кривых телах O(n²)**: две сферы по 2 300
   треугольников исчерпывают 8M work. Ни один бюджет это не лечит.
2. **Триангуляция многоугольника с отверстиями** (`planar-geometry`):
   эвристика мостов падает на выровненной сетке 4×4, бюджет 2 048 вершин
   отсекает 64 отверстия по 32 сегмента. От неё зависят prism-путь,
   `linear_extrude` профилей с отверстиями и `simplify`.
3. **Накопление фрагментов при последовательном BSP**: плоскости отсечения
   бесконечны, каждый шаг заново режет всю крышку; без работающего
   `simplify` рост квадратичный (измерено: 14 700 треугольников после 14
   одиночных вычитаний квадратных столбиков из плиты 8×8).

## 6. Конкуренты: чего не хватает и что уже уникально

Источник: отчёт агента с первичными ссылками (дата просмотра 2026-09-19).

Лучше у конкурентов (по убыванию цены закрытия здесь):

- Monaco/CodeMirror с автодополнением по импортам и inline-диагностикой
  (OpenSCAD Playground, openscad-studio, Zoo): здесь textarea. Средняя цена.
- Manifold по умолчанию с многопоточностью (OpenSCAD nightly с 2025-08-17);
  в WASM у Playground он однопоточный, но робастный. Высокая цена; см. ADR
  «Manifold as peer» и вывод 5 из раздела 0.
- Библиотеки BOSL2/MCAD и multi-file (Playground). Средняя.
- OpenCSG-превью без полного CSG на каждое нажатие (nightly); здесь
  промежуточный ответ: content-addressed subtree cache из архитектурного
  плана. Высокая.
- PWA/offline (Playground, CascadeStudio): низкая цена.
- WebGL-fallback: везде кроме этого проекта. Средняя-высокая.
- Полноценный measure (длина/площадь/радиус/расстояние между сущностями,
  Onshape/ocp_vscode), секции с крышками и несколькими плоскостями.
- Экспорт GLB, анимация `$t` в viewport, цветной 3MF с lazy-union.

Никто в наборе не делает: печать по LAN с превью G-code в той же вкладке,
двунаправленную provenance «исходник ↔ треугольники», crash-safe журнал
персистентности, оракульную конформность к upstream как публичный сигнал
доверия, MCP с каталогом ревизий в DuckDB, панель замеров сборки с экспортом.
Это и есть позиционирование.

Факты о производительности для калибровки ожиданий: Manifold против CGAL
19.8-36.7× на моделях kintel; 1.31M-треугольная union сфер 3.46 с у Manifold
и 23.9 с у CGAL на 20 ядрах; Manifold WASM «serial-only for now».

## 7. Если бы делали с нуля

1. **Один семантический эвалуатор OpenSCAD** с двумя backend'ами (mesh,
   B-rep) вместо двух эвалуаторов; Rust-фронтенд `openscad-core` либо
   становится этим эвалуатором за conformance-gate'ом, либо уходит за feature
   flag. Сейчас 9.5k строк Rust несут ноль продуктовой ценности.
2. **Arrangement-based булеан на точных предикатах** (`cad-predicates` уже
   есть) вместо BSP с бюджетами; BSP оставить для быстрых выпуклых случаев.
   Пока такого ядра нет, честная альтернатива: `manifold-rust`.
3. **Типизированные массивы в транспорте** (`value-codec` с тегом
   `f64[]`/`u32[]`) или всюду handle-путь; `Value` только для скаляров и
   метаданных.
4. **Один `analyze_solid` в ядре**: меш + нормали + BVH + рёбра + face ids +
   hash за один вызов и одну копию, вместо четырёх загрузок одного меша.
5. **Один владелец сцены**: `SceneController` авторитетен для meshes,
   visibility, selection; рендерер потребляет дельты и не хранит семантику.
6. **Общий `History<T>`, `weldKey`, `smoothNormals`, `downloadFile`,
   `cameraGestures`** для трёх workspace'ов; инструменты Solid/Mesh на том
   же BVH-picking, что и viewport.
7. **`math-core` как единственный источник векторной математики и допусков**
   для всех крейтов, включая `brep-core`.

## 8. Как воспроизвести замеры

```sh
node --import tsx benchmarks/own-cad/bench-csg-scaling.mts --out output/csg-scaling-<date>.json
```

```sh
cargo run --release -p polygon-core --example bench_boolean -- --profile quick
```

```sh
node scripts/bench-cpu.mjs --quick
```

Baseline HEAD снимался теми же командами до правок ядра; результаты в
[csg-scaling-2026-09-19.md](csg-scaling-2026-09-19.md).

## 9. План (по убыванию рычага)

| # | Работа | Затрагивает | Доказательство |
| --- | --- | --- | --- |
| 1 | Робастная триангуляция многоугольника с отверстиями (монотонное разбиение sweep-line, O(n log n), без сдвига координат) | `planar-geometry/src/triangulation.rs`, `polygon-core` `simplify`/`extrude_rings` | тест на сетках отверстий 4×4…10×10, `bench_boolean`, `bench:csg` плита 64/100 |
| 2 | ADR по булеану: arrangement на точных предикатах или `manifold-rust` | `polygon-core`, ADR 0010/Manifold-peer | union 2 сфер `$fn ≥ 48`, Menger-подобные модели |
| 3 | `analyze_solid` одним вызовом; убрать двойную валидацию публикации и `inferSurfaceIds` с главного потока | `geometry-bridge`, `meshAnalysis.ts`, `geometryWorkerProtocol.ts`, `buildCoordinator.ts`, `meshSurfaceGroups.ts` | `bench:cpu` analyze/result-validation, тесты протокола |
| 4 | Удалить 5 231 строку мёртвого кода и стейл-чанк в `vite.config.ts` | 12 файлов | `vite build` + `verify-dist` |
| 5 | Слить два TS-эвалуатора на общем ядре (`openscadEvalCore.ts`) | ~10 файлов | оракульные тесты обоих lane'ов |
| 6 | `Mesh::edges()` через сортировку; переиспользование `inspect` в булеане; `simplify` без клонов позиций | `polygon-core` | `bench_boolean`, тесты булеана |
| 7 | Clippy в CI с allow-list, восстановление членов workspace (сделано), `#[inline(always)]` и `math-core` в `brep-core` | CI, `brep-core` | CI зелёный, тесты brep-core |
| 8 | Один владелец сцены; композаблы `useRendererHost`/`usePublication`; декомпозиция рендерера по уже существующим модулям | `App.vue`, контроллеры, `webgpuRenderer.ts` | тесты рендерера и контроллеров |
| 9 | Продуктовые пробелы: PWA, Monaco, GLB, анимация `$t`, measure/секции с крышками | фронтенд | ручная проверка + Playwright |
