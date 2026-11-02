use nurbs_core::{
    curve::Curve,
    curve_partition_agreement::{
        concatenate::{self, Input},
        mapped,
    },
};
fn line(a: [f64; 2], b: [f64; 2]) -> Curve {
    Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap()
}
#[test]
fn mixed_degree_reversed_sources_keep_independent_original_clock_maps() {
    let a = line([0., 0.], [1., 1.]);
    let b = Curve {
        degree: 2,
        knots: vec![-3., -3., -3., 7., 7., 7.],
        control_points: vec![vec![2., 0.], vec![1.5, 0.8], vec![1., 1.]],
        weights: vec![2., 0.75, 3.],
        periodic: false,
    };
    let r = concatenate::concatenate(
        &[
            Input {
                curve: &a,
                reversed: false,
            },
            Input {
                curve: &b,
                reversed: true,
            },
        ],
        false,
        1e-10,
        1000,
    )
    .unwrap();
    assert_eq!(r.curve().degree, 2);
    assert_eq!(r.curve().domain(), [0., 2.]);
    assert_eq!(r.segments()[1].source_parameters, [7., -3.]);
    assert_eq!(r.curve().control_points.first().unwrap(), &vec![0., 0.]);
    assert_eq!(r.curve().control_points.last().unwrap(), &vec![2., 0.]);
    assert!(r.segments().iter().all(|s| s.agreement.accepted));
    assert!(r.visited() <= 1000);
    r.reverify().unwrap();
}
#[test]
fn a_closed_rational_chain_with_weight_gauges_and_distinct_knots_is_admitted() {
    let c = nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 2.).unwrap();
    let d = c.domain();
    let mut parts = Vec::new();
    for i in 0..4 {
        let a = d[0] + (d[1] - d[0]) * i as f64 / 4.;
        let b = d[0] + (d[1] - d[0]) * (i + 1) as f64 / 4.;
        let mut p = c.trim(a, b).unwrap();
        if i == 1 {
            p = p.elevate(3).unwrap();
        }
        let pd = p.domain();
        for k in &mut p.knots {
            *k = 100. * i as f64 + 3. * (*k - pd[0]) / (pd[1] - pd[0]);
        }
        for w in &mut p.weights {
            *w *= 2.25 + i as f64;
        }
        if i == 2 {
            p = p.reverse().unwrap();
        }
        parts.push(p);
    }
    let saved = parts.clone();
    let input = parts
        .iter()
        .enumerate()
        .map(|(i, c)| Input {
            curve: c,
            reversed: i == 2,
        })
        .collect::<Vec<_>>();
    let r = concatenate::concatenate(&input, true, 1e-9, 5000).unwrap();
    assert!(r.closed());
    assert_eq!(
        r.curve().control_points.first(),
        r.curve().control_points.last()
    );
    assert!(nurbs_core::curve_partition_agreement::verify_source_closure(r.curve()).unwrap());
    r.reverify().unwrap();
    assert_eq!(parts, saved);
}
#[test]
fn mapped_subranges_do_not_accept_a_changed_current_curve_or_wrong_clock() {
    let a = Curve {
        degree: 2,
        knots: vec![-3., -3., -3., 7., 7., 7.],
        control_points: vec![vec![0., 0.], vec![1., 2.], vec![2., 0.]],
        weights: vec![1., 0.75, 2.],
        periodic: false,
    };
    let mut b = a.clone();
    for k in &mut b.knots {
        *k = 4. + (*k + 3.) * 0.8;
    }
    assert!(
        mapped::verify(&a, &b, [-3., 7.], [4., 12.], 1e-10, 256)
            .unwrap()
            .accepted
    );
    assert!(
        !mapped::verify(&a, &b, [-3., 7.], [12., 4.], 1e-10, 256)
            .unwrap()
            .accepted
    );
    assert!(
        mapped::verify(&a, &b, [7., -3.], [12., 4.], 1e-10, 256)
            .unwrap()
            .accepted
    );
    b.control_points[1][0] += 0.1;
    assert!(
        !mapped::verify(&a, &b, [-3., 7.], [4., 12.], 1e-10, 256)
            .unwrap()
            .accepted
    );
    assert!(mapped::verify(&a, &b, [-4., 7.], [4., 12.], 1e-10, 256).is_err());
}
#[test]
fn failed_join_closure_and_budget_do_not_publish_or_mutate_sources() {
    let a = line([0., 0.], [1., 0.]);
    let b = line([10., 0.], [11., 0.]);
    let saved = (a.clone(), b.clone());
    assert!(
        concatenate::concatenate(
            &[
                Input {
                    curve: &a,
                    reversed: false
                },
                Input {
                    curve: &b,
                    reversed: false
                }
            ],
            false,
            1e-8,
            256
        )
        .is_err()
    );
    assert!(
        concatenate::concatenate(
            &[Input {
                curve: &a,
                reversed: false
            }],
            true,
            1e-8,
            256
        )
        .is_err()
    );
    let c = line([1., 0.], [2., 0.]);
    assert!(
        concatenate::concatenate(
            &[
                Input {
                    curve: &a,
                    reversed: false
                },
                Input {
                    curve: &c,
                    reversed: false
                }
            ],
            false,
            1e-8,
            1
        )
        .is_err()
    );
    assert_eq!((a, b), saved);
}
