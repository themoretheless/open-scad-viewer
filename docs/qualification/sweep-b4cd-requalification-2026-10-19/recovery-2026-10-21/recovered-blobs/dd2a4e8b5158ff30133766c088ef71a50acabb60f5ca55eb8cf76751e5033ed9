use nurbs_core::{
    curve::Curve,
    surface::{Axis, Surface},
    surface_collar,
    surface_g2::Orientation,
    surface_join::Decision,
};
fn case() -> (Surface, Curve, Surface) {
    let s = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                [0., 1.]
                    .iter()
                    .map(|&v| vec![i as f64 / 2., v, if i == 2 { 0.25 + v } else { v }])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2], vec![0.7; 2], vec![1.; 2]],
        periodic_u: false,
        periodic_v: false,
    };
    let pc = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0.2, 0.4], vec![0.4, 0.5], vec![0.7, 0.3]],
        weights: vec![1., 0.7, 1.2],
        periodic: false,
    };
    let c = surface_collar::construct(&s, &pc, [0.05, 0.1], 1e-7, 2000000)
        .unwrap()
        .surface;
    (s, pc, c)
}
fn inspect(s: &Surface, pc: &Curve, c: &Surface) -> surface_collar::G2Report {
    surface_collar::inspect_g2(s, pc, [0.05, 0.1], c, [1e-7, 1e-5, 1e-4], 100000, 2000000).unwrap()
}
fn check_independent_formula(pc: &Curve, c: &Surface, sign: f64, upper: [f64; 3]) {
    for t in [0., 0.13, 0.5, 0.81, 1.] {
        let u = pc.evaluate(t).unwrap().point[0];
        let (w, dw, ddw) = (1. - 0.6 * u + 0.6 * u * u, -0.6 + 1.2 * u, 1.2);
        let derivative = |n: f64, dn: f64, ddn: f64| {
            let cross = dn * w - n * dw;
            (
                cross / (w * w),
                (ddn * w - n * ddw) / (w * w) - 2. * cross * dw / (w * w * w),
            )
        };
        let (x, xx) = derivative(0.7 * u + 0.3 * u * u, 0.7 + 0.6 * u, 0.6);
        let (z, zz) = derivative(0.25 * u * u, 0.5 * u, 0.5);
        let length = (z * z + 2. * x * x).sqrt();
        let normal = [-z / length, -x / length, x / length].map(|v| sign * v);
        let dual = [
            2. * x / (length * length),
            -z / (length * length),
            z / (length * length),
        ];
        let second = sign * (x * zz - z * xx) / length;
        let actual = nurbs_core::surface_shape_operator::at(
            c,
            [t, 0.],
            [nurbs_core::curve_differential::Side::Automatic; 2],
        )
        .unwrap();
        let n = actual.normal.unwrap();
        let b = actual.tensor.unwrap();
        let normal_error = (0..3)
            .map(|k| {
                (n[k][0] - normal[k])
                    .abs()
                    .max((n[k][1] - normal[k]).abs())
                    .powi(2)
            })
            .sum::<f64>()
            .sqrt();
        let tensor_error = (0..3)
            .flat_map(|i| {
                (0..3).map(move |j| {
                    let expected = second * dual[i] * dual[j];
                    (b[i][j][0] - expected)
                        .abs()
                        .max((b[i][j][1] - expected).abs())
                        .powi(2)
                })
            })
            .sum::<f64>()
            .sqrt();
        assert!(normal_error <= upper[1] && tensor_error <= upper[2]);
    }
}
#[test]
fn actual_candidate_gauge_reversal_and_budgets_keep_original_source_authority() {
    let (s, pc, c) = case();
    let saved = (s.clone(), pc.clone(), c.clone());
    let proof = inspect(&s, &pc, &c);
    assert_eq!(proof.decision, Decision::WithinTolerance);
    assert_eq!(proof.orientation, Some(Orientation::Same));
    check_independent_formula(&pc, &c, 1., proof.upper.unwrap());
    let mut gauged = c.clone();
    for row in &mut gauged.weights {
        for w in row {
            *w *= 2.;
        }
    }
    let gauge_saved = gauged.clone();
    assert_eq!(
        inspect(&s, &pc, &gauged).decision,
        Decision::WithinTolerance
    );
    assert_eq!(gauged, gauge_saved);
    let reversed = pc.reverse().unwrap();
    let reversed_surface = nurbs_core::surface_edit::reverse(&c, Axis::U).unwrap();
    let opposite = inspect(&s, &reversed, &reversed_surface);
    assert_eq!(opposite.decision, Decision::WithinTolerance);
    assert_eq!(opposite.orientation, Some(Orientation::Opposite));
    check_independent_formula(&reversed, &reversed_surface, -1., opposite.upper.unwrap());
    let mut chart = s.clone();
    let mut chart_pc = pc.clone();
    for k in &mut chart.knots_u {
        *k = 2. + 2. * *k;
    }
    for k in &mut chart.knots_v {
        *k = -1. + 4. * *k;
    }
    for point in &mut chart_pc.control_points {
        point[0] = 2. + 2. * point[0];
        point[1] = -1. + 4. * point[1];
    }
    let scaled = surface_collar::inspect_g2(
        &chart,
        &chart_pc,
        [0.1, 0.4],
        &c,
        [1e-7, 1e-5, 1e-4],
        100000,
        2000000,
    )
    .unwrap();
    assert_eq!(scaled.decision, Decision::WithinTolerance);
    check_independent_formula(&pc, &c, 1., scaled.upper.unwrap());
    let angle = 0.37_f64;
    let pose = |s: &Surface| {
        let mut surface = s.clone();
        for row in &mut surface.control_points {
            for p in row {
                let old = p.clone();
                *p = vec![
                    angle.cos() * old[0] + angle.sin() * old[2] + 2.,
                    old[1] - 3.,
                    -angle.sin() * old[0] + angle.cos() * old[2] + 1.,
                ];
            }
        }
        surface
    };
    let posed = inspect(&pose(&s), &pc, &pose(&c));
    assert_eq!(posed.decision, Decision::WithinTolerance);
    assert_eq!(posed.orientation, Some(Orientation::Same));
    let mut displaced = pose(&c);
    for row in &mut displaced.control_points {
        for point in row {
            point[0] += 0.001;
        }
    }
    let wrong_pose = inspect(&pose(&s), &pc, &displaced);
    assert_eq!(
        wrong_pose.decision,
        Decision::CorrespondenceExceedsTolerance
    );
    assert!(wrong_pose.witness_parameter.is_some());
    let scarce = surface_collar::inspect_g2(
        &s,
        &pc,
        [0.05, 0.1],
        &c,
        [1e-7, 1e-5, 1e-4],
        proof.cells - 1,
        2000000,
    )
    .unwrap();
    assert_eq!(scarce.decision, Decision::Unresolved);
    assert!(scarce.upper.is_none());
    let scarce = surface_collar::inspect_g2(
        &s,
        &pc,
        [0.05, 0.1],
        &c,
        [1e-7, 1e-5, 1e-4],
        100000,
        proof.coefficient_work - 1,
    )
    .unwrap();
    assert_eq!(scarce.decision, Decision::Unresolved);
    assert!(scarce.upper.is_none());
    assert_eq!((s, pc, c), saved);
}
#[test]
fn changed_original_surface_and_pcurve_are_freshly_recomputed() {
    let (s, pc, c) = case();
    let mut changed = s.clone();
    changed.control_points[1][0][2] += 0.01;
    let bad = inspect(&changed, &pc, &c);
    assert_eq!(bad.decision, Decision::CorrespondenceExceedsTolerance);
    assert!(bad.witness_parameter.is_some());
    let mut changed_pc = pc.clone();
    changed_pc.control_points[1][0] += 0.03;
    let bad = inspect(&s, &changed_pc, &c);
    assert_eq!(bad.decision, Decision::CorrespondenceExceedsTolerance);
    assert!(bad.witness_parameter.is_some());
}
