# Инвентаризация: `brep-core`

Модулей верхнего уровня: 247. Модулей в циклах: 107 (236395 LOC из 349961).

Столбцы: LOC и Fn только по продакшен-коду (без `#[cfg(test)]`, tests/, examples/). `max cog` это максимальная когнитивная сложность функции в модуле. `Клоны %` это доля значимых строк, входящих в точные дубли длиной от 30 строк. `Churn 60д` это число коммитов за 60 дней по файлам модуля. `Dead` это элементы, которые rustc считает мёртвыми при `--force-warn dead_code`. `Цикл` это номер группы циклических зависимостей (SCC) между модулями верхнего уровня. Приоритет считается как churn x суммарная когнитивная сложность: верхние 10% это P1, следующие 25% это P2.

| Модуль | LOC | Файлов | Fn | max cog | Fn >100 | cy >15 | par >5 | unwrap | Клоны % | Churn 60д | Clippy | Dead | Цикл | Приоритет | Статус |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|:--:|:--:|---|
| `profile_certificate` | 52161 | 173 | 1801 | 144 | 61 | 107 | 60 | 283 | 1 | 252 | 129 | 2 | C2 | P1 | todo |
| `minkowski3d` | 23608 | 93 | 719 | 117 | 33 | 56 | 16 | 347 | 0 | 162 | 72 | 1 | C2 | P1 | todo |
| `intersections` | 20764 | 34 | 222 | 173 | 35 | 43 | 16 | 33 | 65 | 171 | 1 | 0 | C2 | P1 | todo |
| `region_replacement` | 19712 | 70 | 436 | 187 | 47 | 56 | 51 | 98 | 2 | 110 | 64 | 1 | C2 | P1 | todo |
| `source_profile_body` | 16288 | 54 | 408 | 245 | 31 | 49 | 55 | 89 | 1 | 239 | 42 | 0 |  | P1 | todo |
| `feature_ops` | 13598 | 68 | 570 | 98 | 31 | 29 | 26 | 37 | 0 | 69 | 13 | 0 | C2 | P1 | todo |
| `direct_edit` | 7876 | 16 | 153 | 148 | 19 | 14 | 6 | 0 | 0 | 41 | 16 | 1 | C2 | P1 | todo |
| `analytic` | 7169 | 21 | 171 | 238 | 16 | 16 | 26 | 36 | 6 | 81 | 32 | 4 | C2 | P1 | todo |
| `shell_thickness` | 6870 | 27 | 283 | 78 | 16 | 19 | 26 | 22 | 0 | 29 | 11 | 0 |  | P1 | todo |
| `face_cut` | 6160 | 32 | 82 | 88 | 20 | 22 | 41 | 33 | 0 | 62 | 21 | 0 | C2 | P1 | todo |
| `sweep_boundary_owned` | 5783 | 19 | 144 | 150 | 14 | 20 | 32 | 36 | 0 | 117 | 29 | 0 |  | P1 | todo |
| `step_interchange_v3` | 5598 | 7 | 112 | 213 | 12 | 22 | 9 | 12 | 42 | 47 | 15 | 0 | C4 | P1 | todo |
| `ss_face_membership` | 5082 | 24 | 176 | 90 | 13 | 13 | 6 | 21 | 0 | 86 | 11 | 0 |  | P1 | todo |
| `source_seam_tangency` | 4702 | 20 | 162 | 124 | 14 | 16 | 9 | 16 | 0 | 65 | 11 | 2 | C1 | P1 | todo |
| `mixed_root_model` | 4405 | 27 | 209 | 59 | 7 | 12 | 5 | 5 | 0 | 88 | 9 | 0 |  | P1 | todo |
| `nurbs_ss_g6` | 3650 | 9 | 77 | 94 | 9 | 12 | 4 | 9 | 41 | 8 | 0 | 0 | C2 | P1 | todo |
| `exact_graph_strip` | 3603 | 17 | 165 | 62 | 9 | 8 | 3 | 42 | 0 | 23 | 6 | 0 |  | P1 | todo |
| `analytic_features` | 3404 | 10 | 50 | 82 | 12 | 16 | 0 | 6 | 6 | 13 | 3 | 1 | C2 | P1 | todo |
| `analysis` | 3326 | 11 | 83 | 79 | 4 | 6 | 3 | 4 | 0 | 22 | 17 | 0 | C2 | P1 | todo |
| `step_interchange` | 3314 | 3 | 81 | 55 | 7 | 6 | 0 | 2 | 64 | 10 | 0 | 0 |  | P1 | todo |
| `operations` | 2803 | 9 | 73 | 77 | 4 | 9 | 0 | 2 | 0 | 25 | 7 | 0 | C2 | P1 | todo |
| `face_contacts` | 2775 | 10 | 59 | 189 | 2 | 10 | 5 | 7 | 5 | 31 | 9 | 1 | C3 | P1 | todo |
| `iges_interchange_v2` | 2688 | 1 | 62 | 81 | 7 | 6 | 1 | 8 | 29 | 13 | 2 | 0 |  | P1 | todo |
| `thread` | 2680 | 6 | 53 | 57 | 10 | 9 | 11 | 9 | 10 | 15 | 9 | 1 |  | P1 | todo |
| `gear` | 2442 | 9 | 83 | 106 | 4 | 8 | 7 | 18 | 0 | 22 | 3 | 1 |  | P1 | todo |
| `bezier_profile` | 2395 | 7 | 45 | 308 | 3 | 12 | 0 | 36 | 0 | 21 | 5 | 0 | C2 | P1 | todo |
| `healing` | 2237 | 5 | 51 | 40 | 3 | 4 | 0 | 0 | 0 | 8 | 1 | 0 |  | P2 | todo |
| `trim_sew` | 2124 | 6 | 44 | 43 | 6 | 5 | 1 | 0 | 0 | 12 | 1 | 0 | C2 | P2 | todo |
| `aag_features` | 2094 | 5 | 35 | 172 | 5 | 4 | 0 | 7 | 0 | 16 | 4 | 0 | C2 | P1 | todo |
| `sweep_smoothness` | 1882 | 6 | 15 | 160 | 4 | 6 | 2 | 4 | 0 | 26 | 4 | 0 | C3 | P1 | todo |
| `shared_boundary` | 1881 | 8 | 35 | 121 | 3 | 12 | 1 | 7 | 0 | 20 | 4 | 0 | C3 | P1 | todo |
| `boundary_hull_contact` | 1747 | 4 | 15 | 266 | 4 | 8 | 0 | 0 | 22 | 29 | 5 | 0 | C3 | P1 | todo |
| `step_pmi` | 1744 | 3 | 44 | 58 | 4 | 5 | 1 | 2 | 0 | 5 | 4 | 0 |  | P2 | todo |
| `fillet_contact` | 1712 | 6 | 32 | 122 | 3 | 7 | 17 | 26 | 0 | 9 | 3 | 0 | C2 | P2 | todo |
| `source_shared_edge` | 1692 | 8 | 49 | 87 | 2 | 3 | 8 | 3 | 0 | 22 | 2 | 0 | C1 | P1 | todo |
| `source_contour_proposal` | 1652 | 8 | 25 | 90 | 6 | 3 | 5 | 5 | 0 | 15 | 5 | 0 | C1 | P2 | todo |
| `imprint_pipeline` | 1610 | 5 | 24 | 39 | 3 | 4 | 8 | 2 | 0 | 10 | 2 | 0 |  | P2 | todo |
| `tolerant_boolean` | 1528 | 5 | 46 | 69 | 2 | 4 | 0 | 6 | 0 | 13 | 0 | 0 |  | P1 | todo |
| `closed_solid` | 1493 | 10 | 79 | 34 | 1 | 2 | 4 | 4 | 0 | 12 | 4 | 0 |  | P2 | todo |
| `step_v3_export` | 1490 | 7 | 38 | 99 | 4 | 5 | 0 | 4 | 68 | 13 | 0 | 0 | C4 | P1 | todo |
| `edge_partition` | 1438 | 5 | 41 | 77 | 5 | 5 | 10 | 4 | 12 | 8 | 5 | 1 | C2 | P2 | todo |
| `planar_trim` | 1410 | 4 | 38 | 110 | 6 | 8 | 0 | 3 | 0 | 6 | 0 | 0 | C2 | P2 | todo |
| `source_pole_tangency` | 1402 | 7 | 45 | 78 | 3 | 5 | 0 | 2 | 0 | 18 | 1 | 0 | C1 | P1 | todo |
| `sweep_miter_owned` | 1397 | 4 | 45 | 91 | 3 | 3 | 2 | 8 | 0 | 13 | 6 | 0 | C2 | P2 | todo |
| `retained_annular_cap` | 1386 | 4 | 21 | 44 | 4 | 4 | 3 | 1 | 0 | 13 | 4 | 0 |  | P2 | todo |
| `uv_arrangement` | 1292 | 4 | 19 | 126 | 3 | 5 | 0 | 5 | 0 | 6 | 0 | 0 | C2 | P2 | todo |
| `source_shell_incidence` | 1279 | 6 | 41 | 58 | 3 | 2 | 16 | 3 | 0 | 51 | 10 | 0 | C1 | P1 | todo |
| `contact_graft` | 1263 | 4 | 16 | 103 | 3 | 3 | 8 | 8 | 0 | 6 | 4 | 0 |  | P2 | todo |
| `sphere_mate` | 1225 | 4 | 44 | 116 | 3 | 3 | 2 | 2 | 0 | 8 | 0 | 0 | C2 | P2 | todo |
| `nurbs_step_shared` | 1214 | 2 | 41 | 50 | 1 | 1 | 0 | 0 | 87 | 7 | 0 | 0 |  | P2 | todo |
| `healing_report` | 1168 | 3 | 50 | 68 | 2 | 1 | 0 | 0 | 0 | 4 | 0 | 0 |  | P2 | todo |
| `circular_blend` | 1155 | 3 | 26 | 50 | 3 | 1 | 10 | 12 | 0 | 7 | 7 | 0 |  | P2 | todo |
| `root_model` | 1116 | 11 | 84 | 59 | 1 | 8 | 0 | 13 | 0 | 23 | 1 | 0 |  | P1 | todo |
| `close_topology` | 1085 | 4 | 33 | 73 | 2 | 2 | 0 | 2 | 4 | 5 | 0 | 0 |  | P2 | todo |
| `boundary_embedding` | 1083 | 3 | 17 | 69 | 2 | 3 | 4 | 12 | 8 | 25 | 4 | 0 | C3 | P1 | todo |
| `source_volume` | 1068 | 6 | 44 | 60 | 2 | 2 | 1 | 10 | 0 | 23 | 2 | 0 | C1 | P2 | todo |
| `pcurve_repair` | 1046 | 2 | 27 | 82 | 1 | 1 | 4 | 0 | 0 | 3 | 4 | 0 |  | P2 | todo |
| `face_domain` | 1033 | 5 | 23 | 42 | 2 | 4 | 3 | 1 | 0 | 14 | 2 | 0 |  | P2 | todo |
| `fillet_radius` | 1031 | 2 | 15 | 65 | 3 | 2 | 10 | 3 | 0 | 4 | 11 | 0 | C2 | P2 | todo |
| `watertight` | 965 | 3 | 15 | 72 | 3 | 6 | 0 | 3 | 0 | 10 | 0 | 0 |  | P2 | todo |
| `analytic_ss` | 933 | 1 | 21 | 33 | 2 | 3 | 5 | 0 | 96 | 4 | 0 | 0 | C2 | P2 | todo |
| `solid_audit` | 913 | 2 | 20 | 54 | 4 | 4 | 0 | 0 | 0 | 11 | 0 | 0 | C2 | P2 | todo |
| `source_face_contacts` | 912 | 2 | 10 | 169 | 2 | 2 | 3 | 0 | 0 | 21 | 2 | 0 | C1 | P1 | todo |
| `sweep_cap_contacts` | 863 | 2 | 10 | 68 | 2 | 3 | 1 | 2 | 0 | 7 | 1 | 0 | C3 | P2 | todo |
| `source_ruled_projection_contact` | 841 | 2 | 11 | 106 | 1 | 2 | 0 | 1 | 0 | 9 | 10 | 24 | C1 | P2 | todo |
| `model_identity` | 840 | 3 | 25 | 42 | 3 | 1 | 0 | 2 | 0 | 3 | 0 | 0 |  | P2 | todo |
| `analytic_boolean` | 837 | 3 | 23 | 36 | 1 | 3 | 0 | 0 | 0 | 5 | 1 | 0 | C2 | P2 | todo |
| `stepped_prism` | 816 | 3 | 24 | 72 | 2 | 3 | 0 | 0 | 0 | 6 | 0 | 0 |  | P2 | todo |
| `source_boundary_fragment` | 792 | 2 | 19 | 64 | 1 | 1 | 0 | 0 | 0 | 11 | 2 | 0 | C1 | P2 | todo |
| `trimmed_shell_recipe` | 789 | 1 | 11 | 57 | 1 | 1 | 0 | 0 | 0 | 6 | 1 | 0 |  | P2 | todo |
| `prism` | 779 | 1 | 21 | 82 | 2 | 3 | 0 | 1 | 0 | 13 | 0 | 0 | C2 | P2 | todo |
| `boundary_agreement` | 775 | 2 | 9 | 39 | 0 | 1 | 0 | 1 | 0 | 15 | 4 | 0 |  | P2 | todo |
| `feature_history` | 772 | 1 | 34 | 13 | 0 | 0 | 2 | 0 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `feature_graph` | 765 | 1 | 37 | 22 | 0 | 0 | 0 | 0 | 0 | 4 | 1 | 0 |  | P3 | todo |
| `nurbs_step_solid` | 752 | 1 | 13 | 57 | 2 | 1 | 0 | 1 | 94 | 3 | 0 | 0 |  | P2 | todo |
| `same_surface_uv_boolean` | 749 | 1 | 30 | 52 | 1 | 3 | 3 | 8 | 0 | 1 | 3 | 0 |  | P3 | todo |
| `source_contour_winding` | 747 | 1 | 17 | 95 | 1 | 1 | 2 | 0 | 0 | 3 | 3 | 0 | C1 | P2 | todo |
| `source_projective_edge` | 747 | 2 | 19 | 25 | 0 | 0 | 5 | 0 | 0 | 6 | 7 | 0 | C1 | P2 | todo |
| `linear_canal` | 735 | 1 | 14 | 18 | 0 | 0 | 0 | 0 | 0 | 7 | 0 | 0 | C1 | P3 | todo |
| `journal` | 722 | 2 | 36 | 6 | 0 | 0 | 0 | 0 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `worm` | 696 | 3 | 35 | 25 | 0 | 0 | 0 | 3 | 0 | 5 | 0 | 0 |  | P2 | todo |
| `material_chord` | 693 | 2 | 4 | 45 | 2 | 2 | 3 | 0 | 0 | 4 | 3 | 0 |  | P2 | todo |
| `rounded_patch_network` | 680 | 2 | 15 | 108 | 1 | 1 | 4 | 2 | 0 | 5 | 1 | 0 | C2 | P2 | todo |
| `trimmed_face_recipe` | 673 | 3 | 14 | 29 | 2 | 2 | 6 | 3 | 0 | 8 | 5 | 0 |  | P2 | todo |
| `aag` | 668 | 1 | 19 | 25 | 1 | 0 | 1 | 0 | 0 | 7 | 3 | 0 | C2 | P2 | todo |
| `aag_fillets` | 662 | 1 | 8 | 91 | 1 | 1 | 0 | 0 | 0 | 3 | 1 | 0 | C2 | P2 | todo |
| `source_wall_coverage` | 648 | 1 | 17 | 178 | 1 | 1 | 5 | 1 | 0 | 16 | 5 | 0 |  | P2 | todo |
| `imprint_assembly` | 637 | 1 | 24 | 41 | 0 | 0 | 2 | 0 | 0 | 7 | 0 | 0 |  | P2 | todo |
| `offset_edge_supports` | 635 | 3 | 24 | 36 | 2 | 1 | 5 | 4 | 0 | 5 | 2 | 0 |  | P2 | todo |
| `shell_distance` | 632 | 1 | 2 | 77 | 1 | 1 | 2 | 0 | 0 | 8 | 3 | 0 |  | P2 | todo |
| `solid_distance` | 624 | 1 | 2 | 49 | 1 | 1 | 0 | 0 | 0 | 7 | 2 | 0 |  | P2 | todo |
| `defects` | 594 | 1 | 22 | 33 | 1 | 1 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `face_injectivity` | 589 | 1 | 14 | 33 | 0 | 1 | 0 | 0 | 0 | 18 | 4 | 0 |  | P2 | todo |
| `source_projective_use` | 589 | 2 | 17 | 17 | 1 | 0 | 0 | 10 | 0 | 7 | 0 | 0 | C1 | P2 | todo |
| `model_validation` | 582 | 2 | 9 | 92 | 1 | 2 | 0 | 8 | 0 | 6 | 1 | 0 |  | P2 | todo |
| `(root)` | 558 | 1 | 9 | 5 | 0 | 0 | 0 | 0 | 0 | 144 | 0 | 0 |  | P2 | todo |
| `transactions` | 551 | 2 | 21 | 14 | 0 | 0 | 0 | 0 | 0 | 10 | 0 | 0 | C2 | P3 | todo |
| `boundary_contact_tube` | 538 | 1 | 5 | 89 | 2 | 2 | 2 | 7 | 0 | 2 | 1 | 0 |  | P2 | todo |
| `source_shared_chord_map` | 530 | 2 | 28 | 40 | 2 | 2 | 1 | 0 | 0 | 9 | 2 | 0 |  | P2 | todo |
| `step_part21` | 530 | 1 | 23 | 65 | 1 | 1 | 0 | 1 | 37 | 3 | 0 | 0 |  | P2 | todo |
| `algebraic_model` | 516 | 1 | 24 | 28 | 1 | 1 | 0 | 0 | 0 | 1 | 1 | 0 | C2 | P3 | todo |
| `planar_cap` | 516 | 2 | 12 | 30 | 0 | 0 | 6 | 0 | 0 | 5 | 2 | 0 |  | P2 | todo |
| `boolean_robustness` | 509 | 1 | 13 | 44 | 1 | 1 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `offset_end_boundary` | 509 | 1 | 3 | 10 | 0 | 0 | 3 | 2 | 0 | 3 | 4 | 0 |  | P3 | todo |
| `sweep_retained_decomposition` | 501 | 1 | 2 | 98 | 2 | 2 | 1 | 0 | 0 | 2 | 1 | 0 |  | P3 | todo |
| `retained_uv_boolean` | 497 | 1 | 19 | 55 | 0 | 1 | 2 | 3 | 0 | 1 | 4 | 0 |  | P3 | todo |
| `source_inverse_shear` | 497 | 1 | 8 | 21 | 1 | 1 | 0 | 8 | 31 | 7 | 0 | 0 | C1 | P2 | todo |
| `source_vertex_contact` | 495 | 2 | 16 | 86 | 1 | 1 | 0 | 2 | 0 | 7 | 1 | 0 | C1 | P2 | todo |
| `predicate_evidence` | 489 | 1 | 15 | 8 | 0 | 0 | 0 | 0 | 100 | 3 | 0 | 0 | C2 | P3 | todo |
| `face_curvature` | 488 | 1 | 10 | 65 | 1 | 1 | 6 | 0 | 0 | 3 | 5 | 2 |  | P3 | todo |
| `material_wall_coverage` | 485 | 1 | 5 | 66 | 1 | 1 | 1 | 4 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `prism_frame` | 483 | 1 | 21 | 69 | 1 | 2 | 0 | 2 | 0 | 2 | 0 | 0 |  | P2 | todo |
| `history_snapshots` | 480 | 1 | 20 | 11 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `sweep_retained` | 476 | 1 | 5 | 73 | 1 | 3 | 1 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `volume_validity` | 476 | 1 | 15 | 30 | 1 | 1 | 5 | 2 | 24 | 20 | 0 | 0 | C2 | P2 | todo |
| `cylinder_sphere_boolean` | 470 | 1 | 9 | 32 | 1 | 1 | 0 | 0 | 0 | 3 | 0 | 0 | C2 | P3 | todo |
| `nurbs_step_trimmed` | 466 | 1 | 7 | 9 | 0 | 0 | 0 | 1 | 100 | 4 | 0 | 0 |  | P3 | todo |
| `box_sphere_boolean` | 465 | 4 | 11 | 62 | 1 | 2 | 0 | 0 | 0 | 6 | 0 | 0 | C2 | P2 | todo |
| `contact_bind` | 463 | 1 | 7 | 71 | 2 | 2 | 3 | 0 | 0 | 1 | 2 | 0 |  | P3 | todo |
| `boundary_certificate` | 455 | 3 | 12 | 52 | 1 | 3 | 0 | 0 | 0 | 12 | 0 | 0 |  | P2 | todo |
| `contact_end` | 454 | 1 | 4 | 54 | 2 | 2 | 2 | 0 | 0 | 3 | 2 | 0 | C2 | P3 | todo |
| `body_edit` | 451 | 1 | 5 | 11 | 1 | 0 | 0 | 0 | 0 | 6 | 1 | 0 |  | P3 | todo |
| `source_hull_separation` | 450 | 1 | 12 | 42 | 1 | 1 | 0 | 2 | 0 | 5 | 1 | 0 | C1 | P3 | todo |
| `source_wall_pair` | 446 | 1 | 10 | 151 | 1 | 1 | 2 | 0 | 0 | 12 | 2 | 0 |  | P2 | todo |
| `ray_parity` | 442 | 2 | 3 | 53 | 1 | 1 | 3 | 0 | 0 | 4 | 0 | 0 |  | P3 | todo |
| `corner_close` | 437 | 1 | 4 | 110 | 2 | 2 | 3 | 5 | 0 | 3 | 3 | 0 |  | P2 | todo |
| `nurbs_ss_general` | 427 | 2 | 13 | 17 | 1 | 0 | 0 | 0 | 0 | 9 | 0 | 0 |  | P2 | todo |
| `coverage_verifier` | 418 | 2 | 9 | 15 | 0 | 0 | 1 | 0 | 84 | 10 | 0 | 0 | C2 | P2 | todo |
| `sweep_station_reconstruction` | 416 | 1 | 2 | 27 | 0 | 0 | 1 | 0 | 0 | 4 | 2 | 0 | C2 | P3 | todo |
| `source_wall_self_coverage` | 413 | 1 | 11 | 40 | 2 | 2 | 1 | 5 | 0 | 7 | 0 | 0 |  | P2 | todo |
| `graph_body` | 409 | 2 | 23 | 22 | 1 | 1 | 0 | 1 | 0 | 5 | 0 | 0 |  | P3 | todo |
| `face_contact_groups` | 402 | 1 | 6 | 46 | 1 | 1 | 1 | 0 | 0 | 4 | 0 | 0 |  | P2 | todo |
| `import_policy` | 402 | 1 | 10 | 18 | 0 | 0 | 0 | 0 | 0 | 1 | 3 | 0 |  | P3 | todo |
| `source_paired_fiber_contact` | 399 | 1 | 6 | 123 | 1 | 1 | 0 | 3 | 0 | 6 | 0 | 0 | C1 | P2 | todo |
| `affine_lattice` | 398 | 1 | 3 | 53 | 1 | 1 | 0 | 0 | 0 | 3 | 0 | 0 | C2 | P3 | todo |
| `source_allowed_contact` | 398 | 1 | 13 | 127 | 1 | 1 | 0 | 2 | 0 | 9 | 0 | 0 | C1 | P2 | todo |
| `source_mapped_edge` | 397 | 1 | 6 | 59 | 1 | 1 | 3 | 5 | 0 | 12 | 2 | 0 | C1 | P2 | todo |
| `source_exchange_endpoints` | 392 | 2 | 16 | 55 | 1 | 1 | 0 | 4 | 0 | 7 | 0 | 0 |  | P2 | todo |
| `sphere_boolean` | 375 | 1 | 6 | 26 | 0 | 0 | 1 | 0 | 0 | 6 | 0 | 0 | C2 | P3 | todo |
| `source_carrier_point` | 371 | 1 | 21 | 15 | 0 | 0 | 0 | 0 | 0 | 7 | 1 | 0 | C1 | P3 | todo |
| `sweep_miter_layout` | 367 | 1 | 7 | 12 | 0 | 0 | 1 | 0 | 0 | 3 | 1 | 0 |  | P3 | todo |
| `nurbs_step_interchange` | 364 | 1 | 5 | 9 | 0 | 0 | 0 | 0 | 100 | 3 | 0 | 0 |  | P3 | todo |
| `source_curved_boundary_contact` | 360 | 1 | 10 | 98 | 1 | 2 | 0 | 8 | 0 | 1 | 1 | 0 | C1 | P3 | todo |
| `uv_regions` | 359 | 1 | 10 | 54 | 1 | 1 | 0 | 0 | 0 | 5 | 0 | 0 |  | P2 | todo |
| `material_segment` | 355 | 1 | 4 | 35 | 1 | 1 | 1 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `source_body_model` | 351 | 1 | 12 | 31 | 1 | 1 | 0 | 2 | 0 | 5 | 1 | 0 |  | P3 | todo |
| `transform` | 351 | 1 | 5 | 22 | 0 | 0 | 0 | 0 | 0 | 8 | 0 | 0 | C2 | P2 | todo |
| `source_implicit_body` | 349 | 1 | 9 | 47 | 0 | 2 | 0 | 0 | 0 | 3 | 0 | 0 |  | P2 | todo |
| `source_pole_plane_image` | 343 | 1 | 6 | 72 | 1 | 1 | 0 | 1 | 0 | 2 | 0 | 0 | C1 | P3 | todo |
| `serialization` | 334 | 1 | 8 | 18 | 0 | 0 | 0 | 1 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `source_edge_restriction` | 322 | 1 | 12 | 10 | 0 | 0 | 0 | 0 | 0 | 6 | 1 | 0 | C1 | P3 | todo |
| `source_root_parameter` | 322 | 1 | 3 | 33 | 0 | 1 | 0 | 0 | 0 | 3 | 2 | 0 |  | P3 | todo |
| `shell_nesting` | 299 | 1 | 2 | 91 | 1 | 1 | 2 | 0 | 0 | 6 | 0 | 0 | C2 | P2 | todo |
| `trimmed_vertex_contact` | 298 | 1 | 3 | 110 | 1 | 1 | 0 | 1 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `source_region_restore` | 295 | 1 | 7 | 15 | 1 | 0 | 1 | 0 | 0 | 5 | 1 | 0 | C1 | P3 | todo |
| `ss_trimmed_face` | 295 | 3 | 37 | 36 | 0 | 2 | 0 | 0 | 0 | 6 | 0 | 0 |  | P2 | todo |
| `source_face_gap` | 294 | 1 | 8 | 67 | 1 | 1 | 0 | 1 | 0 | 7 | 2 | 0 |  | P2 | todo |
| `tolerance_unify` | 293 | 1 | 5 | 6 | 1 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `face_window` | 292 | 1 | 6 | 46 | 1 | 1 | 2 | 2 | 0 | 3 | 1 | 0 |  | P3 | todo |
| `profile_region` | 292 | 1 | 5 | 78 | 1 | 1 | 0 | 0 | 0 | 3 | 3 | 0 | C2 | P3 | todo |
| `transform_plane_charts` | 291 | 1 | 3 | 57 | 1 | 1 | 0 | 0 | 0 | 2 | 1 | 0 |  | P3 | todo |
| `shell_orientation` | 286 | 1 | 2 | 17 | 1 | 1 | 2 | 1 | 0 | 4 | 1 | 0 |  | P3 | todo |
| `material_wall` | 284 | 1 | 1 | 20 | 0 | 0 | 1 | 0 | 0 | 2 | 1 | 0 |  | P3 | todo |
| `miter_seams` | 283 | 1 | 6 | 44 | 0 | 2 | 0 | 0 | 0 | 5 | 2 | 0 |  | P2 | todo |
| `source_contact_point` | 283 | 1 | 12 | 13 | 1 | 0 | 0 | 2 | 0 | 6 | 1 | 0 |  | P3 | todo |
| `source_line_chart_root` | 282 | 1 | 1 | 42 | 1 | 1 | 0 | 2 | 0 | 2 | 1 | 0 | C1 | P3 | todo |
| `source_shell_restore` | 277 | 1 | 6 | 13 | 1 | 0 | 0 | 0 | 0 | 6 | 0 | 0 | C1 | P3 | todo |
| `face_crossings` | 276 | 1 | 6 | 25 | 1 | 1 | 2 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `source_box` | 276 | 1 | 10 | 39 | 1 | 1 | 0 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `face_contact` | 274 | 1 | 2 | 13 | 0 | 0 | 1 | 0 | 0 | 1 | 1 | 0 |  | P3 | todo |
| `source_shell_geometry` | 274 | 1 | 12 | 29 | 1 | 2 | 0 | 0 | 0 | 10 | 0 | 0 | C1 | P2 | todo |
| `wall_opposition` | 272 | 1 | 3 | 56 | 1 | 1 | 2 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `linear_canal_body` | 271 | 1 | 5 | 2 | 0 | 0 | 0 | 0 | 0 | 7 | 3 | 0 | C1 | P3 | todo |
| `self_intersection` | 269 | 2 | 3 | 35 | 1 | 1 | 0 | 0 | 0 | 4 | 0 | 0 | C3 | P3 | todo |
| `source_interior_fiber` | 269 | 1 | 4 | 32 | 1 | 1 | 1 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `source_paired_projection_contact` | 264 | 1 | 9 | 32 | 1 | 1 | 0 | 3 | 0 | 1 | 0 | 0 | C1 | P3 | todo |
| `source_wall_anchor` | 261 | 1 | 10 | 33 | 0 | 1 | 1 | 0 | 0 | 7 | 0 | 0 |  | P2 | todo |
| `source_plane_fiber` | 260 | 1 | 5 | 37 | 0 | 1 | 0 | 0 | 0 | 3 | 0 | 0 | C1 | P3 | todo |
| `radius_transition_body` | 257 | 1 | 4 | 19 | 1 | 1 | 0 | 1 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `source_pole_paired_contact` | 257 | 1 | 5 | 114 | 1 | 1 | 0 | 3 | 0 | 2 | 0 | 0 | C1 | P3 | todo |
| `source_affine_use` | 256 | 1 | 6 | 5 | 0 | 0 | 0 | 0 | 0 | 3 | 0 | 0 | C1 | P3 | todo |
| `sketch` | 254 | 1 | 5 | 22 | 0 | 1 | 0 | 0 | 0 | 3 | 0 | 0 | C2 | P3 | todo |
| `source_material_wall` | 251 | 1 | 7 | 17 | 0 | 0 | 3 | 5 | 0 | 4 | 3 | 0 |  | P3 | todo |
| `source_interior_boundary` | 241 | 1 | 1 | 42 | 1 | 1 | 0 | 0 | 0 | 3 | 1 | 0 | C1 | P3 | todo |
| `trimmed_edge_contact` | 241 | 1 | 1 | 123 | 1 | 1 | 0 | 1 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `source_material_chord` | 240 | 1 | 8 | 25 | 1 | 1 | 2 | 1 | 0 | 4 | 1 | 0 |  | P3 | todo |
| `topology_reference` | 239 | 1 | 3 | 20 | 0 | 0 | 0 | 1 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `trimmed_shell` | 229 | 1 | 4 | 36 | 1 | 2 | 0 | 1 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `source_profile_boundary_identity` | 228 | 1 | 6 | 27 | 0 | 1 | 0 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `source_exchange_step` | 227 | 1 | 3 | 29 | 1 | 1 | 0 | 0 | 77 | 3 | 0 | 0 |  | P3 | todo |
| `rational_cap_translation` | 222 | 1 | 5 | 25 | 0 | 2 | 0 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `wall_clearance` | 221 | 1 | 2 | 33 | 1 | 1 | 2 | 0 | 0 | 1 | 3 | 0 |  | P3 | todo |
| `source_interior_contact` | 214 | 1 | 5 | 56 | 1 | 1 | 1 | 1 | 0 | 2 | 0 | 0 | C1 | P3 | todo |
| `wall_search` | 209 | 1 | 6 | 32 | 0 | 1 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `source_fiber_boundary` | 202 | 1 | 2 | 40 | 1 | 1 | 0 | 1 | 0 | 3 | 0 | 0 | C1 | P3 | todo |
| `source_retained_face_gap` | 202 | 1 | 6 | 41 | 1 | 1 | 0 | 0 | 0 | 4 | 0 | 0 |  | P3 | todo |
| `rational_cap_affine` | 200 | 1 | 6 | 51 | 0 | 1 | 1 | 10 | 0 | 1 | 2 | 0 |  | P3 | todo |
| `contact_trim` | 199 | 1 | 3 | 24 | 1 | 1 | 1 | 2 | 0 | 1 | 0 | 0 | C2 | P3 | todo |
| `edge_tangency` | 199 | 1 | 2 | 13 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `face_replacement` | 199 | 1 | 3 | 48 | 1 | 1 | 1 | 0 | 0 | 1 | 1 | 0 |  | P3 | todo |
| `source_corner_plane_image` | 197 | 1 | 5 | 37 | 1 | 1 | 0 | 0 | 0 | 1 | 2 | 0 | C1 | P3 | todo |
| `source_fiber_contact` | 194 | 1 | 6 | 45 | 1 | 1 | 1 | 1 | 0 | 20 | 0 | 0 | C1 | P2 | todo |
| `boolean_support` | 189 | 1 | 7 | 22 | 0 | 1 | 1 | 0 | 0 | 6 | 2 | 0 |  | P3 | todo |
| `shell_relation` | 189 | 1 | 3 | 28 | 0 | 1 | 2 | 0 | 0 | 5 | 2 | 0 |  | P3 | todo |
| `source_vertex_links` | 176 | 1 | 2 | 58 | 1 | 1 | 0 | 0 | 0 | 3 | 0 | 0 | C1 | P3 | todo |
| `profile_imprint` | 175 | 1 | 5 | 23 | 0 | 1 | 0 | 0 | 0 | 5 | 0 | 0 | C2 | P3 | todo |
| `source_body_topology` | 174 | 1 | 4 | 22 | 1 | 0 | 0 | 1 | 0 | 3 | 1 | 0 |  | P3 | todo |
| `source_pole_planar_contact` | 171 | 1 | 5 | 58 | 1 | 1 | 0 | 1 | 0 | 2 | 0 | 0 | C1 | P3 | todo |
| `keypoints` | 163 | 1 | 4 | 35 | 0 | 1 | 0 | 0 | 0 | 2 | 3 | 0 |  | P3 | todo |
| `source_exchange_trims` | 159 | 1 | 9 | 36 | 1 | 1 | 0 | 0 | 0 | 3 | 1 | 0 |  | P3 | todo |
| `sweep_retained_walls` | 159 | 1 | 3 | 40 | 0 | 1 | 0 | 0 | 0 | 3 | 2 | 0 |  | P3 | todo |
| `source_root_refinement` | 156 | 1 | 4 | 18 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `source_projective_shell_restore` | 155 | 1 | 4 | 22 | 1 | 0 | 0 | 0 | 0 | 3 | 0 | 0 | C1 | P3 | todo |
| `source_planar_flux` | 146 | 1 | 3 | 35 | 1 | 1 | 1 | 1 | 0 | 2 | 0 | 0 | C1 | P3 | todo |
| `source_boundary_network` | 140 | 1 | 3 | 14 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `source_wall_search` | 138 | 1 | 1 | 38 | 1 | 1 | 1 | 1 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `source_collapsed_boundary` | 136 | 1 | 3 | 6 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | C1 | P3 | todo |
| `source_material_segment` | 134 | 1 | 2 | 35 | 1 | 1 | 1 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `source_support_shell` | 133 | 1 | 3 | 23 | 1 | 1 | 0 | 0 | 0 | 241 | 0 | 0 | C1 | P1 | todo |
| `profile_intersections` | 130 | 1 | 1 | 30 | 0 | 1 | 0 | 0 | 0 | 1 | 3 | 0 |  | P3 | todo |
| `sweep_affine_boundary` | 129 | 1 | 1 | 9 | 0 | 0 | 1 | 0 | 0 | 2 | 0 | 0 | C2 | P3 | todo |
| `source_world_wire` | 126 | 1 | 4 | 5 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | C1 | P3 | todo |
| `control_hull_separation` | 125 | 1 | 1 | 76 | 0 | 1 | 0 | 1 | 0 | 4 | 0 | 0 |  | P3 | todo |
| `source_planar_material_hull` | 125 | 1 | 6 | 34 | 0 | 1 | 0 | 0 | 0 | 2 | 0 | 0 | C1 | P3 | todo |
| `retained_uv_rim` | 122 | 1 | 2 | 54 | 1 | 1 | 0 | 0 | 0 | 1 | 1 | 0 |  | P3 | todo |
| `cuboid_primitive` | 117 | 1 | 1 | 11 | 1 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `rational_identity` | 117 | 1 | 6 | 4 | 0 | 0 | 0 | 5 | 0 | 2 | 2 | 0 |  | P3 | todo |
| `blade` | 114 | 1 | 3 | 14 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `sweep_retained_charts` | 114 | 1 | 4 | 19 | 0 | 0 | 0 | 0 | 0 | 3 | 1 | 0 |  | P3 | todo |
| `source_region_display` | 104 | 1 | 1 | 29 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | C1 | P3 | todo |
| `feature_family` | 101 | 1 | 2 | 9 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `source_wall_scan` | 99 | 1 | 3 | 15 | 0 | 0 | 1 | 0 | 0 | 2 | 1 | 0 |  | P3 | todo |
| `source_boundary_display` | 98 | 1 | 2 | 14 | 0 | 0 | 0 | 0 | 0 | 2 | 1 | 0 |  | P3 | todo |
| `operation_tolerance` | 68 | 1 | 3 | 2 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | C2 | P3 | todo |
| `source_halfplane_side` | 68 | 1 | 1 | 18 | 0 | 1 | 0 | 0 | 0 | 2 | 0 | 0 | C1 | P3 | todo |
| `source_shared_edge_restore` | 68 | 1 | 3 | 5 | 0 | 0 | 0 | 0 | 0 | 3 | 0 | 0 | C1 | P3 | todo |
| `source_projective_body_restore` | 59 | 1 | 3 | 5 | 0 | 0 | 0 | 0 | 0 | 3 | 0 | 0 | C1 | P3 | todo |
| `uv_coverage` | 56 | 1 | 2 | 15 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `sweep_retained_caps` | 54 | 1 | 1 | 37 | 0 | 1 | 0 | 0 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `source_body_restore` | 50 | 1 | 1 | 3 | 0 | 0 | 0 | 0 | 0 | 3 | 0 | 0 | C1 | P3 | todo |
| `face_senses` | 48 | 1 | 1 | 15 | 0 | 0 | 0 | 0 | 97 | 1 | 0 | 0 |  | P3 | todo |
| `exact_affine_source` | 41 | 1 | 3 | 10 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `rack` | 37 | 1 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `model_report` | 14 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `spatial_rmf_qualification` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 4 | 0 | 0 |  | P3 | todo |
| `sweep_boundary_owned_tests` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 21 | 0 | 0 |  | P3 | todo |
| `tests` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
