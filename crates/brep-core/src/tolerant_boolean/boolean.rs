use super::*;

pub(super) fn fit(points: &[TracePoint], a: &Chart<'_>, b: &Chart<'_>) -> Result<Fitted> {
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

pub(super) struct Solid<'m> {
    charts: Vec<Chart<'m>>,
    scale: f64,
}

impl<'m> Solid<'m> {
    pub(super) fn new(model: &'m Model) -> Result<Self> {
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
    pub(super) fn contains(&self, p: [f64; 3]) -> Result<bool> {
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

pub(super) fn hulls_overlap(a: ([f64; 3], [f64; 3]), b: ([f64; 3], [f64; 3]), margin: f64) -> bool {
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
    let mut asm = Assembler::new(a, b, operation)?;
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
