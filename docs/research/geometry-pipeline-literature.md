# Научный обзор геометрического пайплайна open-scad-viewer

Срез источников: 2026-07-14. Ниже — 25 первичных работ: статьи в proceedings/журналах, страницы авторов и arXiv для явно обозначенного препринта 2026 года. Обзор привязан к текущему коду: лимит модели 750 000 треугольников; picking после object AABB линейно перебирает все треугольники; edge mode создаёт шесть индексов на каждый исходный треугольник и показывает рёбра триангуляции; Worker возвращает только один монолитный full-detail результат.

## 1. Robust mesh booleans, exact predicates, manifold repair

1. Jonathan R. Shewchuk — “Adaptive Precision Floating-Point Arithmetic and Fast Robust Geometric Predicates” (1997), Discrete & Computational Geometry 18(3), 305–363. [Официальная страница автора и код](https://www.cs.cmu.edu/~quake/robust.html).
   - Адаптивные фильтры дают точный знак orientation/incircle-предикатов: почти всегда остаются на быстром float-пути и повышают точность только для почти вырожденного случая.
   - Здесь: если появится собственная классификация пересечений, триангуляция или repair, решения о топологии нельзя принимать через один EPSILON. Нужен proven predicate kernel; текущий Möller–Trumbore допустим для picking, но не как основа CSG.

2. Marcel Campen, Leif Kobbelt — “Exact and Robust (Self-)Intersections for Polygonal Meshes” (2010), Computer Graphics Forum 29(2). [DOI 10.1111/j.1467-8659.2009.01609.x](https://doi.org/10.1111/j.1467-8659.2009.01609.x).
   - Работа локализует дорогую exact-обработку: adaptive octree находит критические ячейки, внутри них используется plane-based BSP.
   - Здесь: будущий импорт/repair должен иметь broad phase по AABB/BVH и exact narrow phase только вокруг пересечений; глобальный exact-проход по всей модели для браузера не нужен.

3. Marco Attene — “A Lightweight Approach to Repairing Digitized Polygon Meshes” (2010), The Visual Computer 26, 1393–1406. [DOI 10.1007/s00371-010-0416-3](https://doi.org/10.1007/s00371-010-0416-3).
   - Repair меняет тесную окрестность дефекта, восстанавливая orientation/manifoldness/holes, а не пересэмплирует всю поверхность.
   - Здесь: если добавить Repair, он должен быть отдельной подтверждаемой командой с отчётом before/after; автоматическое молчаливое исправление исходной OpenSCAD-геометрии недопустимо.

4. Alec Jacobson, Ladislav Kavan, Olga Sorkine-Hornung — “Robust Inside-Outside Segmentation using Generalized Winding Numbers” (2013), ACM TOG 32(4). [DOI 10.1145/2461912.2461916](https://doi.org/10.1145/2461912.2461916).
   - Generalized winding number корректен для watertight-модели и плавно деградирует на дырках, self-intersections и non-manifold fragments.
   - Здесь: полезен как диагностический inside/outside слой для будущих импортированных meshes и как проверка результата; не следует подменять им exact OpenSCAD semantics.

5. Qingnan Zhou, Eitan Grinspun, Denis Zorin, Alec Jacobson — “Mesh Arrangements for Solid Geometry” (2016), ACM TOG 35(4). [Официальная project page](https://www.cs.columbia.edu/cg/mesh-arrangements/), [DOI 10.1145/2897824.2925901](https://doi.org/10.1145/2897824.2925901).
   - Сначала строится arrangement всех пересечений, затем Boolean expression извлекает границу по winding-vector; это разделяет геометрию пересечений и семантику операции.
   - Здесь: CSG должен оставаться DAG/variadic operation, а не цепочкой округлённых binary exports. Для будущего exact mode arrangement можно переиспользовать для нескольких выражений.

6. Gavin Barill, Neil G. Dickson, Ryan Schmidt, David I. W. Levin, Alec Jacobson — “Fast Winding Numbers for Soups and Clouds” (2018), ACM TOG 37(4). [Официальная project page и код](https://www.dgp.toronto.edu/projects/fast-winding-numbers/), [DOI 10.1145/3197517.3201337](https://doi.org/10.1145/3197517.3201337).
   - Иерархическое приближение сокращает inside/outside queries по triangle soup с помощью дерева; работа показывает применение к signing distances, voxelization и defect-tolerant booleans.
   - Здесь: один и тот же BVH-подобный spatial index может обслуживать диагностику, SDF sampling и будущие section/measurement tools.

7. Gianmarco Cherchi, Marco Livesu, Riccardo Scateni, Marco Attene — “Fast and Robust Mesh Arrangements using Floating-point Arithmetic” (2020), ACM TOG 39(6). [DOI 10.1145/3414685.3417818](https://doi.org/10.1145/3414685.3417818).
   - Indirect predicates хранят intersection points как невычисленные выражения от входных вершин; топологические решения остаются exact, а основной путь использует float hardware и хорошо параллелится.
   - Здесь: не писать собственный epsilon-boolean на TypeScript. Если Manifold перестанет удовлетворять тестам, следующий kernel должен быть WASM-портом proven implementation и проверяться на degeneracies.

8. Gianmarco Cherchi, Fabio Pellacini, Marco Attene, Marco Livesu — “Interactive and Robust Mesh Booleans” (2022), ACM TOG 41(6), Article 248. [DOI 10.1145/3550454.3555460](https://doi.org/10.1145/3550454.3555460).
   - Robust Boolean разделён на resolution of intersections и merge of conforming patches; авторы демонстрируют interactive frame rates до примерно 200k triangles и variadic cases.
   - Здесь: завести отдельный performance target «редакторский preview <=200k», не смешивая его с нынешним hard cap 750k для финального результата.

9. Bruno Lévy — “Exact Predicates, Exact Constructions and Combinatorics for Mesh CSG” (TOG 2025; preprint 2024). [ACM DOI 10.1145/3744642](https://doi.org/10.1145/3744642), [arXiv 2405.12949](https://arxiv.org/abs/2405.12949).
   - Exact constructions + constrained Delaunay triangulation дают однозначную ретриангуляцию coplanar overlaps; Weiler model хранит volumetric adjacency. Работа отдельно обсуждает удаление «шрамов» после chained CSG.
   - Здесь: добавить corpus из ThingiCSG и тесты coincident/coplanar/nested booleans; именно эти случаи должны определять, нужен ли когда-либо отдельный exact/server mode.

## 2. SDF и гибридное представление

10. Sarah F. Frisken, Ronald N. Perry, Alyn P. Rockwood, Thouis R. Jones — “Adaptively Sampled Distance Fields: A General Representation of Shape for Computer Graphics” (2000), SIGGRAPH. [DOI 10.1145/344779.344899](https://doi.org/10.1145/344779.344899).
    - Adaptive distance field концентрирует samples у сложной поверхности и объединяет offsets, collision, sculpting и LOD.
    - Здесь: SDF имеет смысл как opt-in approximate backend для preview/offset/repair, но не как замена mesh CSG для точных плоскостей и размеров.

11. Leif Kobbelt, Mario Botsch, Ulrich Schwanecke, Hans-Peter Seidel — “Feature Sensitive Surface Extraction from Volume Data” (2001), SIGGRAPH. [DOI 10.1145/383259.383265](https://doi.org/10.1145/383259.383265).
    - Feature-sensitive sampling сохраняет sharp edges и corners при меньшем числе полигонов, чем слепой uniform contouring.
    - Здесь: любой SDF fallback должен хранить gradient/Hermite-like information; обычный Marching Cubes сгладит CAD-фаски и отверстия.

12. Tao Ju, Frank Losasso, Scott Schaefer, Joe Warren — “Dual Contouring of Hermite Data” (2002), SIGGRAPH, 339–346. [DOI 10.1145/566570.566586](https://doi.org/10.1145/566570.566586), [PDF автора](https://www.cse.wustl.edu/~taoju/research/dualContour.pdf).
    - QEF размещает вершину по intersection points и normals, сохраняя sharp features; adaptive octree extraction не требует crack patching и содержит topology test.
    - Здесь: это предпочтительный extraction stage для sparse SDF, но нужны crack/topology regression tests на соседних уровнях octree.

13. Ken Museth — “VDB: High-Resolution Sparse Volumes with Dynamic Topology” (2013), ACM TOG 32(3). [DOI 10.1145/2487228.2487235](https://doi.org/10.1145/2487228.2487235), [официальная документация OpenVDB](https://www.openvdb.org/documentation/).
    - Иерархические tiles/leaves делают память пропорциональной активной узкой полосе, а не объёму bounding box, сохраняя быстрый random access.
    - Здесь: плотная 3D texture/grid в браузере исключена. SDF следует строить narrow-band bricks в Worker с жёстким memory budget.

14. Xiana Carrera, Ningna Wang, Christopher Batty, Oded Stein, Silvia Sellán — “Dual Contouring of Signed Distance Data” (arXiv preprint, 2026). [arXiv 2604.00157](https://arxiv.org/abs/2604.00157).
    - Новый QEF восстанавливает sharp features только из дискретных SDF samples, без произвольного доступа к функции и без gradient input.
    - Здесь: снижает сложность экспериментального browser prototype, но это свежий непрошедший production battle-testing препринт; только P2 и feature flag.

## 3. Tessellation и feature edges

15. Pierre Alliez, David Cohen-Steiner, Olivier Devillers, Bruno Lévy, Mathieu Desbrun — “Anisotropic Polygonal Remeshing” (2003), ACM TOG 22(3). [DOI 10.1145/882262.882296](https://doi.org/10.1145/882262.882296).
    - Curvature directions управляют плотностью и ориентацией tessellation, сохраняя anisotropic features меньшим числом элементов.
    - Здесь: display-only LOD должен закреплять boundaries/creases и расходовать triangles на curvature, а не равномерно; измерения и export остаются full-resolution.

16. Aaron Hertzmann, Denis Zorin — “Illustrating Smooth Surfaces” (2000), SIGGRAPH. [Официальная project page](https://mrl.cs.nyu.edu/publications/illustrating-smooth/), [DOI 10.1145/344779.345074](https://doi.org/10.1145/344779.345074).
    - Silhouettes, sharp creases и object boundaries передают инженерную форму гораздо лучше, чем все рёбра триангуляции.
    - Здесь: режим Edges должен строить semantic edge set: boundary + non-manifold + dihedral crease + view-dependent silhouette.

17. Doug DeCarlo, Adam Finkelstein, Szymon Rusinkiewicz, Anthony Santella — “Suggestive Contours for Conveying Shape” (2003), ACM TOG 22(3). [Официальная project page и sample implementation](https://gfx.cs.princeton.edu/gfx/pubs/DeCarlo_2003_SCF/index.php).
    - Zero crossings radial curvature добавляют линии там, где contour появится при малом изменении view, и лучше объясняют гладкую форму.
    - Здесь: возможный P2 «technical illustration» mode после semantic edges; не P0, потому что производные на faceted CSG mesh шумны и линии меняются с камерой.

## 4. BVH и ray picking

18. Tomas Möller, Ben Trumbore — “Fast, Minimum Storage Ray-Triangle Intersection” (1997), Journal of Graphics Tools 2(1). [DOI 10.1080/10867651.1997.10487468](https://doi.org/10.1080/10867651.1997.10487468).
    - Barycentric ray/triangle test не хранит plane equations и является хорошим узким тестом.
    - Здесь: нынешний math3d уже использует этот принцип; менять narrow phase не нужно, нужно перестать вызывать его для каждого triangle.

19. Ingo Wald — “On Fast Construction of SAH-based Bounding Volume Hierarchies” (2007), IEEE Symposium on Interactive Ray Tracing. [DOI 10.1109/RT.2007.4342588](https://doi.org/10.1109/RT.2007.4342588), [PDF автора](https://www.sci.utah.edu/publications/wald07/fastbuild.pdf).
    - Binned SAH даёт качественное дерево при достаточно быстрой полной перестройке.
    - Здесь: build per-mesh BVH в geometry Worker, leaf 8–16 triangles, near-first traversal и раннее ограничение nearest distance. Проверять random rays против нынешнего brute force.

20. Tero Karras — “Maximizing Parallelism in the Construction of BVHs, Octrees, and k-d Trees” (2012), High Performance Graphics. [Официальная NVIDIA Research page](https://research.nvidia.com/publication/2012-06_maximizing-parallelism-construction-bvhs-octrees-and-k-d-trees).
    - Binary radix tree по Morton codes строится полностью параллельно и подходит GPU/LBVH.
    - Здесь: WebGPU compute BVH оправдан только после перехода к многомиллионным meshes; при нынешнем cap Worker SAH проще, стабильнее и дешевле в разработке.

## 5. Simplification, LOD, progressive/out-of-core

21. Hugues Hoppe — “Progressive Meshes” (1996), SIGGRAPH, 99–108. [Официальная project page](https://hhoppe.com/proj/pm/), [DOI 10.1145/237170.237216](https://doi.org/10.1145/237170.237216).
    - Base mesh + vertex splits образуют continuous-resolution stream, поддерживают progressive transmission и selective refinement с сохранением attributes.
    - Здесь: renderer protocol можно расширить до coarse-first chunks/refinements; selection IDs, material/color и normals должны переживать LOD.

22. Michael Garland, Paul S. Heckbert — “Surface Simplification Using Quadric Error Metrics” (1997), SIGGRAPH. [DOI 10.1145/258734.258849](https://doi.org/10.1145/258734.258849), [страница автора и код](https://mgarland.org/research/quadrics.html).
    - QEM хранит компактную локальную оценку геометрической ошибки и даёт быстрые edge/pair contractions.
    - Здесь: Worker может генерировать display-only LOD; boundary/crease constraints обязательны, а volume/surfaceArea и export должны считаться по исходной сетке.

23. Peter Lindstrom — “Out-of-Core Simplification of Large Polygonal Models” (2000), SIGGRAPH. [DOI 10.1145/344779.344912](https://doi.org/10.1145/344779.344912), [PDF](https://www.cs.princeton.edu/courses/archive/spr01/cs598b/papers/lindstrom00.pdf).
    - Single-pass clustering с quadrics ограничивает память размером output и обрабатывает модель в linear time.
    - Здесь: после снятия cap 750k полную mesh нельзя сначала материализовать в UI; упрощение/partitioning должно происходить в Worker до transfer.

24. Paolo Cignoni, Fabio Ganovelli, Enrico Gobbetti, Fabio Marton, Federico Ponchio, Roberto Scopigno — “Batched Multi Triangulation” (2005), IEEE Visualization, 207–214. [DOI 10.1109/VISUAL.2005.1532797](https://doi.org/10.1109/VISUAL.2005.1532797), [официальная Nexus/BMT page](https://vcg.isti.cnr.it/vcgtools/nexus/).
    - Mesh fragments разных resolutions образуют DAG, но GPU получает готовые triangle patches; coarse-grained batching уменьшает traversal/draw overhead.
    - Здесь: chunks по 8–32k triangles, screen-space error, frustum visibility, GPU-buffer LRU; WebGPU storage/indirect buffers позволяют позднее добавить GPU-driven selection.

25. Martin Isenburg, Peter Lindstrom — “Streaming Meshes” (2005), IEEE Visualization, 231–238. [DOI 10.1109/VISUAL.2005.1532800](https://doi.org/10.1109/VISUAL.2005.1532800), [официальная страница автора LLNL](https://people.llnl.gov/lindstrom2).
    - Streamable ordering ограничивает active vertex window и позволяет sequential processing огромных meshes.
    - Здесь: GeometryResponse должен эволюционировать от одного массива к begin/chunk/end messages; Worker и renderer смогут освобождать временные arrays по мере загрузки.

## Важная граница текущего backend

Текущий mesh backend — собственный Rust CAD kernel. UI/README не должны называть его exact. Нужны topology/status diagnostics и degeneracy corpus. Exact kernel имеет смысл только как отдельно измеренный режим, если тесты докажут проблему.

WebGPU уже предоставляет storage buffers, compute и indirect draw/dispatch arguments, поэтому chunked GPU-driven path технически возможен. Это инженерная спецификация, не научная работа: [актуальный WebGPU Editor’s Draft](https://gpuweb.github.io/gpuweb/).

## Ровно шесть предложений

### P0-A — BVH для picking

- Что строить: в geometry Worker построить для каждого MeshData binned-SAH BVH; packed typed arrays: node AABB + child/first/count, leaf 8–16 triangles. Передавать BVH transferables вместе с mesh. В math3d оставить текущий Möller–Trumbore как leaf narrow phase, обходить near-first и отбрасывать nodes дальше текущего nearest.
- Проверка: property test на случайных rays — BVH distance/index строго совпадает с существующим brute force; benchmark на 10k/100k/750k triangles.
- Эффект: click из O(T) становится примерно O(log T + candidates), UI не зависает на верхнем лимите.
- Риск: низкий. Основной риск — память; не делать «один triangle = один leaf», использовать компактные arrays.

### P0-B — semantic feature edges вместо рёбер триангуляции

- Что строить: в Worker canonicalize topology с учётом Manifold mergeFromVert/mergeToVert, построить undirected half-edge adjacency. В static edge buffer включать boundaries, non-manifold edges и creases по dihedral threshold (старт 30°). Silhouette определять по знакам faceNormal·view и обновлять отдельно; internal coplanar diagonals скрыть.
- Проверка: cube/cylinder/sphere/boolean seams, property-split vertices, non-manifold synthetic mesh; memory benchmark против нынешних шести index values на triangle.
- Эффект: Plasticity/CAD-подобный читаемый viewport и меньше edge-buffer.
- Риск: средний. Нельзя weld только по округлённым позициям — можно ошибочно склеить близкие независимые детали.

### P1-A — topology/status diagnostics + robustness corpus

- Что строить: до transfer вызывать Manifold status(), затем проверять finite vertices, degenerate triangle count и canonical edge incidence/orientation; вернуть эти поля в GeometrySuccess и показывать compact warning/details. Добавить fixtures: coincident faces, coplanar cuts, tangent solids, tiny/huge scale, repeated chained CSG, ThingiCSG subset.
- Проверка: ни один invalid status не должен тихо превращаться в пустой mesh; volume/bounds/manifold invariants фиксируются snapshot-тестами.
- Эффект: честно закрывает разницу между «guaranteed topological manifold» и exact geometry и ловит backend regressions при npm updates.
- Риск: низко-средний. Полная edge map на 750k triangles затратна; делать в Worker, canonical indices и бюджет памяти.

### P1-B — двухступенчатый preview/full build с настоящей отменой

- Что строить: расширить request protocol quality=preview|final и поддержать OpenSCAD-подобный $preview. Preview снижает только автоматическую tessellation/quality budget; после idle запускается final. Старый расчёт не просто игнорируется, а отменяется через Manifold ExecutionContext/cancel, иначе текущая serial queue блокирует свежий результат. Метрики маркировать preview/final.
- Проверка: rapid typing 20 updates, гарантированно выводится последний source; explicit user $fn не меняется без документированного preview rule; нет скачка stale metrics.
- Эффект: быстрый feedback при редактировании, при этом финальная геометрия остаётся полной.
- Риск: средний. Два расчёта могут увеличить CPU, если cancellation не проходит через eager evaluation; UI должен явно обозначать качество.

### P2-A — opt-in sparse SDF repair/offset backend

- Что строить: отдельная команда Approximate repair/offset; narrow-band sparse bricks/octree в Worker, sampling через BVH/fast winding, feature-sensitive Dual Contouring, error/voxel-size control и before/after Hausdorff/volume estimate. Никогда не включать автоматически для обычного CSG.
- Проверка: sharp CAD corners, thin walls, small holes, disconnected shells; показать numerical error и запретить параметры, которые съедают минимальную толщину.
- Эффект: сможет обрабатывать dirty imported meshes и сложные offsets, где surface Boolean не подходит.
- Риск: высокий. Потеря exact dimensions/topology, большой WASM memory, cracks между levels и сложная UX-семантика.

### P2-B — chunked multi-resolution WebGPU renderer

- Что строить: Worker выдаёт begin/chunk/end; chunks 8–32k triangles имеют AABB, geometric error и 2–4 QEM LOD. Renderer выбирает по projected error, делает frustum culling, держит GPU-buffer LRU; позже объединяет visible draw args в indirect buffers. Full-resolution mesh/metrics остаются в Worker, selection хранит stable object/patch IDs.
- Проверка: камера у границ chunks без cracks/popping, deterministic picking across LOD, device-loss rebuild, жёсткие CPU/GPU memory budgets.
- Эффект: путь к моделям выше нынешних 750k и progressive first paint без монолитного transfer.
- Риск: высокий. Crack prevention, stable IDs, cache eviction и lifecycle GPU buffers существенно усложняют renderer; делать только после P0/P1 и реальных профилей.

## Рекомендуемый порядок

1. P0-A BVH picking.
2. P0-B semantic edges.
3. P1-A diagnostics/corpus.
4. P1-B cancellable preview.
5. Профилирование реальных моделей.
6. Только затем выбирать между P2-A SDF и P2-B chunked LOD по подтверждённому bottleneck.
