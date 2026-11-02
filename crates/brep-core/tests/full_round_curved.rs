use brep_core::{Model, feature_ops::full_round_curved::build};
use nurbs_core::curve::Curve;
fn source(weight: f64) -> (Model, [usize; 3]) {
    let curve = |points: [[f64; 2]; 3]| Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: points.map(|p| p.to_vec()).to_vec(),
        weights: vec![1., weight, 1.],
        periodic: false,
    };
    let line =
        |a: [f64; 2], b: [f64; 2]| Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap();
    let profile = vec![
        line([0., 0.], [4., 0.]),
        curve([[4., 0.], [5., 4.], [4., 8.]]),
        line([4., 8.], [0., 8.]),
        curve([[0., 8.], [-1., 4.], [0., 0.]]),
    ];
    let m = brep_core::prism::extrude(&[profile], 0., 6.).unwrap();
    let side = |left: bool| {
        m.faces
            .iter()
            .position(|f| {
                f.surface.degree_u == 2
                    && f.surface
                        .control_points
                        .iter()
                        .flatten()
                        .all(|p| if left { p[0] <= 0. } else { p[0] >= 4. })
            })
            .unwrap()
    };
    let middle = m
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[1] == 8.)
        })
        .unwrap();
    let faces = [side(true), middle, side(false)];
    (m, faces)
}
#[test]
fn full_round_builds_closed_body_on_original_polynomial_and_rational_supports() {
    for weight in [1., 0.7] {
        let (source, faces) = source(weight);
        let before = source.clone();
        let r = build(&source, faces).unwrap();
        assert_eq!(source, before);
        assert_eq!(r.source(), &source);
        for support in r.retained_supports() {
            support.reverify().unwrap();
            assert_eq!(support.source().weights, vec![1., weight, 1.]);
            assert_eq!(support.spans().len(), 1);
        }
        assert_eq!(r.exact_profile().len(), 1);
        let exact_wire = &r.exact_profile()[0];
        assert_eq!(exact_wire.editable().iter().filter(|&&v| v).count(), 2);
        exact_wire.reverify().unwrap();
        let smooth = r.exact_smoothness();
        assert_eq!(smooth.wire().pieces(), exact_wire.pieces());
        assert_eq!(smooth.junctions().len(), 3);
        assert_eq!(smooth.regular_span_count(), exact_wire.pieces().iter().map(Vec::len).sum::<usize>());
        smooth.reverify().unwrap();
        let exact_shell = r.exact_shell();
        exact_shell.reverify().unwrap();
        let jordan = exact_shell.qualify_profile_jordan(
            brep_core::profile_certificate::original_restriction::wire::jordan::Limits::default(),
        ).unwrap();
        let spans = smooth.regular_span_count();
        assert_eq!(jordan.pairs().len(), spans * (spans + 1) / 2);
        jordan.reverify().unwrap();
        let solid = r.qualify_exact_solid(
            brep_core::profile_certificate::original_restriction::wire::jordan::Limits::default(),
            brep_core::profile_certificate::original_restriction::wire::nesting::Limits::default(),
        ).unwrap();
        assert_eq!(solid.model().bodies.len(), 1);
        assert_eq!(solid.model().faces, exact_shell.model().faces);
        solid.reverify().unwrap();
        assert_eq!(exact_shell.source_profiles()[0].pieces(), exact_wire.pieces());
        assert_eq!(exact_shell.wall_sources().len(), smooth.regular_span_count());
        assert_eq!(exact_shell.agreement().coedges, 6 * smooth.regular_span_count());
        assert!(exact_shell.model().bodies.is_empty());
        for (i, editable) in exact_wire.editable().iter().enumerate() {
            if !editable {
                assert_eq!(exact_wire.pieces()[i], exact_wire.sources()[i].spans());
            }
        }
        assert!(r.proposal().radius() > 2.);
        assert!(
            r.proposal()
                .source_error_upper()
                .iter()
                .all(|e| *e <= source.tolerance_mm / 32.)
        );
        assert_eq!(r.construction().support_faces, [faces[0], faces[2]]);
        let q = &r.construction().tangency;
        assert_eq!(q.model(), r.model());
        assert_eq!(q.seams().len(), 3);
        assert_eq!(q.envelopes().len(), 2);
        assert!(
            q.envelopes()
                .iter()
                .all(|(_, e)| e.radius().error_upper() <= source.tolerance_mm)
        );
        assert!(r.construction().audit.ok);
        r.model().validate().unwrap();
    }
}
#[test]
fn full_round_curved_refuses_wrong_original_face_ownership_atomically() {
    let (source, faces) = source(0.7);
    let before = source.clone();
    let wrong = source
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[1] == 0.)
        })
        .unwrap();
    assert!(build(&source, [wrong, faces[1], faces[2]]).is_err());
    assert!(build(&source, [faces[0], faces[1], faces[0]]).is_err());
    assert_eq!(source, before);
}
