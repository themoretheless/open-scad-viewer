use brep_core::{
    Model,
    feature_ops::profile_blend::{self, Limits},
    watertight,
};
use nurbs_core::{curve::Curve, surface_join::Decision};
fn source(curved: bool) -> Model {
    let bottom = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![0., 0.],
            vec![2., if curved { -1. } else { 0. }],
            vec![4., 0.],
        ],
        weights: vec![1.; 3],
        periodic: false,
    };
    let line =
        |a: [f64; 2], b: [f64; 2]| Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap();
    brep_core::prism::extrude(
        &[vec![
            bottom,
            line([4., 0.], [4., 4.]),
            line([4., 4.], [0., 4.]),
            line([0., 4.], [0., 0.]),
        ]],
        0.,
        3.,
    )
    .unwrap()
}
fn preceding(m: &Model) -> usize {
    let p = brep_core::prism::recognize(m).unwrap().unwrap();
    p.loops[0]
        .iter()
        .position(|c| {
            let e = c.evaluate(c.domain()[1]).unwrap().point;
            (e[0] - 4.).hypot(e[1]) < 1e-10
        })
        .unwrap()
}
fn volume(m: &Model) -> f64 {
    brep_core::analysis::mass_properties(m, 1e-6, 2000000)
        .unwrap()
        .signed_volume_mm3
}
#[test]
fn curved_support_round_authors_closed_solid_with_whole_radius_and_g1_proofs() {
    let source = source(true);
    let before = source.clone();
    let index = preceding(&source);
    let out = profile_blend::round_corner(&source, 0, index, 0.5, Limits::default()).unwrap();
    assert_eq!(source, before);
    assert_eq!(out.model().faces.len(), 7);
    assert_eq!(
        out.closure().closure.status,
        watertight::Closure::WithinTolerance
    );
    assert!(out.audit().ok);
    assert_eq!(out.tangent_seams().len(), 2);
    for seam in out.tangent_seams() {
        assert_eq!(seam.decision, Decision::WithinTolerance);
        assert!(seam.positions.proven && seam.regularity_proven);
    }
    assert!(out.radius_certificate().unwrap().error_upper() <= source.tolerance_mm);
    let (cells, work) = out.proof_work();
    assert!(cells <= 100000 && work <= 1000000);
    let result_volume = volume(out.model());
    assert!(result_volume < volume(&source) && result_volume > 40.);
    assert!(out.model().faces.iter().any(|f| f.surface.degree_u >= 2));
}
#[test]
fn straight_control_case_matches_independent_round_volume() {
    let source = source(false);
    let out = profile_blend::round_corner(&source, 0, preceding(&source), 0.5, Limits::default())
        .unwrap();
    let expected = 48. - (1. - std::f64::consts::FRAC_PI_4) * 0.25 * 3.;
    assert!((volume(out.model()) - expected).abs() < 1e-5);
}
#[test]
fn curved_profile_chamfer_uses_original_arc_length_setbacks_and_closes_caps() {
    let source = source(true);
    let before = source.clone();
    let out = profile_blend::chamfer_corner(
        &source,
        0,
        preceding(&source),
        [0.4, 0.6],
        Limits::default(),
    )
    .unwrap();
    assert_eq!(source, before);
    assert!(out.audit().ok);
    assert!(out.radius_certificate().is_none());
    assert!(
        out.corner()
            .setback_errors()
            .unwrap()
            .iter()
            .all(|e| *e <= source.tolerance_mm / 4.)
    );
    assert_eq!(
        out.closure().closure.status,
        watertight::Closure::WithinTolerance
    );
    assert!(volume(out.model()) < volume(&source));
}
#[test]
fn oversized_and_invalid_blends_refuse_without_mutating_original() {
    let source = source(true);
    let before = source.clone();
    let i = preceding(&source);
    for r in [-1., 0., 100., f64::NAN] {
        assert!(profile_blend::round_corner(&source, 0, i, r, Limits::default()).is_err());
    }
    assert!(profile_blend::round_corner(&source, 999, i, 0.5, Limits::default()).is_err());
    assert!(profile_blend::chamfer_corner(&source, 0, i, [100., 0.6], Limits::default()).is_err());
    assert_eq!(source, before);
}

#[test]
fn nonuniform_rational_support_receives_the_same_closed_radius_and_tangent_gates() {
    let source = source(true);
    let mut profile = brep_core::prism::recognize(&source).unwrap().unwrap();
    let i = preceding(&source);
    profile.loops[0][i].weights[1] = 1.3;
    let rational = brep_core::prism::extrude(&profile.loops, profile.z_min, profile.z_max).unwrap();
    let before = rational.clone();
    let out =
        profile_blend::round_corner(&rational, 0, preceding(&rational), 0.5, Limits::default())
            .unwrap();
    assert_eq!(rational, before);
    assert!(out.audit().ok);
    assert!(out.radius_certificate().unwrap().error_upper() <= rational.tolerance_mm);
    assert!(
        out.tangent_seams()
            .iter()
            .all(|s| s.positions.proven && s.decision == Decision::WithinTolerance)
    );
}
