//! Positive tensor-geometric weights prove an EXACT independent Mobius clock
//! in U and V. The polynomial view has the same original Euclidean poles and
//! knots. Only global injectivity transfers from it: its metric/Jacobian bounds
//! and rank/smoothness flags are NOT original-parameter certificates.
use super::*;
pub(super) fn certify(s: &Surface, used: &mut usize, max: usize) -> Result<bool> {
    let sizes = [s.control_points.len(), s.control_points[0].len()];
    if s.periodic_u
        || s.periodic_v
        || sizes != [s.degree_u + 1, s.degree_v + 1]
        || !(1..=32).contains(&s.degree_u)
        || !(1..=32).contains(&s.degree_v)
        || s.weights.iter().flatten().all(|w| *w == s.weights[0][0])
    {
        return Ok(false);
    }
    for (knots, n) in [(&s.knots_u, sizes[0]), (&s.knots_v, sizes[1])] {
        for (i, &k) in knots.iter().enumerate() {
            if *used == max {
                return Ok(false);
            }
            *used += 1;
            if k != knots[if i < n { n - 1 } else { n }] {
                return Ok(false);
            }
        }
    }
    if !crate::numerics::geometric_weights::tensor(&s.weights, used, max) || *used == max {
        return Ok(false);
    }
    let mut view = s.clone();
    for row in &mut view.weights {
        row.fill(1.);
    }
    // Constant weights make the recursive clock factory inapplicable. Its
    // successful proof cannot be supplied by a caller or a prior report.
    let inner = match super::certify_chart(&view, max - *used) {
        Ok(proof) => proof,
        Err(_) => {
            *used = max;
            return Ok(false);
        }
    };
    *used += inner.spans;
    Ok(inner.proven)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rational_mixed_product_chart_owns_both_clocks_without_rank_credit() {
        let m = [[1., 1., 1.], [1., 2., 1.], [1., 1., 2.]];
        let s = Surface {
            degree_u: 5,
            degree_v: 1,
            knots_u: [vec![0.; 6], vec![1.; 6]].concat(),
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..6)
                .map(|i| {
                    (0..2)
                        .map(|j| {
                            let p = [
                                if i % 2 == 0 { -10. } else { 10. },
                                2. * (i * i) as f64 - 9. * i as f64 + 7.5,
                                2. * j as f64,
                            ];
                            (0..3)
                                .map(|k| (0..3).map(|a| m[k][a] * p[a]).sum())
                                .collect()
                        })
                        .collect()
                })
                .collect(),
            weights: (0..6)
                .map(|i| (0..2).map(|j| 2f64.powi(5 - i) * 3f64.powi(j)).collect())
                .collect(),
            periodic_u: false,
            periodic_v: false,
        };
        let before = s.clone();
        let mut used = 0;
        assert!(certify(&s, &mut used, 1000).unwrap(), "used={used}");
        let mut replay = 0;
        assert!(certify(&s, &mut replay, used).unwrap());
        assert_eq!(replay, used);
        let mut short = 0;
        assert!(!certify(&s, &mut short, used - 1).unwrap());
        assert!(short <= used - 1);
        let result = certify_chart(&s, 1000).unwrap();
        assert!(result.proven, "{result:?}");
        assert_eq!(result.reason, "global-positive-rational-product-clock");
        assert!(result.contraction_upper.is_none() && result.polar_p_matrix.is_none());
        let mut drift = s.clone();
        drift.weights[2][1] = drift.weights[2][1].next_up();
        assert!(!certify(&drift, &mut 0, 1000).unwrap());
        let mut folded = s.clone();
        folded.control_points[5] = folded.control_points[0].clone();
        assert!(!certify(&folded, &mut 0, 1000).unwrap());
        assert_eq!(s, before);
    }
}
