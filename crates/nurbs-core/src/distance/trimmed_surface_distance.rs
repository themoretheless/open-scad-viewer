//! Distance over UV-trimmed surface images. Only proven outside cells are
//! removed, and only proven inside points may supply an upper distance bound.
use crate::distance::subdivision::{HeapQueue, Queue, Search, Split};
use crate::distance_bounds::{Interval, box_distance};
use crate::surface_distance::{Patch, Witness, at, patch_lower, patches};
use crate::{
    Result, check, resource,
    surface::Surface,
    trim_domain::{Location, TrimDomain},
};
use std::collections::HashMap;
#[cfg(feature = "codec")]
mod serialization;

#[derive(Debug)]
pub struct TrimmedDistance {
    pub lower_bound_mm: f64,
    pub upper_bound_mm: Option<f64>,
    pub parameters: Option<[[f64; 2]; 2]>,
    pub points: Option<[[f64; 3]; 2]>,
    pub point_enclosures: Option<[Vec<[f64; 2]>; 2]>,
    pub converged: bool,
    pub reason: crate::DistanceStopReason,
    pub cells: usize,
    pub max_cells: usize,
    pub domain_cells: usize,
    pub max_domain_cells: usize,
    pub tolerance_mm: f64,
}

struct Domains<'a> {
    regions: [&'a TrimDomain; 2],
    cells: usize,
    max: usize,
    points: [HashMap<[u64; 2], Location>; 2],
}
impl Domains<'_> {
    fn classify(&mut self, side: usize, rectangle: [[f64; 2]; 2]) -> Result<Location> {
        let remaining = self.max - self.cells;
        if remaining == 0 {
            return Ok(Location::Unresolved);
        }
        // A single difficult boundary query must not monopolize the complete
        // search. Unresolved cells stay in the queue; unresolved points are not
        // witnesses. This limit never changes an unresolved answer to outside.
        let result = self.regions[side].classify(rectangle, remaining.min(4096))?;
        self.cells += result.cells;
        Ok(result.location)
    }
    fn point(&mut self, side: usize, uv: [f64; 2]) -> Result<Location> {
        let key = uv.map(f64::to_bits);
        if let Some(&v) = self.points[side].get(&key) {
            return Ok(v);
        }
        let result = self.classify(side, uv.map(|t| [t; 2]))?;
        self.points[side].insert(key, result);
        Ok(result)
    }
}
#[derive(Clone)]
struct Part {
    patch: Patch,
    location: Location,
}
impl Part {
    fn score(&self, axis: usize) -> f64 {
        let parameter = if self.location == Location::Unresolved {
            self.patch.parameter_score(axis)
        } else {
            self.patch.score(axis)
        };
        if parameter < 0. {
            return parameter;
        }
        // Allocate subdivisions by physical image size, not equal UV fractions
        // on differently sized faces. Coverage and pruning remain unchanged.
        let largest = self
            .patch
            .parameter_score(0)
            .max(self.patch.parameter_score(1));
        let extent = self.patch.spatial_size();
        parameter / largest * if extent > 0. { extent } else { 1. }
    }
}
struct Cell {
    parts: [Part; 2],
}
fn midpoint([lo, hi]: [f64; 2]) -> f64 {
    lo * 0.5 + hi * 0.5
}
struct Sample {
    raw: [f64; 2],
    parameters: [f64; 2],
    point: [f64; 3],
    bounds: Vec<Interval>,
}
fn sample_pair(
    a: &Surface,
    b: &Surface,
    parts: &[Part; 2],
    domains: &mut Domains,
    best: &mut Option<Witness>,
) -> Result<()> {
    let mut samples: [Vec<Sample>; 2] = [Vec::new(), Vec::new()];
    for side in 0..2 {
        let [[u0, u1], [v0, v1]] = parts[side].patch.domain;
        let values = |lo: f64, hi: f64| {
            [
                lo * 0.875 + hi * 0.125,
                midpoint([lo, hi]),
                lo * 0.125 + hi * 0.875,
            ]
        };
        // Probe both sides of a trim band: its midpoint may be on the boundary.
        for u in values(u0, u1) {
            for v in values(v0, v1) {
                let (parameters, point, bounds) = at([a, b][side], [u, v])?;
                samples[side].push(Sample {
                    raw: [u, v],
                    parameters,
                    point,
                    bounds,
                });
            }
        }
    }
    for sa in &samples[0] {
        for sb in &samples[1] {
            let upper = box_distance(&sa.bounds, &sb.bounds)?.1;
            if best.as_ref().is_some_and(|w| w.upper <= upper) {
                continue;
            }
            let pair = [sa, sb];
            let mut admitted = true;
            for side in 0..2 {
                // The returned parameter may be normalized across a periodic seam.
                // In that case the original cell's Inside proof does not suffice.
                if (parts[side].location != Location::Inside
                    || pair[side].parameters != pair[side].raw)
                    && domains.point(side, pair[side].parameters)? != Location::Inside
                {
                    admitted = false;
                    break;
                }
            }
            if admitted {
                *best = Some(Witness {
                    parameters: [sa.parameters, sb.parameters],
                    points: [sa.point, sb.point],
                    bounds: [sa.bounds.clone(), sb.bounds.clone()],
                    upper,
                });
            }
        }
    }
    Ok(())
}
struct TrimmedSearch<'a> {
    a: &'a Surface,
    b: &'a Surface,
    domains: Domains<'a>,
    best: Option<Witness>,
}
impl Search for TrimmedSearch<'_> {
    type Cell = Cell;
    fn upper(&self) -> Option<f64> {
        self.best.as_ref().map(|w| w.upper)
    }
    fn early_stop(&self, queue_empty: bool) -> Option<crate::DistanceStopReason> {
        if queue_empty {
            return Some(crate::DistanceStopReason::EmptyDomain);
        }
        if self.domains.cells == self.domains.max {
            return Some(crate::DistanceStopReason::DomainWorkLimit);
        }
        None
    }
    fn split(&mut self, cell: Cell) -> Result<Split<Cell>> {
        let (side, axis, score) = (0..2)
            .flat_map(|side| (0..2).map(move |axis| (side, axis)))
            .map(|(side, axis)| (side, axis, cell.parts[side].score(axis)))
            .max_by(|a, b| a.2.total_cmp(&b.2))
            .unwrap();
        if score < 0. {
            return Ok(Split::Precision(cell));
        }
        let selected = &cell.parts[side];
        let [lo, hi] = selected.patch.domain[axis];
        let mid = if selected.location == Location::Unresolved {
            self.domains.regions[side]
                .split_hint(selected.patch.domain, axis)
                .unwrap_or_else(|| midpoint([lo, hi]))
        } else {
            midpoint([lo, hi])
        };
        let mut children = [None, None];
        for (i, range) in [[lo, mid], [mid, hi]].into_iter().enumerate() {
            let mut domain = selected.patch.domain;
            domain[axis] = range;
            let location = if selected.location == Location::Inside {
                Location::Inside
            } else {
                self.domains.classify(side, domain)?
            };
            if location == Location::Outside {
                continue;
            }
            let patch = Patch::new([self.a, self.b][side], selected.patch.span, domain)?;
            let mut parts = cell.parts.clone();
            parts[side] = Part { patch, location };
            sample_pair(
                self.a,
                self.b,
                &parts,
                &mut self.domains,
                &mut self.best,
            )?;
            let lower = patch_lower(&parts[0].patch, &parts[1].patch)?;
            if self.best.as_ref().is_none_or(|w| lower <= w.upper) {
                children[i] = Some((Cell { parts }, lower));
            }
        }
        Ok(Split::Children(children))
    }
}

/// Bounds cover both trimmed images, with joins interpreted by TrimDomain's
/// explicit UV tolerance. A missing witness is represented by null upper bound,
/// never by zero distance or fabricated points. This is not volume containment.
pub fn distance(
    a: &Surface,
    domain_a: &TrimDomain,
    b: &Surface,
    domain_b: &TrimDomain,
    tolerance_mm: f64,
    max_cells: usize,
    max_domain_cells: usize,
) -> Result<TrimmedDistance> {
    a.validate()?;
    b.validate()?;
    check(
        tolerance_mm.is_finite() && tolerance_mm > 0.,
        "Trimmed surface distance tolerance must be positive and finite",
    )?;
    check(
        (1..=100000).contains(&max_cells),
        "Trimmed surface distance needs 1..100000 cells",
    )?;
    check(
        (1..=8000000).contains(&max_domain_cells),
        "Trimmed surface distance needs 1..8000000 domain cells",
    )?;
    let aa = patches(a)?;
    let bb = patches(b)?;
    if aa.len().saturating_mul(bb.len()) > max_cells {
        return Err(resource(
            "Trimmed distance initial knot pairs exceed the cell budget",
        ));
    }
    let mut domains = Domains {
        regions: [domain_a, domain_b],
        cells: 0,
        max: max_domain_cells,
        points: std::array::from_fn(|_| HashMap::new()),
    };
    let mut prepare = |patches: Vec<Patch>, side| -> Result<Vec<Part>> {
        patches
            .into_iter()
            .map(|patch| {
                let location = domains.classify(side, patch.domain)?;
                Ok(Part { patch, location })
            })
            .collect()
    };
    let aa = prepare(aa, 0)?;
    let bb = prepare(bb, 1)?;
    let mut queue = HeapQueue::default();
    let mut best = None;
    let mut cells = 0;
    for pa in &aa {
        for pb in &bb {
            cells += 1;
            if pa.location == Location::Outside || pb.location == Location::Outside {
                continue;
            }
            let parts = [pa.clone(), pb.clone()];
            if domains.cells < domains.max {
                sample_pair(a, b, &parts, &mut domains, &mut best)?;
            }
            queue.push(Cell { parts }, patch_lower(&pa.patch, &pb.patch)?);
        }
    }
    let mut search = TrimmedSearch {
        a,
        b,
        domains,
        best,
    };
    let outcome =
        crate::distance::subdivision::run(&mut search, &mut queue, cells, max_cells, tolerance_mm)?;
    let best = search.best;
    let upper_bound_mm = best.as_ref().map(|w| w.upper);
    // Result control points must be finite before leaving the module (1093).
    if let Some(w) = &best {
        crate::foundation::guards::require_finite_point(&w.points[0], "trimmed point a")?;
        crate::foundation::guards::require_finite_point(&w.points[1], "trimmed point b")?;
    }
    Ok(TrimmedDistance {
        lower_bound_mm: outcome.lower,
        upper_bound_mm,
        parameters: best.as_ref().map(|w| w.parameters),
        points: best.as_ref().map(|w| w.points),
        point_enclosures: best.map(|w| {
            w.bounds
                .map(|p| p.into_iter().map(|i| [i.lo, i.hi]).collect())
        }),
        converged: outcome.reason == crate::DistanceStopReason::Tolerance,
        reason: outcome.reason,
        cells: outcome.cells,
        max_cells,
        domain_cells: search.domains.cells,
        max_domain_cells,
        tolerance_mm,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::Curve;
    fn rectangle(x: [f64; 2], y: [f64; 2]) -> Curve {
        Curve::from_polyline(vec![
            vec![x[0], y[0]],
            vec![x[1], y[0]],
            vec![x[1], y[1]],
            vec![x[0], y[1]],
            vec![x[0], y[0]],
        ])
        .unwrap()
    }
    fn region(x: [f64; 2], y: [f64; 2]) -> TrimDomain {
        TrimDomain::new(&[vec![rectangle(x, y)]], 1e-7).unwrap()
    }
    fn plane(x: [f64; 2], y: [f64; 2], z: f64) -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![x[0], y[0], z], vec![x[0], y[1], z]],
                vec![vec![x[1], y[0], z], vec![x[1], y[1], z]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    fn hole() -> TrimDomain {
        TrimDomain::new(
            &[
                vec![rectangle([0., 1.], [0., 1.])],
                vec![rectangle([0.4, 0.6], [0.4, 0.6]).reverse().unwrap()],
            ],
            1e-7,
        )
        .unwrap()
    }
    fn contains(r: &TrimmedDistance, value: f64) {
        assert!(
            r.lower_bound_mm <= value && r.upper_bound_mm.unwrap() >= value,
            "expected {value}, {r:?}"
        );
    }
    #[test]
    fn excludes_hole_interior_from_upper_witnesses() {
        let a = plane([0., 10.], [0., 10.], 0.);
        let b = plane([5., 5.], [5., 5.], 2.);
        let da = hole();
        let db = region([-1., 2.], [-1., 2.]);
        let full = crate::surface_distance::distance(&a, &b, 0.001, 10000).unwrap();
        assert!(full.distance_interval_mm[1] < 2.001);
        let r = distance(&a, &da, &b, &db, 0.002, 100000, 1000000).unwrap();
        contains(&r, 5_f64.sqrt());
        assert!(r.converged, "{r:?}");
        for (i, d) in [&da, &db].into_iter().enumerate() {
            assert_eq!(
                d.classify(r.parameters.unwrap()[i].map(|t| [t; 2]), 10000)
                    .unwrap()
                    .location,
                Location::Inside
            );
        }
        assert!(
            r.points.unwrap()[0][0] < 4.
                || r.points.unwrap()[0][0] > 6.
                || r.points.unwrap()[0][1] < 4.
                || r.points.unwrap()[0][1] > 6.
        );
    }
    #[test]
    fn two_finite_trimmed_patches_approach_the_hole_boundary() {
        let a = plane([0., 10.], [0., 10.], 0.);
        let b = plane([4.75, 5.25], [4.75, 5.25], 2.);
        let da = hole();
        let db = region([0., 1.], [0., 1.]);
        let r = distance(&a, &da, &b, &db, 0.005, 100000, 1000000).unwrap();
        contains(&r, (4.0_f64 + 0.75 * 0.75).sqrt());
        assert!(r.converged, "{r:?}");
        for (i, s) in [&a, &b].into_iter().enumerate() {
            let uv = r.parameters.unwrap()[i];
            assert_eq!(
                r.points.unwrap()[i],
                s.evaluate(uv[0], uv[1]).unwrap().point
            );
        }
    }
    #[test]
    #[cfg(feature = "codec")]
    fn budgets_keep_coverage_and_do_not_fabricate_a_witness() {
        let a = plane([0., 10.], [0., 10.], 0.);
        let b = plane([5., 5.], [5., 5.], 2.);
        let da = hole();
        let db = region([-1., 2.], [-1., 2.]);
        for (cells, domain, reason) in [
            (1, 1000, crate::DistanceStopReason::WorkLimit),
            (1000, 1, crate::DistanceStopReason::DomainWorkLimit),
        ] {
            let r = distance(&a, &da, &b, &db, 0.001, cells, domain).unwrap();
            assert_eq!(r.reason, reason);
            assert!(!r.converged);
            if domain == 1 {
                assert!(r.upper_bound_mm.is_none());
                assert!(r.parameters.is_none());
                assert!(r.points.is_none());
                assert!(r.to_value()["distanceIntervalMm"][1].is_null());
            } else {
                contains(&r, 5_f64.sqrt());
            }
            assert!(r.lower_bound_mm <= 5_f64.sqrt());
            assert!(r.cells <= cells && r.domain_cells <= domain);
        }
    }
    #[test]
    fn rational_circular_hole_excludes_the_surface_center() {
        let mut c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1., 1., 1.],
            control_points: vec![
                [1., 0.],
                [1., 1.],
                [0., 1.],
                [-1., 1.],
                [-1., 0.],
                [-1., -1.],
                [0., -1.],
                [1., -1.],
                [1., 0.],
            ]
            .into_iter()
            .map(|p| p.into_iter().map(|x| 0.5 + 0.2 * x).collect())
            .collect(),
            weights: vec![
                1.,
                std::f64::consts::FRAC_1_SQRT_2,
                1.,
                std::f64::consts::FRAC_1_SQRT_2,
                1.,
                std::f64::consts::FRAC_1_SQRT_2,
                1.,
                std::f64::consts::FRAC_1_SQRT_2,
                1.,
            ],
            periodic: false,
        };
        c = c.reverse().unwrap();
        let da = TrimDomain::new(&[vec![rectangle([0., 1.], [0., 1.])], vec![c]], 1e-7).unwrap();
        let db = region([-1., 2.], [-1., 2.]);
        let a = plane([0., 10.], [0., 10.], 0.);
        let b = plane([5., 5.], [5., 5.], 2.);
        let r = distance(&a, &da, &b, &db, 0.003, 100000, 4000000).unwrap();
        contains(&r, 8_f64.sqrt());
        assert!(r.converged, "{r:?}");
        let p = r.points.unwrap()[0];
        assert!((p[0] - 5.).hypot(p[1] - 5.) > 2.);
    }
    #[test]
    fn empty_domain_is_distinct_from_contact() {
        let a = plane([0., 10.], [0., 10.], 0.);
        let outside = region([2., 3.], [2., 3.]);
        let full = region([-1., 2.], [-1., 2.]);
        let r = distance(&a, &outside, &a, &full, 0.001, 100, 10000).unwrap();
        assert_eq!(r.reason, crate::DistanceStopReason::EmptyDomain);
        assert!(!r.converged);
        assert!(r.upper_bound_mm.is_none());
        assert!(distance(&a, &full, &a, &full, 0., 100, 100).is_err());
        assert!(distance(&a, &full, &a, &full, 0.001, 0, 100).is_err());
        assert!(distance(&a, &full, &a, &full, 0.001, 100, 0).is_err());
    }
    #[test]
    fn periodic_witness_stays_in_the_admitted_uv_branch() {
        let mut a = plane([0., 1.], [0., 1.], 0.);
        a.periodic_u = true;
        a.knots_u = vec![-1., 0., 1., 2., 3., 4., 5.];
        a.control_points = vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]]
            .into_iter()
            .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 1.]])
            .collect();
        a.weights = vec![vec![1.; 2]; 5];
        let b = plane([-1., -1.], [0., 0.], 0.5);
        let da = region([3.5, 4.5], [-1., 2.]);
        let db = region([-1., 2.], [-1., 2.]);
        let r = distance(&a, &da, &b, &db, 0.001, 100000, 1000000).unwrap();
        contains(&r, 1.);
        assert!(r.converged, "{r:?}");
        assert!(r.parameters.unwrap()[0][0] > 3.5);
    }
    #[test]
    fn guards_reject_nan_and_keep_witnesses_finite() {
        let a = plane([0., 10.], [0., 10.], 0.);
        let b = plane([5., 5.], [5., 5.], 2.);
        let da = region([-1., 2.], [-1., 2.]);
        let db = region([-1., 2.], [-1., 2.]);
        let err = distance(&a, &da, &b, &db, f64::NAN, 100, 1000).unwrap_err();
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
        let r = distance(&a, &da, &b, &db, 0.005, 10000, 100000).unwrap();
        assert!(r.converged, "{r:?}");
        assert!(r.points.unwrap().iter().flatten().all(|v| v.is_finite()));
    }
}
