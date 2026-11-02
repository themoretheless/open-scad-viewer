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
    if let Some(reason)=incompatibility(left,right){return Ok(Report{curve:None,refusal:Some(reason)})}
    let mut curve=left.clone();append(&mut curve,right);
    if curve.validate().is_err(){return Ok(Report{curve:None,refusal:Some(Refusal::InvalidOutput)})}
    Ok(Report{curve:Some(curve),refusal:None})
}
fn incompatibility(left:&Curve,right:&Curve)->Option<Refusal>{
    if left.periodic||right.periodic{return Some(Refusal::Periodic)}
    let p=left.degree;
    if p!=right.degree{return Some(Refusal::DegreeMismatch)}
    if left.control_points[0].len()!=right.control_points[0].len(){return Some(Refusal::DimensionMismatch)}
    let na=left.control_points.len();let nb=right.control_points.len();
    for curve in [left,right] {
        let n=curve.control_points.len();let [a,b]=curve.domain();
        if !curve.knots[..=p].iter().all(|&u|u==a)||!curve.knots[n..].iter().all(|&u|u==b){return Some(Refusal::NonClamped)}
    }
    if left.domain()[1]!=right.domain()[0]{return Some(Refusal::NonAdjacentDomain)}
    if na+nb-1>256{return Some(Refusal::ControlLimit)}
    if left.control_points[na-1]!=right.control_points[0]||left.weights[na-1]!=right.weights[0]{return Some(Refusal::SeamMismatch)}
    None
}
// Only called with compatible validated storage. Mutates one private net;
// the chain never allocates another full candidate at each intermediate join.
fn append(curve:&mut Curve,right:&Curve){
    let p=curve.degree;let seam=curve.domain()[1];
    curve.knots.truncate(curve.knots.len()-p-1);
    curve.knots.extend(std::iter::repeat_n(seam,p));curve.knots.extend_from_slice(&right.knots[p+1..]);
    curve.control_points.extend(right.control_points.iter().skip(1).cloned());curve.weights.extend_from_slice(&right.weights[1..]);
}
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct ChainRefusal {
    /// Index of the right source curve at the rejected join (starting at1).
    pub right_source:usize,
    pub reason:Refusal,
}
#[derive(Clone,Debug)]
pub struct ChainReport {
    pub curve:Option<Curve>,
    pub joins_verified:usize,
    pub refusal:Option<ChainRefusal>,
}
/// Atomically assemble an ordered chain with the same exact structural rules.
/// Source count<=256, aggregate source controls<=512 (the maximum input data
/// of the existing two-curve API), output controls<=256. All source curves are
/// validated first. A failed late join emits no partial profile; inputs remain
/// borrowed and unchanged. A single valid source is returned unchanged, and
/// does not assert closure, topology or compatibility with any other source.
pub fn merge_chain(sources:&[Curve])->Result<ChainReport>{
    crate::check(!sources.is_empty()&&sources.len()<=256,"Curve chain needs1..=256 sources")?;
    let total=sources.iter().try_fold(0usize,|n,c|n.checked_add(c.control_points.len()));
    crate::check(total.is_some_and(|n|n<=512),"Curve chain source controls must not exceed512")?;
    for source in sources{source.validate()?;}
    let mut curve=sources[0].clone();let mut verified=0;
    for (index,right) in sources.iter().enumerate().skip(1){
        let failed=|reason|ChainReport{curve:None,joins_verified:verified,refusal:Some(ChainRefusal{right_source:index,reason})};
        if let Some(reason)=incompatibility(&curve,right){return Ok(failed(reason))}
        append(&mut curve,right);
        if curve.validate().is_err(){return Ok(failed(Refusal::InvalidOutput))}
        verified+=1;
    }
    Ok(ChainReport{curve:Some(curve),joins_verified:verified,refusal:None})
}
