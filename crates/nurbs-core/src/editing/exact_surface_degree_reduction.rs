//! Exact one-degree tensor reduction with unchanged complete UV parameter law.
use crate::{Result,surface::{Surface,Axis}};
pub use super::exact_tensor_edit::Refusal;
#[derive(Clone,Debug)]
pub struct Report {
    pub surface:Surface,
    pub reduced:bool,
    pub refusal:Option<Refusal>,
    pub rows_verified:usize,
    pub work_used:u64,
}
/// Lower one axis degree for clamped nonperiodic piecewise Bezier storage.
/// The unedited axis is retained, including its periodic representation.
/// Both row parameter laws and their denominator functions must be identical.
/// On any refusal the complete source is returned, without partial net edits.
pub fn reduce(source:&Surface,axis:Axis,max_work:u64)->Result<Report>{
    let r=super::exact_tensor_edit::apply(source,axis,0.,max_work,|curve,budget|{
        let p=super::exact_degree_reduction::reduce(curve,budget)?;
        Ok(super::exact_tensor_edit::CurveProposal{curve:p.curve,changed:p.reduced,refusal:p.refusal,work_used:p.work_used})
    })?;
    Ok(Report{surface:r.surface,reduced:r.changed,refusal:r.refusal,rows_verified:r.rows_verified,work_used:r.work_used})
}

/// General smooth selected-axis layout, with the same atomic row AND
/// denominator proof. Extraction and compaction are only curve proposals.
pub fn reduce_smooth(source:&Surface,axis:Axis,max_work:u64)->Result<Report>{
    let r=super::exact_tensor_edit::apply(source,axis,0.,max_work,|curve,budget|{
        let p=super::exact_degree_reduction::reduce_smooth(curve,budget)?;
        Ok(super::exact_tensor_edit::CurveProposal{curve:p.curve,changed:p.reduced,refusal:p.refusal,work_used:p.work_used})
    })?;
    Ok(Report{surface:r.surface,reduced:r.changed,refusal:r.refusal,rows_verified:r.rows_verified,work_used:r.work_used})
}
