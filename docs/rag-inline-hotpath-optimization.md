# Оптимизация hotpath: `#[inline(always)]` (RAG-заметка)

## Контекст
Критичные leaf-функции с высокой частотой вызова (hotpath), где метрика — overhead вызова и качество codegen для малых f64-ядер.

## Статус в RAG
- URI: `docs://rag-inline-hotpath-optimization`
- Wing: `open-scad-viewer`
- **`observed` (2026-09-10)** — локальный microbench `math-core` до/после.

## Что сделано
В `crates/math-core/src/lib.rs` для `dot`/`add`/`sub`/`scale`/`norm`/`unit`/`cross`/`mv`/`tr`/`mm`/`det`/`rotation`:
1. `#[inline(always)]`
2. явная индексация `[f64;3]` вместо iterator/`from_fn` в leaf (чтобы size-профиль `opt-level=s` не оставлял call+iter overhead)

Сопутствующие renderer-правки (аллокации, не FPS-сценарий):
- ghost fade → `styleScratch` вместо `new Float32Array` каждый кадр
- morph matrix → `objectUniformScratch` + `invert(m, out)`
- depth `createView()` кэшируется до resize

## Benchmark (Apple Darwin, `cargo test -p math-core --release`)
Workload: 2 000 000 итераций `cross/add/scale/unit/dot/mv/mm/rotation/det`, один и тот же `acc`.

| | ms (p≈) |
|---|---:|
| до (без inline, iterators) | **190.7** |
| после (3 повтора) | **1.19 / 1.24 / 1.27** |

Скорость ~**150×** на этом leaf-микробенче; `acc` совпал. Это не FPS полного приложения и не WASM-пакет целиком — только CPU leaf math-core в native release (`opt-level=s` workspace).

Воспроизведение:

```sh
cd crates && cargo test -p math-core --release --lib leaf_math_throughput -- --nocapture
```

## Раунд 2: inline + `unsafe` в hot-ядрах (observed, 2026-09-10, Windows x64)

Расширение на другие crate'ы; каждый шаг измерен rbench-микробенчем до/после (`--profile quick`, медиана `batch_total`, шум ~3%).

### `sdf-core` (`opt-level=s`) — mesh distance
`#[inline(always)]` на leaf-хелперах, `[[f64;3]]`-view позиций через `as_chunks` (одна проверка границ вместо трёх на вершину), явные массивы вместо `from_fn`, порядок FP-операций сохранён (bit-exact).

| case | до | после |
|---|---:|---:|
| `polygonize/mesh_unsigned` | 334.6 ms | **167.4 ms** (2.0×) |
| `polygonize/mesh_signed` | 460.6 ms | **287.3 ms** (1.6×) |
| `polygonize/csg` | 36.3 ms | 35.1 ms (время в marching-tetra/BTreeMap, не в поле) |

```sh
cd crates && cargo run --release -p sdf-core --example bench_sdf_cpu -- --profile quick
```

### `polygon-core` (`opt-level=3`) — BVH / semantic edges
Unchecked-доступ в приватном `Builder` (инвариант: `order` — перестановка `0..valid_count`, длины проверены `assert_eq!` перед конструированием), общий radix `counting_pass` с unchecked чтением bucket'ов (scatter-запись оставлена проверяемой).

| case | до | после |
|---|---:|---:|
| `edges/semantic` (100k tri) | 9.35–9.5 ms | **8.6–9.1 ms** (~5–8%) |
| `bvh/build` | ~31 ms | ~32 ms (без изменений: memory-latency bound, bounds checks уже элиминированы на O3) |
| `bvh/raycast` (256 rays) | ~0.54 ms | ~0.54 ms |

```sh
cd crates && cargo run --release -p polygon-core --example bench_mesh_kernels -- --profile quick
```

Вывод: на `opt-level=3` `unsafe` почти ничего не даёт — компилятор уже убирает проверки; выигрыш есть только там, где индекс зависит от данных (radix bucket).

### `geometry-bridge` ABI
`abi_import_mesh`: по-элементный `read_unaligned` заменён одним `copy_nonoverlapping` (`read_pod<T: Copy>`).

### `#[inline(always)]` на leaf-хелперах (без отдельного бенча)
`brep-core::intersections` (`point3/dot/sub/cross/distance`), `planar-geometry` (`lerp/dist`), `cad-predicates::arithmetic` (`next_up/next_down/up_add/up_mul/exponent/least_bit_exponent`) — те же критерии, что и в math-core: маленькие чистые f64-ядра под `opt-level=s`.

## Риск
- Рост binary size / давление на I-cache при чрезмерном inlining крупных функций — не трогали `eigen`/`svd`.
- Не размазывать `#[inline(always)]` по polygon/boolean без профиля.
- `unsafe` только за приватными типами с задокументированным инвариантом и `debug_assert!`; там, где корректность индекса зависит от внешних данных (host-валидация, `digit_of`), индексация оставлена проверяемой.

## Предлагаемый формат
- URI: `docs://rag-inline-hotpath-optimization`
- Wing: `open-scad-viewer`
- Тип: оптимизация компиляции Rust; статус **`observed`**
