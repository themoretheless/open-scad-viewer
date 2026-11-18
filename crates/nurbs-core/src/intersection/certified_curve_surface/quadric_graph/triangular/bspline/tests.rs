use super::*;
fn basis(i: usize, p: usize, k: &[Q], t: &Q) -> Q {
    if p == 0 {
        return if k[i] <= *t && *t < k[i + 1] {
            q(1.)
        } else {
            q(0.)
        };
    }
    let mut a = q(0.);
    if k[i + p] > k[i] {
        a += (t - &k[i]) / (&k[i + p] - &k[i]) * basis(i, p - 1, k, t);
    }
    if k[i + p + 1] > k[i + 1] {
        a += (&k[i + p + 1] - t) / (&k[i + p + 1] - &k[i + 1]) * basis(i + 1, p - 1, k, t);
    }
    a
}
fn knots(p: usize) -> Vec<f64> {
    [vec![0.; p + 1], vec![1., 3.], vec![4.; p + 1]].concat()
}
fn ordinates(p: usize, k: &[f64]) -> Vec<Q> {
    (0..p + 3)
        .map(|i| q(k[i + 1..=i + p].iter().sum::<f64>().powi(2)) / q((16 * p * p) as f64))
        .collect()
}
fn forward(s: &Spline, u: &Q, v: &Q) -> Q {
    let (p, r) = (s.degrees[0], s.degrees[1]);
    let mut n = q(0.);
    let mut d = q(0.);
    for i in 0..s.weights.len() {
        for j in 0..s.ordinates.len() {
            let h = &s.weights[i][j]
                * basis(i, p, &s.knots[0], &(u * q(4.)))
                * basis(j, r, &s.knots[1], &(v * q(4.)));
            n += &h * &s.ordinates[j];
            d += h;
        }
    }
    n / d
}
#[test]
fn all_degree_pairs_keep_original_cox_preimages_under_joint_positive_weights() {
    for p in 1..=8 {
        for r in 1..=8 {
            let (ku, kv) = (knots(p), knots(r));
            let c = ordinates(r, &kv);
            for axis in 0..2 {
                let s = Spline {
                    axis,
                    clock: super::super::super::clock::Clock::original(vec![q(1.), q(1.)]),
                    degrees: [p, r],
                    knots: [
                        ku.iter().map(|x| q(*x)).collect(),
                        kv.iter().map(|x| q(*x)).collect(),
                    ],
                    conditional_knots: kv.clone(),
                    ordinates: c.clone(),
                    weights: (0..p + 3)
                        .map(|i| {
                            (0..r + 3)
                                .map(|j| q((1 + (i + 2 * j) * (i + j + 3) % 7) as f64))
                                .collect()
                        })
                        .collect(),
                };
                let b = forward(&s, &q(0.5), &q(0.5));
                let mut input = [[q(0.5), q(0.5)], [b.clone(), b]];
                if axis == 1 {
                    input.swap(0, 1);
                }
                let uv = s.enclosure(&input).unwrap();
                for b in uv {
                    assert!(b[0] <= q(0.5) && q(0.5) <= b[1], "degree {p},{r}: {b:?}");
                }
            }
        }
    }
}
#[test]
fn multispan_cross_identity_census_and_bounded_refusal_cover_all_degrees() {
    for p in 1..=8 {
        for r in 1..=8 {
            let (ku, kv) = (knots(p), knots(r));
            let x: Vec<f64> = (0..p + 3)
                .map(|i| ku[i + 1..=i + p].iter().sum::<f64>().powi(2))
                .collect();
            let y: Vec<f64> = (0..r + 3)
                .map(|j| kv[j + 1..=j + r].iter().sum::<f64>().powi(2))
                .collect();
            let mut s = Surface {
                degree_u: p,
                degree_v: r,
                knots_u: ku,
                knots_v: kv,
                control_points: x
                    .iter()
                    .map(|x| y.iter().map(|y| vec![*x, *y, 0.]).collect())
                    .collect(),
                weights: vec![vec![1.; r + 3]; p + 3],
                periodic_u: false,
                periodic_v: false,
            };
            let (o, u, v) = (
                [q(0.), q(0.)],
                [q((16 * p * p) as f64), q(0.)],
                [q(0.), q((16 * r * r) as f64)],
            );
            let det = &u[0] * &v[1];
            let frame = AffineFrame { origin: o, u, v, det };
            assert!(
                prove(&s, [0, 1], &frame).is_some(),
                "degree {p},{r}"
            );
            assert!(prove_with_limit(&s, [0, 1], &frame, 0).is_none());
            s.control_points[1][1][0] = f64::from_bits(s.control_points[1][1][0].to_bits() + 1);
            assert!(prove(&s, [0, 1], &frame).is_none());
        }
    }
}
#[test]
fn flat_conditional_spans_and_discontinuous_basis_never_certify() {
    let mut s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 0.5, 1., 1.],
        knots_v: vec![0., 0., 0.5, 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![0.5, 0., 0.], vec![0.5, 0., 0.], vec![0.5, 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let (o, u, v) = ([q(0.), q(0.)], [q(1.), q(0.)], [q(0.), q(1.)]);
    assert!(prove(
        &s,
        [0, 1],
        &AffineFrame {
            origin: o.clone(),
            u: u.clone(),
            v: v.clone(),
            det: q(1.),
        },
    ).is_none());
    s.degree_u = 1;
    s.knots_u = vec![0., 0., 0.5, 0.5, 1., 1.];
    s.control_points.insert(2, s.control_points[1].clone());
    s.weights.insert(2, vec![1.; 3]);
    assert!(prove(
        &s,
        [0, 1],
        &AffineFrame {
            origin: o.clone(),
            u: u.clone(),
            v: v.clone(),
            det: q(1.),
        },
    ).is_none());
}

#[test]
fn positive_row_mixture_hulls_cover_rectangles_across_all_original_spans() {
    let (p, r) = (3, 3);
    let ku = knots(p);
    let kv = knots(r);
    for axis in 0..2 {
        let mut s = Spline {
            axis,
            clock: super::super::super::clock::Clock::original(vec![q(1.), q(1.)]),
            degrees: [p, r],
            knots: [
                ku.iter().map(|x| q(*x)).collect(),
                kv.iter().map(|x| q(*x)).collect(),
            ],
            conditional_knots: kv.clone(),
            ordinates: ordinates(r, &kv),
            weights: (0..p + 3)
                .map(|i| {
                    (0..r + 3)
                        .map(|j| q((1 + (i + 2 * j) * (i + j + 3) % 7) as f64))
                        .collect()
                })
                .collect(),
        };
        let mut input = [[q(0.125), q(0.875)], [q(0.25), q(0.75)]];
        if axis == 1 {
            input.swap(0, 1);
        }
        assert!(s.enclose_with_limit(&input, 0).is_none());
        let uv = s.enclosure(&input).unwrap();
        for i in 1..8 {
            for j in 1..8 {
                let (u, v) = (q(i as f64) / q(8.), q(j as f64) / q(8.));
                let b = forward(&s, &u, &v);
                if input[1 - axis][0] <= b && b <= input[1 - axis][1] {
                    assert!(uv[axis][0] <= u && u <= uv[axis][1]);
                    assert!(uv[1 - axis][0] <= v && v <= uv[1 - axis][1]);
                }
            }
        }
        for w in s.weights.iter_mut().flatten() {
            *w *= q(7.);
        }
        assert_eq!(uv, s.enclosure(&input).unwrap());
    }
}
