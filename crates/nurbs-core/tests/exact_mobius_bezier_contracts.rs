use nurbs_core::{
    curve::Curve,
    editing::exact_projective::{Refusal, mobius_bezier_curve, mobius_bezier_surface},
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn clock(a: f64, b: f64, t: f64, lambda: f64) -> f64 {
    a + (b - a) * lambda * t / (1. + (lambda - 1.) * t)
}

fn curve() -> Curve {
    Curve {
        degree: 2,
        knots: vec![2., 2., 2., 6., 6., 6.],
        control_points: vec![vec![0., 0.], vec![1., 1.], vec![0., 1.]],
        weights: vec![1., 1., 2.],
        periodic: false,
    }
}

#[test]
fn rational_curve_preserves_the_image_and_expected_clock() {
    let source = curve();
    let r = mobius_bezier_curve(&source, 2., WORK).unwrap();
    let out = r.geometry.unwrap_or_else(|| panic!("{:?}", r.refusal));
    assert_eq!(source.control_points, out.control_points);
    assert_eq!(source.knots, out.knots);
    assert_eq!(
        out.weights
            .iter()
            .map(|w| w / out.weights[0])
            .collect::<Vec<_>>(),
        [1., 2., 8.]
    );
    for t in [0., 0.125, 0.5, 0.875, 1.] {
        let actual = out.evaluate(2. + 4. * t).unwrap().point;
        let expected = source.evaluate(clock(2., 6., t, 2.)).unwrap().point;
        assert!(
            actual
                .iter()
                .zip(expected)
                .all(|(a, b)| (a - b).abs() < 1e-14)
        );
    }
    let restored = mobius_bezier_curve(&out, 0.5, WORK)
        .unwrap()
        .geometry
        .unwrap();
    assert_eq!(
        restored
            .weights
            .iter()
            .map(|w| w / restored.weights[0])
            .collect::<Vec<_>>(),
        source.weights
    );
}

#[test]
fn tensor_uses_one_common_gauge_for_independent_uv_clocks() {
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 6., 6.],
        knots_v: vec![7., 7., 9., 9.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 1.]],
        ],
        weights: vec![vec![1., 2.], vec![1., 2.]],
        periodic_u: false,
        periodic_v: false,
    };
    let r = mobius_bezier_surface(&s, [2., 0.5], WORK).unwrap();
    let out = r.geometry.unwrap();
    let base = out.weights[0][0];
    assert_eq!(
        out.weights
            .iter()
            .flatten()
            .map(|w| w / base)
            .collect::<Vec<_>>(),
        [1., 1., 2., 2.]
    );
    assert_eq!(out.control_points, s.control_points);
    for (u, v) in [(0., 0.), (0.25, 0.75), (0.5, 0.5), (1., 1.)] {
        let actual = out.evaluate(2. + 4. * u, 7. + 2. * v).unwrap().point;
        let expected = s
            .evaluate(clock(2., 6., u, 2.), clock(7., 9., v, 0.5))
            .unwrap()
            .point;
        assert!(
            actual
                .iter()
                .zip(expected)
                .all(|(a, b)| (a - b).abs() < 1e-14)
        );
    }
}

#[test]
fn exact_storage_refuses_a_nonrepresentable_weight_ratio() {
    let mut s = curve();
    s.degree = 1;
    s.knots = vec![2., 2., 6., 6.];
    s.control_points.remove(1);
    s.weights = vec![1., 1f64.next_up()];
    let r = mobius_bezier_curve(&s, 1f64.next_up(), WORK).unwrap();
    assert_eq!(r.refusal, Some(Refusal::NotRepresentable));
    assert!(r.geometry.is_none());
}

#[test]
fn nonpositive_clocks_and_short_work_cannot_create_an_output() {
    let s = curve();
    for invalid in [0., -1., f64::NAN] {
        assert!(mobius_bezier_curve(&s, invalid, WORK).is_err());
    }
    let r = mobius_bezier_curve(&s, 2., 1).unwrap();
    assert_eq!(r.refusal, Some(Refusal::ResourceLimit));
    assert!(r.geometry.is_none());
}
