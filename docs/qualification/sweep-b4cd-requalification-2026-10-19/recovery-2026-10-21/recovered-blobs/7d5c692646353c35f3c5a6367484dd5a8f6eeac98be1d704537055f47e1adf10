#[path = "../examples/support/curved_trim_fixture.rs"]
mod fixture;
#[path = "../examples/support/fiber_cut_fixture.rs"]
mod sheets;
use brep_core::{face_cut, face_domain::FaceDomain, watertight};
use nurbs_core::{ss_intersection::FiberAxis, surface::Surface};
pub fn cutter() -> brep_core::Model {
    let square = [1., -2., 4.];
    sheets::sheet(Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        vec![
                            -1. + 1.5 * i as f64,
                            -1. + 1.5 * j as f64,
                            -0.2 + 0.125 * square[i] + 0.02 * square[j],
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    })
}
#[test]
fn inferred_network_charts_close_and_preserve_shared_original_roots() {
    let source = fixture::build();
    let cutter = cutter();
    let before = (source.clone(), cutter.clone());
    let requests = [2, 3, 4, 5].map(|face| face_cut::AutomaticFiberCapRequest {
        face,
        remove_uv: [0.5, 0.95],
    });
    let out = face_cut::trim_fiber_and_cap_automatic(
        &source,
        &requests,
        &[1],
        &cutter,
        0,
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    assert!(out.cap.replacement.trims.all_valid && out.cap.interior.certified);
    assert_eq!(
        out.cap.replacement.closure.closure.status,
        watertight::Closure::WithinTolerance
    );
    assert!(out.cells <= 100000 && out.exact_work <= 1000000 && out.evaluations <= 4096);
    assert_eq!(out.root_vertices.len(), 4);
    let model = &out.cap.replacement.model;
    assert_eq!(model.faces.len(), 6);
    for &(_, _, vertex) in &out.root_vertices {
        assert_eq!(
            model
                .edges
                .iter()
                .filter(|e| e.vertices.contains(&vertex))
                .count(),
            3
        );
    }
    let restored: brep_core::Model =
        value_codec::from_str(&value_codec::to_string(model).unwrap()).unwrap();
    assert_eq!(&restored, model);
    assert!(
        face_cut::trim_fiber_and_cap_automatic(
            &source,
            &requests,
            &[1],
            &cutter,
            0,
            1e-8,
            32,
            1000000,
        )
        .is_err()
    );
    assert_eq!((source, cutter), before);
}
#[test]
fn shared_curved_sections_replace_one_region_with_an_owned_curved_cap() {
    let source = fixture::build();
    let before = source.clone();
    let cutter = cutter();
    let requests = [2, 3, 4, 5].map(|face| face_cut::FiberFaceRequest {
        face,
        parameter_box: [[0., 1.], [0.1, 0.95], [0.25, 0.75], [0.25, 0.75]],
        axis: FiberAxis::FirstU,
    });
    let out =
        face_cut::partition_fiber_network(&source, &requests, &cutter, 0, 1e-8, 100000, 1000000)
            .unwrap();
    println!(
        "network cells={} roots={} edges={:?} exact={}",
        out.cells,
        out.root_vertices.len(),
        out.sections
            .iter()
            .map(|s| s.edges.len())
            .collect::<Vec<_>>(),
        out.exact_work
    );
    assert_eq!(out.root_vertices.len(), 4);
    for &(_, _, v) in &out.root_vertices {
        assert_eq!(
            out.sections
                .iter()
                .filter(|s| s.endpoints.contains(&v))
                .count(),
            2
        );
    }
    let mut cells = out.cells;
    let mut selected = vec![1];
    for section in &out.sections {
        let mut owners = Vec::new();
        for face in section.faces {
            let r = FaceDomain::new(&out.model, face, 1e-8)
                .unwrap()
                .classify([[0.5; 2], [0.95; 2]], 100000 - cells)
                .unwrap();
            cells += r.cells;
            if r.location == nurbs_core::trim_domain::Location::Inside && r.winding == Some(1) {
                owners.push(face);
            }
        }
        assert_eq!(owners.len(), 1);
        selected.push(owners[0]);
    }
    let cap = brep_core::region_replacement::cap_region(
        &out.model,
        &selected,
        &cutter.faces[0].surface,
        1e-8,
        100000 - cells,
        1000000 - out.exact_work,
    )
    .unwrap();
    cells += cap.cells;
    assert!(cells <= 100000 && out.exact_work + cap.replacement.exact_work <= 1000000);
    assert!(cap.replacement.trims.all_valid && cap.interior.certified);
    assert_eq!(
        cap.replacement.closure.closure.status,
        watertight::Closure::WithinTolerance
    );
    assert_eq!(cap.replacement.model.faces.len(), 6);
    let restored: brep_core::Model =
        value_codec::from_str(&value_codec::to_string(&cap.replacement.model).unwrap()).unwrap();
    assert_eq!(restored, cap.replacement.model);
    assert!(
        face_cut::partition_fiber_network(
            &source,
            &requests,
            &cutter,
            0,
            1e-8,
            out.cells - 1,
            1000000
        )
        .is_err()
    );
    let cap_requests = requests.map(|section| face_cut::FiberCapRequest {
        section,
        remove_uv: [0.5, 0.95],
    });
    let atomic = face_cut::trim_fiber_and_cap(
        &source,
        &cap_requests,
        &[1],
        &cutter,
        0,
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    assert_eq!(atomic.cap.replacement.model, cap.replacement.model);
    assert!(atomic.cells <= 100000 && atomic.evaluations <= 4096);
    for &(edge, ordinal, vertex) in &atomic.root_vertices {
        let old = out
            .root_vertices
            .iter()
            .find(|&&(e, o, _)| e == edge && o == ordinal)
            .unwrap()
            .2;
        assert_eq!(
            atomic.cap.replacement.model.vertices[vertex],
            out.model.vertices[old]
        );
        assert_eq!(
            atomic.cap.replacement.model.1.vertices[vertex],
            out.model.1.vertices[old]
        );
    }
    assert_eq!(source, before);
}
