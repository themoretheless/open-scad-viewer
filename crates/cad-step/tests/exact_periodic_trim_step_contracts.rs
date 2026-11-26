use brep_core::face_domain::FaceDomain;
use cad_step::{import_step_v9, periodic_trim_step::export_exact_periodic_trim_sheet_step};
use nurbs_core::{
    curve::Curve,
    surface::{Surface, extrude},
    trim_domain::Location,
};

fn support() -> Surface {
    extrude(
        &Curve {
            degree: 1,
            knots: vec![-1., 0., 1., 2., 3., 4., 5.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![1., 0., 0.],
                vec![1., 1., 0.],
                vec![0., 1., 0.],
                vec![0., 0., 0.],
            ],
            weights: vec![1.; 5],
            periodic: true,
        },
        [0., 0., 2.],
    )
    .unwrap()
}

#[test]
fn periodic_trim_step_round_trips_support_hole_and_exact_edge_pairs() {
    let source = support();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let hole = [
        [0.125, 0.25],
        [0.375, 0.25],
        [0.375, 0.75],
        [0.125, 0.75],
        [0.125, 0.25],
    ];
    let report =
        export_exact_periodic_trim_sheet_step(&source, &[&outer, &hole], 1e-8, 1_000_000).unwrap();
    let original = report.sheet.model.unwrap();
    let text = report.text.unwrap();
    if let Some(path) = std::env::var_os("STEP_PERIODIC_SHEET_OUTPUT") {
        std::fs::write(path, &text).expect("write independently inspectable STEP sheet");
    }
    assert!(text.contains("OPEN_SHELL"));
    assert!(text.contains("FACE_BOUND"));
    assert!(text.contains("PCURVE"));
    let (imported, _, _) = import_step_v9(&text).unwrap();
    imported.validate().unwrap();
    assert!(!imported.shells[0].closed);
    assert_eq!(imported.faces.len(), 1);
    assert_eq!(imported.faces[0].holes.len(), 1);
    assert_eq!(imported.faces[0].surface, original.faces[0].surface);
    for a in [original.faces[0].outer, original.faces[0].holes[0]] {
        let b = if a == original.faces[0].outer {
            imported.faces[0].outer
        } else {
            imported.faces[0].holes[0]
        };
        assert_eq!(
            original.loops[a].coedges.len(),
            imported.loops[b].coedges.len()
        );
        for (before, after) in original.loops[a]
            .coedges
            .iter()
            .zip(&imported.loops[b].coedges)
        {
            assert_eq!(before.pcurve, after.pcurve);
            assert_eq!(
                original.edges[before.edge].curve,
                imported.edges[after.edge].curve
            );
        }
    }
    let domain = FaceDomain::new(&imported, 0, 1e-8).unwrap();
    for (uv, expected) in [
        ([4., 0.5], Location::Inside),
        ([4.25, 0.5], Location::Outside),
    ] {
        assert_eq!(
            domain.classify(uv.map(|x| [x, x]), 1024).unwrap().location,
            expected
        );
    }
}

#[test]
fn periodic_trim_step_returns_no_text_for_a_refused_contour() {
    let bow = [[0., 0.], [1., 1.], [0., 1.], [1., 0.], [0., 0.]];
    let report =
        export_exact_periodic_trim_sheet_step(&support(), &[&bow], 1e-8, 1_000_000).unwrap();
    assert!(report.text.is_none());
    assert!(report.sheet.refusal.is_some());
}
