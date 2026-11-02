//! Exact finite membership at a proved singleton original-span root.
use super::affine_plane::{enclosure, q, subtract};
use super::original_patch_contacts::{OriginalPatchContact, OriginalPatchUvProof};
use super::*;
use num_rational::BigRational as Q;
use num_traits::Zero;
fn fail(message: &str) -> crate::Error {
    crate::Error::new("NURBS_ROOT_PROOF_CONTRADICTION", message)
}
fn point(c: &Curve, span: usize, t: &Q) -> Result<[Q; 3]> {
    let p = c.degree;
    let mut d: Vec<[Q; 4]> = (span - p..=span)
        .map(|i| {
            let w = q(c.weights[i]);
            [
                q(c.control_points[i][0]) * &w,
                q(c.control_points[i][1]) * &w,
                q(c.control_points[i][2]) * &w,
                w,
            ]
        })
        .collect();
    for r in 1..=p {
        for j in (r..=p).rev() {
            let i = span - p + j;
            let den = q(c.knots[i + p - r + 1]) - q(c.knots[i]);
            if den <= Q::zero() {
                return Err(fail("Exact root evaluation has a degenerate source basis"));
            }
            let a = (t - q(c.knots[i])) / den;
            let b = q(1.) - &a;
            d[j] = std::array::from_fn(|k| &d[j - 1][k] * &b + &d[j][k] * &a);
        }
    }
    if d[p][3] <= Q::zero() {
        return Err(fail(
            "Exact root evaluation lost its positive source weight",
        ));
    }
    Ok(std::array::from_fn(|k| &d[p][k] / &d[p][3]))
}
fn bound(x: &Q) -> Result<[f64; 2]> {
    let b =
        enclosure(x).ok_or_else(|| crate::numeric_err("No finite exact membership enclosure"))?;
    Ok([b.lo, b.hi])
}
fn clock(a: &Q, w0: f64, w1: f64) -> Q {
    q(w0) * a / (q(w1) * (q(1.) - a) + q(w0) * a)
}
/// `normalized` is a singleton supplied by the exact scalar isolator, never a
/// midpoint of an unresolved root interval. None proves finite-patch exclusion.
pub(super) fn contact(
    c: &Curve,
    s: &Surface,
    span: usize,
    normalized: f64,
    multiplicity: usize,
) -> Result<Option<OriginalPatchContact>> {
    let t = q(c.knots[span]) + (q(c.knots[span + 1]) - q(c.knots[span])) * q(normalized);
    let p = point(c, span, &t)?;
    let pole = |i: usize, j: usize| std::array::from_fn(|k| q(s.control_points[i][j][k]));
    let origin = pole(0, 0);
    let u = subtract(&pole(1, 0), &origin);
    let v = subtract(&pole(0, 1), &origin);
    let (i, j, det) = [(0, 1), (0, 2), (1, 2)]
        .into_iter()
        .map(|(i, j)| (i, j, &u[i] * &v[j] - &u[j] * &v[i]))
        .find(|(_, _, d)| !d.is_zero())
        .ok_or_else(|| fail("Exact membership has no affine inverse"))?;
    let di = &p[i] - &origin[i];
    let dj = &p[j] - &origin[j];
    let a = (&di * &v[j] - &dj * &v[i]) / &det;
    let b = (&u[i] * &dj - &u[j] * &di) / &det;
    if (0..3).any(|k| &origin[k] + &u[k] * &a + &v[k] * &b != p[k]) {
        return Err(fail(
            "Singleton scalar root does not lie on its original patch plane",
        ));
    }
    if a < Q::zero() || a > q(1.) || b < Q::zero() || b > q(1.) {
        return Ok(None);
    }
    let exact_uv = if a.is_zero() {
        Some([q(0.), clock(&b, s.weights[0][0], s.weights[0][1])])
    } else if a == q(1.) {
        Some([q(1.), clock(&b, s.weights[1][0], s.weights[1][1])])
    } else if b.is_zero() {
        Some([clock(&a, s.weights[0][0], s.weights[1][0]), q(0.)])
    } else if b == q(1.) {
        Some([clock(&a, s.weights[0][1], s.weights[1][1]), q(1.)])
    } else {
        None
    };
    let (uv, uv_proof) = if let Some(uv) = exact_uv {
        (
            [
                bound(&(q(s.knots_u[1]) + (q(s.knots_u[2]) - q(s.knots_u[1])) * &uv[0]))?,
                bound(&(q(s.knots_v[1]) + (q(s.knots_v[2]) - q(s.knots_v[1])) * &uv[1]))?,
            ],
            OriginalPatchUvProof::ExactBoundaryProjective,
        )
    } else {
        (
            [[s.knots_u[1], s.knots_u[2]], [s.knots_v[1], s.knots_v[2]]],
            OriginalPatchUvProof::PositiveBilinearGlobalInverse,
        )
    };
    Ok(Some(OriginalPatchContact {
        uv_proof,
        parameter_box: [bound(&t)?, uv[0], uv[1]],
        position_bounds: [bound(&p[0])?, bound(&p[1])?, bound(&p[2])?],
        source_spans: vec![span],
        multiplicities: vec![multiplicity],
    }))
}
