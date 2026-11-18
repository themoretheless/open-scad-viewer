#[path = "../examples/support/curved_trim_fixture.rs"]
mod fixture;
use brep_core::{feature_ops::retrim::{self, Limits}, watertight};
use nurbs_core::{patches, surface::Surface};
fn cutter() -> Surface {
    let square = [1., -2., 4.];
    patches::bezier(
        (0..3)
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
        None,
    )
    .unwrap()
}
#[test]
fn original_region_incidence_drives_curved_retrim_and_closed_replacement() {
    let source = fixture::build();
    let before = source.clone();
    let surface = cutter();
    let out = retrim::replace_region(&source, &[1], &surface, Limits::default()).unwrap();
    assert_eq!(source, before);
    assert_eq!(out.model().faces.len(), 6);
    assert_eq!(out.neighbors().len(), 4);
    assert_eq!(out.roots().len(), 4);
    assert_eq!(out.removed_faces(), &[source.1.faces[1]]);
    assert_eq!(out.model().faces[out.replacement_face()].surface, surface);
    assert_eq!(
        out.closure().closure.status,
        watertight::Closure::WithinTolerance
    );
    assert!(out.audit().ok);
    let (cells, exact) = out.work();
    assert!(cells <= 100000 && exact <= 1000000);
    for &(old, face) in out.neighbors() {
        let original = source.1.faces.iter().position(|id| *id == old).unwrap();
        assert_eq!(
            out.model().faces[face].surface,
            source.faces[original].surface
        );
    }
    for &(_, _, vertex) in out.roots() {
        assert_eq!(
            out.model()
                .edges
                .iter()
                .filter(|e| e.vertices.contains(&vertex))
                .count(),
            3
        );
    }
    let v = brep_core::analysis::mass_properties(out.model(), 1e-6, 2000000)
        .unwrap()
        .signed_volume_mm3;
    let expected = 0.8 + 0.125 / 3. + 0.02 / 3.;
    assert!((v - expected).abs() < 1e-5, "{v} versus {expected}");
}
#[test]
fn refused_retrim_does_not_mutate_source_or_publish_partial_sectors() {
    let source = fixture::build();
    let before = source.clone();
    let surface = cutter();
    for faces in [vec![], vec![1, 1], vec![999], vec![0, 1]] {
        assert!(retrim::replace_region(&source, &faces, &surface, Limits::default()).is_err());
    }
    assert!(
        retrim::replace_region(
            &source,
            &[1],
            &surface,
            Limits {
                max_cells: 32,
                ..Limits::default()
            }
        )
        .is_err()
    );
    let mut misses = surface.clone();
    for p in misses.control_points.iter_mut().flatten() {
        p[2] += 10.;
    }
    assert!(retrim::replace_region(&source, &[1], &misses, Limits::default()).is_err());
    assert_eq!(source, before);
}
#[test]
fn original_face_chart_transposition_does_not_change_region_intent() {
    let mut source = fixture::build();
    fixture::transpose_charts(&mut source);
    let before = source.clone();
    let out = retrim::replace_region(&source, &[1], &cutter(), Limits::default()).unwrap();
    assert_eq!(source, before);
    assert_eq!(out.neighbors().len(), 4);
    assert_eq!(out.roots().len(), 4);
    assert!(out.audit().ok);
}
