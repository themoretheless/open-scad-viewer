//! Whole-source seam tangent-plane qualification. Positional ownership comes
//! from a privately admitted SharedEdge; angular bounds cover original charts.
use crate::source_shared_edge::SharedEdge;
use nurbs_core::{interval_eval::Interval as I, Error, Result};
pub struct Limits {
    pub max_sine_squared: f64,
    pub cells: usize,
    pub curve_spans: usize,
    pub normal_spans: usize,
}
pub struct Seam {
    edge: SharedEdge,
    max_sine_squared: f64,
    bound: [f64; 2],
}
impl Seam {
    pub fn edge(&self) -> &SharedEdge {
        &self.edge
    }
    pub fn tolerance(&self) -> f64 {
        self.max_sine_squared
    }
    pub fn sine_squared_bounds(&self) -> [f64; 2] {
        self.bound
    }
}
pub struct Report {
    pub seam: Option<Seam>,
    pub cells: usize,
    pub curve_spans: usize,
    pub normal_spans: usize,
    pub accepted_cells: usize,
    pub uncertain_canonical: Option<[f64; 2]>,
    pub reason: &'static str,
}
fn ranges(edge: &SharedEdge) -> [[[f64; 2]; 2]; 2] {
    edge.ranges().unwrap_or_else(|| {
        std::array::from_fn(|side| {
            // SharedEdge stores traversal orientation, while the map is for the
            // forward original pcurve regardless of fragment traversal.
            let r = edge.reversed()[side] ^ edge.uses()[side].reversed();
            if r {
                [[1., 1.], [0., 1.]]
            } else {
                [[0., 1.], [1., 1.]]
            }
        })
    })
}
fn map(edge: &SharedEdge, side: usize, parameter: I) -> Result<I> {
    let d = edge.uses()[side].curve().domain();
    let range = ranges(edge)[side];
    let first = I::point(range[0][0]).div(I::point(range[0][1]))?;
    let last = I::point(range[1][0]).div(I::point(range[1][1]))?;
    let normalized = parameter
        .sub(I::point(d[0]))?
        .div(I::point(d[1]).sub(I::point(d[0]))?)?;
    first.add(normalized.mul(last.sub(first)?)?)
}
fn inverse(edge: &SharedEdge, side: usize, canonical: I) -> Result<I> {
    let d = edge.uses()[side].curve().domain();
    let range = ranges(edge)[side];
    let first = I::point(range[0][0]).div(I::point(range[0][1]))?;
    let last = I::point(range[1][0]).div(I::point(range[1][1]))?;
    let normalized = canonical.sub(first)?.div_signed(last.sub(first)?)?;
    I::point(d[0])
        .add(normalized.mul(I::point(d[1]).sub(I::point(d[0]))?)?)?
        .intersect(d[0], d[1])
}
/// Every accepted interval bounds all corresponding normal pairs, including
/// endpoint root enclosures and both incident original knot sides. Antiparallel
/// normals are compatible tangent planes; orientation is checked by the shell.
/// This certifies tolerance-based G1 only, not curvature/G2 or fillet radius.
pub fn qualify(edge: &SharedEdge, limits: Limits) -> Result<Report> {
    if !limits.max_sine_squared.is_finite()
        || !(0. ..1.).contains(&limits.max_sine_squared)
        || !(1..=100000).contains(&limits.cells)
        || !(1..=100000).contains(&limits.curve_spans)
        || !(1..=100000).contains(&limits.normal_spans)
    {
        return Err(Error::new(
            "BREP_SOURCE_SEAM_TANGENCY",
            "Choose finite angular tolerance and bounded seam work",
        ));
    }
    let ends = edge.uses()[0].parameter_bounds();
    let lo = ends.iter().map(|r| r[0]).fold(f64::INFINITY, f64::min);
    let hi = ends.iter().map(|r| r[1]).fold(f64::NEG_INFINITY, f64::max);
    let coverage = map(edge, 0, I::new(lo, hi)?)?.intersect(0., 1.)?;
    let mut queue = vec![[coverage.lo, coverage.hi]];
    // Only literal full-carrier endpoints have exact canonical representatives.
    // Root enclosures and affine ranges retain whole-interval qualification.
    if edge.ranges().is_none()
        && edge.uses().iter().all(|f| f.parameter_bounds().iter().all(|r| r[0] == r[1]))
    {
        queue.push([coverage.hi, coverage.hi]);
        queue.push([coverage.lo, coverage.lo]);
    }
    let mut out = Report {
        seam: None,
        cells: 0,
        curve_spans: 0,
        normal_spans: 0,
        accepted_cells: 0,
        uncertain_canonical: None,
        reason: "source-seam-work-limit",
    };
    let mut bound = [1_f64, 0_f64];
    while let Some(q) = queue.pop() {
        out.uncertain_canonical = Some(q);
        if out.cells == limits.cells || out.normal_spans == limits.normal_spans {
            return Ok(out);
        }
        out.cells += 1;
        let mut rectangles = [[[0.; 2]; 2]; 2];
        for side in 0..2 {
            let fragment = &edge.uses()[side];
            let c = fragment.curve();
            let t = inverse(edge, side, I::new(q[0], q[1])?)?;
            let count = (c.degree..c.control_points.len())
                .filter(|&i| {
                    c.knots[i] < c.knots[i + 1] && c.knots[i] <= t.hi && c.knots[i + 1] >= t.lo
                })
                .count();
            if count > limits.curve_spans - out.curve_spans {
                return Ok(out);
            }
            out.curve_spans += count;
            let uv = nurbs_core::interval_eval::evaluate_interval(c, t)?;
            let s = fragment.surface();
            let chart = [
                [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
                [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
            ];
            for axis in 0..2 {
                // Fragment admission proves the actual source path lies in
                // this chart; intersecting an outward enclosure is sound.
                let b = uv[axis].intersect(chart[axis][0], chart[axis][1])?;
                rectangles[side][axis] = [b.lo, b.hi];
            }
        }
        let r = nurbs_core::normal_alignment::inspect_pair(
            [edge.uses()[0].surface(), edge.uses()[1].surface()],
            rectangles,
            limits.max_sine_squared,
            limits.normal_spans - out.normal_spans,
        )?;
        out.normal_spans += r.spans;
        if r.aligned == Some(true) {
            let b = r.sine_squared_interval.unwrap();
            bound[0] = bound[0].min(b[0]);
            bound[1] = bound[1].max(b[1]);
            out.accepted_cells += 1;
            continue;
        }
        if r.aligned == Some(false) {
            out.reason = "source-seam-angular-envelope-oblique";
            return Ok(out);
        }
        if r.reason == "span-limit" {
            return Ok(out);
        }
        if q[0] == q[1] {
            out.reason = "source-seam-endpoint-normal-unresolved";
            return Ok(out);
        }
        let mid = q[0] * 0.5 + q[1] * 0.5;
        if !(q[0] < mid && mid < q[1]) {
            out.reason = "source-seam-resolution-limit";
            return Ok(out);
        }
        queue.push([mid, q[1]]);
        queue.push([q[0], mid]);
    }
    out.seam = Some(Seam {
        edge: edge.clone(),
        max_sine_squared: limits.max_sine_squared,
        bound,
    });
    out.uncertain_canonical = None;
    out.reason = "source-seam-tangent-planes-qualified";
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_boundary_fragment::{Endpoint, Fragment, Role};
    use nurbs_core::{curve::Curve, surface::Surface};
    fn limits() -> Limits {
        Limits {
            max_sine_squared: 1e-3,
            cells: 10000,
            curve_spans: 20000,
            normal_spans: 20000,
        }
    }
    fn patch(crease: bool) -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: (0..3)
                .map(|u| {
                    (0..3)
                        .map(|v| {
                            vec![
                                u as f64 / 2.,
                                v as f64 / 2.,
                                f64::from(u == 2)
                                    + f64::from(v == 2)
                                    + if crease { v as f64 / 2. } else { 0. },
                            ]
                        })
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }
    fn shared(crease: bool) -> SharedEdge {
        let mut a = patch(false);
        for row in &mut a.control_points {
            for (v, p) in row.iter_mut().enumerate() {
                p[2] -= f64::from(v == 2);
            }
        }
        let b = patch(crease);
        let pc = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        let world = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![0.5, 0., 0.], vec![1., 0., 1.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let fa = Fragment::new(&a, &pc, Endpoint::Parameter(0.), Endpoint::Parameter(1.)).unwrap();
        let fb = Fragment::new(&b, &pc, Endpoint::Parameter(1.), Endpoint::Parameter(0.)).unwrap();
        crate::source_shared_edge::qualify(&world, [&fa, &fb], [false, false], 100_000_000)
            .unwrap()
            .edge
            .unwrap()
    }
    #[test]
    fn rotating_tangent_planes_are_certified_over_the_complete_curved_seam() {
        let edge = shared(false);
        let before = edge.uses()[0].definition();
        let r = qualify(&edge, limits()).unwrap();
        assert!(
            r.seam.is_some(),
            "{} {:?} cells {}",
            r.reason,
            r.uncertain_canonical,
            r.cells
        );
        assert!(r.accepted_cells > 1);
        let seam = r.seam.unwrap();
        assert!(seam.sine_squared_bounds()[1] <= 1e-3);
        assert_eq!(seam.edge().uses()[0].definition(), before);
        let mut short = limits();
        short.cells = 1;
        let r = qualify(&edge, short).unwrap();
        assert!(r.seam.is_none());
        assert!(r.uncertain_canonical.is_some());
        let mut short = limits();
        short.curve_spans = 1;
        let r = qualify(&edge, short).unwrap();
        assert!(r.seam.is_none());
        assert_eq!(r.curve_spans, 1);
        let mut short = limits();
        short.normal_spans = 1;
        let r = qualify(&edge, short).unwrap();
        assert!(r.seam.is_none());
        assert_eq!(r.normal_spans, 1);
    }
    #[test]
    fn positional_coincidence_does_not_hide_a_crease() {
        let r = qualify(&shared(true), limits()).unwrap();
        assert!(r.seam.is_none());
        assert!(r.uncertain_canonical.is_some());
    }
    #[test]
    fn rational_root_ends_keep_their_source_definitions() {
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., -0.5, 0.], vec![0., 0.5, 0.]],
                vec![vec![1., -0.5, 0.], vec![1., 0.5, 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let mut pc = Curve::from_polyline(vec![vec![-0.25, 0.5], vec![1.25, 0.5]]).unwrap();
        pc.weights[1] = 0.75;
        let mut world =
            Curve::from_polyline(vec![vec![-0.25, 0., 0.], vec![1.25, 0., 0.]]).unwrap();
        world.weights = pc.weights.clone();
        let mut ends = Vec::new();
        for (u, selector) in [(0.125, [0.2, 0.4]), (0.875, [0.7, 0.9])] {
            let boundary = Curve::from_polyline(vec![vec![u, 0.], vec![u, 1.]]).unwrap();
            let point = crate::source_contact_point::qualify(
                &s,
                &boundary,
                &pc,
                [[0.4, 0.6], selector],
                16,
            )
            .unwrap()
            .point
            .unwrap();
            ends.push(Endpoint::Crossing {
                point,
                role: Role::Contact,
            });
        }
        let a = Fragment::new(&s, &pc, ends[0].clone(), ends[1].clone()).unwrap();
        let b = Fragment::new(&s, &pc, ends[1].clone(), ends[0].clone()).unwrap();
        let edge =
            crate::source_shared_edge::qualify(&world, [&a, &b], [false, false], 100_000_000)
                .unwrap()
                .edge
                .unwrap();
        let before = [a.definition(), b.definition()];
        let r = qualify(&edge, limits()).unwrap();
        assert!(r.seam.is_some(), "{}", r.reason);
        let seam = r.seam.unwrap();
        assert_eq!(seam.edge().uses()[0].definition(), before[0]);
        assert_eq!(seam.edge().uses()[1].definition(), before[1]);
    }
    #[test]
    fn reversed_original_curve_and_affine_partial_carriers_preserve_correspondence() {
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., -0.5, 0.], vec![0., 0.5, 0.]],
                vec![vec![1., -0.5, 0.], vec![1., 0.5, 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let a = Curve::from_polyline(vec![vec![0., 0.5], vec![1., 0.5]]).unwrap();
        let mut b = Curve::from_polyline(vec![vec![1., 0.5], vec![0., 0.5]]).unwrap();
        b.knots = vec![10., 10., 18., 18.];
        let fa = Fragment::new(&s, &a, Endpoint::Parameter(0.), Endpoint::Parameter(1.)).unwrap();
        let fb = Fragment::new(&s, &b, Endpoint::Parameter(10.), Endpoint::Parameter(18.)).unwrap();
        let world = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
        let full =
            crate::source_shared_edge::qualify(&world, [&fa, &fb], [false, true], 100_000_000)
                .unwrap()
                .edge
                .unwrap();
        assert!(qualify(&full, limits()).unwrap().seam.is_some());
        let carrier = Curve::from_polyline(vec![vec![-1., 0., 0.], vec![2., 0., 0.]]).unwrap();
        let ranges = [[[1., 3.], [2., 3.]], [[2., 3.], [1., 3.]]];
        let partial = crate::source_mapped_edge::qualify(
            &carrier,
            [&fa, &fb],
            ranges,
            [None; 2],
            100_000_000,
            0,
        )
        .unwrap()
        .edge
        .unwrap();
        let report = qualify(&partial, limits()).unwrap();
        assert!(report.seam.is_some(), "{}", report.reason);
        assert_eq!(report.seam.unwrap().edge().ranges(), Some(ranges));
    }
    #[test]
    fn singular_endpoint_is_not_a_regular_tangent_plane() {
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 0., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let pc = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        let world = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
        let a = Fragment::new(&s, &pc, Endpoint::Parameter(0.), Endpoint::Parameter(1.)).unwrap();
        let b = Fragment::new(&s, &pc, Endpoint::Parameter(1.), Endpoint::Parameter(0.)).unwrap();
        let edge =
            crate::source_shared_edge::qualify(&world, [&a, &b], [false, false], 100_000_000)
                .unwrap()
                .edge
                .unwrap();
        let mut budget = limits();
        budget.cells = 32;
        let report = qualify(&edge, budget).unwrap();
        assert!(report.seam.is_none());
        assert_eq!(report.reason, "source-seam-endpoint-normal-unresolved");
        assert_eq!(report.cells, 1);
        let uncertain = report.uncertain_canonical.unwrap();
        assert!(uncertain[0] <= 0. && uncertain[1] >= 0.);
    }
}
