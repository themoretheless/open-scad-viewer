//! Strict diagonal projected Jacobian of an ORIGINAL Bezier x affine product.
use super::*;
pub(super) fn inspect(s: &Surface) -> Option<Report> {
    let n = s.control_points.len();
    if s.periodic_u
        || s.periodic_v
        || s.degree_v != 1
        || s.control_points[0].len() != 2
        || n != s.degree_u + 1
        || s.control_points[0][0].len() != 3
    {
        return None;
    }
    let u = [s.knots_u[s.degree_u], s.knots_u[n]];
    let v = [s.knots_v[1], s.knots_v[2]];
    if !s.knots_u[..=s.degree_u].iter().all(|&t| t == u[0])
        || !s.knots_u[n..].iter().all(|&t| t == u[1])
        || s.knots_v != [v[0], v[0], v[1], v[1]]
    {
        return None;
    }
    for free in 0..3 {
        let a = s.control_points[0][0][free];
        let b = s.control_points[0][1][free];
        if a == b
            || !s.control_points.iter().zip(&s.weights).all(|(p, w)| {
                w[0] == w[1]
                    && (0..3).all(|k| {
                        if k == free {
                            p[0][k] == a && p[1][k] == b
                        } else {
                            p[0][k] == p[1][k]
                        }
                    })
            })
        {
            continue;
        }
        let mut projection = [[0i8; 3]; 2];
        let mut monotone = true;
        for k in 0..3 {
            if k == free {
                continue;
            }
            let increasing = s
                .control_points
                .windows(2)
                .all(|p| p[0][0][k] <= p[1][0][k]);
            let decreasing = s
                .control_points
                .windows(2)
                .all(|p| p[0][0][k] >= p[1][0][k]);
            if !increasing && !decreasing {
                monotone = false;
                break;
            }
            if s.control_points[0][0][k] != s.control_points[n - 1][0][k] {
                projection[0][k] = if increasing { 1 } else { -1 };
            }
        }
        if !monotone
            || projection[0] == [0; 3]
            || s.control_points[0][0] == s.control_points[1][0]
            || s.control_points[n - 1][0] == s.control_points[n - 2][0]
        {
            continue;
        }
        projection[1][free] = if b > a { 1 } else { -1 };
        // Every signed profile-coordinate pole increment is nonnegative.
        // Thus the EXACT integer sum has strictly ordered endpoint pairs and
        // is nonconstant. Positive rational Bernstein weights give derivative
        // Cov(projected pole,index)/(t*(1-t)) > 0 in the open interval; the
        // original endpoint pairs give strictly positive closed-end jets.
        // Both cross derivatives vanish by the literal product structure.
        // The SAME fixed projection therefore has a strictly positive
        // diagonal symmetric Jacobian everywhere on the closed rectangle.
        // No rounded projected Cartesian poles or sampled metric are used.
        return Some(Report {
            certified: true,
            cells: 1,
            projection,
            row_weights: [1.; 2],
            profile_parameterization: None,
            reason: None,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Surface {
        let c = crate::construct::curves::bounded_corner::propose(0.1, 10., 2.5e-8)
            .unwrap()
            .quartic_g2()
            .unwrap();
        Surface {
            degree_u: 4,
            degree_v: 1,
            knots_u: c.knots,
            knots_v: vec![0., 0., 1., 1.],
            control_points: c
                .control_points
                .iter()
                .map(|p| vec![p.clone(), vec![p[0], p[1], 2.]])
                .collect(),
            weights: c.weights.iter().map(|w| vec![*w; 2]).collect(),
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn original_quartic_product_has_one_fixed_strict_common_metric() {
        let s = fixture();
        let before = s.clone();
        let report = inspect_candidate(&s, 1).unwrap();
        assert!(report.certified);
        assert_eq!(report.cells, 1);
        assert_eq!(report.projection, [[1, -1, 0], [0, 0, 1]]);
        assert_eq!(report.row_weights, [1.; 2]);
        assert!(report.profile_parameterization.is_none());
        assert!(!inspect_candidate(&s, 0).unwrap().certified);
        assert_eq!(s, before);
    }
    #[test]
    fn exact_coordinate_orders_do_not_need_representable_projected_poles() {
        let s = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: [[0.1, 0.7], [0.3, 0.3], [0.9, 0.2]]
                .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 1.]])
                .to_vec(),
            weights: vec![vec![1.; 2], vec![2.; 2], vec![3.; 2]],
            periodic_u: false,
            periodic_v: false,
        };
        assert!(exact_projected_surface(&s, [[1, -1, 0], [0, 0, 1]]).is_none());
        assert!(inspect_candidate(&s, 1).unwrap().certified);
    }
    #[test]
    fn stationary_endpoints_reversal_and_station_weight_drift_disable_the_product_proof() {
        let mut s = fixture();
        s.weights[1][1] = s.weights[1][1].next_up();
        assert!(inspect(&s).is_none());
        s.weights[1][1] = s.weights[1][0];
        s.control_points[1] = s.control_points[0].clone();
        assert!(inspect(&s).is_none());
        s.control_points[1][0][0] = 1.;
        s.control_points[1][1][0] = 1.;
        assert!(inspect(&s).is_none());
    }
}
