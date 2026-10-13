use super::*;

impl<'m> Tracer<'m> {
    /// Gauss-Newton on `Pa(u,v) - Pb(s,t) = 0` (4 unknowns, 3 residuals,
    /// minimum-norm update), from a start; returns the converged point.
    pub(super) fn project(&self, mut ua: [f64; 2], mut ub: [f64; 2]) -> Result<Option<TracePoint>> {
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
    pub(super) fn direction(&self, pt: &TracePoint) -> Result<Option<[f64; 3]>> {
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
    pub(super) fn correct(
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
    pub(super) fn trace_all(&self, seeds_per_axis: usize) -> Result<Vec<Chain>> {
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
pub(super) fn chord_parameters(points: &[[f64; 3]]) -> Vec<f64> {
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
pub(super) struct Fitted {
    pub(super) curve: Curve,
    pub(super) pcurve_a: Curve,
    pub(super) pcurve_b: Curve,
    pub(super) deviation: f64,
}
