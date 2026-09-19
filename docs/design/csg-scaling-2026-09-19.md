# Масштабирование CSG-ядра: замеры и правки 2026-09-19

Продолжение: [исправление триангуляции профиля](profile-triangulation-2026-09-19.md)
устраняет отказ 4×4 и добавляет проверенную поддержку 64/100 отверстий.
Ниже сохранён исторический снимок до этой правки. Текущий CSG-бенч считает
настоящую медиану семи samples после двух прогревов; старый снимок использовал
минимум из двух samples, ошибочно названный медианой. Новые сравнения сделаны
заново на одинаковых исходниках, а не с числами старого снимка.

Контекст: [optimization-audit-2026-09-19.md](optimization-audit-2026-09-19.md),
раздел 5. Реестр: [recommendation.md](../recommendation.md), «Native P0 — CSG
kernel scaling». Все замеры: один Mac на M-серии, Node 20, own-Rust ядро;
`bench:csg` идёт через production-парсер и WASM (полное качество, медиана
из двух прогонов после прогрева), rbench нативно в release. Одиночный прогон
показывает порядок величин, не гарантию.

## 1. Что было на HEAD `fdf7b1a`

`node scripts/bench-cpu.mjs --quick` падал на второй же фикстуре
(`medium-csg`: куб минус 64 цилиндра при `$fn=32`) и на `dense-sphere`
(union трёх сфер при `$fn=256`) с одним и тем же ответом ядра: «Boolean
work/fragment/output budget exceeded». Лестница `bench:csg` на HEAD:

| Сценарий | HEAD | Причина в коде |
| --- | --- | --- |
| Плита минус N цилиндров, `$fn=32`: N = 16 / 25 / 36 | 90 / 165 / 284 мс | суперлинейно: один BSP-резак из N цилиндров режет все грани плиты бесконечными плоскостями |
| N = 49 / 64 | ошибка бюджета (stitch) | вывод BSP превышает `max_output_triangles = 20 000` до упрощения |
| Union 3 разнесённых сфер, `$fn=32` | 48 мс | |
| `$fn=64` и выше | ошибка | `if input_triangles > 10_000 { Err }` стоял до быстрых путей (`separated`/`contains`), хотя результат для разнесённых тел это просто `join` |
| Union 2 пересекающихся сфер, `$fn=32` | 68 мс | |
| `$fn=48` и выше | ошибка «Build A/B: work budget exceeded during BSP construction» | выбор плоскости в `Bsp::build` классифицирует весь набор до 4 раз на узел; на кривых телах глубина ~n, итого O(n²) |
| Просверленная плита через `prism_boolean` | не срабатывал | требовал одинаковый z-диапазон детали и резака; в OpenSCAD резак всегда длиннее, чтобы не было z-fighting |

## 2. Что изменено

1. `polygon-core::solid::boolean::boolean`: предел `BSP_INPUT_TRIANGLES`
   (10 000) применяется только к пути BSP. Разнесённые, вложенные,
   идентичные и пустые операнды выше предела получают точный результат из
   целых входных поверхностей; допусковые аудиты самопересечений для них не
   запускаются, и отчёт честно говорит `self_intersection_status:
   "not_checked"`. Ниже предела поведение не изменилось (аудиты как раньше,
   `checked_with_tolerance`). `max_output_triangles` теперь ограничивает
   именно вывод отсечения; тест, который пинил его на идентичных операндах,
   переписан на пересекающиеся.
2. `union_many` / `difference_many` (там же): n-арные операции сворачиваются
   по связности AABB (union-find с допуском, относительным к габариту);
   разнесённые группы `join`-ятся без CSG, резаки вычитаются батчами не более
   чем из 8 взаимно разнесённых тел (`DIFFERENCE_BATCH` в
   `geometry-bridge/src/mesh.rs`). Мост и `cad_boolean.rs` используют их.
3. Парсер (`openscadParser.ts::differenceChildren`): каждый дочерний
   statement `difference()` передаётся ядру отдельным операндом (loop/module
   как union своих фигур), а не сначала union всех резаков. Это байт в байт
   совпадает с SemanticProgram-lane (проверено архивом
   `manifold-plan-oracle-v1.json`). Политика смешанных размерностей вынесена
   в `homogeneousShapes` и общая для union/intersection/difference.
4. `prism_boolean`: difference принимает вертикальный резак, охватывающий
   z-диапазон детали; intersection берёт пересечение диапазонов; union
   по-прежнему требует совпадения. Мост считает prism-путь ускорением: его
   отказ (триангуляция профиля) возвращает шаг в общий булеан, а не наружу.
5. Нормали с crease-разбиением: `geometry-bridge/src/mesh.rs::render_buffers`
   и `abi_render_mesh`; TS-реализация в `services/geometry/module.ts`
   удалена. `Math.hypot`/`Math.round` воспроизведены побитово.
6. `crates/Cargo.toml`: восстановлены `mechanical-core`, `languages-bridge`,
   `languages-wasm` в `workspace.members` (регрессия `fdf7b1a`), `Cargo.lock`
   вернул две записи.
7. Бенчи: `node --import tsx benchmarks/own-cad/bench-csg-scaling.mts` (без npm-скрипта: `package.json` заморожен G0-fingerprint-ом;
   проверяет аналитический объём), `cargo run --release -p polygon-core
   --example bench_boolean`, `bench:cpu` больше не теряет остальные фикстуры,
   если ядро отказало одной (отчёт `status: partial`, код выхода 1).

## 3. После

### 3.1 `bench:csg` (WASM, production-парсер)

| Сценарий | HEAD | Эта ветка | Примечание |
| --- | --- | --- | --- |
| Плита − 1 / 4 цилиндра | 111* / 25 мс | 1.2 / 3.1 мс | prism-путь; * холодный старт |
| Плита − 16 | 90 мс | 98 мс | триангуляция профиля 4×4 отказала → BSP-fallback батчами по 8 |
| Плита − 36 | 284 мс | 73 мс | prism-путь, объём совпадает с аналитическим до 1e-16 |
| Плита − 64 / 100 | ошибка | ошибка | профиль > 2 048 вершин → fallback → рост фрагментов → stitch-бюджет |
| Плита − 16 сферических карманов, `$fn=16` | не измерялось | 110 мс | BSP батчами по 8 |
| Плита − 64 карманов | не измерялось | ошибка (stitch) | см. раздел 4 |
| Union 3 разнесённых сфер `$fn=64` / `128` | ошибка | 17 / 72 мс | 11.9k / 48.4k треугольников |
| `$fn=192` | ошибка | ошибка «Mesh exceeds the polygon resource budget» | 3 × 36k > `MAX_MESH_TRIANGLES = 100 000`, документированный предел |
| Union 2 пересекающихся сфер `$fn=32` | 68 мс | 60 мс | |
| `$fn=48+` | ошибка | ошибка | BSP на кривых телах, без изменений |

### 3.2 `bench:cpu --quick` (медианы, 3 выборки)

| Фикстура | Стадия | HEAD | Эта ветка |
| --- | --- | --- | --- |
| `small-bracket` (плита, 4 отверстия) | build | 15.2 мс | 2.6 мс |
| | evaluate | 11.6 мс | 1.6 мс |
| | analyze | 3.3 мс | 0.8 мс |
| `many-bodies` (256 кубов, без булеанов) | build | 23.2 мс | 17.3 мс |
| | analyze | 16.5 мс | 11.1 мс |
| `medium-csg` (64 отверстия) | | отказ | отказ (те же причины, что плита − 64) |
| `dense-sphere` (3 сферы `$fn=256`) | | отказ | отказ (100k-предел на меш) |

`analyze` включает нормали, BVH, рёбра и хэш; выигрыш в нём даёт перенос
нормалей в ядро и отказ от трёх лишних копий массивов. Реплеи `bvh`, `edges`,
`hash`, `validation`, `stl` не изменились (они уже были в Rust).

### 3.3 rbench, нативно, release (`--profile quick`, медиана batch)

| Случай | Время |
| --- | --- |
| `difference_many/plate-36-holes/batch-8` | 22.1 мс |
| `difference_many/plate-36-holes/batch-36` (одна планарная операция) | 8.1 мс |
| `union_many/36-separated-cylinders` | 0.69 мс |
| `union_many/3-separated-spheres-above-bsp-cap` (49k треугольников) | 8.7 мс |
| `boolean/separated-spheres-pairwise-fast-path` (2 × 16k) | 40.2 мс |
| `boolean/overlapping-boxes-bsp` | 0.15 мс |

Попарный быстрый путь дороже `union_many`, потому что всё ещё делает
`normalized()` + `validate_solid()` на обоих входах (BTreeMap на вершину и на
ребро): это пункт 1 из списка Rust-агента (`Mesh::edges()` через сортировку и
переиспользование `inspect`).

## 4. Что осталось и почему

1. **Триангуляция многоугольника с отверстиями** (`planar-geometry/src/triangulation.rs`):
   ear clipping с эвристикой мостов и бюджетом 2 048 вершин. Воспроизведение:
   тест `prism_difference_grid_of_holes_reports_triangulation_limits` в
   `polygon-core/src/solid/primitives.rs` (сетка 4×4: «cannot be triangulated
   without crossing its boundary»; 64 × 32 сегмента: «budget exceeded»). От
   неё зависят prism-путь, `extrude_rings` (крышки `linear_extrude`) и
   `simplify` (копланарная ретриангуляция после BSP). Замена на монотонное
   разбиение sweep-line (O(n log n), без сдвига координат, `orient2d` из
   `cad-predicates` для предикатов) снимает первые две строки отказов в
   таблице 3.1 и делает последовательный BSP линейным.
2. **Накопление фрагментов в последовательном BSP**: плоскости бесконечны,
   каждый шаг заново режет всю крышку; без работающего `simplify` рост
   квадратичный (14 700 треугольников после 14 шагов по одному квадратному
   резаку на плите 8×8, тест `difference_many_subtracts_separated_cutters_in_batches`).
   `DIFFERENCE_BATCH = 8` смягчает; данных для выбора 16/32 на BSP-пути нет,
   на prism-пути один большой батч быстрее (таблица 3.3).
3. **BSP-построение на кривых телах O(n²)**: не лечится бюджетами. Решение
   уровня ADR: arrangement-based булеан на точных предикатах (`cad-predicates`
   уже есть) либо зависимость от `manifold-rust` 0.13 (pure-Rust порт
   Manifold 3.5, Apache-2.0, однопоточный в WASM). Оба варианта совместимы с
   ADR 0010 только через отдельное решение о провенансе.
4. **`MAX_MESH_TRIANGLES = 100 000`**: предел на один меш; `dense-sphere`
   при `$fn=256` его превышает по определению. Либо фикстура пересматривается,
   либо предел растёт вместе с транспортным лимитом 32 MiB.

## 5. Состояние тестов и заморозок

`cargo test -p polygon-core -p geometry-bridge`: все цели зелёные (246 + 84
unit, интеграционные `tests/*` моста тоже). Полный `vitest run` на этой ветке:
3081 passed, 33 failed; разбор падений:

| Группа | Причина | Чьё |
| --- | --- | --- |
| `engineManifest` (`kernelFingerprint`), `g0ToolchainFingerprints` (WASM-отпечаток), `qualificationPlanArtifact` (v27: `g1-pinned-legacy-support-bundle` теперь тоже отличается, потому что изменён `openscadParser.ts`) | ядро и парсер изменились по существу; заморозки G0/G1 и манифест `own-rust-node-v2` требуют новой версии по существующей процедуре (`scripts/release-own-rust-v*.mjs`, re-freeze генератор). Это квалификационное событие, которое должен провести владелец, здесь не имитировалось | эта ветка |
| `g0ToolchainFingerprints` «package.json: expected 2822» | `package.json` на HEAD `fdf7b1a` уже 2929 байт: отпечаток устарел до этой ветки | HEAD |
| `mcpServer`, `modelGraphNurbsMcp` (лишний инструмент `modelgraph_nurbs_intersect`) | инструмент зарегистрирован коммитом `ddce85c`, тесты не обновлены | HEAD |
| `brepAnalytic` (torus ∪ box больше не бросает), `brepNativeProgramSemantics` (sphere ∩ sphere выполняется) | `brep-core` не зависит от `polygon-core` и не тронут; ядро в основном checkout собрано в 16:48, до merge `5734538` (16:54) с новыми сферическими булеанами, так что ожидания тестов отстали от HEAD | HEAD |
| `brepQualificationV5…V12` («Cannot find package ajv/dist/2020.js»), `wasmBrotliPacking` (нет `photogrammetry_wasm.wasm` в target этого worktree) | окружение worktree (node_modules и target из основного checkout) | среда |
| 20 падений `modelGraph*`, `mcpDirectGeometrySupervisor`, `brepDiagnosticExecutor` | contention одноразовых worker'ов при параллельных файлах; при `--no-file-parallelism` проходят | среда |

## 6. Воспроизведение

```sh
node --import tsx benchmarks/own-cad/bench-csg-scaling.mts --out output/csg-scaling-<date>.json
```

```sh
cargo run --release --manifest-path crates/Cargo.toml -p polygon-core --example bench_boolean -- --profile quick
```

```sh
node scripts/bench-cpu.mjs --quick
```

```sh
cargo test --locked --manifest-path crates/Cargo.toml -p polygon-core -p geometry-bridge
```

Результат `bench:csg` этой ветки: [output/csg-scaling-2026-09-19.json](../../output/csg-scaling-2026-09-19.json).
