//! Exact affine-coordinate bounds of original positive-weight span controls.
use super::affine_plane::{q, subtract};
use super::original_patch_contacts::{OriginalPatchContact, OriginalPatchUvProof};
use super::patch_exact_membership::{bound, clock};
use super::*;
use num_rational::BigRational as Q;
use num_traits::Zero;
#[derive(Clone)]
pub(super) struct CoordinateBounds {
    origin: [Q; 3],
    u: [Q; 3],
    v: [Q; 3],
    ranges: [[Q; 2]; 2],
}
pub(super) enum Decision {
    Outside,
    Unresolved,
    Contact(OriginalPatchContact),
}
fn fail() -> crate::Error {
    crate::Error::new(
        "NURBS_ROOT_PROOF_CONTRADICTION",
        "Original root bounds contradict exact affine control bounds",
    )
}
/// An affine functional of a positive rational spline is the weighted convex
/// combination of that functional on ORIGINAL active controls. Constant edge
/// coordinates therefore prove a plane identity on the whole original span;
/// no extracted or approximately planar curve is used.
pub(super) fn original(c: &Curve, s: &Surface, span: usize) -> Result<CoordinateBounds> {
    let pole = |i: usize, j: usize| std::array::from_fn(|k| q(s.control_points[i][j][k]));
    let origin = pole(0, 0);
    let u = subtract(&pole(1, 0), &origin);
    let v = subtract(&pole(0, 1), &origin);
    let (i, j, det) = [(0, 1), (0, 2), (1, 2)]
        .into_iter()
        .map(|(i, j)| (i, j, &u[i] * &v[j] - &u[j] * &v[i]))
        .find(|(_, _, d)| !d.is_zero())
        .ok_or_else(fail)?;
    let normal = [
        &u[1] * &v[2] - &u[2] * &v[1],
        &u[2] * &v[0] - &u[0] * &v[2],
        &u[0] * &v[1] - &u[1] * &v[0],
    ];
    let mut controls = Vec::new();
    let mut residuals = Vec::new();
    let mut ranges: Option<[[Q; 2]; 2]> = None;
    for p in &c.control_points[span - c.degree..=span] {
        let di = q(p[i]) - &origin[i];
        let dj = q(p[j]) - &origin[j];
        let values = [
            (&di * &v[j] - &dj * &v[i]) / &det,
            (&u[i] * &dj - &u[j] * &di) / &det,
        ];
        residuals.push((0..3).fold(Q::zero(), |r, k| r + &normal[k] * (q(p[k]) - &origin[k])));
        controls.push(values.clone());
        if let Some(r) = &mut ranges {
            for axis in 0..2 {
                r[axis][0] = r[axis][0].clone().min(values[axis].clone());
                r[axis][1] = r[axis][1].clone().max(values[axis].clone());
            }
        } else {
            ranges = Some(values.map(|x| [x.clone(), x]));
        }
    }
    let mut ranges = ranges.unwrap();
    // At a certified original-plane root, residual = 0. An exact relation
    // coordinate_i = slope * residual_i + intercept on every active original
    // control survives positive rational blending, with arbitrary knots/weights.
    // Thus its intercept is a conditional exact coordinate at that root. No
    // approximate root evaluation or comparison of rounded polynomials is used.
    if let Some(j) = (1..residuals.len()).find(|&j| residuals[j] != residuals[0]) {
        for axis in 0..2 {
            let slope = (&controls[j][axis] - &controls[0][axis]) / (&residuals[j] - &residuals[0]);
            let intercept = &controls[0][axis] - &slope * &residuals[0];
            if controls
                .iter()
                .zip(&residuals)
                .all(|(c, r)| c[axis] == &slope * r + &intercept)
            {
                ranges[axis] = [intercept.clone(), intercept];
            }
        }
    }
    Ok(CoordinateBounds {
        origin,
        u,
        v,
        ranges,
    })
}
impl CoordinateBounds {
    /// Used only after an independent exact common-root proof for this root.
    pub(super) fn constrain_edge(&mut self, edge: usize) {
        let x = q((edge % 2) as f64);
        self.ranges[edge / 2] = [x.clone(), x];
    }
    /// Clip only after an exact sign proves the root lies on the inward
    /// side of this edge. This is an enclosure, not a snapped coordinate.
    pub(super) fn constrain_inward(&mut self, edge: usize) {
        let r = &mut self.ranges[edge / 2];
        if edge % 2 == 0 {
            r[0] = r[0].clone().max(q(0.));
        } else {
            r[1] = r[1].clone().min(q(1.));
        }
    }
    pub(super) fn contact(
        &self,
        s: &Surface,
        span: usize,
        multiplicity: usize,
        t: I,
        p: &[I],
        raw: [I; 2],
    ) -> Result<Decision> {
        let ranges: [[Q; 2]; 2] = std::array::from_fn(|axis| {
            [
                self.ranges[axis][0].clone().max(q(raw[axis].lo)),
                self.ranges[axis][1].clone().min(q(raw[axis].hi)),
            ]
        });
        if ranges.iter().any(|r| r[0] > r[1]) {
            return Err(fail());
        }
        if ranges.iter().any(|r| r[1] < Q::zero() || r[0] > q(1.)) {
            return Ok(Decision::Outside);
        }
        if ranges.iter().any(|r| r[0] < Q::zero() || r[1] > q(1.)) {
            return Ok(Decision::Unresolved);
        }
        let side = |axis: usize| {
            if ranges[axis][0] == ranges[axis][1] {
                if ranges[axis][0].is_zero() {
                    Some(0)
                } else if ranges[axis][0] == q(1.) {
                    Some(1)
                } else {
                    None
                }
            } else {
                None
            }
        };
        let (sa, sb) = (side(0), side(1));
        let natural = |axis: usize, r: &[Q; 2], w0: f64, w1: f64| -> Result<[f64; 2]> {
            let k = if axis == 0 { &s.knots_u } else { &s.knots_v };
            let width = q(k[2]) - q(k[1]);
            Ok([
                bound(&(q(k[1]) + &width * clock(&r[0], w0, w1)))?[0],
                bound(&(q(k[1]) + width * clock(&r[1], w0, w1)))?[1],
            ])
        };
        let (uv, uv_proof) = if let Some(i) = sa {
            let edge = s.knots_u[1 + i];
            let v = if let Some(j) = sb {
                [s.knots_v[1 + j]; 2]
            } else {
                natural(1, &ranges[1], s.weights[i][0], s.weights[i][1])?
            };
            (
                [[edge, edge], v],
                OriginalPatchUvProof::OriginalAffineCoordinateBounds,
            )
        } else if let Some(j) = sb {
            let edge = s.knots_v[1 + j];
            (
                [
                    natural(0, &ranges[0], s.weights[0][j], s.weights[1][j])?,
                    [edge, edge],
                ],
                OriginalPatchUvProof::OriginalAffineCoordinateBounds,
            )
        } else {
            (
                [[s.knots_u[1], s.knots_u[2]], [s.knots_v[1], s.knots_v[2]]],
                OriginalPatchUvProof::PositiveBilinearGlobalInverse,
            )
        };
        let mut position = [[0.; 2]; 3];
        for k in 0..3 {
            let ar = [&self.u[k] * &ranges[0][0], &self.u[k] * &ranges[0][1]];
            let br = [&self.v[k] * &ranges[1][0], &self.v[k] * &ranges[1][1]];
            let lo = &self.origin[k]
                + ar[0].clone().min(ar[1].clone())
                + br[0].clone().min(br[1].clone());
            let hi = &self.origin[k]
                + ar[0].clone().max(ar[1].clone())
                + br[0].clone().max(br[1].clone());
            position[k] = [bound(&lo)?[0].max(p[k].lo), bound(&hi)?[1].min(p[k].hi)];
            if position[k][0] > position[k][1] {
                return Err(fail());
            }
        }
        Ok(Decision::Contact(OriginalPatchContact {
            uv_proof,
            parameter_box: [[t.lo, t.hi], uv[0], uv[1]],
            position_bounds: position,
            source_spans: vec![span],
            multiplicities: vec![multiplicity],
        }))
    }
}
