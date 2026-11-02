//! Structural C0 concatenation retaining both original absolute parameter laws.
use crate::{Result,curve::Curve};
#[derive(Clone,Debug,PartialEq,Eq)]
pub enum Refusal {
    Periodic,
    DegreeMismatch,
    DimensionMismatch,
    NonClamped,
    NonAdjacentDomain,
    SeamMismatch,
    ControlLimit,
    InvalidOutput,
}
#[derive(Clone,Debug)]
pub struct Report {
    /// No approximate or partly merged curve is emitted on refusal.
    pub curve:Option<Curve>,
    pub refusal:Option<Refusal>,
}
/// Merge same-degree clamped nonperiodic curves with exactly adjacent domains
/// and identical shared Cartesian control AND stored weight. No normalization,
/// fitting, snapping, gauge conversion or geometric matching is performed.
/// The output retains the seam at degree multiplicity; optional exact knot
/// simplification is separate. No G1/G2, simplicity or wire-topology claim.
/// Bounded structural copies use the unchanged256-control limit, with no
/// numeric geometry arithmetic or additional predicate/work budget.
pub fn merge(left:&Curve,right:&Curve)->Result<Report>{
    left.validate()?;right.validate()?;
    let refuse=|reason|Ok(Report{curve:None,refusal:Some(reason)});
    if left.periodic||right.periodic{return refuse(Refusal::Periodic)}
    let p=left.degree;
    if p!=right.degree{return refuse(Refusal::DegreeMismatch)}
    if left.control_points[0].len()!=right.control_points[0].len(){return refuse(Refusal::DimensionMismatch)}
    let na=left.control_points.len();let nb=right.control_points.len();
    for curve in [left,right] {
        let n=curve.control_points.len();let [a,b]=curve.domain();
        if !curve.knots[..=p].iter().all(|&u|u==a)||!curve.knots[n..].iter().all(|&u|u==b){return refuse(Refusal::NonClamped)}
    }
    let seam=left.domain()[1];
    if seam!=right.domain()[0]{return refuse(Refusal::NonAdjacentDomain)}
    if na+nb-1>256{return refuse(Refusal::ControlLimit)}
    if left.control_points[na-1]!=right.control_points[0]||left.weights[na-1]!=right.weights[0]{return refuse(Refusal::SeamMismatch)}
    let mut curve=left.clone();curve.knots.truncate(left.knots.len()-p-1);
    curve.knots.extend(std::iter::repeat_n(seam,p));curve.knots.extend_from_slice(&right.knots[p+1..]);
    curve.control_points.extend(right.control_points.iter().skip(1).cloned());curve.weights.extend_from_slice(&right.weights[1..]);
    if curve.validate().is_err(){return refuse(Refusal::InvalidOutput)}
    Ok(Report{curve:Some(curve),refusal:None})
}
