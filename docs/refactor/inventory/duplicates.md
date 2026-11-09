# Дубликаты и мёртвые копии

Источник: `tools/refactor-audit/dupes.py` (точные клоны: совпадение нормализованных значимых строк, окно от 30 строк для Rust и от 20 для TypeScript) и разбор дерева модулей `tools/refactor-audit/audit-rs`.

## Rust: сводка

- Значимых строк: 885482. В точных клонах от 30 строк: 85837 (9.7%).
- Наборов полностью идентичных файлов: 27.

### Клоны по крейтам

| Крейт | Строк в клонах | Значимых строк | % |
|---|---:|---:|---:|
| `crates/brep-core` | 31791 | 376819 | 8.4 |
| `crates/nurbs-core` | 16701 | 282045 | 5.9 |
| `crates/cad-step` | 9116 | 13090 | 69.6 |
| `crates/nurbs-intersect` | 8156 | 9091 | 89.7 |
| `crates/brep-intersect` | 5667 | 8594 | 65.9 |
| `crates/modelgraph-runtime` | 3545 | 4843 | 73.2 |
| `crates/rush-runtime` | 3545 | 6007 | 59.0 |
| `crates/geometry-bridge` | 2911 | 46716 | 6.2 |
| `crates/bridge-svg` | 1547 | 3624 | 42.7 |
| `crates/sketch-core` | 806 | 4446 | 18.1 |
| `crates/slicer-core` | 776 | 2411 | 32.2 |
| `crates/cad-predicates` | 568 | 26129 | 2.2 |
| `crates/sdf-core` | 452 | 4511 | 10.0 |
| `crates/mesh-topology` | 118 | 1681 | 7.0 |
| `crates/modelgraph-text` | 37 | 39 | 94.9 |
| `crates/rush-frontend` | 37 | 4497 | 0.8 |
| `crates/photogrammetry-core` | 32 | 14218 | 0.2 |
| `crates/photogrammetry-ffi` | 32 | 1458 | 2.2 |

### Масса клонов между крейтами

| A | B | Строк |
|---|---|---:|
| `crates/brep-core` | `crates/cad-step` | 9116 |
| `crates/nurbs-core` | `crates/nurbs-core` | 8415 |
| `crates/brep-core` | `crates/nurbs-intersect` | 8157 |
| `crates/brep-core` | `crates/brep-intersect` | 5723 |
| `crates/brep-core` | `crates/brep-core` | 5564 |
| `crates/modelgraph-runtime` | `crates/rush-runtime` | 3545 |
| `crates/bridge-svg` | `crates/geometry-bridge` | 1547 |
| `crates/geometry-bridge` | `crates/geometry-bridge` | 1059 |
| `crates/sketch-core` | `crates/sketch-core` | 403 |
| `crates/slicer-core` | `crates/slicer-core` | 388 |
| `crates/brep-core` | `crates/geometry-bridge` | 352 |
| `crates/cad-predicates` | `crates/cad-predicates` | 284 |
| `crates/sdf-core` | `crates/sdf-core` | 226 |
| `crates/brep-core` | `crates/nurbs-core` | 86 |
| `crates/mesh-topology` | `crates/mesh-topology` | 59 |
| `crates/brep-intersect` | `crates/brep-intersect` | 38 |
| `crates/modelgraph-text` | `crates/rush-frontend` | 37 |
| `crates/photogrammetry-core` | `crates/photogrammetry-ffi` | 32 |

### 80 крупнейших клонов

| Строк | Копия A | Копия B |
|---:|---|---|
| 920 | `crates/brep-core/src/intersections/tests/tests_integration.rs:1997` (тест) | `crates/nurbs-intersect/src/queries/tests/mod.rs:1253` (тест) |
| 629 | `crates/nurbs-core/src/construct/surfaces/gordon/cartesian.rs:3` (прод) | `crates/nurbs-core/src/gordon_cartesian.rs:3` (не компилируется) |
| 584 | `crates/brep-core/src/tests/step_interchange_v3.rs:63` (тест) | `crates/cad-step/src/tests/step_interchange_v3.rs:60` (тест) |
| 576 | `crates/nurbs-core/src/construct/surfaces/loft/guided_loft.rs:4` (прод) | `crates/nurbs-core/src/guided_loft.rs:4` (не компилируется) |
| 513 | `crates/brep-core/src/tests/step_interchange_v3.rs:1453` (тест) | `crates/cad-step/src/tests/step_interchange_v3.rs:1203` (тест) |
| 458 | `crates/modelgraph-runtime/src/eval/evaluator.rs:204` (не компилируется) | `crates/rush-runtime/src/eval/evaluator_impl_2.rs:4` (прод) |
| 438 | `crates/nurbs-core/src/offset/surface_offset/tests/source.rs:2` (тест) | `crates/nurbs-core/src/surface_offset/tests/mod.rs:2` (не компилируется) |
| 417 | `crates/nurbs-core/src/construct/surfaces/loft/alignment.rs:3` (прод) | `crates/nurbs-core/src/loft_alignment.rs:3` (не компилируется) |
| 374 | `crates/brep-core/src/step_interchange.rs:1260` (прод) | `crates/cad-step/src/step_interchange/export_step.rs:61` (прод) |
| 366 | `crates/modelgraph-runtime/src/schema/expression.rs:3` (не компилируется) | `crates/rush-runtime/src/schema/expression.rs:3` (прод) |
| 359 | `crates/brep-core/src/analytic_ss.rs:99` (прод) | `crates/nurbs-intersect/src/analytic_ss/analytic_ss_component.rs:69` (прод) |
| 347 | `crates/brep-core/src/sweep_miter_owned/tests/direct_source_regressions_integration.rs:794` (тест) | `crates/brep-core/src/tests/sweep_miter_owned.rs:194` (тест) |
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
| 247 | `crates/nurbs-core/src/sweeps/progressive_miter/authored_frame_certificate/tests/tests_integration.rs:31` (тест) | `crates/nurbs-core/src/sweeps/progressive_miter/tests/authored_frame_certificate.rs:2` (не компилируется) |
| 241 | `crates/modelgraph-runtime/src/schema/validate.rs:235` (не компилируется) | `crates/rush-runtime/src/schema/validate.rs:235` (прод) |
| 237 | `crates/brep-core/src/nurbs_step_shared.rs:8` (прод) | `crates/cad-step/src/nurbs_step_shared/refuse.rs:3` (прод) |
| 234 | `crates/nurbs-core/src/construct/curves/paths/miter_sections.rs:12` (прод) | `crates/nurbs-core/src/paths/miter_sections.rs:12` (не компилируется) |
| 233 | `crates/brep-core/src/intersections/serialization.rs:4` (прод) | `crates/nurbs-intersect/src/queries/codec.rs:2` (прод) |
| 232 | `crates/brep-core/src/intersections.rs:2060` (прод) | `crates/nurbs-intersect/src/queries/surface_surface.rs:39` (прод) |
| 224 | `crates/bridge-svg/src/svg/tests/mod.rs:288` (тест) | `crates/geometry-bridge/src/svg/tests/tests_fixtures.rs:288` (не компилируется) |
| 221 | `crates/brep-core/src/nurbs_ss_g6/tests/mod.rs:994` (тест) | `crates/nurbs-intersect/src/nurbs_ss/tests/mod.rs:496` (тест) |
| 209 | `crates/brep-core/src/step_interchange_v3.rs:2710` (прод) | `crates/cad-step/src/step_interchange_v3/compose_step_v8_occurrences.rs:144` (прод) |
| 206 | `crates/nurbs-core/src/construct/surfaces/gordon.rs:118` (прод) | `crates/nurbs-core/src/gordon.rs:117` (не компилируется) |
| 203 | `crates/bridge-svg/src/svg/tests/mod.rs:2` (тест) | `crates/geometry-bridge/src/svg/tests/tests_fixtures.rs:2` (не компилируется) |
| 201 | `crates/brep-core/src/intersections/sphere_torus.rs:68` (прод) | `crates/brep-intersect/src/sphere_torus.rs:67` (прод) |
| 201 | `crates/brep-core/src/intersections.rs:316` (прод) | `crates/nurbs-intersect/src/queries/curve_plane.rs:73` (прод) |
| 198 | `crates/brep-core/src/step_v3_export/algorithm_periodicized_sphere.rs:4` (прод) | `crates/cad-step/src/step_interchange_v3/quadratic_patch_knots.rs:149` (прод) |
| 197 | `crates/brep-core/src/nurbs_step_interchange.rs:9` (прод) | `crates/cad-step/src/nurbs_step_interchange.rs:9` (прод) |
| 197 | `crates/brep-core/src/nurbs_ss_g6/certify_exact_planar_iso_intersection.rs:8` (прод) | `crates/nurbs-intersect/src/nurbs_ss/certify_exact_planar_iso_intersection.rs:8` (прод) |
| 197 | `crates/brep-core/src/intersections.rs:1760` (прод) | `crates/nurbs-intersect/src/queries/curve_surface.rs:3` (прод) |
| 190 | `crates/brep-core/src/step_interchange.rs:885` (прод) | `crates/cad-step/src/step_interchange/step_writer.rs:280` (прод) |
| 182 | `crates/nurbs-core/src/curve_offset.rs:270` (не компилируется) | `crates/nurbs-core/src/offset/tests/curve_offset.rs:3` (тест) |
| 179 | `crates/bridge-svg/src/svg/preflight_inner.rs:3` (прод) | `crates/geometry-bridge/src/svg/preflight_validation.rs:4` (не компилируется) |
| 178 | `crates/brep-core/src/coverage_verifier.rs:132` (прод) | `crates/nurbs-intersect/src/coverage_verifier.rs:86` (прод) |
| 177 | `crates/brep-core/src/intersections/cylinder_torus.rs:106` (прод) | `crates/brep-intersect/src/cylinder_torus/cylinder_torus_component.rs:34` (прод) |
| 176 | `crates/modelgraph-runtime/src/schema/node.rs:3` (не компилируется) | `crates/rush-runtime/src/schema/node.rs:3` (прод) |
| 176 | `crates/brep-core/src/step_interchange.rs:1854` (прод) | `crates/cad-step/src/step_interchange/tests/mod.rs:5` (тест) |
| 175 | `crates/nurbs-core/src/construct/curves/paths.rs:5` (прод) | `crates/nurbs-core/src/paths.rs:5` (не компилируется) |
| 172 | `crates/brep-core/src/intersections/plane_cone.rs:1221` (прод) | `crates/brep-intersect/src/plane_cone/intersect_plane_cone.rs:273` (прод) |
| 170 | `crates/brep-core/src/intersections.rs:1402` (прод) | `crates/nurbs-intersect/src/queries/surface_plane.rs:367` (прод) |
| 168 | `crates/brep-core/src/intersections/plane_torus.rs:568` (прод) | `crates/brep-intersect/src/plane_torus/torus_patch_curve.rs:218` (прод) |
| 165 | `crates/brep-core/src/iges_interchange_v2.rs:2475` (прод) | `crates/cad-step/src/iges_interchange_v2/export_iges_v2.rs:37` (прод) |
| 162 | `crates/nurbs-core/src/analysis/surface_measure.rs:14` (прод) | `crates/nurbs-core/src/surface_measure.rs:13` (не компилируется) |
| 160 | `crates/brep-core/src/nurbs_ss_g6/g6_maturity.rs:304` (прод) | `crates/nurbs-intersect/src/nurbs_ss/refuse.rs:268` (прод) |
| 158 | `crates/brep-core/src/nurbs_ss_general/tests.rs:4` (тест) | `crates/brep-core/tests/ss_report_admission.rs:10` (тест) |
| 157 | `crates/modelgraph-runtime/src/eval/evaluator.rs:3` (не компилируется) | `crates/rush-runtime/src/eval/evaluator_impl_1.rs:3` (прод) |
| 156 | `crates/brep-core/src/intersections/tests/tests_integration.rs:3` (тест) | `crates/nurbs-intersect/src/queries/tests/mod.rs:3` (тест) |
| 155 | `crates/modelgraph-runtime/src/eval/tests/mod.rs:2` (не компилируется) | `crates/rush-runtime/src/tests/eval.rs:2` (тест) |
| 154 | `crates/brep-core/src/intersections/cone_torus.rs:179` (прод) | `crates/brep-intersect/src/cone_torus/cone_torus_component.rs:89` (прод) |
| 151 | `crates/slicer-core/src/infill.rs:182` (прод) | `crates/slicer-core/src/infill_planning.rs:5` (не компилируется) |
| 151 | `crates/brep-core/src/step_v3_export.rs:281` (прод) | `crates/cad-step/src/step_interchange_v3/export_step_v3.rs:11` (прод) |
| 149 | `crates/nurbs-core/src/construct/curves/hermite.rs:10` (прод) | `crates/nurbs-core/src/hermite.rs:10` (не компилируется) |
| 149 | `crates/brep-core/src/intersections.rs:4806` (прод) | `crates/nurbs-intersect/src/queries/ruled_points.rs:261` (прод) |
| 146 | `crates/nurbs-core/src/construct/surfaces/gordon/tangent_fields.rs:5` (прод) | `crates/nurbs-core/src/gordon_tangent_fields.rs:5` (не компилируется) |
| 145 | `crates/nurbs-core/src/offset/surface_offset/source.rs:10` (прод) | `crates/nurbs-core/src/surface_offset/bounds.rs:4` (не компилируется) |
| 145 | `crates/cad-predicates/src/bspline_internal_jets/tests/tests_integration.rs:49` (тест) | `crates/cad-predicates/src/tests/bspline_internal_jets.rs:46` (не компилируется) |
| 145 | `crates/brep-core/src/intersections.rs:5006` (прод) | `crates/nurbs-intersect/src/queries/curve_ruled_surface.rs:345` (прод) |
| 144 | `crates/sdf-core/src/differential_and_queries.rs:15` (прод) | `crates/sdf-core/src/surface_polygonization.rs:8` (не компилируется) |
| 144 | `crates/brep-core/src/intersections.rs:4474` (прод) | `crates/nurbs-intersect/src/queries/curve_ruled_surface.rs:127` (прод) |
| 142 | `crates/brep-core/src/nurbs_ss_g6/join_certified_multispan_fragments.rs:4` (прод) | `crates/nurbs-intersect/src/nurbs_ss/join_certified_multispan_fragments.rs:4` (прод) |
| 138 | `crates/slicer-core/src/adhesion_and_support.rs:110` (прод) | `crates/slicer-core/src/infill_planning.rs:328` (не компилируется) |
| 138 | `crates/brep-core/src/step_interchange_v3.rs:554` (прод) | `crates/cad-step/src/step_interchange_v3/analytic_surface.rs:4` (прод) |
| 138 | `crates/brep-core/src/intersections.rs:3448` (прод) | `crates/nurbs-intersect/src/queries/curve_coincidence.rs:57` (прод) |
| 138 | `crates/brep-core/src/analytic_ss.rs:595` (прод) | `crates/nurbs-intersect/src/analytic_ss/cylinder_sphere.rs:6` (прод) |
| 136 | `crates/bridge-svg/src/svg/preflight_inner.rs:403` (прод) | `crates/geometry-bridge/src/svg/preflight_validation.rs:411` (не компилируется) |
| 136 | `crates/brep-core/src/nurbs_ss_g6/tests/mod.rs:566` (тест) | `crates/nurbs-intersect/src/nurbs_ss/tests/mod.rs:304` (тест) |

## Rust: функции, текстуально идентичные в двух компилируемых файлах

Сравнение полного текста функции без пробелов. Тесты из этого списка выполняются дважды.

| Идентичных fn | из них в тестовом коде | LOC | Файл A | Файл B |
|---:|---:|---:|---|---|
| 9 | 0 | 260 | `crates/brep-core/src/minkowski3d/curved/atlas/bernstein.rs` | `crates/brep-core/src/minkowski3d/curved/atlas_legacy/bernstein.rs` |
| 9 | 9 | 196 | `crates/brep-core/src/nurbs_ss_general/tests.rs` | `crates/brep-core/tests/ss_report_admission.rs` |
| 9 | 9 | 158 | `crates/nurbs-core/src/surface_quotient_injectivity/report/tests/mod.rs` | `crates/nurbs-core/src/surface_quotient_injectivity/tests/mod.rs` |
| 8 | 0 | 199 | `crates/brep-core/src/minkowski3d/curved/atlas.rs` | `crates/brep-core/src/minkowski3d/curved/atlas_legacy.rs` |
| 8 | 8 | 536 | `crates/brep-core/src/sweep_miter_owned/tests/direct_source_regressions_integration.rs` | `crates/brep-core/src/tests/sweep_miter_owned.rs` |
| 7 | 1 | 72 | `crates/brep-core/src/minkowski3d/rounding.rs` | `crates/brep-core/src/minkowski3d/rounding_legacy.rs` |
| 6 | 0 | 106 | `crates/brep-core/src/minkowski3d/curved/atlas/transport.rs` | `crates/brep-core/src/minkowski3d/curved/atlas_legacy/transport.rs` |
| 5 | 5 | 76 | `crates/geometry-bridge/tests/support/fibered_source_reference.rs` | `crates/geometry-bridge/tests/support/rational_cap_flux.rs` |
| 5 | 5 | 79 | `crates/nurbs-core/src/trim/trim_simplicity/tests/multispan_regressions_integration.rs` | `crates/nurbs-core/src/trim/trim_simplicity/tests/tests_integration.rs` |
| 4 | 4 | 81 | `crates/brep-core/src/gear/tests/involute_certificates.rs` | `crates/brep-core/src/gear/tests/mod.rs` |
| 4 | 4 | 52 | `crates/brep-core/src/profile_certificate/winding/periodic/multispan.rs` | `crates/brep-core/tests/periodic_multispan_regions.rs` |
| 4 | 4 | 70 | `crates/cad-predicates/tests/construction3_recipes.rs` | `crates/cad-predicates/tests/construction_recipes.rs` |
| 4 | 1 | 98 | `crates/nurbs-core/src/numerics/exact_curve_segments.rs` | `crates/nurbs-core/src/sweep_support/exact_curve_segments.rs` |
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
| 2 | 0 | 49 | `crates/brep-core/src/minkowski3d/curved/atlas/codec.rs` | `crates/brep-core/src/minkowski3d/curved/atlas_legacy/codec.rs` |
| 2 | 0 | 14 | `crates/brep-core/src/projected_trim_contact.rs` | `crates/brep-core/src/same_support_contact.rs` |
| 2 | 0 | 16 | `crates/brep-core/src/ss_face_membership/closed/annulus.rs` | `crates/brep-core/src/ss_face_membership/closed/boolean.rs` |

## Rust: некомпилируемые файлы

Файл считается некомпилируемым, если он не достижим из корня ни одной цели Cargo (lib, bin, test, example, bench) через `mod`, `#[path]` или `include!`.

### Доказуемо устаревшие копии, кандидаты на удаление (0 файлов, 0 LOC)

Критерии: файл в индексе git и не изменён; не менее 80% его строк есть в компилируемом файле того же крейта; каждое имя функции из него существует в живом коде крейта; путь к нему не упоминается в отслеживаемых не-Rust файлах; он не дочерний модуль оставленного файла. Удалённое ранее перечислено в `removed-*.md`.

| Файл | LOC | Доля строк в живом файле | Живой двойник |
|---|---:|---:|---|

### Оставлено до решения владельца (93 файлов, 26218 LOC)

| Файл | LOC | Почему не удалён автоматически |
|---|---:|---|
| `crates/cad-predicates/src/tests/bspline_internal_jets.rs` | 282 | 2 fn(s) with no live namesake: zero_order_position_at_full_multiplicity_never_uses_a_saturating_continuity_shortcut, zero_order_nonclamped_endpoint_position_is_exact_and_keeps_the_work_limit |
| `crates/geometry-bridge/src/svg.rs` | 763 | 23 fn(s) with no live namesake: limit, effect, check_urls; only 0.02 of lines exist in a compiled file |
| `crates/geometry-bridge/src/svg/tests/tests_fixtures.rs` | 592 | 30 fn(s) with no live namesake: area, bounds, open_scad_dimensions_and_viewbox_are_binary64; only 0.04 of lines exist in a compiled file |
| `crates/geometry-bridge/src/svg/preflight_validation.rs` | 583 | 2 fn(s) with no live namesake: preflight_inner, cost; only 0.02 of lines exist in a compiled file |
| `crates/geometry-bridge/src/svg/raster_normalization.rs` | 443 | 5 fn(s) with no live namesake: normalize_raster_image, legacy_source, px_to_pt; only 0.04 of lines exist in a compiled file; path referenced from tracked non-Rust files |
| `crates/geometry-bridge/src/progressive_annular_tests.rs` | 218 | 6 fn(s) with no live namesake: progressive_annular_wire_owns_caps_full_bound_and_actual_jets, progressive_annular_wire_refuses_injection_and_unproved_budgets, progressive_annular_wire_reconstruction_keeps_all_caps_and_replays_original_source; only 0.14 of lines exist in a compiled file |
| `crates/geometry-bridge/src/svg/operation_dispatch.rs` | 184 | only 0.13 of lines exist in a compiled file |
| `crates/math-core/src/spatial/tests/mod.rs` | 254 | crate dir outside workspace; 9 fn(s) with no live namesake: quat_axis_angle_slerp_and_mat3_roundtrip, dual_quat_transforms_and_blends_rigid_motions, ray_aabb_sphere_and_plane_intersections; only 0.00 of lines exist in a compiled file |
| `crates/math-core/src/spatial/methods_Quat_IDENTITY.rs` | 185 | crate dir outside workspace; 14 fn(s) with no live namesake: new, identity, from_axis_angle; only 0.00 of lines exist in a compiled file |
| `crates/math-core/src/spatial/methods_Aabb3_empty.rs` | 155 | crate dir outside workspace; 14 fn(s) with no live namesake: empty, new, is_empty; only 0.00 of lines exist in a compiled file |
| `crates/math-core/src/spatial/methods_Ray3_new.rs` | 109 | crate dir outside workspace; 6 fn(s) with no live namesake: new, at, intersect_aabb; only 0.00 of lines exist in a compiled file |
| `crates/math-core/src/spatial/methods_DualQuat_IDENTITY.rs` | 101 | crate dir outside workspace; 6 fn(s) with no live namesake: from_rotation_translation, normalize, mul; only 0.00 of lines exist in a compiled file |
| `crates/math-core/src/spatial/methods_Frustum_from_view_proj_row_major.rs` | 97 | crate dir outside workspace; 6 fn(s) with no live namesake: from_view_proj_row_major, from_view_proj_col_major, contains_point; only 0.00 of lines exist in a compiled file |
| `crates/modelgraph-runtime/src/eval/evaluator.rs` | 1018 | crate dir outside workspace; 13 fn(s) with no live namesake: new, allocate, tick; only 0.00 of lines exist in a compiled file; path referenced from tracked non-Rust files |
| `crates/modelgraph-runtime/src/schema/validate.rs` | 557 | crate dir outside workspace; 12 fn(s) with no live namesake: required, optional, number_default; only 0.00 of lines exist in a compiled file |
| `crates/modelgraph-runtime/src/schema/expression.rs` | 475 | crate dir outside workspace; 8 fn(s) with no live namesake: expression, type_descriptor, type_descriptor_depth |
| `crates/modelgraph-runtime/src/nurbs/compile.rs` | 450 | crate dir outside workspace; 12 fn(s) with no live namesake: s, err, valid_id; only 0.00 of lines exist in a compiled file |
| `crates/modelgraph-runtime/src/nurbs/compile_text.rs` | 372 | crate dir outside workspace; 4 fn(s) with no live namesake: compile_text, select, field; only 0.00 of lines exist in a compiled file |
| `crates/modelgraph-runtime/src/emit/compile.rs` | 285 | crate dir outside workspace; 3 fn(s) with no live namesake: collect_assertion_targets, assertion_source, compile; only 0.00 of lines exist in a compiled file |
| `crates/modelgraph-runtime/src/emit/emitter/transforms.rs` | 268 | crate dir outside workspace; 7 fn(s) with no live namesake: emit_affine, emit_mirror, emit_projection; only 0.00 of lines exist in a compiled file |
| `crates/modelgraph-runtime/src/nurbs/tests/mod.rs` | 267 | crate dir outside workspace; 12 fn(s) with no live namesake: sphere, document, curve_extrusion_references_participate_in_graph_and_conditional_traversal; only 0.00 of lines exist in a compiled file |
| `crates/modelgraph-runtime/src/emit/emitter/emitter.rs` | 244 | crate dir outside workspace; 6 fn(s) with no live namesake: value, vector, geometry_vector; only 0.00 of lines exist in a compiled file |
| `crates/modelgraph-runtime/src/emit/emitter/control.rs` | 239 | crate dir outside workspace; 8 fn(s) with no live namesake: emit_assert, emit_evaluate, emit_call; only 0.00 of lines exist in a compiled file |
| `crates/modelgraph-runtime/src/schema/node.rs` | 206 | crate dir outside workspace; 1 fn(s) with no live namesake: node |
| `crates/modelgraph-runtime/src/eval/tests/mod.rs` | 205 | crate dir outside workspace; 10 fn(s) with no live namesake: document, evaluate, scalars |
| `crates/modelgraph-runtime/src/emit/assertion_tests/mod.rs` | 187 | crate dir outside workspace; 13 fn(s) with no live namesake: quantity, width, hidden_and_transformed_targets_have_separate_sources; only 0.00 of lines exist in a compiled file |
| `crates/modelgraph-runtime/src/emit/number.rs` | 186 | crate dir outside workspace; 10 fn(s) with no live namesake: number, json_string, append; only 0.00 of lines exist in a compiled file |
| `crates/modelgraph-runtime/src/schema/tests/mod.rs` | 182 | crate dir outside workspace; 13 fn(s) with no live namesake: document, matches_original_zod_schema_corpus, reports_exact_nested_paths_and_budget_error_codes; only 0.00 of lines exist in a compiled file |
| `crates/modelgraph-runtime/src/emit/emitter/solids.rs` | 147 | crate dir outside workspace; 4 fn(s) with no live namesake: emit_advanced_extrude, emit_cone, emit_torus; only 0.00 of lines exist in a compiled file |
| `crates/modelgraph-runtime/src/emit/emitter/sketches.rs` | 137 | crate dir outside workspace; 2 fn(s) with no live namesake: emit_loft, emit_sketch; only 0.00 of lines exist in a compiled file |
| `crates/modelgraph-runtime/src/emit/emitter/mechanisms.rs` | 129 | crate dir outside workspace; 2 fn(s) with no live namesake: emit_planetary_spinner, emit_mechanical; only 0.00 of lines exist in a compiled file |
| `crates/modelgraph-runtime/src/emit/emitter/assemblies.rs` | 127 | crate dir outside workspace; 1 fn(s) with no live namesake: emit_assembly; only 0.00 of lines exist in a compiled file |
| `crates/modelgraph-runtime/src/emit/emitter/primitives.rs` | 112 | crate dir outside workspace; 3 fn(s) with no live namesake: emit_rectangle_box, emit_round, emit_polygon; only 0.00 of lines exist in a compiled file |
| `crates/modelgraph-runtime/src/eval/value.rs` | 81 | crate dir outside workspace; 5 fn(s) with no live namesake: from, from, numeric_value |
| `crates/modelgraph-runtime/src/emit/dynamic_vector_tests/mod.rs` | 62 | crate dir outside workspace; 3 fn(s) with no live namesake: document, uses_dynamic_box_and_transform_vectors_with_correct_units, rejects_wrong_vector_lengths_scalar_results_and_dimensions; only 0.00 of lines exist in a compiled file |
| `crates/modelgraph-runtime/src/nurbs/cartesian_loft_tests/mod.rs` | 52 | crate dir outside workspace; 3 fn(s) with no live namesake: document, cartesian_loft_graph_retains_audit_options_and_refuses_ignored_fields, section_maps_resolve_dimensionless_values_and_validate_nullable_shapes; only 0.00 of lines exist in a compiled file |
| `crates/modelgraph-runtime/src/eval/memo_tests/mod.rs` | 44 | crate dir outside workspace; 2 fn(s) with no live namesake: memo_reuses_successful_checks_only_within_the_same_scope, memo_rejects_conflicting_bodies_and_caches_failures |
| `crates/modelgraph-runtime/src/eval/guarded_tests/mod.rs` | 35 | crate dir outside workspace; 2 fn(s) with no live namesake: guard_steps_keep_statement_order_and_stop_before_later_items, guarded_sequence_returns_original_values_with_lexical_binding |
| `crates/modelgraph-runtime/src/eval/mod.rs` | 26 | crate dir outside workspace; only 0.00 of lines exist in a compiled file |
| `crates/modelgraph-runtime/src/emit/emitter/mod.rs` | 25 | crate dir outside workspace; only 0.00 of lines exist in a compiled file |
| `crates/modelgraph-runtime/src/emit/mod.rs` | 23 | crate dir outside workspace; only 0.00 of lines exist in a compiled file |
| `crates/modelgraph-runtime/src/emit/match_tests/mod.rs` | 18 | crate dir outside workspace; 1 fn(s) with no live namesake: geometry_match_keeps_bindings_local_and_builds_only_selected_arm; only 0.00 of lines exist in a compiled file |
| `crates/modelgraph-runtime/src/nurbs/mod.rs` | 17 | crate dir outside workspace; only 0.00 of lines exist in a compiled file |
| `crates/modelgraph-runtime/src/eval/array.rs` | 15 | crate dir outside workspace; 4 fn(s) with no live namesake: array, string, number |
| `crates/modelgraph-runtime/src/schema/mod.rs` | 15 | crate dir outside workspace; only 0.00 of lines exist in a compiled file |
| `crates/modelgraph-text/src/lower/compiler/geometry_reference.rs` | 49 | crate dir outside workspace; 1 fn(s) with no live namesake: geometry; only 0.00 of lines exist in a compiled file |
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
| `crates/nurbs-core/src/chord_arrangement.rs` | 352 | 6 fn(s) with no live namesake: same_crossing, three_exact_crossings_share_one_vertex_without_collapsed_cuts, nonbinary_concurrent_intersections_share_an_exact_recipe; only 0.67 of lines exist in a compiled file |
| `crates/nurbs-core/src/sweeps/progressive_miter/tests/authored_frame_certificate.rs` | 339 | 1 fn(s) with no live namesake: original_axis_plane_identity_preserves_constant_normal_jets_with_rational_seed_magnitude |
| `crates/nurbs-core/src/curve_decomposition_certificate.rs` | 310 | 1 fn(s) with no live namesake: last_span_exhaustion_discards_previous_bound_and_preserves_sources |
| `crates/nurbs-core/src/surface_measure/jets.rs` | 307 | path referenced from tracked non-Rust files |
| `crates/nurbs-core/src/surface_offset_transport_tests.rs` | 307 | 10 fn(s) with no live namesake: offset_transport_preserves_numerical_and_interval_claim_boundaries, offset_section_transport_does_not_promote_a_contact_to_a_whole_fillet, offset_candidate_transport_retains_incomplete_work_and_validates_second_operand; only 0.14 of lines exist in a compiled file |
| `crates/nurbs-core/src/paths/miter_sections.rs` | 302 | child module of kept file crates/nurbs-core/src/paths.rs |
| `crates/nurbs-core/src/gordon_tangent_audit.rs` | 295 | path referenced from tracked non-Rust files |
| `crates/nurbs-core/src/curve_offset_diagnostics.rs` | 284 | only 0.53 of lines exist in a compiled file |
| `crates/nurbs-core/src/analysis/surface_injectivity/tests.rs` | 234 | 1 fn(s) with no live namesake: affine_corners_never_admit_an_interior_fold_or_hide_proposal_work |
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
| `crates/raster-core/src/postprocess/tests/mod.rs` | 154 | crate dir outside workspace; 6 fn(s) with no live namesake: evaluate_culling_stats_classifies_frustum_and_backface_triangles, ssao_darkens_concave_depth_pockets_and_preserves_flat_planes, sobel_edge_outline_detects_step_boundaries; only 0.00 of lines exist in a compiled file |
| `crates/sdf-core/src/surface_polygonization.rs` | 390 | only 0.58 of lines exist in a compiled file |
| `crates/sketch-core/src/constraint_evaluation.rs` | 446 | only 0.72 of lines exist in a compiled file |
| `crates/sketch-core/src/constraint_jacobian.rs` | 386 | only 0.63 of lines exist in a compiled file |
| `crates/slicer-core/src/infill_planning.rs` | 499 | only 0.65 of lines exist in a compiled file |
| `crates/tensor-core/src/host_tensor/methods_Tensor_new.rs` | 524 | crate dir outside workspace; 17 fn(s) with no live namesake: new, from_shape, shape; only 0.00 of lines exist in a compiled file |
| `crates/tensor-core/src/host_tensor/methods_Tensor_conv1d.rs` | 472 | crate dir outside workspace; 7 fn(s) with no live namesake: conv1d, max_pool1d, avg_pool1d; only 0.00 of lines exist in a compiled file |
| `crates/tensor-core/src/host_tensor/tests/mod.rs` | 193 | crate dir outside workspace; 6 fn(s) with no live namesake: layer_norm_and_rms_norm_normalize_rows, gelu_and_silu_match_reference_values, cosine_similarity_and_topk_compute_expected_outputs; only 0.00 of lines exist in a compiled file |

## TypeScript (`src/`)

- Значимых строк: 100746. В точных клонах от 20 строк: 12990 (12.9%).

| Каталог | Строк в клонах | Значимых строк | % |
|---|---:|---:|---:|
| `src/mcp` | 4217 | 12377 | 34.1 |
| `src/services` | 3971 | 55956 | 7.1 |
| `src/semantic` | 2401 | 2736 | 87.8 |
| `src/importers` | 1571 | 1962 | 80.1 |
| `src/parser` | 416 | 2305 | 18.0 |
| `src/features` | 368 | 9618 | 3.8 |
| `src` | 23 | 1608 | 1.4 |
| `src/composables` | 23 | 2185 | 1.1 |

### 40 крупнейших клонов

| Строк | Копия A | Копия B |
|---:|---|---|
| 329 | `src/semantic/lowererBuilder/SemanticProgramBuilder.ts:56` | `src/services/openscadSemanticLowerer.ts:237` |
| 210 | `src/importers/types.ts:5` | `src/services/openScadImport.ts:19` |
| 207 | `src/semantic/occurrenceValidator/validateOccurrences.ts:28` | `src/services/semanticProgramValidator/isChildrenExpansionContinuation.ts:117` |
| 160 | `src/mcp/createServer/createOpenScadMcpServer.ts:812` | `src/mcp/server/registerResourcesAndPrompts/review_model.ts:89` |
| 155 | `src/semantic/lowererBuilder/countStatements.ts:57` | `src/services/openscadSemanticLowerer/evalUserModule.ts:100` |
| 152 | `src/semantic/schemaPrimitives/sameValueType.ts:59` | `src/services/semanticProgramValidator/validateNodes.ts:7` |
| 150 | `src/semantic/schemaPrimitives/SemanticProgramValidationError.ts:140` | `src/services/semanticProgramValidator/fail.ts:91` |
| 146 | `src/mcp/createServer/createOpenScadMcpServer.ts:455` | `src/mcp/server/registerTools/check.ts:139` |
| 132 | `src/semantic/lowererBuilder/registerOperations.ts:56` | `src/services/openscadSemanticLowerer/semanticValueType.ts:75` |
| 126 | `src/importers/xmlAndArchive/XmlElement.ts:75` | `src/services/openScadImport/parseXml.ts:6` |
| 121 | `src/mcp/createServer/createOpenScadMcpServer.ts:631` | `src/mcp/server/registerTools/customize_model.ts:139` |
| 108 | `src/mcp/createServer/officialToolInputSchema.ts:271` | `src/mcp/server/wireMappers.ts:50` |
| 107 | `src/mcp/createServer.ts:350` | `src/mcp/server/schemas/idSchema.ts:240` |
| 103 | `src/importers/stlOffImporter.ts:125` | `src/services/openScadImport/parseOpenScadStl.ts:252` |
| 100 | `src/semantic/lowererExpressionEval.ts:372` | `src/services/openscadSemanticLowerer/evalExpression.ts:220` |
| 97 | `src/importers/dxfImporter/resolvePreparedDxfQueryMetadata.ts:176` | `src/services/openScadImport/queryPreparedOpenScadDxfDimension.ts:89` |
| 95 | `src/mcp/createServer/officialToolInputSchema.ts:155` | `src/mcp/server/officialWireMappers.ts:33` |
| 92 | `src/semantic/occurrenceValidator/SemanticModuleActivationAnchor.ts:125` | `src/services/semanticProgramValidator/isChildrenExpansionContinuation.ts:7` |
| 92 | `src/mcp/createServer/createOpenScadMcpServer.ts:88` | `src/mcp/server/registerTools/save_model.ts:183` |
| 91 | `src/importers/xmlAndArchive/decodeOpenScadAmfText.ts:226` | `src/services/openScadImport/parseOpenScad3mf.ts:11` |
| 90 | `src/mcp/server/registerTools/official_export.ts:6` | `src/mcp/server/registerTools/save_model.ts:6` |
| 90 | `src/mcp/server/registerTools/customize_model.ts:6` | `src/mcp/server/registerTools/save_model.ts:6` |
| 90 | `src/mcp/server/registerTools/customize_model.ts:6` | `src/mcp/server/registerTools/official_export.ts:6` |
| 90 | `src/mcp/server/registerTools/check.ts:6` | `src/mcp/server/registerTools/save_model.ts:6` |
| 90 | `src/mcp/server/registerTools/check.ts:6` | `src/mcp/server/registerTools/official_export.ts:6` |
| 90 | `src/mcp/server/registerTools/check.ts:6` | `src/mcp/server/registerTools/customize_model.ts:6` |
| 90 | `src/mcp/createServer/createOpenScadMcpServer.ts:301` | `src/mcp/server/registerTools/official_export.ts:187` |
| 89 | `src/mcp/createServer/persistedFailureStatus.ts:87` | `src/mcp/server/wireMappers.ts:268` |
| 84 | `src/semantic/lowererExpressionEval.ts:217` | `src/services/openscadSemanticLowerer/evalExpression.ts:71` |
| 76 | `src/importers/xmlAndArchive/decodeOpenScadAmfText.ts:28` | `src/services/openScadImport/unzip3mf.ts:149` |
| 74 | `src/services/gpuNurbsReference.ts:182` | `src/services/gpuNurbsReference.ts:325` |
| 72 | `src/semantic/lowererGeometryOps/makePolyhedron.ts:51` | `src/services/openscadSemanticLowerer/makeCylinder.ts:48` |
| 70 | `src/semantic/occurrenceValidator/SemanticModuleActivationAnchor.ts:44` | `src/services/semanticProgramValidator/validateNodes.ts:222` |
| 70 | `src/mcp/createServer/createOpenScadMcpServer.ts:1081` | `src/mcp/server/registerResourcesAndPrompts/geometry_parity.ts:154` |
| 65 | `src/importers/xmlAndArchive/XmlElement.ts:229` | `src/services/openScadImport/parseXml.ts:160` |
| 65 | `src/importers/types.ts:335` | `src/services/openScadImport/parseOpenScadStl.ts:34` |
| 64 | `src/importers/dxfImporter/Matrix2.ts:58` | `src/services/openScadImport/parseOpenScad3mf.ts:214` |
| 63 | `src/importers/xmlAndArchive/inflateRaw.ts:202` | `src/services/openScadImport/unzip3mf.ts:8` |
| 60 | `src/parser/geometryEvaluator/dxfRotateExtrude.ts:13` | `src/parser/geometryEvaluator/segments.ts:13` |
| 60 | `src/parser/geometryEvaluator/differenceChildren.ts:13` | `src/parser/geometryEvaluator/segments.ts:13` |
