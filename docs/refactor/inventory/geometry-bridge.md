# Инвентаризация: `geometry-bridge`

Модулей верхнего уровня: 113. Модулей в циклах: 8 (8232 LOC из 38836).

Столбцы: LOC и Fn только по продакшен-коду (без `#[cfg(test)]`, tests/, examples/). `max cog` это максимальная когнитивная сложность функции в модуле. `Клоны %` это доля значимых строк, входящих в точные дубли длиной от 30 строк. `Churn 60д` это число коммитов за 60 дней по файлам модуля. `Dead` это элементы, которые rustc считает мёртвыми при `--force-warn dead_code`. `Цикл` это номер группы циклических зависимостей (SCC) между модулями верхнего уровня. Приоритет считается как churn x суммарная когнитивная сложность: верхние 10% это P1, следующие 25% это P2.

| Модуль | LOC | Файлов | Fn | max cog | Fn >100 | cy >15 | par >5 | unwrap | Клоны % | Churn 60д | Clippy | Dead | Цикл | Приоритет | Статус |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|:--:|:--:|---|
| `dispatch` | 3211 | 15 | 25 | 69 | 8 | 10 | 0 | 2 | 12 | 46 | 13 | 0 | C2 | P1 | todo |
| `brep` | 2033 | 6 | 48 | 51 | 1 | 10 | 1 | 9 | 5 | 17 | 2 | 1 |  | P1 | todo |
| `cad_lattice` | 1831 | 8 | 64 | 44 | 3 | 5 | 0 | 0 | 0 | 19 | 4 | 0 |  | P1 | todo |
| `sweep_pipeline` | 1688 | 5 | 40 | 56 | 1 | 6 | 0 | 17 | 0 | 23 | 6 | 0 | C2 | P1 | todo |
| `brep_identity` | 1426 | 5 | 35 | 120 | 4 | 4 | 1 | 0 | 0 | 9 | 0 | 3 |  | P2 | todo |
| `path2d` | 1407 | 6 | 24 | 40 | 3 | 3 | 0 | 0 | 0 | 20 | 0 | 0 |  | P2 | todo |
| `brep_production` | 1156 | 2 | 18 | 180 | 3 | 3 | 0 | 0 | 0 | 11 | 0 | 0 | C1 | P2 | todo |
| `abi` | 1015 | 2 | 46 | 10 | 0 | 0 | 12 | 0 | 0 | 25 | 2 | 0 |  | P2 | todo |
| `cad_miter_owned` | 976 | 2 | 15 | 46 | 4 | 3 | 0 | 1 | 0 | 9 | 11 | 0 |  | P2 | todo |
| `drawing_projection` | 954 | 2 | 12 | 101 | 3 | 5 | 1 | 11 | 0 | 4 | 7 | 0 |  | P2 | todo |
| `mesh_analysis` | 796 | 1 | 9 | 48 | 2 | 1 | 0 | 1 | 0 | 15 | 1 | 0 |  | P2 | todo |
| `mesh` | 788 | 1 | 19 | 174 | 1 | 1 | 0 | 2 | 0 | 18 | 0 | 0 |  | P2 | todo |
| `brep_envelope` | 772 | 1 | 13 | 22 | 1 | 2 | 0 | 0 | 0 | 7 | 0 | 0 | C1 | P2 | todo |
| `frame` | 709 | 1 | 16 | 7 | 0 | 0 | 0 | 12 | 0 | 4 | 0 | 0 |  | P3 | todo |
| `cad_face_contacts` | 673 | 1 | 18 | 24 | 1 | 0 | 0 | 1 | 0 | 27 | 3 | 0 |  | P2 | todo |
| `cad_profile_prepare` | 671 | 1 | 4 | 72 | 2 | 3 | 0 | 4 | 0 | 6 | 0 | 0 |  | P2 | todo |
| `brep_profile` | 620 | 1 | 6 | 84 | 1 | 1 | 0 | 0 | 0 | 11 | 2 | 0 |  | P2 | todo |
| `reconstruction` | 618 | 1 | 16 | 49 | 1 | 1 | 0 | 0 | 0 | 15 | 0 | 0 |  | P2 | todo |
| `brep_session` | 614 | 1 | 17 | 36 | 1 | 1 | 0 | 0 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `brep_provenance` | 612 | 1 | 13 | 26 | 0 | 0 | 0 | 0 | 0 | 6 | 0 | 0 |  | P2 | todo |
| `cad_sweep_boundary` | 587 | 4 | 12 | 35 | 3 | 1 | 2 | 0 | 0 | 17 | 2 | 0 |  | P2 | todo |
| `cad_source_body` | 540 | 1 | 6 | 150 | 1 | 1 | 0 | 2 | 10 | 27 | 0 | 0 |  | P2 | todo |
| `intersections` | 508 | 6 | 8 | 17 | 1 | 1 | 0 | 0 | 0 | 37 | 0 | 0 |  | P2 | todo |
| `brep_attestation` | 493 | 3 | 19 | 42 | 0 | 1 | 0 | 0 | 0 | 4 | 0 | 0 | C1 | P2 | todo |
| `cad_runout` | 482 | 4 | 11 | 18 | 1 | 1 | 0 | 0 | 0 | 6 | 0 | 0 |  | P3 | todo |
| `mesh_render` | 476 | 1 | 10 | 24 | 0 | 0 | 0 | 0 | 0 | 10 | 0 | 0 |  | P2 | todo |
| `step_lod0` | 458 | 1 | 11 | 23 | 0 | 0 | 0 | 0 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `polygon_station_body` | 431 | 2 | 4 | 29 | 0 | 1 | 0 | 0 | 0 | 2 | 3 | 0 | C2 | P3 | todo |
| `brep_semantic` | 400 | 1 | 8 | 62 | 1 | 1 | 0 | 1 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `cad_body_affine` | 391 | 1 | 10 | 11 | 0 | 0 | 0 | 1 | 0 | 6 | 1 | 0 |  | P3 | todo |
| `brep_session_abi` | 388 | 1 | 4 | 45 | 1 | 1 | 0 | 2 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `cad_texture` | 353 | 1 | 13 | 70 | 1 | 1 | 0 | 0 | 0 | 4 | 0 | 0 |  | P2 | todo |
| `sweep_cap_evidence` | 345 | 1 | 5 | 78 | 1 | 1 | 1 | 1 | 0 | 2 | 1 | 0 |  | P3 | todo |
| `step_scene` | 332 | 1 | 6 | 11 | 1 | 0 | 0 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `cad_display` | 309 | 1 | 6 | 39 | 0 | 1 | 0 | 0 | 0 | 5 | 5 | 0 |  | P2 | todo |
| `cad_expression_shell` | 309 | 1 | 4 | 61 | 1 | 1 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `cad_mesh_planes` | 309 | 1 | 14 | 46 | 0 | 1 | 0 | 0 | 0 | 4 | 0 | 0 |  | P2 | todo |
| `cad_sections` | 302 | 1 | 4 | 36 | 1 | 1 | 0 | 0 | 0 | 4 | 0 | 0 |  | P3 | todo |
| `topology_requests` | 297 | 1 | 6 | 4 | 1 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `brep_result` | 289 | 1 | 6 | 31 | 1 | 1 | 0 | 0 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `cad_sweep_smoothness` | 278 | 2 | 13 | 2 | 0 | 0 | 0 | 0 | 0 | 7 | 2 | 1 |  | P3 | todo |
| `print_geometry` | 266 | 1 | 2 | 30 | 0 | 1 | 0 | 0 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `cad_client_geometry` | 259 | 1 | 4 | 22 | 1 | 1 | 0 | 0 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `cad_clearance` | 255 | 1 | 7 | 53 | 0 | 1 | 0 | 0 | 0 | 5 | 0 | 0 |  | P2 | todo |
| `mesh_surface_groups` | 253 | 1 | 3 | 41 | 1 | 1 | 0 | 0 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `mesh_picking` | 247 | 1 | 7 | 9 | 0 | 0 | 0 | 0 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `brep_graph` | 244 | 1 | 11 | 8 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 | C3 | P3 | todo |
| `sdf_gpu` | 241 | 1 | 5 | 8 | 0 | 0 | 0 | 0 | 0 | 6 | 0 | 0 |  | P3 | todo |
| `brep_graph_runner` | 237 | 1 | 9 | 18 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | C3 | P3 | todo |
| `cad_diagnostics` | 217 | 1 | 9 | 14 | 0 | 0 | 0 | 0 | 0 | 4 | 1 | 0 |  | P3 | todo |
| `cad_selection` | 209 | 1 | 10 | 27 | 1 | 1 | 0 | 0 | 0 | 2 | 2 | 0 |  | P3 | todo |
| `sweep_viewport` | 207 | 1 | 9 | 55 | 1 | 1 | 1 | 1 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `(root)` | 199 | 1 | 6 | 3 | 0 | 0 | 0 | 0 | 0 | 86 | 0 | 0 |  | P2 | todo |
| `cad_solid_distance` | 198 | 1 | 2 | 0 | 0 | 0 | 0 | 0 | 0 | 9 | 0 | 0 |  | P3 | todo |
| `cad_hole` | 184 | 1 | 4 | 20 | 0 | 0 | 0 | 0 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `cad_miter_layout` | 184 | 1 | 7 | 0 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `cad_material_chord` | 164 | 1 | 3 | 4 | 0 | 0 | 1 | 0 | 0 | 3 | 1 | 0 |  | P3 | todo |
| `cad_pattern` | 154 | 1 | 3 | 36 | 0 | 1 | 0 | 1 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `cad_dimensions` | 144 | 1 | 1 | 59 | 1 | 1 | 0 | 0 | 0 | 4 | 0 | 0 |  | P3 | todo |
| `transparent_bsp` | 136 | 1 | 3 | 16 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `cad_face_selection` | 135 | 1 | 4 | 18 | 0 | 1 | 0 | 0 | 0 | 5 | 0 | 0 |  | P3 | todo |
| `cad_material_segment` | 126 | 1 | 3 | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `cad_surface_diagnostics` | 126 | 1 | 3 | 15 | 0 | 0 | 0 | 0 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `mesh_export_file` | 126 | 1 | 3 | 10 | 0 | 0 | 0 | 0 | 0 | 4 | 0 | 0 |  | P3 | todo |
| `native_snapshot` | 126 | 1 | 3 | 10 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `brep_display` | 122 | 1 | 2 | 24 | 0 | 0 | 0 | 0 | 0 | 4 | 0 | 0 |  | P3 | todo |
| `cad_sketch` | 120 | 1 | 6 | 3 | 0 | 0 | 0 | 0 | 0 | 4 | 0 | 0 |  | P3 | todo |
| `cad_split` | 120 | 1 | 2 | 16 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `brep_json_size` | 119 | 1 | 4 | 10 | 0 | 0 | 0 | 3 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `cad_planar_edit` | 117 | 1 | 1 | 16 | 0 | 0 | 0 | 0 | 0 | 5 | 0 | 0 |  | P3 | todo |
| `mesh_editor` | 115 | 1 | 1 | 9 | 0 | 1 | 0 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `cad_geometric_g2` | 111 | 1 | 6 | 18 | 0 | 0 | 0 | 1 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `cad_full_round_exact` | 109 | 1 | 7 | 3 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `cad_material_wall` | 106 | 1 | 2 | 1 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `cad_local_mesh_tools` | 102 | 1 | 4 | 9 | 0 | 0 | 0 | 0 | 0 | 4 | 0 | 0 |  | P3 | todo |
| `cad_draft` | 101 | 1 | 1 | 23 | 0 | 1 | 0 | 0 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `cad_bridge_curve` | 98 | 1 | 1 | 13 | 0 | 0 | 0 | 0 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `cad_full_round_history` | 97 | 1 | 4 | 5 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `cad_boolean` | 96 | 1 | 2 | 6 | 0 | 0 | 0 | 0 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `cad_whole_wall` | 90 | 1 | 1 | 1 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `cad_fibered_body` | 89 | 1 | 1 | 3 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `nurbs_tessellation` | 85 | 1 | 6 | 10 | 0 | 0 | 0 | 4 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `request_codec` | 84 | 1 | 11 | 1 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `cad_quantity` | 83 | 1 | 1 | 8 | 0 | 0 | 0 | 0 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `brep_execution_plan` | 78 | 1 | 1 | 8 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `cad_boundary_certificate` | 77 | 1 | 4 | 0 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `cad_boundary_agreement` | 75 | 1 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `cad_minkowski3d` | 75 | 1 | 3 | 18 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `brep_scene_plan` | 74 | 1 | 2 | 3 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `cad_shell_distance` | 74 | 1 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `viewport` | 74 | 1 | 1 | 20 | 0 | 1 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `scene_picking` | 71 | 1 | 2 | 13 | 0 | 0 | 0 | 0 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `cad_implicit_body` | 70 | 1 | 1 | 3 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `cad_sketch_trim` | 68 | 1 | 3 | 4 | 0 | 0 | 0 | 0 | 0 | 5 | 0 | 0 |  | P3 | todo |
| `cad_thread` | 63 | 1 | 2 | 11 | 0 | 0 | 0 | 0 | 0 | 4 | 0 | 0 |  | P3 | todo |
| `camera_gestures` | 59 | 1 | 2 | 3 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `mesh_import` | 57 | 1 | 1 | 17 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `cad_mesh_buffer` | 51 | 1 | 8 | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `cad_profile_tools` | 51 | 1 | 1 | 7 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `mesh_display` | 49 | 1 | 1 | 9 | 0 | 0 | 0 | 0 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `brep_exact_full_round` | 46 | 1 | 2 | 7 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `cad_body_keypoints` | 45 | 1 | 1 | 4 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 |  | P3 | todo |
| `cad_centered_lattice` | 43 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `cad_edge_edit` | 43 | 1 | 1 | 4 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `cad_sketch_offset` | 38 | 1 | 3 | 4 | 0 | 0 | 0 | 0 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `miter_smoothness` | 28 | 1 | 3 | 0 | 0 | 0 | 0 | 0 | 0 | 2 | 1 | 1 |  | P3 | todo |
| `cad_mesh_topology` | 25 | 1 | 2 | 0 | 0 | 0 | 0 | 0 | 0 | 6 | 0 | 0 |  | P3 | todo |
| `cad_boundary_regularity` | 21 | 1 | 2 | 2 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |  | P3 | todo |
| `cad_face_distance` | 21 | 1 | 1 | 1 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |  | P3 | todo |
| `cad_path` | 8 | 1 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 3 | 0 | 0 |  | P3 | todo |
| `lattice_gpu` | 2 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 11 | 0 | 0 |  | P3 | todo |
| `mesh_shell` | 2 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 0 | 0 |  | P3 | todo |
| `tests` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 3 | 0 | 0 |  | P3 | todo |
