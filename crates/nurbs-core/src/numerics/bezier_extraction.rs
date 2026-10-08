//! Bézier extraction for B-spline curves and tensor-product NURBS surfaces
//! (checklist 591-594), after Borden/Scott et al., "Bézier extraction
//! operators for NURBS" — the dense small-degree variant.
//!
//! For each nonempty knot span (element) `e`, the extraction operator `C^e`
//! is the `(p+1) x n` matrix mapping the global control vector to the local
//! Bernstein coefficients of element `e`. We build it by composing the row
//! operations of repeated single-knot insertion (each interior knot raised
//! to multiplicity `p`), starting from the identity: the final insertion
//! matrix *is* the stacked operator, and its per-element row blocks are the
//! `C^e`. Degrees are bounded by the crate-wide budget (`<= 25`, controls
//! `<= 256`), so the dense representation is at most `26 x 256` per element.
//!
//! Recovery identity used in the tests: with `B_e` the local Bernstein basis
//! of element `e` restricted to its span, the global B-spline basis satisfies
//! `N = sum_e (C^e)^T B_e`; equivalently, evaluating the extracted rational
//! Bézier segment of the element owning `u` reproduces `curve.evaluate(u)`.
//! A discrete shadow of the identity is the partition-of-unity check: every
//! row of every `C^e` sums to one (knot insertion preserves affine
//! combinations).
use crate::{
    Result, check,
    curve::{Curve, find_span, validate_basis},
    foundation::guards::Budget,
    surface::Surface,
};

/// Per-element extraction operator: `matrix[i][j]` is the coefficient of the
/// global control point `j` inside the local Bernstein coefficient `i`.
#[derive(Clone, Debug)]
pub struct ExtractionOperator {
    /// Element parameter span `[t_e, t_{e+1}]` (nonempty).
    pub span: [f64; 2],
    /// Dense `(degree+1) x n_controls` operator.
    pub matrix: Vec<Vec<f64>>,
}

/// One element of a curve as a rational Bézier segment on its own parameter
/// span (the original parameter, not remapped to [0, 1]).
#[derive(Clone, Debug)]
pub struct BezierSegment {
    pub degree: usize,
    pub domain: [f64; 2],
    pub control_points: Vec<Vec<f64>>,
    pub weights: Vec<f64>,
}

impl BezierSegment {
    /// Rational Bernstein (de Casteljau) evaluation at `u in domain`.
    pub fn evaluate(&self, u: f64) -> Result<Vec<f64>> {
        let [a, b] = self.domain;
        check(
            u.is_finite() && a <= u && u <= b,
            "Bézier segment evaluation lies outside its span",
        )?;
        let t = if b > a { (u - a) / (b - a) } else { 0. };
        let dim = self.control_points[0].len();
        // Homogeneous de Casteljau.
        let mut work: Vec<Vec<f64>> = self
            .control_points
            .iter()
            .zip(&self.weights)
            .map(|(p, &w)| {
                let mut h = p.iter().map(|&x| w * x).collect::<Vec<f64>>();
                h.push(w);
                h
            })
            .collect();
        for r in 1..=self.degree {
            for i in 0..=self.degree - r {
                for k in 0..=dim {
                    work[i][k] = (1. - t) * work[i][k] + t * work[i + 1][k];
                }
            }
        }
        let w = work[0][dim];
        check(w != 0., "Bézier segment weight vanished during evaluation")?;
        Ok(work[0][..dim].iter().map(|&x| x / w).collect())
    }
}

/// One element of a tensor surface as a rational Bézier patch.
#[derive(Clone, Debug)]
pub struct BezierPatch {
    pub degree_u: usize,
    pub degree_v: usize,
    /// `[u0, u1, v0, v1]` in the original surface parameters.
    pub domain: [f64; 4],
    /// `(degree_u+1) x (degree_v+1)` grid of 3D points.
    pub control_points: Vec<Vec<[f64; 3]>>,
    pub weights: Vec<Vec<f64>>,
}

impl BezierPatch {
    /// Rational tensor Bernstein (de Casteljau) evaluation.
    pub fn evaluate(&self, u: f64, v: f64) -> Result<[f64; 3]> {
        let [u0, u1, v0, v1] = self.domain;
        check(
            u.is_finite() && v.is_finite() && u0 <= u && u <= u1 && v0 <= v && v <= v1,
            "Bézier patch evaluation lies outside its element",
        )?;
        let s = if u1 > u0 { (u - u0) / (u1 - u0) } else { 0. };
        let t = if v1 > v0 { (v - v0) / (v1 - v0) } else { 0. };
        // Homogeneous grid.
        let mut work: Vec<Vec<[f64; 4]>> = self
            .control_points
            .iter()
            .zip(&self.weights)
            .map(|(row, wrow)| {
                row.iter()
                    .zip(wrow)
                    .map(|(p, &w)| [w * p[0], w * p[1], w * p[2], w])
                    .collect()
            })
            .collect();
        // Collapse v first, then u.
        for r in 1..=self.degree_v {
            for row in work.iter_mut() {
                for j in 0..=self.degree_v - r {
                    for k in 0..4 {
                        row[j][k] = (1. - t) * row[j][k] + t * row[j + 1][k];
                    }
                }
            }
        }
        for r in 1..=self.degree_u {
            for i in 0..=self.degree_u - r {
                for k in 0..4 {
                    work[i][0][k] = (1. - s) * work[i][0][k] + s * work[i + 1][0][k];
                }
            }
        }
        let h = work[0][0];
        check(h[3] != 0., "Bézier patch weight vanished during evaluation")?;
        Ok([h[0] / h[3], h[1] / h[3], h[2] / h[3]])
    }
}

/// Knot-insertion row operation on the operator matrix (and knot vector):
/// inserting `u` into the span `s` of a degree-`p` vector with current
/// controls represented by rows of `m`.
fn insert_row(p: usize, knots: &mut Vec<f64>, m: &mut Vec<Vec<f64>>, s: usize, u: f64) {
    let n = m.len();
    let mut next: Vec<Vec<f64>> = Vec::with_capacity(n + 1);
    for row in m.iter().take(s + 1 - p) {
        next.push(row.clone());
    }
    for i in s + 1 - p..=s {
        let alpha = (u - knots[i]) / (knots[i + p] - knots[i]);
        let row: Vec<f64> = m[i]
            .iter()
            .zip(&m[i - 1])
            .map(|(&a, &b)| alpha * a + (1. - alpha) * b)
            .collect();
        next.push(row);
    }
    for row in m.iter().skip(s).take(n - s) {
        next.push(row.clone());
    }
    knots.insert(s + 1, u);
    *m = next;
}

/// Extraction operators of every nonempty span of the knot vector
/// (checklist 591). `n_controls` must match the knot vector
/// (`knots.len() == n_controls + degree + 1`). Interior knots already at
/// multiplicity `degree` (C0 joints) need no insertion. Disconnected
/// curves are rejected by basis validation and require separate nodes.
pub fn extraction_operators(
    degree: usize,
    knots: &[f64],
    n_controls: usize,
) -> Result<Vec<ExtractionOperator>> {
    let [a, b] = validate_basis(degree, knots, n_controls)?;
    // Unique interior knot values and their required insertion counts.
    let mut interior: Vec<f64> = Vec::new();
    for &t in knots {
        if t > a && t < b && interior.last() != Some(&t) {
            interior.push(t);
        }
    }
    let mut k = knots.to_vec();
    let mut m: Vec<Vec<f64>> = (0..n_controls)
        .map(|i| {
            (0..n_controls)
                .map(|j| if i == j { 1. } else { 0. })
                .collect()
        })
        .collect();
    for u in interior {
        let multiplicity = k.iter().filter(|&&x| x == u).count();
        for _ in multiplicity..degree {
            let s = find_span(degree, &k, m.len(), u)?;
            insert_row(degree, &mut k, &mut m, s, u);
        }
    }
    // Element boundaries: unique knots in [a, b].
    let mut breaks: Vec<f64> = k.iter().copied().filter(|&t| a <= t && t <= b).collect();
    breaks.dedup();
    check(breaks.len() >= 2, "Knot vector has no nonempty span")?;
    // Block starts: consecutive elements share the boundary control row when
    // the separating knot has multiplicity `degree`; a knot at multiplicity
    // `degree + 1` disconnects the blocks (extra row).
    let mut operators = Vec::with_capacity(breaks.len() - 1);
    let mut start = 0usize;
    for e in 0..breaks.len() - 1 {
        let (t0, t1) = (breaks[e], breaks[e + 1]);
        if t1 <= t0 {
            continue;
        }
        check(
            start + degree < m.len(),
            "Extraction block exceeds the Bézier control count",
        )?;
        operators.push(ExtractionOperator {
            span: [t0, t1],
            matrix: m[start..=start + degree].to_vec(),
        });
        if e + 1 < breaks.len() - 1 {
            let sep_multiplicity = k.iter().filter(|&&x| x == t1).count();
            start += degree + usize::from(sep_multiplicity > degree);
        }
    }
    check(
        start + degree + 1 == m.len(),
        "Extraction operators do not partition the Bézier control net",
    )?;
    Ok(operators)
}

/// Homogeneous 4D image of a weighted control point of any dimension.
fn homogeneous(point: &[f64], w: f64) -> Vec<f64> {
    let mut h: Vec<f64> = point.iter().map(|&x| w * x).collect();
    h.push(w);
    h
}

/// Apply an operator to a homogeneous control vector.
fn apply_operator(op: &ExtractionOperator, h: &[Vec<f64>]) -> Result<Vec<Vec<f64>>> {
    let dim = h[0].len();
    op.matrix
        .iter()
        .map(|row| {
            let mut out = vec![0.; dim];
            for (j, &c) in row.iter().enumerate() {
                if c == 0. {
                    continue;
                }
                for k in 0..dim {
                    out[k] += c * h[j][k];
                }
            }
            Ok(out)
        })
        .collect()
}

/// Dehomogenize local homogeneous coefficients into points + weights.
fn split_homogeneous(local: Vec<Vec<f64>>) -> Result<(Vec<Vec<f64>>, Vec<f64>)> {
    let dim = local[0].len() - 1;
    let mut points = Vec::with_capacity(local.len());
    let mut weights = Vec::with_capacity(local.len());
    for h in local {
        let w = h[dim];
        check(
            w > 0.,
            "Extracted Bézier coefficient has a nonpositive weight",
        )?;
        points.push(h[..dim].iter().map(|&x| x / w).collect());
        weights.push(w);
    }
    Ok((points, weights))
}

/// Every element of `curve` as a rational Bézier segment (checklist 593).
/// Periodic curves are rejected: clamp them first (`Curve::insert` at the
/// natural-domain endpoints up to multiplicity `degree + 1`).
pub fn extract_bezier_segments(curve: &Curve) -> Result<Vec<BezierSegment>> {
    curve.validate()?;
    check(
        !curve.periodic,
        "Bézier extraction requires a clamped (non-periodic) curve",
    )?;
    let ops = extraction_operators(curve.degree, &curve.knots, curve.control_points.len())?;
    let h: Vec<Vec<f64>> = curve
        .control_points
        .iter()
        .zip(&curve.weights)
        .map(|(p, &w)| homogeneous(p, w))
        .collect();
    ops.iter()
        .map(|op| {
            let (control_points, weights) = split_homogeneous(apply_operator(op, &h)?)?;
            Ok(BezierSegment {
                degree: curve.degree,
                domain: op.span,
                control_points,
                weights,
            })
        })
        .collect()
}

/// Every element of `surface` as a rational Bézier patch: the tensor product
/// `C^e_u ⊗ C^e_v` applied to the homogeneous control grid (checklist 592).
pub fn extract_bezier_patches(surface: &Surface) -> Result<Vec<BezierPatch>> {
    surface.validate()?;
    check(
        !surface.periodic_u && !surface.periodic_v,
        "Bézier extraction requires clamped (non-periodic) surfaces",
    )?;
    let nu = surface.control_points.len();
    let nv = surface.control_points[0].len();
    let ops_u = extraction_operators(surface.degree_u, &surface.knots_u, nu)?;
    let ops_v = extraction_operators(surface.degree_v, &surface.knots_v, nv)?;
    // Homogeneous grid h[i][j][0..4].
    let h: Vec<Vec<[f64; 4]>> = surface
        .control_points
        .iter()
        .zip(&surface.weights)
        .map(|(row, wrow)| {
            row.iter()
                .zip(wrow)
                .map(|(p, &w)| [w * p[0], w * p[1], w * p[2], w])
                .collect()
        })
        .collect();
    let mut patches = Vec::with_capacity(ops_u.len() * ops_v.len());
    for ou in &ops_u {
        for ov in &ops_v {
            // local[i][j] = sum_a sum_b Cu[i][a] Cv[j][b] h[a][b].
            let mut local = vec![vec![[0.; 4]; ov.matrix.len()]; ou.matrix.len()];
            for (i, cu) in ou.matrix.iter().enumerate() {
                for (j, cv) in ov.matrix.iter().enumerate() {
                    for (a, &ca) in cu.iter().enumerate() {
                        if ca == 0. {
                            continue;
                        }
                        for (b, &cb) in cv.iter().enumerate() {
                            if cb == 0. {
                                continue;
                            }
                            let c = ca * cb;
                            for k in 0..4 {
                                local[i][j][k] += c * h[a][b][k];
                            }
                        }
                    }
                }
            }
            let mut control_points = Vec::with_capacity(local.len());
            let mut weights = Vec::with_capacity(local.len());
            for row in local {
                let mut points = Vec::with_capacity(row.len());
                let mut ws = Vec::with_capacity(row.len());
                for h in row {
                    check(
                        h[3] > 0.,
                        "Extracted Bézier coefficient has a nonpositive weight",
                    )?;
                    points.push([h[0] / h[3], h[1] / h[3], h[2] / h[3]]);
                    ws.push(h[3]);
                }
                control_points.push(points);
                weights.push(ws);
            }
            patches.push(BezierPatch {
                degree_u: surface.degree_u,
                degree_v: surface.degree_v,
                domain: [ou.span[0], ou.span[1], ov.span[0], ov.span[1]],
                control_points,
                weights,
            });
        }
    }
    Ok(patches)
}

/// Gauss-Legendre nodes and weights on `[-1, 1]` (Newton on the Legendre
/// polynomial, classic Golub-Welsch-free variant; `n <= 64`).
pub fn gauss_legendre(n: usize) -> Result<Vec<(f64, f64)>> {
    // Newton on the Legendre polynomial converges in a handful of steps;
    // 100 iterations per node is generous insurance (item 1065).
    gauss_legendre_with_budget(n, Budget::with_iterations(100)?)
}

/// [`gauss_legendre`] with an explicit unified [`Budget`] for the Newton
/// node refinement loops; exhaustion aborts with `BudgetExhausted`.
fn gauss_legendre_with_budget(n: usize, budget: Budget) -> Result<Vec<(f64, f64)>> {
    check((1..=64).contains(&n), "Gauss order must be in [1, 64]")?;
    let mut out = vec![(0., 0.); n];
    for i in 0..(n + 1) / 2 {
        // One budget per node: Newton converges in a handful of steps.
        let mut guard = budget.guard("gauss-legendre");
        let mut x = (std::f64::consts::PI * (i as f64 + 0.75) / (n as f64 + 0.5)).cos();
        loop {
            guard.tick()?;
            let (mut p0, mut p1) = (1., x);
            for j in 2..=n {
                let p2 = ((2 * j - 1) as f64 * x * p1 - (j - 1) as f64 * p0) / j as f64;
                p0 = p1;
                p1 = p2;
            }
            let dp = n as f64 * (x * p1 - p0) / (x * x - 1.);
            let dx = p1 / dp;
            x -= dx;
            if dx.abs() <= 1e-15 * (1. + x.abs()) {
                out[i] = (-x, 2. / ((1. - x * x) * dp * dp));
                out[n - 1 - i] = (x, 2. / ((1. - x * x) * dp * dp));
                break;
            }
        }
    }
    Ok(out)
}

/// Elementwise Gauss quadrature of a scalar field over a curve
/// (checklist 594): `sum_e sum_q w_q f(u_q, C(u_q))` with `n_gauss` nodes
/// per extracted Bézier element, using the original (unmapped) parameter.
/// Intended as the IGA assembly primitive; the field receives both the
/// parameter and the evaluated point.
pub fn gauss_quadrature_over_curve(
    curve: &Curve,
    n_gauss: usize,
    f: impl Fn(f64, &[f64]) -> f64,
) -> Result<f64> {
    let segments = extract_bezier_segments(curve)?;
    let rule = gauss_legendre(n_gauss)?;
    let mut sum = 0.;
    for seg in &segments {
        let [a, b] = seg.domain;
        let (half, mid) = ((b - a) / 2., (a + b) / 2.);
        for &(x, w) in &rule {
            let u = mid + half * x;
            sum += w * half * f(u, &seg.evaluate(u)?);
        }
    }
    Ok(sum)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives;

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z ^ (z >> 31)
        }
        fn f64(&mut self) -> f64 {
            (self.next() >> 11) as f64 / (1u64 << 53) as f64
        }
        fn range(&mut self, a: f64, b: f64) -> f64 {
            a + (b - a) * self.f64()
        }
    }

    #[test]
    fn gauss_legendre_budget_exhaustion_is_a_resource_error() {
        // A 1-iteration budget cannot refine any node: typed BudgetExhausted.
        let err = gauss_legendre_with_budget(8, Budget::with_iterations(1).unwrap()).unwrap_err();
        assert_eq!(err.code, crate::RESOURCE_LIMIT, "{err:?}");
        assert!(err.contains("gauss-legendre"), "{err}");
        // The default budget computes correct finite nodes.
        let rule = gauss_legendre(8).unwrap();
        assert_eq!(rule.len(), 8);
        assert!(rule.iter().all(|(x, w)| x.is_finite() && w.is_finite()));
    }

    fn random_curve(rng: &mut Rng) -> Curve {
        let p = 1 + (rng.next() % 4) as usize;
        let interior = 1 + (rng.next() % 5) as usize;
        let mut knots = vec![0.; p + 1];
        for i in 0..interior {
            knots.push((i + 1) as f64 / (interior + 1) as f64);
        }
        knots.extend(vec![1.; p + 1]);
        let n = knots.len() - p - 1;
        let control_points = (0..n)
            .map(|_| (0..3).map(|_| rng.range(-5., 5.)).collect::<Vec<f64>>())
            .collect();
        let weights = (0..n).map(|_| rng.range(0.5, 2.)).collect();
        let curve = Curve {
            degree: p,
            knots,
            control_points,
            weights,
            periodic: false,
        };
        curve.validate().unwrap();
        curve
    }

    fn close(a: &[f64], b: &[f64], tol: f64) {
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b) {
            assert!(
                (x - y).abs() <= tol * (1. + x.abs().max(y.abs())),
                "|{x} - {y}| exceeds {tol}"
            );
        }
    }

    /// Segment owning `u` (right-hand owner at shared knots, last at `b`).
    fn owner<'a>(segments: &'a [BezierSegment], u: f64) -> &'a BezierSegment {
        segments
            .iter()
            .find(|s| s.domain[0] <= u && u < s.domain[1])
            .or_else(|| segments.last().filter(|s| u == s.domain[1]))
            .unwrap()
    }

    #[test]
    fn operators_preserve_partition_of_unity() {
        let mut rng = Rng(7);
        for _ in 0..20 {
            let c = random_curve(&mut rng);
            let ops = extraction_operators(c.degree, &c.knots, c.control_points.len()).unwrap();
            assert!(!ops.is_empty());
            for op in &ops {
                assert_eq!(op.matrix.len(), c.degree + 1);
                for (i, row) in op.matrix.iter().enumerate() {
                    let sum: f64 = row.iter().sum();
                    assert!(
                        (sum - 1.).abs() < 1e-12,
                        "row {i} of C^e sums to {sum}, not 1"
                    );
                }
            }
        }
    }

    #[test]
    fn extracted_segments_reproduce_the_curve() {
        let mut rng = Rng(13);
        for _ in 0..20 {
            let c = random_curve(&mut rng);
            let segments = extract_bezier_segments(&c).unwrap();
            // One segment per nonempty span.
            let spans = c
                .knots
                .windows(2)
                .filter(|w| w[1] > w[0] && w[0] >= 0. && w[1] <= 1.)
                .count();
            assert_eq!(segments.len(), spans);
            for _ in 0..64 {
                let u = rng.range(0., 1.);
                let reference = c.evaluate(u).unwrap().point;
                close(&owner(&segments, u).evaluate(u).unwrap(), &reference, 1e-9);
            }
            // Element endpoints agree with the curve at the knots.
            for seg in &segments {
                for &u in &seg.domain {
                    let reference = c.evaluate(u).unwrap().point;
                    close(&seg.evaluate(u).unwrap(), &reference, 1e-9);
                }
            }
        }
    }

    #[test]
    fn extraction_handles_c0_joints() {
        // Degree 2 with an interior knot at full multiplicity (C0 joint).
        let c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.4, 0.4, 0.7, 0.7, 1., 1., 1.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![1., 2., 0.],
                vec![2., -1., 1.],
                vec![3., 2., 0.],
                vec![4., 0., 1.],
                vec![5., 1., 0.],
                vec![6., -2., 0.],
            ],
            weights: vec![1., 1.2, 0.8, 1., 1.5, 0.9, 1.],
            periodic: false,
        };
        c.validate().unwrap();
        let segments = extract_bezier_segments(&c).unwrap();
        // Three nonempty spans: [0, 0.4], [0.4, 0.7], [0.7, 1].
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].domain, [0., 0.4]);
        assert_eq!(segments[1].domain, [0.4, 0.7]);
        assert_eq!(segments[2].domain, [0.7, 1.]);
        let mut rng = Rng(3);
        for _ in 0..64 {
            let u = rng.range(0., 1.);
            let reference = c.evaluate(u).unwrap().point;
            close(&owner(&segments, u).evaluate(u).unwrap(), &reference, 1e-9);
        }
    }

    #[test]
    fn disconnected_elements_require_separate_curve_nodes() {
        let curve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.5, 0.5, 0.5, 1., 1., 1.],
            control_points: vec![
                vec![0., 0.],
                vec![1., 2.],
                vec![2., 0.],
                vec![8., 1.],
                vec![9., -2.],
                vec![10., 1.],
            ],
            weights: vec![1., 0.8, 1., 1., 1.2, 1.],
            periodic: false,
        };
        let error = extract_bezier_segments(&curve).unwrap_err();
        assert_eq!(error.code, "NURBS_INVALID_INPUT");
        assert!(
            error
                .message
                .contains("disconnected curves need separate nodes")
        );
    }

    #[test]
    fn extracted_patches_reproduce_the_surface() {
        let s = Surface {
            degree_u: 2,
            degree_v: 3,
            knots_u: vec![0., 0., 0., 0.5, 1., 1., 1.],
            knots_v: vec![0., 0., 0., 0., 0.3, 0.6, 1., 1., 1., 1.],
            control_points: (0..4)
                .map(|i| {
                    (0..6)
                        .map(|j| {
                            vec![
                                i as f64 + 0.1 * (i * j) as f64,
                                j as f64 * 0.7,
                                ((i * 3 + j) % 4) as f64 - 1.,
                            ]
                        })
                        .collect()
                })
                .collect(),
            weights: (0..4)
                .map(|i| (0..6).map(|j| 0.6 + 0.2 * ((i + j) % 3) as f64).collect())
                .collect(),
            periodic_u: false,
            periodic_v: false,
        };
        s.validate().unwrap();
        let patches = extract_bezier_patches(&s).unwrap();
        assert_eq!(patches.len(), 2 * 3);
        let mut rng = Rng(23);
        for _ in 0..128 {
            let u = rng.range(0., 1.);
            let v = rng.range(0., 1.);
            let reference = s.evaluate(u, v).unwrap().point;
            let patch = patches
                .iter()
                .find(|p| {
                    p.domain[0] <= u && u < p.domain[1] && p.domain[2] <= v && v < p.domain[3]
                })
                .or_else(|| {
                    // Right/top boundary: the closed corner element owns it.
                    patches.iter().find(|p| {
                        p.domain[0] <= u
                            && u <= p.domain[1]
                            && p.domain[2] <= v
                            && v <= p.domain[3]
                            && u >= p.domain[1] - (p.domain[1] - p.domain[0])
                            && v >= p.domain[3] - (p.domain[3] - p.domain[2])
                    })
                })
                .unwrap();
            let got = patch.evaluate(u, v).unwrap();
            close(&got, &reference, 1e-9);
        }
    }

    #[test]
    fn gauss_quadrature_integrates_over_elements() {
        // f == 1 integrates to the parameter-domain length.
        let mut rng = Rng(31);
        for _ in 0..10 {
            let c = random_curve(&mut rng);
            let total = gauss_quadrature_over_curve(&c, 8, |_, _| 1.).unwrap();
            assert!((total - 1.).abs() < 1e-12, "integral of 1 is {total}");
        }
        // Straight line (0,0,0) -> (2,0,0), parameter [0,1]: int x du = 1.
        let line = primitives::line([0., 0., 0.], [2., 0., 0.]).unwrap();
        let total = gauss_quadrature_over_curve(&line, 4, |_, p| p[0]).unwrap();
        assert!((total - 1.).abs() < 1e-12, "integral of x is {total}");
        // Parameter identity: int u du over [0,1] = 0.5.
        let total = gauss_quadrature_over_curve(&line, 4, |u, _| u).unwrap();
        assert!((total - 0.5).abs() < 1e-12);
    }
}
