//! Regularity of actual retained patches, independent of source-frame and error
//! certificates. Shared work exhaustion never certifies a partial level.
use super::{Level, MultiLevel};
use crate::{Result, check, surface::Surface};

#[derive(Clone, Debug)]
pub struct RetainedRegularityReport {
    pub spanwise_regular: bool,
    pub cells: usize,
    pub unresolved_patches: Vec<usize>,
}

pub fn inspect(patches: &[Surface], max_cells: usize) -> Result<RetainedRegularityReport> {
    check(max_cells <= 100000, "Retained regularity budget exceeds100000")?;
    check(!patches.is_empty() && patches.len() <= 4096, "Invalid retained patch count")?;
    // Invalid geometry must remain an input error, even when work is exhausted.
    for patch in patches { patch.validate()?; }
    let mut report=RetainedRegularityReport {spanwise_regular:false,cells:0,unresolved_patches:Vec::new()};
    for (index, patch) in patches.iter().enumerate() {
        let remaining=max_cells-report.cells;
        if remaining==0 {report.unresolved_patches.push(index);continue;}
        match crate::surface_regularity::inspect(patch,remaining) {
            Ok(r)=>{
                report.cells+=r.cells;
                if !r.spanwise_regular {report.unresolved_patches.push(index);}
            }
            Err(_)=>{
                // Conservative numerical failure: consume the remaining work
                // and leave this and subsequent patches unresolved.
                report.cells=max_cells;
                report.unresolved_patches.push(index);
            }
        }
    }
    report.spanwise_regular=report.unresolved_patches.is_empty();
    Ok(report)
}

impl Level {
    /// Certifies only actual retained surface Jacobians, for every frame mode.
    /// Does not promote previews, certify global embedding, caps or seams.
    pub fn certify_retained_regularity(&self,max_cells:usize)->Result<RetainedRegularityReport>{
        inspect(&self.patches,max_cells)
    }
}
impl MultiLevel {
    /// All profiles share one budget. Profile ownership/nesting is separate.
    pub fn certify_retained_regularity(&self,max_cells:usize)->Result<RetainedRegularityReport>{
        inspect(&self.patches,max_cells)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn patch(fold:bool)->Surface{
        Surface {degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],vec![vec![1.,0.,0.],vec![if fold {-1.} else {1.},1.,0.]]],
            weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false}
    }
    #[cfg(feature="transport")]
    #[test]
    fn retained_regularity_transport_does_not_promote_other_obligations(){
        use value_codec::json;
        let request=json!({"op":"sweep_retained_patch_regularity","patches":[patch(false)],"maxCells":1000});
        let r=crate::transport::dispatch(request.clone()).unwrap();
        assert_eq!(r["surfaceRegularityCertified"],true);
        for field in ["continuousBound","globalEmbeddingCertified","solidCertified"] {assert_eq!(r[field],false);}
        let mut zero=request;zero["maxCells"]=json!(0);
        let r=crate::transport::dispatch(zero).unwrap();
        assert_eq!(r["surfaceRegularityCertified"],false);assert_eq!(r["unresolvedPatches"],json!([0]));
        assert!(crate::transport::dispatch(json!({"op":"sweep_retained_patch_regularity","patches":[],"maxCells":1000})).is_err());
    }
    #[test]
    fn shared_level_budget_and_interior_fold_are_not_partial_certificates(){
        let regular=patch(false);
        let full=inspect(&[regular.clone(),regular.clone()],1000).unwrap();
        assert!(full.spanwise_regular);assert!(full.cells>0);
        let short=inspect(&[regular.clone(),regular],full.cells-1).unwrap();
        assert!(!short.spanwise_regular);assert!(short.cells<=full.cells-1);
        assert!(!short.unresolved_patches.is_empty());
        let folded=inspect(&[patch(true)],1000).unwrap();
        assert!(!folded.spanwise_regular);assert_eq!(folded.unresolved_patches,vec![0]);
        let zero=inspect(&[patch(false),patch(false)],0).unwrap();
        assert_eq!(zero.cells,0);assert_eq!(zero.unresolved_patches,vec![0,1]);
        let mut invalid=patch(false);invalid.weights[0][0]=-1.;
        assert!(inspect(&[invalid],0).is_err());
    }
}
