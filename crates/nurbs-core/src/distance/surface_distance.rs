//! Global distance between complete positive-weight tensor-product surfaces.
//! All nonempty knot rectangles and their pairings remain covered until pruned
//! by a proven lower bound. Trimmed B-rep faces need additional domain handling.
use crate::distance::subdivision::{HeapQueue, Queue, Search, Split};
use crate::distance_bounds::{Interval, box_distance};
use crate::{Result, check, resource, surface::Surface};
#[cfg(feature = "codec")]
mod serialization;
mod bezier_restriction;
#[cfg(test)]
mod shared_patch_tests;
pub(crate) fn complete_bernstein_span(degree: usize, knots: &[f64], span: usize) -> bool {
    bezier_restriction::is_complete_span(degree, knots, span)
}
pub(crate) fn restrict_bernstein_controls(controls: &[[Interval;4]], span: [f64;2], domain: [f64;2], weights: [f64;2]) -> Result<Vec<[Interval;4]>> {
    bezier_restriction::restrict(controls, span, domain, weights)
}


fn midpoint(a: f64, b: f64) -> f64 {
    a * 0.5 + b * 0.5
}
fn spans(degree: usize, knots: &[f64], count: usize) -> Vec<usize> {
    (degree..count)
        .filter(|&i| knots[i] < knots[i + 1])
        .collect()
}
/// de Boor in homogeneous interval coordinates, without rounding a new patch's
/// controls first. Inputs stay inside one original knot span.
pub(crate) fn interpolate(
    mut d: Vec<[Interval; 4]>,
    degree: usize,
    knots: &[f64],
    span: usize,
    parameters: &[f64],
    weights: [f64; 2],
) -> Result<[Interval; 4]> {
    for r in 1..=degree {
        for j in (r..=degree).rev() {
            let i = span - degree + j;
            let denominator =
                Interval::point(knots[i + degree - r + 1]).sub(Interval::point(knots[i]))?;
            let alpha = Interval::point(parameters[r - 1])
                .sub(Interval::point(knots[i]))?
                .div(denominator)?
                .intersect(0., 1.)?;
            let beta = Interval::point(1.).sub(alpha)?.intersect(0., 1.)?;
            for axis in 0..4 {
                let a = d[j - 1][axis];
                let b = d[j][axis];
                d[j][axis] = a
                    .mul(beta)?
                    .add(b.mul(alpha)?)?
                    .intersect(a.lo.min(b.lo), a.hi.max(b.hi))?;
            }
            d[j][3] = d[j][3].intersect(weights[0], weights[1])?;
        }
    }
    Ok(d[degree])
}
// One immutable knot/parameter sequence gives identical de Boor coefficients
// for every homogeneous row. Reuse only these coefficients; preserve every
// original control arithmetic operation, intersection and traversal order.
pub(crate) fn interpolation_plan(degree: usize, knots: &[f64], span: usize, parameters: &[f64])
    -> Result<Vec<(usize, Interval, Interval)>> {
    let mut plan = Vec::new();
    for r in 1..=degree {
        for j in (r..=degree).rev() {
            let i = span - degree + j;
            let denominator = Interval::point(knots[i + degree - r + 1])
                .sub(Interval::point(knots[i]))?;
            let alpha = Interval::point(parameters[r - 1]).sub(Interval::point(knots[i]))?
                .div(denominator)?.intersect(0., 1.)?;
            let beta = Interval::point(1.).sub(alpha)?.intersect(0., 1.)?;
            plan.push((j, alpha, beta));
        }
    }
    Ok(plan)
}
fn mul_blend_factor(a: Interval, b: Interval) -> Result<Interval> {
    if b.lo >= 0. {
        let (lo, hi) = if a.lo >= 0. {(a.lo*b.lo,a.hi*b.hi)}
            else if a.hi <= 0. {(a.lo*b.hi,a.hi*b.lo)}
            else {(a.lo*b.hi,a.hi*b.hi)};
        // Keep original signed-zero and overflow/error behavior on the fallback.
        if lo != 0. && hi != 0. && lo.is_finite() && hi.is_finite() {
            return Interval::new(lo.next_down(),hi.next_up());
        }
    }
    a.mul(b)
}
pub(crate) fn interpolate_planned(mut d: Vec<[Interval; 4]>, degree: usize,
    plan: &[(usize, Interval, Interval)], weights: [f64; 2]) -> Result<[Interval; 4]> {
    for &(j, alpha, beta) in plan {
        for axis in 0..4 {
            let a = d[j - 1][axis];
            let b = d[j][axis];
            d[j][axis] = mul_blend_factor(a,beta)?.add(mul_blend_factor(b,alpha)?)?
                .intersect(a.lo.min(b.lo), a.hi.max(b.hi))?;
        }
        d[j][3] = d[j][3].intersect(weights[0], weights[1])?;
    }
    Ok(d[degree])
}
/// Enclose the original rational image of a rectangle inside a knot cell.
fn restricted_controls(
    s: &Surface,
    span: [usize; 2],
    domain: [[f64; 2]; 2],
) -> Result<Vec<Vec<Interval>>> {
    let [iu, iv] = span;
    let origin = &s.control_points[iu - s.degree_u][iv - s.degree_v];
    let mut scale: f64 = 0.;
    for i in iu - s.degree_u..=iu {
        for j in iv - s.degree_v..=iv {
            scale = scale.max(s.weights[i][j]);
        }
    }
    let mut weights = [f64::INFINITY, 0_f64];
    let mut controls = Vec::new();
    let mut hull = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for i in iu - s.degree_u..=iu {
        let mut row = Vec::new();
        for j in iv - s.degree_v..=iv {
            let ratio = s.weights[i][j] / scale;
            let w = if s.weights[i][j] == scale {
                Interval::point(1.)
            } else {
                Interval::new(ratio.next_down().max(0.), ratio.next_up())?
            };
            weights[0] = weights[0].min(w.lo);
            weights[1] = weights[1].max(w.hi);
            let mut p = [Interval::point(0.); 4];
            for axis in 0..3 {
                let x = s.control_points[i][j][axis];
                hull[axis][0] = hull[axis][0].min(x);
                hull[axis][1] = hull[axis][1].max(x);
                p[axis] = Interval::point(x)
                    .sub(Interval::point(origin[axis]))?
                    .mul(w)?;
            }
            p[3] = w;
            row.push(p);
        }
        controls.push(row);
    }
    // A complete clamped original Bernstein span can be restricted with two
    // convex subdivisions instead of separately evaluating every blossom.
    // This changes interval computation only; original surface controls stay intact.
    if bezier_restriction::is_complete_span(s.degree_u, &s.knots_u, iu)
        && bezier_restriction::is_complete_span(s.degree_v, &s.knots_v, iv) {
        let rows = controls.iter().map(|row| bezier_restriction::restrict(row,
            [s.knots_v[iv], s.knots_v[iv+1]], domain[1], weights))
            .collect::<Result<Vec<_>>>()?;
        let mut restricted = Vec::new();
        for j in 0..rows[0].len() {
            let column = rows.iter().map(|row| row[j]).collect::<Vec<_>>();
            for h in bezier_restriction::restrict(&column,
                [s.knots_u[iu], s.knots_u[iu+1]], domain[0], weights)? {
                let mut control = Vec::new();
                for k in 0..3 {
                    control.push(h[k].div(h[3])?.add(Interval::point(origin[k]))?
                        .intersect(hull[k][0], hull[k][1])?);
                }
                restricted.push(control);
            }
        }
        return Ok(restricted);
    }
    // Blossom values at lo^(p-k), hi^k are the restricted Bernstein
    // coefficients of this original span. Keep their homogeneous arithmetic
    // interval-valued; convex hulls of the positive rational coefficients then
    // enclose the entire rectangle, including edges and knot endpoints.
    let parameters = |degree: usize, d: [f64; 2]| -> Vec<Vec<f64>> {
        if d[0] == d[1] {
            return vec![vec![d[0]; degree]];
        }
        (0..=degree)
            .map(|k| {
                (0..degree)
                    .map(|j| if j < degree - k { d[0] } else { d[1] })
                    .collect()
            })
            .collect()
    };
    let us = parameters(s.degree_u, domain[0]);
    let vs = parameters(s.degree_v, domain[1]);
    let u_plans = us.iter().map(|u| interpolation_plan(s.degree_u, &s.knots_u, iu, u))
        .collect::<Result<Vec<_>>>()?;
    let mut restricted = Vec::new();
    for v in vs {
        let v_plan = interpolation_plan(s.degree_v, &s.knots_v, iv, &v)?;
        let rows = controls
            .iter()
            .map(|row| interpolate_planned(row.clone(), s.degree_v, &v_plan, weights))
            .collect::<Result<Vec<_>>>()?;
        for u_plan in &u_plans {
            let h = interpolate_planned(rows.clone(), s.degree_u, u_plan, weights)?;
            let mut control = Vec::new();
            for k in 0..3 {
                let value = h[k]
                    .div(h[3])?
                    .add(Interval::point(origin[k]))?
                    .intersect(hull[k][0], hull[k][1])?;
                control.push(value);
            }
            restricted.push(control);
        }
    }
    Ok(restricted)
}
fn control_bounds(controls: &[Vec<Interval>]) -> Vec<Interval> {
    (0..3)
        .map(|k| Interval {
            lo: controls
                .iter()
                .map(|p| p[k].lo)
                .fold(f64::INFINITY, f64::min),
            hi: controls
                .iter()
                .map(|p| p[k].hi)
                .fold(f64::NEG_INFINITY, f64::max),
        })
        .collect()
}
pub(crate) fn enclosure(
    s: &Surface,
    span: [usize; 2],
    domain: [[f64; 2]; 2],
) -> Result<Vec<Interval>> {
    Ok(control_bounds(&restricted_controls(s, span, domain)?))
}

#[derive(Clone)]
pub(crate) struct Patch {
    pub(crate) span: [usize; 2],
    pub(crate) domain: [[f64; 2]; 2],
    pub(crate) bounds: Vec<Interval>,
    controls: Vec<Vec<Interval>>,
    // Relative span length ensures alternating subdivision even with very
    // different U/V knot scales. Exact constant directions need no splitting.
    original_width: [f64; 2],
    active: [bool; 2],
}
impl Patch {
    pub(crate) fn new(s: &Surface, span: [usize; 2], domain: [[f64; 2]; 2]) -> Result<Self> {
        let [i, j] = span;
        let same = |a: usize, b: usize, c: usize, d: usize| {
            s.control_points[a][b] == s.control_points[c][d] && s.weights[a][b] == s.weights[c][d]
        };
        let active = [
            (i - s.degree_u + 1..=i).any(|u| (j - s.degree_v..=j).any(|v| !same(u, v, u - 1, v))),
            (j - s.degree_v + 1..=j).any(|v| (i - s.degree_u..=i).any(|u| !same(u, v, u, v - 1))),
        ];
        let controls = restricted_controls(s, span, domain)?;
        Ok(Self {
            span,
            domain,
            bounds: control_bounds(&controls),
            controls,
            original_width: [
                s.knots_u[i + 1] - s.knots_u[i],
                s.knots_v[j + 1] - s.knots_v[j],
            ],
            active,
        })
    }
    pub(crate) fn spatial_size(&self) -> f64 {
        self.bounds.iter().map(|b| b.hi - b.lo).fold(0., f64::max)
    }
    pub(crate) fn score(&self, axis: usize) -> f64 {
        if !self.active[axis] {
            return -1.;
        }
        self.parameter_score(axis)
    }
    pub(crate) fn parameter_score(&self, axis: usize) -> f64 {
        let [lo, hi] = self.domain[axis];
        let middle = midpoint(lo, hi);
        if middle <= lo || middle >= hi {
            return -1.;
        }
        (hi - lo) / self.original_width[axis]
    }
}
/// A support direction separates convex rational control hulls more tightly
/// than axis boxes near a curved closest point. Any fixed binary64 direction is
/// valid; outward projection arithmetic and an upper norm bound keep it safe.
pub(crate) fn patch_lower(a: &Patch, b: &Patch) -> Result<f64> {
    let axis_lower = box_distance(&a.bounds, &b.bounds)?.0;
    let origin: Vec<f64> = a.bounds.iter().map(|i| midpoint(i.lo, i.hi)).collect();
    let mut n: Vec<f64> = b
        .bounds
        .iter()
        .enumerate()
        .map(|(k, i)| origin[k] * 0.5 - midpoint(i.lo, i.hi) * 0.5)
        .collect();
    let scale = n.iter().map(|v| v.abs()).fold(0., f64::max);
    if scale == 0. {
        return Ok(axis_lower);
    }
    for v in &mut n {
        *v /= scale;
    }
    let norm_upper = box_distance(
        &n.iter().map(|&v| Interval::point(v)).collect::<Vec<_>>(),
        &vec![Interval::point(0.); 3],
    )?
    .1;
    let project = |patch: &Patch| -> Result<Interval> {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for p in &patch.controls {
            let mut d = Interval::point(0.);
            for k in 0..3 {
                d = d.add(
                    p[k].sub(Interval::point(origin[k]))?
                        .mul(Interval::point(n[k]))?,
                )?;
            }
            lo = lo.min(d.lo);
            hi = hi.max(d.hi);
        }
        Interval::new(lo, hi)
    };
    let gap = box_distance(&[project(a)?], &[project(b)?])?.0;
    Ok(axis_lower.max((gap / norm_upper).next_down().max(0.)))
}
struct Cell {
    parts: [Patch; 2],
}

#[derive(Clone, Debug)]
pub struct SurfaceDistance {
    pub distance_interval_mm: [f64; 2],
    pub parameters: [[f64; 2]; 2],
    pub points: [[f64; 3]; 2],
    pub point_enclosures: [Vec<[f64; 2]>; 2],
    pub converged: bool,
    pub reason: crate::DistanceStopReason,
    pub cells: usize,
    pub max_cells: usize,
    pub tolerance_mm: f64,
}

pub(crate) struct Witness {
    pub(crate) parameters: [[f64; 2]; 2],
    pub(crate) points: [[f64; 3]; 2],
    pub(crate) bounds: [Vec<Interval>; 2],
    pub(crate) upper: f64,
}
pub(crate) fn at(s: &Surface, mut uv: [f64; 2]) -> Result<([f64; 2], [f64; 3], Vec<Interval>)> {
    let counts = [s.control_points.len(), s.control_points[0].len()];
    let knots = [&s.knots_u, &s.knots_v];
    let degree = [s.degree_u, s.degree_v];
    let periodic = [s.periodic_u, s.periodic_v];
    let mut span = [0; 2];
    for k in 0..2 {
        if periodic[k] && uv[k] == knots[k][counts[k]] {
            uv[k] = knots[k][degree[k]];
        }
        span[k] = (degree[k]..counts[k])
            .rev()
            .find(|&i| knots[k][i] < knots[k][i + 1] && knots[k][i] <= uv[k])
            .unwrap();
    }
    Ok((
        uv,
        s.evaluate_validated(uv[0], uv[1])?.point,
        enclosure(s, span, [[uv[0]; 2], [uv[1]; 2]])?,
    ))
}
pub(crate) fn consider(
    a: &Surface,
    b: &Surface,
    parameters: [[f64; 2]; 2],
    best: &mut Option<Witness>,
) -> Result<()> {
    let (ua, pa, ba) = at(a, parameters[0])?;
    let (ub, pb, bb) = at(b, parameters[1])?;
    let upper = box_distance(&ba, &bb)?.1;
    if best.as_ref().is_none_or(|w| upper < w.upper) {
        *best = Some(Witness {
            parameters: [ua, ub],
            points: [pa, pb],
            bounds: [ba, bb],
            upper,
        });
    }
    Ok(())
}
pub(crate) fn patches(s: &Surface) -> Result<Vec<Patch>> {
    let mut result = Vec::new();
    for u in spans(s.degree_u, &s.knots_u, s.control_points.len()) {
        for v in spans(s.degree_v, &s.knots_v, s.control_points[0].len()) {
            result.push(Patch::new(
                s,
                [u, v],
                [
                    [s.knots_u[u], s.knots_u[u + 1]],
                    [s.knots_v[v], s.knots_v[v + 1]],
                ],
            )?);
        }
    }
    Ok(result)
}
struct SurfaceSearch<'a> {
    a: &'a Surface,
    b: &'a Surface,
    best: Option<Witness>,
}
impl Search for SurfaceSearch<'_> {
    type Cell = Cell;
    fn upper(&self) -> Option<f64> {
        self.best.as_ref().map(|w| w.upper)
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
        let [lo, hi] = selected.domain[axis];
        let mid = midpoint(lo, hi);
        let mut children = [None, None];
        for (i, range) in [[lo, mid], [mid, hi]].into_iter().enumerate() {
            let mut domain = selected.domain;
            domain[axis] = range;
            let child = Patch::new([self.a, self.b][side], selected.span, domain)?;
            let mut parts = cell.parts.clone();
            parts[side] = child;
            let uv = parts
                .each_ref()
                .map(|p| p.domain.map(|[lo, hi]| midpoint(lo, hi)));
            consider(self.a, self.b, uv, &mut self.best)?;
            let lower = patch_lower(&parts[0], &parts[1])?;
            if self.best.as_ref().is_none_or(|w| lower <= w.upper) {
                children[i] = Some((Cell { parts }, lower));
            }
        }
        Ok(Split::Children(children))
    }
}

/// Global interval for the distance between full surface images, including
/// their knot seams and outer boundaries. A work/precision stop stays explicit.
pub fn distance(
    a: &Surface,
    b: &Surface,
    tolerance_mm: f64,
    max_cells: usize,
) -> Result<SurfaceDistance> {
    a.validate()?;
    b.validate()?;
    check(
        tolerance_mm.is_finite() && tolerance_mm > 0.,
        "Surface distance tolerance must be positive and finite",
    )?;
    check(
        (1..=100_000).contains(&max_cells),
        "Surface distance needs 1..100000 cells",
    )?;
    let aa = patches(a)?;
    let bb = patches(b)?;
    if aa.len().saturating_mul(bb.len()) > max_cells {
        return Err(resource(
            "Surface distance initial knot pairs exceed the cell budget",
        ));
    }
    let mut queue = HeapQueue::default();
    let mut best = None;
    let mut cells = 0;
    for pa in &aa {
        for pb in &bb {
            // All corners and the center are witnesses, not a proof of the minimum.
            let candidates = |p: &Patch| {
                let [[u0, u1], [v0, v1]] = p.domain;
                [
                    [u0, v0],
                    [u0, v1],
                    [u1, v0],
                    [u1, v1],
                    [midpoint(u0, u1), midpoint(v0, v1)],
                ]
            };
            for u in candidates(pa) {
                for v in candidates(pb) {
                    consider(a, b, [u, v], &mut best)?;
                }
            }
            queue.push(
                Cell {
                    parts: [pa.clone(), pb.clone()],
                },
                patch_lower(pa, pb)?,
            );
            cells += 1;
        }
    }
    let mut search = SurfaceSearch { a, b, best };
    let outcome =
        crate::distance::subdivision::run(&mut search, &mut queue, cells, max_cells, tolerance_mm)?;
    let best = search.best.unwrap();
    // Result control points must be finite before leaving the module (1093).
    crate::foundation::guards::require_finite_point(&best.points[0], "surface distance point a")?;
    crate::foundation::guards::require_finite_point(&best.points[1], "surface distance point b")?;
    Ok(SurfaceDistance {
        distance_interval_mm: [outcome.lower, best.upper],
        parameters: best.parameters,
        points: best.points,
        point_enclosures: best
            .bounds
            .map(|bounds| bounds.into_iter().map(|i| [i.lo, i.hi]).collect()),
        converged: outcome.reason == crate::DistanceStopReason::Tolerance,
        reason: outcome.reason,
        cells: outcome.cells,
        max_cells,
        tolerance_mm,
    })
}

/// Outward image bounds of an original UV rectangle, across all knot spans.
/// This is also the enclosure primitive needed by trimmed-face domain traversal.
pub fn rectangle_bounds(s: &Surface, domain: [[f64; 2]; 2]) -> Result<Vec<[f64; 2]>> {
    s.validate()?;
    crate::foundation::guards::require_finite_point(&domain[0], "rectangle u")?;
    crate::foundation::guards::require_finite_point(&domain[1], "rectangle v")?;
    let natural = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    check(
        (0..2).all(|k| {
            domain[k].iter().all(|x| x.is_finite())
                && domain[k][0] >= natural[k][0]
                && domain[k][1] <= natural[k][1]
                && domain[k][0] <= domain[k][1]
        }),
        "Distance rectangle must lie in the surface domain",
    )?;
    let mut union = vec![[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for u in spans(s.degree_u, &s.knots_u, s.control_points.len()) {
        for v in spans(s.degree_v, &s.knots_v, s.control_points[0].len()) {
            let rect = [
                [
                    domain[0][0].max(s.knots_u[u]),
                    domain[0][1].min(s.knots_u[u + 1]),
                ],
                [
                    domain[1][0].max(s.knots_v[v]),
                    domain[1][1].min(s.knots_v[v + 1]),
                ],
            ];
            if rect.iter().any(|d| d[0] > d[1]) {
                continue;
            }
            let bounds = enclosure(s, [u, v], rect)?;
            for k in 0..3 {
                union[k][0] = union[k][0].min(bounds[k].lo);
                union[k][1] = union[k][1].max(bounds[k].hi);
            }
        }
    }
    Ok(union)
}

/// Whole-cell oblique gap from outward rational control enclosures. A single
/// original knot cell per rectangle bounds work; other cases remain unproved.
pub(crate) fn rectangle_control_gap(a: &Surface, b: &Surface, domains: [[[f64; 2]; 2]; 2]) -> Result<Option<f64>> {
    Ok(rectangle_control_query(a, b, domains)?.map(|(gap, _)| gap))
}
/// Validate actual source surfaces and parameter rectangles before requesting
/// a sufficient outward control-enclosure separation bound. A positive gap
/// proves complete image separation; None or zero remains unresolved.
pub fn verify_rectangle_separation_gap(a:&Surface,b:&Surface,domains:[[[f64;2];2];2])->Result<Option<f64>> {
    for (surface,domain) in [a,b].into_iter().zip(domains) {
        surface.validate()?;
        let natural=[[surface.knots_u[surface.degree_u],surface.knots_u[surface.control_points.len()]],[surface.knots_v[surface.degree_v],surface.knots_v[surface.control_points[0].len()]]];
        check((0..2).all(|axis|domain[axis].iter().all(|v|v.is_finite()) && natural[axis][0]<=domain[axis][0] && domain[axis][0]<=domain[axis][1] && domain[axis][1]<=natural[axis][1]),"Separation rectangle must lie in its actual source domain")?;
    }
    rectangle_control_gap(a,b,domains)
}
// A midpoint projection only proposes a subdivision direction. It never
// certifies separation, rejects a contact, or removes any parameter cell.
fn projected_split_proposal(a: &Patch, b: &Patch, surfaces: [&Surface; 2]) -> [f64; 4] {
    let mut n = std::array::from_fn::<_, 3, _>(|k|
        midpoint(a.bounds[k].lo, a.bounds[k].hi) - midpoint(b.bounds[k].lo, b.bounds[k].hi));
    let scale = n.iter().map(|x| x.abs()).fold(0., f64::max);
    if !scale.is_finite() || scale == 0. { return [0.; 4]; }
    for x in &mut n { *x /= scale; }
    let mut scores = [0.; 4];
    for (side, patch) in [a, b].into_iter().enumerate() {
        let degree = [surfaces[side].degree_u, surfaces[side].degree_v];
        let count = std::array::from_fn::<_, 2, _>(|k|
            if patch.domain[k][0] == patch.domain[k][1] { 1 } else { degree[k] + 1 });
        if count[0] * count[1] != patch.controls.len() { return [0.; 4]; }
        let projection = |i: usize| (0..3).map(|k|
            midpoint(patch.controls[i][k].lo, patch.controls[i][k].hi) * n[k]).sum::<f64>();
        for axis in 0..2 {
            let mut score = 0_f64;
            for v in 0..count[1] { for u in 0..count[0] {
                let next = if axis == 0 && u + 1 < count[0] { Some(v * count[0] + u + 1) }
                    else if axis == 1 && v + 1 < count[1] { Some((v + 1) * count[0] + u) }
                    else { None };
                if let Some(j) = next {
                    score = score.max((projection(v * count[0] + u) - projection(j)).abs());
                }
            }}
            scores[2 * side + axis] = score * degree[axis] as f64;
        }
    }
    if scores.iter().any(|x| !x.is_finite()) { [0.; 4] } else { scores }
}
// Fixed integer normals use only -1,0,1. Sign reversal and multiplication
// by one are exact in binary64; zero terms contribute exactly zero. Round
// only actual sums outward instead of widening exact identities first.
fn integer_projection(p: &Patch, n: [i32;3]) -> Result<Interval> {
    let mut lo=f64::INFINITY;let mut hi=f64::NEG_INFINITY;
    for c in &p.controls {
        let mut value: Option<Interval>=None;
        for k in 0..3 {
            let term=match n[k] {
                0=>continue,
                1=>c[k],
                -1=>Interval {lo:-c[k].hi,hi:-c[k].lo},
                _=>unreachable!(),
            };
            value=Some(match value {None=>term,Some(sum)=>sum.add(term)?});
        }
        let value=value.unwrap_or(Interval::point(0.));
        lo=lo.min(value.lo);hi=hi.max(value.hi);
    }
    Interval::new(lo,hi)
}
pub(crate) fn rectangle_control_query(
    a: &Surface,
    b: &Surface,
    domains: [[[f64; 2]; 2]; 2],
) -> Result<Option<(f64, [f64; 4])>> {
    let span = |s: &Surface, d: [[f64; 2]; 2]| -> Option<[usize; 2]> {
        let u = (s.degree_u..s.control_points.len()).find(|&i| {
            s.knots_u[i] < s.knots_u[i + 1]
                && d[0][0] >= s.knots_u[i]
                && d[0][1] <= s.knots_u[i + 1]
        })?;
        let v = (s.degree_v..s.control_points[0].len()).find(|&i| {
            s.knots_v[i] < s.knots_v[i + 1]
                && d[1][0] >= s.knots_v[i]
                && d[1][1] <= s.knots_v[i + 1]
        })?;
        Some([u, v])
    };
    let (Some(sa), Some(sb)) = (span(a, domains[0]), span(b, domains[1])) else {
        return Ok(None);
    };
    let pa = Patch::new(a, sa, domains[0])?;
    let pb = Patch::new(b, sb, domains[1])?;
    let lower = patch_lower(&pa, &pb)?;
    if lower > 0. {
        return Ok(Some((lower, [0.; 4])));
    }
    // Fixed integer normals include inverse directions of simple shears.
    // Each interval dot encloses every actual rational control pole; no
    // rounded separating plane or control midpoint is a positive premise.
    for x in 0..=1 {
        for y in -1..=1 {
            for z in -1..=1 {
                if x == 0 && (y < 0 || y == 0 && z <= 0) {
                    continue;
                }
                let n = [x, y, z];
                let gap = box_distance(&[integer_projection(&pa,n)?], &[integer_projection(&pb,n)?])?.0;
                if gap > 0. {
                    let norm = ((n[0]*n[0]+n[1]*n[1]+n[2]*n[2]) as f64)
                        .sqrt()
                        .next_up();
                    return Ok(Some(((gap / norm).next_down().max(0.), [0.; 4])));
                }
            }
        }
    }
    Ok(Some((0., projected_split_proposal(&pa, &pb, [a, b]))))
}

/// Search-local cache tied to one immutable surface. Work charges are unchanged.
/// Both entry count and retained control count are bounded.
pub(crate) struct PreparedControlSurface<'a> {
    source: &'a Surface,
    entries: std::collections::VecDeque<([u64; 4], Patch)>,
    controls: usize,
}
impl<'a> PreparedControlSurface<'a> {
    pub(crate) fn new(source: &'a Surface) -> Self {
        Self { source, entries: Default::default(), controls: 0 }
    }
    /// Reuse the exact original enclosure used by an oblique control query.
    /// A single complete clamped span makes this identical to rectangle_bounds:
    /// same restriction arithmetic, control order and extrema. Other charts
    /// retain the general multi-span union, including boundary knot behavior.
    pub(crate) fn single_span_bounds(&mut self, domain: [[f64; 2]; 2]) -> Result<Option<Vec<Interval>>> {
        let s = self.source;
        let span = [s.degree_u, s.degree_v];
        if s.control_points.len() != s.degree_u + 1
            || s.control_points[0].len() != s.degree_v + 1
            || s.control_points.len().saturating_mul(s.control_points[0].len()) > 64
            || !complete_bernstein_span(s.degree_u, &s.knots_u, span[0])
            || !complete_bernstein_span(s.degree_v, &s.knots_v, span[1])
            || !(0..2).all(|k| {
                let knots = if k == 0 { &s.knots_u } else { &s.knots_v };
                domain[k].iter().all(|x| x.is_finite())
                    && domain[k][0] >= knots[span[k]]
                    && domain[k][1] <= knots[span[k] + 1]
                    && domain[k][0] <= domain[k][1]
            }) {
            return Ok(None);
        }
        Ok(Some(self.patch(span, domain)?.bounds.clone()))
    }
    fn patch(&mut self, span: [usize; 2], domain: [[f64; 2]; 2]) -> Result<&Patch> {
        let key = [domain[0][0].to_bits(), domain[0][1].to_bits(), domain[1][0].to_bits(), domain[1][1].to_bits()];
        if let Some(i) = self.entries.iter().position(|(k, _)| *k == key) {
            let entry = self.entries.remove(i).unwrap();
            self.entries.push_back(entry);
        } else {
            let patch = Patch::new(self.source, span, domain)?;
            let count = patch.controls.len();
            while !self.entries.is_empty() && (self.entries.len() >= 32 || self.controls.saturating_add(count) > 2048) {
                let (_, old) = self.entries.pop_front().unwrap();
                self.controls -= old.controls.len();
            }
            self.controls += count;
            self.entries.push_back((key, patch));
        }
        Ok(&self.entries.back().unwrap().1)
    }
}

pub(crate) fn rectangle_control_query_prepared(
    cached_a: &mut PreparedControlSurface<'_>,
    cached_b: &mut PreparedControlSurface<'_>,
    domains: [[[f64; 2]; 2]; 2],
) -> Result<Option<(f64, [f64; 4])>> {
    let a = cached_a.source;
    let b = cached_b.source;
    let span = |s: &Surface, d: [[f64; 2]; 2]| -> Option<[usize; 2]> {
        let u = (s.degree_u..s.control_points.len()).find(|&i| {
            s.knots_u[i] < s.knots_u[i + 1]
                && d[0][0] >= s.knots_u[i]
                && d[0][1] <= s.knots_u[i + 1]
        })?;
        let v = (s.degree_v..s.control_points[0].len()).find(|&i| {
            s.knots_v[i] < s.knots_v[i + 1]
                && d[1][0] >= s.knots_v[i]
                && d[1][1] <= s.knots_v[i + 1]
        })?;
        Some([u, v])
    };
    let (Some(sa), Some(sb)) = (span(a, domains[0]), span(b, domains[1])) else {
        return Ok(None);
    };
    let pa = cached_a.patch(sa, domains[0])?;
    let pb = cached_b.patch(sb, domains[1])?;
    let lower = patch_lower(&pa, &pb)?;
    if lower > 0. {
        return Ok(Some((lower, [0.; 4])));
    }
    // Fixed integer normals include inverse directions of simple shears.
    // Each interval dot encloses every actual rational control pole; no
    // rounded separating plane or control midpoint is a positive premise.
    for x in 0..=1 {
        for y in -1..=1 {
            for z in -1..=1 {
                if x == 0 && (y < 0 || y == 0 && z <= 0) {
                    continue;
                }
                let n = [x, y, z];
                let gap = box_distance(&[integer_projection(&pa,n)?], &[integer_projection(&pb,n)?])?.0;
                if gap > 0. {
                    let norm = ((n[0]*n[0]+n[1]*n[1]+n[2]*n[2]) as f64)
                        .sqrt()
                        .next_up();
                    return Ok(Some(((gap / norm).next_down().max(0.), [0.; 4])));
                }
            }
        }
    }
    Ok(Some((0., projected_split_proposal(&pa, &pb, [a, b]))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_projection_keeps_exact_axes_and_encloses_all_corner_sums() {
        let source=plane(0.);let mut p=Patch::new(&source,[1,1],[[0.,1.];2]).unwrap();
        let coordinates=[Interval::new(-2.,3.).unwrap(),Interval::new(4.,8.).unwrap(),Interval::new(-16.,-8.).unwrap()];
        p.controls=vec![coordinates.to_vec()];
        for x in -1..=1 {for y in -1..=1 {for z in -1..=1 {
            let n=[x,y,z];let bound=integer_projection(&p,n).unwrap();
            for corner in 0..8 {
                let exact=(0..3).map(|k|if corner&(1<<k)==0 {coordinates[k].lo*n[k] as f64}
                    else {coordinates[k].hi*n[k] as f64}).sum::<f64>();
                assert!(bound.lo<=exact&&exact<=bound.hi);
            }
        }}}
        p.controls[0][0]=Interval::new(-f64::MAX,f64::MAX).unwrap();
        assert_eq!(integer_projection(&p,[1,0,0]).unwrap(),p.controls[0][0]);
        assert_eq!(integer_projection(&p,[-1,0,0]).unwrap(),p.controls[0][0]);
    }
    #[test]
    fn immutable_control_patch_cache_matches_original_query_bits_after_eviction() {
        let mut a = plane(0.);
        a.weights = vec![vec![1., 0.75], vec![1.5, 2.]];
        let before = a.clone();
        for z in [0., 0.0001, 3.] {
            let mut b = plane(z);
            for row in &mut b.control_points { for p in row { p[0] += p[1] * 0.7; p[2] += p[0] * 0.1; } }
            let mut ca = PreparedControlSurface::new(&a);
            let mut cb = PreparedControlSurface::new(&b);
            let bits = |x: Option<(f64,[f64;4])>| x.map(|(g,s)|(g.to_bits(),s.map(f64::to_bits)));
            for i in (0..80).chain((0..80).rev()) {
                let lo = i as f64 / 160.;
                let da = [[lo, lo+0.5], [0.,1.]];
                let db = [[0.,1.], [0.,1.]];
                assert_eq!(bits(rectangle_control_query(&a,&b,[da,db]).unwrap()),bits(rectangle_control_query_prepared(&mut ca,&mut cb,[da,db]).unwrap()));
                assert!(ca.entries.len()<=32 && cb.entries.len()<=32);
                assert!(ca.controls<=2048 && cb.controls<=2048);
                assert_eq!(bits(rectangle_control_query(&b,&a,[db,da]).unwrap()),bits(rectangle_control_query_prepared(&mut cb,&mut ca,[db,da]).unwrap()));
            }
        }
        assert_eq!(a.control_points,before.control_points);
        assert_eq!(a.weights,before.weights);
        assert_eq!(a.knots_u,before.knots_u);
        assert_eq!(a.knots_v,before.knots_v);
    }

    #[test]
    fn prepared_deboor_coefficients_keep_original_interval_bits() {
        for degree in 0..=8 {
            let knots = [vec![0.; degree+1], vec![1.; degree+1]].concat();
            for lo in [0., 0.125, 0.5, 1.] { for hi in [lo, 1.] {
                for split in 0..=degree {
                    let parameters = (0..degree).map(|i| if i<split {lo} else {hi}).collect::<Vec<_>>();
                    let controls = (0..=degree).map(|i| {
                        let x = i as f64 * 0.125 - 0.5;
                        [Interval::new(x.next_down(), x.next_up()).unwrap(),
                            Interval::point(-x), Interval::point(x*2.), Interval::point(1.)]
                    }).collect::<Vec<_>>();
                    let expected = interpolate(controls.clone(), degree, &knots, degree, &parameters, [1.,1.]).unwrap();
                    let plan = interpolation_plan(degree, &knots, degree, &parameters).unwrap();
                    let actual = interpolate_planned(controls, degree, &plan, [1.,1.]).unwrap();
                    assert_eq!(actual.map(|x| [x.lo.to_bits(),x.hi.to_bits()]),
                        expected.map(|x| [x.lo.to_bits(),x.hi.to_bits()]));
                    assert_eq!(plan.len(), degree*(degree+1)/2);
                }
            }}
        }
    }

    #[test]
    fn outward_oblique_control_gap_proves_shear_and_refuses_touch_and_crossing() {
        let wall = |offset: f64| Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., offset], vec![0., 1., 1. + offset]],
                vec![vec![1., 0., 1. + offset], vec![1., 1., 2. + offset]],
            ],
            weights: vec![vec![1., 1.], vec![0.5, 0.5]],
            periodic_u: false,
            periodic_v: false,
        };
        let a = wall(0.);
        let b = wall(0.125);
        let domains = [[[0., 1.], [0., 1.]]; 2];
        let aa = rectangle_bounds(&a, domains[0]).unwrap();
        let bb = rectangle_bounds(&b, domains[1]).unwrap();
        assert!((0..3).all(|k| aa[k][0] <= bb[k][1] && bb[k][0] <= aa[k][1]));
        assert!(rectangle_control_gap(&a, &b, domains).unwrap().unwrap() > 0.);
        assert!(rectangle_control_gap(&b, &a, domains).unwrap().unwrap() > 0.);
        assert_eq!(rectangle_control_gap(&a, &a, domains).unwrap(), Some(0.));
        let mut crossing = b;
        for p in &mut crossing.control_points[0] {
            p[2] -= 0.25;
        }
        assert_eq!(
            rectangle_control_gap(&a, &crossing, domains).unwrap(),
            Some(0.)
        );
        let before = a.clone();
        rectangle_control_gap(&a, &crossing, domains).unwrap();
        assert_eq!(a, before);
    }
    fn plane(z: f64) -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., z], vec![0., 10., z]],
                vec![vec![10., 0., z], vec![10., 10., z]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    fn point(p: [f64; 3]) -> Surface {
        let mut s = plane(0.);
        for row in &mut s.control_points {
            for cp in row {
                *cp = p.to_vec();
            }
        }
        s
    }
    fn contains(r: &SurfaceDistance, exact: f64) {
        assert!(
            r.distance_interval_mm[0] <= exact && r.distance_interval_mm[1] >= exact,
            "exact {exact}, {r:?}"
        );
    }
    #[test]
    fn planes_and_interior_point_projection() {
        let a = plane(0.);
        let b = plane(3.);
        let r = distance(&a, &b, 1e-8, 100).unwrap();
        contains(&r, 3.);
        assert!(r.converged);
        assert_eq!(r.cells, 1);
        let b = point([3.7, 6.2, 2.]);
        let r = distance(&a, &b, 1e-5, 10000).unwrap();
        contains(&r, 2.);
        assert!(r.converged, "{r:?}");
        assert!((r.parameters[0][0] - 0.37).abs() < 0.002);
        assert!((r.parameters[0][1] - 0.62).abs() < 0.002);
        for (i, s) in [&a, &b].into_iter().enumerate() {
            assert_eq!(
                r.points[i],
                s.evaluate(r.parameters[i][0], r.parameters[i][1])
                    .unwrap()
                    .point
            );
        }
    }
    #[test]
    fn rational_cylinder_patch_and_weight_scaling() {
        for scale in [1e-11, 1., 1e11] {
            let mut a = plane(0.);
            a.degree_u = 2;
            a.knots_u = vec![0., 0., 0., 1., 1., 1.];
            a.control_points = vec![
                vec![vec![10., 0., 0.], vec![10., 0., 5.]],
                vec![vec![10., 10., 0.], vec![10., 10., 5.]],
                vec![vec![0., 10., 0.], vec![0., 10., 5.]],
            ];
            a.weights = vec![
                vec![scale; 2],
                vec![scale * std::f64::consts::FRAC_1_SQRT_2; 2],
                vec![scale; 2],
            ];
            let b = point([15., 15., 2.37]);
            let r = distance(&a, &b, 1e-3, 20000).unwrap();
            contains(&r, 450_f64.sqrt() - 10.);
            assert!(r.converged, "{r:?}");
            assert!((r.points[0][0].hypot(r.points[0][1]) - 10.).abs() < 1e-10);
        }
    }
    fn bowl() -> Surface {
        let c = [0.37, 0.62];
        let q = c.map(|x| [x * x, x * x - x, (1. - x) * (1. - x)]);
        Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| vec![i as f64 / 2., j as f64 / 2., q[0][i] + q[1][j]])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn joint_minimum_inside_both_curved_surfaces() {
        let a = bowl();
        let mut b = a.clone();
        for row in &mut b.control_points {
            for p in row {
                p[2] = -p[2] - 2.;
            }
        }
        let r = distance(&a, &b, 0.002, 100000).unwrap();
        contains(&r, 2.);
        assert!(r.converged, "{r:?}");
        assert!(
            r.parameters
                .iter()
                .all(|uv| uv[0] > 0. && uv[0] < 1. && uv[1] > 0. && uv[1] < 1.)
        );
    }
    #[test]
    fn work_limit_is_not_a_completed_minimum() {
        let a = plane(0.);
        let b = point([3.7, 6.2, 2.]);
        let r = distance(&a, &b, 1e-9, 1).unwrap();
        contains(&r, 2.);
        assert!(!r.converged);
        assert_eq!(r.reason, crate::DistanceStopReason::WorkLimit);
        assert_eq!(r.cells, 1);
    }
    #[test]
    fn original_knots_multiple_spans_and_periodic_seam() {
        let mut a = plane(0.);
        a.knots_u = vec![0., 0., 0.4, 1., 1.];
        a.control_points
            .insert(1, vec![vec![4., 0., 0.], vec![4., 10., 0.]]);
        a.weights.insert(1, vec![1.; 2]);
        let b = point([7.3, 6.2, 2.]);
        let r = distance(&a, &b, 1e-4, 10000).unwrap();
        contains(&r, 2.);
        assert!(r.converged, "{r:?}");
        assert!(distance(&a, &b, 1e-3, 1).is_err());
        let mut ring = plane(0.);
        ring.periodic_u = true;
        ring.knots_u = vec![-1., 0., 1., 2., 3., 4., 5.];
        ring.control_points = vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]]
            .into_iter()
            .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 2.]])
            .collect();
        ring.weights = vec![vec![1.; 2]; 5];
        let r = distance(&ring, &point([-1., 0., 0.]), 1e-8, 100).unwrap();
        contains(&r, 1.);
        assert!(r.converged);
        assert_eq!(r.parameters[0][0], 0.);
    }
    #[test]
    fn translation_and_invalid_inputs() {
        let mut a = plane(0.);
        for row in &mut a.control_points {
            for p in row {
                p.iter_mut().for_each(|x| *x += 1e6);
            }
        }
        let r = distance(&a, &point([1e6 + 3.7, 1e6 + 6.2, 1e6 + 2.]), 0.001, 10000).unwrap();
        contains(&r, 2.);
        assert!(r.converged);
        assert!(distance(&a, &a, 0., 100).is_err());
        assert!(distance(&a, &a, 1e-3, 0).is_err());
        let mut bad = a.clone();
        bad.weights[0][0] = 0.;
        assert!(distance(&a, &bad, 0.001, 100).is_err());
    }
    #[test]
    fn non_finite_boundaries_are_rejected_and_results_stay_finite() {
        let a = plane(0.);
        // nearest_point query point must be finite (1093).
        let err = nearest_point(&a, [f64::NAN, 0., 0.], 1e-6, 100).unwrap_err();
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
        assert!(err.contains("point"), "{err}");
        // rectangle_bounds domain must be finite.
        let err = rectangle_bounds(&a, [[0., f64::INFINITY], [0., 1.]]).unwrap_err();
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
        // A normal run returns finite witness points.
        let r = distance(&a, &point([3.7, 6.2, 2.]), 1e-5, 1000).unwrap();
        assert!(r.converged);
        assert!(r.points.iter().flatten().all(|v| v.is_finite()));
    }
}

/// Conservative distance bounds between two supplied Cartesian enclosure boxes.
pub fn enclosure_distance(a: &[[f64; 2]], b: &[[f64; 2]]) -> Result<(f64, f64)> {
    check(
        a.len() == 3 && b.len() == 3,
        "Distance boxes must have three axes",
    )?;
    let convert = |p: &[[f64; 2]]| {
        p.iter()
            .map(|v| Interval::new(v[0], v[1]))
            .collect::<Result<Vec<_>>>()
    };
    box_distance(&convert(a)?, &convert(b)?)
}

/// Candidate and global distance bounds for a query against a full surface.
/// UV coordinates identify a witness, not a unique or certified exact minimizer.
#[derive(Clone, Debug)]
pub struct NearestPoint {
    pub parameters: [f64; 2],
    pub point: [f64; 3],
    pub point_enclosure: Vec<[f64; 2]>,
    pub distance_interval: [f64; 2],
    pub converged: bool,
    pub reason: crate::DistanceStopReason,
    pub cells: usize,
}

/// Search the full untrimmed active surface image, including its boundary.
/// Numeric limits, tolerance and work budgets follow `distance`.
pub fn nearest_point(
    surface: &Surface,
    point: [f64; 3],
    tolerance: f64,
    max_cells: usize,
) -> Result<NearestPoint> {
    crate::foundation::guards::require_finite_point(&point, "point")?;
    let constant = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![vec![point.to_vec(); 2]; 2],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let report = distance(surface, &constant, tolerance, max_cells)?;
    Ok(NearestPoint {
        parameters: report.parameters[0],
        point: report.points[0],
        point_enclosure: report.point_enclosures[0].clone(),
        distance_interval: report.distance_interval_mm,
        converged: report.converged,
        reason: report.reason,
        cells: report.cells,
    })
}

#[cfg(test)]
mod blend_factor_regression {
use super::*;
    #[test]
    fn nonnegative_factor_matches_original_interval_bits() {
        let bits=|x:Interval|[x.lo.to_bits(),x.hi.to_bits()];
        let values=[-f64::MAX,-1e200,-1.,-f64::MIN_POSITIVE,-f64::from_bits(1),-0.,0.,f64::from_bits(1),f64::MIN_POSITIVE,1.,1e200,f64::MAX];
        for &lo in &values {for &hi in &values {if lo>hi {continue}
            for factor in [[0.,0.],[0.,1.],[0.125,0.875],[1.,1.],[f64::from_bits(1),f64::MIN_POSITIVE]] {
                let a=Interval{lo,hi};let b=Interval{lo:factor[0],hi:factor[1]};
                assert_eq!(a.mul(b).map(bits),mul_blend_factor(a,b).map(bits));
            }
        }}
        let mut seed=0x784abc3712d051f3_u64;
        for _ in 0..1000000 {
            let mut next=||{seed^=seed<<13;seed^=seed>>7;seed^=seed<<17;f64::from_bits(seed)};
            let x=next();let y=next();let z=next().abs();let w=next().abs();
            if ![x,y,z,w].iter().all(|v|v.is_finite()){continue}
            let a=Interval{lo:x.min(y),hi:x.max(y)};let b=Interval{lo:z.min(w),hi:z.max(w)};
            assert_eq!(a.mul(b).map(bits),mul_blend_factor(a,b).map(bits));
        }
    }
}
