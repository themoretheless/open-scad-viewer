//! Cross-scale selection primitive. Caller owns maps and discards partial work on cancellation.
use super::{cancelled, DepthMap};
use crate::Result;

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct SelectionStats {
    pub compared: usize,
    pub replaced: usize,
}

/// Select only conflicting, valid small-footprint estimates. Never fills holes.
/// Performs no allocations; validates both complete maps before the first mutation.
pub(super) fn select_depth(
    primary: &mut DepthMap,
    secondary: &DepthMap,
    tolerance: f64,
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<SelectionStats> {
    cancelled(progress, "depth-selection", 0, primary.depth.len())?;
    let count = primary.width.checked_mul(primary.height)
        .ok_or("Depth selection dimensions overflow")?;
    if !tolerance.is_finite() || tolerance < 0.
        || primary.image != secondary.image
        || primary.width != secondary.width || primary.height != secondary.height
        || !primary.step.is_finite() || primary.step <= 0. || primary.step != secondary.step
        || primary.depth.len() != count || secondary.depth.len() != count
        || primary.confidence.len() != count || secondary.confidence.len() != count {
        return Err("Incompatible depth selection maps or tolerance".into());
    }
    for i in 0..count {
        if i % 4096 == 0 { cancelled(progress, "depth-selection-validate", i, count)?; }
        if !primary.depth[i].is_finite() || primary.depth[i] < 0.
            || !secondary.depth[i].is_finite() || secondary.depth[i] < 0.
            || !primary.confidence[i].is_finite() || !secondary.confidence[i].is_finite() {
            return Err("Invalid depth selection sample".into());
        }
    }
    let mut stats = SelectionStats::default();
    for i in 0..count {
        if i % 4096 == 0 { cancelled(progress, "depth-selection", i, count)?; }
        let (a, b) = (primary.depth[i], secondary.depth[i]);
        if a <= 0. || b <= 0. { continue; }
        stats.compared += 1;
        if (a - b).abs() > tolerance * a {
            primary.depth[i] = b;
            primary.confidence[i] = secondary.confidence[i];
            stats.replaced += 1;
        }
    }
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn map(depth: Vec<f64>, confidence: f32) -> DepthMap {
        DepthMap { image: 0, width: depth.len(), height: 1, step: 1.,
            confidence: vec![confidence; depth.len()], depth, neighbors: vec![1,2] }
    }
    #[test]
    fn selection_preserves_agreement_holes_and_reference_metadata() {
        let mut a=map(vec![0.,4.,4.,4.,4.],0.8);
        let b=map(vec![3.,0.,4.01,3.,5.],0.9);
        let stats=select_depth(&mut a,&b,0.025,&mut |_,_,_|true).unwrap();
        assert_eq!(a.depth,vec![0.,4.,4.,3.,5.]);
        assert_eq!(a.confidence,vec![0.8,0.8,0.8,0.9,0.9]);
        assert_eq!(a.neighbors,vec![1,2]);
        assert_eq!(stats,SelectionStats{compared:3,replaced:2});
    }
    #[test]
    fn malformed_late_sample_fails_before_any_mutation() {
        let mut a=map(vec![4.,4.],0.8);let b=map(vec![3.,f64::NAN],0.9);
        assert!(select_depth(&mut a,&b,0.025,&mut |_,_,_|true).is_err());
        assert_eq!(a.depth,vec![4.,4.]);
        let mut b=map(vec![3.,3.],0.9);b.image=1;
        assert!(select_depth(&mut a,&b,0.025,&mut |_,_,_|true).is_err());
    }
    #[test]
    fn cancellation_is_polled_during_large_selection() {
        let mut a=map(vec![4.;8193],0.8);let b=map(vec![3.;8193],0.9);
        assert!(select_depth(&mut a,&b,0.025,&mut |stage,i,_| !(stage=="depth-selection" && i==4096)).is_err());
        assert_eq!(a.depth[0],3.);assert_eq!(a.depth[4096],4.);
        // Partial scratch map must not be published by the owning pipeline.
    }
}

#[cfg(test)]
#[test]
fn estimator_work_merge_is_transactional_and_preserves_output_counts() {
    use super::DenseDiagnostics;
    let mut a=DenseDiagnostics { evaluated_hypotheses:10,evaluated_source_patches:20,
        sampled_source_pixels:30,estimated_maps:4,vertices:12,..Default::default() };
    let b=DenseDiagnostics { evaluated_hypotheses:7,evaluated_source_patches:8,
        sampled_source_pixels:9,estimated_maps:4,vertices:99,..Default::default() };
    a.add_estimation_work(&b).unwrap();
    assert_eq!((a.evaluated_hypotheses,a.evaluated_source_patches,a.sampled_source_pixels),(17,28,39));
    assert_eq!((a.estimated_maps,a.vertices),(4,12));
    let b=DenseDiagnostics { evaluated_hypotheses:1,evaluated_source_patches:1,
        sampled_source_pixels:u64::MAX,..Default::default() };
    assert!(a.add_estimation_work(&b).is_err());
    assert_eq!((a.evaluated_hypotheses,a.evaluated_source_patches,a.sampled_source_pixels),(17,28,39));
}

/// Extra retained heap allocations while a second estimator runs.
/// This counts capacities, not process RSS or allocator metadata.
pub(super) fn retained_map_bytes(maps: &[DepthMap], outer_capacity: usize) -> Result<usize> {
    if outer_capacity < maps.len() { return Err("Invalid depth map capacity".into()); }
    let overflow = || "Retained depth map size overflow".to_string();
    let mut bytes=outer_capacity.checked_mul(std::mem::size_of::<DepthMap>()).ok_or_else(overflow)?;
    for map in maps {
        for (capacity,size) in [
            (map.depth.capacity(),std::mem::size_of::<f64>()),
            (map.confidence.capacity(),std::mem::size_of::<f32>()),
            (map.neighbors.capacity(),std::mem::size_of::<usize>()),
        ] {
            bytes=bytes.checked_add(capacity.checked_mul(size).ok_or_else(overflow)?).ok_or_else(overflow)?;
        }
    }
    Ok(bytes)
}

pub(super) fn check_secondary_budget(estimated: Option<usize>, retained: usize, limit: usize) -> Result<()> {
    match estimated.and_then(|bytes|bytes.checked_add(retained)) {
        Some(total) if total<=limit => Ok(()),
        _ => Err("Secondary depth estimation exceeds planning budget".into()),
    }
}

#[cfg(test)]
#[test]
fn retained_capacity_and_secondary_budget_are_checked() {
    let map=DepthMap {image:0,width:1,height:1,step:1.,depth:Vec::with_capacity(20),
        confidence:Vec::with_capacity(30),neighbors:Vec::with_capacity(4)};
    let expected=2*std::mem::size_of::<DepthMap>()+map.depth.capacity()*8
        +map.confidence.capacity()*4+map.neighbors.capacity()*std::mem::size_of::<usize>();
    assert_eq!(retained_map_bytes(&[map],2).unwrap(),expected);
    assert!(retained_map_bytes(&[],usize::MAX).is_err());
    assert!(check_secondary_budget(Some(90),10,100).is_ok());
    assert!(check_secondary_budget(Some(90),11,100).is_err());
    assert!(check_secondary_budget(Some(usize::MAX),1,usize::MAX).is_err());
    assert!(check_secondary_budget(None,0,100).is_err());
}
