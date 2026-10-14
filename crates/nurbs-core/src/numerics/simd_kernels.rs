//! Batch evaluation kernels with a structure-of-arrays layout, written so
//! the inner loops are friendly to LLVM autovectorization without any
//! external dependency and without `std::simd` (still unstable). The code is
//! plain portable scalar Rust organized in fixed-width lanes of 4; a future
//! `std::simd` port can replace the lane loops one-to-one.
//!
//! Guaranteed parity: `batch_evaluate_curve` reproduces
//! [`crate::curve::Curve::evaluate`] up to a small multiple of the scalar
//! evaluator's own rounding bound. The only numerical difference is the
//! summation strategy: the scalar evaluator uses a per-call Neumaier
//! compensated accumulator with an origin shift, while the batch kernel uses
//! branch-free lane accumulators over the `degree + 1` nonzero basis terms.
//! Both compute the same mathematical weighted homogeneous sum; the basis
//! values themselves are bitwise identical because `batch_basis_funs`
//! replicates the exact operation order of the scalar recursion restricted
//! to the local window. In the tests below the observed deviation on O(1)
//! geometry is at most a few `f64::EPSILON` and is asserted against a
//! scale-aware bound.
use crate::curve::{Curve, find_span};
use crate::{Result, check, numeric};

/// Structure-of-arrays point/weight batch. All four columns have identical
/// length; `w` holds the (positive) homogeneous weights.
#[derive(Clone, Debug, PartialEq)]
pub struct PointBatch {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    pub z: Vec<f64>,
    pub w: Vec<f64>,
}

impl PointBatch {
    /// Build a batch from AoS points and weights, validating finiteness,
    /// weight positivity and a hard size budget.
    pub fn from_points_weights(points: &[[f64; 3]], weights: &[f64]) -> Result<Self> {
        check(
            points.len() == weights.len(),
            "Point batch points and weights must have equal length",
        )?;
        check(
            points.len() <= 1_000_000,
            "Point batch is limited to 1_000_000 entries",
        )?;
        check(
            points.iter().flatten().all(|v| v.is_finite()),
            "Point batch coordinates must be finite",
        )?;
        check(
            weights
                .iter()
                .all(|&w| w.is_finite() && w > 0. && w <= 1e9),
            "Point batch weights must be positive, finite and bounded by 1e9",
        )?;
        Ok(Self {
            x: points.iter().map(|p| p[0]).collect(),
            y: points.iter().map(|p| p[1]).collect(),
            z: points.iter().map(|p| p[2]).collect(),
            w: weights.to_vec(),
        })
    }

    pub fn len(&self) -> usize {
        self.x.len()
    }

    pub fn is_empty(&self) -> bool {
        self.x.is_empty()
    }
}

/// Basis values of the `degree + 1` functions nonzero on `span`, written to
/// `out[0..=degree]`. This is the local-window restriction of the scalar
/// recursion used by `crate::curve::basis`: identical formula, identical
/// operand order (division before multiplication, left term accumulated
/// before the right term), so the produced values are bitwise identical to
/// the corresponding entries of the full-width scalar basis. Out-of-window
/// full-width entries are exactly zero there and only contribute exact
/// no-ops to the scalar accumulation.
///
/// `u` must satisfy `knots[span] <= u <= knots[span + 1]` with
/// `degree <= span` and `span + degree + 1 < knots.len()` (the usual
/// convention that `span` is the last index with `knots[span] <= u`, plus
/// the end-of-domain convention `span = n - 1` for `u == knots.last()`).
#[inline]
pub fn batch_basis_funs(
    span: usize,
    u: f64,
    degree: usize,
    knots: &[f64],
    out: &mut [f64],
) -> Result<()> {
    let p = degree;
    check((1..=25).contains(&p), "Degree must be an integer in [1, 25]")?;
    check(out.len() >= p + 1, "Basis output must hold degree + 1 values")?;
    check(
        knots.len() >= 2 * p + 2,
        "Knot vector is too short for the degree",
    )?;
    check(
        span >= p && span + p + 1 < knots.len(),
        "Span is outside the valid knot range",
    )?;
    check(
        u.is_finite() && u >= knots[span] && u <= knots[span + 1],
        "Parameter is outside the given span",
    )?;
    // Window `cur[idx]` at recursion order `r` holds the order-`r` basis
    // value of global index `span - r + idx`; entries before `p - r` are
    // always zero. Initialize with the order-0 window (a single 1 at the
    // global `span` position).
    let mut cur = [0.; 26];
    cur[p] = 1.;
    let mut next = [0.; 26];
    for r in 1..=p {
        for entry in next.iter_mut().take(p + 1) {
            *entry = 0.;
        }
        // Global indices `i` in `span - r ..= span`; local idx = i-(span-r).
        for idx in 0..=r {
            let i = span - r + idx;
            // Window value at global i (order r-1 lives in cur at p-(r-1)+..).
            let at = |global: usize| -> f64 {
                let lo = span + 1 - r; // first global index of the order r-1 window
                if global < lo || global > span {
                    0.
                } else {
                    cur[p + 1 - r + (global - lo)]
                }
            };
            let mut v = 0.;
            let left = knots[i + r] - knots[i];
            if left != 0. {
                v += (u - knots[i]) / left * at(i);
            }
            let right = knots[i + r + 1] - knots[i + 1];
            if right != 0. {
                v += (knots[i + r + 1] - u) / right * at(i + 1);
            }
            // Keep the invariant: order-`r` window occupies indices p-r..=p.
            next[p - r + idx] = v;
        }
        cur[..p + 1].copy_from_slice(&next[..p + 1]);
    }
    numeric(
        cur[..p + 1].iter().all(|v| v.is_finite()),
        "Batch basis computation exhausted finite precision",
    )?;
    out[..p + 1].copy_from_slice(&cur[..p + 1]);
    Ok(())
}

/// Batch evaluation of a validated 3D NURBS curve at `params`.
///
/// Parameters are processed in lane groups of 4: spans and local basis
/// windows are computed per lane, then the weighted homogeneous accumulation
/// runs over the `degree + 1` active control points with branch-free
/// fixed-width inner loops over the lanes (SoA-friendly, autovectorizable).
/// Trailing partial groups use zeroed padding lanes that contribute exact
/// zeros.
///
/// Errors: non-3D or invalid curve (`check`), non-finite or out-of-domain
/// parameters (`check`), more than 1_000_000 parameters (`resource`).
pub fn batch_evaluate_curve(curve: &Curve, params: &[f64]) -> Result<Vec<[f64; 3]>> {
    curve.validate()?;
    check(
        curve.control_points[0].len() == 3,
        "Batch curve evaluation expects 3D control points",
    )?;
    check(
        params.len() <= 1_000_000,
        "Batch curve evaluation is limited to 1_000_000 parameters",
    )?;
    let [a, b] = curve.domain();
    check(
        params
            .iter()
            .all(|&u| u.is_finite() && u >= a && u <= b),
        "Batch parameters must be finite and inside the curve domain",
    )?;
    let p = curve.degree;
    let n = curve.control_points.len();
    let mut out = Vec::with_capacity(params.len());
    // Lane storage: basis windows and spans per lane.
    let mut windows = [[0.; 26]; 4];
    let mut spans = [0_usize; 4];
    let mut active = [0_usize; 4];
    let weights = PointBatch {
        x: curve.control_points.iter().map(|c| c[0]).collect(),
        y: curve.control_points.iter().map(|c| c[1]).collect(),
        z: curve.control_points.iter().map(|c| c[2]).collect(),
        w: curve.weights.clone(),
    };
    let mut start = 0;
    while start < params.len() {
        let lanes = (params.len() - start).min(4);
        for k in 0..4 {
            if k < lanes {
                let u = params[start + k];
                spans[k] = find_span(p, &curve.knots, n, u)?;
                batch_basis_funs(spans[k], u, p, &curve.knots, &mut windows[k])?;
                active[k] = 1;
            } else {
                // Padding lane: a valid span with an all-zero window so the
                // fixed-width loops contribute exact zeros.
                spans[k] = p;
                windows[k] = [0.; 26];
                active[k] = 0;
            }
        }
        let mut acc_w = [0.; 4];
        let mut acc_x = [0.; 4];
        let mut acc_y = [0.; 4];
        let mut acc_z = [0.; 4];
        for j in 0..=p {
            let mut hw = [0.; 4];
            let mut hx = [0.; 4];
            let mut hy = [0.; 4];
            let mut hz = [0.; 4];
            // Gather: per-lane control index `span - p + j`.
            for k in 0..4 {
                let i = spans[k] - p + j;
                let c = windows[k][j] * weights.w[i] * active[k] as f64;
                hw[k] = c;
                hx[k] = c * weights.x[i];
                hy[k] = c * weights.y[i];
                hz[k] = c * weights.z[i];
            }
            // Fixed-width branch-free accumulation (autovectorization target).
            for k in 0..4 {
                acc_w[k] += hw[k];
                acc_x[k] += hx[k];
                acc_y[k] += hy[k];
                acc_z[k] += hz[k];
            }
        }
        for k in 0..lanes {
            let w = acc_w[k];
            numeric(
                w > 0. && w.is_finite(),
                "Rational denominator lost its positive finite value in batch evaluation",
            )?;
            out.push([acc_x[k] / w, acc_y[k] / w, acc_z[k] / w]);
        }
        start += lanes;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic xorshift64* generator; no external dependencies.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> f64 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            let bits = x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11;
            (bits as f64) / ((1u64 << 53) as f64)
        }
    }

    fn random_curve(rng: &mut Rng, degree: usize, controls: usize, rational: bool) -> Curve {
        let interior = controls - degree - 1;
        let mut knots: Vec<f64> = std::iter::repeat_n(0., degree + 1).collect();
        let mut u = 0.;
        for i in 0..interior {
            u += 0.5 + rng.next();
            knots.push(u);
        }
        let end = u + 0.5 + rng.next();
        knots.extend(std::iter::repeat_n(end, degree + 1));
        Curve {
            degree,
            knots,
            control_points: (0..controls)
                .map(|_| {
                    vec![
                        2. * rng.next() - 1.,
                        2. * rng.next() - 1.,
                        2. * rng.next() - 1.,
                    ]
                })
                .collect(),
            weights: (0..controls)
                .map(|_| {
                    if rational {
                        0.25 + 1.75 * rng.next()
                    } else {
                        1.
                    }
                })
                .collect(),
            periodic: false,
        }
    }

    #[test]
    fn batch_basis_matches_scalar_basis_bitwise() {
        let mut rng = Rng(0x1234_5678_9abc_def0);
        for (degree, controls) in [(1, 4), (2, 6), (3, 8), (4, 12)] {
            let curve = random_curve(&mut rng, degree, controls, false);
            let [a, b] = curve.domain();
            for sample in 0..64 {
                let t = sample as f64 / 63.;
                let u = a + (b - a) * t;
                let span = find_span(degree, &curve.knots, controls, u).unwrap();
                let mut local = [0.; 26];
                batch_basis_funs(span, u, degree, &curve.knots, &mut local).unwrap();
                let full = crate::curve::basis(degree, &curve.knots, controls, u, false).unwrap();
                for j in 0..=degree {
                    assert_eq!(
                        local[j].to_bits(),
                        full.basis[span - degree + j].to_bits(),
                        "basis mismatch at degree {degree} u={u} j={j}"
                    );
                }
            }
        }
    }

    #[test]
    fn batch_evaluate_matches_scalar_within_rounding() {
        let mut rng = Rng(0xdead_beef_cafe_f00d);
        for rational in [false, true] {
            for (degree, controls) in [(1, 2), (2, 5), (3, 9), (5, 16)] {
                let curve = random_curve(&mut rng, degree, controls, rational);
                let [a, b] = curve.domain();
                // Include a non-multiple-of-4 count to exercise padding lanes.
                let params: Vec<f64> = (0..61)
                    .map(|i| a + (b - a) * i as f64 / 60.)
                    .collect();
                let batch = batch_evaluate_curve(&curve, &params).unwrap();
                assert_eq!(batch.len(), params.len());
                for (u, got) in params.iter().zip(&batch) {
                    let scalar = curve.evaluate(*u).unwrap();
                    for axis in 0..3 {
                        let want = scalar.point[axis];
                        let scale = 1. + want.abs();
                        // The batch kernel replaces Neumaier accumulation with
                        // plain lane sums over the degree+1 nonzero terms; the
                        // deviation is bounded by a few ulps of the largest
                        // accumulated magnitude.
                        assert!(
                            (got[axis] - want).abs() <= 1e-14 * scale,
                            "parity failure rational={rational} degree={degree} u={u} axis={axis}: got={} want={want}",
                            got[axis]
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn batch_evaluate_rejects_bad_input() {
        let curve = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0., 0.], vec![1., 1.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        // 2D curve is rejected.
        assert!(batch_evaluate_curve(&curve, &[0.5]).is_err());
        let curve3d = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![1., 1., 1.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        assert!(batch_evaluate_curve(&curve3d, &[-0.1]).is_err());
        assert!(batch_evaluate_curve(&curve3d, &[f64::NAN]).is_err());
        assert!(PointBatch::from_points_weights(&[[0.; 3]], &[0.]).is_err());
        assert!(PointBatch::from_points_weights(&[[0.; 3]], &[]).is_err());
    }

    #[test]
    fn point_batch_roundtrip() {
        let points = [[1., 2., 3.], [4., 5., 6.]];
        let batch = PointBatch::from_points_weights(&points, &[1., 2.]).unwrap();
        assert_eq!(batch.len(), 2);
        assert!(!batch.is_empty());
        assert_eq!(batch.x, vec![1., 4.]);
        assert_eq!(batch.w, vec![1., 2.]);
        assert!(PointBatch::from_points_weights(&[], &[]).unwrap().is_empty());
    }
}
