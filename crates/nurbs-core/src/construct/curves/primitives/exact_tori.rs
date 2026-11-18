//! Full affine ring tori with checked rational storage and product proof.
use super::exact_circle::Template;
use crate::{
    Result, check,
    core::bounded_rational::{Error, Work},
    surface::Surface,
};
use num_rational::BigRational as Q;
mod proof;
#[cfg(test)]
mod tests;
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
    /// None on every refusal, including late verification exhaustion.
    pub surface: Option<Surface>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}
type PResult<T> = std::result::Result<T, Refusal>;
fn construct(
    center: [f64; 3],
    axes: [[f64; 3]; 3],
    major: f64,
    minor: f64,
    reference: &Template,
    w: &mut Work,
) -> PResult<Surface> {
    let c = reference.circle();
    let major = w.lift(major)?;
    let minor = w.lift(minor)?;
    let mut points = Vec::new();
    let mut weights = Vec::new();
    for i in 0..9 {
        let mut row = Vec::new();
        let mut row_weights = Vec::new();
        for j in 0..9 {
            let x = w.lift(c.control_points[i][0])?;
            let y = w.lift(c.control_points[i][1])?;
            let xm = w.lift(c.control_points[j][0])?;
            let ym = w.lift(c.control_points[j][1])?;
            let radius = w.op(&minor, &xm, |a, b| a * b)?;
            let radius = w.op(&major, &radius, |a, b| a + b)?;
            let local = [
                w.op(&x, &radius, |a, b| a * b)?,
                w.op(&y, &radius, |a, b| a * b)?,
                w.op(&minor, &ym, |a, b| a * b)?,
            ];
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
    let s = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: c.knots.clone(),
        knots_v: c.knots.clone(),
        control_points: points,
        weights,
        periodic_u: false,
        periodic_v: false,
    };
    s.validate().map_err(|_| Refusal::InvalidOutput)?;
    Ok(s)
}
/// Entire affine image of ((R+r*cos(v))*cos(u), (R+r*cos(v))*sin(u),r*sin(v)).
/// The three authored axes must be exactly independent, R>r>0. Both angles
/// traverse full circles with variable rational clocks over domain [0,1]².
/// Output is a closed 9×9 quadratic surface with two repeated seams, not a Solid.
/// Exact binary64 poles/positive weights or atomic refusal; one shared 1m-work
/// ledger includes reference proof, with the unchanged 4096-bit rational guard.
pub fn ring(
    center: [f64; 3],
    axes: [[f64; 3]; 3],
    major: f64,
    minor: f64,
    max_work: u64,
) -> Result<Report> {
    check(
        center
            .iter()
            .chain(axes.iter().flatten())
            .all(|x| x.is_finite())
            && major.is_finite()
            && minor.is_finite(),
        "Exact torus inputs must be finite",
    )?;
    check(
        major > minor && minor > 0.,
        "Exact ring torus requires major > minor > 0",
    )?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact torus work must be in1..=1000000",
    )?;
    let mut w = Work {
        used: 0,
        limit: max_work,
    };
    let result = (|| -> PResult<Surface> {
        let reference = Template::load(&mut w)?;
        let s = construct(center, axes, major, minor, &reference, &mut w)?;
        proof::verify(center, axes, major, minor, &reference, &s, &mut w)?;
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
