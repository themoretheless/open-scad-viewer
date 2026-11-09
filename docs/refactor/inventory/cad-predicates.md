# Инвентаризация: `cad-predicates`

Модулей верхнего уровня: 55. Модулей в циклах: 4 (5369 LOC из 25812).

Столбцы: LOC и Fn только по продакшен-коду (без `#[cfg(test)]`, tests/, examples/). `max cog` это максимальная когнитивная сложность функции в модуле. `Клоны %` это доля значимых строк, входящих в точные дубли длиной от 30 строк. `Churn 60д` это число коммитов за 60 дней по файлам модуля. `Dead` это элементы, которые rustc считает мёртвыми при `--force-warn dead_code`. `Цикл` это номер группы циклических зависимостей (SCC) между модулями верхнего уровня. Приоритет считается как churn x суммарная когнитивная сложность: верхние 10% это P1, следующие 25% это P2.

| Модуль | LOC | Файлов | Fn | max cog | Fn >100 | cy >15 | par >5 | unwrap | Клоны % | Churn 60д | Clippy | Dead | Цикл | Приоритет | Статус |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|:--:|:--:|---|
| `bspline_strip_nonlinear` | 3701 | 11 | 88 | 83 | 7 | 11 | 13 | 27 | 0 | 15 | 33 | 0 | C1 | P1 | todo |
| `bspline_internal_jets` | 2205 | 12 | 34 | 65 | 6 | 12 | 20 | 1 | 4 | 33 | 20 | 0 |  | P1 | todo |
| `planar_bezier_roots` | 1686 | 4 | 44 | 61 | 2 | 6 | 0 | 12 | 0 | 16 | 14 | 0 |  | P1 | todo |
| `construction3` | 1674 | 4 | 53 | 59 | 2 | 2 | 4 | 0 | 0 | 4 | 0 | 0 |  | P2 | todo |
| `projected_surface_jacobian` | 1138 | 3 | 37 | 41 | 3 | 3 | 8 | 5 | 0 | 20 | 12 | 0 |  | P1 | todo |
| `bezier_composition_identity` | 1057 | 3 | 31 | 90 | 0 | 5 | 12 | 3 | 0 | 14 | 25 | 0 |  | P1 | todo |
| `bezier_curvature_bound` | 958 | 3 | 15 | 84 | 2 | 2 | 3 | 11 | 0 | 3 | 6 | 0 |  | P2 | todo |
| `(root)` | 780 | 1 | 15 | 38 | 1 | 1 | 1 | 13 | 0 | 43 | 0 | 0 |  | P2 | todo |
| `bspline_strip_jets` | 702 | 1 | 9 | 71 | 2 | 2 | 7 | 1 | 0 | 3 | 9 | 0 | C2 | P2 | todo |
| `tensor_identity` | 671 | 3 | 13 | 37 | 0 | 1 | 4 | 1 | 0 | 9 | 0 | 0 |  | P2 | todo |
| `line_crossing_parameter` | 667 | 1 | 11 | 36 | 0 | 0 | 1 | 1 | 0 | 6 | 1 | 0 |  | P2 | todo |
| `fibered_affine` | 625 | 2 | 11 | 61 | 2 | 1 | 2 | 0 | 0 | 3 | 5 | 0 |  | P3 | todo |
| `bspline_strip_map` | 582 | 1 | 10 | 66 | 1 | 2 | 3 | 4 | 0 | 2 | 8 | 0 | C1 | P3 | todo |
| `arithmetic` | 565 | 1 | 38 | 37 | 0 | 1 | 0 | 0 | 0 | 10 | 0 | 0 |  | P2 | todo |
| `context` | 565 | 2 | 33 | 13 | 0 | 0 | 0 | 0 | 0 | 6 | 0 | 0 |  | P3 | todo |
| `bspline_restriction_identity` | 533 | 2 | 6 | 82 | 2 | 3 | 3 | 1 | 0 | 5 | 6 | 0 |  | P2 | todo |
| `construction` | 526 | 1 | 28 | 13 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `natural_meridian_g2` | 400 | 2 | 16 | 57 | 1 | 1 | 1 | 2 | 0 | 7 | 2 | 0 |  | P2 | todo |
| `paired_graph_projection` | 393 | 2 | 11 | 59 | 2 | 2 | 3 | 0 | 0 | 4 | 1 | 0 |  | P2 | todo |
| `bspline_strip_spans` | 384 | 1 | 4 | 69 | 2 | 2 | 3 | 3 | 0 | 2 | 4 | 0 | C2 | P3 | todo |
| `projective_strip_jets` | 376 | 1 | 4 | 122 | 1 | 1 | 1 | 2 | 0 | 6 | 2 | 0 |  | P2 | todo |
| `bezier_identity` | 371 | 1 | 2 | 40 | 0 | 1 | 1 | 0 | 0 | 6 | 4 | 0 |  | P2 | todo |
| `recipe_codec3` | 329 | 1 | 12 | 31 | 0 | 2 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `pole_quadratic_jet` | 326 | 1 | 14 | 46 | 1 | 1 | 0 | 2 | 0 | 2 | 2 | 0 |  | P3 | todo |
| `bspline_endpoint_jets` | 319 | 1 | 1 | 46 | 1 | 1 | 0 | 0 | 0 | 2 | 1 | 0 |  | P3 | todo |
| `bezier_quadric_identity` | 281 | 1 | 5 | 30 | 0 | 0 | 0 | 0 | 0 | 5 | 8 | 0 |  | P2 | todo |
| `bspline_projective_correspondence` | 276 | 1 | 1 | 75 | 1 | 1 | 1 | 0 | 0 | 1 | 2 | 0 |  | P3 | todo |
| `bezier_projective_correspondence` | 274 | 1 | 3 | 27 | 0 | 0 | 2 | 0 | 0 | 2 | 3 | 0 |  | P3 | todo |
| `bezier_strip_jets` | 271 | 1 | 1 | 69 | 1 | 1 | 0 | 1 | 0 | 4 | 1 | 0 |  | P3 | todo |
| `quadratic_shear_identity` | 243 | 1 | 2 | 35 | 1 | 0 | 1 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `bezier_point_identity` | 236 | 1 | 6 | 30 | 0 | 1 | 0 | 1 | 0 | 2 | 5 | 0 |  | P3 | todo |
| `line_chart_identity` | 219 | 1 | 2 | 54 | 0 | 1 | 0 | 0 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `bspline_trace_identity` | 216 | 1 | 3 | 70 | 1 | 1 | 3 | 1 | 0 | 2 | 3 | 0 |  | P3 | todo |
| `bspline_knot_jets` | 215 | 1 | 1 | 51 | 1 | 1 | 0 | 2 | 0 | 3 | 1 | 0 |  | P3 | todo |
| `bspline_lift_point` | 196 | 1 | 5 | 26 | 1 | 0 | 1 | 2 | 0 | 1 | 1 | 0 |  | P3 | todo |
| `bilinear_graph_side` | 192 | 1 | 1 | 34 | 0 | 1 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `bezier_line_image` | 185 | 1 | 1 | 85 | 1 | 1 | 0 | 3 | 0 | 2 | 2 | 0 |  | P3 | todo |
| `bezier_restriction_identity` | 182 | 1 | 1 | 37 | 1 | 1 | 1 | 0 | 0 | 1 | 3 | 0 |  | P3 | todo |
| `oriented_ray_fan` | 181 | 1 | 1 | 25 | 0 | 1 | 0 | 1 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `curve_geometric_jets` | 158 | 1 | 1 | 53 | 1 | 1 | 0 | 1 | 0 | 2 | 2 | 0 |  | P3 | todo |
| `algorithm_expression` | 135 | 1 | 1 | 5 | 1 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `operation_contract` | 109 | 1 | 2 | 6 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `bspline_quadric_identity` | 106 | 1 | 3 | 48 | 0 | 1 | 1 | 1 | 0 | 5 | 2 | 0 |  | P2 | todo |
| `quadratic_curve_separator` | 93 | 1 | 1 | 18 | 0 | 0 | 0 | 2 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `bezier_trim_join` | 92 | 1 | 2 | 10 | 0 | 0 | 0 | 0 | 0 | 1 | 2 | 0 |  | P3 | todo |
| `algorithm_segment_intersects_segment_2d` | 63 | 1 | 1 | 22 | 0 | 1 | 0 | 2 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `algorithm_collinear3d` | 53 | 1 | 1 | 12 | 0 | 0 | 0 | 2 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `algorithm_evaluate` | 53 | 1 | 1 | 9 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `algorithm_coplanar3d` | 52 | 1 | 1 | 6 | 0 | 0 | 0 | 3 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `algorithm_point_in_triangle_2d` | 51 | 1 | 1 | 10 | 0 | 0 | 0 | 2 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `algorithm_insphere3d` | 42 | 1 | 1 | 3 | 0 | 0 | 0 | 3 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `algorithm_distance_enclosure` | 41 | 1 | 1 | 5 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `algorithm_exact_inputs` | 38 | 1 | 1 | 14 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `combinatorics` | 26 | 1 | 1 | 3 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `tests` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 10 | 0 | 0 |  | P3 | todo |
