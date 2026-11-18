//! Checked affine ellipsoids from proved rational circle factors.
use crate::{
    Result, check,
    core::bounded_rational::{Error, Work},
    surface::Surface,
};
mod proof;
#[cfg(test)]
mod tests;
use super::exact_circle::Template;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    ResourceLimit,
    NotRepresentable,
    DependentAxes,
    InvalidOutput,
    UnprovedIdentity,
}
impl From<Error> for Refusal {
    fn from(e: Error) -> Self {
        match e {
            Error::ResourceLimit => Self::ResourceLimit,
            Error::NotRepresentable => Self::NotRepresentable,
        }
    }
}
impl From<super::exact_conics::Refusal> for Refusal {
    fn from(e: super::exact_conics::Refusal) -> Self {
        use super::exact_conics::Refusal as C;
        match e {
            C::ResourceLimit => Self::ResourceLimit,
            C::NotRepresentable => Self::NotRepresentable,
            C::DependentAxes => Self::DependentAxes,
            C::InvalidOutput => Self::InvalidOutput,
            C::UnprovedIdentity => Self::UnprovedIdentity,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Report {
    /// No partial surface is published on refusal.
    pub surface: Option<Surface>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}
type PResult<T> = std::result::Result<T, Refusal>;
const V_KNOTS: [f64; 8] = [0., 0., 0., 0.5, 0.5, 1., 1., 1.];

fn construct(
    center: [f64; 3],
    axes: [[f64; 3]; 3],
    reference: &Template,
    w: &mut Work,
) -> PResult<Surface> {
    let c = reference.circle();
    let mut points = Vec::new();
    let mut weights = Vec::new();
    for i in 0..9 {
        let mut row = Vec::new();
        let mut row_weights = Vec::new();
        for j in 0..5 {
            let x = w.lift(c.control_points[i][0])?;
            let y = w.lift(c.control_points[i][1])?;
            let r = w.lift(c.control_points[j][1])?;
            let z = w.lift(-c.control_points[j][0])?;
            let local = [w.op(&x, &r, |a, b| a * b)?, w.op(&y, &r, |a, b| a * b)?, z];
            let mut point = Vec::new();
            for axis in 0..3 {
                let mut value = w.lift(center[axis])?;
                for k in 0..3 {
                    let a = w.lift(axes[k][axis])?;
                    let term = w.op(&a, &local[k], |a, b| a * b)?;
                    value = w.op(&value, &term, |a, b| a + b)?;
                }
                point.push(w.float(&value)?);
            }
            row.push(point);
            let a = w.lift(c.weights[i])?;
            let b = w.lift(c.weights[j])?;
            let weight = w.op(&a, &b, |a, b| a * b)?;
            row_weights.push(w.float(&weight)?);
        }
        points.push(row);
        weights.push(row_weights);
    }
    let surface = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: c.knots.clone(),
        knots_v: V_KNOTS.to_vec(),
        control_points: points,
        weights,
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate().map_err(|_| Refusal::InvalidOutput)?;
    Ok(surface)
}

/// Entire affine image of the unit sphere under the three authored axes.
/// Axes may be oblique or orientation reversing but must be exactly independent.
/// Domain [0,1]², 9×5 quadratic net, nonperiodic clamped storage, exact longitude
/// seam and two collapsed latitude poles. Variable rational angular clocks.
/// Returns a geometric surface, without solid topology or evaluator-bit claims.
/// Exact binary64 poles/positive weights or atomic refusal, with one shared
/// 1m-work ledger and the existing 4096-bit rational arithmetic guard.
pub fn ellipsoid(center: [f64; 3], axes: [[f64; 3]; 3], max_work: u64) -> Result<Report> {
    check(
        center
            .iter()
            .chain(axes.iter().flatten())
            .all(|x| x.is_finite()),
        "Exact ellipsoid inputs must be finite",
    )?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact ellipsoid work must be in1..=1000000",
    )?;
    let mut w = Work {
        used: 0,
        limit: max_work,
    };
    let result = (|| -> PResult<Surface> {
        let reference = Template::load(&mut w)?;
        let s = construct(center, axes, &reference, &mut w)?;
        proof::verify(center, axes, &reference, &s, &mut w)?;
        Ok(s)
    })();
    Ok(match result {
        Ok(s) => Report {
            surface: Some(s),
            work_used: w.used,
            refusal: None,
        },
        Err(e) => Report {
            surface: None,
            work_used: w.used,
            refusal: Some(e),
        },
    })
}
