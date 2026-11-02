//! Exact finite-chart projective transformations, preserving the original basis.
use crate::core::bounded_rational::{Error, Work};
use crate::{Result, check, curve::Curve, surface::Surface};
use num_rational::BigRational as Q;
use num_traits::{Signed, Zero};
mod proof;
/// Row-major homogeneous map: `[x*w,y*w,z*w,w] -> M * H`.
pub type Matrix = [[f64; 4]; 4];
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    ResourceLimit,
    NotRepresentable,
    SingularMatrix,
    /// Mixed/zero transformed control weights do not prove a finite chart.
    /// This can refuse a finite image whose denominator needs a sharper proof.
    UnprovedDenominator,
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
pub struct Report<T> {
    /// Absent on refusal. No partial geometry or topology authority is issued.
    pub geometry: Option<T>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}
type PResult<T> = std::result::Result<T, Refusal>;
fn input(matrix: &Matrix, max_work: u64) -> Result<()> {
    check(
        matrix.iter().flatten().all(|x| x.is_finite()),
        "Projective matrix must be finite",
    )?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact projective work must be in [1,1000000]",
    )
}
fn matrix(m: &Matrix, w: &mut Work) -> PResult<Vec<Vec<Q>>> {
    let m = m
        .iter()
        .map(|row| row.iter().map(|&x| w.lift(x).map_err(Into::into)).collect())
        .collect::<PResult<Vec<Vec<Q>>>>()?;
    proof::invertible(&m, w)?;
    Ok(m)
}
fn transformed(
    points: &[Vec<f64>],
    weights: &[f64],
    m: &[Vec<Q>],
    w: &mut Work,
) -> PResult<Vec<Vec<Q>>> {
    let mut out = vec![Vec::new(); 4];
    for (p, &weight) in points.iter().zip(weights) {
        let weight = w.lift(weight)?;
        let mut h = Vec::new();
        for &x in p {
            let x = w.lift(x)?;
            h.push(w.op(&x, &weight, |a, b| a * b)?);
        }
        h.push(weight);
        for row in 0..4 {
            let mut sum = Q::zero();
            for axis in 0..4 {
                let term = w.op(&m[row][axis], &h[axis], |a, b| a * b)?;
                sum = w.op(&sum, &term, |a, b| a + b)?;
            }
            out[row].push(sum);
        }
    }
    if !(out[3].iter().all(|x| x.is_positive()) || out[3].iter().all(|x| x.is_negative())) {
        return Err(Refusal::UnprovedDenominator);
    }
    Ok(out)
}
fn store(h: &[Vec<Q>], w: &mut Work) -> PResult<(Vec<Vec<f64>>, Vec<f64>)> {
    super::rational_edit::store(h, w).map_err(|e| match e {
        super::rational_factor::Refusal::ResourceLimit => Refusal::ResourceLimit,
        super::rational_factor::Refusal::NotRepresentable => Refusal::NotRepresentable,
        super::rational_factor::Refusal::NonPositiveWeights => Refusal::UnprovedDenominator,
        _ => Refusal::InvalidOutput,
    })
}
fn finish<T>(out: PResult<T>, w: Work) -> Report<T> {
    match out {
        Ok(geometry) => Report {
            geometry: Some(geometry),
            work_used: w.used,
            refusal: None,
        },
        Err(refusal) => Report {
            geometry: None,
            work_used: w.used,
            refusal: Some(refusal),
        },
    }
}
/// Apply an invertible projective map to a validated 3D curve, without changing
/// knots, degree, periodic storage or the parameter clock. All transformed
/// homogeneous weights must have one strict sign, proving no pole anywhere on
/// the original domain. Output binary64 poles/weights must be exact, otherwise
/// refuse atomically under the existing 4096-bit/1m-work and geometry bounds.
/// No bitwise evaluator, topology or external certificate transfer is promised.
pub fn curve(source: &Curve, m: Matrix, max_work: u64) -> Result<Report<Curve>> {
    source.validate()?;
    check(
        source.control_points[0].len() == 3,
        "Exact projective curves must be 3D",
    )?;
    input(&m, max_work)?;
    let mut w = Work {
        used: 0,
        limit: max_work,
    };
    let out = (|| {
        let m = matrix(&m, &mut w)?;
        let h = transformed(&source.control_points, &source.weights, &m, &mut w)?;
        let (points, weights) = store(&h, &mut w)?;
        proof::stored(&h, &points, &weights, &mut w)?;
        let mut c = source.clone();
        c.control_points = points;
        c.weights = weights;
        if c.validate().is_err() {
            return Err(Refusal::InvalidOutput);
        }
        Ok(c)
    })();
    Ok(finish(out, w))
}
/// Tensor-product counterpart of [`curve`]. One common homogeneous storage
/// gauge spans the entire net, not separate rows, preserving the full UV map.
pub fn surface(source: &Surface, m: Matrix, max_work: u64) -> Result<Report<Surface>> {
    source.validate()?;
    input(&m, max_work)?;
    let mut w = Work {
        used: 0,
        limit: max_work,
    };
    let out = (|| {
        let m = matrix(&m, &mut w)?;
        let points = source
            .control_points
            .iter()
            .flatten()
            .cloned()
            .collect::<Vec<_>>();
        let weights = source.weights.iter().flatten().copied().collect::<Vec<_>>();
        let h = transformed(&points, &weights, &m, &mut w)?;
        let (points, weights) = store(&h, &mut w)?;
        proof::stored(&h, &points, &weights, &mut w)?;
        let count = source.control_points[0].len();
        let mut s = source.clone();
        s.control_points = points.chunks(count).map(|r| r.to_vec()).collect();
        s.weights = weights.chunks(count).map(|r| r.to_vec()).collect();
        if s.validate().is_err() {
            return Err(Refusal::InvalidOutput);
        }
        Ok(s)
    })();
    Ok(finish(out, w))
}
