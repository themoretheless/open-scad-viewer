# P0: исходная геометрия в сцене

2026-09-08. Реализован первый вертикальный срез [архитектуры](native-geometry-architecture.md).

`MeshData.nativeGeometry` сохраняет versioned immutable JSON snapshot: evaluated f64 geometry, границы trim (если есть), исходный нормализованный ModelGraph, node ID и SHA-256 геометрической/документной ревизии. Render vertices остаются Float32, но больше не служат единственным геометрическим представлением опубликованного объекта. Payload ограничен суммарно 4 Mi characters; Worker проверяет структуру и hashes. Hash подтверждает целостность данных, не геометрическую правильность и не соответствие произвольного результата исходной программе. Последующие native операции обязаны валидировать данные своим ядром.

Native surface и B-rep root можно `show` без авторского tessellate. Viewer задаёт собственную display policy; native surface/B-rep/subdivision/patches могут получить производную сетку без изменения source. Пример: [native-surface.mg](../../examples/modelgraph-text/native-surface.mg). SDF требует явного grid/ROI через существующий `sdf_tessellate`; для curve/profile пока нет line renderer.

При preview→full сохраняется исходная geometry revision и node ID. Renderer восстанавливает выделение native face по face ID только при совпадении kind/node/revision и placement. Старый triangle index и barycentric не переиспользуются: для face overlay вычисляется новая опорная точка в центре первого треугольника выбранной грани. Это сохранение выбранной грани, не точного места предыдущего клика. После изменения geometry/trim или при отсутствии native face correspondence восстановление отклоняется; App может сохранить выбор объекта, но не старую грань.

Явные Boolean/thicken результаты остаются mesh authority. Им не приписываются NURBS face IDs от произвольного входа. Explicit tessellation сохраняет исходную surface/B-rep/cage/field/patch definition. Совместимый узел `subdivision` продолжает выдавать mesh для старых операций, но в display snapshot сохраняет исходный cage.

Снимок переносится через Worker, Scene adapters и App publication. Геометрия открытой поверхности не имеет enclosed volume: viewer возвращает нулевой sentinel и явное предупреждение, вместо отрицательного signed-volume интеграла, нарушающего контракт публикации.

## Проверки

- Native surface → preview/full: разное число треугольников, одинаковая geometry revision, исходные f64 координаты сохранены.
- Native B-rep box: исходные шесть face IDs, корректный объём.
- Изменение trim меняет revision; изменение плотности явной tessellation — нет.
- Scene roundtrip, structured clone и Worker validation сохраняют/проверяют snapshot; подмена payload отклоняется.
- Renderer publication восстанавливает native face и отклоняет изменившуюся геометрию (тест с GPU mock; не live WebGPU проверка).

## Не завершено

Это не весь P0. Нет persistent naming после split/merge, полного source-span mapping из compact syntax, выбора native edges/control points, scene-native объекта без render mesh, durable snapshot repository и native slicing. Пустой mesh result пока не имеет MeshData-носителя snapshot; исходник остаётся в документе. Нельзя применять старые face IDs к изменившейся topology. Optional metadata имеет свою версию 1; интегрированные producer/consumer обновлены вместе, старые strict consumers могут отклонять новый optional field. Frozen qualification v1–v15 сохранены, v16 только перепривязывает evidence и включает новый source dependency и не заявляет прохождение CAD qualification.
