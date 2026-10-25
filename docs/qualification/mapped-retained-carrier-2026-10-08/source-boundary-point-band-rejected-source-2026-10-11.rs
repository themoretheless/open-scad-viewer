//! Winding of unchanged source curves with root-valued endpoints. Chords are
//! winding witnesses only; they never become replacement geometry.
use crate::source_boundary_fragment::{Endpoint, Fragment};
use nurbs_core::{
    Error, Result,
    curve::Curve,
    interval_eval::{self, Interval as I},
    trim_domain::Location,
};
use std::{borrow::Borrow, collections::BTreeMap};
pub struct Report {
    pub location: Location,
    pub winding: Option<i32>,
    pub cells: usize,
    pub endpoint_queries: usize,
    pub reason: &'static str,
    pub uncertain: Option<(usize, usize)>,
    /// The resolved result also applies to the full padded bounding rectangle.
    pub rectangle_proven: bool,
    pub enclosure_queries: usize,
    pub enclosure_cache_hits: usize,
}
/// Query-local cache owns only references to one immutable contour inventory.
/// Its keys are loop/fragment indexes and exact parameter endpoint bits.
pub struct Prepared<'a> {
    loops: Vec<Vec<&'a Fragment>>,
    cache: EnclosureCache,
}
struct EnclosureCache {
    entries: BTreeMap<(usize, usize, u64, u64), [I; 2]>,
    capacity: usize,
}
impl<'a> Prepared<'a> {
    pub fn new<F: Borrow<Fragment>>(loops: &'a [Vec<F>], capacity: usize) -> Result<Self> {
        if capacity > 100000 || loops.len() > 16 || loops.iter().map(Vec::len).sum::<usize>() > 256
        {
            return Err(Error::new(
                "BREP_SOURCE_WINDING_RESOURCE",
                "Bound prepared contour inventory and cache size",
            ));
        }
        Ok(Self {
            loops: loops
                .iter()
                .map(|wire| wire.iter().map(Borrow::borrow).collect())
                .collect(),
            cache: EnclosureCache {
                entries: BTreeMap::new(),
                capacity,
            },
        })
    }
    pub fn from_references(loops: Vec<Vec<&'a Fragment>>, capacity: usize) -> Result<Self> {
        if capacity > 100000 || loops.len() > 16 || loops.iter().map(Vec::len).sum::<usize>() > 256
        {
            return Err(Error::new(
                "BREP_SOURCE_WINDING_RESOURCE",
                "Bound prepared contour inventory and cache size",
            ));
        }
        Ok(Self {
            loops,
            cache: EnclosureCache {
                entries: BTreeMap::new(),
                capacity,
            },
        })
    }
    pub fn classify_triangle(
        &mut self,
        triangle: [[f64; 2]; 3],
        tolerance: f64,
        cells: usize,
    ) -> Result<Report> {
        let rectangle = triangle_rectangle(triangle)?;
        classify_query(
            &self.loops,
            rectangle,
            Some(triangle),
            tolerance,
            cells,
            Some(&mut self.cache),
        )
    }
}
fn hull(a: [I; 2], b: [I; 2]) -> [I; 2] {
    std::array::from_fn(|i| I {
        lo: a[i].lo.min(b[i].lo),
        hi: a[i].hi.max(b[i].hi),
    })
}
fn representative(a: [I; 2]) -> [f64; 2] {
    a.map(|x| (x.lo * 0.5 + x.hi * 0.5).clamp(x.lo, x.hi))
}
fn fixed(c: &Curve, t: f64, out: &mut Report) -> Result<[I; 2]> {
    out.endpoint_queries += 1;
    let d = c.domain();
    let n = c.control_points.len();
    if t == d[0] && c.knots[..=c.degree].iter().all(|&k| k == t) {
        return Ok([
            I::point(c.control_points[0][0]),
            I::point(c.control_points[0][1]),
        ]);
    }
    if t == d[1] && c.knots[n..].iter().all(|&k| k == t) {
        return Ok([
            I::point(c.control_points[n - 1][0]),
            I::point(c.control_points[n - 1][1]),
        ]);
    }
    let p = interval_eval::evaluate_interval(c, I::point(t))?;
    Ok([p[0], p[1]])
}
fn endpoint(c: &Curve, end: &Endpoint, out: &mut Report) -> Result<[I; 2]> {
    match end {
        Endpoint::Carrier { point, side } => {
            out.endpoint_queries += 1;
            let b = point.uv_bounds()[*side];
            Ok([I::new(b[0][0], b[0][1])?, I::new(b[1][0], b[1][1])?])
        }
        Endpoint::Parameter(t) => fixed(c, *t, out),
        Endpoint::Crossing { point, .. } => {
            out.endpoint_queries += 1;
            let b = point.uv_box();
            Ok([I::new(b[0][0], b[0][1])?, I::new(b[1][0], b[1][1])?])
        }
    }
}
fn separated(a: [I; 2], b: [I; 2]) -> bool {
    (0..2).any(|i| a[i].hi < b[i].lo || b[i].hi < a[i].lo)
}
// The source subarc and its witness chord share this convex box. A strict
// separating axis therefore proves neither intersects the padded triangle.
fn separated_triangle(bounds: [I; 2], triangle: [[f64; 2]; 3], tolerance: f64) -> Result<bool> {
    for i in 0..3 {
        let a = triangle[i];
        let b = triangle[(i + 1) % 3];
        let normal = [
            I::point(a[1]).sub(I::point(b[1]))?,
            I::point(b[0]).sub(I::point(a[0]))?,
        ];
        let projection = |p: [I; 2]| normal[0].mul(p[0])?.add(normal[1].mul(p[1])?);
        let curve = projection(bounds)?;
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for vertex in triangle {
            let v = projection(vertex.map(I::point))?;
            lo = lo.min(v.lo);
            hi = hi.max(v.hi);
        }
        let width = I::point(normal[0].lo.abs().max(normal[0].hi.abs()))
            .add(I::point(normal[1].lo.abs().max(normal[1].hi.abs())))?
            .mul(I::point(tolerance))?;
        let padded = I::new(lo, hi)?.add(I::new(-width.hi, width.hi)?)?;
        if curve.hi < padded.lo || padded.hi < curve.lo {
            return Ok(true);
        }
    }
    Ok(false)
}
fn chord(a: [I; 2], b: [I; 2], bounds: [I; 2], q: [f64; 2]) -> i32 {
    if bounds[0].lo <= q[0] {
        return 0;
    }
    let a = representative(a);
    let b = representative(b);
    if a[1] <= q[1] && b[1] > q[1] {
        1
    } else if b[1] <= q[1] && a[1] > q[1] {
        -1
    } else {
        0
    }
}
fn image(
    c: &Curve,
    d: [f64; 2],
    a: [I; 2],
    b: [I; 2],
    address: (usize, usize),
    cache: &mut Option<&mut EnclosureCache>,
    out: &mut Report,
) -> Result<[I; 2]> {
    let key = (address.0, address.1, d[0].to_bits(), d[1].to_bits());
    let cached = cache
        .as_ref()
        .and_then(|cache| cache.entries.get(&key))
        .copied();
    let bounds = if let Some(bounds) = cached {
        out.enclosure_cache_hits += 1;
        bounds
    } else {
        let p = interval_eval::evaluate_interval(c, I::new(d[0], d[1])?)?;
        out.enclosure_queries += 1;
        let bounds = [p[0], p[1]];
        if let Some(cache) = cache.as_mut() {
            if cache.entries.len() < cache.capacity {
                cache.entries.insert(key, bounds);
            }
        }
        bounds
    };
    // Root endpoint caps are query-independent but remain explicit contributions.
    Ok(hull(hull(bounds, a), b))
}
/// Classify an entire query rectangle, inflated by the given UV tolerance.
/// Every actual subarc and witness chord must occupy one convex enclosure that
/// excludes that rectangle. Shared source endpoints preserve closed homotopy.
pub fn classify<F: Borrow<Fragment>>(
    loops: &[Vec<F>],
    rectangle: [[f64; 2]; 2],
    tolerance_uv: f64,
    max_cells: usize,
) -> Result<Report> {
    classify_query(loops, rectangle, None, tolerance_uv, max_cells, None)
}
/// Classify the entire triangle, including a UV tolerance band. Source curves
/// remain exact; separating axes only exclude their enclosing convex boxes.
pub fn classify_triangle<F: Borrow<Fragment>>(
    loops: &[Vec<F>],
    triangle: [[f64; 2]; 3],
    tolerance_uv: f64,
    max_cells: usize,
) -> Result<Report> {
    let rectangle = triangle_rectangle(triangle)?;
    classify_query(
        loops,
        rectangle,
        Some(triangle),
        tolerance_uv,
        max_cells,
        None,
    )
}
fn triangle_rectangle(triangle: [[f64; 2]; 3]) -> Result<[[f64; 2]; 2]> {
    if !triangle.iter().flatten().all(|v| v.is_finite()) {
        return Err(Error::new(
            "BREP_SOURCE_WINDING_INPUT",
            "Triangle UV coordinates must be finite",
        ));
    }
    Ok(std::array::from_fn(|k| {
        [
            triangle.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min),
            triangle
                .iter()
                .map(|p| p[k])
                .fold(f64::NEG_INFINITY, f64::max),
        ]
    }))
}
fn classify_query<F: Borrow<Fragment>>(
    loops: &[Vec<F>],
    rectangle: [[f64; 2]; 2],
    triangle: Option<[[f64; 2]; 3]>,
    tolerance_uv: f64,
    max_cells: usize,
    mut cache: Option<&mut EnclosureCache>,
) -> Result<Report> {
    if loops.is_empty()
        || loops.len() > 16
        || loops.iter().map(Vec::len).sum::<usize>() > 256
        || !rectangle
            .iter()
            .all(|x| x.iter().all(|x| x.is_finite()) && x[0] <= x[1])
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || !(1..=100000).contains(&max_cells)
    {
        return Err(Error::new(
            "BREP_SOURCE_WINDING_INPUT",
            "Choose bounded closed source contours, a finite rectangle and positive UV tolerance",
        ));
    }
    let mut out = Report {
        location: Location::Unresolved,
        winding: None,
        cells: 0,
        endpoint_queries: 0,
        reason: "source-contour-joins-unproven",
        uncertain: None,
        rectangle_proven: false,
        enclosure_queries: 0,
        enclosure_cache_hits: 0,
    };
    if !loops.iter().all(|wire| {
        !wire.is_empty()
            && (0..wire.len()).all(|i| wire[i].borrow().joins(wire[(i + 1) % wire.len()].borrow()))
    }) {
        return Ok(out);
    }
    for f in loops.iter().flatten() {
        let f = f.borrow();
        if f.curve().degree > 25 || f.curve().control_points.len() > 256 {
            return Err(Error::new(
                "BREP_SOURCE_WINDING_RESOURCE",
                "Source winding needs degrees at most 25 and at most 256 controls",
            ));
        }
    }
    let query = [
        I::new(
            (rectangle[0][0] - tolerance_uv).next_down(),
            (rectangle[0][1] + tolerance_uv).next_up(),
        )?,
        I::new(
            (rectangle[1][0] - tolerance_uv).next_down(),
            (rectangle[1][1] + tolerance_uv).next_up(),
        )?,
    ];
    let q = triangle.map_or_else(
        || rectangle.map(|x| x[0] * 0.5 + x[1] * 0.5),
        |t| std::array::from_fn(|k| t[0][k] / 3. + t[1][k] / 3. + t[2][k] / 3.),
    );
    if let Some(t) = triangle {
        let mut positive = true;
        let mut negative = true;
        for i in 0..3 {
            let a = t[i];
            let b = t[(i + 1) % 3];
            let sub = |x, y| I::point(x).sub(I::point(y));
            let determinant = sub(b[0], a[0])?
                .mul(sub(q[1], a[1])?)?
                .sub(sub(b[1], a[1])?.mul(sub(q[0], a[0])?)?)?;
            positive &= determinant.lo > 0.;
            negative &= determinant.hi < 0.;
        }
        if !positive && !negative {
            out.reason = "source-triangle-reference-unproven";
            return Ok(out);
        }
    }
    let used_triangle = std::cell::Cell::new(false);
    let excludes = |bounds| -> Result<bool> {
        if separated(bounds, query) {
            return Ok(true);
        }
        match triangle {
            // The existing ray witness also requires its reference point to
            // be outside the enclosure's axis-aligned box. Otherwise refine
            // before deciding its ray crossing contribution.
            Some(t) if separated(bounds, q.map(I::point)) => {
                let proved = separated_triangle(bounds, t, tolerance_uv)?;
                if proved {
                    used_triangle.set(true);
                }
                Ok(proved)
            }
            Some(_) => Ok(false),
            None => Ok(false),
        }
    };
    let mut winding = 0;
    for (li, wire) in loops.iter().enumerate() {
        // Reference representatives need not coincide numerically even when
        // the actual source point is identical. Explicit zero-source join
        // homotopies close the witness polygon without relying on cache equality.
        for i in 0..wire.len() {
            out.uncertain = Some((li, i));
            if out.cells == max_cells {
                out.reason = "source-winding-work-limit";
                return Ok(out);
            }
            out.cells += 1;
            let a = endpoint(
                wire[i].borrow().curve(),
                &wire[i].borrow().endpoints()[1],
                &mut out,
            )?;
            let next = wire[(i + 1) % wire.len()].borrow();
            let b = endpoint(next.curve(), &next.endpoints()[0], &mut out)?;
            let bounds = hull(a, b);
            if !excludes(bounds)? {
                out.reason = "source-join-band";
                return Ok(out);
            }
            winding += chord(a, b, bounds, q);
        }
        for (fi, fragment) in wire.iter().enumerate() {
            let fragment = fragment.borrow();
            out.uncertain = Some((li, fi));
            let c = fragment.curve();
            let e = fragment.parameter_bounds();
            let rev = fragment.reversed();
            let anchors = [e[0][usize::from(!rev)], e[1][usize::from(rev)]];
            let ends = [
                endpoint(c, &fragment.endpoints()[0], &mut out)?,
                endpoint(c, &fragment.endpoints()[1], &mut out)?,
            ];
            let stations = [
                fixed(c, anchors[0], &mut out)?,
                fixed(c, anchors[1], &mut out)?,
            ];
            // Root uncertainty caps are indivisible without new root refinement.
            // Never replace them by a midpoint parameter or drop their contribution.
            for i in 0..2 {
                if matches!(fragment.endpoints()[i], Endpoint::Crossing { .. }) {
                    if out.cells == max_cells {
                        out.reason = "source-winding-work-limit";
                        return Ok(out);
                    }
                    out.cells += 1;
                    let (a, b) = if i == 0 {
                        (ends[0], stations[0])
                    } else {
                        (stations[1], ends[1])
                    };
                    let bounds = image(c, e[i], a, b, (li, fi), &mut cache, &mut out)?;
                    if !excludes(bounds)? {
                        out.reason = "source-root-cap-band";
                        return Ok(out);
                    }
                    winding += chord(a, b, bounds, q);
                }
            }
            let d = [anchors[0].min(anchors[1]), anchors[0].max(anchors[1])];
            let mut cuts = vec![d[0]];
            cuts.extend(c.knots.iter().copied().filter(|&k| k > d[0] && k < d[1]));
            cuts.push(d[1]);
            cuts.sort_by(f64::total_cmp);
            cuts.dedup();
            for pair in cuts.windows(2) {
                let mut stack = vec![([pair[0], pair[1]], 0usize)];
                while let Some((d, depth)) = stack.pop() {
                    if out.cells == max_cells {
                        out.reason = "source-winding-work-limit";
                        return Ok(out);
                    }
                    out.cells += 1;
                    let a = fixed(c, d[0], &mut out)?;
                    let b = fixed(c, d[1], &mut out)?;
                    // These stations lie inside the owned fragment's core.
                    // A complete point enclosure contained in the padded query
                    // proves this boundary cannot be excluded from that band.
                    // Keep the query unresolved; never promote this to winding
                    // or infer triangle containment from its bounding box.
                    if triangle.is_none() && [a,b].into_iter().any(|point|
                        (0..2).all(|k|point[k].lo>=query[k].lo && point[k].hi<=query[k].hi)) {
                        out.reason="source-boundary-point-band";
                        return Ok(out);
                    }
                    let bounds = image(c, d, a, b, (li, fi), &mut cache, &mut out)?;
                    if excludes(bounds)? {
                        winding += chord(a, b, bounds, q) * if rev { -1 } else { 1 };
                        continue;
                    }
                    let mid = d[0] * 0.5 + d[1] * 0.5;
                    if depth == 32 || mid <= d[0] || mid >= d[1] {
                        out.reason = "source-winding-boundary-band";
                        return Ok(out);
                    }
                    stack.push(([mid, d[1]], depth + 1));
                    stack.push(([d[0], mid], depth + 1));
                }
            }
        }
    }
    out.uncertain = None;
    out.winding = Some(winding);
    out.location = if winding == 0 {
        Location::Outside
    } else {
        Location::Inside
    };
    out.rectangle_proven = !used_triangle.get();
    out.reason = "source-winding-homotopy-proven";
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nurbs_core::surface::Surface;
    fn surface() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1., 1.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn rational_contour_winding_uses_original_arcs_and_refuses_boundary_band() {
        let s = surface();
        let arc = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
            weights: vec![1., 0.5f64.sqrt(), 1.],
            periodic: false,
        };
        let curves = vec![
            arc,
            Curve::from_polyline(vec![vec![0., 1.], vec![0., 0.]]).unwrap(),
            Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap(),
        ];
        let loops = vec![
            curves
                .iter()
                .map(|c| {
                    Fragment::new(&s, c, Endpoint::Parameter(0.), Endpoint::Parameter(1.)).unwrap()
                })
                .collect::<Vec<_>>(),
        ];
        // The exact source arc crosses this rectangle away from its ends.
        // A contained station must stop before exhausting subdivision work.
        let crossing=classify(&loops,[[0.6,0.8],[0.6,0.8]],1e-8,10000).unwrap();
        assert_eq!(crossing.location,Location::Unresolved);
        assert_eq!(crossing.reason,"source-boundary-point-band");
        assert!(crossing.cells<32);
        assert_eq!(crossing.winding,None);
        assert!(!crossing.rectangle_proven);
        let outside=classify(&loops,[[0.8,0.9],[0.8,0.9]],1e-8,10000).unwrap();
        assert_eq!(outside.location,Location::Outside);
        assert!(outside.rectangle_proven);
        let borrowed = loops
            .iter()
            .map(|wire| wire.iter().collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let same = |a: Report, b: Report| {
            assert_eq!(
                (
                    a.location,
                    a.winding,
                    a.cells,
                    a.endpoint_queries,
                    a.reason,
                    a.uncertain,
                    a.rectangle_proven
                ),
                (
                    b.location,
                    b.winding,
                    b.cells,
                    b.endpoint_queries,
                    b.reason,
                    b.uncertain,
                    b.rectangle_proven
                )
            );
        };
        let triangle = [[0.1, 0.1], [0.85, 0.1], [0.1, 0.85]];
        let mut prepared = Prepared::new(&loops, 100000).unwrap();
        let first = prepared.classify_triangle(triangle, 1e-8, 10000).unwrap();
        assert!(first.enclosure_queries > 0);
        let repeated = prepared.classify_triangle(triangle, 1e-8, 10000).unwrap();
        assert_eq!(repeated.enclosure_queries, 0);
        assert!(repeated.enclosure_cache_hits > 0);
        same(first, repeated);
        for capacity in [0, 1, 8, 100000] {
            let mut prepared = Prepared::new(&loops, capacity).unwrap();
            for triangle in [triangle, [[0.7, 0.9], [0.9, 0.7], [0.9, 0.9]], triangle] {
                same(
                    classify_triangle(&loops, triangle, 1e-8, 10000).unwrap(),
                    prepared.classify_triangle(triangle, 1e-8, 10000).unwrap(),
                );
                assert!(prepared.cache.entries.len() <= capacity);
            }
        }
        assert!(Prepared::new(&loops, 100001).is_err());
        for budget in [1, 1000, 10000] {
            for rectangle in [
                [[0.2, 0.3], [0.2, 0.3]],
                [[0.9, 0.95], [0.9, 0.95]],
                [[0., 0.], [0.3, 0.3]],
            ] {
                same(
                    classify(&loops, rectangle, 1e-8, budget).unwrap(),
                    classify(&borrowed, rectangle, 1e-8, budget).unwrap(),
                );
            }
            for triangle in [
                [[0.1, 0.1], [0.85, 0.1], [0.1, 0.85]],
                [[0.7, 0.9], [0.9, 0.7], [0.9, 0.9]],
                [[0., 0.], [0.2, 0.], [0., 0.2]],
            ] {
                same(
                    classify_triangle(&loops, triangle, 1e-8, budget).unwrap(),
                    classify_triangle(&borrowed, triangle, 1e-8, budget).unwrap(),
                );
            }
        }
        let r = classify(&loops, [[0.2, 0.3], [0.2, 0.3]], 1e-8, 10000).unwrap();
        assert_eq!(r.location, Location::Inside);
        assert_eq!(r.winding, Some(1));
        assert!(r.rectangle_proven);
        assert!(
            classify_triangle(&loops, [[0.2, 0.2], [0.3, 0.2], [0.2, 0.3]], 1e-8, 10000)
                .unwrap()
                .rectangle_proven
        );
        let inside = [[0.1, 0.1], [0.85, 0.1], [0.1, 0.85]];
        assert_eq!(
            classify(&loops, [[0.1, 0.85], [0.1, 0.85]], 1e-8, 10000)
                .unwrap()
                .location,
            Location::Unresolved
        );
        assert_eq!(
            classify_triangle(&loops, inside, 1e-8, 10000)
                .unwrap()
                .location,
            Location::Inside
        );
        assert!(
            !classify_triangle(&loops, inside, 1e-8, 10000)
                .unwrap()
                .rectangle_proven
        );
        let outside = [[0.7, 0.9], [0.9, 0.7], [0.9, 0.9]];
        assert_eq!(
            classify_triangle(&loops, outside, 1e-8, 10000)
                .unwrap()
                .location,
            Location::Outside
        );
        assert_eq!(
            classify_triangle(&loops, [[0., 0.], [0.2, 0.], [0., 0.2]], 1e-8, 10000)
                .unwrap()
                .location,
            Location::Unresolved
        );

        let r = classify(&loops, [[0.9, 0.95], [0.9, 0.95]], 1e-8, 10000).unwrap();
        assert_eq!(r.location, Location::Outside);
        assert!(r.rectangle_proven);
        // The certified rectangle is wholly beyond the rational quarter arc;
        // all contained rectangles must have the same outside classification.
        for rectangle in [
            [[0.91, 0.94], [0.91, 0.94]],
            [[0.9, 0.95], [0.925, 0.925]],
            [[0.95, 0.95], [0.95, 0.95]],
        ] {
            let nested = classify(&loops, rectangle, 1e-8, 10000).unwrap();
            assert_eq!(nested.location, Location::Outside);
            assert!(nested.rectangle_proven);
        }
        assert_eq!(
            classify(&loops, [[0., 0.], [0.3, 0.3]], 1e-8, 1000)
                .unwrap()
                .location,
            Location::Unresolved
        );
        assert_eq!(
            classify(&loops, [[0.2, 0.3], [0.2, 0.3]], 1e-8, 1)
                .unwrap()
                .location,
            Location::Unresolved
        );
    }
}
