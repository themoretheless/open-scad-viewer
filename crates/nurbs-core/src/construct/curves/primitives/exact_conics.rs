//! Full affine ellipses with checked binary-rational storage and complete proof.
use crate::{
    Result, check,
    core::bounded_rational::{Error, Work},
    curve::Curve,
};
use num_rational::BigRational as Q;
mod proof;
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
#[derive(Clone, Debug)]
pub struct Report {
    /// None on every refusal. No partly constructed curve is published.
    pub curve: Option<Curve>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}
type PResult<T> = std::result::Result<T, Refusal>;
const POLES: [[i32; 2]; 9] = [
    [1, 0],
    [1, 1],
    [0, 1],
    [-1, 1],
    [-1, 0],
    [-1, -1],
    [0, -1],
    [1, -1],
    [1, 0],
];
const WEIGHTS: [f64; 9] = [1., 1., 2., 1., 1., 1., 2., 1., 1.];
fn construct(center: [f64; 3], a: [f64; 3], b: [f64; 3], work: &mut Work) -> PResult<Curve> {
    let mut points = Vec::new();
    for pole in POLES {
        let mut p = Vec::new();
        for axis in 0..3 {
            let c = work.lift(center[axis])?;
            let x = work.lift(a[axis])?;
            let y = work.lift(b[axis])?;
            let x = work.op(&x, &Q::from_integer(pole[0].into()), |x, n| x * n)?;
            let y = work.op(&y, &Q::from_integer(pole[1].into()), |y, n| y * n)?;
            let v = work.op(&c, &x, |x, y| x + y)?;
            let v = work.op(&v, &y, |x, y| x + y)?;
            p.push(work.float(&v)?);
        }
        points.push(p);
    }
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1., 1., 1.],
        control_points: points,
        weights: WEIGHTS.to_vec(),
        periodic: false,
    };
    if c.validate().is_err() {
        return Err(Refusal::InvalidOutput);
    }
    Ok(c)
}
/// Construct `center + axis_u*cos(theta) + axis_v*sin(theta)` as a closed
/// four-quarter rational curve, without evaluating trigonometry. The axes may
/// be oblique in3D but must be exactly independent. Positive rational weights
/// and nine literal poles are proved against the complete implicit conic.
/// Domain[0,1], clamped closed storage, variable angular speed; no promise of
/// the legacy trigonometric clock or bitwise evaluator identity. Existing
/// geometric limits and the1m-work/4096-bit exact arithmetic guards apply.
pub fn ellipse(
    center: [f64; 3],
    axis_u: [f64; 3],
    axis_v: [f64; 3],
    max_work: u64,
) -> Result<Report> {
    check(
        center
            .iter()
            .chain(&axis_u)
            .chain(&axis_v)
            .all(|x| x.is_finite()),
        "Exact conic inputs must be finite",
    )?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact conic work must be in1..=1000000",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let result = (|| -> PResult<Curve> {
        let c = construct(center, axis_u, axis_v, &mut work)?;
        proof::verify(center, axis_u, axis_v, &c, &mut work)?;
        Ok(c)
    })();
    Ok(match result {
        Ok(c) => Report {
            curve: Some(c),
            work_used: work.used,
            refusal: None,
        },
        Err(r) => Report {
            curve: None,
            work_used: work.used,
            refusal: Some(r),
        },
    })
}
