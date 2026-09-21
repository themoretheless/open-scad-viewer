//! Tolerant Boolean fallback: numerical surface/surface intersection with a
//! stated tolerance, for operand pairs the exact analytic matrix refuses.
//!
//! Every face pair whose control hulls overlap is traced: seeds on the first
//! face are projected onto the second by Gauss-Newton, each converged seed
//! that lies inside both trimmed regions starts a predictor/corrector march
//! along `N_a × N_b` in both directions until the curve leaves a trimmed
//! region (the exit is then refined to the crossing of the boundary edge
//! with the other surface, which becomes a shared hit vertex) or closes on
//! itself. Chains are interpolated by cubic B-splines with one common
//! parameterization for the 3D curve and both pcurves; the achieved
//! deviation, measured against both surfaces between the samples, becomes
//! the result's `tolerance_mm` (refused when it would exceed the kernel's
//! 1e-2 mm ceiling). Regions of every face are classified by ray parity
//! against the other solid's exact surfaces and assembled through
//! `imprint_assembly`.
//!
//! This path never claims exactness: the result carries its tolerance, and
//! the caller can tell it apart from an exact result by that field.
use crate::imprint_assembly::{Assembler, ChartVertices, EKey, VKey, face_reversed};
use crate::imprint_pipeline::{self, SpatialRelation};
use crate::uv_regions::{Piece, pcurve_point};
use crate::{Coedge, Model};
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};

/// Target chordal tolerance of the traced curves.
const TARGET: f64 = 1e-4;
/// Kernel ceiling for `tolerance_mm`.
const CEILING: f64 = 1e-2;
/// Smallest `tolerance_mm` a tolerant result reports; exact results stay at
/// the operands' 1e-7 mm, so anything above 1e-6 mm is tolerant.
pub const TOLERANT_FLOOR: f64 = 2e-6;
/// Newton convergence for surface/surface points, relative to the scale.
const CONVERGE: f64 = 1e-11;
/// Largest turning angle between consecutive march directions.
const MAX_TURN: f64 = 0.12;
const MAX_POINTS: usize = 4000;
const DENSE: usize = 32;

fn unsupported(message: impl Into<String>) -> Error {
    Error::new("BREP_UNSUPPORTED_OPERATION", message)
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn scale(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: [f64; 3]) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
fn unit(a: [f64; 3]) -> Option<[f64; 3]> {
    let n = norm(a);
    (n > 0.).then(|| scale(a, 1. / n))
}
fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    norm(sub(a, b))
}

// ---------------------------------------------------------------------------
// Surface access
// ---------------------------------------------------------------------------

struct Jet {
    p: [f64; 3],
    du: [f64; 3],
    dv: [f64; 3],
}

/// Trimmed face with its UV domain, dense boundary polygons and per-coedge
/// samples for locating boundary crossings.
struct Chart<'m> {
    model: &'m Model,
    face: usize,
    domain: [[f64; 2]; 2],
    /// Sampled loops (outer first) in UV.
    polygons: Vec<Vec<[f64; 2]>>,
    /// For every loop, for every coedge: UV samples along the coedge
    /// (DENSE + 1 points) and the coedge's edge / reversed flag.
    coedges: Vec<Vec<(Vec<[f64; 2]>, usize, bool)>>,
}

impl<'m> Chart<'m> {
    fn new(model: &'m Model, face: usize) -> Result<Self> {
        let f = &model.faces[face];
        let s = &f.surface;
        let domain = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        let mut polygons = Vec::new();
        let mut coedges = Vec::new();
        for &wire in std::iter::once(&f.outer).chain(&f.holes) {
            let mut polygon = Vec::new();
            let mut list = Vec::new();
            for coedge in &model.loops[wire].coedges {
                let [a, b] = coedge.pcurve.domain();
                let mut samples = Vec::with_capacity(DENSE + 1);
                for i in 0..=DENSE {
                    let t = a + (b - a) * i as f64 / DENSE as f64;
                    samples.push(pcurve_point(&coedge.pcurve, t)?);
                }
                polygon.extend(samples[..DENSE].iter().copied());
                list.push((samples, coedge.edge, coedge.reversed));
            }
            polygons.push(polygon);
            coedges.push(list);
        }
        Ok(Self {
            model,
            face,
            domain,
            polygons,
            coedges,
        })
    }
    fn surface(&self) -> &'m Surface {
        &self.model.faces[self.face].surface
    }
    fn clamp(&self, uv: [f64; 2]) -> [f64; 2] {
        [
            uv[0].clamp(self.domain[0][0], self.domain[0][1]),
            uv[1].clamp(self.domain[1][0], self.domain[1][1]),
        ]
    }
    fn jet(&self, uv: [f64; 2]) -> Result<Jet> {
        let uv = self.clamp(uv);
        let e = self.surface().evaluate(uv[0], uv[1])?;
        let (du, dv) = e.first_derivatives().ok_or_else(|| {
            unsupported("Tolerant Boolean: surface derivative undefined at a traced point")
        })?;
        Ok(Jet { p: e.point, du, dv })
    }
    fn point(&self, uv: [f64; 2]) -> Result<[f64; 3]> {
        let uv = self.clamp(uv);
        Ok(self.surface().evaluate(uv[0], uv[1])?.point)
    }
    /// Signed containment in the trimmed region: winding parity over the
    /// sampled loops.
    fn inside(&self, uv: [f64; 2]) -> bool {
        let mut crossings = 0usize;
        for polygon in &self.polygons {
            for i in 0..polygon.len() {
                let a = polygon[i];
                let b = polygon[(i + 1) % polygon.len()];
                if (a[1] > uv[1]) != (b[1] > uv[1]) {
                    let x = a[0] + (uv[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0]);
                    if x > uv[0] {
                        crossings += 1;
                    }
                }
            }
        }
        crossings % 2 == 1
    }
    /// Distance from a UV point to the nearest boundary sample segment.
    fn boundary_distance(&self, uv: [f64; 2]) -> f64 {
        let mut best = f64::INFINITY;
        for polygon in &self.polygons {
            for i in 0..polygon.len() {
                let a = polygon[i];
                let b = polygon[(i + 1) % polygon.len()];
                best = best.min(segment_distance(a, b, uv));
            }
        }
        best
    }
    /// The coedge whose sampled polyline the UV segment `from -> to`
    /// crosses first: (loop, coedge index, coedge-local parameter in [0, 1]).
    fn crossing(&self, from: [f64; 2], to: [f64; 2]) -> Option<(usize, usize, f64)> {
        let mut best: Option<(f64, (usize, usize, f64))> = None;
        for (li, list) in self.coedges.iter().enumerate() {
            for (ci, (samples, _, _)) in list.iter().enumerate() {
                for k in 0..DENSE {
                    let (a, b) = (samples[k], samples[k + 1]);
                    if let Some((s, u)) = segments_intersect(from, to, a, b) {
                        let local = (k as f64 + u) / DENSE as f64;
                        if best.map(|(bs, _)| s < bs).unwrap_or(true) {
                            best = Some((s, (li, ci, local)));
                        }
                    }
                }
            }
        }
        best.map(|(_, hit)| hit)
    }
}

fn segment_distance(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let len2 = d[0] * d[0] + d[1] * d[1];
    let t = if len2 > 0. {
        (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / len2).clamp(0., 1.)
    } else {
        0.
    };
    let q = [a[0] + t * d[0], a[1] + t * d[1]];
    (q[0] - p[0]).hypot(q[1] - p[1])
}

/// Parameters (s along p->q, u along a->b) of a proper crossing.
fn segments_intersect(p: [f64; 2], q: [f64; 2], a: [f64; 2], b: [f64; 2]) -> Option<(f64, f64)> {
    let r = [q[0] - p[0], q[1] - p[1]];
    let d = [b[0] - a[0], b[1] - a[1]];
    let den = r[0] * d[1] - r[1] * d[0];
    if den.abs() < 1e-18 {
        return None;
    }
    let w = [a[0] - p[0], a[1] - p[1]];
    let s = (w[0] * d[1] - w[1] * d[0]) / den;
    let u = (w[0] * r[1] - w[1] * r[0]) / den;
    ((-1e-9..=1. + 1e-9).contains(&s) && (-1e-9..=1. + 1e-9).contains(&u))
        .then(|| (s.clamp(0., 1.), u.clamp(0., 1.)))
}

/// Solve a small dense linear system by Gaussian elimination with partial
/// pivoting; `None` when singular.
fn solve(mut m: Vec<Vec<f64>>, mut rhs: Vec<f64>) -> Option<Vec<f64>> {
    let n = rhs.len();
    for col in 0..n {
        let pivot = (col..n).max_by(|&i, &j| m[i][col].abs().total_cmp(&m[j][col].abs()))?;
        if m[pivot][col].abs() < 1e-300 {
            return None;
        }
        m.swap(col, pivot);
        rhs.swap(col, pivot);
        for row in col + 1..n {
            let f = m[row][col] / m[col][col];
            if f == 0. {
                continue;
            }
            for k in col..n {
                m[row][k] -= f * m[col][k];
            }
            rhs[row] -= f * rhs[col];
        }
    }
    let mut x = vec![0.; n];
    for row in (0..n).rev() {
        let mut acc = rhs[row];
        for k in row + 1..n {
            acc -= m[row][k] * x[k];
        }
        if m[row][row].abs() < 1e-300 {
            return None;
        }
        x[row] = acc / m[row][row];
    }
    Some(x)
}

// ---------------------------------------------------------------------------
// Surface/surface tracing
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct TracePoint {
    p: [f64; 3],
    a: [f64; 2],
    b: [f64; 2],
}

/// Where an open chain ends: on a trim boundary of operand `operand`, at
/// (edge, edge parameter), with the UV on the other face.
#[derive(Clone, Copy, Debug)]
struct BoundaryHit {
    operand: usize,
    edge: usize,
    t: f64,
    point: [f64; 3],
    a: [f64; 2],
    b: [f64; 2],
}

struct Chain {
    points: Vec<TracePoint>,
    closed: bool,
    start: Option<BoundaryHit>,
    end: Option<BoundaryHit>,
}

struct Tracer<'m> {
    a: Chart<'m>,
    b: Chart<'m>,
    scale: f64,
    step: f64,
}

impl<'m> Tracer<'m> {
    /// Gauss-Newton on `Pa(u,v) - Pb(s,t) = 0` (4 unknowns, 3 residuals,
    /// minimum-norm update), from a start; returns the converged point.
    fn project(&self, mut ua: [f64; 2], mut ub: [f64; 2]) -> Result<Option<TracePoint>> {
        for _ in 0..40 {
            let ja = self.a.jet(ua)?;
            let jb = self.b.jet(ub)?;
            let r = sub(ja.p, jb.p);
            if norm(r) <= CONVERGE * self.scale {
                return Ok(Some(TracePoint {
                    p: ja.p,
                    a: self.a.clamp(ua),
                    b: self.b.clamp(ub),
                }));
            }
            // J = [du_a dv_a -du_b -dv_b] (3x4); minimum-norm step solves
            // (J J^T) y = -r, x = J^T y.
            let cols = [ja.du, ja.dv, scale(jb.du, -1.), scale(jb.dv, -1.)];
            let mut jjt = vec![vec![0.; 3]; 3];
            for i in 0..3 {
                for j in 0..3 {
                    jjt[i][j] = cols.iter().map(|c| c[i] * c[j]).sum();
                }
            }
            let Some(y) = solve(jjt, scale(r, -1.).to_vec()) else {
                return Ok(None);
            };
            let x: Vec<f64> = cols
                .iter()
                .map(|c| c[0] * y[0] + c[1] * y[1] + c[2] * y[2])
                .collect();
            ua = self.a.clamp([ua[0] + x[0], ua[1] + x[1]]);
            ub = self.b.clamp([ub[0] + x[2], ub[1] + x[3]]);
        }
        Ok(None)
    }

    /// Unit march direction at a point, or `None` when the surfaces are
    /// tangential there.
    fn direction(&self, pt: &TracePoint) -> Result<Option<[f64; 3]>> {
        let ja = self.a.jet(pt.a)?;
        let jb = self.b.jet(pt.b)?;
        let na = unit(cross(ja.du, ja.dv));
        let nb = unit(cross(jb.du, jb.dv));
        let (Some(na), Some(nb)) = (na, nb) else {
            return Ok(None);
        };
        let d = cross(na, nb);
        if norm(d) < 1e-3 {
            return Ok(None);
        }
        Ok(unit(d))
    }

    /// Corrector: the intersection point on the plane through `target`
    /// perpendicular to `dir`, from the predicted parameters.
    fn correct(
        &self,
        ua: [f64; 2],
        ub: [f64; 2],
        target: [f64; 3],
        dir: [f64; 3],
    ) -> Result<Option<TracePoint>> {
        let (mut ua, mut ub) = (ua, ub);
        for _ in 0..40 {
            let ja = self.a.jet(ua)?;
            let jb = self.b.jet(ub)?;
            let r = sub(ja.p, jb.p);
            let plane = dot(sub(ja.p, target), dir);
            if norm(r) <= CONVERGE * self.scale && plane.abs() <= CONVERGE * self.scale {
                return Ok(Some(TracePoint {
                    p: ja.p,
                    a: self.a.clamp(ua),
                    b: self.b.clamp(ub),
                }));
            }
            // 4x4 system: rows = residual components + plane constraint.
            let cols = [ja.du, ja.dv, scale(jb.du, -1.), scale(jb.dv, -1.)];
            let mut m = vec![vec![0.; 4]; 4];
            for i in 0..3 {
                for (j, c) in cols.iter().enumerate() {
                    m[i][j] = c[i];
                }
            }
            m[3] = vec![dot(ja.du, dir), dot(ja.dv, dir), 0., 0.];
            let rhs = vec![-r[0], -r[1], -r[2], -plane];
            let Some(x) = solve(m, rhs) else {
                return Ok(None);
            };
            ua = self.a.clamp([ua[0] + x[0], ua[1] + x[1]]);
            ub = self.b.clamp([ub[0] + x[2], ub[1] + x[3]]);
        }
        Ok(None)
    }

    fn inside_both(&self, pt: &TracePoint) -> bool {
        self.a.inside(pt.a) && self.b.inside(pt.b)
    }

    /// March from `start` in the sense of `sign`. Returns the points after
    /// the start and the boundary hit when the chain leaves a region.
    fn march(
        &self,
        start: TracePoint,
        sign: f64,
        closed: &mut bool,
    ) -> Result<(Vec<TracePoint>, Option<BoundaryHit>)> {
        let mut out = Vec::new();
        let mut current = start;
        let Some(mut dir) = self.direction(&current)? else {
            return Err(unsupported(
                "Tolerant Boolean: surfaces are tangential along the intersection",
            ));
        };
        dir = scale(dir, sign);
        let mut h = self.step;
        let mut arc = 0.;
        while out.len() < MAX_POINTS {
            let target = add(current.p, scale(dir, h));
            let (ja, jb) = (self.a.jet(current.a)?, self.b.jet(current.b)?);
            // Predictor in UV: least-squares of the tangent onto each chart.
            let guess = |j: &Jet, uv: [f64; 2]| -> [f64; 2] {
                let g = [
                    [dot(j.du, j.du), dot(j.du, j.dv)],
                    [dot(j.du, j.dv), dot(j.dv, j.dv)],
                ];
                let rhs = [dot(j.du, dir) * h, dot(j.dv, dir) * h];
                let det = g[0][0] * g[1][1] - g[0][1] * g[1][0];
                if det.abs() < 1e-300 {
                    return uv;
                }
                [
                    uv[0] + (rhs[0] * g[1][1] - rhs[1] * g[0][1]) / det,
                    uv[1] + (g[0][0] * rhs[1] - g[1][0] * rhs[0]) / det,
                ]
            };
            let (ga, gb) = (guess(&ja, current.a), guess(&jb, current.b));
            // The predicted step leaves a trimmed region (possibly the
            // parameter domain itself): creep up to the boundary, then hand
            // over to the exact edge/surface refinement.
            if !self.a.inside(ga) || !self.b.inside(gb) {
                if h > self.step * 1e-3 {
                    h *= 0.5;
                    continue;
                }
                let hit = self.exit(current, ga, gb)?;
                return Ok((out, Some(hit)));
            }
            let next = self.correct(ga, gb, target, dir)?;
            let Some(next) = next else {
                h *= 0.5;
                if h < 1e-9 * self.scale {
                    return Err(unsupported(
                        "Tolerant Boolean: intersection march failed to converge",
                    ));
                }
                continue;
            };
            let Some(next_dir) = self.direction(&next)? else {
                return Err(unsupported(
                    "Tolerant Boolean: surfaces are tangential along the intersection",
                ));
            };
            let next_dir = if dot(next_dir, dir) < 0. {
                scale(next_dir, -1.)
            } else {
                next_dir
            };
            let turn = dot(next_dir, dir).clamp(-1., 1.).acos();
            if turn > MAX_TURN && h > 1e-7 * self.scale {
                h *= 0.5;
                continue;
            }
            // The corrected point landed outside a region although the
            // prediction was inside: creep as above.
            if !self.inside_both(&next) {
                if h > self.step * 1e-3 {
                    h *= 0.5;
                    continue;
                }
                let hit = self.exit(current, next.a, next.b)?;
                return Ok((out, Some(hit)));
            }
            arc += dist(current.p, next.p);
            // Closing: back near the start after a full turn.
            if out.len() >= 4 && dist(next.p, start.p) < 1.5 * h && arc > 6. * h {
                *closed = true;
                return Ok((out, None));
            }
            out.push(next);
            current = next;
            dir = next_dir;
            if turn < MAX_TURN * 0.3 {
                h = (h * 1.5).min(self.step);
            }
        }
        Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Tolerant Boolean: intersection trace exceeded its point budget",
        ))
    }

    /// The exit of the curve from a trimmed region, between the inside point
    /// `inside` and the predicted parameters `ga` / `gb` just outside: the
    /// crossing of the trim edge's 3D curve with the other surface.
    fn exit(&self, inside: TracePoint, ga: [f64; 2], gb: [f64; 2]) -> Result<BoundaryHit> {
        let lo = inside;
        let hi = TracePoint {
            p: inside.p,
            a: ga,
            b: gb,
        };
        // Which chart was left, and through which coedge.
        let left_a = !self.a.inside(hi.a);
        let left_b = !self.b.inside(hi.b);
        if left_a && left_b {
            return Err(unsupported(
                "Tolerant Boolean: the intersection leaves both faces at one point (edge/edge contact)",
            ));
        }
        let (chart, operand, from, to) = if left_a {
            (&self.a, 0usize, lo.a, hi.a)
        } else {
            (&self.b, 1usize, lo.b, hi.b)
        };
        let Some((li, ci, local)) = chart.crossing(from, to) else {
            return Err(unsupported(
                "Tolerant Boolean: could not locate the trim edge the intersection leaves through",
            ));
        };
        let (_, edge, reversed) = &chart.coedges[li][ci];
        let edge = *edge;
        let t0 = if *reversed { 1. - local } else { local };
        // Refine: edge curve C(t) meets the other surface S(s,r).
        let curve = &chart.model.edges[edge].curve;
        let other = if left_a { &self.b } else { &self.a };
        let mut t = t0;
        let mut uv = if left_a { hi.b } else { hi.a };
        let mut converged = false;
        for _ in 0..50 {
            let ce = curve.evaluate(t.clamp(0., 1.))?;
            let cp = ce.point;
            let cp = [cp[0], cp[1], cp[2]];
            let j = other.jet(uv)?;
            let r = sub(cp, j.p);
            if norm(r) <= CONVERGE * self.scale {
                converged = true;
                break;
            }
            // Tangent of the edge by finite difference (rational curves).
            let dt = 1e-6;
            let cq = curve.evaluate((t + dt).clamp(0., 1.))?.point;
            let cm = curve.evaluate((t - dt).clamp(0., 1.))?.point;
            let tangent = scale(
                sub([cq[0], cq[1], cq[2]], [cm[0], cm[1], cm[2]]),
                1. / (2. * dt),
            );
            let cols = [tangent, scale(j.du, -1.), scale(j.dv, -1.)];
            let mut m = vec![vec![0.; 3]; 3];
            for i in 0..3 {
                for (k, c) in cols.iter().enumerate() {
                    m[i][k] = c[i];
                }
            }
            let Some(x) = solve(m, scale(r, -1.).to_vec()) else {
                break;
            };
            t = (t + x[0]).clamp(0., 1.);
            uv = other.clamp([uv[0] + x[1], uv[1] + x[2]]);
        }
        if !converged {
            return Err(unsupported(
                "Tolerant Boolean: boundary hit did not converge on the trim edge",
            ));
        }
        if t < 1e-6 || t > 1. - 1e-6 {
            return Err(unsupported(
                "Tolerant Boolean: the intersection passes through a vertex of an operand",
            ));
        }
        if !other.inside(uv) || other.boundary_distance(uv) < 1e-6 {
            return Err(unsupported(
                "Tolerant Boolean: the intersection leaves a face at another face's boundary",
            ));
        }
        let p = curve.evaluate(t)?.point;
        let point = [p[0], p[1], p[2]];
        // UV on the left chart: the coedge pcurve at the local parameter.
        let (samples, _, _) = &chart.coedges[li][ci];
        let local = if *reversed { 1. - t } else { t };
        let coedge_uv = {
            let wire = std::iter::once(&chart.model.faces[chart.face].outer)
                .chain(&chart.model.faces[chart.face].holes)
                .nth(li)
                .copied()
                .unwrap();
            let coedge: &Coedge = &chart.model.loops[wire].coedges[ci];
            let [d0, d1] = coedge.pcurve.domain();
            pcurve_point(&coedge.pcurve, d0 + (d1 - d0) * local)?
        };
        let _ = samples;
        let (a, b) = if left_a {
            (coedge_uv, uv)
        } else {
            (uv, coedge_uv)
        };
        Ok(BoundaryHit {
            operand,
            edge,
            t,
            point,
            a,
            b,
        })
    }

    /// From an intersection point on a trim edge, the point half a step
    /// along the curve that lies clear inside both regions, if either sense
    /// leads there.
    fn step_inside(&self, pt: &TracePoint) -> Result<Option<TracePoint>> {
        let Some(dir) = self.direction(pt)? else {
            return Ok(None);
        };
        for sign in [1., -1.] {
            let d = scale(dir, sign);
            let mut h = self.step * 0.5;
            for _ in 0..6 {
                let target = add(pt.p, scale(d, h));
                let (ja, jb) = (self.a.jet(pt.a)?, self.b.jet(pt.b)?);
                let guess = |j: &Jet, uv: [f64; 2]| -> [f64; 2] {
                    let g = [
                        [dot(j.du, j.du), dot(j.du, j.dv)],
                        [dot(j.du, j.dv), dot(j.dv, j.dv)],
                    ];
                    let rhs = [dot(j.du, d) * h, dot(j.dv, d) * h];
                    let det = g[0][0] * g[1][1] - g[0][1] * g[1][0];
                    if det.abs() < 1e-300 {
                        return uv;
                    }
                    [
                        uv[0] + (rhs[0] * g[1][1] - rhs[1] * g[0][1]) / det,
                        uv[1] + (g[0][0] * rhs[1] - g[1][0] * rhs[0]) / det,
                    ]
                };
                let (ga, gb) = (guess(&ja, pt.a), guess(&jb, pt.b));
                if self.a.inside(ga)
                    && self.b.inside(gb)
                    && let Some(next) = self.correct(ga, gb, target, d)?
                    && self.inside_both(&next)
                    && self.a.boundary_distance(next.a) >= 1e-4
                    && self.b.boundary_distance(next.b) >= 1e-4
                {
                    return Ok(Some(next));
                }
                h *= 0.5;
            }
        }
        Ok(None)
    }

    /// All intersection chains of the face pair.
    fn trace_all(&self, seeds_per_axis: usize) -> Result<Vec<Chain>> {
        let mut chains: Vec<Chain> = Vec::new();
        let mut covered: Vec<[f64; 3]> = Vec::new();
        let mut grazing = 0usize;
        let grid = |d: [[f64; 2]; 2], i: usize, j: usize| -> [f64; 2] {
            [
                d[0][0] + (d[0][1] - d[0][0]) * (i as f64 + 0.5) / seeds_per_axis as f64,
                d[1][0] + (d[1][1] - d[1][0]) * (j as f64 + 0.5) / seeds_per_axis as f64,
            ]
        };
        // Coarse samples of b for the nearest-point start.
        let mut b_samples = Vec::new();
        for i in 0..seeds_per_axis {
            for j in 0..seeds_per_axis {
                let uv = grid(self.b.domain, i, j);
                b_samples.push((uv, self.b.point(uv)?));
            }
        }
        for i in 0..seeds_per_axis {
            for j in 0..seeds_per_axis {
                let ua = grid(self.a.domain, i, j);
                if !self.a.inside(ua) {
                    continue;
                }
                let pa = self.a.point(ua)?;
                let ub = b_samples
                    .iter()
                    .min_by(|x, y| dist(x.1, pa).total_cmp(&dist(y.1, pa)))
                    .map(|s| s.0)
                    .unwrap();
                let Some(pt) = self.project(ua, ub)? else {
                    continue;
                };
                if !self.inside_both(&pt) {
                    continue;
                }
                let pt = if self.a.boundary_distance(pt.a) < 1e-4
                    || self.b.boundary_distance(pt.b) < 1e-4
                {
                    // Newton slid this seed onto a trim edge (a clamped seam
                    // crossing is a valid intersection point). Step along the
                    // curve into the regions and trace from there; when no
                    // such step exists the surfaces meet along the edge
                    // itself, which this path cannot author.
                    match self.step_inside(&pt)? {
                        Some(inner) => inner,
                        None => {
                            grazing += 1;
                            continue;
                        }
                    }
                } else {
                    pt
                };
                if covered.iter().any(|c| dist(*c, pt.p) < 2. * self.step) {
                    continue;
                }
                let mut closed = false;
                let (forward, end) = self.march(pt, 1., &mut closed)?;
                let (backward, start) = if closed {
                    (Vec::new(), None)
                } else {
                    let mut c2 = false;
                    self.march(pt, -1., &mut c2)?
                };
                let mut points: Vec<TracePoint> = backward.into_iter().rev().collect();
                points.push(pt);
                points.extend(forward);
                for p in &points {
                    covered.push(p.p);
                }
                chains.push(Chain {
                    points,
                    closed,
                    start,
                    end,
                });
            }
        }
        if chains.is_empty() && grazing > 0 {
            return Err(unsupported(format!(
                "Tolerant Boolean: the surfaces of face {} (first operand) and face {} (second \
                 operand) meet along an existing trim edge (coincident boundaries); such contact \
                 is not regularized numerically",
                self.a.face, self.b.face
            )));
        }
        Ok(chains)
    }
}

// ---------------------------------------------------------------------------
// Cubic B-spline interpolation with a shared parameterization
// ---------------------------------------------------------------------------

/// Chord-length parameters in [0, 1].
fn chord_parameters(points: &[[f64; 3]]) -> Vec<f64> {
    let mut t = vec![0.];
    for w in points.windows(2) {
        t.push(t.last().unwrap() + dist(w[0], w[1]));
    }
    let total = *t.last().unwrap();
    t.iter().map(|x| x / total).collect()
}

/// Clamped cubic B-spline through `data` at parameters `params`, with the
/// averaged knot vector. Rows of `data` may have any dimension.
pub(crate) fn interpolate(params: &[f64], data: &[Vec<f64>]) -> Result<Curve> {
    let n = params.len();
    if n < 2 || data.len() != n {
        return Err(unsupported(
            "Tolerant Boolean: too few points to interpolate",
        ));
    }
    let dim = data[0].len();
    if n == 2 {
        return Ok(Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: data.to_vec(),
            weights: vec![1., 1.],
            periodic: false,
        });
    }
    let degree = 3.min(n - 1);
    let mut knots = vec![0.; degree + 1];
    for j in 1..n - degree {
        let avg = params[j..j + degree].iter().sum::<f64>() / degree as f64;
        knots.push(avg);
    }
    knots.extend(std::iter::repeat_n(1., degree + 1));
    // Piegl & Tiller A2.1 / A2.2: span search and the nonzero basis
    // functions at t.
    let basis_row = |t: f64| -> Vec<f64> {
        let last = n - 1;
        let span = if t >= knots[last + 1] {
            last
        } else {
            let (mut lo, mut hi) = (degree, last + 1);
            let mut mid = (lo + hi) / 2;
            while t < knots[mid] || t >= knots[mid + 1] {
                if t < knots[mid] {
                    hi = mid;
                } else {
                    lo = mid;
                }
                mid = (lo + hi) / 2;
            }
            mid
        };
        let mut nb = vec![0.; degree + 1];
        let mut left = vec![0.; degree + 1];
        let mut right = vec![0.; degree + 1];
        nb[0] = 1.;
        for j in 1..=degree {
            left[j] = t - knots[span + 1 - j];
            right[j] = knots[span + j] - t;
            let mut saved = 0.;
            for r in 0..j {
                let temp = nb[r] / (right[r + 1] + left[j - r]);
                nb[r] = saved + right[r + 1] * temp;
                saved = left[j - r] * temp;
            }
            nb[j] = saved;
        }
        let mut row = vec![0.; n];
        for (k, value) in nb.into_iter().enumerate() {
            row[span - degree + k] = value;
        }
        row
    };
    let matrix: Vec<Vec<f64>> = params.iter().map(|&t| basis_row(t)).collect();
    let mut control = vec![vec![0.; dim]; n];
    for d in 0..dim {
        let rhs: Vec<f64> = data.iter().map(|row| row[d]).collect();
        let x = solve(matrix.clone(), rhs)
            .ok_or_else(|| unsupported("Tolerant Boolean: interpolation system is singular"))?;
        for (i, v) in x.into_iter().enumerate() {
            control[i][d] = v;
        }
    }
    let curve = Curve {
        degree,
        knots,
        control_points: control,
        weights: vec![1.; n],
        periodic: false,
    };
    curve.validate()?;
    Ok(curve)
}

/// A fitted chain: 3D curve plus both pcurves, and the measured deviation.
struct Fitted {
    curve: Curve,
    pcurve_a: Curve,
    pcurve_b: Curve,
    deviation: f64,
}

fn fit(points: &[TracePoint], a: &Chart<'_>, b: &Chart<'_>) -> Result<Fitted> {
    // Consecutive samples closer than the convergence band (a boundary hit
    // reached by creeping, a closing duplicate) would make the chord
    // parameters coincide; keep the first of each such pair.
    let mut points: Vec<TracePoint> = points.to_vec();
    let mut i = 1;
    while i < points.len() {
        if dist(points[i].p, points[i - 1].p) < 1e-9 {
            let last = i + 1 == points.len();
            // Keep the endpoints themselves: drop the interior neighbour.
            points.remove(if last { i - 1 } else { i });
        } else {
            i += 1;
        }
    }
    let points = &points[..];
    let p3: Vec<[f64; 3]> = points.iter().map(|p| p.p).collect();
    let params = chord_parameters(&p3);
    let curve = interpolate(&params, &p3.iter().map(|p| p.to_vec()).collect::<Vec<_>>())?;
    let pcurve_a = interpolate(
        &params,
        &points.iter().map(|p| p.a.to_vec()).collect::<Vec<_>>(),
    )?;
    let pcurve_b = interpolate(
        &params,
        &points.iter().map(|p| p.b.to_vec()).collect::<Vec<_>>(),
    )?;
    // Deviation between samples: the 3D curve against both surfaces through
    // the pcurves, at the validator's own sampling density and finer.
    let mut deviation: f64 = 0.;
    let checks = (points.len() * 3).max(16);
    for i in 0..=checks {
        let t = i as f64 / checks as f64;
        let c = curve.evaluate(t)?.point;
        let c = [c[0], c[1], c[2]];
        let ua = pcurve_point(&pcurve_a, t)?;
        let ub = pcurve_point(&pcurve_b, t)?;
        deviation = deviation
            .max(dist(c, a.point(ua)?))
            .max(dist(c, b.point(ub)?));
    }
    Ok(Fitted {
        curve,
        pcurve_a,
        pcurve_b,
        deviation,
    })
}

// ---------------------------------------------------------------------------
// Point in solid by ray parity against exact surfaces
// ---------------------------------------------------------------------------

struct Solid<'m> {
    charts: Vec<Chart<'m>>,
    scale: f64,
}

impl<'m> Solid<'m> {
    fn new(model: &'m Model) -> Result<Self> {
        let charts = (0..model.faces.len())
            .map(|f| Chart::new(model, f))
            .collect::<Result<Vec<_>>>()?;
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for v in &model.vertices {
            for k in 0..3 {
                lo[k] = lo[k].min(v.point[k]);
                hi[k] = hi[k].max(v.point[k]);
            }
        }
        Ok(Self {
            charts,
            scale: dist(lo, hi).max(1.),
        })
    }

    /// Ray/surface crossings along `origin + λ dir` on one chart: the λ of
    /// every converged crossing inside the trimmed region.
    fn ray_hits(
        &self,
        chart: &Chart<'m>,
        origin: [f64; 3],
        dir: [f64; 3],
    ) -> Result<Option<Vec<f64>>> {
        let mut hits: Vec<f64> = Vec::new();
        let n = 6;
        for i in 0..n {
            for j in 0..n {
                let mut uv = [
                    chart.domain[0][0]
                        + (chart.domain[0][1] - chart.domain[0][0]) * (i as f64 + 0.5) / n as f64,
                    chart.domain[1][0]
                        + (chart.domain[1][1] - chart.domain[1][0]) * (j as f64 + 0.5) / n as f64,
                ];
                let mut lambda = 0.;
                let mut ok = false;
                for _ in 0..40 {
                    let jt = chart.jet(uv)?;
                    let r = sub(jt.p, add(origin, scale(dir, lambda)));
                    if norm(r) <= 1e-10 * self.scale {
                        ok = true;
                        break;
                    }
                    let cols = [jt.du, jt.dv, scale(dir, -1.)];
                    let mut m = vec![vec![0.; 3]; 3];
                    for a in 0..3 {
                        for (k, c) in cols.iter().enumerate() {
                            m[a][k] = c[a];
                        }
                    }
                    let Some(x) = solve(m, scale(r, -1.).to_vec()) else {
                        break;
                    };
                    let next = [uv[0] + x[0], uv[1] + x[1]];
                    // Diverging outside the domain: give up on this seed.
                    let clamped = chart.clamp(next);
                    if dist([next[0], next[1], 0.], [clamped[0], clamped[1], 0.]) > 0.5 {
                        break;
                    }
                    uv = clamped;
                    lambda += x[2];
                }
                if !ok || lambda <= 1e-9 * self.scale {
                    continue;
                }
                if !chart.inside(uv) {
                    continue;
                }
                if chart.boundary_distance(uv) < 1e-6 {
                    // Grazing a trim edge: this direction is not decidable.
                    return Ok(None);
                }
                let jt = chart.jet(uv)?;
                let normal = cross(jt.du, jt.dv);
                if dot(normal, dir).abs() < 1e-6 * norm(normal) {
                    return Ok(None);
                }
                if !hits.iter().any(|h| (h - lambda).abs() <= 1e-8 * self.scale) {
                    hits.push(lambda);
                }
            }
        }
        Ok(Some(hits))
    }

    /// Parity along three fixed directions; the verdicts must agree, which
    /// also rejects points lying on the boundary itself.
    #[expect(
        clippy::approx_constant,
        reason = "ray directions intentionally use varied decimal fixtures"
    )]
    fn contains(&self, p: [f64; 3]) -> Result<bool> {
        let directions = [
            [0.3141592, 0.7182818, 0.6180340],
            [-0.5772157, 0.4142136, 0.7071068],
            [0.8660254, -0.2360680, 0.4472136],
        ];
        let mut verdicts = Vec::new();
        'dirs: for d in directions {
            let dir = unit(d).unwrap();
            let mut parity = 0usize;
            for chart in &self.charts {
                let Some(hits) = self.ray_hits(chart, p, dir)? else {
                    continue 'dirs;
                };
                parity += hits.len();
            }
            verdicts.push(parity % 2 == 1);
        }
        match verdicts.as_slice() {
            [] => Err(unsupported(
                "Tolerant Boolean: point classification is undecidable (rays graze trim edges)",
            )),
            [first, rest @ ..] if rest.iter().all(|v| v == first) => Ok(*first),
            _ => Err(unsupported(
                "Tolerant Boolean: point classification is inconsistent (sample on a boundary)",
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// Boolean
// ---------------------------------------------------------------------------

fn hull(model: &Model, face: usize) -> ([f64; 3], [f64; 3]) {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in model.faces[face].surface.control_points.iter().flatten() {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    (lo, hi)
}

fn hulls_overlap(a: ([f64; 3], [f64; 3]), b: ([f64; 3], [f64; 3]), margin: f64) -> bool {
    (0..3).all(|k| a.0[k] <= b.1[k] + margin && b.0[k] <= a.1[k] + margin)
}

/// Tolerant Boolean of two closed single-shell solids. The result's
/// `tolerance_mm` is the achieved deviation of the intersection curves.
pub fn boolean(a: &Model, b: &Model, operation: &str) -> Result<Model> {
    a.validate()?;
    b.validate()?;
    if !matches!(operation, "union" | "difference" | "intersection") {
        return Err(unsupported(
            "Tolerant Boolean admits union, difference or intersection",
        ));
    }
    for (name, m) in [("first", a), ("second", b)] {
        if m.bodies.len() != 1 || m.shells.len() != 1 || !m.bodies[0].inner_shells.is_empty() {
            return Err(unsupported(format!(
                "Tolerant Boolean needs single-shell solid operands; the {name} operand is not one"
            )));
        }
    }
    let solid_a = Solid::new(a)?;
    let solid_b = Solid::new(b)?;
    let scale = solid_a.scale.max(solid_b.scale);
    let step0 = scale * 0.02;

    // Trace every overlapping face pair, refining the step until the fitted
    // curves meet the target tolerance.
    struct Traced {
        fa: usize,
        fb: usize,
        chain: Chain,
        fitted: Fitted,
    }
    let mut traced: Vec<Traced> = Vec::new();
    for fa in 0..a.faces.len() {
        for fb in 0..b.faces.len() {
            if !hulls_overlap(hull(a, fa), hull(b, fb), 1e-6 * scale) {
                continue;
            }
            let mut step = step0;
            let mut best: Option<Vec<(Chain, Fitted)>> = None;
            for _ in 0..4 {
                let tracer = Tracer {
                    a: Chart::new(a, fa)?,
                    b: Chart::new(b, fb)?,
                    scale,
                    step,
                };
                let chains = tracer.trace_all(12)?;
                let mut fitted = Vec::with_capacity(chains.len());
                let mut worst: f64 = 0.;
                for chain in chains {
                    let mut pts = chain.points.clone();
                    if let Some(h) = chain.start {
                        pts.insert(
                            0,
                            TracePoint {
                                p: h.point,
                                a: h.a,
                                b: h.b,
                            },
                        );
                    }
                    if let Some(h) = chain.end {
                        pts.push(TracePoint {
                            p: h.point,
                            a: h.a,
                            b: h.b,
                        });
                    }
                    if chain.closed {
                        pts.push(pts[0]);
                    }
                    let f = fit(&pts, &tracer.a, &tracer.b)?;
                    worst = worst.max(f.deviation);
                    fitted.push((
                        Chain {
                            points: pts,
                            ..chain
                        },
                        f,
                    ));
                }
                best = Some(fitted);
                if worst <= TARGET {
                    break;
                }
                step *= 0.5;
            }
            for (chain, fitted) in best.unwrap() {
                traced.push(Traced {
                    fa,
                    fb,
                    chain,
                    fitted,
                });
            }
        }
    }
    let tolerance = traced
        .iter()
        .map(|t| t.fitted.deviation)
        .fold(1e-7, f64::max)
        * 2.;
    if tolerance > CEILING {
        return Err(unsupported(format!(
            "Tolerant Boolean: intersection curves could not be fitted within {CEILING} mm \
             (achieved {:.2e} mm)",
            tolerance / 2.
        )));
    }
    // A tolerant result never poses as exact: even a fit far inside the
    // operands' tolerance is reported at the tolerant floor, which callers
    // use to tell the two apart.
    let tolerance = tolerance
        .max(a.tolerance_mm)
        .max(b.tolerance_mm)
        .max(TOLERANT_FLOOR);

    if traced.is_empty() {
        let relation = if solid_b.contains(a.vertices[0].point)? {
            SpatialRelation::BContainsA
        } else if solid_a.contains(b.vertices[0].point)? {
            SpatialRelation::AContainsB
        } else {
            SpatialRelation::Disjoint
        };
        return imprint_pipeline::regularized_empty_algebra(a, b, operation, relation);
    }

    // Network: hit vertices shared through (operand, edge, t), chains as
    // network edges (closed chains split in two at a midpoint vertex).
    let mut asm = Assembler::new(a, b, operation);
    let hit_vertex = |asm: &mut Assembler, h: &BoundaryHit| -> usize {
        if let Some(id) = asm.split_at(h.operand, h.edge, h.t, 1e-7) {
            return id;
        }
        let id = asm.special(h.point);
        asm.attach_edge(id, h.operand, h.edge, h.t);
        id
    };
    // Per face: (net edge, pcurve on that face).
    let mut pieces_a: Vec<Vec<(usize, Curve)>> = vec![Vec::new(); a.faces.len()];
    let mut pieces_b: Vec<Vec<(usize, Curve)>> = vec![Vec::new(); b.faces.len()];
    for t in &traced {
        let chain = &t.chain;
        if chain.closed {
            // Split at the middle sample: two curves over the same points.
            let mid = chain.points.len() / 2;
            let first = &chain.points[..=mid];
            let second = &chain.points[mid..];
            let v0 = asm.special(chain.points[0].p);
            let v1 = asm.special(chain.points[mid].p);
            let charts = (Chart::new(a, t.fa)?, Chart::new(b, t.fb)?);
            for (pts, ends) in [(first, [v0, v1]), (second, [v1, v0])] {
                let f = fit(pts, &charts.0, &charts.1)?;
                let k = asm.push_net([VKey::Special(ends[0]), VKey::Special(ends[1])], f.curve);
                pieces_a[t.fa].push((k, f.pcurve_a));
                pieces_b[t.fb].push((k, f.pcurve_b));
            }
            continue;
        }
        let (Some(start), Some(end)) = (chain.start, chain.end) else {
            return Err(unsupported(
                "Tolerant Boolean: open intersection chain without boundary hits",
            ));
        };
        let v0 = hit_vertex(&mut asm, &start);
        let v1 = hit_vertex(&mut asm, &end);
        let k = asm.push_net(
            [VKey::Special(v0), VKey::Special(v1)],
            t.fitted.curve.clone(),
        );
        pieces_a[t.fa].push((k, t.fitted.pcurve_a.clone()));
        pieces_b[t.fb].push((k, t.fitted.pcurve_b.clone()));
    }

    // Emit every face of both operands through the chart arrangement.
    for (o, model, pieces, other) in [
        (0usize, a, &pieces_a, &solid_b),
        (1usize, b, &pieces_b, &solid_a),
    ] {
        let want = asm.want_inside[o];
        for (face, (_, face_pieces)) in model.faces.iter().zip(pieces.iter()).enumerate() {
            let reversed = face_reversed(model, face);
            let mut chart = ChartVertices::default();
            let mut list: Vec<Piece<EKey>> = Vec::new();
            for loop_pieces in asm.boundary_loops(o, face)? {
                for (ka, kb, pcurve, key, rev) in loop_pieces {
                    let ua = pcurve_point(&pcurve, 0.)?;
                    let ub = pcurve_point(&pcurve, 1.)?;
                    let ia = chart.index(asm.chart_key(ka), ua);
                    let ib = chart.index(asm.chart_key(kb), ub);
                    list.push(Piece {
                        v: [ia, ib],
                        pcurve,
                        key,
                        reversed: rev,
                    });
                }
            }
            for (k, pcurve) in face_pieces {
                let mut pcurve = pcurve.clone();
                let ends = asm.net[*k].ends;
                let n = pcurve.control_points.len();
                let ua = pcurve_point(&pcurve, 0.)?;
                let ub = pcurve_point(&pcurve, 1.)?;
                let ia = chart.index(asm.chart_key(ends[0]), ua);
                let ib = chart.index(asm.chart_key(ends[1]), ub);
                pcurve.control_points[0] = chart.uvs[ia].to_vec();
                pcurve.control_points[n - 1] = chart.uvs[ib].to_vec();
                list.push(Piece {
                    v: [ia, ib],
                    pcurve,
                    key: EKey::Net(*k),
                    reversed: false,
                });
            }
            let keep = |p: [f64; 3]| -> Result<bool> { Ok(other.contains(p)? == want) };
            asm.emit_chart(o, face, reversed, list, chart.len(), &keep)?;
        }
    }
    asm.finish(tolerance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cuboid, cylinder, sphere, transform};

    #[test]
    fn dense_solver_handles_pivoting_and_singularity() {
        let m = vec![vec![0., 2., 1.], vec![1., 1., 1.], vec![2., 0., 3.]];
        let x = solve(m, vec![5., 6., 13.]).unwrap();
        for (got, want) in x.iter().zip([2., 1., 3.]) {
            assert!((got - want).abs() < 1e-12);
        }
        assert!(solve(vec![vec![1., 2.], vec![2., 4.]], vec![1., 2.]).is_none());
        assert!(solve(vec![vec![0., 0.], vec![0., 0.]], vec![0., 0.]).is_none());
    }

    #[test]
    fn segment_intersection_and_distance() {
        let hit = segments_intersect([0., 0.], [2., 2.], [0., 2.], [2., 0.]).unwrap();
        assert!((hit.0 - 0.5).abs() < 1e-12 && (hit.1 - 0.5).abs() < 1e-12);
        assert!(segments_intersect([0., 0.], [1., 0.], [0., 1.], [1., 1.]).is_none());
        assert!(segments_intersect([0., 0.], [1., 1.], [2., 2.], [3., 3.]).is_none());
        assert!((segment_distance([0., 0.], [2., 0.], [1., 1.]) - 1.).abs() < 1e-12);
        assert!((segment_distance([0., 0.], [2., 0.], [3., 0.]) - 1.).abs() < 1e-12);
        assert!((segment_distance([1., 1.], [1., 1.], [1., 4.]) - 3.).abs() < 1e-12);
    }

    #[test]
    fn chord_parameters_are_normalized_and_monotone() {
        let t = chord_parameters(&[[0., 0., 0.], [1., 0., 0.], [1., 2., 0.], [1., 2., 3.]]);
        assert_eq!(t[0], 0.);
        assert_eq!(*t.last().unwrap(), 1.);
        assert!(t.windows(2).all(|w| w[1] > w[0]));
        assert!((t[1] - 1. / 6.).abs() < 1e-12 && (t[2] - 0.5).abs() < 1e-12);
    }

    #[test]
    fn interpolation_reproduces_samples_and_partitions_unity() {
        let pts: Vec<Vec<f64>> = (0..9)
            .map(|i| {
                let t = i as f64 / 8. * std::f64::consts::PI;
                vec![t.cos(), t.sin(), t * 0.1]
            })
            .collect();
        let params: Vec<f64> = (0..9).map(|i| i as f64 / 8.).collect();
        let curve = interpolate(&params, &pts).unwrap();
        assert_eq!(curve.degree, 3);
        curve.validate().unwrap();
        for (t, p) in params.iter().zip(&pts) {
            let q = curve.evaluate(*t).unwrap().point;
            for k in 0..3 {
                assert!((q[k] - p[k]).abs() < 1e-9, "sample {t} not reproduced");
            }
        }
        // Two points: a straight line; three: a quadratic through them.
        let line = interpolate(&[0., 1.], &[vec![0., 0.], vec![2., 4.]]).unwrap();
        assert_eq!(line.degree, 1);
        let mid = line.evaluate(0.5).unwrap().point;
        assert!((mid[0] - 1.).abs() < 1e-12 && (mid[1] - 2.).abs() < 1e-12);
        let quad =
            interpolate(&[0., 0.5, 1.], &[vec![0., 0.], vec![1., 1.], vec![2., 0.]]).unwrap();
        assert_eq!(quad.degree, 2);
        let p = quad.evaluate(0.5).unwrap().point;
        assert!((p[0] - 1.).abs() < 1e-12 && (p[1] - 1.).abs() < 1e-12);
        assert!(interpolate(&[0.], &[vec![0., 0.]]).is_err());
        let flat = vec![vec![0., 0.]; 3];
        assert!(
            interpolate(&[0., 0., 1.], &flat).is_err(),
            "coincident parameters are singular"
        );
    }

    #[test]
    fn chart_membership_on_a_box_face_and_a_sphere_patch() {
        let b = cuboid([0., 0., 0.], [2., 3., 4.]).unwrap();
        let chart = Chart::new(&b, 0).unwrap();
        assert!(chart.inside([0.5, 0.5]));
        assert!(!chart.inside([1.5, 0.5]));
        assert!(!chart.inside([0.5, -0.1]));
        assert!((chart.boundary_distance([0.5, 0.25]) - 0.25).abs() < 1e-9);
        assert!(chart.crossing([0.5, 0.5], [0.5, 1.5]).is_some());
        assert!(chart.crossing([0.2, 0.2], [0.4, 0.4]).is_none());
        let s = sphere(2.).unwrap();
        let patch = Chart::new(&s, 0).unwrap();
        assert!(patch.inside([0.3, 0.3]));
        assert!(!patch.inside([0.8, 0.8]), "outside the quarter disk");
        assert!(patch.boundary_distance([0.1, 0.1]) <= 0.1 + 1e-9);
        let (li, ci, local) = patch.crossing([0.3, 0.3], [0.9, 0.9]).unwrap();
        assert_eq!(li, 0);
        assert!(ci < 3 && (0.0..=1.0).contains(&local));
        let jet = patch.jet([0.3, 0.3]).unwrap();
        assert!(
            (norm(jet.p) - 2.).abs() < 1e-12,
            "jet point lies on the sphere"
        );
        assert!(norm(cross(jet.du, jet.dv)) > 0.);
    }

    #[test]
    fn projection_and_direction_on_a_known_intersection() {
        // Cylinder wall (face 0) against an off-axis sphere: every converged
        // point lies on both surfaces and the march direction is orthogonal
        // to both normals.
        let c = cylinder(3., 10.).unwrap();
        let s = placed(sphere(2.).unwrap(), [2.5, 0.4, 5.]);
        let tracer = Tracer {
            a: Chart::new(&c, 0).unwrap(),
            b: Chart::new(&s, 0).unwrap(),
            scale: 10.,
            step: 0.2,
        };
        let pt = tracer
            .project([0.5, 0.5], [0.3, 0.3])
            .unwrap()
            .expect("converges");
        let pa = tracer.a.point(pt.a).unwrap();
        let pb = tracer.b.point(pt.b).unwrap();
        assert!(dist(pa, pb) < 1e-9);
        assert!((pa[0].hypot(pa[1]) - 3.).abs() < 1e-9, "on the wall");
        assert!(
            (dist(pa, [2.5, 0.4, 5.]) - 2.).abs() < 1e-9,
            "on the sphere"
        );
        let dir = tracer.direction(&pt).unwrap().expect("transversal");
        let ja = tracer.a.jet(pt.a).unwrap();
        let jb = tracer.b.jet(pt.b).unwrap();
        assert!(dot(dir, cross(ja.du, ja.dv)).abs() < 1e-9);
        assert!(dot(dir, cross(jb.du, jb.dv)).abs() < 1e-9);
        assert!((norm(dir) - 1.).abs() < 1e-12);
        // The corrector lands on the plane through the target.
        let target = add(pt.p, scale(dir, 0.1));
        let next = tracer
            .correct(pt.a, pt.b, target, dir)
            .unwrap()
            .expect("corrects");
        assert!(dot(sub(next.p, target), dir).abs() < 1e-9);
        assert!((dist(next.p, [2.5, 0.4, 5.]) - 2.).abs() < 1e-9);
        assert!((next.p[0].hypot(next.p[1]) - 3.).abs() < 1e-9);
    }

    #[test]
    fn marching_closes_a_loop_that_stays_inside_both_faces() {
        // A box face whose normal is the octant diagonal cuts a unit sphere
        // 0.9 mm from its centre: the cap is a ring inside one sphere patch
        // and inside the face, so the pair yields exactly one closed chain.
        let n = [1. / 3f64.sqrt(); 3];
        let u = [1. / 2f64.sqrt(), -1. / 2f64.sqrt(), 0.];
        let v = [
            n[1] * u[2] - n[2] * u[1],
            n[2] * u[0] - n[0] * u[2],
            n[0] * u[1] - n[1] * u[0],
        ];
        let t = 0.9 - 50.;
        let b = transform::affine(
            &cuboid([-50., -50., -50.], [50., 50., 50.]).unwrap(),
            [
                [u[0], v[0], n[0], t * n[0]],
                [u[1], v[1], n[1], t * n[1]],
                [u[2], v[2], n[2], t * n[2]],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let s = sphere(1.).unwrap();
        // The face at the top of the tilted box: its centre is nearest the
        // sphere.
        let face = (0..6)
            .min_by(|&i, &j| {
                let c = |f: usize| {
                    let p = b.faces[f].surface.evaluate(0.5, 0.5).unwrap().point;
                    norm([p[0], p[1], p[2]])
                };
                c(i).total_cmp(&c(j))
            })
            .unwrap();
        let tracer = Tracer {
            a: Chart::new(&b, face).unwrap(),
            b: Chart::new(&s, 0).unwrap(),
            scale: 100.,
            step: 0.05,
        };
        let chains = tracer.trace_all(12).unwrap();
        assert_eq!(chains.len(), 1, "one ring expected");
        assert!(chains[0].closed);
        assert!(chains[0].start.is_none() && chains[0].end.is_none());
        let rho = (1f64 - 0.81).sqrt();
        for p in &chains[0].points {
            assert!((norm(p.p) - 1.).abs() < 1e-9, "on the sphere");
            assert!((dot(p.p, n) - 0.9).abs() < 1e-9, "on the face plane");
            let radial = sub(p.p, scale(n, 0.9));
            assert!((norm(radial) - rho).abs() < 1e-9, "on the section circle");
        }
        let mut pts = chains[0].points.clone();
        pts.push(pts[0]);
        let f = fit(&pts, &tracer.a, &tracer.b).unwrap();
        assert!(f.deviation < 1e-4, "deviation {:e}", f.deviation);
    }

    #[test]
    fn marching_exits_through_trim_edges_with_shared_hits() {
        let c = cylinder(3., 10.).unwrap();
        let s = placed(sphere(2.).unwrap(), [2.5, 0.4, 5.]);
        let tracer = Tracer {
            a: Chart::new(&c, 0).unwrap(),
            b: Chart::new(&s, 0).unwrap(),
            scale: 10.,
            step: 0.2,
        };
        let chains = tracer.trace_all(12).unwrap();
        assert!(!chains.is_empty());
        for chain in &chains {
            assert!(!chain.closed);
            let (start, end) = (chain.start.unwrap(), chain.end.unwrap());
            for hit in [start, end] {
                let model = if hit.operand == 0 { &c } else { &s };
                let on_edge = model.edges[hit.edge].curve.evaluate(hit.t).unwrap().point;
                assert!(dist([on_edge[0], on_edge[1], on_edge[2]], hit.point) < 1e-9);
                assert!((hit.point[0].hypot(hit.point[1]) - 3.).abs() < 1e-8);
                assert!((dist(hit.point, [2.5, 0.4, 5.]) - 2.).abs() < 1e-8);
            }
        }
    }

    #[test]
    fn ray_parity_classifies_points_against_exact_solids() {
        let sphere_model = sphere(2.).unwrap();
        let s = Solid::new(&sphere_model).unwrap();
        assert!(s.contains([0.3, -0.2, 0.1]).unwrap());
        assert!(!s.contains([2.5, 0., 0.]).unwrap());
        assert!(!s.contains([0., 0., -7.]).unwrap());
        let cylinder_model = cylinder(3., 10.).unwrap();
        let c = Solid::new(&cylinder_model).unwrap();
        assert!(c.contains([1., 1., 5.]).unwrap());
        assert!(!c.contains([1., 1., 11.]).unwrap());
        assert!(!c.contains([3.5, 0., 5.]).unwrap());
        let box_model = cuboid([0., 0., 0.], [1., 2., 3.]).unwrap();
        let b = Solid::new(&box_model).unwrap();
        assert!(b.contains([0.5, 1., 1.5]).unwrap());
        assert!(!b.contains([-0.5, 1., 1.5]).unwrap());
        // A point on the boundary is inconsistent across rays and refuses
        // rather than guessing.
        assert!(b.contains([0., 1., 1.5]).is_err());
    }

    #[test]
    fn hull_overlap_prefilter() {
        let a = ([0., 0., 0.], [1., 1., 1.]);
        let b = ([1.5, 0., 0.], [2., 1., 1.]);
        assert!(!hulls_overlap(a, b, 0.));
        assert!(hulls_overlap(a, b, 0.6));
        assert!(hulls_overlap(a, ([0.5, 0.5, 0.5], [3., 3., 3.]), 0.));
    }

    fn placed(model: Model, at: [f64; 3]) -> Model {
        transform::affine(
            &model,
            [
                [1., 0., 0., at[0]],
                [0., 1., 0., at[1]],
                [0., 0., 1., at[2]],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap()
    }
    fn rotated_x(model: Model, degrees: f64) -> Model {
        let (s, c) = degrees.to_radians().sin_cos();
        transform::affine(
            &model,
            [
                [1., 0., 0., 0.],
                [0., c, -s, 0.],
                [0., s, c, 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap()
    }

    #[test]
    fn off_axis_sphere_through_a_cylinder_wall() {
        let c = cylinder(3., 10.).unwrap();
        let s = placed(sphere(2.).unwrap(), [2.5, 0.4, 5.]);
        for op in ["union", "difference", "intersection"] {
            let model = boolean(&c, &s, op).unwrap();
            model.validate().unwrap();
            assert!(model.tolerance_mm <= CEILING, "{op}");
            assert!(model.tolerance_mm > 1e-7, "{op} carries its tolerance");
            assert_eq!(model.bodies.len(), 1, "{op}");
        }
    }

    #[test]
    fn crossing_cylinders() {
        let a = cylinder(2., 12.).unwrap();
        let b = placed(rotated_x(cylinder(1.2, 12.).unwrap(), 90.), [0.3, 6., 6.]);
        for op in ["union", "difference", "intersection"] {
            let model = boolean(&a, &b, op).unwrap();
            model.validate().unwrap();
            assert_eq!(model.bodies.len(), 1, "{op}");
        }
    }

    #[test]
    fn box_corner_into_a_sphere_off_axis_is_not_needed_but_works() {
        let b = cuboid([-4., -4., -4.], [4., 4., 4.]).unwrap();
        let s = placed(sphere(3.).unwrap(), [3.1, 2.7, 2.9]);
        let model = boolean(&b, &s, "difference").unwrap();
        model.validate().unwrap();
    }

    #[test]
    fn the_public_boolean_falls_back_after_an_exact_refusal() {
        let c = cylinder(3., 10.).unwrap();
        let s = placed(sphere(2.).unwrap(), [2.5, 0.4, 5.]);
        let model = crate::boolean(&c, &s, "difference").unwrap();
        assert!(model.tolerance_mm > 1e-7 && model.tolerance_mm <= CEILING);
        // Exact pairs stay exact: the axial sphere keeps the kernel tolerance.
        let axial = placed(sphere(4.).unwrap(), [0., 0., 5.]);
        let exact = crate::boolean(&c, &axial, "difference").unwrap();
        assert!(exact.tolerance_mm <= 1e-6);
        // Hard errors are not masked by the fallback.
        let err = crate::boolean(&c, &s, "xor").unwrap_err();
        assert!(err.message.contains("xor") || err.code != "BREP_INVALID_OPERATION");
    }

    #[test]
    fn torus_through_a_box_corner_region() {
        let b = cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
        let t = placed(crate::torus(4., 1.).unwrap(), [0.3, 0.7, 0.4]);
        for op in ["union", "difference", "intersection"] {
            let model = crate::boolean(&b, &t, op).unwrap();
            model.validate().unwrap();
            assert!(model.tolerance_mm > 1e-6, "{op}");
        }
        // The torus at the origin meets the box exactly along its own seam
        // circles (the planes x = 0, y = 0, z = 0 are torus patch seams):
        // coincident boundaries refuse instead of guessing a containment.
        let seam = crate::torus(4., 1.).unwrap();
        let err = crate::boolean(&b, &seam, "union").unwrap_err();
        assert!(err.message.contains("coincident"), "{}", err.message);
    }

    #[test]
    fn disjoint_pairs_resolve_by_empty_algebra() {
        let c = cylinder(3., 10.).unwrap();
        let s = placed(sphere(2.).unwrap(), [20., 0., 5.]);
        assert_eq!(boolean(&c, &s, "union").unwrap().bodies.len(), 2);
        assert!(boolean(&c, &s, "intersection").unwrap().is_empty());
    }
}
