//! Exact structural full-rectangle coverage after fresh original trim audit.
//! Arbitrary segmented sides and either winding are accepted; all joins,
//! containment, monotone edge traversal and exactly four corners are checked.
use super::Boundary;
pub(super) fn covers(wires: &[Vec<Boundary>], corners: [[f64; 2]; 4]) -> bool {
    if wires.len() != 1 || wires[0].len() < 4 {
        return false;
    }
    let u = [corners[0][0], corners[1][0]];
    let v = [corners[0][1], corners[2][1]];
    if !(u[0] < u[1] && v[0] < v[1]) {
        return false;
    }
    let mut entries = Vec::new();
    for b in &wires[0] {
        let c = &b.pcurve;
        if c.degree != 1
            || c.periodic
            || c.control_points.len() != 2
            || c.knots.len() != 4
            || c.weights.len() != 2
            || c.weights.iter().any(|w| !w.is_finite() || *w <= 0.)
            || c.control_points.iter().any(|p| p.len() != 2)
        {
            return false;
        }
        let d = c.domain();
        if d[0] >= d[1]
            || c.knots[..2].iter().any(|t| *t != d[0])
            || c.knots[2..].iter().any(|t| *t != d[1])
        {
            return false;
        }
        let (a, b) = (&c.control_points[0], &c.control_points[1]);
        if a == b
            || [a, b]
                .iter()
                .any(|p| !(u[0] <= p[0] && p[0] <= u[1] && v[0] <= p[1] && p[1] <= v[1]))
        {
            return false;
        }
        let side = if a[1] == v[0] && b[1] == v[0] {
            Some((0, a[0] < b[0]))
        } else if a[0] == u[1] && b[0] == u[1] {
            Some((1, a[1] < b[1]))
        } else if a[1] == v[1] && b[1] == v[1] {
            Some((2, a[0] > b[0]))
        } else if a[0] == u[0] && b[0] == u[0] {
            Some((3, a[1] > b[1]))
        } else {
            None
        };
        let Some((edge, forward)) = side else {
            return false;
        };
        entries.push((edge, forward, a, b));
    }
    let orientation = entries[0].1;
    let mut turns = 0;
    for i in 0..entries.len() {
        let a = entries[i];
        let b = entries[(i + 1) % entries.len()];
        if a.1 != orientation || a.3 != b.2 {
            return false;
        }
        if a.0 != b.0 {
            if b.0 != (a.0 + if orientation { 1 } else { 3 }) % 4 {
                return false;
            }
            turns += 1;
        }
    }
    turns == 4
}
#[cfg(test)]
mod tests {
    use super::*;
    use nurbs_core::curve::Curve;
    const C: [[f64; 2]; 4] = [[-3., 7.], [5., 7.], [5., 11.], [-3., 11.]];
    fn wire(points: &[[f64; 2]]) -> Vec<Boundary> {
        points
            .windows(2)
            .map(|p| {
                let c = Curve {
                    degree: 1,
                    knots: vec![13., 13., 19., 19.],
                    control_points: p.iter().map(|p| p.to_vec()).collect(),
                    weights: vec![1., 2.],
                    periodic: false,
                };
                Boundary {
                    curve: c.clone(),
                    pcurve: c,
                    reversed: false,
                }
            })
            .collect()
    }
    #[test]
    fn segmented_whole_rectangle_accepts_both_windings_and_interior_start() {
        let p = [
            [0., 7.],
            [5., 7.],
            [5., 9.],
            [5., 11.],
            [-3., 11.],
            [-3., 7.],
            [0., 7.],
        ];
        assert!(covers(&[wire(&p)], C));
        let reverse = p.into_iter().rev().collect::<Vec<_>>();
        assert!(covers(&[wire(&reverse)], C));
    }
    #[test]
    fn gaps_diagonals_interior_loops_retracing_and_holes_do_not_claim_the_full_chart() {
        let p = [[-3., 7.], [5., 7.], [5., 11.], [-3., 11.], [-3., 7.]];
        let mut gap = wire(&p);
        gap[1].pcurve.control_points[0][0] -= 1e-12;
        assert!(!covers(&[gap], C));
        assert!(!covers(
            &[wire(&[[-3., 7.], [5., 11.], [-3., 11.], [-3., 7.]])],
            C
        ));
        assert!(!covers(
            &[wire(&[[0., 8.], [1., 8.], [1., 9.], [0., 9.], [0., 8.]])],
            C
        ));
        assert!(!covers(&[wire(&p), wire(&p)], C));
        let mut repeated = wire(&p);
        repeated.extend(wire(&p));
        assert!(!covers(&[repeated], C));
    }
}
