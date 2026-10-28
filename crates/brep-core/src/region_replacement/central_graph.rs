//! Six-sided Hermite graph proposals. Sampling builds a candidate only;
//! original-curve interval inspection is the separate admission authority.
use super::CornerCollars;
use nurbs_core::{
    Error, Result,
    curve::{Curve, basis},
    surface::Surface,
};
#[path = "central_graph/constrained.rs"]
mod constrained;
#[path = "central_graph/minimum_norm.rs"]
mod minimum_norm;
#[path = "central_graph/parameters.rs"]
mod parameters;
fn refuse(s: &str) -> Error {
    Error::new("BREP_CENTRAL_GRAPH_UNPROVEN", s)
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| a[k] * b[k]).sum()
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|k| a[k] - b[k])
}
fn point(p: &[f64]) -> [f64; 3] {
    [p[0], p[1], p[2]]
}
#[derive(Clone, Copy)]
pub struct GraphOptions {
    /// Orthonormal world axes: chart x, chart y, graph height.
    pub frame: [[f64; 3]; 3],
    pub degree: usize,
    /// Uniform intervals per axis, or per nonlinear boundary when aligned.
    pub spans: usize,
    /// Add degree-2 multiplicity knots at projected boundary junctions.
    pub align_junctions: bool,
    /// Additional simple knots around each junction; zero disables refinement.
    pub junction_refinement: f64,
    pub stations_per_side: usize,
    /// Proposal conditioning only; never an admission tolerance.
    pub regularization: f64,
}
pub struct CentralGraph {
    pub surface: Surface,
    /// Same oriented source-owned six curves projected into this chart.
    pub pcurves: Vec<Curve>,
    pub sampled_jet_residual: [f64; 6],
}
/// A soft interior fairing constraint. It supplies no geometry, material or
/// G2 certificate; the resulting stored candidate must be freshly inspected.
#[derive(Clone, Copy)]
pub struct GraphGuide {
    pub point: [f64; 3],
    pub weight: f64,
}
struct Chart {
    frame: [[f64; 3]; 3],
    origin: [f64; 3],
    low: [f64; 2],
    span: [f64; 2],
}
impl Chart {
    fn uv(&self, p: [f64; 3]) -> [f64; 2] {
        std::array::from_fn(|k| {
            (dot(sub(p, self.origin), self.frame[k]) - self.low[k]) / self.span[k]
        })
    }
    fn jets(&self, s: &Surface, uv: [f64; 2]) -> Result<([f64; 2], [f64; 6])> {
        let e = s.evaluate(uv[0], uv[1])?;
        let (a, b) = e
            .first_derivatives()
            .ok_or_else(|| refuse("Missing original support tangents"))?;
        let second = e
            .second_derivatives()
            .ok_or_else(|| refuse("Missing original support curvature"))?;
        let j: [[f64; 2]; 2] = std::array::from_fn(|k| {
            [
                dot(a, self.frame[k]) / self.span[k],
                dot(b, self.frame[k]) / self.span[k],
            ]
        });
        let det = j[0][0] * j[1][1] - j[0][1] * j[1][0];
        let scale = j.iter().flatten().map(|x| x.abs()).fold(0_f64, f64::max);
        if !det.is_finite() || det.abs() <= 1e-10 * scale * scale {
            return Err(refuse(
                "Original support is not a regular graph in supplied frame",
            ));
        }
        let inv = [
            [j[1][1] / det, -j[0][1] / det],
            [-j[1][0] / det, j[0][0] / det],
        ];
        let height_first = [dot(a, self.frame[2]), dot(b, self.frame[2])];
        let g: [f64; 2] =
            std::array::from_fn(|k| (0..2).map(|l| height_first[l] * inv[l][k]).sum());
        let r: [f64; 3] = [second.0, second.1, second.2].map(|d| {
            dot(d, self.frame[2])
                - g[0] * dot(d, self.frame[0]) / self.span[0]
                - g[1] * dot(d, self.frame[1]) / self.span[1]
        });
        let h = |a: usize, b: usize| {
            inv[0][a] * r[0] * inv[0][b]
                + (inv[0][a] * inv[1][b] + inv[1][a] * inv[0][b]) * r[1]
                + inv[1][a] * r[2] * inv[1][b]
        };
        Ok((
            self.uv(e.point),
            [
                dot(sub(e.point, self.origin), self.frame[2]),
                g[0],
                g[1],
                h(0, 0),
                h(0, 1),
                h(1, 1),
            ],
        ))
    }
}
// Column-pivoted Householder QR, not normal equations. The small diagonal
// regularizer chooses unconstrained interior modes; no geometric proof uses it.
fn least_squares(a: Vec<Vec<f64>>, b: Vec<f64>, extra: &[Vec<(usize, f64)>]) -> Result<Vec<f64>> {
    least_squares_constrained(a, b, extra, &[])
}
fn least_squares_constrained(
    mut a: Vec<Vec<f64>>,
    mut b: Vec<f64>,
    extra: &[Vec<(usize, f64)>],
    constraints: &[(Vec<f64>, f64)],
) -> Result<Vec<f64>> {
    let m = a.len();
    let n = a[0].len();
    if constraints.is_empty() && b.iter().all(|&x| x == 0.) {
        return Ok(vec![0.; n]);
    }
    // Normalize columns before rank-revealing QR. Clustered knots otherwise
    // mix tiny value columns with enormous second-derivative columns.
    let column_scale: Vec<f64> = (0..n)
        .map(|j| a.iter().map(|r| r[j] * r[j]).sum::<f64>().sqrt())
        .collect();
    if column_scale.iter().any(|&s| !s.is_finite() || s == 0.) {
        return Err(refuse("Invalid Hermite column scale"));
    }
    for row in &mut a {
        for j in 0..n {
            row[j] /= column_scale[j];
        }
    }
    let mut rank = n;
    let mut order: Vec<usize> = (0..n).collect();
    for k in 0..n {
        // Accumulate every remaining column norm in one row-major pass.
        // Recomputing two strided column scans for each pivot comparison
        // made large fairing fits spend minutes on cache misses alone.
        let mut norms = vec![0.; n - k];
        for row in a.iter().skip(k) {
            for (sum, &x) in norms.iter_mut().zip(&row[k..]) {
                *sum += x * x;
            }
        }
        let col = k
            + (0..norms.len())
                .max_by(|&i, &j| norms[i].total_cmp(&norms[j]))
                .unwrap();
        let norm = norms[col - k].sqrt();
        for row in &mut a {
            row.swap(k, col);
        }
        order.swap(k, col);
        if !norm.is_finite() {
            return Err(refuse(
                "Hermite fitting system is numerically rank deficient",
            ));
        }
        if norm <= 1e-13 && extra.is_empty() && constraints.is_empty() {
            rank = k;
            break;
        }
        let alpha = -norm.copysign(a[k][k]);
        let mut v: Vec<f64> = (k..m).map(|i| a[i][k]).collect();
        v[0] -= alpha;
        let beta = 2. / v.iter().map(|x| x * x).sum::<f64>();
        // Each column dot retains ascending row order; only independent
        // columns are interleaved. Apply the same Householder reflector in
        // contiguous rows. This is still pivoted QR, not normal equations.
        let mut dots = vec![0.; n - k - 1];
        for (i, row) in a.iter().enumerate().skip(k) {
            for (sum, &x) in dots.iter_mut().zip(&row[k + 1..]) {
                *sum += v[i - k] * x;
            }
        }
        for d in &mut dots {
            *d *= beta;
        }
        for (i, row) in a.iter_mut().enumerate().skip(k) {
            for (x, &d) in row[k + 1..].iter_mut().zip(&dots) {
                *x -= d * v[i - k];
            }
        }
        let d = beta * (k..m).map(|i| v[i - k] * b[i]).sum::<f64>();
        for i in k..m {
            b[i] -= d * v[i - k];
        }
        a[k][k] = alpha;
        for row in a.iter_mut().skip(k + 1) {
            row[k] = 0.;
        }
    }
    // Append sparse fairing rows to the completed QR factor, rather than
    // expanding thousands of five-entry rows into another dense matrix.
    // Positive original Tikhonov rows retain full rank for this path.
    for row in extra {
        let mut values = vec![0.; n];
        let mut inverse_order = vec![0; n];
        for (j, &original) in order.iter().enumerate() {
            inverse_order[original] = j;
        }
        for &(j, x) in row {
            values[inverse_order[j]] = x / column_scale[j];
        }
        let mut rhs = 0.;
        for j in 0..n {
            if values[j] == 0. {
                continue;
            }
            let radius = a[j][j].hypot(values[j]);
            if !radius.is_finite() || radius == 0. {
                return Err(refuse("Sparse fairing QR rotation is singular"));
            }
            let c = a[j][j] / radius;
            let s = values[j] / radius;
            a[j][j] = radius;
            values[j] = 0.;
            for k in j + 1..n {
                let x = a[j][k];
                let y = values[k];
                a[j][k] = c * x + s * y;
                values[k] = -s * x + c * y;
            }
            let x = b[j];
            b[j] = c * x + s * rhs;
            rhs = -s * x + c * rhs;
        }
    }
    if rank < n {
        return minimum_norm::complete(&a, &b, &order, &column_scale, rank);
    }
    let mut x = vec![0.; n];
    for i in (0..rank).rev() {
        x[i] = (b[i] - (i + 1..rank).map(|j| a[i][j] * x[j]).sum::<f64>()) / a[i][i];
    }
    constrained::correct(&a, &order, &column_scale, &mut x, constraints)?;
    let mut out = vec![0.; n];
    for i in 0..n {
        out[order[i]] = x[i] / column_scale[order[i]];
    }
    if out.iter().any(|x| !x.is_finite()) {
        return Err(refuse("Hermite fit exceeded finite range"));
    }
    Ok(out)
}
fn rows(degree: usize, knots: [&[f64]; 2], uv: [f64; 2]) -> Result<[Vec<(usize, f64)>; 6]> {
    let u = basis(degree, knots[0], knots[0].len() - degree - 1, uv[0], false)?;
    let v = basis(degree, knots[1], knots[1].len() - degree - 1, uv[1], false)?;
    let fields = [
        (&u.basis, &v.basis),
        (&u.d1, &v.basis),
        (&u.basis, &v.d1),
        (&u.d2, &v.basis),
        (&u.d1, &v.d1),
        (&u.basis, &v.d2),
    ];
    Ok(fields.map(|(a, b)| {
        a.iter()
            .enumerate()
            .filter(|(_, x)| **x != 0.)
            .flat_map(|(i, x)| {
                b.iter()
                    .enumerate()
                    .filter(|(_, y)| **y != 0.)
                    .filter_map(move |(j, y)| {
                        let value = x * y;
                        (value != 0.).then_some((i * b.len() + j, value))
                    })
            })
            .collect()
    }))
}
/// Build one regular graph chart covering the six-sided rim. The returned
/// surface is an UNADMITTED numerical proposal. Neither fitting residuals nor
/// the input collar receipts authorize B-rep replacement.
pub fn propose_central_graph(collars: &CornerCollars, o: GraphOptions) -> Result<CentralGraph> {
    propose_central_graph_guided(collars, o, &[])
}
/// Interior guides select free fitting modes without replacing any of the
/// six original position/gradient/Hessian constraints. They are proposals,
/// never an excuse to increase seam tolerances or skip material checks.
pub fn propose_central_graph_guided(
    collars: &CornerCollars,
    o: GraphOptions,
    guides: &[GraphGuide],
) -> Result<CentralGraph> {
    propose_central_graph_fair(collars, o, guides, 0.)
}
/// Add a soft scale-aware control-net Laplacian objective. Every interior
/// pole participates; a single quadrature site per knot cell would miss
/// oscillatory free modes. This does not certify material embedding or
/// allow any boundary tolerance to change. A zero gain keeps the old fit.
pub fn propose_central_graph_fair(
    collars: &CornerCollars,
    o: GraphOptions,
    guides: &[GraphGuide],
    fairness: f64,
) -> Result<CentralGraph> {
    propose_central_graph_refined(collars, o, guides, fairness, [&[], &[]])
}
/// Bounded local knot refinement of the proposal chart. Multiplicities retain
/// at least C2; neither added degrees of freedom nor local guides grant proof.
pub fn propose_central_graph_refined(
    collars: &CornerCollars,
    o: GraphOptions,
    guides: &[GraphGuide],
    fairness: f64,
    extra: [&[(f64, usize)]; 2],
) -> Result<CentralGraph> {
    propose_central_graph_constrained(collars, o, guides, fairness, extra, &[])
}
/// Hard world-point interpolation constraints, capped at 32. These select
/// fitting modes without raising soft weights. A successful solve is only a
/// proposal: all stored seams, material bounds and topology need fresh proofs.
pub fn propose_central_graph_constrained(
    collars: &CornerCollars,
    o: GraphOptions,
    guides: &[GraphGuide],
    fairness: f64,
    extra: [&[(f64, usize)]; 2],
    anchors: &[[f64; 3]],
) -> Result<CentralGraph> {
    if anchors.len() > 32 || anchors.iter().flatten().any(|x| !x.is_finite()) {
        return Err(refuse("Invalid or excessive hard graph anchors"));
    }
    if collars.boundaries.len() != 6
        || extra.iter().any(|axis| {
            axis.len() > 6
                || axis.iter().any(|&(t, m)| {
                    !t.is_finite() || t <= 0. || t >= 1. || m == 0 || m > o.degree.saturating_sub(2)
                })
        })
        || !(fairness == 0. || (fairness.is_finite() && (1e-12..=1e-3).contains(&fairness)))
        || guides.len() > 256
        || guides.iter().any(|g| {
            g.point.iter().any(|x| !x.is_finite())
                || !g.weight.is_finite()
                || !(1e-8..=1.).contains(&g.weight)
        })
        || !(3..=24).contains(&o.degree)
        || !(o.junction_refinement == 0. || (1e-5..=0.02).contains(&o.junction_refinement))
        || !(1..=32).contains(&o.spans)
        || o.degree + o.spans > 64
        || !(9..=129).contains(&o.stations_per_side)
        || !o.regularization.is_finite()
        || !(1e-14..=1e-3).contains(&o.regularization)
    {
        return Err(refuse("Need six boundaries and bounded graph fitting work"));
    }
    for i in 0..3 {
        for j in 0..3 {
            if !dot(o.frame[i], o.frame[j]).is_finite()
                || (dot(o.frame[i], o.frame[j]) - if i == j { 1. } else { 0. }).abs() > 1e-12
            {
                return Err(refuse("Graph frame must be finite and orthonormal"));
            }
        }
    }
    for i in 0..6 {
        let a = &collars.boundaries[i].curve;
        let b = &collars.boundaries[(i + 1) % 6].curve;
        a.validate()?;
        if a.control_points[0].len() != 3 || a.control_points.last() != b.control_points.first() {
            return Err(refuse(
                "Six original boundary curves must form a literal oriented cycle",
            ));
        }
    }
    let origin = point(&collars.boundaries[0].curve.control_points[0]);
    let mut chart = Chart {
        frame: o.frame,
        origin,
        low: [f64::INFINITY; 2],
        span: [0.; 2],
    };
    let mut high = [f64::NEG_INFINITY; 2];
    for c in &collars.boundaries {
        for p in &c.curve.control_points {
            for k in 0..2 {
                let x = dot(sub(point(p), origin), o.frame[k]);
                chart.low[k] = chart.low[k].min(x);
                high[k] = high[k].max(x);
            }
        }
    }
    for k in 0..2 {
        let d = high[k] - chart.low[k];
        if !d.is_finite() || d <= 0. {
            return Err(refuse("Collapsed graph projection"));
        }
        chart.low[k] -= 0.05 * d;
        chart.span[k] = 1.1 * d;
    }
    let axis_knots = |axis: usize| -> Result<Vec<f64>> {
        let mut nodes: Vec<(f64, usize)> = Vec::new();
        nodes.extend(extra[axis].iter().copied());
        if o.align_junctions {
            for c in &collars.boundaries {
                nodes.push((
                    chart.uv(point(&c.curve.control_points[0]))[axis],
                    o.degree - 2,
                ));
                if o.junction_refinement > 0. {
                    let vertex = chart.uv(point(&c.curve.control_points[0]))[axis];
                    for sign in [-1., 1.] {
                        let t = vertex + sign * o.junction_refinement;
                        if 0. < t && t < 1. {
                            nodes.push((t, 1));
                        }
                    }
                }

                // Refine the projected nonlinear support sections, rather
                // than spending most controls in the large planar gaps.
                // This proposes knots only; admission still inspects every
                // original seam and the actual saved surface.
                if c.original_surface.degree_u > 1 || c.original_surface.degree_v > 1 {
                    let d = c.curve.domain();
                    for i in 1..o.spans {
                        let p = c
                            .curve
                            .evaluate(d[0] + (d[1] - d[0]) * i as f64 / o.spans as f64)?
                            .point;
                        nodes.push((chart.uv(point(&p))[axis], 1));
                    }
                }
            }
        } else {
            nodes.extend((1..o.spans).map(|i| (i as f64 / o.spans as f64, 1)));
        }
        nodes.sort_by(|a, b| a.0.total_cmp(&b.0));
        // Nearby projected stations can differ only by the original root
        // rounding, creating artificial 1e-10-wide knot cells. Coarsen the
        // proposal mesh, never the six owned curves or their pcurves. All
        // original geometry is still inspected at unchanged tolerances.
        let mut merged: Vec<(f64, usize)> = Vec::new();
        for (t, m) in nodes {
            if let Some(last) = merged.last_mut().filter(|last| (last.0 - t).abs() < 1e-5) {
                if m > last.1 {
                    last.0 = t;
                }
                last.1 = last.1.max(m);
            } else {
                merged.push((t, m));
            }
        }
        Ok(std::iter::repeat_n(0., o.degree + 1)
            .chain(
                merged
                    .into_iter()
                    .flat_map(|(t, m)| std::iter::repeat_n(t, m)),
            )
            .chain(std::iter::repeat_n(1., o.degree + 1))
            .collect())
    };
    let knots_u = axis_knots(0)?;
    let knots_v = axis_knots(1)?;
    let counts = [knots_u.len() - o.degree - 1, knots_v.len() - o.degree - 1];
    if counts.iter().any(|&n| n > 64) {
        return Err(refuse("Central graph mesh exceeds 64 controls per axis"));
    }
    let greville = |knots: &[f64], count: usize| {
        (0..count)
            .map(|i| knots[i + 1..=i + o.degree].iter().sum::<f64>() / o.degree as f64)
            .collect::<Vec<_>>()
    };
    let gu = greville(&knots_u, counts[0]);
    let gv = greville(&knots_v, counts[1]);
    let mut a: Vec<Vec<(usize, f64)>> = Vec::new();

    let mut b = Vec::new();
    let mut fair_rows = Vec::new();
    let n = counts[0] * counts[1];
    let mut active_mask = vec![false; n];
    let mut stored = 0_usize;
    let scale = [
        1.,
        1. / o.degree as f64,
        1. / o.degree as f64,
        1. / (o.degree * (o.degree - 1)) as f64,
        1. / (o.degree * (o.degree - 1)) as f64,
        1. / (o.degree * (o.degree - 1)) as f64,
    ];
    for c in &collars.boundaries {
        c.original_surface.validate()?;
        c.original_pcurve.validate()?;
        if c.original_pcurve.control_points[0].len() != 2 {
            return Err(refuse("Original support pcurve must be two dimensional"));
        }
        let domain = c.original_pcurve.domain();
        let sites = if o.align_junctions {
            parameters::stations(
                &c.curve,
                &chart,
                [&knots_u, &knots_v],
                o.degree,
                o.stations_per_side,
            )?
        } else {
            (0..o.stations_per_side)
                .map(|i| i as f64 / (o.stations_per_side - 1) as f64)
                .collect()
        };
        for station in sites {
            let t = if station == 0. {
                domain[0]
            } else if station == 1. {
                domain[1]
            } else {
                domain[0] + (domain[1] - domain[0]) * station
            };
            let p = c.original_pcurve.evaluate(t)?.point;
            let (uv, jets) = chart.jets(&c.original_surface, [p[0], p[1]])?;
            for (k, row) in rows(o.degree, [&knots_u, &knots_v], uv)?
                .into_iter()
                .enumerate()
            {
                let row: Vec<_> = row
                    .into_iter()
                    .map(|(j, x)| {
                        active_mask[j] = true;
                        (j, x * scale[k])
                    })
                    .collect();
                stored += row.len();
                if stored > 8_000_000 {
                    return Err(refuse(
                        "Sparse Hermite matrix exceeds eight million entries",
                    ));
                }
                a.push(row);
                b.push(jets[k] * scale[k]);
            }
        }
    }
    for guide in guides {
        let uv = chart.uv(guide.point);
        if uv.iter().any(|x| !x.is_finite() || *x <= 0. || *x >= 1.) {
            return Err(refuse("Interior guide is outside the open graph chart"));
        }
        let row = rows(o.degree, [&knots_u, &knots_v], uv)?[0]
            .iter()
            .map(|&(j, x)| {
                active_mask[j] = true;
                (j, x * guide.weight)
            })
            .collect::<Vec<_>>();
        stored += row.len();
        if stored > 8_000_000 {
            return Err(refuse("Guided Hermite matrix exceeds sparse work budget"));
        }
        a.push(row);
        b.push(dot(sub(guide.point, chart.origin), chart.frame[2]) * guide.weight);
    }
    let mut hard_rows = Vec::new();
    for &anchor in anchors {
        let uv = chart.uv(anchor);
        if uv.iter().any(|x| !x.is_finite() || *x <= 0. || *x >= 1.) {
            return Err(refuse("Hard anchor is outside the open graph chart"));
        }
        let row = rows(o.degree, [&knots_u, &knots_v], uv)?[0].clone();
        for &(j, _) in &row {
            active_mask[j] = true;
        }
        hard_rows.push((row, dot(sub(anchor, chart.origin), chart.frame[2])));
    }
    if fairness > 0. {
        for i in 1..counts[0] - 1 {
            for j in 1..counts[1] - 1 {
                let [left, right, bottom, top] = [
                    (gu[i] - gu[i - 1]) * chart.span[0],
                    (gu[i + 1] - gu[i]) * chart.span[0],
                    (gv[j] - gv[j - 1]) * chart.span[1],
                    (gv[j + 1] - gv[j]) * chart.span[1],
                ];
                let raw = [
                    1. / (left * (left + right)),
                    1. / (right * (left + right)),
                    1. / (bottom * (bottom + top)),
                    1. / (top * (bottom + top)),
                ];
                let sum = raw.iter().sum::<f64>();
                if raw.iter().any(|x| !x.is_finite() || *x <= 0.) || !sum.is_finite() {
                    return Err(refuse("Control-net fairing metric is singular"));
                }
                let neighbors = [
                    (i - 1) * counts[1] + j,
                    (i + 1) * counts[1] + j,
                    i * counts[1] + j - 1,
                    i * counts[1] + j + 1,
                ];
                let mut row = vec![(i * counts[1] + j, fairness)];
                for k in 0..4 {
                    row.push((neighbors[k], -fairness * raw[k] / sum));
                }
                for &(index, _) in &row {
                    active_mask[index] = true;
                }
                stored += row.len();
                if stored > 8_000_000 {
                    return Err(refuse("Fair graph exceeds sparse fitting work budget"));
                }
                fair_rows.push(row);
            }
        }
    }
    let active: Vec<usize> = (0..n).filter(|&j| active_mask[j]).collect();
    if a.len().saturating_mul(active.len()) > 32_000_000 {
        return Err(refuse(
            "Reduced Hermite matrix exceeds 32 million fitting scalars",
        ));
    }
    let mut index = vec![0; n];
    for (i, &j) in active.iter().enumerate() {
        index[j] = i;
    }
    let mut reduced = a
        .into_iter()
        .map(|row| {
            let mut r = vec![0.; active.len()];
            for (j, x) in row {
                r[index[j]] = x;
            }
            r
        })
        .collect::<Vec<_>>();
    for i in 0..active.len() {
        let mut row = vec![0.; active.len()];
        row[i] = o.regularization;
        reduced.push(row);
        b.push(0.);
    }
    let fair_rows = fair_rows
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|(j, x)| (index[j], x))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let constraints = hard_rows
        .into_iter()
        .map(|(row, target)| {
            let mut dense = vec![0.; active.len()];
            for (j, value) in row {
                dense[index[j]] = value;
            }
            (dense, target)
        })
        .collect::<Vec<_>>();
    let solution = if constraints.is_empty() {
        least_squares(reduced, b, &fair_rows)?
    } else {
        least_squares_constrained(reduced, b, &fair_rows, &constraints)?
    };
    let mut heights = vec![0.; n];
    for (i, &j) in active.iter().enumerate() {
        heights[j] = solution[i];
    }
    let surface = Surface {
        degree_u: o.degree,
        degree_v: o.degree,
        knots_u,
        knots_v,
        weights: vec![vec![1.; counts[1]]; counts[0]],
        periodic_u: false,
        periodic_v: false,
        control_points: (0..counts[0])
            .map(|i| {
                (0..counts[1])
                    .map(|j| {
                        (0..3)
                            .map(|k| {
                                origin[k]
                                    + o.frame[0][k] * (chart.low[0] + chart.span[0] * gu[i])
                                    + o.frame[1][k] * (chart.low[1] + chart.span[1] * gv[j])
                                    + o.frame[2][k] * heights[i * counts[1] + j]
                            })
                            .collect()
                    })
                    .collect()
            })
            .collect(),
    };
    surface.validate()?;
    let pcurves = collars
        .boundaries
        .iter()
        .map(|c| {
            let mut pc = c.curve.clone();
            pc.control_points = pc
                .control_points
                .iter()
                .map(|p| chart.uv(point(p)).to_vec())
                .collect();
            pc
        })
        .collect();
    let mut sampled_jet_residual = [0_f64; 6];
    // Independent, staggered sites, not the rows used for fitting.
    for c in &collars.boundaries {
        let d = c.original_pcurve.domain();
        let mut validation = (0..2 * o.stations_per_side)
            .map(|i| (i as f64 + 0.5) / (2 * o.stations_per_side) as f64)
            .collect::<Vec<_>>();
        if o.align_junctions {
            let fit = parameters::stations(
                &c.curve,
                &chart,
                [&surface.knots_u, &surface.knots_v],
                o.degree,
                o.stations_per_side,
            )?;
            validation.extend(fit.windows(2).map(|p| (p[0] + p[1]) * 0.5));
        }
        for station in validation {
            let t = d[0] + (d[1] - d[0]) * station;
            let p = c.original_pcurve.evaluate(t)?.point;
            let (uv, jets) = chart.jets(&c.original_surface, [p[0], p[1]])?;
            for (k, row) in rows(o.degree, [&surface.knots_u, &surface.knots_v], uv)?
                .iter()
                .enumerate()
            {
                let predicted = row.iter().map(|(j, a)| a * heights[*j]).sum::<f64>();
                sampled_jet_residual[k] = sampled_jet_residual[k].max((predicted - jets[k]).abs());
            }
        }
    }
    Ok(CentralGraph {
        surface,
        pcurves,
        sampled_jet_residual,
    })
}

pub struct CentralGraphInspection {
    pub seams: Vec<nurbs_core::surface_g2_pcurves::Report>,
    pub interior: nurbs_core::surface_linear_monotonicity::Report,
    pub rim: super::SpatialRimReport,
    pub cells: usize,
    pub exact_work: u64,
    pub coefficient_work: usize,
    /// Surface/rim admission only; original-face cutting and solid closure
    /// must be qualified separately. Sampled fitting errors are never read.
    pub admitted: bool,
}
#[cfg(test)]
mod streamed_fairing_tests {
    #[test]
    fn streamed_rows_retain_physical_column_scale_and_pivot_order() {
        // Minimize (2x-2)^2+y^2+(x-y)^2: x=8/9, y=4/9.
        let result = super::least_squares(
            vec![vec![2., 0.], vec![0., 1.]],
            vec![2., 0.],
            &[vec![(0, 1.), (1, -1.)]],
        )
        .unwrap();
        assert!(
            (result[0] - 8. / 9.).abs() < 1e-12 && (result[1] - 4. / 9.).abs() < 1e-12,
            "{result:?}"
        );
    }
}
/// Build the actual six-edge trimmed face from unchanged original world
/// curves and candidate pcurves. The face recipe freshly audits its UV region
/// and whole-edge lifts. This does NOT cut retained faces, prove G2, or close
/// a solid, and consumes no previously issued admission flags.
pub fn trim_central_graph(
    context: &cad_predicates::ToleranceContext,
    collars: &CornerCollars,
    candidate: &CentralGraph,
    tolerance_uv: f64,
    limits: crate::trimmed_face_recipe::Limits,
) -> Result<crate::trimmed_face_recipe::Report> {
    if collars.boundaries.len() != 6 || candidate.pcurves.len() != 6 {
        return Err(refuse(
            "A central trimmed face needs six original boundary curves",
        ));
    }
    let mut wire = Vec::new();
    let mut area = 0.;
    for i in 0..6 {
        candidate.pcurves[i].validate()?;
        let pc = &candidate.pcurves[i];
        if pc.control_points[0].len() != 2 {
            return Err(refuse("Central trim pcurve must be 2D"));
        }
        let a = &pc.control_points[0];
        let b = pc.control_points.last().unwrap();
        area += a[0] * b[1] - a[1] * b[0];
        wire.push(crate::trimmed_face_recipe::Boundary {
            curve: collars.boundaries[i].curve.clone(),
            pcurve: pc.clone(),
            reversed: false,
        });
    }
    if !area.is_finite() || area == 0. {
        return Err(refuse("Central trim orientation proposal is degenerate"));
    }
    if area < 0. {
        wire.reverse();
        for edge in &mut wire {
            edge.reversed = true;
            edge.pcurve = edge.pcurve.reverse()?;
        }
    }
    crate::trimmed_face_recipe::assemble(context, &candidate.surface, &[wire], tolerance_uv, limits)
}
/// Fresh inspection of actual candidate coefficients against each original
/// support and curve. Mutating the proposal diagnostics cannot admit geometry.
pub fn inspect_central_graph(
    collars: &CornerCollars,
    candidate: &CentralGraph,
    projection: [[i8; 3]; 2],
    tolerances: [f64; 3],
    max_cells: usize,
    max_exact: u64,
) -> Result<CentralGraphInspection> {
    inspect_graph(
        collars, candidate, projection, tolerances, max_cells, max_exact, None,
    )
}
/// Fresh correlated curve-jet inspection for polynomial graph candidates.
/// The shared coefficient budget is independent of position-predicate work.
/// Admission still concerns this surface/rim, not material or solid closure.
pub fn inspect_central_graph_correlated(
    collars: &CornerCollars,
    candidate: &CentralGraph,
    projection: [[i8; 3]; 2],
    tolerances: [f64; 3],
    max_cells: usize,
    max_exact: u64,
    max_coefficients: usize,
) -> Result<CentralGraphInspection> {
    if (max_coefficients as u64) > 50_000_000_000 {
        return Err(refuse("Central graph coefficient budget is too large"));
    }
    inspect_graph(
        collars,
        candidate,
        projection,
        tolerances,
        max_cells,
        max_exact,
        Some(max_coefficients),
    )
}
fn inspect_graph(
    collars: &CornerCollars,
    candidate: &CentralGraph,
    projection: [[i8; 3]; 2],
    tolerances: [f64; 3],
    max_cells: usize,
    max_exact: u64,
    max_coefficients: Option<usize>,
) -> Result<CentralGraphInspection> {
    if collars.boundaries.len() != 6
        || candidate.pcurves.len() != 6
        || !(80..=600000).contains(&max_cells)
        || max_exact > 1000000
        || tolerances.iter().any(|x| !x.is_finite() || *x <= 0.)
    {
        return Err(refuse(
            "Need six original graph seams and finite shared inspection limits",
        ));
    }
    let curves = collars
        .boundaries
        .iter()
        .map(|c| c.curve.clone())
        .collect::<Vec<_>>();
    let rim = super::inspect_spatial_rim(&curves, tolerances[0], (max_cells / 8).min(100000))?;
    let interior = nurbs_core::surface_linear_monotonicity::inspect(
        &candidate.surface,
        projection,
        (max_cells / 8).min(100000),
    )?;
    let mut cells = rim.cells + interior.cells;
    let mut exact_work = 0;
    let mut coefficient_work = 0;
    let mut seams = Vec::new();
    for (i, c) in collars.boundaries.iter().enumerate() {
        let report = if let Some(maximum) = max_coefficients {
            let proof = nurbs_core::surface_g2_pcurves::inspect_owned_graph(
                &c.curve,
                [&c.original_pcurve, &candidate.pcurves[i]],
                [&c.original_surface, &candidate.surface],
                tolerances,
                ((max_cells - cells) / (6 - i)).min(100000),
                max_exact.saturating_sub(exact_work) / (6 - i) as u64,
                maximum.saturating_sub(coefficient_work) / (6 - i),
            )?;
            coefficient_work = coefficient_work.saturating_add(proof.coefficient_work);
            proof.owned.report
        } else {
            nurbs_core::surface_g2_pcurves::inspect(
                &c.curve,
                [&c.original_pcurve, &candidate.pcurves[i]],
                [&c.original_surface, &candidate.surface],
                [false; 2],
                tolerances[0],
                tolerances[1],
                tolerances[2],
                ((max_cells - cells) / (6 - i)).min(100000),
                max_exact.saturating_sub(exact_work) / (6 - i) as u64,
            )?
        };
        cells += report.geometric.cells;
        exact_work += report.positions.exact_work;
        seams.push(report);
    }
    let admitted = rim.proven_simple
        && interior.certified
        && seams.iter().all(|s| {
            s.positions.proven
                && s.geometric.decision == nurbs_core::surface_join::Decision::WithinTolerance
        });
    Ok(CentralGraphInspection {
        seams,
        interior,
        rim,
        cells,
        exact_work,
        coefficient_work,
        admitted,
    })
}
