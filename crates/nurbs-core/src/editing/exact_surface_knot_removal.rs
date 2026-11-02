//! Atomic exact tensor edit: row geometry AND polynomial denominators.
use crate::{Result,check,curve::Curve,surface::{Surface,Axis}};
use super::exact_knot_removal;

pub use super::exact_tensor_edit::Refusal;
#[derive(Clone,Debug)]
pub struct Report {
    /// Unchanged source clone on every refusal, including late-row failures.
    pub surface:Surface,
    pub removed:bool,
    pub refusal:Option<Refusal>,
    pub rows_verified:usize,
    pub work_used:u64,
}
/// Remove one existing interior knot on a clamped nonperiodic axis.
/// Both row geometry and denominator must remain exactly identical.
pub fn remove(source:&Surface,axis:Axis,knot:f64,max_work:u64)->Result<Report>{
    let r=super::exact_tensor_edit::apply(source,axis,knot,max_work,|curve,budget|{
        let p=exact_knot_removal::remove(curve,knot,budget)?;
        Ok(super::exact_tensor_edit::CurveProposal{curve:p.curve,changed:p.removed,refusal:p.refusal,work_used:p.work_used})
    })?;
    Ok(Report{surface:r.surface,removed:r.changed,refusal:r.refusal,rows_verified:r.rows_verified,work_used:r.work_used})
}
