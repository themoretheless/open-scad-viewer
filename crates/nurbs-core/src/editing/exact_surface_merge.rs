//! Structural concatenation: no fitting, snapping or parameter arithmetic.
use crate::{Result,surface::{Surface,Axis}};

#[derive(Clone,Debug,PartialEq,Eq)]
pub enum Refusal {
    SelectedPeriodic,
    DegreeMismatch,
    OtherBasisMismatch,
    NonClamped,
    NonAdjacentDomain,
    SeamMismatch{row:usize},
    ControlLimit,
    InvalidOutput,
}
#[derive(Clone,Debug)]
pub struct Report {
    /// Absent on every refusal; no partial or approximately merged output.
    pub surface:Option<Surface>,
    pub refusal:Option<Refusal>,
}
fn basis(s:&Surface,axis:Axis)->(usize,&[f64],usize,bool){
    match axis {
        Axis::U=>(s.degree_u,&s.knots_u,s.control_points.len(),s.periodic_u),
        Axis::V=>(s.degree_v,&s.knots_v,s.control_points[0].len(),s.periodic_v),
    }
}
fn opposite(axis:Axis)->Axis{match axis{Axis::U=>Axis::V,Axis::V=>Axis::U}}
/// Combine adjacent selected-axis rectangles, retaining both absolute UV laws.
/// Same degrees and unedited basis, clamped nonperiodic selected axes, exactly
/// adjacent domains and identical seam Cartesian controls AND stored weights
/// are required. The result has a degree-multiplicity C0 knot at the seam.
/// Other-axis periodic storage is allowed and unchanged. No seam smoothing,
/// geometric matching/search, B-rep sewing or solid/regularity claim is made.
/// This bounded structural copy uses existing <=256 controls per axis; no
/// numeric geometry calculation or fresh exact-predicate budget is needed.
pub fn merge(left:&Surface,right:&Surface,axis:Axis)->Result<Report>{
    left.validate()?;right.validate()?;
    let refuse=|reason|Ok(Report{surface:None,refusal:Some(reason)});
    let (p,a,na,pa)=basis(left,axis);let (q,b,nb,pb)=basis(right,axis);
    if pa||pb{return refuse(Refusal::SelectedPeriodic)}
    if p!=q{return refuse(Refusal::DegreeMismatch)}
    if basis(left,opposite(axis))!=basis(right,opposite(axis)){return refuse(Refusal::OtherBasisMismatch)}
    for (knots,n) in [(a,na),(b,nb)] {
        if !knots[..=p].iter().all(|&u|u==knots[p])||!knots[n..].iter().all(|&u|u==knots[n]){
            return refuse(Refusal::NonClamped)
        }
    }
    let seam=a[na];
    if seam!=b[p]{return refuse(Refusal::NonAdjacentDomain)}
    if na+nb-1>256{return refuse(Refusal::ControlLimit)}
    let count=basis(left,opposite(axis)).2;
    for row in 0..count {
        let (ap,aw,bp,bw)=match axis {
            Axis::U=>(&left.control_points[na-1][row],left.weights[na-1][row],&right.control_points[0][row],right.weights[0][row]),
            Axis::V=>(&left.control_points[row][na-1],left.weights[row][na-1],&right.control_points[row][0],right.weights[row][0]),
        };
        if ap!=bp||aw!=bw{return refuse(Refusal::SeamMismatch{row})}
    }
    let mut knots=a[..a.len()-p-1].to_vec();
    knots.extend(std::iter::repeat_n(seam,p));knots.extend_from_slice(&b[p+1..]);
    let mut surface=left.clone();
    match axis {
        Axis::U=>{
            surface.knots_u=knots;
            surface.control_points.extend(right.control_points.iter().skip(1).cloned());
            surface.weights.extend(right.weights.iter().skip(1).cloned());
        }
        Axis::V=>{
            surface.knots_v=knots;
            for (a,b) in surface.control_points.iter_mut().zip(&right.control_points){a.extend(b.iter().skip(1).cloned());}
            for (a,b) in surface.weights.iter_mut().zip(&right.weights){a.extend_from_slice(&b[1..]);}
        }
    }
    if surface.validate().is_err(){return refuse(Refusal::InvalidOutput)}
    Ok(Report{surface:Some(surface),refusal:None})
}
