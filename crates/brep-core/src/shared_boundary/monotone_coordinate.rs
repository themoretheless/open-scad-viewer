//! Shared curved-edge separation with one synchronized monotone coordinate.
//! Only comparisons of original finite binary64 controls enter this proof.
use super::*;

pub(super) fn separates(a: &Surface, b: &Surface, pa: &Curve, pb: &Curve, edge: &Curve) -> bool {
    if a.validate().is_err()
        || b.validate().is_err()
        || pa.validate().is_err()
        || pb.validate().is_err()
        || edge.validate().is_err()
    {
        return false;
    }
    for s in [a, b] {
        if s.periodic_u
            || s.periodic_v
            || s.control_points.len() < 2
            || s.control_points[0].len() < 2
            || s.control_points.len() > 33
            || s.control_points[0].len() > 33
            || s.control_points.iter().flatten().any(|p| p.len() != 3)
            || s.control_points.len() != s.degree_u + 1
            || s.control_points[0].len() != s.degree_v + 1
            || !clamped(&s.knots_u, s.degree_u, s.control_points.len())
            || !clamped(&s.knots_v, s.degree_v, s.control_points[0].len())
            || s.weights
                .iter()
                .any(|row| row.iter().any(|w| *w <= 0. || *w != row[0]))
        {
            return false;
        }
    }
    if edge.control_points.iter().any(|p| p.len() != 3)
        || a.knots_v != b.knots_v
        || a.degree_v != b.degree_v
    {
        return false;
    }
    let Some((0, ra)) = boundary(a, pa, edge) else {
        return false;
    };
    let Some((0, rb)) = boundary(b, pb, edge) else {
        return false;
    };
    let n = a.control_points[0].len();
    if (0..n).any(|j| a.control_points[ra][j] != b.control_points[rb][j]) {
        return false;
    }
    for sync in 0..3 {
        let z = |j: usize| a.control_points[ra][j][sync];
        if !((1..n).all(|j| z(j) > z(j - 1)) || (1..n).all(|j| z(j) < z(j - 1))) {
            continue;
        }
        if [a, b].iter().any(|s| {
            s.control_points
                .iter()
                .any(|row| (0..n).any(|j| row[j][sync] != z(j)))
        }) {
            continue;
        }
        // Positive weights constant along V cancel from the sync coordinate.
        // The identical, strictly monotone Bezier coordinate forces equal V
        // at any contact. At that V the signed offsets below are convex sums.
        for side in 0..3 {
            if side == sync {
                continue;
            }
            let e = |j: usize| a.control_points[ra][j][side];
            let strict = |s: &Surface, r: usize, positive: bool| {
                s.control_points
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != r)
                    .all(|(_, row)| {
                        (0..n).all(|j| {
                            if positive {
                                row[j][side] > e(j)
                            } else {
                                row[j][side] < e(j)
                            }
                        })
                    })
            };
            if (strict(a, ra, true) && strict(b, rb, false))
                || (strict(a, ra, false) && strict(b, rb, true))
            {
                return true;
            }
        }
        // A world-axis side test can miss diagonally opposed profile strips.
        // These two fixed integer normals use exact differences of retained
        // controls. No rounded sum or normal becomes a predicate premise.
        if diagonal_opposite(a, b, ra, rb, sync) {
            return true;
        }
    }
    false
}

fn diagonal_opposite(a: &Surface, b: &Surface, ra: usize, rb: usize, sync: usize) -> bool {
    let free = (0..3).filter(|&k| k != sync).collect::<Vec<_>>();
    let na = a.control_points.len() * a.control_points[0].len();
    let n = a.control_points[0].len();
    for direction in [1., -1.] {
        let mut normal = [0.; 3];
        normal[free[0]] = 1.;
        normal[free[1]] = direction;
        let mut values = [a, b]
            .iter()
            .flat_map(|s| s.control_points.iter().flatten())
            .flat_map(|p| p.iter())
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect::<Vec<_>>();
        let origin = values.len() / 3;
        values.extend(
            [0.; 3]
                .into_iter()
                .chain(normal)
                .map(|x: f64| AuthoredScalar::Binary64Bits(x.to_bits())),
        );
        let Ok(source) = SourceArena::authored("synchronized-diagonal-side", 1, values) else {
            return false;
        };
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &source,
            &tolerance,
            Limits {
                max_work: 100000,
                ..Limits::default()
            },
            None,
        );
        let refs = |i: usize| std::array::from_fn(|k| source.leaf(3 * i + k).unwrap());
        let mut signs = [None, None];
        let mut valid = true;
        for (f, (s, boundary)) in [(a, ra), (b, rb)].into_iter().enumerate() {
            for i in 0..s.control_points.len() {
                if i == boundary {
                    continue;
                }
                for j in 0..n {
                    let p = if f == 0 { i * n + j } else { na + i * n + j };
                    let Ok(proof) = cad_predicates::direction_dot3d(
                        &mut ctx,
                        refs(ra * n + j),
                        refs(p),
                        refs(origin),
                        refs(origin + 1),
                    ) else {
                        return false;
                    };
                    let Outcome::Sign(sign) = proof.outcome else {
                        return false;
                    };
                    if sign == Sign::Zero || signs[f].is_some_and(|prior| prior != sign) {
                        valid = false;
                        break;
                    }
                    signs[f] = Some(sign);
                }
                if !valid {
                    break;
                }
            }
            if !valid {
                break;
            }
        }
        if valid
            && matches!(
                signs,
                [Some(Sign::Positive), Some(Sign::Negative)]
                    | [Some(Sign::Negative), Some(Sign::Positive)]
            )
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn curved_shared_edge_requires_sync_and_strict_opposite_controls() {
        let edge = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![1., 0., 1.], vec![0., 0., 2.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let shifted = |delta: f64| {
            edge.control_points
                .iter()
                .map(|p| vec![p[0] + delta, p[1], p[2]])
                .collect()
        };
        let surface = |rows| Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: edge.knots.clone(),
            control_points: rows,
            weights: vec![vec![1.; 3]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let a = surface(vec![shifted(1.), edge.control_points.clone()]);
        let b = surface(vec![edge.control_points.clone(), shifted(-1.)]);
        let pc = |u| Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![u, 0.], vec![u, 1.]],
            weights: vec![1., 1.],
            periodic: false,
        };
        assert!(separates(&a, &b, &pc(1.), &pc(0.), &edge));
        assert!(separates(&b, &a, &pc(0.), &pc(1.), &edge));
        let mut bad = b.clone();
        bad.control_points[1][1][0] = edge.control_points[1][0];
        assert!(!separates(&a, &bad, &pc(1.), &pc(0.), &edge));
        let mut bad = b.clone();
        bad.control_points[1][1][2] += 0.125;
        assert!(!separates(&a, &bad, &pc(1.), &pc(0.), &edge));
        let mut bad = b.clone();
        bad.weights[1][1] = 2.;
        assert!(!separates(&a, &bad, &pc(1.), &pc(0.), &edge));
        let mut bad = b;
        for row in &mut bad.control_points {
            row[1][2] = 3.;
        }
        assert!(!separates(&a, &bad, &pc(1.), &pc(0.), &edge));
    }
    #[test]
    fn diagonal_synchronized_sides_require_exact_strict_offsets() {
        let edge = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![1., 0., 1.], vec![0., 0., 2.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let shifted = |x: f64, y: f64| {
            edge.control_points
                .iter()
                .map(|p| vec![p[0] + x, p[1] + y, p[2]])
                .collect()
        };
        let surface = |rows| Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: edge.knots.clone(),
            control_points: rows,
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let a = surface(vec![shifted(1., 0.), shifted(0.5, 0.5), shifted(0., 0.)]);
        let b = surface(vec![shifted(0., 0.), shifted(-0.5, -0.5), shifted(0., -1.)]);
        let pc = |u| Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![u, 0.], vec![u, 1.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        assert!(separates(&a, &b, &pc(1.), &pc(0.), &edge));
        assert!(separates(&b, &a, &pc(0.), &pc(1.), &edge));
        let mut crossed = b.clone();
        crossed.control_points[1] = shifted(0.5, 0.5);
        assert!(!separates(&a, &crossed, &pc(1.), &pc(0.), &edge));
        let mut desynchronized = b.clone();
        desynchronized.control_points[1][1][2] += 0.125;
        assert!(!separates(&a, &desynchronized, &pc(1.), &pc(0.), &edge));
        let mut moving_weights = b;
        moving_weights.weights[1][1] = 2.;
        assert!(!separates(&a, &moving_weights, &pc(1.), &pc(0.), &edge));
    }
}
