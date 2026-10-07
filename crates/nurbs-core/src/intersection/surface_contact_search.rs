//! Conservative search over the full product of two natural surface domains.
//! Spatial separation or a complete outside-trim classification removes a cell. A section
//! witness proves existence, never coverage of its surrounding intersection arc.
use crate::{
    Result, check,
    foundation::guards::Budget,
    surface::Surface,
    surface_contact::{self, Witness},
    surface_distance::rectangle_bounds,
};
use std::collections::VecDeque;

pub type Cell = [[[f64; 2]; 2]; 2];
#[derive(Clone, Debug)]
pub struct Report {
    pub contact: Option<Witness>,
    pub cells: usize,
    pub domain_cells: usize,
    /// Empty only after all parameter pairs have been excluded.
    pub unresolved: Vec<Cell>,
    pub absence_proven: bool,
}
fn natural(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
/// Stops at the first contact witness or the work limit. The unprocessed cells
/// remain explicit, including the current search cell. No mesh or seed convergence is
/// used as a proof of absence. This searches surface charts, not trimmed faces.
pub fn search(a: &Surface, b: &Surface, max_cells: usize) -> Result<Report> {
    search_impl(a, b, max_cells, None, 0, false)
}
/// Search authored nonperiodic trimmed charts. Only complete outside
/// classifications prune cells; only two inside classifications accept roots.
pub fn search_trimmed(
    a: &Surface,
    b: &Surface,
    domains: [&crate::trim_domain::TrimDomain; 2],
    max_cells: usize,
    max_domain_cells: usize,
) -> Result<Report> {
    check(
        !(a.periodic_u || a.periodic_v || b.periodic_u || b.periodic_v),
        "Periodic trim charts require branch-aware contact classification",
    )?;
    check(
        (1..=1_000_000).contains(&max_domain_cells),
        "Trim contact work budget must be in 1..1000000",
    )?;
    search_impl(a, b, max_cells, Some(domains), max_domain_cells, false)
}
/// Sweep admission additionally prunes whole cells using outward oblique
/// rational control hulls. Existing plain contact accounting stays unchanged.
pub fn search_trimmed_with_control_hulls(
    a: &Surface,
    b: &Surface,
    domains: [&crate::trim_domain::TrimDomain; 2],
    max_cells: usize,
    max_domain_cells: usize,
) -> Result<Report> {
    check(
        !(a.periodic_u || a.periodic_v || b.periodic_u || b.periodic_v),
        "Periodic trim charts require branch-aware contact classification",
    )?;
    check(
        (1..=1_000_000).contains(&max_domain_cells),
        "Trim contact work budget must be in 1..1000000",
    )?;
    search_impl(a, b, max_cells, Some(domains), max_domain_cells, true)
}
fn search_impl(
    a: &Surface,
    b: &Surface,
    max_cells: usize,
    domains: Option<[&crate::trim_domain::TrimDomain; 2]>,
    max_domain_cells: usize,
    control_hulls: bool,
) -> Result<Report> {
    a.validate()?;
    b.validate()?;
    check(
        (1..=100_000).contains(&max_cells),
        "Surface contact work budget must be in 1..100000",
    )?;
    let initial = [natural(a), natural(b)];
    let mut queue = VecDeque::from([initial]);
    let mut stalled = Vec::new();
    let mut cells = 0;
    let mut contact = None;
    let mut domain_cells = 0;
    let classify = |boxes: Cell, used: &mut usize| -> Result<crate::trim_domain::Location> {
        use crate::trim_domain::Location;
        let Some(domains) = domains else {
            return Ok(Location::Inside);
        };
        let mut inside = true;
        for i in 0..2 {
            if *used == max_domain_cells {
                return Ok(Location::Unresolved);
            }
            let r = domains[i].classify(boxes[i], (max_domain_cells - *used).min(100_000))?;
            *used += r.cells;
            if r.location == Location::Outside {
                return Ok(Location::Outside);
            }
            inside &= r.location == Location::Inside;
        }
        Ok(if inside {
            Location::Inside
        } else {
            Location::Unresolved
        })
    };
    // Unified guard backing the max_cells work limit (item 1065); it fires
    // only if the cell bookkeeping above is ever broken.
    let mut guard = Budget::with_iterations(max_cells)?.guard("surface_contact_search");
    while cells < max_cells {
        guard.tick()?;
        guard.check()?;
        let Some(cell) = queue.pop_front() else { break };
        cells += 1;
        let aa = rectangle_bounds(a, cell[0])?;
        let bb = rectangle_bounds(b, cell[1])?;
        if (0..3).any(|k| aa[k][1] < bb[k][0] || bb[k][1] < aa[k][0]) {
            continue;
        }
        if control_hulls
            && crate::surface_distance::rectangle_control_gap(a, b, cell)?
                .is_some_and(|gap| gap > 0.)
        {
            continue;
        }
        if classify(cell, &mut domain_cells)? == crate::trim_domain::Location::Outside {
            continue;
        }
        if domains.is_some() && domain_cells == max_domain_cells {
            queue.push_front(cell);
            break;
        }
        // Overlap search neighborhoods so a root on an artificial subdivision
        // plane can still be strictly enclosed. These probes prove existence
        // only; they never remove cells from the coverage queue.
        let mut probe = cell;
        for s in 0..2 {
            for k in 0..2 {
                let width = (cell[s][k][1] - cell[s][k][0]) * 0.5;
                probe[s][k] = [
                    (cell[s][k][0] - width).max(initial[s][k][0]),
                    (cell[s][k][1] + width).min(initial[s][k][1]),
                ];
            }
        }
        for axis in 0..2 {
            let fixed = cell[0][axis][0] * 0.5 + cell[0][axis][1] * 0.5;
            if let surface_contact::Verdict::Witness(w) =
                surface_contact::certify(a, b, axis, fixed, probe[0][1 - axis], probe[1])?
            {
                if classify([w.first_uv, w.second_uv], &mut domain_cells)?
                    == crate::trim_domain::Location::Inside
                {
                    contact = Some(w);
                    break;
                }
            }
            // Exclusion of one section cannot exclude the surrounding cell.
        }
        if contact.is_some() || (domains.is_some() && domain_cells == max_domain_cells) {
            queue.push_front(cell);
            break;
        }
        // Normalize by original parameter widths so rescaled charts do not
        // starve one axis. Split both closed halves: their shared boundary is
        // deliberately retained, including possible tangent contacts.
        let axis = (0..4)
            .filter(|&k| {
                let d = cell[k / 2][k % 2];
                let m = d[0] * 0.5 + d[1] * 0.5;
                m > d[0] && m < d[1]
            })
            .max_by(|&x, &y| {
                let width = |k: usize| {
                    let d = cell[k / 2][k % 2];
                    let n = initial[k / 2][k % 2];
                    (d[1] - d[0]) / (n[1] - n[0])
                };
                width(x).total_cmp(&width(y))
            });
        let Some(axis) = axis else {
            stalled.push(cell);
            continue;
        };
        let mut left = cell;
        let mut right = cell;
        let d = cell[axis / 2][axis % 2];
        let mid = d[0] * 0.5 + d[1] * 0.5;
        left[axis / 2][axis % 2][1] = mid;
        right[axis / 2][axis % 2][0] = mid;
        queue.push_back(left);
        queue.push_back(right);
    }
    stalled.extend(queue);
    Ok(Report {
        contact,
        cells,
        domain_cells,
        absence_proven: stalled.is_empty(),
        unresolved: stalled,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn plane() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            weights: vec![vec![1.; 2]; 2],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn separated_charts_are_proven_and_crossing_retains_unresolved_volume() {
        let a = plane();
        let mut b = plane();
        for row in &mut b.control_points {
            for p in row {
                p[2] = 2.;
            }
        }
        let r = search(&a, &b, 1).unwrap();
        assert!(r.absence_proven);
        assert_eq!(r.cells, 1);
        for row in &mut b.control_points {
            for p in row {
                p[2] = p[1] - 0.5;
                p[1] = 0.5;
            }
        }
        let r = search(&a, &b, 100).unwrap();
        assert!(r.contact.is_some());
        assert!(!r.absence_proven);
        assert_eq!(r.unresolved, vec![[natural(&a), natural(&b)]]);
    }
    #[test]
    fn coincident_charts_keep_full_parameter_coverage() {
        let a = plane();
        let r = search(&a, &a, 7).unwrap();
        assert!(!r.absence_proven);
        assert!(r.contact.is_none());
        assert_eq!(r.cells, 7);
        // Every retained cell is unresolved, and no section exclusion may
        // discard the diagonal {(u,v,u,v)} of the full parameter product.
        for i in 0..=16 {
            for j in 0..=16 {
                let p = [i as f64 / 16., j as f64 / 16.];
                assert!(r.unresolved.iter().any(|c| {
                    (0..2).all(|s| (0..2).all(|k| c[s][k][0] <= p[k] && c[s][k][1] >= p[k]))
                }));
            }
        }
    }
    #[test]
    fn overlapping_initial_bounds_need_subdivision_to_prove_separation() {
        let mut a = plane();
        for row in &mut a.control_points {
            for p in row {
                p[2] = p[0] + p[1];
            }
        }
        let mut b = a.clone();
        for row in &mut b.control_points {
            for p in row {
                p[2] += 0.5;
            }
        }
        assert!(!search(&a, &b, 1).unwrap().absence_proven);
        let r = search(&a, &b, 10000).unwrap();
        assert!(
            r.absence_proven,
            "{} unresolved after {}",
            r.unresolved.len(),
            r.cells
        );
        assert!(r.contact.is_none());
    }
    #[test]
    fn tangent_root_is_never_lost_at_subdivision_boundaries() {
        let a = plane();
        let mut b = plane();
        b.degree_u = 2;
        b.knots_u = vec![0., 0., 0., 1., 1., 1.];
        b.weights = vec![vec![1.; 2]; 3];
        b.control_points = (0..3)
            .map(|i| {
                (0..2)
                    .map(|j| vec![i as f64 * 0.5, j as f64, [0.25, -0.25, 0.25][i]])
                    .collect()
            })
            .collect();
        let r = search(&a, &b, 127).unwrap();
        assert!(!r.absence_proven);
        assert!(r.contact.is_none());
        assert!(
            r.unresolved
                .iter()
                .any(|c| c.iter().flatten().all(|d| d[0] <= 0.5 && d[1] >= 0.5))
        );
    }

    #[test]
    fn cell_guard_matches_the_work_limit() {
        // Guard sized exactly at max_cells: the loop can never outrun it.
        let mut guard = Budget::with_iterations(7)
            .unwrap()
            .guard("surface_contact_search");
        for _ in 0..7 {
            guard.tick().unwrap();
        }
        let err = guard.tick().unwrap_err();
        assert_eq!(err.code, crate::RESOURCE_LIMIT, "{err:?}");
        assert!(err.contains("surface_contact_search"));
        // And the public search still honors its own limit unchanged.
        let a = plane();
        let r = search(&a, &a, 7).unwrap();
        assert_eq!(r.cells, 7);
    }
}
