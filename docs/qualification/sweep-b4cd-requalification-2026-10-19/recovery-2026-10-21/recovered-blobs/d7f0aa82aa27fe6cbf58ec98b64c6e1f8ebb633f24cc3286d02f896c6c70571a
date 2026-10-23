use nurbs_core::{curve::Curve, sections};
#[test]
fn compatibility_preserves_independent_curves_and_enables_loft() {
    let a = Curve {
        degree: 1,
        knots: vec![2., 2., 5., 5.],
        control_points: vec![vec![0., 0., 0.], vec![4., 0., 0.]],
        weights: vec![1., 3.],
        periodic: false,
    };
    let b = nurbs_core::paths::bezier(
        vec![vec![0., 0., 2.], vec![2., 3., 2.], vec![4., 0., 2.]],
        Some(vec![1., 2., 1.]),
    )
    .unwrap()
    .insert(0.25, 1)
    .unwrap();
    let out = sections::compatible(&[a.clone(), b.clone()]).unwrap();
    assert_eq!(out[0].degree, 2);
    assert_eq!(out[0].knots, out[1].knots);
    assert_eq!(out[0].domain(), [0., 1.]);
    for i in 0..=200 {
        let t = i as f64 / 200.;
        let x = 12. * t / (1. + 2. * t);
        let p = out[0].evaluate(t).unwrap().point;
        assert!((p[0] - x).abs() < 1e-11);
        let w = (1. - t).powi(2) + 4. * t * (1. - t) + t * t;
        let q = out[1].evaluate(t).unwrap().point;
        assert!((q[0] - (8. * t * (1. - t) + 4. * t * t) / w).abs() < 1e-11);
        assert!((q[1] - 12. * t * (1. - t) / w).abs() < 1e-11);
    }
    let loft = nurbs_core::surface::loft_aligned(&[a, b]).unwrap();
    for i in 0..=50 {
        let t = i as f64 / 50.;
        for (j, c) in out.iter().enumerate() {
            let p = loft.evaluate(t, j as f64).unwrap().point;
            let q = c.evaluate(t).unwrap().point;
            assert!(p.iter().zip(q).all(|(x, y)| (*x - y).abs() < 1e-11));
        }
    }
}
#[test]
fn compatibility_refuses_bad_sets_and_accepts_2d() {
    let c = nurbs_core::primitives::line([0.; 3], [1., 0., 0.]).unwrap();
    assert!(sections::compatible(&[]).is_err());
    assert!(sections::compatible(&[c.clone()]).is_err());
    assert!(sections::compatible(&vec![c.clone(); 33]).is_err());
    let mut d = c.clone();
    for p in &mut d.control_points {
        p.pop();
    }
    assert!(sections::compatible(&[c, d.clone()]).is_err());
    assert!(sections::compatible(&[d.clone(), d]).is_ok());
}

#[test]
fn orientation_reports_reversal_and_ambiguous_sections() {
    use sections::Orientation;
    let a = nurbs_core::primitives::line([0., 0., 0.], [4., 0., 0.]).unwrap();
    let b = nurbs_core::primitives::line([4., 0., 2.], [0., 0., 2.]).unwrap();
    let tie = nurbs_core::primitives::line([2., -1., 3.], [2., 1., 3.]).unwrap();
    let result = sections::orient(&[a.clone(), b.clone(), tie.clone()], 1e-10).unwrap();
    assert_eq!(
        result.orientations,
        vec![
            Orientation::Anchor,
            Orientation::Reversed,
            Orientation::Ambiguous
        ]
    );
    assert_eq!(
        result.curves[1].evaluate(0.).unwrap().point,
        vec![0., 0., 2.]
    );
    assert_eq!(result.curves[2], tie);
    let tolerant = sections::orient(&[a.clone(), b], 100.).unwrap();
    assert_eq!(tolerant.orientations[1], Orientation::Ambiguous);
    assert!(sections::orient(&[a.clone(), a.clone()], -1.).is_err());
    assert!(sections::orient(&[a.clone(), a], f64::NAN).is_err());
}

#[test]
fn periodic_sections_clamp_without_changing_active_period() {
    let c = Curve {
        degree: 2,
        knots: (0..11).map(|i| i as f64).collect(),
        control_points: vec![
            vec![1., 0., 0.],
            vec![0.5, 1., 0.],
            vec![-0.5, 1., 0.],
            vec![-1., 0., 0.],
            vec![-0.5, -1., 0.],
            vec![0.5, -1., 0.],
            vec![1., 0., 0.],
            vec![0.5, 1., 0.],
        ],
        weights: vec![1., 2., 1., 2., 1., 2., 1., 2.],
        periodic: true,
    };
    let reversed = c.reverse().unwrap();
    let out = sections::compatible(&[c.clone(), reversed.clone()]).unwrap();
    assert_eq!(out[0].knots, out[1].knots);
    for (before, after) in [(&c, &out[0]), (&reversed, &out[1])] {
        assert!(!after.periodic);
        assert_eq!(after.domain(), [0., 1.]);
        for i in 0..=200 {
            let t = i as f64 / 200.;
            let p = before.evaluate(2. + 6. * t).unwrap().point;
            let q = after.evaluate(t).unwrap().point;
            assert!(p.iter().zip(q).all(|(x, y)| (*x - y).abs() < 1e-11));
        }
    }
}
#[test]
fn compatible_curves_can_exceed_surface_capacity() {
    let points = (0..64)
        .map(|i| [i as f64, (i % 2) as f64, 0.])
        .collect::<Vec<_>>();
    let c = nurbs_core::primitives::polyline(&points, false).unwrap();
    let pair = [c.clone(), c];
    let out = sections::compatible(&pair).unwrap();
    assert_eq!(out[0].control_points.len(), 64);
    assert!(nurbs_core::surface::loft_aligned(&pair).is_err());
}
