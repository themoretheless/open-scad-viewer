# Второй исследовательский проход: что строить дальше

Срез: **2026-07-14**. Вывод основан на второй независимой сотне
репозиториев, 25 первичных работах по geometry processing и 24
первичных/официальных источниках по CAD/HCI.

Главное продуктовое решение: развивать проект как **code-first CAD
inspector с двусторонней связью кода и результата**, а не как урезанный клон
Plasticity. SCAD-код остаётся источником истины; viewport помогает выбрать,
объяснить, измерить и безопасно подготовить изменение.

## Статус реализации на 2026-07-18

Первый vertical slice выполнен. В Worker теперь строятся transferable BVH и
semantic edges; Manifold `runOriginalID`, `runFlags` и `faceID` преобразуются в
source provenance. Viewport поддерживает hover preselection, point/face/body
режимы с отдельными point/face overlays, переход от выбранной CSG-поверхности
к диапазону SCAD и обратную подсветку всех surviving patches от caret/Outliner.
Повторный клик перебирает bounded front-to-back цели. Scene Outliner, Inspect,
двухточечное измерение и виртуальное сечение дополняют inspection workspace.
Superseded preview реально отменяется restart-ом Worker, затем
idle-запрос публикует full build; visibility, selection, isolate и measurement
переносятся в full по source provenance. Также реализованы базовый Customizer,
topology diagnostics, bounded Previous View stack и контекстная fuzzy command
palette с RU/EN aliases, MRU и причинами недоступности. STL/OBJ export разрешён
только из актуальной full-сборки.

Из P0 остаются semantic CSG tree поверх уже работающего provenance, silhouette
edges, section caps/точный контур и дополнительные snap-типы. Из P1/P2 по-прежнему открыты
multi-file/incremental editor, checkpoints, drag/26-direction ViewCube,
3MF/GLB, chunked LOD и opt-in SDF.

Отдельный [третий проход](ten-new-ideas-third-pass.md) добавил десять
недублирующих открытых направлений. Приоритетный следующий пакет: исполняемые
контракты модели, профилировщик пересборки по AST, матрица пересечений сборки и
предварительная карта пригодности к FFF-печати. Эти функции используют
готовые Worker, provenance и BVH, но дают новые пользовательские результаты и
не заменяют пункты backlog ниже.

## Материалы

- [Вторая сотня репозиториев](top-100-repositories-second.md) и
  [проверяемый TSV](top-100-repositories-second.tsv).
- [25 работ по геометрическому пайплайну](geometry-pipeline-literature.md).
- [24 источника по CAD/3D HCI](cad-hci-literature.md).
- [Уже перенесённые идеи Plasticity](plasticity-patterns.md).
- [Третий проход: ещё 10 недублирующих идей](ten-new-ideas-third-pass.md).

## Что обнаружено в текущей реализации

| Узкое место | Текущее состояние | Следствие |
|---|---|---|
| Код ↔ геометрия | Source spans, provenance runs и двусторонняя bounded cross-highlight | следующая ступень — semantic CSG tree |
| Picking | Compact Worker BVH, hover и bounded front-to-back depth-cycling | добавить более широкий screen-space snapping |
| Edges | Boundary/crease/non-manifold edge index без coplanar diagonals; typed-array radix pipeline ограничивает heap | добавить view-dependent silhouettes |
| Worker | Preview/full state machine и отмена через безопасный restart Worker | профилировать persistent pool/ExecutionContext |
| Масштаб | Worker передаёт один монолитный full-detail mesh | нет progressive first paint, LOD и chunk eviction |
| Надёжность | Наружу уходят topology diagnostics, provenance и face IDs | расширить corpus сложных CSG и kernel status |

Точки кода: [parser](../../src/services/openscadParser.ts),
[worker](../../src/workers/geometry.worker.ts),
[protocol](../../src/services/geometryWorkerProtocol.ts),
[renderer](../../src/services/webgpuRenderer.ts) и
[ray math](../../src/services/math3d.ts).

## Приоритетный backlog

Таблица ниже сохраняет исходные формулировки и теперь читается вместе со
статусом: **#1 частично** (provenance/cross-highlight готовы, semantic tree
остаётся); **#2 готов по пользовательскому результату** (BVH/cycling работают,
но текущая BVH использует balanced median split, а не записанный binned-SAH);
**#4 готов**; **#3 частично** (silhouettes остаются); **#5
частично** (section caps и дополнительные snaps остаются); **#6 частично**
(нужны расширенный degeneracy corpus и kernel status); **#7 частично**
(Customizer готов, direct handles остаются); **#8 и #9 открыты**; **#10
частично** (Previous View готов, drag/26 направлений/анимация остаются); **#11
готов**; **#12 частично**; **#13 частично** (STL/OBJ готовы, units/3MF/GLB
остаются); **#14 и #15 открыты**.

| # | Приоритет | Что добавить | MVP и причина |
|---:|---|---|---|
| 1 | P0 | **AST ↔ mesh provenance + CSG inspector** | Дать каждому вычисляемому узлу стабильный `nodeId` и `[start,end]`; пометить исходные Manifold через `reserveIDs`, вернуть `runOriginalID/faceID/runFlags`; клик подсвечивает SCAD range, курсор кода — все фрагменты результата. Это одновременно открывает outliner, semantic tree, diagnostics и будущие direct handles. |
| 2 | P0 | **Worker BVH + preselection/cycling** | Построить compact binned-SAH BVH с leaves по 8–16 triangles; текущий Möller–Trumbore оставить narrow phase. Добавить hover candidate, depth-sorted cycling и object-first filter. Это устраняет O(T) click и становится основой snapping/measure. |
| 3 | P0 | **Semantic feature edges** | Halfedge adjacency по канонической топологии Manifold; показывать boundary, non-manifold, crease по dihedral threshold и silhouette, скрывая coplanar diagonals. Это даёт CAD-читаемость вместо wireframe триангуляции. |
| 4 | P0 | **Cancellable preview → full build** | Протокол `quality=preview|full`, явные состояния `preview/stale/full`, idle full build и отмена superseded evaluation через Manifold `ExecutionContext` либо безопасный restart Worker. Не называть full результат mathematically exact; экспорт разрешать только из full. |
| 5 | P0 | **Inspect workspace** | Picked point/normal, координаты, distance, bounding box, area/volume, snapping и section plane. BVH обслуживает hit/nearest queries; Manifold `slice/splitByPlane` даёт контур/разрез. Единицы по умолчанию — `model units`, пока SCAD не задаёт их явно. |
| 6 | P1 | **Topology diagnostics и degeneracy corpus** | Возвращать kernel status, finite/degenerate counts, edge incidence/orientation; тестировать coincident, coplanar, tangent, tiny/huge scale и chained CSG. Repair — только отдельная подтверждаемая команда, никогда молча. |
| 7 | P1 | **Customizer + безопасные direct handles** | Автогенерировать controls из top-level variables и OpenSCAD metadata comments. Затем разрешить drag transform/dimension только как preview source patch с подтверждением. Это сохраняет код источником истины и закрывает проблемы spatial math. |
| 8 | P1 | **Incremental SCAD tooling и multi-file workspace** | Monaco/CodeMirror + incremental syntax tree, completion/hover/markers, `include/use`, virtual filesystem и dependency graph. Cache вычислений должен быть по AST subtree + environment, а не по строке целого файла. |
| 9 | P1 | **Source history, checkpoints и diff** | Сохранять snapshot только после успешного full build, добавлять named checkpoints, diff/restore и ghost previous result. Историю камеры держать отдельно; не пытаться «обратить» произвольный CSG. |
| 10 | P1 | **Safe camera** | Drag ViewCube, 26 snap orientations, pivot по hit/selection, animated transition и Previous View stack. Текущий cube уже создаёт правильную основу, но click-only навигация не завершает паттерн Plasticity/ViewCube. |
| 11 | P1 | **Контекстная command palette** | RU/EN aliases, fuzzy search, MRU, state-gated commands и объяснение недоступности. Это небольшой слой поверх уже существующего реестра команд. |
| 12 | P1 | **Semantic accessible viewport** | DOM-дерево узлов, keyboard select/focus/isolate/measure, live compile/status, model summary, high contrast, reduced motion и кнопочные альтернативы drag. Основа — тот же provenance, поэтому это не отдельная параллельная архитектура. |
| 13 | P1 | **Export center: 3MF + STL + GLB** | Экспортировать только full result; спрашивать units. 3MF — основной manufacturing format с manifold/units/material metadata, STL — compatibility, GLB с `EXT_mesh_manifold` — web/share. |
| 14 | P2 | **Chunked QEM LOD renderer** | Только после профилей: Worker `begin/chunk/end`, chunks 8–32k triangles, 2–4 crease-preserving QEM LOD, projected error, frustum culling и GPU-buffer LRU. Full geometry и metrics остаются в Worker. |
| 15 | P2 | **Opt-in sparse SDF repair/offset** | Narrow-band sparse bricks + feature-sensitive Dual Contouring как явно approximate tool для dirty imports/offsets. Показывать voxel/error/volume delta; никогда не подменять этим обычную CSG-семантику. |

## Почему provenance — первый шаг

Исследования OpenSCAD прямо называют связь кода с 3D-видом, spatial
transformations и измерения ключевыми трудностями пользователей. UIST 2024
также показал пользу извлечения параметрических выражений через выбранную
геометрию. На уровне ядра идея практически реализуема: Manifold сохраняет
`originalID` и `faceID` исходных mesh через Boolean и отдаёт их triangle runs.

Это не решает persistent naming B-rep граней. Гарантия должна быть
скромнее: «этот фрагмент результата происходит из такого AST/source object».
Этого достаточно для cross-highlight, outliner, subtraction-surface coloring,
measure context и безопасной подготовки source patch.

## Рекомендуемый порядок реализации

1. Расширить AST и Worker protocol: `nodeId`, source span, provenance runs.
2. В том же Worker построить два общих индекса: compact BVH и halfedge
   adjacency. Не дублировать topology для picking, snapping и edges.
3. Подключить hover/preselection, code cross-highlight и source-aware outliner.
4. Добавить preview/full state machine и настоящую отмену.
5. Поверх готовых hit/provenance primitives сделать measure, section и
   Customizer.
6. После benchmark реальных моделей решать, нужен ли chunked LOD или SDF.

Первый законченый vertical slice должен проверять:

- click по Boolean result выбирает правильный source range;
- один source node подсвечивает все сохранившиеся result patches после
  transform/union/difference;
- BVH hit совпадает с brute-force hit на property/random-ray tests;
- cube/cylinder/sphere не показывают coplanar triangulation diagonals;
- rapid typing публикует только последний request и не держит stale build в
  очереди;
- provenance и выбор остаются корректны после Worker transfer и device-loss
  rebuild.

## Что пока не стоит делать

- Не копировать Plasticity push/pull, exact fillets, edge loops и G0–G3
  analysis: они предполагают persistent B-rep/NURBS topology.
- Не писать собственный epsilon-based Boolean на TypeScript; exact predicates
  и constructions должны приходить из проверенного kernel/WASM.
- Не начинать с GPU BVH/LBVH: при текущем cap CPU Worker SAH проще и надёжнее.
- Не включать automatic mesh repair: оно может незаметно изменить размеры и
  топологию пользовательской модели.
- Не обещать «exact geometry» для текущего Manifold backend. Он гарантирует
  manifold topology в своих входных условиях, но не является exact-arithmetic
  CSG kernel.

## Ключевые первичные опоры

- [OpenSCAD user challenges, CHI 2024](https://doi.org/10.1145/3613904.3642566).
- [Parametric definition in programming CAD, UIST 2024](https://doi.org/10.1145/3654777.3676417).
- [Bidirectional programming in CSG CAD](https://arxiv.org/abs/2408.01801).
- [BVH survey, Eurographics 2021](https://diglib.eg.org/items/efde7a39-536b-4a85-8901-c42cd401b859).
- [Mesh Arrangements for Solid Geometry](https://www.cs.columbia.edu/cg/mesh-arrangements/).
- [Quadric Error Metrics](https://mgarland.org/research/quadrics.html).
- [ViewCube, I3D 2008](https://doi.org/10.1145/1342250.1342253).
- [3MF specification](https://3mf.io/spec/).
