# Владельцы алгоритмов: аудит TS → Rust

Дата: 2026-10-03. Аудит границ и остатка миграции завершён для production `src/`.
Перенос перечисленного остатка ещё не завершён. Аудит не доказывает численную
эквивалентность, полноту CAD-операций или ускорение приложения.

## Как проверено

- Перепись всех TS, Vue script и MJS в `src/`: 464 файла, 8108 объявлений
  функций, методов и callbacks. Callbacks учитываются отдельно; это **не число
  самостоятельных алгоритмов**.
- AST-перепись хранит хеш файла, функции, циклы и вызовы `Math`. Комментарии и
  Unicode `String.normalize` не считаются численной геометрией. Generated-файлы
  отмечены отдельно. Координаты Vue относятся к объединённому script, а не SFC.
- Просмотрены остатки геометрии/языка, сопоставлены существующие Rust-модули и
  вызовы. Для смешанных файлов решение относится к указанным операциям;
  обработчики UI и транспорт не становятся Rust-алгоритмами автоматически.
- `Math` используется как дополнительный сигнал. Алгоритмы без `Math` (графы,
  топология, сортировка, разбор, операции над буферами) входят в перепись и
  доменные границы ниже. Перепись не является автоматическим доказательством
  отсутствия скрытого алгоритма внутри callback.
- Tests, benchmarks, scripts, vendor и встроенные WGSL не входят в перепись
  production TS. Тестовые эталоны могут иметь собственную реализацию; production
  не должен импортировать их. Владельцы GPU-алгоритмов перечислены ниже.

Машинные решения: [algorithm-owners.json](algorithm-owners.json).
Перепись с хешами: [qualification](../qualification/algorithm-owners-2026-10-03.json).

```sh
node scripts/audit-algorithm-owners.mjs --check
# После просмотра изменений и обновления решений:
node scripts/audit-algorithm-owners.mjs --write
npm run verify:libraries
```

Проверка обнаруживает изменение переписи, новые неразобранные файлы с
тригонометрией/нормами и устаревшие пути решений. Она не заменяет code review:
новый предикат без `Math` требует назначения владельца при просмотре diff.

## Единые владельцы

| Алгоритмы | Владелец | Что остаётся в TS |
| --- | --- | --- |
| Векторы, affine/projective, численная инфраструктура | `osv-math` (`crates/math-core`), устойчивые предикаты — `cad-predicates` | Камера, интерполяция отображения, CSS-проекция; без изменения геометрии документа |
| BVH, лучи, ближайшие точки, расстояния | `mesh-query` | Кеш, срок жизни snapshot, выбор и подсветка |
| Сечения и проекция мешей, контуры | `mesh-section` | Выбор плоскости, экспортная оболочка |
| Смежность, рёбра, диагностика сетки | `mesh-topology`; проверки manifold-контракта — `manifold-core` | Параметры, транспорт и адаптация ошибок |
| Mesh boolean/edit/shell, нормали и display preparation | `polygon-core` | GPU-буферы, материалы, загрузка и публикация результата |
| Совместные операции представлений, keypoints, программы, BSP/clipping | `geometry-ops` | Маршрутизация разрешённого API, кеш, per-frame обход готового BSP |
| Плоские кривые, профили, offsets, trim, пересечения, центры, привязки | `planar-geometry` | Ранжирование в CSS-пикселях, gestures, draft/undo |
| Ограничения эскиза и решение системы | `sketch-core` | Редактор ограничений; построение профиля принадлежит `planar-geometry` |
| STL/3MF и прочие mesh codecs | `mesh-io` | File/Blob, download, browser asset/ZIP/XML оболочка до переноса bounded decoder |
| NURBS evaluation/intersection, closure, сертифицированные границы ошибки | `nurbs-core` | Хранение документа, инспекция отчётов и transport |
| B-rep topology/solid operations | `brep-topology` / `brep-core` | Диагностика, leases и UI; ограничения ядра сохраняются |
| Subdivision, поля, механические профили | `subdivision-core`, `sdf-core`, `mechanical-core` | Параметры и отображение |
| Статика, thermal/truss/bonded расчёты | `mechanics-core` | Сценарий нагрузки, визуализация результата |
| OpenSCAD parse/eval, числа, fragments и языковая семантика | `openscad-core` | Файлы проекта, runtime integration, форматирование предупреждений |
| ModelGraph lowering/execution | `modelgraph-text` / `modelgraph-runtime` | Runtime admission, build/publication, history; существующие TS compiler paths требуют parity перед заменой |
| Photogrammetry | `photogrammetry-core` | Browser image input, worker/GPU dispatch, preview placement |
| Slicing/G-code/print | `slicer-core`, `gcode-core`, `gcode-optimize`, `printer-core` | Preview transport и device/session I/O |
| GPU numeric work | Соответствующее доменное ядро через `gpu-compute` / compute backend; raster — `raster-core` | WebGPU device, pipelines, uploads и readback |
| Поиск, scheduling, storage, editor splices, сеть, IDs | Application host | Остаются TS: это алгоритмы приложения, а не геометрические ядра |

Bridge принадлежит кодированию, проверке ABI-пределов и преобразованию ошибок.
Он не становится вторым владельцем формул. `polygon-core::solid::{bvh,section,
proximity}` совместимы как фасады; новых реализаций туда не добавлять.
Новые crates для этого остатка не нужны.

## Остаток с конкретными границами

| Порядок | Где и какие операции | Решение и контракт проверки |
| --- | --- | --- |
| 1 | `modelingSnaps.ts`: line/line, line/circle, circle/circle, tangent/perpendicular, arc membership, centroid/quadrants; `DirectModeler.vue`: arc snap sampling, trimAt segment search | Один bounded API в `planar-geometry` возвращает кандидаты. TS выбирает по CSS-проекции. Проверить 3D planes, вырожденные отрезки, касания и прежние tolerances |
| 2 | `solidGpuView.ts::smoothTriangleList`, `photogrammetry/preview.ts::fillNormals`, face normals в `modelGraphTextScene.ts`, `DirectModeler.vue`, `nativeFaceSelection.ts` | Mesh normals/expansion — `polygon-core`, anchor geometry — `geometry-ops`. Сохранить разные flat/area-weighted/crease policies. `toFixed(5)` grouping в smoothTriangleList — отдельная legacy policy, не разрешение topology weld |
| 3 | `solidPointEdit.ts::applySolidPointEdit`: radius/start/end handles, vertex delta; `directModelingTools.ts::circularDirectCopies`; `mainModeling.ts`: circle profile | Координаты — `planar-geometry` / `geometry-ops::point_transform`; IDs, snapshot, selection, history — TS. Проверить signed sweep, полный круг, повторные vertex IDs и ограничения радиуса |
| 4 | `mainSolidExtensions.ts`: prism/profile matching, specializedShell/Bevel, sphereShell | `polygon-core`; не скрывать эвристическое распознавание формы. Сохранить текущие отказы и границы допустимости |
| 5 | `openScadImport.ts`: STL/OFF parsing, degenerate filtering, face ear clipping, mesh placement, DXF bulges/ellipses/contour stitching, dimension/cross | Разделить mesh codec (`mesh-io`), planar geometry (`planar-geometry`), язык (`openscad-core`). `mesh-io` сейчас не содержит всего этого decoder coverage. Сохранить budget, ошибки формата и rounding/endpoint policy |
| 6 | `svgGeometry.ts::meshSvgContours`: plane basis, projection и per-triangle area; `webgpuRenderer.ts::emitMeasurementChange` | Пакетная проекция — `mesh-section` с `osv-math`; model distance — `mesh-query`. SVG strings и callback — TS |
| 7 | `openScadBuiltinFunctions.ts`: round/norm и остальные built-ins; `openscadSemanticLowerer.ts`: rotation/mirror formulas; TS evaluator/value/scope paths | `openscad-core`, затем общая native execution integration. Проверить profiles, Undef/nonfinite/-0 и порядок предупреждений. Парный degree API уже native; глобальное batching ещё открыто |
| 8 | `nurbsErrorComposition.ts`, closure guards в `solidNurbs.ts`, `modelGraphNurbsKernel.ts` | `nurbs-core`: outward binary64 rounding, subnormal/overflow refusal, closure domain. Не заменить сертифицированную границу обычным округлением |
| 9 | `math3d.ts::rayIndexedMeshDistance`, `rayTriangleDistance` | У indexed-ray нет production callers, есть `tests/math3d.test.ts`. Перенести эталон в tests или удалить вместе с переработкой теста. Активные rays обслуживает `mesh-query`; AABB helper оценить отдельно по callers |
| 10 | Порог `Math.cos` в `meshTopology.ts` и `geometry/meshAnalysis.ts` | Оставить адаптер до проверки bit-level compatibility. Сравнение и топология уже в Rust. Отдельный ABI на один cos здесь не улучшает архитектуру |

Нормали, camera projection и language transforms — подтверждённые места
нескольких реализаций. Одинаковое название операции не означает одинаковые
правила: перед DRY-консолидацией зафиксировать policy и parity fixtures.

## Обоснованно оставленные вычисления

- `orbitCameraProjection`, `viewportModel`, `geometryTransformTransition`,
  `viewFrustum`, `uniformFill`: камера, анимация, culling и освещение. Они не
  записывают геометрию модели. Повторные camera formulas следует централизовать
  в viewport service; перенос через ABI оценивается пакетно и по профилю.
- `transparentOrdering` и обход BSP: порядок отображения зависит от камеры.
  Построение BSP и triangle clipping уже в `geometry-ops`.
- Preview normalization в photogrammetry не применяется к экспортным
  координатам. Browser image decoding и preprocessing остаются host pipeline.
- Pixel distance, click thresholds, nearest visible control, grid gestures,
  viewport gizmos, transient drawing preview остаются host. Если preview
  становится сохранённым профилем, его геометрия должна прийти из native API.
- Selection identity/revision checks, overlay deduplication, transport limits,
  Unicode/search, source editing, persistence, cancellation и resource lifetime
  остаются владельцами соответствующих application services.

## Статус

Назначение владельцев и фиксация остатка закрыты этим аудитом. Native evaluator
пока opt-in; production TS semantics и перечисленная геометрия не считаются
перенесёнными. Дальнейшая работа идёт по таблице остатка с native tests, WASM
parity и измерением пакетного transport, без нового дробления библиотек.
