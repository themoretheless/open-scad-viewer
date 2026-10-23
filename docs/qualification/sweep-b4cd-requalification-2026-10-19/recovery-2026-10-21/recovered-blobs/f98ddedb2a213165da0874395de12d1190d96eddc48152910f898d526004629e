use nurbs_core::{curve::Curve, surface::Surface};
use value_codec::json;
fn main() {
    let mut cases = Vec::new();
    for rotation in 0..3 {
        for shifted in [false, true] {
            for weight in [0.5, 0.75, 1.25] {
                for shear in [0., 0.25] {
                    for fold in [false, true] {
                        let mut edge = Curve {
                            degree: 2,
                            knots: vec![0., 0., 0., 1., 1., 1.],
                            control_points: vec![
                                vec![0., 0., 0.],
                                vec![0.5, 1., 0.],
                                vec![1., 0., 0.],
                            ],
                            weights: vec![1., weight, 1.],
                            periodic: false,
                        };
                        let make = |sign: f64| Surface {
                            degree_u: 1,
                            degree_v: 2,
                            knots_u: if shifted {
                                vec![-0.1, -0.1, 1.3, 1.3]
                            } else {
                                vec![0., 0., 1., 1.]
                            },
                            knots_v: if shifted {
                                vec![0.13, 0.13, 0.13, 3.17, 3.17, 3.17]
                            } else {
                                vec![0., 0., 0., 1., 1., 1.]
                            },
                            control_points: vec![
                                edge.control_points.clone(),
                                vec![
                                    vec![0., 0.25, sign],
                                    vec![0.5, 1.5, sign * 0.25],
                                    vec![1., 0.25, sign * 2.],
                                ],
                            ],
                            weights: vec![edge.weights.clone(), vec![1.25, 0.75, 1.]],
                            periodic_u: false,
                            periodic_v: false,
                        };
                        let mut a = make(1.);
                        let mut b = make(-1.);
                        if fold {
                            b.control_points[1][1][2] = 0.25;
                        }
                        let transform = |p: &mut Vec<f64>| {
                            let q = p.clone();
                            for k in 0..3 {
                                p[(k + rotation) % 3] =
                                    [16., -8., 4.][k] + q[k] + shear * q[(k + 1) % 3];
                            }
                        };
                        for p in &mut edge.control_points {
                            transform(p);
                        }
                        for s in [&mut a, &mut b] {
                            for row in &mut s.control_points {
                                for p in row {
                                    transform(p);
                                }
                            }
                        }
                        let pcurve = Curve::from_polyline(vec![
                            vec![a.knots_u[0], a.knots_v[0]],
                            vec![a.knots_u[0], *a.knots_v.last().unwrap()],
                        ])
                        .unwrap();
                        let certified = brep_core::shared_boundary::separates_surfaces(
                            &a, &b, &pcurve, &pcurve, &edge,
                        )
                        .unwrap();
                        cases.push(json!({"a":a,"b":b,"edge":edge,"pcurve":pcurve,"fold":fold,"certified":certified}));
                    }
                }
            }
        }
    }
    println!("{}", json!({"cases":cases}));
}
