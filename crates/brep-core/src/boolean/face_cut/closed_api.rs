//! Preserve both legacy cell budgets and explicit complete closed-cut budgets.
use super::*;
mod sealed { pub trait Sealed {} }
pub trait ClosedCutBudget: sealed::Sealed {
    type Output;
    fn partition(self, source:&Model,face:usize,path:&[Segment],tolerance_uv:f64)->Result<Self::Output>;
}
impl sealed::Sealed for ClosedCutLimits {}
impl ClosedCutBudget for ClosedCutLimits {
    type Output=ClosedPartition;
    fn partition(self,source:&Model,face:usize,path:&[Segment],tolerance_uv:f64)->Result<Self::Output>{
        closed::partition_closed(source,face,path,tolerance_uv,self)
    }
}
impl sealed::Sealed for usize {}
impl ClosedCutBudget for usize {
    type Output=Partition;
    fn partition(self,source:&Model,face:usize,path:&[Segment],tolerance_uv:f64)->Result<Self::Output>{
        legacy_closed::partition_closed(source,face,path,tolerance_uv,self)
    }
}
/// A scalar cell budget retains the legacy Partition contract. Explicit
/// ClosedCutLimits select the complete reference-owned ClosedPartition contract.
pub fn partition_closed<B:ClosedCutBudget>(source:&Model,face:usize,path:&[Segment],tolerance_uv:f64,budget:B)->Result<B::Output>{
    budget.partition(source,face,path,tolerance_uv)
}
