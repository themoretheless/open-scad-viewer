//! Source-preserving sufficient monotonicity for polynomial Bezier coordinates.
//! Only exactly representable dyadic subdivisions are admitted. Each operation
//! spends the caller's existing shared cell ledger; no per-axis fresh budget.
use super::*;

fn mean(a: f64, b: f64) -> Option<f64> {
    let x = a * 0.5;
    let y = b * 0.5;
    if x * 2. != a || y * 2. != b {
        return None;
    }
    let s = x + y;
    let virtual_y = s - x;
    let error = (x - (s - virtual_y)) + (y - virtual_y);
    (s.is_finite() && error == 0.).then_some(s)
}
fn charge(cells: &mut usize, max: usize) -> bool {
    if *cells >= max {
        return false;
    }
    *cells += 1;
    true
}
fn ordered(p: &[f64], positive: bool, cells: &mut usize, max: usize) -> Option<bool> {
    let mut ordered = if positive {
        p[0] < p[p.len() - 1]
    } else {
        p[0] > p[p.len() - 1]
    };
    for pair in p.windows(2) {
        if !charge(cells, max) {
            return None;
        }
        ordered &= if positive {
            pair[0] <= pair[1]
        } else {
            pair[0] >= pair[1]
        };
    }
    Some(ordered)
}
pub(super) fn prove(c: &Curve, cells: &mut usize, max: usize) -> Result<bool> {
    c.validate()?;
    if !bezier(c) || c.degree == 0 || c.degree > 32 || c.weights.iter().any(|w| *w != c.weights[0]) {
        return Ok(false);
    }
    for axis in 0..2 {
        let start = c.control_points[0][axis];
        let end = c.control_points.last().unwrap()[axis];
        if start == end {
            continue;
        }
        let positive = start < end;
        let mut pending = vec![(
            c.control_points.iter().map(|p| p[axis]).collect::<Vec<_>>(),
            0,
        )];
        let mut complete = true;
        while let Some((mut p, depth)) = pending.pop() {
            match ordered(&p, positive, cells, max) {
                Some(true) => continue,
                None => return Ok(false),
                Some(false) => {}
            }
            if depth == 16 {
                complete = false;
                break;
            }
            let n = p.len();
            let mut left = vec![p[0]];
            let mut right = vec![p[n - 1]];
            'levels: for level in 1..n {
                for i in 0..n - level {
                    if !charge(cells, max) {
                        return Ok(false);
                    }
                    let Some(mid) = mean(p[i], p[i + 1]) else {
                        complete = false;
                        break 'levels;
                    };
                    p[i] = mid;
                }
                left.push(p[0]);
                right.push(p[n - level - 1]);
            }
            if !complete {
                break;
            }
            right.reverse();
            pending.push((right, depth + 1));
            pending.push((left, depth + 1));
        }
        if complete {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn critical_quintic_coordinate_requires_exact_subdivision_and_shared_work() {
        let c = Curve {
            degree: 5,
            knots: [vec![0.; 6], vec![1.; 6]].concat(),
            control_points: (0..6)
                .map(|i| vec![if i % 2 == 0 { -10. } else { 10. }, 0.])
                .collect(),
            weights: vec![1.; 6],
            periodic: false,
        };
        let before = c.clone();
        let mut cells = 0;
        assert!(prove(&c, &mut cells, 1000).unwrap());
        assert!(cells > 0);
        assert_eq!(c, before);
        let mut short = 0;
        assert!(!prove(&c, &mut short, cells - 1).unwrap());
        assert!(short <= cells - 1);
        let mut folded = c.clone();
        for (p, x) in folded
            .control_points
            .iter_mut()
            .zip([-10., 100., -100., 100., -100., 10.])
        {
            p[0] = x;
        }
        assert!(!prove(&folded, &mut 0, 1000).unwrap());
        assert_eq!(mean(f64::MAX, f64::MAX), Some(f64::MAX));
        assert!(mean(1., 1f64.next_up()).is_none());
        assert!(mean(f64::from_bits(1), 0.).is_none());
    }
}
