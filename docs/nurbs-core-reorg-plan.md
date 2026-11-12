# План реорганизации `nurbs-core`

Актуализировано 2026-11-13: удалены 23 завершённых пункта (этапы 0–8 вместе с целевым деревом модулей, DRY #2, #3, #8, #9, #10, сплиты intersection, curve, robust_solvers, progressive_miter, surface_offset, foundation-хаб, progressive_sweep, primitives, конструкторы surface). Ниже остались только открытые работы.

Состояние на 2026-11-13. Перемещения по тематическим каталогам выполнены (`core`, `analysis`, `intersection`, `offset` с `offset/chord/`, `sweeps` с `sweeps/audit/`, `construct`, `editing`, `join`, `trim`, `tessellation`). Сверх исходного плана часть слоёв вынесена в отдельные крейты (O3 в `docs/refactor/`): `nurbs-geometry` (curve, surface, bounds, affine, …), `nurbs-numerics` (compensated, sturm, robust_solvers, interval_vec3, …), `nurbs-foundation` (foundation, distance, certificates). `nurbs-core` держит прежние пути через `pub use` (`src/core.rs`, `src/numerics.rs`, `src/lib.rs`). Прежние цифры (146 модулей, 246 файлов, ~76k строк, 790 тестов) устарели: сейчас в `crates/nurbs-core/src` около 1086 `.rs`-файлов и ~275k строк.

## 1. Неподключённые копии в корне `src/`

В корне `crates/nurbs-core/src` остались отслеживаемые git файлы, которые по grep не подключены ни через `mod`, ни через `#[path]`; их актуальные версии лежат в новых каталогах: `affine.rs`, `chord_arrangement.rs`, `chord_faces.rs`, `curve_jets.rs`, `curve_measure.rs`, `curve_offset.rs`, `curve_offset_diagnostics.rs`, `gordon*.rs`, `guided_loft.rs`, `hermite.rs`, `loft_alignment.rs`, `loft_continuity.rs`, `natural_loft.rs`, `paths.rs` и `paths/`, `primitives.rs`, `surface_measure.rs` и `surface_measure/`, а также каталоги `surface_offset/` и `ss_intersection/`. Содержимое расходится с перенесёнными версиями (например, `gordon.rs` и `construct/surfaces/gordon.rs`). Перед удалением подтвердить сборкой, что они действительно не компилируются.

## 2. DRY-аудит (открытые находки)

| # | Что дублируется | Файлы / строки | Предложение |
|---|---|---|---|
| 1 | Локальные векторные хелперы `unit` для `[f64;3]`. `biarc.rs` уже переведён на `math_core::{…}`, общий хелпер с сообщением есть: `nurbs_geometry::vec3_ext::unit` | `sweeps/progressive_miter.rs:31`, `sweeps/framed_sweep.rs:11`, `construct/curves/paths/miter_sections.rs:4`, `construct/surfaces/transitions/circle.rs:10` (свои `unit`) | Заменить на `math_core::{…}` или `vec3_ext::unit` там, где семантика совпадает. Низкий риск |
| 4 | Компенсированное суммирование: собственный `CompensatedSum` (Neumaier) при наличии EFT-набора в `compensated` (two_sum, dot_k) | `crates/nurbs-geometry/src/curve.rs:65` vs `nurbs_numerics::compensated` | НЕ трогать горячий путь `evaluate` сейчас; в перспективе — опциональный отдельный коммит: `CompensatedSum` выражается через `compensated::two_sum`, с бенчем. Пометить как «low priority, verify no perf regression» |
| 5 | Однородный интервальный де Бур | `distance/curve_distance.rs`, `distance/surface_distance.rs`, `interval_eval.rs`, `interval_newton.rs` (все в `nurbs-foundation`), `analysis/normal_cone.rs` (интервальный треугольник Кокса–де Бура) | Общий homogeneous-interval ядерный код (span locate + blossom/de Boor) с 1D- и 2D-адаптерами. В `nurbs-foundation/src/distance/homogeneous_net.rs` уже собраны однородные Bernstein-сети поверхностей, но единого ядра для кривых и поверхностей не найдено: пункт считается открытым. Самая рискованная дедупликация, отдельным этапом |
| 6 | Обобщённый Cox–de Boor по скаляру | `dual.rs` (generic lift), `curve.rs` (`basis`, `basis_funs_ders`), `foundation/interpolation.rs` | НЕ объединять: `curve.rs` — горячий путь с CompensatedSum и ошибочными статусами; generic-вариант dual.rs оставить для AD. Зафиксировать в doc-комментариях взаимные ссылки |
| 7 | Скелет «аудит-отчёта»: `Report { …, unresolved: Vec<…> }` + `tolerance_evidence` + serialization | `sweeps/audit/{pair,wall,contour,seam}.rs`, `trim/trim_region_audit.rs`, `join/periodic_seam_verification.rs`, `join/periodic_surface_verification.rs`, `analysis/curve_regularity.rs`, `analysis/surface_injectivity.rs` | Группировка в `sweeps/audit/` и `trim/` выполнена, общих типов `audit::Unresolved { subject, reason }` и trait сериализации пока нет. Слияние логики НЕ делать (контракты у каждого свои, это осознанно) |
| 11 | Проверенное НЕ-дублирование | `robust_solvers.rs` явно документирует, что LM-демпинг в `foundation/fitting.rs` — локальный, а point inversion живёт в `foundation::point_inversion_solver`; `interval_eval.rs` — `pub use distance_bounds::Interval` (тип Interval един) | Оставить как есть; зафиксировано, чтобы не «чинить» |

## 3. Что ещё РАЗЪЕДИНИТЬ

| Файл | Строк | Проблема | Действие |
|---|---|---|---|
| `nurbs-foundation/src/distance/convex_distance.rs` | 776 | GJK + EPA + OBB SAT + control-cloud адаптеры | `broadphase/{gjk_epa,obb_sat,clouds}.rs` — низкий приоритет |
| `analysis/normal_cone.rs` | 756 | Конус + силуэт + кривизна + offset-гладкость + draft | Опционально 2 файла (cone / applications) |
| `transport/` | — | JSON-диспетчер | НЕ ТРОГАТЬ (см. риски) |

## 4. Оставшиеся этапы

Каждый этап — отдельный коммит, после каждого: `cargo test -p nurbs-core` + `cargo check -p brep-core --all-features`.

- **Этап 9. DRY-извлечения** (каждое — свой коммит, в порядке роста риска): #1 оставшиеся локальные `unit` → #7 audit-скелет → #5 общий interval de Boor → (опционально) #4 CompensatedSum через two_sum с бенчмарком.
- **Этап 10. Оставшиеся сплиты** (таблица §3): convex_distance.rs, затем опционально normal_cone.rs.
- **Этап 11. Финализация API.** Объявить каноническими новые пути в документации и `domains.rs` (сейчас `domains.rs` реэкспортирует плоские пути вида `crate::curve`); старые шимы пометить `#[deprecated]` (сейчас ни одного) или убрать на мажорном бампе. `README.md` крейта обновить (он включается через `#![doc = include_str!]`). Потребителей перевести на новые пути отдельным PR: `brep-core` по-прежнему импортирует плоские пути (`nurbs_core::curve::` около 170 раз, `nurbs_core::curve_surface_agreement::`, `nurbs_core::thread::`).

## 5. Риски и «не трогать»

1. **`transport/` (feature `transport`)** — внешний JSON-контракт; имена команд/полей не менять, файлы не двигать до отдельного решения.
2. **Горячие пути**: `Curve::evaluate`, `basis_funs_ders`, `find_span` (`nurbs-geometry`) и интервальный de Boor в curve_distance/surface_distance — переносить байт-в-байт, без «заодно поправить». DRY #4/#5 — только после бенчмарка.
3. **`serialization.rs` подфайлы (feature `codec`)** — wire-форматы value-codec не меняются; файлы двигаются только вместе с родителем, `#[path]` обновляется аккуратно (паттерн `#[path = "x/serialization.rs"]` ломается при переносе родителя).
4. **`foundation/`** — сертификационный код с outward-rounding инвариантами (`next_down/next_up`); логику не рефакторить.
5. **Модули с осознанно «честными» контрактами** (audit/verification — каждый документирует, что он НЕ доказывает): не объединять семантику, только группировать.
6. **brep-core и другие потребители** импортируют плоские пути — шимы сохраняют компиляцию; перевод потребителей только после этапа 11.

## 6. Заметки по соседним крейтам

- **brep-core**: `parts/rack.rs` и `parts/thread.rs` (+ `thread_verified.rs`, `thread_internal.rs`) — топологические обёртки над `nurbs_core::rack` / `nurbs_core::thread` (профили/патчи в nurbs-core, тела/швы в brep-core).
- **math-core**: экспортирует `cross/dot/norm/sub/unit/eigen/point_moments`; оставшиеся локальные дубли перечислены в DRY #1. Никакой геометрии NURBS в math-core нет.
- **geometry-ops**: уровень mesh/polygon-операций, пересечений с NURBS-ядром не видно.
- Отдельного аудита brep-core не делалось; если появится план реорганизации brep-core, первым кандидатом проверить `uv_arrangement/uv_regions` против `nurbs_core::trim_domain` и `offset::chord` — по названиям возможна тематическая близость.
