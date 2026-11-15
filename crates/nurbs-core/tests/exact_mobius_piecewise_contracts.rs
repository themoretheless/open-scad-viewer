use nurbs_core::{
    curve::Curve,
    editing::exact_projective::{
        Refusal, mobius_piecewise_bezier_curve, mobius_piecewise_bezier_surface,
    },
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn curve() -> Curve {
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 2., 2., 3., 3., 3.],
        control_points: vec![
            vec![0., 0.],
            vec![1., 1.],
            vec![2., 0.],
            vec![3., 1.],
            vec![4., 0.],
        ],
        weights: vec![1., 2., 1., 1., 1.],
        periodic: false,
    }
}

fn clock(t: f64, seam: f64, lambda: [f64; 2]) -> f64 {
    let (a, b, l) = if t < seam {
        (0., seam, lambda[0])
    } else {
        (seam, 3., lambda[1])
    };
    let x = (t - a) / (b - a);
    a + (b - a) * l * x / (1. + (l - 1.) * x)
}

#[test]
fn local_clocks_keep_the_absolute_seam_and_both_rational_span_images() {
    let source = curve();
    let r = mobius_piecewise_bezier_curve(&source, &[2., 0.5], WORK).unwrap();
    let out = r.geometry.unwrap_or_else(|| panic!("{:?}", r.refusal));
    assert_eq!(out.control_points, source.control_points);
    assert_eq!(out.knots, source.knots);
    assert_eq!(
        out.weights
            .iter()
            .map(|w| w / out.weights[0])
            .collect::<Vec<_>>(),
        [1., 4., 4., 2., 1.]
    );
    for new in [0., 0.25, 1.75, 2., 2.5, 3.] {
        let a = out.evaluate(new).unwrap().point;
        let b = source.evaluate(clock(new, 2., [2., 0.5])).unwrap().point;
        assert!(a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-14));
    }
    let restored = mobius_piecewise_bezier_curve(&out, &[0.5, 2.], WORK)
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
fn tensor_local_clocks_share_one_gauge_at_the_crossing_of_two_seams() {
    let c = curve();
    let source = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: c.knots,
        knots_v: vec![0., 0., 1., 3., 3.],
        control_points: c
            .control_points
            .iter()
            .map(|p| {
                [0., 1., 3.]
                    .map(|v| vec![p[0], v, p[1] * (1. + v)])
                    .to_vec()
            })
            .collect(),
        weights: c.weights.iter().map(|&w| vec![w; 3]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let r = mobius_piecewise_bezier_surface(&source, &[2., 0.5], &[0.5, 4.], WORK).unwrap();
    let out = r.geometry.unwrap_or_else(|| panic!("{:?}", r.refusal));
    assert_eq!(out.knots_u, source.knots_u);
    assert_eq!(out.knots_v, source.knots_v);
    assert_eq!(out.weights[2][1] / out.weights[0][0], 2.);
    for (u, v) in [(0., 0.), (1., 0.5), (2., 1.), (2.5, 2.), (3., 3.)] {
        let actual = out.evaluate(u, v).unwrap().point;
        let expected = source
            .evaluate(clock(u, 2., [2., 0.5]), clock(v, 1., [0.5, 4.]))
            .unwrap()
            .point;
        assert!(
            actual
                .iter()
                .zip(expected)
                .all(|(x, y)| (x - y).abs() < 1e-14)
        );
    }
}

#[test]
fn unsupported_span_basis_and_insufficient_work_are_refused() {
    let source = curve();
    assert!(mobius_piecewise_bezier_curve(&source, &[1.], WORK).is_err());
    let full = mobius_piecewise_bezier_curve(&source, &[2., 0.5], WORK).unwrap();
    let short = mobius_piecewise_bezier_curve(&source, &[2., 0.5], full.work_used - 1).unwrap();
    assert_eq!(short.refusal, Some(Refusal::ResourceLimit));
    assert!(short.geometry.is_none());
    let mut smooth = source;
    smooth.knots.remove(3);
    smooth.control_points.remove(2);
    smooth.weights.remove(2);
    assert!(mobius_piecewise_bezier_curve(&smooth, &[2., 0.5], WORK).is_err());
}

#[test]
fn curve_relayout_moves_a_seam_and_preserves_both_span_images() {
    use nurbs_core::editing::exact_projective::{BezierSpanLayout, relayout_bezier_curve};
    let source = curve();
    let out = relayout_bezier_curve(
        &source,
        BezierSpanLayout {
            breaks: &[-2., 4., 10.],
            factors: &[2., 0.5],
        },
        WORK,
    )
    .unwrap()
    .geometry
    .unwrap();
    assert_eq!(out.knots, [-2., -2., -2., 4., 4., 10., 10., 10.]);
    for (new, old) in [(-2., 0.), (0., 1.), (4., 2.), (7., 7. / 3.), (10., 3.)] {
        let a = out.evaluate(new).unwrap().point;
        let b = source.evaluate(old).unwrap().point;
        assert!(
            a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-14),
            "{new} -> {old}"
        );
    }
}

#[test]
fn tensor_relayout_changes_both_absolute_axes_and_refuses_invalid_partitions() {
    use nurbs_core::editing::exact_projective::{BezierSpanLayout, relayout_bezier_surface};
    let source = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [0., 1.]
            .map(|u| [0., 1.].map(|v| vec![u, v, u * v]).to_vec())
            .to_vec(),
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let u = BezierSpanLayout {
        breaks: &[7., 9.],
        factors: &[2.],
    };
    let v = BezierSpanLayout {
        breaks: &[-3., 3.],
        factors: &[0.5],
    };
    let r = relayout_bezier_surface(&source, [u, v], WORK).unwrap();
    let out = r.geometry.unwrap();
    let a = out.evaluate(8., 0.).unwrap().point;
    let b = source.evaluate(2. / 3., 1. / 3.).unwrap().point;
    assert!(a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-14));
    assert!(
        relayout_bezier_surface(
            &source,
            [
                BezierSpanLayout {
                    breaks: &[9., 7.],
                    ..u
                },
                v
            ],
            WORK
        )
        .is_err()
    );
    let short = relayout_bezier_surface(&source, [u, v], r.work_used - 1).unwrap();
    assert_eq!(short.refusal, Some(Refusal::ResourceLimit));
    assert!(short.geometry.is_none());
}
