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
    let mut plan = Vec::with_capacity(degree * (degree + 1) / 2);
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
pub(crate) fn interpolate_planned(d: &[[Interval; 4]], degree: usize,
    plan: &[(usize, Interval, Interval)], weights: [f64; 2]) -> Result<[Interval; 4]> {
    let mut buf = [[Interval::point(0.); 4]; 33];
    buf[..=degree].copy_from_slice(&d[..=degree]);
    for &(j, alpha, beta) in plan {
        for axis in 0..4 {
            let a = buf[j - 1][axis];
            let b = buf[j][axis];
            buf[j][axis] = mul_blend_factor(a,beta)?.add(mul_blend_factor(b,alpha)?)?
                .intersect(a.lo.min(b.lo), a.hi.max(b.hi))?;
        }
        buf[j][3] = buf[j][3].intersect(weights[0], weights[1])?;
    }
    Ok(buf[degree])
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
    let origin = [
        midpoint(a.bounds[0].lo, a.bounds[0].hi),
        midpoint(a.bounds[1].lo, a.bounds[1].hi),
        midpoint(a.bounds[2].lo, a.bounds[2].hi),
    ];
    let mut n = [
        origin[0] * 0.5 - midpoint(b.bounds[0].lo, b.bounds[0].hi) * 0.5,
        origin[1] * 0.5 - midpoint(b.bounds[1].lo, b.bounds[1].hi) * 0.5,
        origin[2] * 0.5 - midpoint(b.bounds[2].lo, b.bounds[2].hi) * 0.5,
    ];
    let scale = n[0].abs().max(n[1].abs()).max(n[2].abs());
    if scale == 0. {
        return Ok(axis_lower);
    }
    for v in &mut n {
        *v /= scale;
    }
    let norm_upper = box_distance(
        &[Interval::point(n[0]), Interval::point(n[1]), Interval::point(n[2])],
        &[Interval::point(0.); 3],
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
#[path = "surface_distance/tests/tests_integration.rs"]
mod tests;

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
#[path = "surface_distance/tests/blend_factor_regression_integration.rs"]
mod blend_factor_regression;

#[path = "surface_distance/restriction_query.rs"]
mod restriction_query;
use restriction_query::restricted_controls;
