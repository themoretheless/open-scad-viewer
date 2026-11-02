//! Full affine planar disks/annuli with exact storage and coverage proof.
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
pub enum RegionKind {
    FilledDisk,
    Annulus,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub surface: Option<Surface>,
    /// None on refusals. A filled disk has a collapsed V=0 parameter boundary
    /// at an intrinsically regular plane point, not a sharp cone apex.
    pub kind: Option<RegionKind>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}
type PResult<T> = std::result::Result<T, Refusal>;
const V_KNOTS: [f64; 4] = [0., 0., 1., 1.];
fn construct(
    center: [f64; 3],
    axes: [[f64; 3]; 2],
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
        for radius in radii {
            let radius = w.lift(radius)?;
            let x = w.lift(c.control_points[i][0])?;
            let y = w.lift(c.control_points[i][1])?;
            let local = [
                w.op(&x, &radius, |a, b| a * b)?,
                w.op(&y, &radius, |a, b| a * b)?,
            ];
            let mut point = Vec::new();
            for axis in 0..3 {
                let mut value = w.lift(center[axis])?;
                for k in 0..2 {
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
/// Full affine planar image of (r(v)*cos(u),r(v)*sin(u)), where r(v)
/// interpolates [inner,outer] linearly over v in[0,1]. Requires exactly
/// independent authored axes and 0<=inner<outer. A zero inner radius gives a
/// filled disk with a smooth plane image at its collapsed parameter center.
/// Degree2×1, 9×2 net, rational angular clock, exact seam and both boundary
/// circles (one collapses for a disk). No B-rep/solid or normal-error claim.
/// Exact binary64 output or atomic refusal; shared1m-work/4096-bit guards.
pub fn radial(
    center: [f64; 3],
    axes: [[f64; 3]; 2],
    radii: [f64; 2],
    max_work: u64,
) -> Result<Report> {
    check(
        center
            .iter()
            .chain(axes.iter().flatten())
            .chain(&radii)
            .all(|x| x.is_finite()),
        "Exact planar region inputs must be finite",
    )?;
    check(
        radii[0] >= 0. && radii[1] > radii[0],
        "Exact planar region requires 0 <= inner < outer",
    )?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact planar region work must be in1..=1000000",
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
            kind: Some(if radii[0] == 0. {
                RegionKind::FilledDisk
            } else {
                RegionKind::Annulus
            }),
            work_used: w.used,
            refusal: None,
        },
        Err(e) => Report {
            surface: None,
            kind: None,
            work_used: w.used,
            refusal: Some(e),
        },
    })
}
