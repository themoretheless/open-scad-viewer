//! Original material-region component grouping through exact selected winding.
use super::*;
use std::sync::Arc;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalFilledRegion {
    pub outer: usize,
    pub holes: Vec<usize>,
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalFilledRegions {
    owner: Arc<OwnedOriginalFilledBoundaries>,
    limits: ParameterOrderLimits,
    left: Vec<Vec<i64>>,
    right: Vec<Vec<i64>>,
    regions: Vec<OriginalFilledRegion>,
}
const MAX_REFINEMENTS: usize = 512;
const MAX_RESULTANT_SIZE: usize = 128;
const MESSAGES: filled_containment::Messages = filled_containment::Messages {
    jumps: "Original selected component containment has inconsistent side jumps",
    classify: "Original material component is not a qualified outer or hole boundary",
    hole: "Original hole does not have one proved innermost material outer boundary",
    simple: "Original material containment is not a simple boundary relation",
};
/// Winding vectors on the left and right of one boundary; every halfedge of
/// the boundary must report the same pair.
fn boundary_sides(
    owner: &OwnedOriginalFilledBoundaries,
    edges: &[usize],
    limits: ParameterOrderLimits,
) -> Result<(Vec<i64>, Vec<i64>)> {
    let mut sides: Option<(Vec<i64>, Vec<i64>)> = None;
    for &h in edges {
        let side = &owner.side_witnesses()[h / 2];
        let winding = |end: usize| {
            boundary_winding::evaluate_verified(owner, &side.certificates()[end].point, limits)
        };
        let a = winding(h % 2)?;
        let b = winding(1 - h % 2)?;
        match &sides {
            Some((l, r)) if *l != a || *r != b => {
                return Err(refused(
                    "Original boundary component has inconsistent exact side containment",
                ));
            }
            Some(_) => {}
            None => sides = Some((a, b)),
        }
    }
    sides.ok_or_else(|| refused("Original material boundary is empty"))
}
fn derive(
    owner: &OwnedOriginalFilledBoundaries,
    limits: ParameterOrderLimits,
) -> Result<filled_containment::Containment> {
    owner.reverify_source()?;
    if limits.max_refinements > MAX_REFINEMENTS
        || !(1..=MAX_RESULTANT_SIZE).contains(&limits.max_resultant_size)
    {
        return Err(refused("Invalid original material containment allowance"));
    }
    let boundaries = owner.boundaries();
    filled_containment::containment(
        boundaries.len(),
        |boundary| boundary_sides(owner, &boundaries[boundary], limits),
        &MESSAGES,
    )
}
impl OwnedOriginalFilledRegions {
    pub fn owner(&self) -> &Arc<OwnedOriginalFilledBoundaries> {
        &self.owner
    }
    pub fn regions(&self) -> &[OriginalFilledRegion] {
        &self.regions
    }
    pub fn left_containment(&self) -> &[Vec<i64>] {
        &self.left
    }
    pub fn right_containment(&self) -> &[Vec<i64>] {
        &self.right
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (left, right, regions) = derive(&self.owner, self.limits)?;
        if left != self.left || right != self.right || regions != self.regions {
            return Err(refused(
                "Original filled material grouping or containment changed",
            ));
        }
        Ok(())
    }
}
/// Group outer boundaries and holes using exact winding clipped to original
/// algebraic carriers. Every carrier side of each boundary must independently
/// agree on containment. This owns planar material components; periodic seam
/// quotient, physical surface attachment and common-component overlaps require
/// their own further admission proofs.
pub fn own_original_filled_regions(
    owner: Arc<OwnedOriginalFilledBoundaries>,
    limits: ParameterOrderLimits,
) -> Result<OwnedOriginalFilledRegions> {
    let (left, right, regions) = derive(&owner, limits)?;
    Ok(OwnedOriginalFilledRegions {
        owner,
        limits,
        left,
        right,
        regions,
    })
}
/// Automatic original closed-wire material components, retaining exact NURBS
/// carriers and algebraic cuts through all region and hole ownership proofs.
pub fn assemble_original_filled_regions(
    loops: &[Vec<Curve>],
    coefficients: &[i32],
    width: f64,
    assembly: OriginalBoundaryAssemblyLimits,
    containment: ParameterOrderLimits,
) -> Result<OwnedOriginalFilledRegions> {
    own_original_filled_regions(
        Arc::new(assemble_original_filled_boundaries(
            loops,
            coefficients,
            width,
            assembly,
        )?),
        containment,
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn material_region_replay_rejects_containment_and_component_tampering() {
        let points = [[0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]];
        let wire = points
            .windows(2)
            .map(|p| Curve::from_polyline(vec![p[0].to_vec(), p[1].to_vec()]).unwrap())
            .collect();
        let r = assemble_original_filled_regions(
            &[wire],
            &[1],
            1e-12,
            Default::default(),
            Default::default(),
        )
        .unwrap();
        r.reverify_source().unwrap();
        let mut bad = r.clone();
        bad.regions[0].outer = 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = r.clone();
        bad.regions[0].holes.push(0);
        assert!(bad.reverify_source().is_err());
        let mut bad = r.clone();
        bad.left[0][0] = 0;
        assert!(bad.reverify_source().is_err());
        let mut bad = r.clone();
        bad.right[0][0] = 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = r;
        bad.limits.max_refinements = 513;
        assert!(bad.reverify_source().is_err());
    }
}
