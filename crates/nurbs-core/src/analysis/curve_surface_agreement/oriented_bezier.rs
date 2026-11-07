//! Exact orientation transport for FULL clamped Bernstein traversals.
//! Clone/reverse poles and weights, RETAIN original endpoint knots. No rounded
//! extracted or projected source replaces the original definitions.
use super::*;
fn mirrored(c: &Curve) -> Option<Curve> {
    let p = c.degree;
    let n = c.control_points.len();
    let d = c.domain();
    if c.periodic
        || n != p + 1
        || !c.knots[..=p].iter().all(|&t| t == d[0])
        || !c.knots[n..].iter().all(|&t| t == d[1])
    {
        return None;
    }
    let mut reversed = c.clone();
    reversed.control_points.reverse();
    reversed.weights.reverse();
    Some(reversed)
}
pub(super) fn retry(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    reversed: bool,
    interval: [f64; 2],
    tolerance: f64,
    max_cells: usize,
    mut report: Report,
) -> Result<Report> {
    if report.status != Status::Unresolved || report.cells >= max_cells || interval != p.domain() {
        return Ok(report);
    }
    let (Some(c), Some(p)) = (mirrored(c), mirrored(p)) else {
        return Ok(report);
    };
    // Bernstein reversal gives C_mirror(tau)=C(1-tau) on NORMALIZED natural
    // clocks, exactly, with arbitrary different original natural domains.
    // Mirroring BOTH curves preserves the supremum correspondence distance,
    // including when the coedge flag already reverses the world curve.
    match verify_on_direct(
        &c,
        &p,
        s,
        reversed,
        interval,
        tolerance,
        max_cells - report.cells,
    ) {
        Ok(r) => {
            report.cells += r.cells;
            if r.status == Status::WithinTolerance {
                report.status = Status::WithinTolerance;
            }
            // A negative witness at tau cannot be blindly rounded to 1-tau.
            // Keep the original unresolved result unless a full cover succeeds.
        }
        Err(_) => {
            report.cells = max_cells;
        }
    }
    Ok(report)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Curve, Curve, Surface) {
        let c = Curve {
            degree: 3,
            knots: [vec![2.; 4], vec![5.; 4]].concat(),
            control_points: vec![
                vec![0.1, 0.2, 0.],
                vec![0.2, 0.7, 0.],
                vec![0.5, 0.4, 0.],
                vec![0.9, 0.8, 0.],
            ],
            weights: vec![1., 2., 3., 1.],
            periodic: false,
        };
        let mut p = c.clone();
        for v in &mut p.control_points {
            v.pop();
        }
        p.knots = [vec![-2.; 4], vec![3.; 4]].concat();
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        (c, p, s)
    }
    fn unresolved(cells: usize) -> Report {
        Report {
            status: Status::Unresolved,
            cells,
            witness: None,
            witness_distance: None,
        }
    }
    #[test]
    fn original_domains_and_coedge_orientation_are_preserved_by_full_cover_transport() {
        let (c, p, s) = fixture();
        for reversed in [false, true] {
            let world = if reversed {
                mirrored(&c).unwrap()
            } else {
                c.clone()
            };
            let r = retry(
                &world,
                &p,
                &s,
                reversed,
                p.domain(),
                1e-7,
                1000,
                unresolved(7),
            )
            .unwrap();
            assert_eq!(r.status, Status::WithinTolerance);
            assert!(r.cells > 7 && r.cells <= 1000);
            let m = mirrored(&world).unwrap();
            assert_eq!(m.knots, world.knots);
            assert_eq!(mirrored(&m).unwrap(), world);
        }
    }
    #[test]
    fn missing_budget_partial_source_and_changed_geometry_cannot_inherit_acceptance() {
        let (mut c, p, s) = fixture();
        assert_eq!(
            retry(&c, &p, &s, false, p.domain(), 1e-7, 10, unresolved(10))
                .unwrap()
                .status,
            Status::Unresolved
        );
        assert_eq!(
            retry(&c, &p, &s, false, [-1., 2.], 1e-7, 1000, unresolved(0))
                .unwrap()
                .cells,
            0
        );
        c.control_points[1][2] = 0.01;
        assert_ne!(
            retry(&c, &p, &s, false, p.domain(), 1e-7, 1000, unresolved(0))
                .unwrap()
                .status,
            Status::WithinTolerance
        );
        assert!(mirrored(&c.insert(3., 1).unwrap()).is_none());
    }
}
