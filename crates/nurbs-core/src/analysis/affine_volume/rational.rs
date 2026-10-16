//! Original rational affine laws. Positive homogeneous weights, original
//! coefficient derivative numerators and no sampled frame interpolation.
use super::*;
use crate::numerics::bernstein_tensor::{Net, derivative, evaluate_line, minus, product};
mod factor;
pub use factor::ColumnFactor;
pub(super) use factor::prove_factor;

/// One fixed affine coordinate change for the ENTIRE original volume. Rows
/// and offset are literal proposal values; determinant excludes zero by outward
/// arithmetic. Coefficients are transformed as intervals, never rounded poles.
#[derive(Clone, Debug)]
pub struct ProofCoordinates {
    pub rows: [[f64; 3]; 3],
    pub offset: [f64; 3],
    pub determinant: [f64; 2],
    /// Exact original coefficient identity A_dependent=scale*A_base+constant.
    pub column_factor: Option<ColumnFactor>,
}
#[derive(Clone, Debug)]
pub struct PreconditionedReport {
    pub volume: IdealVolumeReport,
    /// None means that no invertible proposal was established and the original
    /// world-coordinate atlas was used. All attempts share the volume ledger.
    pub coordinates: Option<ProofCoordinates>,
}

/// One clamped rational Bezier pose law, degree 1..8. Adjacent pieces occupy
/// equal NORMALIZED traversal intervals. Every one-sided derivative is owned.
#[derive(Clone, Debug)]
pub struct RationalPiece {
    pub controls: Vec<Pose>,
    pub weights: Vec<f64>,
}
impl RationalPiece {
    pub(super) fn valid(&self) -> bool {
        (2..=9).contains(&self.controls.len())
            && self.weights.len() == self.controls.len()
            && self
                .weights
                .iter()
                .all(|w| w.is_finite() && (1e-50..=1e9).contains(w))
            && self
                .controls
                .iter()
                .flatten()
                .flatten()
                .all(|x| x.is_finite() && x.abs() <= 1e9)
    }
}
fn scalar(values: &[f64], weights: &[f64], t: I, count: usize) -> Result<[I; 2]> {
    if values.iter().all(|x| *x == values[0]) {
        return Ok([I::point(values[0]), I::point(0.)]);
    }
    let degree = values.len() - 1;
    if weights.iter().all(|w| *w == weights[0]) {
        let d = values
            .windows(2)
            .map(|p| {
                I::point(p[1])
                    .sub(I::point(p[0]))?
                    .mul(I::point((degree * count) as f64))
            })
            .collect::<Result<Vec<_>>>()?;
        return Ok([
            bezier(values.iter().copied().map(I::point).collect(), t)?,
            bezier(d, t)?,
        ]);
    }
    let w: Net = weights.iter().map(|w| vec![I::point(*w)]).collect();
    let h: Net = values
        .iter()
        .zip(weights)
        .map(|(x, w)| Ok(vec![I::point(*x).mul(I::point(*w))?]))
        .collect::<Result<_>>()?;
    let dh = derivative(&h, 0)?;
    let dw = derivative(&w, 0)?;
    let numerator = minus(product(&dh, &w)?, product(&h, &dw)?)?;
    let denominator = evaluate_line(w.iter().map(|r| r[0]).collect(), t)?;
    let value = evaluate_line(h.iter().map(|r| r[0]).collect(), t)?
        .div(denominator)?
        .intersect(
            values.iter().copied().fold(f64::INFINITY, f64::min),
            values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        )?;
    let first = evaluate_line(numerator.iter().map(|r| r[0]).collect(), t)?
        .div(denominator.mul(denominator)?)?
        .mul(I::point(count as f64))?;
    Ok([value, first])
}
// Original coefficient linear combinations, outward enclosed BEFORE rational
// evaluation and the carrier product. Transforming world boxes afterwards loses
// the correlation of common radial factors and cannot prove the same theorem.
fn enclosed_scalar(
    values: &[I],
    weights: &[f64],
    t: I,
    count: usize,
    constant: bool,
) -> Result<[I; 2]> {
    if constant {
        return Ok([values[0], I::point(0.)]);
    }
    if weights.iter().all(|w| *w == weights[0]) {
        let d = values
            .windows(2)
            .map(|p| {
                p[1].sub(p[0])?
                    .mul(I::point(((values.len() - 1) * count) as f64))
            })
            .collect::<Result<Vec<_>>>()?;
        return Ok([bezier(values.to_vec(), t)?, bezier(d, t)?]);
    }
    let w: Net = weights.iter().map(|w| vec![I::point(*w)]).collect();
    let h: Net = values
        .iter()
        .zip(weights)
        .map(|(x, w)| Ok(vec![x.mul(I::point(*w))?]))
        .collect::<Result<_>>()?;
    let numerator = minus(
        product(&derivative(&h, 0)?, &w)?,
        product(&h, &derivative(&w, 0)?)?,
    )?;
    let denominator = evaluate_line(w.iter().map(|r| r[0]).collect(), t)?;
    Ok([
        evaluate_line(h.iter().map(|r| r[0]).collect(), t)?
            .div(denominator)?
            .intersect(
                values.iter().map(|x| x.lo).fold(f64::INFINITY, f64::min),
                values
                    .iter()
                    .map(|x| x.hi)
                    .fold(f64::NEG_INFINITY, f64::max),
            )?,
        evaluate_line(numerator.iter().map(|r| r[0]).collect(), t)?
            .div(denominator.mul(denominator)?)?
            .mul(I::point(count as f64))?,
    ])
}
pub(super) fn pose_in_coordinates(
    p: &RationalPiece,
    t: I,
    count: usize,
    c: &ProofCoordinates,
) -> Result<EnclosedPose> {
    let mut values = [[[I::point(0.); 3]; 2]; 4];
    for col in 0..4 {
        // Literal original vector equality proves a constant coefficient law;
        // equality of overlapping transformed interval bounds would not.
        let replacement = c
            .column_factor
            .as_ref()
            .filter(|f| f.dependent == col)
            .map(|f| f.constant);
        let constant =
            replacement.is_some() || p.controls.iter().all(|x| x[col] == p.controls[0][col]);
        for row in 0..3 {
            let controls = p
                .controls
                .iter()
                .map(|pose| {
                    let mut v = replacement.unwrap_or(pose[col]).map(I::point);
                    if col == 3 {
                        for k in 0..3 {
                            v[k] = if pose[col][k] == c.offset[k] {
                                I::point(0.)
                            } else {
                                v[k].sub(I::point(c.offset[k]))?
                            };
                        }
                    }
                    iv::dot_tight(c.rows[row].map(I::point), v)
                })
                .collect::<Result<Vec<_>>>()?;
            let jet = enclosed_scalar(&controls, &p.weights, t, count, constant)?;
            for order in 0..2 {
                values[col][order][row] = jet[order];
            }
        }
    }
    Ok(EnclosedPose {
        origin: values[3],
        linear: [values[0], values[1], values[2]],
    })
}
pub(super) fn pose_on(p: &RationalPiece, t: I, count: usize) -> Result<EnclosedPose> {
    let mut values = [[[I::point(0.); 3]; 2]; 4];
    for col in 0..4 {
        for row in 0..3 {
            let controls = p.controls.iter().map(|p| p[col][row]).collect::<Vec<_>>();
            let j = scalar(&controls, &p.weights, t, count)?;
            for order in 0..2 {
                values[col][order][row] = j[order];
            }
        }
    }
    Ok(EnclosedPose {
        origin: values[3],
        linear: [values[0], values[1], values[2]],
    })
}
/// Whole ORIGINAL rational-motion filled volume. A positive result is global
/// regularity/embedding/topology of this exact coefficient law, not a sampled
/// approximation. Material trims, BRep/cap binding, orientation and Gk remain
/// independent native owner obligations. No success report is accepted as input.
pub fn certify_rational(
    carrier: &Surface,
    origin: [f64; 3],
    pieces: &[RationalPiece],
    closed: bool,
    projection: [[f64; 3]; 3],
    max_cells: usize,
    max_exact_work: u64,
) -> Result<IdealVolumeReport> {
    certify_motion(
        carrier,
        origin,
        Motion::Rational(pieces),
        closed,
        projection,
        max_cells,
        max_exact_work,
    )
}
/// Same original-map theorem with one independently verified affine coordinate
/// proposal applied to ORIGINAL coefficients. All source queries, failed
/// proposals and determinant checks consume the same explicit cell budget.
/// The returned coordinates preserve world embedding/topology by invertibility;
/// world orientation must still use the original Jacobian sign function.
pub fn certify_rational_preconditioned(
    carrier: &Surface,
    origin: [f64; 3],
    pieces: &[RationalPiece],
    closed: bool,
    projection: [[f64; 3]; 3],
    max_cells: usize,
    max_exact_work: u64,
) -> Result<PreconditionedReport> {
    let mut coordinates = None;
    let volume = certify_motion_impl(
        carrier,
        origin,
        Motion::Rational(pieces),
        closed,
        projection,
        max_cells,
        max_exact_work,
        true,
        &mut coordinates,
    )?;
    Ok(PreconditionedReport {
        volume,
        coordinates,
    })
}
/// Conditional world Jacobian sign; GLOBAL meaning requires the same exact
/// rational source's independent successful filled-volume certificate.
pub fn jacobian_orientation_rational(
    carrier: &Surface,
    origin: [f64; 3],
    pieces: &[RationalPiece],
    point: [f64; 3],
    max_cells: usize,
) -> Result<OrientationReport> {
    orientation_motion(carrier, origin, Motion::Rational(pieces), point, max_cells)
}
#[cfg(test)]
mod tests;
