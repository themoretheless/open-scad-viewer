# Инвентаризация: `nurbs-core`

Модулей верхнего уровня: 79. Модулей в циклах: 44 (234086 LOC из 259762).

Столбцы: LOC и Fn только по продакшен-коду (без `#[cfg(test)]`, tests/, examples/). `max cog` это максимальная когнитивная сложность функции в модуле. `Клоны %` это доля значимых строк, входящих в точные дубли длиной от 30 строк. `Churn 60д` это число коммитов за 60 дней по файлам модуля. `Dead` это элементы, которые rustc считает мёртвыми при `--force-warn dead_code`. `Цикл` это номер группы циклических зависимостей (SCC) между модулями верхнего уровня. Приоритет считается как churn x суммарная когнитивная сложность: верхние 10% это P1, следующие 25% это P2.

| Модуль | LOC | Файлов | Fn | max cog | Fn >100 | cy >15 | par >5 | unwrap | Клоны % | Churn 60д | Clippy | Dead | Цикл | Приоритет | Статус |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|:--:|:--:|---|
| `intersection` | 38340 | 174 | 1389 | 222 | 59 | 82 | 51 | 186 | 2 | 486 | 111 | 4 | C1 | P1 | todo |
| `sweeps` | 34982 | 135 | 748 | 310 | 50 | 79 | 163 | 334 | 1 | 509 | 179 | 12 | C1 | P1 | todo |
| `analysis` | 33865 | 122 | 757 | 191 | 62 | 105 | 76 | 127 | 2 | 358 | 118 | 14 | C1 | P1 | todo |
| `construct` | 31017 | 129 | 812 | 89 | 34 | 52 | 80 | 89 | 17 | 276 | 82 | 3 | C1 | P1 | todo |
| `offset` | 14281 | 63 | 418 | 110 | 26 | 36 | 31 | 34 | 10 | 111 | 47 | 13 | C1 | P1 | todo |
| `foundation` | 10722 | 35 | 270 | 106 | 14 | 13 | 16 | 37 | 2 | 122 | 12 | 4 | C1 | P1 | todo |
| `numerics` | 9613 | 30 | 282 | 56 | 6 | 11 | 1 | 9 | 7 | 85 | 34 | 3 | C1 | P1 | todo |
| `trim` | 9484 | 37 | 208 | 118 | 23 | 35 | 20 | 54 | 0 | 77 | 20 | 0 | C1 | P1 | todo |
| `editing` | 7488 | 49 | 229 | 34 | 2 | 4 | 5 | 34 | 0 | 130 | 22 | 0 | C1 | P1 | todo |
| `join` | 7380 | 29 | 206 | 110 | 12 | 16 | 40 | 42 | 0 | 95 | 38 | 0 | C1 | P1 | todo |
| `constrained_surface` | 6537 | 24 | 235 | 96 | 10 | 18 | 40 | 6 | 0 | 24 | 21 | 0 |  | P1 | todo |
| `surface_g2_pcurves` | 6241 | 23 | 147 | 100 | 12 | 22 | 24 | 7 | 0 | 42 | 30 | 6 | C1 | P1 | todo |
| `original_graph_transition` | 4797 | 17 | 141 | 68 | 6 | 8 | 6 | 35 | 0 | 21 | 15 | 0 |  | P1 | todo |
| `core` | 3997 | 18 | 147 | 53 | 3 | 5 | 1 | 7 | 3 | 61 | 5 | 0 | C1 | P1 | todo |
| `distance` | 3049 | 12 | 98 | 39 | 1 | 2 | 4 | 13 | 0 | 37 | 10 | 1 | C1 | P1 | todo |
| `profile_section_bridge` | 2431 | 7 | 53 | 108 | 4 | 9 | 5 | 0 | 0 | 41 | 25 | 0 | C1 | P1 | todo |
| `factored_surface_normals` | 2427 | 10 | 66 | 60 | 5 | 4 | 4 | 20 | 0 | 37 | 1 | 0 | C1 | P1 | todo |
| `transport` | 2274 | 11 | 23 | 212 | 6 | 7 | 0 | 4 | 0 | 23 | 8 | 1 | C1 | P1 | todo |
| `surface_weight_profile` | 1765 | 5 | 71 | 71 | 2 | 5 | 6 | 1 | 0 | 47 | 15 | 0 | C1 | P1 | todo |
| `surface_quotient_injectivity` | 1678 | 7 | 41 | 74 | 4 | 4 | 6 | 10 | 0 | 19 | 7 | 0 | C1 | P1 | todo |
| `corner_blend` | 1577 | 3 | 39 | 97 | 5 | 5 | 13 | 12 | 0 | 4 | 7 | 0 |  | P2 | todo |
| `surface_self_chord` | 1386 | 10 | 54 | 72 | 2 | 8 | 14 | 0 | 0 | 52 | 10 | 1 | C1 | P1 | todo |
| `curve_partition_agreement` | 1246 | 6 | 37 | 25 | 3 | 3 | 2 | 22 | 0 | 9 | 3 | 0 | C1 | P2 | todo |
| `normal_alignment` | 1191 | 9 | 46 | 36 | 0 | 5 | 4 | 3 | 0 | 55 | 7 | 0 | C1 | P1 | todo |
| `tessellation` | 1043 | 6 | 13 | 50 | 3 | 4 | 1 | 2 | 0 | 16 | 3 | 0 |  | P2 | todo |
| `profile_expression_radius` | 1032 | 4 | 36 | 85 | 5 | 4 | 6 | 0 | 0 | 14 | 17 | 0 |  | P1 | todo |
| `certificates` | 1011 | 4 | 21 | 26 | 3 | 3 | 0 | 3 | 28 | 9 | 0 | 0 | C1 | P2 | todo |
| `moving_radius` | 938 | 2 | 28 | 56 | 2 | 3 | 4 | 1 | 0 | 7 | 2 | 0 |  | P2 | todo |
| `offset_contact_trims` | 739 | 1 | 5 | 30 | 1 | 1 | 2 | 1 | 0 | 6 | 3 | 0 | C1 | P3 | todo |
| `offset_face_loops` | 726 | 1 | 8 | 73 | 1 | 2 | 4 | 4 | 0 | 3 | 3 | 0 |  | P2 | todo |
| `offset_contact_pcurve` | 689 | 2 | 10 | 67 | 2 | 2 | 5 | 2 | 0 | 4 | 6 | 0 | C1 | P2 | todo |
| `rational_pole_differential` | 689 | 1 | 20 | 53 | 1 | 1 | 2 | 4 | 0 | 16 | 0 | 0 | C1 | P2 | todo |
| `surface_g1_pcurves` | 667 | 2 | 17 | 66 | 1 | 2 | 3 | 2 | 0 | 3 | 3 | 0 |  | P2 | todo |
| `offset_envelope_fit` | 634 | 1 | 8 | 56 | 1 | 1 | 4 | 0 | 0 | 3 | 3 | 0 | C1 | P3 | todo |
| `sweep_support` | 629 | 6 | 26 | 26 | 0 | 0 | 0 | 2 | 66 | 6 | 3 | 13 | C1 | P2 | todo |
| `offset_envelope` | 599 | 1 | 7 | 28 | 1 | 1 | 2 | 2 | 10 | 4 | 2 | 0 | C1 | P3 | todo |
| `curve_surface_affine` | 574 | 1 | 2 | 32 | 2 | 2 | 1 | 2 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `surface_self_chord_coverage` | 566 | 1 | 15 | 133 | 1 | 2 | 4 | 0 | 0 | 24 | 4 | 1 |  | P2 | todo |
| `offset_contact_tangent` | 542 | 2 | 9 | 23 | 1 | 1 | 2 | 0 | 11 | 6 | 1 | 0 | C1 | P2 | todo |
| `offset_path_pcurves` | 541 | 1 | 2 | 51 | 1 | 1 | 2 | 5 | 0 | 1 | 2 | 0 |  | P3 | todo |
| `profile_expression_bernstein` | 538 | 1 | 6 | 40 | 2 | 3 | 3 | 0 | 0 | 6 | 3 | 0 |  | P2 | todo |
| `loft_reparameterization` | 524 | 1 | 5 | 14 | 0 | 0 | 1 | 1 | 0 | 3 | 7 | 0 |  | P3 | todo |
| `moving_envelope` | 508 | 2 | 10 | 46 | 2 | 2 | 1 | 0 | 0 | 3 | 1 | 0 |  | P3 | todo |
| `uv_curve_crossings` | 502 | 1 | 7 | 65 | 1 | 2 | 1 | 0 | 0 | 4 | 2 | 0 |  | P2 | todo |
| `contact_normal_agreement` | 465 | 1 | 4 | 30 | 1 | 1 | 2 | 0 | 0 | 2 | 2 | 0 | C1 | P3 | todo |
| `surface_boundary_secant` | 447 | 1 | 13 | 127 | 1 | 1 | 1 | 0 | 0 | 2 | 2 | 0 | C1 | P3 | todo |
| `offset_source_boundary` | 418 | 1 | 3 | 53 | 1 | 1 | 1 | 0 | 0 | 4 | 1 | 0 | C1 | P3 | todo |
| `surface_flux` | 400 | 1 | 6 | 22 | 0 | 0 | 2 | 0 | 0 | 6 | 0 | 0 | C1 | P3 | todo |
| `curve_surface_plane` | 386 | 1 | 4 | 18 | 0 | 1 | 1 | 2 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `curve_surface_lift` | 375 | 1 | 5 | 33 | 1 | 1 | 0 | 5 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `(root)` | 374 | 1 | 5 | 0 | 0 | 0 | 0 | 0 | 0 | 77 | 0 | 0 |  | P3 | todo |
| `offset_path_trims` | 352 | 1 | 1 | 47 | 1 | 1 | 1 | 0 | 0 | 1 | 4 | 0 |  | P3 | todo |
| `offset_contact_path` | 344 | 1 | 2 | 54 | 1 | 1 | 1 | 0 | 0 | 1 | 0 | 0 | C1 | P3 | todo |
| `profile_projection_jacobian` | 332 | 1 | 4 | 57 | 1 | 1 | 0 | 3 | 0 | 3 | 1 | 0 |  | P3 | todo |
| `curve_axis_driver` | 302 | 1 | 4 | 31 | 1 | 2 | 0 | 1 | 0 | 3 | 1 | 0 |  | P3 | todo |
| `domains` | 298 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 16 | 0 | 0 |  | P3 | todo |
| `trimmed_offset_contact` | 296 | 1 | 2 | 19 | 1 | 0 | 1 | 0 | 0 | 3 | 3 | 0 | C1 | P3 | todo |
| `planar_area` | 291 | 1 | 6 | 18 | 0 | 0 | 0 | 1 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `retained_wall_coefficients` | 270 | 1 | 5 | 64 | 0 | 4 | 0 | 0 | 0 | 5 | 1 | 0 | C1 | P2 | todo |
| `surface_projection_jacobian` | 255 | 1 | 4 | 34 | 1 | 1 | 0 | 1 | 0 | 3 | 1 | 0 |  | P3 | todo |
| `offset_contact_predictor` | 244 | 1 | 4 | 41 | 1 | 1 | 1 | 0 | 0 | 1 | 3 | 0 | C1 | P3 | todo |
| `profile_projected_jordan` | 242 | 1 | 7 | 43 | 1 | 1 | 0 | 0 | 35 | 1 | 0 | 0 |  | P3 | todo |
| `curve_quadratic_separator` | 229 | 1 | 6 | 19 | 1 | 1 | 0 | 2 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `surface_projected_jordan` | 229 | 1 | 7 | 39 | 1 | 1 | 0 | 0 | 37 | 1 | 0 | 0 |  | P3 | todo |
| `curve_point_identity` | 207 | 1 | 2 | 23 | 0 | 1 | 0 | 5 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `polyhedral_spline` | 200 | 1 | 13 | 10 | 0 | 0 | 1 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `bernstein_interpolation` | 195 | 1 | 4 | 25 | 0 | 0 | 0 | 0 | 0 | 3 | 0 | 1 | C1 | P3 | todo |
| `boundary_partition` | 186 | 1 | 1 | 20 | 0 | 0 | 1 | 0 | 0 | 1 | 1 | 0 |  | P3 | todo |
| `polar_surface_differential` | 184 | 1 | 1 | 31 | 0 | 1 | 0 | 0 | 0 | 1 | 3 | 0 | C1 | P3 | todo |
| `synchronized_boundary_normals` | 158 | 1 | 11 | 48 | 0 | 2 | 1 | 2 | 0 | 1 | 1 | 0 |  | P3 | todo |
| `offset_patch_boundary` | 153 | 1 | 2 | 8 | 0 | 0 | 0 | 2 | 0 | 1 | 0 | 0 | C1 | P3 | todo |
| `surface_coordinate_flow` | 98 | 1 | 1 | 35 | 0 | 1 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `curve_coordinate_clock` | 92 | 1 | 1 | 37 | 0 | 1 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `retained_wall_domain` | 91 | 1 | 3 | 24 | 0 | 1 | 0 | 0 | 0 | 5 | 0 | 0 | C1 | P3 | todo |
| `sweep_section_correction` | 78 | 1 | 4 | 19 | 0 | 1 | 0 | 0 | 0 | 1 | 0 | 0 | C1 | P3 | todo |
| `sweep_seam_set` | 77 | 1 | 1 | 12 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | C1 | P3 | todo |
| `retained_wall_domain_certificate` | 35 | 1 | 1 | 23 | 0 | 1 | 0 | 0 | 0 | 2 | 0 | 0 | C1 | P3 | todo |
| `tests` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `sweep_boundary_owned` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
