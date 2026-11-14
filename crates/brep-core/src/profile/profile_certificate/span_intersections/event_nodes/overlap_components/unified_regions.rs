//! Original nonzero material regions and holes from exact unified-boundary containment.
use super::*;
#[derive(Clone, Debug)]
pub struct OwnedOriginalUnifiedRegions {
    owner: Arc<OwnedOriginalUnifiedFilled>,
    limits: ParameterOrderLimits,
    left: Vec<Vec<i64>>,
    right: Vec<Vec<i64>>,
    regions: Vec<OriginalFilledRegion>,
}
const MAX_REFINEMENTS: usize = 512;
const MAX_RESULTANT_SIZE: usize = 128;
const MAX_BOUNDARIES: usize = 1024;
const MAX_CENSUS: usize = 1_048_576;
fn verify_limits(owner: &OwnedOriginalUnifiedFilled, limits: ParameterOrderLimits) -> Result<()> {
    if limits.max_refinements > MAX_REFINEMENTS
        || !(1..=MAX_RESULTANT_SIZE).contains(&limits.max_resultant_size)
    {
        return Err(refused("Invalid unified material containment limits"));
    }
    let retained = owner.retained().len();
    if owner.boundaries().len() > MAX_BOUNDARIES
        || retained.saturating_mul(retained).saturating_mul(2) > MAX_CENSUS
    {
        return Err(refused(
            "Unified material containment census allowance exceeded",
        ));
    }
    Ok(())
}
/// Winding vectors on the left and right of one boundary; every dart of the
/// boundary must report the same pair.
fn boundary_sides(
    owner: &OwnedOriginalUnifiedFilled,
    darts: &[usize],
    limits: ParameterOrderLimits,
) -> Result<(Vec<i64>, Vec<i64>)> {
    let ports = owner.cycles().owner().owner();
    let mut sides: Option<(Vec<i64>, Vec<i64>)> = None;
    for &dart in darts {
        let p = &ports.ports()[dart];
        let side = &owner.sides()[p.edge];
        let winding = |end: usize| {
            unified_boundary_winding::evaluate_verified(
                owner,
                &side.certificates()[end].point,
                limits,
            )
        };
        let a = winding(p.endpoint)?;
        let b = winding(1 - p.endpoint)?;
        match &sides {
            Some((l, r)) if *l != a || *r != b => {
                return Err(refused(
                    "Unified selected boundary side containment is not constant",
                ));
            }
            Some(_) => {}
            None => sides = Some((a, b)),
        }
    }
    sides.ok_or_else(|| refused("Unified material boundary is empty"))
}
/// Crossing the boundary changes only its own winding, by exactly one.
fn verify_jumps(boundary: usize, n: usize, l: &[i64], r: &[i64]) -> Result<()> {
    if l.len() != n
        || r.len() != n
        || l.iter().zip(r).enumerate().any(|(other, (a, b))| {
            if other == boundary {
                a.checked_sub(*b) != Some(1)
            } else {
                a != b
            }
        })
    {
        return Err(refused(
            "Unified component containment has inconsistent exact side jumps",
        ));
    }
    Ok(())
}
/// Every hole joins the unique innermost outer boundary that contains it.
fn assign_holes(
    outer: &[usize],
    holes: Vec<usize>,
    left: &[Vec<i64>],
) -> Result<Vec<OriginalFilledRegion>> {
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
            .filter(|&c| {
                candidates
                    .iter()
                    .all(|&other| c == other || left[c][other] == 1)
            })
            .collect::<Vec<_>>();
        let [owner] = innermost[..] else {
            return Err(refused(
                "Unified hole has no unique proved innermost outer boundary",
            ));
        };
        regions
            .iter_mut()
            .find(|r| r.outer == owner)
            .ok_or_else(|| refused("Unified hole has no unique proved innermost outer boundary"))?
            .holes
            .push(hole);
    }
    Ok(regions)
}
type Containment = (Vec<Vec<i64>>, Vec<Vec<i64>>, Vec<OriginalFilledRegion>);
fn derive(owner: &OwnedOriginalUnifiedFilled, limits: ParameterOrderLimits) -> Result<Containment> {
    owner.reverify_source()?;
    verify_limits(owner, limits)?;
    let n = owner.boundaries().len();
    let mut left = Vec::new();
    let mut right = Vec::new();
    let mut outer = Vec::new();
    let mut holes = Vec::new();
    for (boundary, darts) in owner.boundaries().iter().enumerate() {
        let (l, r) = boundary_sides(owner, darts, limits)?;
        verify_jumps(boundary, n, &l, &r)?;
        match (l[boundary], r[boundary]) {
            (1, 0) => outer.push(boundary),
            (0, -1) => holes.push(boundary),
            _ => {
                return Err(refused(
                    "Unified material component is not a proved outer or hole boundary",
                ));
            }
        }
        left.push(l);
        right.push(r);
    }
    let regions = assign_holes(&outer, holes, &left)?;
    if (0..n).any(|i| (0..n).any(|j| i != j && !matches!(left[i][j], -1..=1))) {
        return Err(refused(
            "Unified material containment is not a simple component relation",
        ));
    }
    Ok((left, right, regions))
}
impl OwnedOriginalUnifiedRegions {
    pub fn owner(&self) -> &Arc<OwnedOriginalUnifiedFilled> {
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
        let (l, r, g) = derive(&self.owner, self.limits)?;
        if l != self.left || r != self.right || g != self.regions {
            return Err(refused("Unified material region or hole ownership changed"));
        }
        Ok(())
    }
}
pub fn own_original_unified_regions(
    owner: Arc<OwnedOriginalUnifiedFilled>,
    limits: ParameterOrderLimits,
) -> Result<OwnedOriginalUnifiedRegions> {
    let (left, right, regions) = derive(&owner, limits)?;
    Ok(OwnedOriginalUnifiedRegions {
        owner,
        limits,
        left,
        right,
        regions,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_rejects_containment_and_hole_owner_changes() {
        let a = Curve::from_polyline(vec![
            vec![0., 0.],
            vec![1., 0.],
            vec![1., 1.],
            vec![0., 1.],
            vec![0., 0.],
        ])
        .unwrap();
        let loops = vec![vec![a.clone()]];
        let d = Arc::new(
            own_original_common_dictionary(&[a], 1e-12, ParameterOrderLimits::default()).unwrap(),
        );
        let c = Arc::new(own_original_source_carriers(d, ParameterOrderLimits::default()).unwrap());
        let g = Arc::new(own_original_unified_graph(c).unwrap());
        let p = Arc::new(own_original_unified_ports(g.clone(), 192).unwrap());
        let r =
            Arc::new(own_original_unified_rotation(p, ContactPortOrderLimits::default()).unwrap());
        let cycles = Arc::new(own_original_unified_cycles(r).unwrap());
        let sides = own_original_unified_sides_automatically(
            g,
            &loops,
            ParameterOrderLimits::default(),
            32,
        )
        .unwrap();
        let f = Arc::new(own_original_unified_filled(cycles, sides, &[1]).unwrap());
        let x = own_original_unified_regions(f, ParameterOrderLimits::default()).unwrap();
        x.reverify_source().unwrap();
        let mut bad = x.clone();
        bad.left[0][0] = 0;
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.regions[0].holes.push(0);
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.regions.clear();
        assert!(bad.reverify_source().is_err());
    }
}
