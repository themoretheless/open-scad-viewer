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

## Риск
- Рост binary size / давление на I-cache при чрезмерном inlining крупных функций — не трогали `eigen`/`svd`.
- Не размазывать `#[inline(always)]` по polygon/boolean без профиля.

## Предлагаемый формат
- URI: `docs://rag-inline-hotpath-optimization`
- Wing: `open-scad-viewer`
- Тип: оптимизация компиляции Rust; статус **`observed`**
