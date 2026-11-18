//! Exact affine cylindrical/conical sides with explicit apex metadata.
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    Start,
    End,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub surface: Option<Surface>,
    /// Successful cone sides have one intrinsically singular, collapsed end.
    /// This is geometric metadata, not a B-rep vertex or a smooth pole claim.
    pub collapsed_end: Option<End>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}
type PResult<T> = std::result::Result<T, Refusal>;
const V_KNOTS: [f64; 4] = [0., 0., 1., 1.];
fn construct(
    center: [f64; 3],
    axes: [[f64; 3]; 3],
    radii: [f64; 2],
    reference: &Template,
    w: &mut Work,
) -> PResult<Surface> {
    let c = reference.circle();
    let mut points = Vec::new();
    let mut weights = Vec::new();
    for i in 0..9 {
        let mut row = Vec::new();
        let mut row_weights = Vec::new();
        for j in 0..2 {
            let x = w.lift(c.control_points[i][0])?;
            let y = w.lift(c.control_points[i][1])?;
            let radius = w.lift(radii[j])?;
            let local = [
                w.op(&x, &radius, |a, b| a * b)?,
                w.op(&y, &radius, |a, b| a * b)?,
                w.lift(j as f64)?,
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
            let weight = w.lift(c.weights[i])?;
            row_weights.push(w.float(&weight)?);
        }
        points.push(row);
        weights.push(row_weights);
    }
    let s = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: c.knots.clone(),
        knots_v: V_KNOTS.to_vec(),
        control_points: points,
        weights,
        periodic_u: false,
        periodic_v: false,
    };
    s.validate().map_err(|_| Refusal::InvalidOutput)?;
    Ok(s)
}
/// Full affine image of (r(v)*cos(u),r(v)*sin(u),v), r(v)=(1-v)*r0+v*r1.
/// Three exactly independent authored axes; finite nonnegative end radii, not
/// both zero. Equal radii form a cylinder; one zero end forms a sharp cone apex.
/// Degree2×1, 9×2 net, domain [0,1]², variable rational angular clock, no caps.
/// Exact binary64 storage or atomic refusal under one shared 1m-work ledger
/// (including fresh circle proof) and the unchanged 4096-bit arithmetic guard.
pub fn side(
    center: [f64; 3],
    axes: [[f64; 3]; 3],
    radii: [f64; 2],
    max_work: u64,
) -> Result<Report> {
    check(
        center
            .iter()
            .chain(axes.iter().flatten())
            .chain(&radii)
            .all(|x| x.is_finite()),
        "Exact frustum inputs must be finite",
    )?;
    check(
        radii.iter().all(|&x| x >= 0.) && radii.iter().any(|&x| x > 0.),
        "Exact frustum radii must be nonnegative and not both zero",
    )?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact frustum work must be in1..=1000000",
    )?;
    let mut w = Work {
        used: 0,
        limit: max_work,
    };
    let result = (|| -> PResult<Surface> {
        let reference = Template::load(&mut w)?;
        let s = construct(center, axes, radii, &reference, &mut w)?;
        proof::verify(center, axes, radii, &reference, &s, &mut w)?;
        Ok(s)
    })();
    Ok(match result {
        Ok(s) => Report {
            surface: Some(s),
            collapsed_end: if radii[0] == 0. {
                Some(End::Start)
            } else if radii[1] == 0. {
                Some(End::End)
            } else {
                None
            },
            work_used: w.used,
            refusal: None,
        },
        Err(e) => Report {
            surface: None,
            collapsed_end: None,
            work_used: w.used,
            refusal: Some(e),
        },
    })
}
