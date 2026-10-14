# План реорганизации `nurbs-core`

Дата: анализ текущего HEAD. Крейт: 146 публичных модулей в `src/lib.rs` (205 строк), 246 `.rs`-файлов, ~76k строк, 790 тестов. Ноль `unsafe`, ошибки через `Result`, deps: math-core, geometry-ops, brep-topology, cad-predicates, value-codec.

## 1. Целевое дерево модулей

Ключевой принцип: **физическая иерархия меняется, логические пути нет** — на переходный период в `lib.rs` остаются `pub use` шимы для всех старых путей (`crate::curve_offset::…`). Уже существующий паттерн `file.rs + dir/` (foundation, continuity, intersection, ss_intersection, progressive_miter, progressive_sweep, curve_offset_diagnostics, trim_domain, paths, formula, helix, bounds, surface_measure, surface_distance, curve_distance, trimmed_surface_distance, distance_bounds, curve, surface) становится нормой: корневой файл категории превращается в `mod.rs`-хаб с реэкспортами.

Уже существующий `domains.rs` — прототип «view-слоя»; он сохраняется и обновляется на новые пути.

```
src/
├── lib.rs                     // pub use-шимы старых путей (этап перехода), domains, transport, tests
├── domains.rs                 // обновить на новые пути
├── transport.rs               // НЕ ТРОГАТЬ (JSON-граница)
├── tests.rs
│
├── core/                      // базовые типы и точечная семантика
│   ├── curve.rs               // Curve, Evaluation, Basis, Sampler (после сплита curve.rs)
│   ├── curve/basis.rs         // basis, basis_funs_ders, find_span(_hinted)
│   ├── curve/knot_ops.rs      // insert, refine, elevate, trim, split, reverse, merge_near_knots, clamped, normalize_knots
│   ├── curve/decompose.rs     // decompose → Segment
│   ├── surface.rs             // Surface, Axis, Evaluation; loft/extrude/revolve/sweep остаются здесь (см. §4)
│   ├── bounds.rs              // + bounds/serialization.rs
│   ├── affine.rs
│   ├── coordinate_frame.rs
│   ├── canonical.rs
│   └── conditioning.rs
│
├── numerics/                  // численный слой (кроме самого evaluate)
│   ├── interval.rs            // distance_bounds::Interval — единственный владелец типа
│   ├── interval_eval.rs       // публичные обёртки (сегодня реэкспортирует Interval)
│   ├── compensated.rs         // + сюда exact_products.rs (точный знак суммы произведений — та же тема EFT)
│   ├── dual.rs                // Dual/HyperDual
│   ├── sturm.rs
│   ├── interval_newton.rs
│   ├── robust_solvers.rs      // после сплита: rrqr.rs, trust_region.rs, lm.rs, inversion.rs, rank.rs
│   ├── bezier_extraction.rs
│   ├── normal_cone.rs
│   ├── periodic_chart.rs      // (private)
│   ├── curve_jets.rs          // (private)
│   ├── exact_curve_segments.rs// (private)
│   └── broadphase/
│       ├── convex_distance.rs // GJK/EPA/OBB SAT
│       └── obb_tree.rs        // уже зависит от convex_distance
│
├── analysis/                  // диффгеом и меры (read-only запросы)
│   ├── curve_differential.rs  ├── surface_differential.rs
│   ├── curve_measure.rs       ├── surface_measure.rs(+jets.rs)
│   ├── curve_extrema.rs       ├── curve_analysis.rs
│   ├── curve_deviation.rs     ├── curve_plane.rs
│   ├── curve_regularity.rs    ├── surface_regularity.rs
│   ├── surface_injectivity.rs ├── surface_monotonicity.rs
│   ├── surface_linear_monotonicity.rs
│   ├── surface_parameter_bounds.rs
│   ├── radial_bounds.rs
│   ├── surface_shape_operator.rs
│   ├── geodesic.rs            ├── unrolling.rs
│   ├── frechet.rs             ├── ray_surface.rs
│   ├── curve_surface_agreement.rs
│   ├── curve_pullback.rs      └── curve_surface_composition.rs (private)
│
├── distance/
│   ├── curve_distance.rs      ├── surface_distance.rs
│   ├── curve_surface_distance.rs
│   └── trimmed_surface_distance.rs
│   (Interval остаётся в numerics; obb_tree/convex_distance — numerics/broadphase)
│
├── intersection/
│   ├── mod.rs                 // сегодняшний intersection.rs (1773 строки — сплит, §4)
│   ├── plane.rs, curve_surface.rs, bezier_clip.rs, serialization.rs  // уже есть
│   ├── self_curve.rs          // curve_self_intersection.rs
│   ├── self_surface.rs        // surface_self_intersection.rs
│   └── ss/                    // сегодняшние ss_intersection.rs + ss_intersection/*
│
├── offset/                    // кривые/поверхности-оффсеты + chord-топология оффсетов
│   ├── curve_offset.rs        ├── curve_offset_join.rs
│   ├── curve_offset_wire.rs   ├── diagnostics/  (curve_offset_diagnostics)
│   ├── surface_offset.rs      // сплит: offset/validity/deviation (§4)
│   └── chord/
│       ├── arrangement.rs     ├── embedding.rs
│       ├── faces.rs           ├── intersection.rs
│       ├── winding.rs         ├── witness.rs
│       └── fill_selection.rs
│
├── sweeps/
│   ├── framed_sweep.rs        ├── profile_sweep.rs
│   ├── progressive_sweep.rs(+tests, serialization)
│   ├── progressive_miter.rs(+10 certificate-файлов — уже поддерево)
│   ├── scaled_sweep.rs        ├── twist_sweep.rs
│   ├── two_guide_sweep.rs     ├── helical_sweep.rs
│   ├── pipe.rs                ├── ribbon.rs
│   └── audit/
│       ├── pair.rs            // sweep_pair_audit (716 строк — кандидат на сплит)
│       ├── wall.rs            // sweep_wall_audit
│       ├── contour.rs         // sweep_contour_audit
│       ├── seam.rs            // sweep_seam_audit
│       ├── cap_boundary.rs    // sweep_cap_boundary
│       └── cap_wall.rs        // sweep_cap_wall
│
├── construct/
│   ├── curves/
│   │   ├── primitives.rs      // коники и поверхности вращения (1080 строк — сплит conics/revolution)
│   │   ├── engineering_profiles.rs
│   │   ├── helix.rs(+serialization)  ├── involute.rs
│   │   ├── archimedean_spiral.rs     ├── logarithmic_spiral.rs
│   │   ├── toroidal_spiral.rs        ├── spherical_spiral.rs
│   │   ├── lissajous.rs              ├── trochoid.rs
│   │   ├── circular_rolling.rs       ├── catenary.rs
│   │   ├── clothoid.rs               ├── biarc.rs
│   │   ├── hermite.rs                ├── natural_spline.rs
│   │   ├── closed_spline.rs          ├── curve_chain.rs
│   │   ├── curve_reconstruction.rs   ├── curve_extension.rs
│   │   ├── paths.rs(+miter_sections, round_polyline)
│   │   ├── formula.rs(+text, serialization)
│   │   ├── rack.rs                   └── thread.rs
│   └── surfaces/
│       ├── function_surface.rs       ├── boundary_fill.rs
│       ├── coons.rs                  ├── gordon.rs
│       ├── patches.rs                ├── triangular_patch.rs
│       ├── hermite_patch.rs          ├── extrusion_patches.rs
│       ├── polynomial.rs             ├── grid_spline.rs
│       ├── catenoid.rs               ├── helicoid.rs
│       ├── screw_surface.rs
│       ├── loft/
│       │   ├── natural_loft.rs       ├── guided_loft.rs
│       │   ├── alignment.rs          └── continuity.rs  (loft_continuity)
│       ├── sections/
│       │   ├── sections.rs           ├── section_projection.rs
│       │   ├── section_circle_repair.rs └── section_phase.rs
│       └── transitions/
│           ├── circle.rs             // circle_transition
│           ├── ellipse.rs            // ellipse_transition
│           └── circle_rectangle.rs   // circle_rectangle_transition
│
├── editing/
│   ├── edit.rs                ├── weight_edit.rs
│   ├── surface_edit.rs        ├── direct_edit.rs
│   ├── morph.rs               └── fairing.rs
│
├── join/                      // непрерывность и сшивка
│   ├── continuity.rs(+поддерево: preparation, bounds, regularity, exact_strip, curve_match, deviation, report)
│   ├── curve_join.rs          ├── surface_join.rs
│   ├── surface_g1.rs          ├── surface_g2.rs
│   ├── periodic_seam.rs
│   ├── periodic_seam_verification.rs
│   └── periodic_surface_verification.rs
│
├── trim/
│   ├── trim_domain.rs         ├── trim_point.rs
│   ├── trim_region_audit.rs   └── trim_simplicity.rs
│
├── tessellation/
│   ├── curve_tessellation.rs  ├── surface_tessellation.rs
│   └── shared_tessellation.rs
│
├── certificates/
│   └── curve_decomposition_certificate.rs
│
└── foundation/                // поддерево уже есть; foundation.rs → mod.rs-хаб (§4)
```

## 2. DRY-аудит (конкретные находки)

| # | Что дублируется | Файлы / строки | Предложение |
|---|---|---|---|
| 1 | Векторные хелперы `sub/dot/cross/norm/unit` для `[f64;3]` при том, что math-core экспортирует `cross, dot, norm, sub, unit` | `biarc.rs:53–69` (свои sub/dot/cross/norm); `progressive_miter.rs:15–25` (norm/unit); `paths/miter_sections.rs:4–7` (norm/unit); `framed_sweep.rs:10` (unit); `progressive_sweep.rs:86,1105` (unit/norm); `circle_transition.rs:10–20` (unit) | Заменить на `math_core::{…}` там, где семантика совпадает; для `unit` с `check(...)`-обёрткой — один crate-private `util::vec3` хелпер. Чистый выигрыш, низкий риск |
| 2 | То же для интервальных скаляров (`I`/`Interval`): `sub/dot/cross/norm/unit` на интервалах | `progressive_miter/frame_certificate.rs:528–551`, `progressive_miter/authored_frame_certificate.rs:120–132`, `surface_differential.rs:34–45`, `curve_differential.rs:44–48`, `surface_measure.rs:56–65`, `surface_shape_operator.rs:22` | Общий `numerics/interval_vec3.rs` (generic over outward-rounded scalar trait либо конкретно над `distance_bounds::Interval`). Средний риск: семантика Result-ошибок в каждом файле своя — выносить с параметром сообщения |
| 3 | 2D-интервальный `cross` | `chord_intersection.rs:15` и `curve_offset_join.rs:14` — идентичная сигнатура | Общий хелпер в `offset/chord/` (после группировки они окажутся соседями) |
| 4 | Компенсированное суммирование: приватный `CompensatedSum` (Neumaier) в `curve.rs:49–87` при наличии полного EFT-набора в `compensated.rs` (two_sum, dot_k) | `curve.rs:49–87` vs `compensated.rs` | НЕ трогать горячий путь `evaluate` сейчас; в перспективе — опциональный отдельный коммит: `CompensatedSum` выражается через `compensated::two_sum`, с бенчем. Пометить как «low priority, verify no perf regression» |
| 5 | Однородный интервальный де Бур | `curve_distance.rs:9+` (кривые), `surface_distance.rs:18+` (поверхности), `interval_eval.rs` (обёртка), `interval_newton.rs` (потребитель), `normal_cone.rs:76` (интервальный треугольник Кокса–де Бура) | Выделить `numerics/interval_eval/` как поддерево: общий homogeneous-interval ядерный код (span locate + blossom/de Boor) с 1D- и 2D-адаптерами. Крупнейшая настоящая дедупликация, но и самая рискованная — делать после всех перемещений, отдельным этапом |
| 6 | Обобщённый Cox–de Boor по скаляру | `dual.rs:387` (generic lift), `curve.rs:573,895` (`basis`, `basis_funs_ders`), `foundation/interpolation.rs:872` | НЕ объединять: `curve.rs` — горячий путь с CompensatedSum и ошибочными статусами; generic-вариант dual.rs оставить для AD. Зафиксировать в doc-комментариях взаимные ссылки |
| 7 | Скелет «аудит-отчёта»: `Report { …, unresolved: Vec<…> }` + `tolerance_evidence` + serialization | `sweep_pair_audit.rs`, `sweep_wall_audit.rs`, `sweep_contour_audit.rs`, `sweep_seam_audit.rs`, `trim_region_audit.rs`, `periodic_seam_verification.rs`, `periodic_surface_verification.rs`, `curve_regularity.rs`, `surface_injectivity.rs` | Общие типы `audit::Unresolved { subject, reason }` и макро/trait для сериализации — только после группировки в `sweeps/audit/` и `trim/`; слияние логики НЕ делать (контракты у каждого свои, это осознанно) |
| 8 | Семейство transition: построение двух сечений + `surface::loft` | `circle_transition.rs` (140), `ellipse_transition.rs:13–21` (уже тонкий), `circle_rectangle_transition.rs` (187) | Объединить в `construct/surfaces/transitions/` с общим `ruled_loft_two(a, b)` хелпером; модули слишком малы, чтобы жить по отдельности, но публичные типы (`CircleSection`, `EllipseSection`) сохранить |
| 9 | Chord-семейство опирается на `curve_offset::Segment` и `curve_offset_diagnostics` — это один домен, размазанный по 8 файлам верхнего уровня | `chord_arrangement.rs:4,50–87`, `chord_winding.rs:3`, `chord_embedding.rs:6`, `chord_witness.rs:4` | Не слияние кода, а группировка в `offset/chord/`; внутри — общий `mod predicates` из curve_offset_diagnostics |
| 10 | Каркас субдивизионного distance (очередь ячеек, enclosure, сплит, отсев) | `curve_distance.rs` (570), `surface_distance.rs` (735), `curve_surface_distance.rs` (183), `trimmed_surface_distance.rs` (534) | После этапа 5 общий `distance/subdivision.rs` (бюджет ячеек, цикл, DistanceStopReason уже общий через `distance_bounds`). Только каркас — enclosure-логика остаётся per-dimension |
| 11 | Проверенное НЕ-дублирование | `robust_solvers.rs:14–17` явно документирует, что LM-демпинг в `foundation/fitting.rs` — локальный; `interval_eval.rs:17` — `pub use distance_bounds::Interval` (тип Interval уже един) | Оставить как есть; зафиксировать в плане, чтобы не «чинить» |

## 3. Что РАЗЪЕДИНИТЬ (топ по размеру и смешанной ответственности)

| Файл | Строк | Проблема | Действие |
|---|---|---|---|
| `intersection.rs` | 1773 | mod.rs-хаб + вся curve/curve логика + константы + хелперы | Перенести c/c-ядро в `intersection/curve_curve.rs`; в mod.rs оставить допуски/константы/реэкспорты |
| `curve.rs` | 1748 | Тип + валидация + evaluate + basis + knot-операции + decompose + merge/clamp/normalize + тесты | Сплит на `core/curve/{mod,basis,knot_ops,decompose}.rs` (таблица §1). Поведение не менять, только перенос |
| `robust_solvers.rs` | 1523 | 5 независимых solver'ов в одном файле | `numerics/robust_solvers/{rrqr,trust_region,lm,inversion,rank}.rs` — границы уже обозначены в doc-заголовке файла |
| `progressive_miter.rs` | 1371 (+ ~3400 в поддереве) | Ядро свипа + валидация законов + 10 сертификатов | Ядро → `progressive_miter/core.rs`; law-валидация → `law.rs`; сертификаты уже в поддереве |
| `surface_offset.rs` | 1233 | Конструктор + OffsetValidityReport (кривизненное precondition) + сертификация отклонения | `offset/surface_offset/{mod,validity,deviation}.rs` |
| `foundation.rs` | 1167 | Хаб реэкспортов (~80 строк) + хелперы `interval/box_of/context/copy_context` + MAX_CERTIFICATE_CELLS + certify-логика | `foundation/mod.rs` = только реэкспорты; хелперы → `foundation/common.rs`; остальное уже в поддереве (fitting 1002, interpolation 883, parameter_mapping 863, certificates 522 — дальше не дробить) |
| `progressive_sweep.rs` | 1156 (+ tests 1048) | Свип + сериализация + большие тесты | Тесты уже отдельно; ядро приемлемо, дробить только если этап 9 покажет дубли с progressive_miter |
| `convex_distance.rs` | 1017 | GJK + EPA + OBB SAT + control-cloud адаптеры | `broadphase/{gjk_epa,obb_sat,clouds}.rs` — низкий приоритет |
| `normal_cone.rs` | 941 | Конус + силуэт + кривизна + offset-гладкость + draft | Опционально 2 файла (cone / applications) |
| `primitives.rs` | 1080 | Коники + поверхности вращения | `construct/curves/primitives/{conics,revolution}.rs` |
| `transport.rs` | 1022 | JSON-диспетчер | НЕ ТРОГАТЬ (см. риски) |
| `surface.rs` | 681 | Тип + evaluate + loft/extrude/revolve/sweep конструкторы | Конструкторы можно увести в `construct/surfaces/native.rs`, но `surface::loft` — широко используемый путь (ellipse_transition и др.); только с шимом. Средний приоритет |

## 4. Поэтапный план миграции

Каждый этап — отдельный коммит, после каждого: `cargo test -p nurbs-core` (все 790 зелёные) + `cargo check -p brep-core --all-features`. Все перемещения через `git mv`, `#[path = "..."]` атрибуты обновляются в том же коммите (затронуты: curve/serialization, curve_offset_diagnostics/*, intersection/*, progressive_miter/*, continuity/*, foundation/*).

- **Этап 0. Подготовка.** Зафиксировать инвентарь (этот документ). Договориться: старые пути = `pub use` шимы, удаление только на мажорной версии.
- **Этап 1. `numerics/`** (самая новая, самая автономная группа): compensated, dual, sturm, interval_newton, robust_solvers, bezier_extraction, normal_cone, convex_distance, obb_tree, exact_products, conditioning, interval_eval, distance_bounds, periodic_chart, curve_jets, exact_curve_segments. В `lib.rs` — `pub use numerics::compensated;` и т.д. для всех.
- **Этап 2. `core/`**: curve(+subdir), surface(+serialization), bounds, affine, coordinate_frame, canonical. Риск этапа — выше всех (все зависят от curve/surface); шимы обязательны.
- **Этап 3. `distance/`**: curve_distance, surface_distance, curve_surface_distance, trimmed_surface_distance (+ их serialization-подфайлы).
- **Этап 4. `intersection/`**: intersection (+bezier_clip/plane/curve_surface), ss_intersection → `intersection/ss/`, curve_self_intersection, surface_self_intersection.
- **Этап 5. `offset/` + chord**: curve_offset, curve_offset_join, curve_offset_wire, curve_offset_diagnostics, surface_offset, chord_×7. Внутри сразу сделать DRY #3 (общий 2D interval cross).
- **Этап 6. `sweeps/`**: 8 свипов + 6 аудитов в `sweeps/audit/`.
- **Этап 7. `construct/`**: curves (21 модуль: спирали, helix, involute, rack, thread, clothoid, biarc, hermite, paths, formula, primitives, engineering_profiles, …), surfaces (lofts, sections, transitions — с DRY #8).
- **Этап 8. Остальное**: analysis/, editing/, join/, trim/, tessellation/, certificates/. Обновить `domains.rs` на новые пути (внутренне), шимы держат старые.
- **Этап 9. DRY-извлечения** (каждое — свой коммит, в порядке роста риска): #1 math-core vec-хелперы → #3 (уже в этапе 5) → #2 interval vec3 → #7 audit-скелет → #10 distance-каркас → #5 общий interval de Boor → (опционально) #4 CompensatedSum через two_sum с бенчмарком.
- **Этап 10. Сплиты крупных файлов** (таблица §3): curve.rs → intersection.rs → robust_solvers.rs → foundation.rs-хаб → surface_offset.rs → progressive_miter.rs. Каждый сплит — отдельный коммит.
- **Этап 11. Финализация API.** Объявить каноническими новые пути в документации и `domains.rs`; старые шимы пометить `#[deprecated]` (или убрать на мажорном бампе). `README.md` крейта обновить (он включается через `#![doc = include_str!]`).

## 5. Риски и «не трогать»

1. **`transport.rs` (1022 строки, feature `transport`)** — внешний JSON-контракт; имена команд/полей не менять, файл не двигать до отдельного решения.
2. **Горячие пути**: `Curve::evaluate`, `basis_funs_ders`, `find_span` (curve.rs) и интервальный de Boor в curve_distance/surface_distance — переносить байт-в-байт, без «заодно поправить». DRY #4/#5 — только после бенчмарка.
3. **`serialization.rs` подфайлы (feature `codec`)** — wire-форматы value-codec не меняются; файлы двигаются только вместе с родителем, `#[path]` обновляется аккуратно (паттерн `#[path = "x/serialization.rs"]` ломается при переносе родителя — главный механический риск этапов 1–8).
4. **`foundation/`** — сертификационный код с outward-rounding инвариантами (`next_down/next_up`); сплитить только хаб (этап 10), логику не рефакторить.
5. **Модули с осознанно «честными» контрактами** (audit/verification — каждый документирует, что он НЕ доказывает): не объединять семантику, только группировать.
6. **Порядок**: numerics первым (наименее связан), core вторым (наиболее связан, но шимы снимают риск), DRY и сплиты — строго после перемещений, чтобы диффы перемещений оставались тривиально проверяемыми (`git mv` + правка use).
7. **brep-core и другие потребители** импортируют плоские пути (`nurbs_core::{curve::Curve, rack, thread, curve_surface_agreement, curve_offset_diagnostics, …}`) — шимы сохраняют компиляцию на всём переходе; после этапа 11 потребителей перевести на новые пути отдельным PR.

## 6. Заметки по соседним крейтам

- **brep-core**: `rack.rs` (37 строк) и `thread.rs` (1618 строк) — тонкие топологические обёртки над `nurbs_core::rack` / `nurbs_core::thread` (профили/патчи в nurbs-core, тела/швы в brep-core). Расслоение чистое, дублирования геометрии нет; переносить ничего не нужно.
- **math-core**: экспортирует `cross/dot/norm/sub/unit/eigen/point_moments` — nurbs-core частично пользуется (convex_distance, obb_tree, progressive_miter используют `math_core::{cross,dot,sub}`), но biarc.rs и др. дублируют (DRY #1). Никакой геометрии NURBS в math-core нет.
- **geometry-ops**: уровень mesh/polygon-операций (sculpt, extrude_slices, revolution, polygon_mesh…), пересечений с NURBS-ядром не видно.
- Отдельного аудита brep-core не делалось; если появится план реорганизации brep-core, первым кандидатом проверить `uv_arrangement/uv_regions` против `nurbs_core::trim_domain` и `chord_*` — по названиям возможна тематическая близость.
