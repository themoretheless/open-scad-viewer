use brep_core::{
    feature_ops::full_round_curved::{build, source_caps},
    profile_certificate::original_restriction::plane,
};
use nurbs_core::{curve::Curve, surface::Surface};
fn surface() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn curve(points: Vec<Vec<f64>>) -> Curve {
    let degree = points.len() - 1;
    Curve {
        degree,
        knots: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
        weights: vec![1.; points.len()],
        control_points: points,
        periodic: false,
    }
}
#[test]
fn exact_box_caps_have_strict_boundary_identity_and_outward_normals() {
    let p = Curve::from_polyline(vec![
        vec![0., 0.],
        vec![4., 0.],
        vec![4., 8.],
        vec![0., 8.],
        vec![0., 0.],
    ])
    .unwrap();
    let source = brep_core::prism::extrude(&[vec![p]], 0., 6.).unwrap();
    let proof = source_caps::qualify(&source).unwrap();
    assert_eq!(proof.caps().len(), 2);
    assert!(proof.exact());
    proof.require_exact().unwrap().reverify().unwrap();
}
#[test]
fn curved_native_caps_keep_nonzero_seam_bound_instead_of_claiming_exact_closure() {
    let q = |p: [[f64; 2]; 3]| Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: p.map(|p| p.to_vec()).to_vec(),
        weights: vec![1., 0.7, 1.],
        periodic: false,
    };
    let line =
        |a: [f64; 2], b: [f64; 2]| Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap();
    let source = brep_core::prism::extrude(
        &[vec![
            line([0., 0.], [4., 0.]),
            q([[4., 0.], [5., 4.], [4., 8.]]),
            line([4., 8.], [0., 8.]),
            q([[0., 8.], [-1., 4.], [0., 0.]]),
        ]],
        0.,
        6.,
    )
    .unwrap();
    let before = source.clone();
    let r = build(&source, [3, 2, 1]).unwrap();
    let proof = r.qualify_source_caps().unwrap();
    assert_eq!(source, before);
    assert!(!proof.exact());
    assert!(proof
        .caps()
        .iter()
        .flat_map(|c| c.boundaries())
        .any(|b| !b.proof().exact()));
    proof.reverify().unwrap();
    let canonical = proof.canonical_pullbacks().unwrap();
    assert_eq!(canonical.original().source(), &before);
    assert!(!canonical.original().exact());
    assert_eq!(canonical.caps().len(), 2);
    for cap in canonical.caps() {
        let original = proof
            .caps()
            .iter()
            .find(|c| c.face() == cap.face())
            .unwrap();
        assert_eq!(cap.boundaries().len(), original.boundaries().len());
        for (a, b) in cap.boundaries().iter().zip(original.boundaries()) {
            assert_eq!(a.edge(), b.edge());
            assert_eq!(a.coedge(), b.coedge());
            assert_eq!(a.inverse().world_curve(), b.proof().world_curve());
            assert_eq!(a.inverse().chart().surface(), original.chart().surface());
        }
    }
    canonical.reverify().unwrap();
    assert!(proof.require_exact().is_err());
}
#[test]
fn actual_bilinear_mixed_term_is_preserved_in_continuous_boundary_identity() {
    let mut s = surface();
    s.control_points[1][1][0] = 1.125;
    let uv = curve(vec![vec![0., 0.], vec![1., 1.]]);
    let world = curve(vec![vec![0., 0.], vec![0.5, 0.5], vec![1.125, 1.]]);
    let proof = plane::qualify(&s, &uv, &world, 0.).unwrap();
    assert!(proof.exact());
    assert_eq!(
        proof.chart().mixed_xy_coefficient(),
        [
            ["1".to_string(), "8".to_string()],
            ["0".to_string(), "1".to_string()]
        ]
    );
    assert_eq!(proof.chart().surface(), &s);
    proof.reverify().unwrap();
}
#[test]
fn whole_domain_proof_allows_even_tangency_despite_control_hull_outside_chart() {
    let uv = curve(vec![vec![0.25, 0.5], vec![1.75, 0.5], vec![0.25, 0.5]]);
    plane::qualify(&surface(), &uv, &uv, 0.)
        .unwrap()
        .reverify()
        .unwrap();
    let outside = curve(vec![vec![0.25, 0.5], vec![1.875, 0.5], vec![0.25, 0.5]]);
    assert!(plane::qualify(&surface(), &outside, &outside, 0.).is_err());
}
#[test]
fn folded_chart_and_nonzero_tolerance_disguised_as_exact_are_refused() {
    let mut folded = surface();
    folded.control_points[1][1] = vec![-1., -1., 0.];
    assert!(plane::chart(&folded).is_err());
    let uv = curve(vec![vec![0., 0.], vec![1., 0.]]);
    let delta = 2f64.powi(-40);
    let world = curve(vec![vec![0., delta], vec![1., delta]]);
    let proof = plane::qualify(&surface(), &uv, &world, 2f64.powi(-30)).unwrap();
    assert!(!proof.exact());
    proof.reverify().unwrap();
    assert!(plane::qualify(&surface(), &uv, &world, 0.).is_err());
}
