# Дубликаты и мёртвые копии

Источник: `tools/refactor-audit/dupes.py` (точные клоны: совпадение нормализованных значимых строк, окно от 30 строк для Rust и от 20 для TypeScript) и разбор дерева модулей `tools/refactor-audit/audit-rs`.

## Rust: сводка

- Значимых строк: 873708. В точных клонах от 30 строк: 70847 (8.1%).
- Наборов полностью идентичных файлов: 19.

### Клоны по крейтам

| Крейт | Строк в клонах | Значимых строк | % |
|---|---:|---:|---:|
| `crates/brep-core` | 29861 | 375334 | 8.0 |
| `crates/nurbs-core` | 14257 | 280322 | 5.1 |
| `crates/cad-step` | 9116 | 13090 | 69.6 |
| `crates/nurbs-intersect` | 8156 | 9091 | 89.7 |
| `crates/brep-intersect` | 5667 | 8594 | 65.9 |
| `crates/geometry-bridge` | 1364 | 44848 | 3.0 |
| `crates/sketch-core` | 806 | 4446 | 18.1 |
| `crates/slicer-core` | 776 | 2405 | 32.3 |
| `crates/sdf-core` | 452 | 4511 | 10.0 |
| `crates/cad-predicates` | 210 | 25946 | 0.8 |
| `crates/mesh-topology` | 118 | 1681 | 7.0 |
| `crates/photogrammetry-core` | 32 | 14212 | 0.2 |
| `crates/photogrammetry-ffi` | 32 | 1449 | 2.2 |

### Масса клонов между крейтами

| A | B | Строк |
|---|---|---:|
| `crates/brep-core` | `crates/cad-step` | 9116 |
| `crates/brep-core` | `crates/nurbs-intersect` | 8157 |
| `crates/nurbs-core` | `crates/nurbs-core` | 7193 |
| `crates/brep-core` | `crates/brep-intersect` | 5723 |
| `crates/brep-core` | `crates/brep-core` | 4599 |
| `crates/geometry-bridge` | `crates/geometry-bridge` | 1059 |
| `crates/sketch-core` | `crates/sketch-core` | 403 |
| `crates/slicer-core` | `crates/slicer-core` | 388 |
| `crates/brep-core` | `crates/geometry-bridge` | 352 |
| `crates/sdf-core` | `crates/sdf-core` | 226 |
| `crates/cad-predicates` | `crates/cad-predicates` | 105 |
| `crates/brep-core` | `crates/nurbs-core` | 86 |
| `crates/mesh-topology` | `crates/mesh-topology` | 59 |
| `crates/brep-intersect` | `crates/brep-intersect` | 38 |
| `crates/photogrammetry-core` | `crates/photogrammetry-ffi` | 32 |

### 80 крупнейших клонов

| Строк | Копия A | Копия B |
|---:|---|---|
| 920 | `crates/brep-core/src/intersections/tests/tests_integration.rs:1997` (тест) | `crates/nurbs-intersect/src/queries/tests/mod.rs:1253` (тест) |
| 629 | `crates/nurbs-core/src/construct/surfaces/gordon/cartesian.rs:3` (прод) | `crates/nurbs-core/src/gordon_cartesian.rs:3` (не компилируется) |
| 584 | `crates/brep-core/src/tests/step_interchange_v3.rs:63` (тест) | `crates/cad-step/src/tests/step_interchange_v3.rs:60` (тест) |
| 576 | `crates/nurbs-core/src/construct/surfaces/loft/guided_loft.rs:4` (прод) | `crates/nurbs-core/src/guided_loft.rs:4` (не компилируется) |
| 513 | `crates/brep-core/src/tests/step_interchange_v3.rs:1453` (тест) | `crates/cad-step/src/tests/step_interchange_v3.rs:1203` (тест) |
| 438 | `crates/nurbs-core/src/offset/surface_offset/tests/source.rs:2` (тест) | `crates/nurbs-core/src/surface_offset/tests/mod.rs:2` (не компилируется) |
| 417 | `crates/nurbs-core/src/construct/surfaces/loft/alignment.rs:3` (прод) | `crates/nurbs-core/src/loft_alignment.rs:3` (не компилируется) |
| 374 | `crates/brep-core/src/step_interchange.rs:1260` (прод) | `crates/cad-step/src/step_interchange/export_step.rs:61` (прод) |
| 359 | `crates/brep-core/src/analytic_ss.rs:99` (прод) | `crates/nurbs-intersect/src/analytic_ss/analytic_ss_component.rs:69` (прод) |
| 347 | `crates/brep-core/src/sweep_miter_owned/tests/direct_source_regressions_integration.rs:794` (тест) | `crates/brep-core/src/tests/sweep_miter_owned.rs:132` (тест) |
| 341 | `crates/brep-core/src/predicate_evidence.rs:7` (прод) | `crates/nurbs-intersect/src/predicate_evidence.rs:7` (прод) |
| 334 | `crates/brep-core/src/intersections/tests/tests_integration.rs:878` (тест) | `crates/nurbs-intersect/src/queries/tests/mod.rs:445` (тест) |
| 321 | `crates/brep-core/src/intersections.rs:617` (прод) | `crates/nurbs-intersect/src/queries/surface_trace.rs:3` (прод) |
| 318 | `crates/brep-core/src/nurbs_step_shared/linked_graph.rs:226` (прод) | `crates/cad-step/src/nurbs_step_shared/validate_curve_pcurve_correspondence.rs:3` (прод) |
| 290 | `crates/nurbs-core/src/construct/surfaces/loft/natural_loft.rs:105` (прод) | `crates/nurbs-core/src/natural_loft.rs:102` (не компилируется) |
| 284 | `crates/brep-core/src/nurbs_step_solid.rs:23` (прод) | `crates/cad-step/src/nurbs_step_solid.rs:23` (прод) |
| 278 | `crates/brep-core/src/nurbs_step_trimmed.rs:8` (прод) | `crates/cad-step/src/nurbs_step_trimmed.rs:8` (прод) |
| 276 | `crates/brep-core/src/intersections.rs:2370` (прод) | `crates/nurbs-intersect/src/queries/curve_segment.rs:6` (прод) |
| 274 | `crates/brep-core/src/intersections.rs:4103` (прод) | `crates/nurbs-intersect/src/queries/ruled_coincidence.rs:100` (прод) |
| 273 | `crates/brep-core/src/nurbs_ss_g6/join_certified_multispan_fragments.rs:242` (прод) | `crates/nurbs-intersect/src/nurbs_ss/join_certified_multispan_fragments.rs:242` (прод) |
| 269 | `crates/nurbs-core/src/construct/curves/paths/round_polyline.rs:12` (прод) | `crates/nurbs-core/src/paths/round_polyline.rs:10` (не компилируется) |
| 262 | `crates/brep-core/src/intersections.rs:3036` (прод) | `crates/nurbs-intersect/src/queries/curve_curve.rs:120` (прод) |
| 256 | `crates/brep-core/src/intersections/plane_cylinder.rs:275` (прод) | `crates/brep-intersect/src/plane_cylinder/plane_cylinder_component.rs:210` (прод) |
| 250 | `crates/nurbs-core/src/construct/surfaces/gordon/tangent_audit.rs:4` (прод) | `crates/nurbs-core/src/gordon_tangent_audit.rs:4` (не компилируется) |
| 249 | `crates/brep-core/src/step_interchange.rs:53` (прод) | `crates/cad-step/src/step_interchange/step_identity_report.rs:19` (прод) |
| 237 | `crates/brep-core/src/nurbs_step_shared.rs:8` (прод) | `crates/cad-step/src/nurbs_step_shared/refuse.rs:3` (прод) |
| 234 | `crates/nurbs-core/src/construct/curves/paths/miter_sections.rs:12` (прод) | `crates/nurbs-core/src/paths/miter_sections.rs:12` (не компилируется) |
| 233 | `crates/brep-core/src/intersections/serialization.rs:4` (прод) | `crates/nurbs-intersect/src/queries/codec.rs:2` (прод) |
| 232 | `crates/brep-core/src/intersections.rs:2060` (прод) | `crates/nurbs-intersect/src/queries/surface_surface.rs:39` (прод) |
| 221 | `crates/brep-core/src/nurbs_ss_g6/tests/mod.rs:994` (тест) | `crates/nurbs-intersect/src/nurbs_ss/tests/mod.rs:496` (тест) |
| 209 | `crates/brep-core/src/step_interchange_v3.rs:2710` (прод) | `crates/cad-step/src/step_interchange_v3/compose_step_v8_occurrences.rs:144` (прод) |
| 206 | `crates/nurbs-core/src/construct/surfaces/gordon.rs:118` (прод) | `crates/nurbs-core/src/gordon.rs:117` (не компилируется) |
| 201 | `crates/brep-core/src/intersections/sphere_torus.rs:68` (прод) | `crates/brep-intersect/src/sphere_torus.rs:67` (прод) |
| 201 | `crates/brep-core/src/intersections.rs:316` (прод) | `crates/nurbs-intersect/src/queries/curve_plane.rs:73` (прод) |
| 198 | `crates/brep-core/src/step_v3_export/algorithm_periodicized_sphere.rs:4` (прод) | `crates/cad-step/src/step_interchange_v3/quadratic_patch_knots.rs:149` (прод) |
| 197 | `crates/brep-core/src/nurbs_step_interchange.rs:9` (прод) | `crates/cad-step/src/nurbs_step_interchange.rs:9` (прод) |
| 197 | `crates/brep-core/src/nurbs_ss_g6/certify_exact_planar_iso_intersection.rs:8` (прод) | `crates/nurbs-intersect/src/nurbs_ss/certify_exact_planar_iso_intersection.rs:8` (прод) |
| 197 | `crates/brep-core/src/intersections.rs:1760` (прод) | `crates/nurbs-intersect/src/queries/curve_surface.rs:3` (прод) |
| 190 | `crates/brep-core/src/step_interchange.rs:885` (прод) | `crates/cad-step/src/step_interchange/step_writer.rs:280` (прод) |
| 182 | `crates/nurbs-core/src/curve_offset.rs:270` (не компилируется) | `crates/nurbs-core/src/offset/tests/curve_offset.rs:3` (тест) |
| 178 | `crates/brep-core/src/coverage_verifier.rs:132` (прод) | `crates/nurbs-intersect/src/coverage_verifier.rs:86` (прод) |
| 177 | `crates/brep-core/src/intersections/cylinder_torus.rs:106` (прод) | `crates/brep-intersect/src/cylinder_torus/cylinder_torus_component.rs:34` (прод) |
| 176 | `crates/brep-core/src/step_interchange.rs:1854` (прод) | `crates/cad-step/src/step_interchange/tests/mod.rs:5` (тест) |
| 175 | `crates/nurbs-core/src/construct/curves/paths.rs:5` (прод) | `crates/nurbs-core/src/paths.rs:5` (не компилируется) |
| 172 | `crates/brep-core/src/intersections/plane_cone.rs:1221` (прод) | `crates/brep-intersect/src/plane_cone/intersect_plane_cone.rs:273` (прод) |
| 170 | `crates/brep-core/src/intersections.rs:1402` (прод) | `crates/nurbs-intersect/src/queries/surface_plane.rs:367` (прод) |
| 168 | `crates/brep-core/src/intersections/plane_torus.rs:568` (прод) | `crates/brep-intersect/src/plane_torus/torus_patch_curve.rs:218` (прод) |
| 165 | `crates/brep-core/src/iges_interchange_v2.rs:2475` (прод) | `crates/cad-step/src/iges_interchange_v2/export_iges_v2.rs:37` (прод) |
| 162 | `crates/nurbs-core/src/analysis/surface_measure.rs:14` (прод) | `crates/nurbs-core/src/surface_measure.rs:13` (не компилируется) |
| 160 | `crates/brep-core/src/nurbs_ss_g6/g6_maturity.rs:304` (прод) | `crates/nurbs-intersect/src/nurbs_ss/refuse.rs:268` (прод) |
| 156 | `crates/brep-core/src/intersections/tests/tests_integration.rs:3` (тест) | `crates/nurbs-intersect/src/queries/tests/mod.rs:3` (тест) |
| 154 | `crates/brep-core/src/intersections/cone_torus.rs:179` (прод) | `crates/brep-intersect/src/cone_torus/cone_torus_component.rs:89` (прод) |
| 151 | `crates/slicer-core/src/infill.rs:182` (прод) | `crates/slicer-core/src/infill_planning.rs:5` (не компилируется) |
| 151 | `crates/brep-core/src/step_v3_export.rs:281` (прод) | `crates/cad-step/src/step_interchange_v3/export_step_v3.rs:11` (прод) |
| 149 | `crates/nurbs-core/src/construct/curves/hermite.rs:10` (прод) | `crates/nurbs-core/src/hermite.rs:10` (не компилируется) |
| 149 | `crates/brep-core/src/intersections.rs:4806` (прод) | `crates/nurbs-intersect/src/queries/ruled_points.rs:261` (прод) |
| 146 | `crates/nurbs-core/src/construct/surfaces/gordon/tangent_fields.rs:7` (прод) | `crates/nurbs-core/src/gordon_tangent_fields.rs:5` (не компилируется) |
| 145 | `crates/nurbs-core/src/offset/surface_offset/source.rs:10` (прод) | `crates/nurbs-core/src/surface_offset/bounds.rs:4` (не компилируется) |
| 145 | `crates/brep-core/src/intersections.rs:5006` (прод) | `crates/nurbs-intersect/src/queries/curve_ruled_surface.rs:345` (прод) |
| 144 | `crates/sdf-core/src/differential_and_queries.rs:15` (прод) | `crates/sdf-core/src/surface_polygonization.rs:8` (не компилируется) |
| 144 | `crates/brep-core/src/intersections.rs:4474` (прод) | `crates/nurbs-intersect/src/queries/curve_ruled_surface.rs:127` (прод) |
| 142 | `crates/brep-core/src/nurbs_ss_g6/join_certified_multispan_fragments.rs:4` (прод) | `crates/nurbs-intersect/src/nurbs_ss/join_certified_multispan_fragments.rs:4` (прод) |
| 138 | `crates/slicer-core/src/adhesion_and_support.rs:110` (прод) | `crates/slicer-core/src/infill_planning.rs:328` (не компилируется) |
| 138 | `crates/brep-core/src/step_interchange_v3.rs:554` (прод) | `crates/cad-step/src/step_interchange_v3/analytic_surface.rs:4` (прод) |
| 138 | `crates/brep-core/src/intersections.rs:3448` (прод) | `crates/nurbs-intersect/src/queries/curve_coincidence.rs:57` (прод) |
| 138 | `crates/brep-core/src/analytic_ss.rs:595` (прод) | `crates/nurbs-intersect/src/analytic_ss/cylinder_sphere.rs:6` (прод) |
| 136 | `crates/brep-core/src/nurbs_ss_g6/tests/mod.rs:566` (тест) | `crates/nurbs-intersect/src/nurbs_ss/tests/mod.rs:304` (тест) |
| 134 | `crates/nurbs-core/src/curve_offset.rs:45` (не компилируется) | `crates/nurbs-core/src/offset/curve_offset.rs:49` (прод) |
| 133 | `crates/brep-core/src/nurbs_ss_g6/tests/mod.rs:4` (тест) | `crates/nurbs-intersect/src/nurbs_ss/tests/mod.rs:16` (тест) |
| 133 | `crates/brep-core/src/intersections/tests/tests_integration.rs:1398` (тест) | `crates/nurbs-intersect/src/queries/tests/mod.rs:872` (тест) |
| 132 | `crates/geometry-bridge/src/dispatch/nurbs.rs:372` (прод) | `crates/geometry-bridge/src/dispatch/sweep_profile.rs:6` (прод) |
| 131 | `crates/nurbs-core/src/construct/surfaces/gordon/denominators.rs:5` (прод) | `crates/nurbs-core/src/gordon_denominators.rs:5` (не компилируется) |
| 129 | `crates/nurbs-core/src/construct/curves/primitives/conics.rs:7` (прод) | `crates/nurbs-core/src/primitives.rs:46` (не компилируется) |
| 129 | `crates/brep-core/src/intersections/cone_cone.rs:160` (прод) | `crates/brep-intersect/src/cone_cone/cone_cone_component.rs:112` (прод) |
| 128 | `crates/nurbs-core/src/tests/mod.rs:963` (тест) | `crates/nurbs-core/tests/ss_coverage.rs:182` (тест) |
| 128 | `crates/brep-core/src/step_interchange.rs:2081` (прод) | `crates/cad-step/src/step_interchange/tests/mod.rs:232` (тест) |
| 128 | `crates/brep-core/src/intersections/tests/tests_integration.rs:1718` (тест) | `crates/nurbs-intersect/src/queries/tests/mod.rs:1039` (тест) |
| 128 | `crates/brep-core/src/intersections/cylinder_cylinder.rs:154` (прод) | `crates/brep-intersect/src/cylinder_cylinder/cylinder_cylinder_component.rs:115` (прод) |
| 127 | `crates/brep-core/src/intersections/plane_sphere.rs:419` (прод) | `crates/brep-intersect/src/plane_sphere/plane_patch_curve.rs:245` (прод) |
| 126 | `crates/brep-core/src/nurbs_ss_g6/certify_exact_planar_iso_intersection.rs:320` (прод) | `crates/nurbs-intersect/src/nurbs_ss/certify_exact_planar_iso_intersection.rs:320` (прод) |

## Rust: функции, текстуально идентичные в двух компилируемых файлах

Сравнение полного текста функции без пробелов. Тесты из этого списка выполняются дважды.

| Идентичных fn | из них в тестовом коде | LOC | Файл A | Файл B |
|---:|---:|---:|---|---|
| 7 | 1 | 72 | `crates/brep-core/src/minkowski3d/rounding.rs` | `crates/brep-core/src/minkowski3d/rounding_legacy.rs` |
| 7 | 7 | 475 | `crates/brep-core/src/sweep_miter_owned/tests/direct_source_regressions_integration.rs` | `crates/brep-core/src/tests/sweep_miter_owned.rs` |
| 5 | 5 | 76 | `crates/geometry-bridge/tests/support/fibered_source_reference.rs` | `crates/geometry-bridge/tests/support/rational_cap_flux.rs` |
| 4 | 4 | 52 | `crates/brep-core/src/profile_certificate/winding/periodic/multispan.rs` | `crates/brep-core/tests/periodic_multispan_regions.rs` |
| 4 | 4 | 70 | `crates/cad-predicates/tests/construction3_recipes.rs` | `crates/cad-predicates/tests/construction_recipes.rs` |
| 3 | 3 | 52 | `crates/brep-core/examples/support/contact_graft_fixture.rs` | `crates/brep-core/tests/contact_bind.rs` |
| 3 | 0 | 22 | `crates/brep-core/src/analytic/rational_loft.rs` | `crates/brep-core/src/analytic/rational_loft_legacy.rs` |
| 3 | 3 | 42 | `crates/brep-core/src/boundary_embedding/tests/incremental_tests.rs` | `crates/brep-core/src/boundary_embedding/tests/mod.rs` |
| 3 | 3 | 50 | `crates/brep-core/tests/offset_cap_conform.rs` | `crates/brep-core/tests/offset_cap_partition.rs` |
| 3 | 3 | 50 | `crates/brep-core/tests/offset_cap_conform.rs` | `crates/brep-core/tests/offset_cap_regions.rs` |
| 3 | 3 | 50 | `crates/brep-core/tests/offset_cap_conform.rs` | `crates/brep-core/tests/offset_cap_sewing.rs` |
| 3 | 3 | 50 | `crates/brep-core/tests/offset_cap_partition.rs` | `crates/brep-core/tests/offset_cap_regions.rs` |
| 3 | 3 | 50 | `crates/brep-core/tests/offset_cap_partition.rs` | `crates/brep-core/tests/offset_cap_sewing.rs` |
| 3 | 3 | 50 | `crates/brep-core/tests/offset_cap_regions.rs` | `crates/brep-core/tests/offset_cap_sewing.rs` |
| 3 | 3 | 41 | `crates/nurbs-core/tests/offset_boundary_contact.rs` | `crates/nurbs-core/tests/offset_contact_chain.rs` |
| 3 | 3 | 41 | `crates/nurbs-core/tests/offset_boundary_contact.rs` | `crates/nurbs-core/tests/offset_contact_region.rs` |
| 3 | 3 | 41 | `crates/nurbs-core/tests/offset_boundary_contact.rs` | `crates/nurbs-core/tests/offset_endpoint_attachment.rs` |
| 3 | 3 | 48 | `crates/nurbs-core/tests/offset_cap.rs` | `crates/nurbs-core/tests/offset_nested_region.rs` |
| 3 | 3 | 41 | `crates/nurbs-core/tests/offset_contact_chain.rs` | `crates/nurbs-core/tests/offset_contact_region.rs` |
| 3 | 3 | 41 | `crates/nurbs-core/tests/offset_contact_chain.rs` | `crates/nurbs-core/tests/offset_endpoint_attachment.rs` |
| 3 | 3 | 41 | `crates/nurbs-core/tests/offset_contact_region.rs` | `crates/nurbs-core/tests/offset_endpoint_attachment.rs` |
| 3 | 0 | 56 | `crates/nurbs-core/src/construct/surfaces/gordon/tangent_audit.rs` | `crates/nurbs-core/src/reparameterization_residual.rs` |
| 3 | 3 | 25 | `crates/nurbs-core/src/intersection/intersect_curve_surface_report.rs` | `crates/nurbs-core/src/intersection/original_curve_exclusion_tests/mod.rs` |
| 2 | 2 | 61 | `crates/brep-core/examples/corner_patch_network_diagnostics.rs` | `crates/brep-core/tests/nurbs_edge_transition.rs` |
| 2 | 2 | 22 | `crates/brep-core/examples/support/contact_graft_fixture.rs` | `crates/brep-core/tests/contact_end.rs` |
| 2 | 2 | 22 | `crates/brep-core/examples/support/contact_graft_fixture.rs` | `crates/brep-core/tests/contact_trim.rs` |
| 2 | 2 | 22 | `crates/brep-core/tests/contact_bind.rs` | `crates/brep-core/tests/contact_end.rs` |
| 2 | 2 | 22 | `crates/brep-core/tests/contact_bind.rs` | `crates/brep-core/tests/contact_trim.rs` |
| 2 | 2 | 22 | `crates/brep-core/tests/contact_end.rs` | `crates/brep-core/tests/contact_trim.rs` |
| 2 | 2 | 20 | `crates/brep-core/src/box_sphere_boolean/tests/mod.rs` | `crates/brep-core/src/cylinder_sphere_boolean.rs` |
| 2 | 2 | 18 | `crates/brep-core/src/intersections/cone_torus.rs` | `crates/brep-core/src/intersections/torus_torus.rs` |
| 2 | 0 | 31 | `crates/brep-core/src/intersections/plane_cone.rs` | `crates/brep-core/src/intersections/plane_cylinder.rs` |
| 2 | 2 | 19 | `crates/brep-core/src/intersections/tests/tests_integration.rs` | `crates/brep-core/tests/intersection_engine_on_models.rs` |
| 2 | 0 | 15 | `crates/brep-core/src/source_retained_face_gap.rs` | `crates/brep-core/src/source_wall_pair.rs` |
| 2 | 0 | 14 | `crates/brep-core/src/projected_trim_contact.rs` | `crates/brep-core/src/same_support_contact.rs` |
| 2 | 0 | 16 | `crates/brep-core/src/ss_face_membership/closed/annulus.rs` | `crates/brep-core/src/ss_face_membership/closed/boolean.rs` |
| 2 | 2 | 25 | `crates/brep-core/tests/full_round_cap_pullback.rs` | `crates/brep-core/tests/full_round_source_caps.rs` |
| 2 | 2 | 43 | `crates/brep-core/tests/offset_cap_automatic_regions.rs` | `crates/brep-core/tests/offset_cap_regions.rs` |
| 2 | 2 | 20 | `crates/brep-core/tests/full_round_exact_history.rs` | `crates/brep-core/tests/full_round_history_regenerate.rs` |
| 2 | 2 | 35 | `crates/brep-core/tests/intersected_pair_thickness.rs` | `crates/brep-core/tests/mapped_network_thickness.rs` |
| 2 | 2 | 18 | `crates/brep-intersect/src/cone_torus/tests/mod.rs` | `crates/brep-intersect/src/torus_torus/tests/mod.rs` |
| 2 | 2 | 15 | `crates/nurbs-core/tests/exact_area_normal_contracts.rs` | `crates/nurbs-core/tests/exact_metric_field_contracts.rs` |
| 2 | 2 | 32 | `crates/nurbs-core/tests/offset_boundary_contact.rs` | `crates/nurbs-core/tests/offset_contact_curve_fit.rs` |
| 2 | 2 | 32 | `crates/nurbs-core/tests/offset_boundary_contact.rs` | `crates/nurbs-core/tests/offset_contact_world_curve.rs` |
| 2 | 2 | 32 | `crates/nurbs-core/tests/offset_boundary_contact.rs` | `crates/nurbs-core/tests/offset_uv_regions.rs` |

## Rust: некомпилируемые файлы

Файл считается некомпилируемым, если он не достижим из корня ни одной цели Cargo (lib, bin, test, example, bench) через `mod`, `#[path]` или `include!`.

### Доказуемо устаревшие копии, кандидаты на удаление (0 файлов, 0 LOC)

Критерии: файл в индексе git и не изменён; не менее 80% его строк есть в компилируемом файле того же крейта; каждое имя функции из него существует в живом коде крейта; путь к нему не упоминается в отслеживаемых не-Rust файлах; он не дочерний модуль оставленного файла. Удалённое ранее перечислено в `removed-*.md`.

| Файл | LOC | Доля строк в живом файле | Живой двойник |
|---|---:|---:|---|

### Оставлено до решения владельца (39 файлов, 13464 LOC)

| Файл | LOC | Почему не удалён автоматически |
|---|---:|---|
| `crates/nurbs-core/src/gordon_cartesian.rs` | 774 | child module of kept file crates/nurbs-core/src/gordon.rs |
| `crates/nurbs-core/src/guided_loft.rs` | 738 | path referenced from tracked non-Rust files |
| `crates/nurbs-core/src/surface_offset/bounds.rs` | 591 | child module of kept file crates/nurbs-core/src/surface_offset/mod.rs |
| `crates/nurbs-core/src/sweeps/progressive_sweep/authored_error/transport.rs` | 590 | only 0.55 of lines exist in a compiled file |
| `crates/nurbs-core/src/ss_intersection/grids_excluded_ss.rs` | 565 | 1 fn(s) with no live namesake: proportional_grids; only 0.50 of lines exist in a compiled file |
| `crates/nurbs-core/src/curve_offset.rs` | 559 | only 0.53 of lines exist in a compiled file |
| `crates/nurbs-core/src/surface_offset/tests/mod.rs` | 557 | child module of kept file crates/nurbs-core/src/surface_offset/mod.rs |
| `crates/nurbs-core/src/ss_intersection/intersect_surface_surface.rs` | 531 | 1 fn(s) with no live namesake: encode_ss_report; only 0.34 of lines exist in a compiled file |
| `crates/nurbs-core/src/loft_alignment.rs` | 514 | path referenced from tracked non-Rust files |
| `crates/nurbs-core/src/natural_loft.rs` | 499 | path referenced from tracked non-Rust files |
| `crates/nurbs-core/src/curve_measure.rs` | 472 | path referenced from tracked non-Rust files |
| `crates/nurbs-core/src/loft_continuity.rs` | 422 | 2 fn(s) with no live namesake: prepared, matched; only 0.78 of lines exist in a compiled file; path referenced from tracked non-Rust files |
| `crates/nurbs-core/src/gordon.rs` | 410 | path referenced from tracked non-Rust files |
| `crates/nurbs-core/src/ss_intersection/refine_seed.rs` | 403 | only 0.51 of lines exist in a compiled file |
| `crates/nurbs-core/src/paths/round_polyline.rs` | 356 | child module of kept file crates/nurbs-core/src/paths.rs |
| `crates/nurbs-core/src/chord_arrangement.rs` | 352 | 3 fn(s) with no live namesake: three_exact_crossings_share_one_vertex_without_collapsed_cuts, nonbinary_concurrent_intersections_share_an_exact_recipe, concurrent_closed_profile_materializes_connected_filled_loops; only 0.78 of lines exist in a compiled file |
| `crates/nurbs-core/src/surface_measure/jets.rs` | 307 | path referenced from tracked non-Rust files |
| `crates/nurbs-core/src/paths/miter_sections.rs` | 302 | child module of kept file crates/nurbs-core/src/paths.rs |
| `crates/nurbs-core/src/gordon_tangent_audit.rs` | 295 | path referenced from tracked non-Rust files |
| `crates/nurbs-core/src/curve_offset_diagnostics.rs` | 284 | only 0.53 of lines exist in a compiled file |
| `crates/nurbs-core/src/paths.rs` | 232 | path referenced from tracked non-Rust files |
| `crates/nurbs-core/src/primitives.rs` | 225 | 1 fn(s) with no live namesake: full_ellipse_and_signed_quadrants_share_authored_axis_coefficients; only 0.77 of lines exist in a compiled file; path referenced from tracked non-Rust files |
| `crates/nurbs-core/src/surface_measure.rs` | 222 | path referenced from tracked non-Rust files |
| `crates/nurbs-core/src/construct/curves/airfoil/cst.rs` | 216 | only 0.26 of lines exist in a compiled file |
| `crates/nurbs-core/src/chord_faces.rs` | 211 | 1 fn(s) with no live namesake: adjacent_binary64_rays_have_exact_angular_order |
| `crates/nurbs-core/src/hermite.rs` | 192 | path referenced from tracked non-Rust files |
| `crates/nurbs-core/src/gordon_tangent_fields.rs` | 188 | path referenced from tracked non-Rust files |
| `crates/nurbs-core/src/gordon_denominators.rs` | 171 | child module of kept file crates/nurbs-core/src/gordon.rs |
| `crates/nurbs-core/src/curve_jets.rs` | 169 | path referenced from tracked non-Rust files |
| `crates/nurbs-core/src/affine.rs` | 129 | path referenced from tracked non-Rust files |
| `crates/nurbs-core/src/surface_offset/intersection_candidates.rs` | 110 | child module of kept file crates/nurbs-core/src/surface_offset/mod.rs |
| `crates/nurbs-core/src/analysis/surface_collar/source_span.rs` | 65 | only 0.08 of lines exist in a compiled file |
| `crates/nurbs-core/src/sweeps/progressive_sweep/tests/fixed_normal_arc_line.rs` | 52 | 1 fn(s) with no live namesake: fixed_normal_arc_line_uses_exact_length_image_but_charges_retained_endpoints; only 0.39 of lines exist in a compiled file |
| `crates/nurbs-core/src/ss_intersection/mod.rs` | 28 | only 0.29 of lines exist in a compiled file |
| `crates/nurbs-core/src/surface_offset/mod.rs` | 12 | only 0.43 of lines exist in a compiled file |
| `crates/sdf-core/src/surface_polygonization.rs` | 390 | only 0.58 of lines exist in a compiled file |
| `crates/sketch-core/src/constraint_evaluation.rs` | 446 | only 0.72 of lines exist in a compiled file |
| `crates/sketch-core/src/constraint_jacobian.rs` | 386 | only 0.63 of lines exist in a compiled file |
| `crates/slicer-core/src/infill_planning.rs` | 499 | only 0.65 of lines exist in a compiled file |

## TypeScript (`src/`)

- Значимых строк: 94046. В точных клонах от 20 строк: 2808 (3.0%).

| Каталог | Строк в клонах | Значимых строк | % |
|---|---:|---:|---:|
| `src/services` | 1063 | 55944 | 1.9 |
| `src/semantic` | 853 | 924 | 92.3 |
| `src/parser` | 394 | 2305 | 17.1 |
| `src/features` | 368 | 9618 | 3.8 |
| `src/mcp` | 84 | 9463 | 0.9 |
| `src` | 23 | 1608 | 1.4 |
| `src/composables` | 23 | 2185 | 1.1 |

### 40 крупнейших клонов

| Строк | Копия A | Копия B |
|---:|---|---|
| 329 | `src/semantic/lowererBuilder/SemanticProgramBuilder.ts:56` | `src/services/openscadSemanticLowerer.ts:237` |
| 155 | `src/semantic/lowererBuilder/countStatements.ts:57` | `src/services/openscadSemanticLowerer/evalUserModule.ts:100` |
| 132 | `src/semantic/lowererBuilder/registerOperations.ts:56` | `src/services/openscadSemanticLowerer/semanticValueType.ts:75` |
| 74 | `src/services/gpuNurbsReference.ts:182` | `src/services/gpuNurbsReference.ts:325` |
| 60 | `src/parser/geometryEvaluator/dxfRotateExtrude.ts:13` | `src/parser/geometryEvaluator/segments.ts:13` |
| 60 | `src/parser/geometryEvaluator/differenceChildren.ts:13` | `src/parser/geometryEvaluator/segments.ts:13` |
| 60 | `src/parser/geometryEvaluator/differenceChildren.ts:13` | `src/parser/geometryEvaluator/dxfRotateExtrude.ts:13` |
| 60 | `src/parser/geometryEvaluator/EvalNodesFn.ts:13` | `src/parser/geometryEvaluator/segments.ts:13` |
| 60 | `src/parser/geometryEvaluator/EvalNodesFn.ts:13` | `src/parser/geometryEvaluator/dxfRotateExtrude.ts:13` |
| 60 | `src/parser/geometryEvaluator/EvalNodesFn.ts:13` | `src/parser/geometryEvaluator/differenceChildren.ts:13` |
| 54 | `src/semantic/lowererBuilder/MAX_SHAPES.ts:117` | `src/services/openscadSemanticLowerer.ts:146` |
| 50 | `src/features/DirectModeler.svt:504` | `src/features/directModeler/useDisplayAndSnapQueues.ts:327` |
| 46 | `src/parser/expressionEvaluator.ts:303` | `src/services/openscadSemanticLowerer/evalExpression.ts:80` |
| 42 | `src/mcp/createServer.ts:179` | `src/mcp/server/contracts.ts:36` |
| 39 | `src/semantic/lowererBuilder/MAX_SHAPES.ts:217` | `src/services/openscadSemanticLowerer/semanticValueType.ts:23` |
| 36 | `src/semantic/lowererBuilder/countStatements.ts:5` | `src/semantic/lowererBuilder/registerOperations.ts:5` |
| 36 | `src/semantic/lowererBuilder/SemanticProgramBuilder.ts:5` | `src/semantic/lowererBuilder/registerOperations.ts:5` |
| 36 | `src/semantic/lowererBuilder/SemanticProgramBuilder.ts:5` | `src/semantic/lowererBuilder/countStatements.ts:5` |
| 36 | `src/semantic/lowererBuilder/MAX_SHAPES.ts:5` | `src/semantic/lowererBuilder/registerOperations.ts:5` |
| 36 | `src/semantic/lowererBuilder/MAX_SHAPES.ts:5` | `src/semantic/lowererBuilder/countStatements.ts:5` |
| 36 | `src/semantic/lowererBuilder/MAX_SHAPES.ts:5` | `src/semantic/lowererBuilder/SemanticProgramBuilder.ts:5` |
| 36 | `src/parser/assetEvaluator/makeImport.ts:9` | `src/parser/assetEvaluator/surfaceCallSource.ts:9` |
| 36 | `src/parser/assetEvaluator/compatibilityDxfProfile.ts:9` | `src/parser/assetEvaluator/surfaceCallSource.ts:9` |
| 36 | `src/parser/assetEvaluator/compatibilityDxfProfile.ts:9` | `src/parser/assetEvaluator/makeImport.ts:9` |
| 35 | `src/services/gpuImageTrace.ts:157` | `src/services/gpuImageTrace.ts:300` |
| 32 | `src/features/DirectModeler.svt:818` | `src/features/directModeler/useViewportLayerAndPolygons.ts:462` |
| 26 | `src/features/MeshModeler/useMeshModeler/meshInteraction.ts:14` | `src/features/MeshModeler/useMeshModeler/meshInteractionContext.ts:15` |
| 26 | `src/features/MeshModeler/useMeshModeler/meshCommandsContext.ts:15` | `src/features/MeshModeler/useMeshModeler/meshInteractionContext.ts:15` |
| 26 | `src/features/MeshModeler/useMeshModeler/meshCommandsContext.ts:15` | `src/features/MeshModeler/useMeshModeler/meshInteraction.ts:14` |
| 26 | `src/features/MeshModeler/useMeshModeler/meshCommands.ts:13` | `src/features/MeshModeler/useMeshModeler/meshInteractionContext.ts:15` |
| 26 | `src/features/MeshModeler/useMeshModeler/meshCommands.ts:13` | `src/features/MeshModeler/useMeshModeler/meshInteraction.ts:14` |
| 26 | `src/features/MeshModeler/useMeshModeler/meshCommands.ts:13` | `src/features/MeshModeler/useMeshModeler/meshCommandsContext.ts:15` |
| 25 | `src/features/MeshModeler/useMeshModeler.ts:15` | `src/features/MeshModeler/useMeshModeler/meshInteractionContext.ts:15` |
| 25 | `src/features/MeshModeler/useMeshModeler.ts:15` | `src/features/MeshModeler/useMeshModeler/meshInteraction.ts:14` |
| 25 | `src/features/MeshModeler/useMeshModeler.ts:15` | `src/features/MeshModeler/useMeshModeler/meshCommandsContext.ts:15` |
| 25 | `src/features/MeshModeler/useMeshModeler.ts:15` | `src/features/MeshModeler/useMeshModeler/meshCommands.ts:13` |
| 25 | `src/features/MeshModeler.svt:89` | `src/features/MeshModeler/useMeshModeler.ts:419` |
| 25 | `src/features/MeshModeler.svt:25` | `src/features/MeshModeler/useMeshModeler/meshInteractionContext.ts:15` |
| 25 | `src/features/MeshModeler.svt:25` | `src/features/MeshModeler/useMeshModeler/meshInteraction.ts:14` |
| 25 | `src/features/MeshModeler.svt:25` | `src/features/MeshModeler/useMeshModeler/meshCommandsContext.ts:15` |
