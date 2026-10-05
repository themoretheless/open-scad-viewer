//! Winding of unchanged source curves with root-valued endpoints. Chords are
//! winding witnesses only; they never become replacement geometry.
use crate::source_boundary_fragment::{Endpoint, Fragment};
use nurbs_core::{
    curve::Curve,
    interval_eval::{self, Interval as I},
    trim_domain::Location,
    Error, Result,
};
pub struct Report {
    pub location: Location,
    pub winding: Option<i32>,
    pub cells: usize,
    pub endpoint_queries: usize,
    pub reason: &'static str,
    pub uncertain: Option<(usize, usize)>,
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
fn image(c: &Curve, d: [f64; 2], a: [I; 2], b: [I; 2]) -> Result<[I; 2]> {
    let p = interval_eval::evaluate_interval(c, I::new(d[0], d[1])?)?;
    Ok(hull(hull([p[0], p[1]], a), b))
}
/// Classify an entire query rectangle, inflated by the given UV tolerance.
/// Every actual subarc and witness chord must occupy one convex enclosure that
/// excludes that rectangle. Shared source endpoints preserve closed homotopy.
pub fn classify(
    loops: &[Vec<Fragment>],
    rectangle: [[f64; 2]; 2],
    tolerance_uv: f64,
    max_cells: usize,
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
    };
    if !loops.iter().all(|wire| {
        !wire.is_empty() && (0..wire.len()).all(|i| wire[i].joins(&wire[(i + 1) % wire.len()]))
    }) {
        return Ok(out);
    }
    for f in loops.iter().flatten() {
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
    let q = rectangle.map(|x| x[0] * 0.5 + x[1] * 0.5);
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
            let a = endpoint(wire[i].curve(), &wire[i].endpoints()[1], &mut out)?;
            let next = &wire[(i + 1) % wire.len()];
            let b = endpoint(next.curve(), &next.endpoints()[0], &mut out)?;
            let bounds = hull(a, b);
            if !separated(bounds, query) {
                out.reason = "source-join-band";
                return Ok(out);
            }
            winding += chord(a, b, bounds, q);
        }
        for (fi, fragment) in wire.iter().enumerate() {
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
                    let bounds = image(c, e[i], a, b)?;
                    if !separated(bounds, query) {
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
                    let bounds = image(c, d, a, b)?;
                    if separated(bounds, query) {
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
        let loops = vec![curves
            .iter()
            .map(|c| {
                Fragment::new(&s, c, Endpoint::Parameter(0.), Endpoint::Parameter(1.)).unwrap()
            })
            .collect::<Vec<_>>()];
        let r = classify(&loops, [[0.2, 0.3], [0.2, 0.3]], 1e-8, 10000).unwrap();
        assert_eq!(r.location, Location::Inside);
        assert_eq!(r.winding, Some(1));
        let r = classify(&loops, [[0.9, 0.95], [0.9, 0.95]], 1e-8, 10000).unwrap();
        assert_eq!(r.location, Location::Outside);
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
