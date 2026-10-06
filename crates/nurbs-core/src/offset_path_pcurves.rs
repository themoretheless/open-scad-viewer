//! Shared endpoint UV proposals with complete branch and original-trim checks.
//! Piecewise linear pcurves approximate source contacts within a declared UV
//! tolerance. They do not construct canonical world edges or fillet solids.
use crate::{
    check,
    curve::Curve,
    offset_contact_pcurve as correspondence, offset_path_trims as trims,
    surface::Surface,
    surface_contact::{Verdict, Witness},
    surface_offset,
    trim_domain::{Classification, Location, TrimDomain},
    Result,
};
#[derive(Clone, Copy)]
pub struct Limits {
    pub trims: trims::Limits,
    pub proposal_cells: usize,
    pub correspondence_cells: usize,
    pub cells_per_side: usize,
    pub station_queries: usize,
    pub station_refinements: usize,
    pub domain_cells: usize,
}
pub struct Station {
    pub parameter: f64,
    pub uv: Option<[[f64; 2]; 2]>,
    pub witness: Option<Witness>,
    pub queries: usize,
}
pub struct Attempt {
    pub interval: [f64; 2],
    pub parent: usize,
    pub stations: [usize; 2],
    pub curves: Option<[Curve; 2]>,
    pub correspondence: Vec<correspondence::Report>,
    pub memberships: Vec<Classification>,
    pub admitted: bool,
}
pub struct Piece {
    pub interval: [f64; 2],
    pub attempt: Option<usize>,
    pub admitted: bool,
}
pub struct Report {
    pub contact: trims::Report,
    pub stations: Vec<Station>,
    pub attempts: Vec<Attempt>,
    pub pieces: Vec<Piece>,
    pub visited: usize,
    pub station_queries: usize,
    pub correspondence_cells: usize,
    pub correspondence_band_queries: usize,
    pub correspondence_section_queries: usize,
    pub domain_cells: usize,
    pub shared_endpoints_proven: bool,
    pub source_pcurves_proven: bool,
    pub reason: &'static str,
}
fn station(
    t: f64,
    axis: usize,
    surfaces: [&Surface; 2],
    distances: [f64; 2],
    limits: Limits,
    report: &mut Report,
    cache: &mut std::collections::BTreeMap<u64, usize>,
) -> Result<usize> {
    if let Some(&i) = cache.get(&t.to_bits()) {
        return Ok(i);
    }
    let i = report.stations.len();
    let mut node = Station {
        parameter: t,
        uv: None,
        witness: None,
        queries: 0,
    };
    let mut free = None;
    let mut second = None;
    // A shared boundary station is checked in the intersection of every
    // incident original tube, retaining the connected parent's root identity.
    for cell in &report.contact.path.cells {
        if t < cell.drive[0] || t > cell.drive[1] {
            continue;
        }
        let intersect = |a: [f64; 2], b: [f64; 2]| [a[0].max(b[0]), a[1].min(b[1])];
        free = Some(free.map_or(cell.first_other, |f| intersect(f, cell.first_other)));
        second = Some(second.map_or(cell.second, |s: [[f64; 2]; 2]| {
            std::array::from_fn(|k| intersect(s[k], cell.second[k]))
        }));
    }
    if let (Some(mut free), Some(mut second)) = (free, second) {
        for _ in 0..=limits.station_refinements {
            if report.station_queries == limits.station_queries
                || free[0] >= free[1]
                || second.iter().any(|d| d[0] >= d[1])
            {
                break;
            }
            report.station_queries += 1;
            node.queries += 1;
            match surface_offset::certify_contact_section(
                surfaces,
                distances,
                axis,
                t,
                free,
                second,
                limits.trims.path.spans,
            )? {
                Verdict::Witness(w) => {
                    free = w.first_uv[1 - axis];
                    second = w.second_uv;
                    node.witness = Some(w);
                }
                Verdict::Excluded if node.witness.is_some() => {
                    return Err(crate::numeric_err(
                        "Station refinement excludes an already certified root",
                    ));
                }
                Verdict::Excluded | Verdict::Unresolved => break,
            }
        }
    }
    if let Some(w) = &node.witness {
        let mid = |r: [f64; 2]| r[0] * 0.5 + r[1] * 0.5;
        let mut a = w.first_uv.map(mid);
        a[axis] = t;
        node.uv = Some([a, w.second_uv.map(mid)]);
    }
    report.stations.push(node);
    cache.insert(t.to_bits(), i);
    Ok(i)
}
pub fn propose(
    surfaces: [&Surface; 2],
    loops: [&[Vec<Curve>]; 2],
    distances: [f64; 2],
    axis: usize,
    drive: [f64; 2],
    seeds: [[[f64; 2]; 2]; 2],
    tolerance_uv: f64,
    limits: Limits,
) -> Result<Report> {
    check(
        (1..=100000).contains(&limits.proposal_cells)
            && (1..=100000).contains(&limits.correspondence_cells)
            && (1..=100000).contains(&limits.cells_per_side)
            && (1..=100000).contains(&limits.station_queries)
            && limits.station_refinements <= 16
            && (1..=1000000).contains(&limits.domain_cells),
        "Choose bounded pcurve proposal, correspondence and station work",
    )?;
    let domains = [
        TrimDomain::new(loops[0], tolerance_uv)?,
        TrimDomain::new(loops[1], tolerance_uv)?,
    ];
    let contact = trims::certify(
        surfaces,
        loops,
        distances,
        axis,
        drive,
        seeds,
        tolerance_uv,
        limits.trims,
    )?;
    let pieces = contact
        .cells
        .iter()
        .map(|c| Piece {
            interval: c.drive,
            attempt: None,
            admitted: false,
        })
        .collect();
    let mut out = Report {
        contact,
        stations: vec![],
        attempts: vec![],
        pieces,
        visited: 0,
        station_queries: 0,
        correspondence_cells: 0,
        correspondence_band_queries: 0,
        correspondence_section_queries: 0,
        domain_cells: 0,
        shared_endpoints_proven: false,
        source_pcurves_proven: false,
        reason: "original-contact-path-unqualified",
    };
    if !out.contact.trimmed_continuous_path_proven {
        return Ok(out);
    }
    let mut queue = out
        .contact
        .cells
        .iter()
        .map(|c| (c.parent, c.drive, 0usize))
        .collect::<std::collections::VecDeque<_>>();
    let mut cache = std::collections::BTreeMap::new();
    out.pieces.clear();
    while let Some((parent, interval, depth)) = queue.pop_front() {
        if out.visited == limits.proposal_cells
            || out.correspondence_cells == limits.correspondence_cells
            || out.domain_cells == limits.domain_cells
        {
            out.pieces.push(Piece {
                interval,
                attempt: None,
                admitted: false,
            });
            continue;
        }
        out.visited += 1;
        let stations = [
            station(
                interval[0],
                axis,
                surfaces,
                distances,
                limits,
                &mut out,
                &mut cache,
            )?,
            station(
                interval[1],
                axis,
                surfaces,
                distances,
                limits,
                &mut out,
                &mut cache,
            )?,
        ];
        let mut attempt = Attempt {
            interval,
            parent,
            stations,
            curves: None,
            correspondence: vec![],
            memberships: vec![],
            admitted: false,
        };
        if let (Some(a), Some(b)) = (out.stations[stations[0]].uv, out.stations[stations[1]].uv) {
            let curves = std::array::from_fn(|side| Curve {
                degree: 1,
                knots: vec![interval[0], interval[0], interval[1], interval[1]],
                control_points: vec![a[side].to_vec(), b[side].to_vec()],
                weights: vec![1.; 2],
                periodic: false,
            });
            let tube = &out.contact.path.cells[parent];
            for side in 0..2 {
                if out.correspondence_cells == limits.correspondence_cells {
                    break;
                }
                let r = correspondence::certify(
                    &curves[side],
                    side,
                    surfaces,
                    distances,
                    axis,
                    interval,
                    tube.first_other,
                    tube.second,
                    limits.trims.path.spans,
                    tolerance_uv,
                    (limits.correspondence_cells - out.correspondence_cells)
                        .min(limits.cells_per_side),
                    limits.station_refinements,
                )?;
                out.correspondence_cells += r.visited;
                out.correspondence_band_queries += r.band_queries;
                out.correspondence_section_queries += r.section_queries;
                attempt.correspondence.push(r);
            }
            if attempt.correspondence.len() == 2
                && attempt
                    .correspondence
                    .iter()
                    .all(|r| r.correspondence_proven)
            {
                for side in 0..2 {
                    if out.domain_cells == limits.domain_cells {
                        break;
                    }
                    let hull = crate::curve_surface_agreement::curve_bounds(
                        &curves[side],
                        crate::distance_bounds::Interval::new(interval[0], interval[1])?,
                    )?;
                    let r = domains[side].classify(
                        [[hull[0].lo, hull[0].hi], [hull[1].lo, hull[1].hi]],
                        (limits.domain_cells - out.domain_cells).min(100000),
                    )?;
                    out.domain_cells += r.cells;
                    attempt.memberships.push(r);
                }
            }
            attempt.curves = Some(curves);
        }
        attempt.admitted = attempt.correspondence.len() == 2
            && attempt
                .correspondence
                .iter()
                .all(|r| r.correspondence_proven)
            && attempt.memberships.len() == 2
            && attempt
                .memberships
                .iter()
                .all(|r| r.location == Location::Inside);
        let admitted = attempt.admitted;
        let has_endpoints = attempt.curves.is_some();
        let index = out.attempts.len();
        out.attempts.push(attempt);
        let mid = interval[0] * 0.5 + interval[1] * 0.5;
        if !admitted
            && has_endpoints
            && out.visited < limits.proposal_cells
            && out.correspondence_cells < limits.correspondence_cells
            && out.domain_cells < limits.domain_cells
            && out.station_queries < limits.station_queries
            && depth < 32
            && mid > interval[0]
            && mid < interval[1]
        {
            queue.push_back((parent, [interval[0], mid], depth + 1));
            queue.push_back((parent, [mid, interval[1]], depth + 1));
        } else {
            out.pieces.push(Piece {
                interval,
                attempt: Some(index),
                admitted,
            });
        }
    }
    out.pieces
        .sort_by(|a, b| a.interval[0].total_cmp(&b.interval[0]));
    if out.pieces.iter().all(|p| p.admitted) {
        out.shared_endpoints_proven = out.pieces.windows(2).all(|pair| {
            let a = out.attempts[pair[0].attempt.unwrap()]
                .curves
                .as_ref()
                .unwrap();
            let b = out.attempts[pair[1].attempt.unwrap()]
                .curves
                .as_ref()
                .unwrap();
            pair[0].interval[1].to_bits() == pair[1].interval[0].to_bits()
                && (0..2).all(|side| {
                    a[side]
                        .control_points
                        .last()
                        .unwrap()
                        .iter()
                        .zip(&b[side].control_points[0])
                        .all(|(x, y)| x.to_bits() == y.to_bits())
                })
        });
        out.source_pcurves_proven = out.shared_endpoints_proven;
    }
    out.reason = if out.source_pcurves_proven {
        "shared-source-pcurves-qualified"
    } else {
        "source-pcurve-proposal-unqualified"
    };
    Ok(out)
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
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 3., 0.]],
                vec![vec![3., 0., 0.], vec![3., 3., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    fn loops() -> Vec<Vec<Curve>> {
        let p = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
        vec![(0..4)
            .map(|i| Curve::from_polyline(vec![p[i].to_vec(), p[(i + 1) % 4].to_vec()]).unwrap())
            .collect()]
    }
    fn limits() -> Limits {
        Limits {
            trims: trims::Limits {
                path: crate::offset_contact_path::Limits {
                    cells: 255,
                    spans: 16,
                    iterations: 8,
                    numerical_tolerance_mm: 1e-8,
                    padding_fraction: 0.01,
                },
                membership_cells: 255,
                region_pairs: 10000,
                region_cells: 10000,
                domain_cells: 100000,
            },
            proposal_cells: 511,
            correspondence_cells: 4096,
            cells_per_side: 1,
            station_queries: 4096,
            station_refinements: 4,
            domain_cells: 100000,
        }
    }
    fn cover(r: &Report, drive: [f64; 2]) {
        assert_eq!(r.pieces.first().unwrap().interval[0], drive[0]);
        assert_eq!(r.pieces.last().unwrap().interval[1], drive[1]);
        for p in r.pieces.windows(2) {
            assert_eq!(p[0].interval[1], p[1].interval[0]);
        }
    }
    fn planar(work: Limits) -> Report {
        let a = plane();
        let mut b = plane();
        for p in b.control_points.iter_mut().flatten() {
            p.swap(1, 2);
        }
        let loops = loops();
        propose(
            [&a, &b],
            [&loops, &loops],
            [0.2, -0.2],
            0,
            [0.1, 0.9],
            [[[0.1, 0.], [0.1, 0.]], [[0.9, 0.], [0.9, 0.]]],
            1e-5,
            work,
        )
        .unwrap()
    }
    #[test]
    fn planar_source_pcurves_have_complete_branch_and_region_evidence() {
        let r = planar(limits());
        cover(&r, [0.1, 0.9]);
        assert!(r.source_pcurves_proven, "{}", r.reason);
        assert_eq!(r.pieces.len(), 1);
        assert_eq!(r.stations.len(), 2);
        assert!(r
            .attempts
            .iter()
            .all(|a| a.admitted && a.correspondence.len() == 2 && a.memberships.len() == 2));
        assert!(r.shared_endpoints_proven);
    }
    #[test]
    fn rational_contact_adapts_with_bit_identical_shared_uv_endpoints() {
        let mut a = plane();
        a.degree_u = 2;
        a.knots_u = vec![0., 0., 0., 1., 1., 1.];
        a.control_points = [[3., 0.], [3., 3.], [0., 3.]]
            .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 5.]])
            .to_vec();
        a.weights = [1., std::f64::consts::FRAC_1_SQRT_2, 1.]
            .map(|w| vec![w; 2])
            .to_vec();
        let b = plane();
        let loops = loops();
        let seeds = [0.2, 0.8].map(|u| {
            let p = surface_offset::evaluate(&a, [u, 0.], -0.2).unwrap().point;
            [[u, 0.], [p[0] / 3., p[1] / 3.]]
        });
        let r = propose(
            [&a, &b],
            [&loops, &loops],
            [-0.2, 0.2],
            0,
            [0.2, 0.8],
            seeds,
            1e-4,
            limits(),
        )
        .unwrap();
        cover(&r, [0.2, 0.8]);
        assert!(
            r.source_pcurves_proven,
            "{}; pieces={}, visits={}, stations={}, uv={}",
            r.reason,
            r.pieces.len(),
            r.visited,
            r.station_queries,
            r.correspondence_cells
        );
        assert!(r.pieces.len() > 1 && r.shared_endpoints_proven);
        for pair in r.pieces.windows(2) {
            let a = &r.attempts[pair[0].attempt.unwrap()];
            let b = &r.attempts[pair[1].attempt.unwrap()];
            assert_eq!(a.stations[1], b.stations[0]);
            for side in 0..2 {
                let left = a.curves.as_ref().unwrap()[side]
                    .control_points
                    .last()
                    .unwrap();
                let right = &b.curves.as_ref().unwrap()[side].control_points[0];
                assert!(left
                    .iter()
                    .zip(right)
                    .all(|(x, y)| x.to_bits() == y.to_bits()));
            }
        }
        assert!(
            r.visited <= limits().proposal_cells && r.station_queries <= limits().station_queries
        );
        assert!(r.correspondence_cells <= limits().correspondence_cells);
        assert!(r.domain_cells <= limits().domain_cells);
    }
    #[test]
    fn station_and_correspondence_work_stops_cannot_admit_partial_curves() {
        let mut work = limits();
        work.station_queries = 1;
        let r = planar(work);
        cover(&r, [0.1, 0.9]);
        assert!(!r.source_pcurves_proven);
        assert_eq!(r.station_queries, 1);
        assert!(r.attempts[0].curves.is_none());
        work = limits();
        work.correspondence_cells = 1;
        let r = planar(work);
        cover(&r, [0.1, 0.9]);
        assert!(!r.source_pcurves_proven);
        assert_eq!(r.correspondence_cells, 1);
        assert_eq!(r.attempts[0].correspondence.len(), 1);
        work = limits();
        work.trims.region_pairs = 1;
        let r = planar(work);
        cover(&r, [0.1, 0.9]);
        assert!(!r.source_pcurves_proven);
        assert_eq!(r.station_queries, 0);
        assert!(r.attempts.is_empty());
    }
}
