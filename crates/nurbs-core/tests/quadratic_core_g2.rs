use nurbs_core::{
    curve::Curve,
    original_graph_transition::{self as graph, Budget, CurvatureDecision, NormalRadiusDecision},
    surface::Surface,
};
fn sources() -> [Surface; 2] {
    std::array::from_fn(|side| Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..2)
            .map(|i| {
                vec![
                    vec![i as f64, 0., 0.],
                    vec![i as f64, if side == 0 { -1. } else { 1. }, -1. / 1024.],
                ]
            })
            .collect(),
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    })
}
fn width() -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![1. / 1024., 0., 0.]; 2],
        weights: vec![1.; 2],
        periodic: false,
    }
}
#[test]
fn rebuilt_profile_preserves_exact_source_jets_and_restores_declared_core_radius() {
    let s = sources();
    let w = width();
    let before = s.clone();
    let mut b = Budget::new(10_000_000).unwrap();
    let p = graph::build_quadratic_core_g2([&s[0], &s[1]], &w, &mut b).unwrap();
    assert_eq!(p.contact_jets(), [true; 2]);
    assert_eq!(p.profile_coefficients().len(), 25);
    for side in 0..2 {
        assert_eq!(p.source_contact(side, &mut b).unwrap().source(), &s[side]);
    }
    let (preview, error) = p.preview(&mut b).unwrap();
    assert_eq!(preview.degree_v, 24);
    assert!(error < 1e-12);
    let r = p
        .inspect_normal_radius(1., 0.001, [[0., 1.], [0.25, 0.75]], 1000, &mut b)
        .unwrap();
    println!(
        "rebuilt core radius: {:?}, cells={}, work={}, reason={}",
        r.decision, r.cells, r.coefficient_work, r.reason
    );
    assert_eq!(r.decision, NormalRadiusDecision::Within);
    assert!(r.unresolved_regions.is_empty());
    let k = p.inspect_curvature(10., 10_000, &mut b).unwrap();
    println!(
        "rebuilt curvature: {:?}, cells={}, work={}, upper={:?}",
        k.decision, k.cells, k.coefficient_work, k.absolute_curvature_upper
    );
    assert_eq!(k.decision, CurvatureDecision::Within);
    assert!(k.absolute_curvature_upper.unwrap() <= 10.);
    let full = p
        .inspect_normal_radius(1., 0.001, [[0., 1.]; 2], 1000, &mut b)
        .unwrap();
    assert_eq!(full.decision, NormalRadiusDecision::Exceeds);
    assert_eq!(s, before);
}
#[test]
fn quartic_comparison_still_exposes_the_original_two_thirds_radius() {
    let s = sources();
    let mut b = Budget::new(1_000_000).unwrap();
    let p = graph::build([&s[0], &s[1]], &width(), &mut b).unwrap();
    let r = p
        .inspect_normal_radius(1., 0.001, [[0., 1.], [0.25, 0.75]], 100, &mut b)
        .unwrap();
    assert_eq!(r.decision, NormalRadiusDecision::Exceeds);
    let radius = r.witness_radius_mm.unwrap();
    assert!(radius[0] <= 2. / 3. && 2. / 3. <= radius[1]);
}
