# Десятиагентный adversarial review идей

Срез: **2026-07-21**. Цель этого прохода — не увеличить backlog, а проверить,
какие идеи можно честно строить поверх текущих Worker, provenance, BVH,
topology и scene-publication контрактов.

## Метод

Десять независимых ролей прошли четыре волны:

1. product/UX ideation;
2. geometry/scientific ideation;
3. architecture/performance ideation;
4. product critic;
5. mathematical critic;
6. principal-engineer critic;
7. portfolio synthesis;
8. executable-contract design;
9. implementation/test planning;
10. final adversarial red-team.

Критики могли отклонить идею, сузить обещание или потребовать prerequisite.
Количество голосов не использовалось. Решение принималось по пользовательскому
результату, доказуемости, использованию существующих primitives, риску ложной
гарантии, effort и архитектурной готовности.

## Итоговая scorecard

Шкала 1–5. Для **Risk** и **Effort** больший балл хуже.

| Кандидат | Impact | Evidence | Leverage | Risk | Effort | Ready | Решение |
|---|---:|---:|---:|---:|---:|---:|---|
| Statement-form `assert()` | 5 | 5 | 5 | 1 | 1 | 5 | **Build now** |
| Startup-only Worker retry | 3 | 4 | 4 | 2 | 2 | 5 | Pilot |
| Per-entity volume + centroid | 4 | 5 | 5 | 2 | 2 | 4 | Pilot; axes defer |
| 2D polygon debugger | 3 | 3 | 4 | 3 | 3 | 2 | After immutable IR |
| Preview/full deviation interval | 4 | 4 | 5 | 5 | 4 | 2 | Research pilot |
| Goal Seek | 4 | 3 | 4 | 4 | 4 | 2 | Defer |
| Inspection Snapshots | 3 | 4 | 3 | 3 | 3 | 2 | Defer |
| Frame-budget SceneIngestor | 2 | 3 | 5 | 4 | 3 | 1 | Defer |
| GPU readiness barrier | 3 | 4 | 4 | 4 | 3 | 2 | Defer |
| Global wall-thickness heatmap | 4 | 3 | 2 | 5 | 5 | 1 | **Kill as stated** |

## Что выдержало критику

### 1. Исполняемые контракты модели

Первый slice — OpenSCAD statement-форма в рамках строгого subset:

```scad
assert(width > 0, "width must be positive") cube(width);
```

Она полезна уже сейчас для guards в параметрических модулях и не требует
нового Worker protocol. Условие использует OpenSCAD truthiness; passing assert
прозрачно пропускает children, failed assert останавливает сборку с source
line/column. Expression-form `assert(condition) value` требует отдельной
грамматики и остаётся unsupported. Официальная опора:
[OpenSCAD assertions](https://en.wikibooks.org/wiki/OpenSCAD_User_Manual/The_OpenSCAD_Language#assert).

Worker публикует не больше одного terminal на job: обычный провал assertion
даёт positioned `failed`, но доставленная отмена или более новая revision имеют
приоритет и дают `cancelled`/`stale`. Preview и full остаются разными jobs и
потому могут независимо диагностировать один и тот же guard.

Следующий слой — full-only post-build contracts. Предпочтительный строгий
формат — одна versioned JSON-аннотация в top-level line comment:

```scad
// @contract/v1 {"id":"volume","metric":"volume","expected":[995,1005],"tolerance":0.01,"quality":"full"}
```

MVP-метрики: aggregate volume, connected-body count и world bounds. Preview
никогда не получает `pass`; stale/cancelled builds не публикуют report;
non-finite или неоднозначная metric даёт `error`, а не guessed result. JSON и
JUnit — только производные от одного typed report. Headless CI boundary должен
использовать тот же full evaluator без WebGPU.

### 2. Per-entity volume и centroid

Это полезный analysis artifact для contracts, interference, tolerance и
будущего Goal Seek. Первый slice ограничен Manifold-produced closed solids,
Float64 compensated sums и явным local/world frame. Principal axes нельзя
обещать без eigengap: для куба или сферы они математически неоднозначны.

Алгоритмическая опора: Brian Mirtich,
[Fast and Accurate Computation of Polyhedral Mass Properties](https://doi.org/10.1080/10867651.1996.10487458).

### 3. Bounded preview/full deviation

Случайные или area-stratified samples не образуют доказанное `h`-покрытие:
маленький высокий spike между samples создаёт ложный pass. Честный алгоритм
возвращает interval `[lower, upper]`, дробит triangle cells с наибольшим upper
bound и при исчерпании бюджета выдаёт `inconclusive`.

Для cell sample `c` с covering radius `r`:

```text
lower = distance(c, target)
upper = lower + r + numericSlack
```

Нужны world-space nearest-point BVH, adaptive subdivision и отдельная обработка
unmatched entity. Гарантия относится только к двум опубликованным triangle
soups, не к аналитической CSG-поверхности. Официальная опора:
[CGAL bounded-error Hausdorff distance](https://doc.cgal.org/latest/Polygon_mesh_processing/group__PMP__distance__grp.html).

### 4. Startup-only Worker retry

Допустима ровно одна повторная попытка, только если Worker умер до первого
`accepted` текущей generation. Parse, assertion, resource-limit, kernel и
protocol failures никогда не повторяются. Перед repost обязательно повторно
проверяется актуальная document revision. Это отдельный reliability pilot, а
не замена cooperative cancellation.

## Идеи, которые были сужены или отложены

### Goal Seek

Идея остаётся сильной: подобрать одну Customizer-переменную под target volume
или AABB за ограниченное число probes. Но slider range не доказывает
монотонность, preview может разойтись с full, а сохранённые world-space точки
не являются пересчитываемой metric. Поэтому prerequisite-порядок такой:

1. typed full metric contract;
2. bounded cancellable ProbeRunner с отдельными ephemeral revisions;
3. sampled bracket/no-bracket semantics;
4. full verification до изменения source literal;
5. только затем Goal Seek UI.

Официальные ориентиры:
[OpenSCAD Customizer](https://en.wikibooks.org/wiki/OpenSCAD_User_Manual/Customizer),
[SOLIDWORKS Design Study](https://help.solidworks.com/2024/english/Solidworks/cworks/c_Using_Optimization_Module.htm).

### Inspection Snapshots

Версия MVP должна называться snapshot, а не PMI/bookmark. Только exact-source
snapshot может восстанавливать camera, visibility, section и geometry anchors.
После изменения source разрешены camera + note; triangle/measurement anchors
становятся `stale` без автоматической перепривязки. Реализация ждёт единого
viewport-state owner и versioned workspace storage.

### 2D Profile Debugger

Безопасный pilot анализирует только выбранный raw `polygon()` leaf, сравнивает
его с фактическим normalized `CrossSection.toPolygons()` и связывает finding со
всем call span. Winding при EvenOdd — информация, а не ошибка. Per-edge source
claim и auto-repair запрещены. Полный workflow ждёт immutable 2D operation IR.

### SceneIngestor и GPU readiness barrier

`setMeshes()` уже выполняет частичную GPU-транзакцию и меняет текущую сцену
после успешного создания ресурсов. До нового ingestor нужны измерения
`validationMs/uploadMs/maxTaskMs`, Worker-computed bounds/byte counts и один
владелец scene assets. До readiness barrier нужен выделенный device/surface
manager; `queue.onSubmittedWorkDone()` не доказывает корректные pixels и не
должен становиться глобальным stall после каждого изменения.

## Отклонённое обещание: «карта минимальной толщины»

Девять лучей в cone вокруг inward normal дают Shape Diameter Function-подобный
descriptor, но не minimum wall thickness. Для slab наклонный ray возвращает
`t / cos(angle)`, в U-channel согласованно попадает в далёкую стенку, а на
sphere измеряет chord. Consensus не является error bound.

Такой инструмент можно вернуть только как явно experimental local chord probe
с raw ray lengths, angular coverage и `unknown`; manufacturing pass/fail и
глобальная heatmap отклонены. Научная опора исходного descriptor:
[Shapira, Shamir, Cohen-Or — Shape Diameter Function](https://doi.org/10.1007/s00371-007-0197-5).

## Принятый порядок

1. Statement-form `assert()` и OpenSCAD-compatible truthiness.
2. Typed full-build metric-contract schema и report boundary.
3. Entity-keyed volume + centroid artifact.
4. Startup-only Worker retry как независимый bounded hardening slice.
5. Headless JSON/JUnit reporting.
6. Cancellable ProbeRunner.
7. Goal Seek для одной переменной и одной deterministic metric.
8. 2D profile pilot после compiler/IR seam.
9. Inspection Snapshots после viewport-state/storage seams.
10. Certified deviation interval только после world-space nearest-point BVH.

Этот порядок сохраняет code-first источник истины и переносит из Plasticity и
классических CAD систем не поверхность интерфейса, а проверяемые workflows:
guard, inspect, analyze, then change.
