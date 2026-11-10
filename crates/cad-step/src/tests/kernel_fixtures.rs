//! STEP round trips of models produced by kernel edits. The kernel tests that build these
//! models compare them with the committed fixtures in `brep-core/tests/fixtures/exchange`.
use brep_core::Model;
use brep_core::watertight::{Closure, inspect};
use nurbs_core::curve::Curve;
use value_codec::Value;

fn fixture(name: &str) -> Value {
    let path = format!("{}/../brep-core/tests/fixtures/exchange/{name}.json", env!("CARGO_MANIFEST_DIR"));
    value_codec::from_str(&std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))).unwrap()
}
fn field<T: for<'a> value_codec::Deserialize<'a>>(v: &Value, key: &str) -> T {
    value_codec::from_value(v[key].clone()).unwrap()
}
fn round_trip_v9(model: &Model) -> Model {
    let (text, export, _) = crate::export_step_v9(model).unwrap();
    assert!(export.complete);
    let (restored, import, _) = crate::import_step_v9(&text).unwrap();
    assert!(import.complete);
    restored
}
fn watertight(model: &Model) {
    assert_eq!(inspect(model, 100000).unwrap().status, Closure::WithinTolerance);
}
fn coedge_domains(model: &Model, edge: usize) -> Vec<[f64; 2]> {
    let mut d: Vec<_> = model.faces.iter()
        .flat_map(|f| std::iter::once(f.outer).chain(f.holes.iter().copied()))
        .flat_map(|w| &model.loops[w].coedges)
        .filter(|c| c.edge == edge)
        .map(|c| c.pcurve.domain())
        .collect();
    d.sort_by(|a, b| a[0].total_cmp(&b[0]).then_with(|| a[1].total_cmp(&b[1])));
    d
}
fn edge_with_curve(model: &Model, curve: &Curve) -> usize {
    let reversed = curve.reverse().unwrap();
    model.edges.iter().position(|e| e.curve == *curve || e.curve == reversed).expect("edited curve must survive STEP")
}

#[test]
fn edited_rational_bridge_body_round_trips_step_with_both_original_clocks() {
    let v = fixture("face_cut_bridge");
    let model: Model = field(&v, "model");
    let restored = round_trip_v9(&model);
    assert_eq!(restored.faces.len(), model.faces.len());
    assert_eq!(restored.edges.len(), model.edges.len());
    for world in field::<Vec<Curve>>(&v, "curves") {
        let edge = edge_with_curve(&restored, &world);
        let uses: Vec<_> = restored.loops.iter().flat_map(|l| &l.coedges).filter(|c| c.edge == edge).collect();
        assert_eq!(uses.len(), 2);
        assert!(uses.iter().all(|c| c.pcurve.domain() == [-3., 5.]));
    }
    watertight(&restored);
}

#[test]
fn step_round_trip_exports_the_actual_compacted_graph_result() {
    let model: Model = field(&fixture("face_cut_path_graph"), "model");
    let restored = round_trip_v9(&model);
    assert_eq!(restored.faces.len(), model.faces.len());
    assert_eq!(restored.edges.len(), model.edges.len());
    assert_eq!(restored.loops.len(), model.loops.len());
    watertight(&restored);
}

#[test]
fn step_round_trip_exports_the_edited_hole_cap_and_original_rational_clocks() {
    let v = fixture("face_cut_hole");
    let edited: Model = field(&v, "model");
    let restored = round_trip_v9(&edited);
    assert_eq!(restored.faces.len(), edited.faces.len());
    assert_eq!(restored.edges.len(), edited.edges.len());
    assert_eq!(restored.faces.len(), field::<usize>(&v, "source_faces") + 1);
    assert_eq!(restored.edges.len(), field::<usize>(&v, "source_edges") + 1);
    let edge = edge_with_curve(&restored, &field(&v, "curve"));
    let uses: Vec<_> = restored.loops.iter().flat_map(|l| &l.coedges).filter(|c| c.edge == edge).collect();
    assert_eq!(uses.len(), 2);
    assert!(uses.iter().all(|c| c.pcurve.domain() == [-3., 5.]));
    watertight(&restored);
}

#[test]
fn step_exports_the_actual_mixed_ss_result_with_its_new_curves_and_native_clocks() {
    let v = fixture("ss_open_network_bridges");
    let models: Vec<Model> = field(&v, "models");
    let source_faces: Vec<usize> = field(&v, "source_faces");
    let cut_edges: Vec<Vec<usize>> = field(&v, "cut_edges");
    for side in 0..2 {
        let actual = &models[side];
        let imported = round_trip_v9(actual);
        assert_eq!(imported.faces.len(), actual.faces.len());
        assert_eq!(imported.edges.len(), actual.edges.len());
        assert!(imported.faces.len() > source_faces[side]);
        for &edge in &cut_edges[side] {
            let restored = edge_with_curve(&imported, &actual.edges[edge].curve);
            assert_eq!(coedge_domains(&imported, restored), coedge_domains(actual, edge));
        }
    }
}

#[test]
fn one_model_retains_both_general_bridge_graphs_and_edited_step_topology() {
    let model: Model = field(&fixture("ss_open_network_graph"), "model");
    let restored = round_trip_v9(&model);
    assert_eq!(restored.faces.len(), 4);
    assert_eq!(restored.edges.len(), model.edges.len());
    assert_eq!(restored.shells.len(), 2);
}

fn round_trip_v6_identity(model: &Model) -> String {
    let (step, _, _) = crate::export_step_v6(model).unwrap();
    let (imported, _, _) = crate::import_step_v6(&step).unwrap();
    assert_step_identity(model, &imported);
    step
}

#[test]
fn projective_tetrahedron_round_trips_step_v6_with_identity() {
    let model: Model = field(&fixture("projective_tetrahedron"), "model");
    round_trip_v6_identity(&model);
}

/// Produces the OCCT inputs of `.github/workflows/independent-step-occt.yml`.
#[test]
fn projective_capped_end_caps_export_step_v6_and_source_step() {
    let v = fixture("projective_capped_end_caps");
    let model: Model = field(&v, "model");
    let step = round_trip_v6_identity(&model);
    if let Some(path) = std::env::var_os("CAD_PROJECTIVE_CAPPED_STEP_OUTPUT") {
        std::fs::write(path, &step).unwrap();
    }
    let restored = brep_core::source_projective_body_restore::restore(
        v["body"].clone(),
        brep_core::source_projective_body_restore::Limits {
            shell: brep_core::source_projective_shell_restore::Limits {
                regions: brep_core::source_region_restore::test_limits(),
                exact_work: 100000000,
                driver_cells: 10000,
            },
            embedding: brep_core::source_shell_geometry::Limits {
                tolerance_uv: 1e-8,
                corners: 10000,
                spans: 100000,
                linear_cells: 100000,
                exact_work: 100_000_000,
                driver_cells: 100000,
                pairs: brep_core::face_contacts::Limits {
                    pairs: 10000,
                    cells: 10000,
                    domain_cells: 100000,
                    cells_per_pair: 32,
                    domain_cells_per_pair: 512,
                },
            },
            volume: brep_core::source_volume::Limits {
                axis: 2,
                origin: 0.,
                absolute_error: 0.25,
                tolerance_uv: 1e-8,
                cells: 50000,
                spans: 100000,
                domain_cells: 100000,
            },
        },
    )
    .unwrap();
    let body = restored.body().expect(restored.reason());
    assert_eq!(brep_core::source_body_model::convert(body, 1e-7).unwrap(), model);
    let exchange = crate::source_exchange_step::prepare(
        body,
        1e-7,
        brep_core::source_exchange_endpoints::Limits {
            root_checks: 10000,
            mapping_cells: 100000,
            replay_mapping_per_use: 10000,
            exact_work: 100000000,
            driver_cells: 10000,
            spans: 100000,
            endpoints: 10000,
        },
        1000000,
    )
    .unwrap()
    .expect("Projective capped source STEP preparation");
    assert!(exchange.text.contains("TRIMMED_CURVE"));
    assert_eq!(exchange.faces, model.faces.len());
    if let Some(path) = std::env::var_os("CAD_PROJECTIVE_CAPPED_SOURCE_STEP_OUTPUT") {
        std::fs::write(path, &exchange.text).unwrap();
    }
}

/// Assert that a STEP export/import round trip preserves the model identity:
/// same entity counts, topology indices and geometry within 1e-6.
fn assert_step_identity(source: &Model, imported: &Model) {
    fn close(a: f64, b: f64, what: &str) {
        assert!((a - b).abs() <= 1e-6, "{what}: {a} vs {b}");
    }
    assert_eq!(source.vertices.len(), imported.vertices.len(), "vertex count");
    assert_eq!(source.edges.len(), imported.edges.len(), "edge count");
    assert_eq!(source.loops.len(), imported.loops.len(), "loop count");
    assert_eq!(source.faces.len(), imported.faces.len(), "face count");
    assert_eq!(source.shells.len(), imported.shells.len(), "shell count");
    assert_eq!(source.bodies.len(), imported.bodies.len(), "body count");
    for (i, (a, b)) in source.vertices.iter().zip(&imported.vertices).enumerate() {
        for d in 0..3 {
            close(a.point[d], b.point[d], &format!("vertex {i} coordinate {d}"));
        }
    }
    for (i, (a, b)) in source.edges.iter().zip(&imported.edges).enumerate() {
        assert_eq!(a.degenerate, b.degenerate, "edge {i} degenerate");
        assert_eq!(a.vertices, b.vertices, "edge {i} vertices");
    }
    for (i, (a, b)) in source.faces.iter().zip(&imported.faces).enumerate() {
        let (sa, sb) = (&a.surface, &b.surface);
        assert_eq!(sa.degree_u, sb.degree_u, "face {i} degree_u");
        assert_eq!(sa.degree_v, sb.degree_v, "face {i} degree_v");
        assert_eq!(sa.control_points.len(), sb.control_points.len(), "face {i} rows");
        for (j, (ra, rb)) in sa.control_points.iter().zip(&sb.control_points).enumerate() {
            assert_eq!(ra.len(), rb.len(), "face {i} row {j} columns");
            for (k, (pa, pb)) in ra.iter().zip(rb).enumerate() {
                for d in 0..3 {
                    close(pa[d], pb[d], &format!("face {i} control point {j}/{k} coordinate {d}"));
                }
            }
        }
        for (j, (ra, rb)) in sa.weights.iter().zip(&sb.weights).enumerate() {
            for (k, (wa, wb)) in ra.iter().zip(rb).enumerate() {
                close(*wa, *wb, &format!("face {i} weight {j}/{k}"));
            }
        }
        for (ka, kb) in sa.knots_u.iter().zip(&sb.knots_u) {
            close(*ka, *kb, &format!("face {i} knot_u"));
        }
        for (ka, kb) in sa.knots_v.iter().zip(&sb.knots_v) {
            close(*ka, *kb, &format!("face {i} knot_v"));
        }
    }
}
