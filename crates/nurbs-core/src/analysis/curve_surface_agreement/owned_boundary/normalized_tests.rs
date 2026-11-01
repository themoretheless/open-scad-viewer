use super::*;
use cad_predicates::{BezierIdentity as B, Reason};
fn source() -> Surface {
    Surface {
        degree_u: 3,
        degree_v: 2,
        knots_u: vec![-3., -3., -3., -3., 1., 1., 1., 5., 5., 5., 5.],
        knots_v: vec![7., 7., 7., 11., 11., 11.],
        control_points: (0..7)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        vec![
                            i as f64 - 3.,
                            j as f64 - 1.,
                            if i == 5 && j == 1 { 1. / 128. } else { 0. },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![2.; 3]; 7],
        periodic_u: false,
        periodic_v: false,
    }
}
fn line(a: Vec<f64>, b: Vec<f64>) -> Curve {
    Curve {
        degree: 1,
        knots: vec![17., 17., 29., 29.],
        control_points: vec![a, b],
        weights: vec![3.; 2],
        periodic: false,
    }
}
#[test]
fn different_degree_multispan_borders_keep_normalized_domains_reversal_and_exact_budgets() {
    let s = source();
    let xyz = [[-3., -1., 0.], [3., -1., 0.], [3., 1., 0.], [-3., 1., 0.]];
    let uv = [[-3., 7.], [5., 7.], [5., 11.], [-3., 11.]];
    for i in 0..4 {
        let p = line(uv[i].to_vec(), uv[(i + 1) % 4].to_vec());
        for reversed in [false, true] {
            let mut c = line(xyz[i].to_vec(), xyz[(i + 1) % 4].to_vec());
            if reversed {
                c.control_points.reverse();
            }
            let before = (c.clone(), p.clone(), s.clone());
            let run = |budget| {
                super::super::verify_exact(&c, &p, &s, reversed, budget)
                    .unwrap()
                    .unwrap()
            };
            let proof = run(1000000);
            assert_eq!(proof.outcome, B::Equal, "edge={i} reversed={reversed}");
            assert!(proof.work_used > 0);
            assert_eq!(run(proof.work_used).outcome, B::Equal);
            for budget in [0, proof.work_used - 1] {
                let refusal = run(budget);
                assert_eq!(refusal.outcome, B::Indeterminate(Reason::ResourceLimit));
                assert!(refusal.work_used <= budget);
            }
            assert_eq!((c, p.clone(), s.clone()), before);
        }
    }
}
#[test]
fn whole_multispan_border_rejects_hidden_control_damage_and_partial_uv_coverage() {
    let mut s = source();
    let c = line(vec![-3., -1., 0.], vec![3., -1., 0.]);
    let p = line(vec![-3., 7.], vec![5., 7.]);
    s.control_points[2][0][2] = 1. / 1024.;
    let r = super::super::verify_exact(&c, &p, &s, false, 1000000)
        .unwrap()
        .unwrap();
    assert_ne!(r.outcome, B::Equal);
    let s = source();
    let p = line(vec![-3., 7.], vec![1., 7.]);
    let r = super::super::verify_exact(&c, &p, &s, false, 1000000).unwrap();
    assert!(r.is_none_or(|r| r.outcome != B::Equal));
}

#[test]
fn different_degrees_keep_nonpolynomial_rational_speed_instead_of_only_the_same_image() {
    // Original rational line has x(t)=15t/(1+2t). The independently authored
    // quadratic two-span border has that SAME clock, not merely the line image.
    // All controls and weights below are exact dyadic source values.
    let s = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [0., 2.5, 3.75, 4.5, 5.]
            .iter()
            .map(|&x| vec![vec![x, 0., 0.], vec![x, 1., 0.]])
            .collect(),
        weights: [1., 1.5, 2., 2.5, 3.].iter().map(|&w| vec![w, w]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let p = line(vec![0., 0.], vec![1., 0.]);
    let mut c = line(vec![0., 0., 0.], vec![5., 0., 0.]);
    c.weights = vec![1., 3.];
    for reversed in [false, true] {
        let mut r = c.clone();
        if reversed {
            r.control_points.reverse();
            r.weights.reverse();
        }
        assert_eq!(
            super::super::verify_exact(&r, &p, &s, reversed, 1000000)
                .unwrap()
                .unwrap()
                .outcome,
            B::Equal
        );
    }
    // Changing only speed preserves the entire geometric image and endpoints,
    // but must fail exact source correspondence on the complete parameter map.
    c.weights = vec![1., 1.];
    assert_ne!(
        super::super::verify_exact(&c, &p, &s, false, 1000000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Equal
    );
}
