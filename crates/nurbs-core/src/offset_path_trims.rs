//! Original trim membership of a freshly certified connected offset path.
//! Adaptive membership leaves remain on their original parent root tube; no
//! sampled contact or caller-supplied report substitutes for root authority.
use crate::{
    check,
    curve::Curve,
    offset_contact_path as path,
    surface::Surface,
    surface_contact::Witness,
    surface_offset::{self, ContactBand},
    trim_domain::{Classification, Location, TrimDomain},
    trim_region_audit, Result,
};
#[derive(Clone, Copy)]
pub struct Limits {
    pub path: path::Limits,
    pub membership_cells: usize,
    pub region_pairs: usize,
    pub region_cells: usize,
    pub domain_cells: usize,
}
pub struct Cell {
    pub drive: [f64; 2],
    pub parent: usize,
    pub root: Option<Witness>,
    pub classifications: Vec<Classification>,
    pub admitted: bool,
}
pub struct Report {
    pub path: path::Report,
    pub regions: Vec<trim_region_audit::Report>,
    pub cells: Vec<Cell>,
    pub visited: usize,
    pub pairs: usize,
    pub region_cells: usize,
    pub domain_cells: usize,
    pub additional_band_queries: usize,
    pub trimmed_continuous_path_proven: bool,
    pub reason: &'static str,
}
pub fn certify(
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
        (1..=100000).contains(&limits.membership_cells)
            && (1..=100000).contains(&limits.region_pairs)
            && (1..=100000).contains(&limits.region_cells)
            && (1..=1000000).contains(&limits.domain_cells),
        "Choose bounded original-region and membership work",
    )?;
    // Both original loop sets validate even when a path or earlier audit stops.
    let domains = [
        TrimDomain::new(loops[0], tolerance_uv)?,
        TrimDomain::new(loops[1], tolerance_uv)?,
    ];
    let path = path::certify(surfaces, distances, axis, drive, seeds, limits.path)?;
    let cells = path
        .cells
        .iter()
        .enumerate()
        .map(|(parent, c)| Cell {
            drive: c.drive,
            parent,
            root: None,
            classifications: vec![],
            admitted: false,
        })
        .collect();
    let mut out = Report {
        path,
        regions: vec![],
        cells,
        visited: 0,
        pairs: 0,
        region_cells: 0,
        domain_cells: 0,
        additional_band_queries: 0,
        trimmed_continuous_path_proven: false,
        reason: "original-region-unresolved",
    };
    for side in 0..2 {
        if out.pairs == limits.region_pairs
            || out.region_cells == limits.region_cells
            || out.domain_cells == limits.domain_cells
        {
            out.reason = "original-region-work-limit";
            return Ok(out);
        }
        let r = trim_region_audit::inspect(
            loops[side],
            tolerance_uv,
            limits.region_pairs - out.pairs,
            limits.region_cells - out.region_cells,
            limits.domain_cells - out.domain_cells,
        )?;
        out.pairs += r.pairs;
        out.region_cells += r.cells;
        out.domain_cells += r.domain_cells;
        let valid = r.valid;
        out.regions.push(r);
        if valid != Some(true) {
            out.reason = if valid == Some(false) {
                "invalid-original-region"
            } else {
                "original-region-unresolved"
            };
            return Ok(out);
        }
    }
    if !out.path.continuous_path_proven {
        out.reason = "connected-offset-path-unresolved";
        return Ok(out);
    }
    let mut queue = out
        .path
        .cells
        .iter()
        .enumerate()
        .map(|(parent, c)| {
            let ContactBand::ContinuousBranch(w) = &c.verdict else {
                unreachable!()
            };
            (parent, c.drive, w.clone(), 0usize)
        })
        .collect::<std::collections::VecDeque<_>>();
    out.cells.clear();
    while let Some((parent, interval, mut root, depth)) = queue.pop_front() {
        if out.visited == limits.membership_cells || out.domain_cells == limits.domain_cells {
            out.cells.push(Cell {
                drive: interval,
                parent,
                root: Some(root),
                classifications: vec![],
                admitted: false,
            });
            continue;
        }
        out.visited += 1;
        if depth > 0 {
            let source = &out.path.cells[parent];
            out.additional_band_queries += 1;
            match surface_offset::certify_contact_band(
                surfaces,
                distances,
                axis,
                interval,
                source.first_other,
                source.second,
                limits.path.spans,
            )? {
                ContactBand::ContinuousBranch(w) => root = w,
                ContactBand::Unresolved => {} // Valid ancestor enclosure still covers this restricted branch.
                ContactBand::Excluded => {
                    return Err(crate::numeric_err(
                        "Refined root exclusion contradicts the certified parent tube",
                    ));
                }
            }
        }
        // Restrict the exact driving coordinate of an already certified branch.
        root.first_uv[axis] = interval;
        let mut classifications = vec![];
        for side in 0..2 {
            if out.domain_cells == limits.domain_cells {
                break;
            }
            let uv = if side == 0 {
                root.first_uv
            } else {
                root.second_uv
            };
            let r =
                domains[side].classify(uv, (limits.domain_cells - out.domain_cells).min(100000))?;
            out.domain_cells += r.cells;
            classifications.push(r);
        }
        let admitted = classifications.len() == 2
            && classifications
                .iter()
                .all(|r| r.location == Location::Inside);
        let outside = classifications
            .iter()
            .any(|r| r.location == Location::Outside);
        let mid = interval[0] * 0.5 + interval[1] * 0.5;
        if !admitted
            && !outside
            && out.visited < limits.membership_cells
            && out.domain_cells < limits.domain_cells
            && depth < 32
            && mid > interval[0]
            && mid < interval[1]
        {
            queue.push_back((parent, [interval[0], mid], root.clone(), depth + 1));
            queue.push_back((parent, [mid, interval[1]], root, depth + 1));
        } else {
            out.cells.push(Cell {
                drive: interval,
                parent,
                root: Some(root),
                classifications,
                admitted,
            });
        }
    }
    out.cells.sort_by(|a, b| a.drive[0].total_cmp(&b.drive[0]));
    out.trimmed_continuous_path_proven = out.cells.iter().all(|c| c.admitted);
    out.reason = if out.trimmed_continuous_path_proven {
        "connected-path-inside-both-original-trims"
    } else if out.cells.iter().any(|c| {
        c.classifications
            .iter()
            .any(|r| r.location == Location::Outside)
    }) {
        "contact-path-outside-original-trim"
    } else {
        "contact-path-trim-unresolved"
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
    fn rectangle(lo: [f64; 2], hi: [f64; 2], reverse: bool) -> Vec<Curve> {
        let mut p = vec![lo, [hi[0], lo[1]], hi, [lo[0], hi[1]]];
        if reverse {
            p.reverse();
        }
        (0..4)
            .map(|i| Curve::from_polyline(vec![p[i].to_vec(), p[(i + 1) % 4].to_vec()]).unwrap())
            .collect()
    }
    fn limits() -> Limits {
        Limits {
            path: path::Limits {
                cells: 63,
                spans: 16,
                iterations: 8,
                numerical_tolerance_mm: 1e-8,
                padding_fraction: 0.01,
            },
            membership_cells: 127,
            region_pairs: 10000,
            region_cells: 10000,
            domain_cells: 100000,
        }
    }
    fn query(loops: [Vec<Vec<Curve>>; 2], limits: Limits) -> Result<Report> {
        let a = plane();
        let mut b = plane();
        for p in b.control_points.iter_mut().flatten() {
            p.swap(1, 2);
        }
        certify(
            [&a, &b],
            [&loops[0], &loops[1]],
            [0.2, -0.2],
            0,
            [0.1, 0.9],
            [[[0.1, 0.], [0.1, 0.]], [[0.9, 0.], [0.9, 0.]]],
            1e-8,
            limits,
        )
    }
    fn outer() -> Vec<Vec<Curve>> {
        vec![rectangle([0., 0.], [1., 1.], false)]
    }
    fn cover(r: &Report) {
        assert_eq!(r.cells.first().unwrap().drive[0], 0.1);
        assert_eq!(r.cells.last().unwrap().drive[1], 0.9);
        for c in r.cells.windows(2) {
            assert_eq!(c[0].drive[1], c[1].drive[0]);
        }
    }
    #[test]
    fn full_connected_path_requires_both_original_regions() {
        let r = query([outer(), outer()], limits()).unwrap();
        cover(&r);
        assert!(r.trimmed_continuous_path_proven, "{}", r.reason);
        assert!(r.path.continuous_path_proven);
        assert_eq!(r.regions.len(), 2);
        assert!(r
            .cells
            .iter()
            .all(|c| c.classifications.len() == 2 && c.admitted));
        let mut second = outer();
        second.push(rectangle([0.4, 0.05], [0.6, 0.08], true));
        let r = query([outer(), second], limits()).unwrap();
        cover(&r);
        assert!(r.path.continuous_path_proven);
        assert!(!r.trimmed_continuous_path_proven);
        assert_eq!(r.reason, "contact-path-outside-original-trim");
        assert!(r.cells.iter().any(|c| c
            .classifications
            .iter()
            .any(|s| s.location == Location::Outside)));
        assert!(r.additional_band_queries > 0);
    }
    #[test]
    fn membership_and_region_work_stops_keep_complete_unqualified_coverage() {
        let mut first = outer();
        first.push(rectangle([0.4, 0.05], [0.6, 0.08], true));
        let mut work = limits();
        work.membership_cells = 1;
        let r = query([first, outer()], work).unwrap();
        cover(&r);
        assert!(r.path.continuous_path_proven && !r.trimmed_continuous_path_proven);
        assert_eq!(r.visited, 1);
        work = limits();
        work.region_pairs = 1;
        let r = query([outer(), outer()], work).unwrap();
        cover(&r);
        assert!(!r.trimmed_continuous_path_proven);
        assert!(r.visited == 0);
        work = limits();
        work.domain_cells = 1;
        let r = query([outer(), outer()], work).unwrap();
        cover(&r);
        assert!(!r.trimmed_continuous_path_proven);
        assert!(r.domain_cells <= 1);
    }
    #[test]
    fn malformed_second_region_cannot_hide_behind_first_work_stop() {
        let mut invalid = outer();
        invalid[0][0].weights[0] = 0.;
        let mut work = limits();
        work.region_pairs = 1;
        assert!(query([outer(), invalid], work).is_err());
    }
}
