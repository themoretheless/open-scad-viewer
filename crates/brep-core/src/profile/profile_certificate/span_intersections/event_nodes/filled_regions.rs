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
type RegionData = (Vec<Vec<i64>>, Vec<Vec<i64>>, Vec<OriginalFilledRegion>);
fn derive(
    owner: &OwnedOriginalFilledBoundaries,
    limits: ParameterOrderLimits,
) -> Result<RegionData> {
    owner.reverify_source()?;
    if limits.max_refinements > 512 || !(1..=128).contains(&limits.max_resultant_size) {
        return Err(refused("Invalid original material containment allowance"));
    }
    let n = owner.boundaries().len();
    let mut left = Vec::new();
    let mut right = Vec::new();
    let mut outer = Vec::new();
    let mut holes = Vec::new();
    for (boundary, edges) in owner.boundaries().iter().enumerate() {
        let mut l = None;
        let mut r = None;
        for &h in edges {
            let side = &owner.side_witnesses()[h / 2];
            let a = boundary_winding::evaluate_verified(
                owner,
                &side.certificates()[h % 2].point,
                limits,
            )?;
            let b = boundary_winding::evaluate_verified(
                owner,
                &side.certificates()[1 - h % 2].point,
                limits,
            )?;
            if let Some(previous) = &l {
                if previous != &a || r.as_ref() != Some(&b) {
                    return Err(refused(
                        "Original boundary component has inconsistent exact side containment",
                    ));
                }
            } else {
                l = Some(a);
                r = Some(b);
            }
        }
        let l = l.ok_or_else(|| refused("Original material boundary is empty"))?;
        let r = r.unwrap();
        if l.len() != n
            || r.len() != n
            || l.iter().zip(&r).enumerate().any(|(j, (a, b))| {
                if j == boundary {
                    a.checked_sub(*b) != Some(1)
                } else {
                    a != b
                }
            })
        {
            return Err(refused(
                "Original selected component containment has inconsistent side jumps",
            ));
        }
        match (l[boundary], r[boundary]) {
            (1, 0) => outer.push(boundary),
            (0, -1) => holes.push(boundary),
            _ => {
                return Err(refused(
                    "Original material component is not a qualified outer or hole boundary",
                ));
            }
        }
        left.push(l);
        right.push(r);
    }
    let mut regions = outer
        .iter()
        .map(|&outer| OriginalFilledRegion {
            outer,
            holes: Vec::new(),
        })
        .collect::<Vec<_>>();
    for hole in holes {
        let candidates = outer
            .iter()
            .copied()
            .filter(|&o| left[hole][o] == 1)
            .collect::<Vec<_>>();
        let innermost = candidates
            .iter()
            .copied()
            .filter(|&candidate| {
                candidates
                    .iter()
                    .all(|&other| candidate == other || left[candidate][other] == 1)
            })
            .collect::<Vec<_>>();
        if innermost.len() != 1 {
            return Err(refused(
                "Original hole does not have one proved innermost material outer boundary",
            ));
        }
        regions
            .iter_mut()
            .find(|r| r.outer == innermost[0])
            .unwrap()
            .holes
            .push(hole);
    }
    for i in 0..n {
        for j in 0..n {
            if i != j && !matches!(left[i][j], -1..=1) {
                return Err(refused(
                    "Original material containment is not a simple boundary relation",
                ));
            }
        }
    }
    Ok((left, right, regions))
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
