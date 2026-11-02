//! Bounded rounded strict-separator proposal. No geometric authority: the
//! caller freshly checks every original hull pole using exact orient3d.
fn unit(v: [f64; 3]) -> Option<[f64; 3]> {
    let norm = v[0].hypot(v[1]).hypot(v[2]);
    (norm.is_finite() && norm > 0.).then(|| v.map(|x| x / norm))
}
fn charge(spent: &mut u64, max: u64) -> Option<()> {
    if *spent >= max {
        return None;
    }
    *spent += 1;
    Some(())
}
pub(super) fn improve(
    points: &[Vec<[f64; 3]>; 2],
    point: [f64; 3],
    spent: &mut u64,
    max: u64,
) -> Option<[f64; 3]> {
    let mut constraints = Vec::new();
    for (slot, points) in points.iter().enumerate() {
        for p in points {
            charge(spent, max)?;
            if *p == point {
                continue;
            }
            let sign = if slot == 0 { 1. } else { -1. };
            constraints.push(unit(std::array::from_fn(|k| sign * (p[k] - point[k])))?);
        }
    }
    let first = *constraints.first()?;
    let sum = constraints
        .iter()
        .fold([0.; 3], |a, q| std::array::from_fn(|k| a[k] + q[k]));
    let mut normal = sum.map(|x| x / constraints.len() as f64);
    // Iterative projections only seek a useful proposal. Neither convergence
    // nor a rounded positive score is accepted as a separation certificate.
    for _ in 0..128 {
        let mut worst = (f64::INFINITY, first);
        for q in &constraints {
            charge(spent, max)?;
            let dot = (0..3).map(|k| normal[k] * q[k]).sum::<f64>();
            if !dot.is_finite() {
                return None;
            }
            if dot < worst.0 {
                worst = (dot, *q);
            }
        }
        let norm = normal[0].hypot(normal[1]).hypot(normal[2]);
        if !norm.is_finite() || norm == 0. {
            return None;
        }
        if worst.0 > 1e-6 * norm {
            return unit(normal);
        }
        // Minimum-norm convex-hull proposal (bounded Frank-Wolfe step).
        // A normalized unit-margin projection can cycle between nearly
        // opposite constraints even when a strict separator exists.
        let d = std::array::from_fn::<_, 3, _>(|k| worst.1[k] - normal[k]);
        let denominator = d.iter().map(|x| x * x).sum::<f64>();
        if !denominator.is_finite() || denominator <= 0. {
            return None;
        }
        let alpha = ((norm * norm - worst.0) / denominator).clamp(0., 1.);
        normal = std::array::from_fn(|k| normal[k] + alpha * d[k]);
    }
    None
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paid_separator_proposal_replays_exact_budget_and_cannot_resolve_opposite_constraints() {
        let points = [
            vec![[0., 3., 6.], [3., 3., 7.125], [0., 6., 6.], [3., 6., 8.25]],
            vec![
                [3., 6., 5.25],
                [6., 6., 7.5],
                [3., 6., 8.25],
                [6., 6., 10.5],
            ],
        ];
        let mut work = 0;
        let n = improve(&points, [3., 6., 8.25], &mut work, 10000).unwrap();
        let mut replay = 0;
        assert_eq!(improve(&points, [3., 6., 8.25], &mut replay, work), Some(n));
        assert_eq!(replay, work);
        let mut short = 0;
        assert!(improve(&points, [3., 6., 8.25], &mut short, work - 1).is_none());
        assert!(short <= work - 1);
        let contradictory = [vec![[1., 0., 0.]], vec![[1., 0., 0.]]];
        let mut spent = 0;
        assert!(improve(&contradictory, [0.; 3], &mut spent, 10000).is_none());
        assert!(spent <= 10000);
    }
}
