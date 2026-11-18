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
use super::super::filled_containment as regions;
const MESSAGES: regions::Messages = regions::Messages {
    jumps: "Unified component containment has inconsistent exact side jumps",
    classify: "Unified material component is not a proved outer or hole boundary",
    hole: "Unified hole has no unique proved innermost outer boundary",
    simple: "Unified material containment is not a simple component relation",
};
fn derive(
    owner: &OwnedOriginalUnifiedFilled,
    limits: ParameterOrderLimits,
) -> Result<regions::Containment> {
    owner.reverify_source()?;
    verify_limits(owner, limits)?;
    let boundaries = owner.boundaries();
    regions::containment(
        boundaries.len(),
        |boundary| boundary_sides(owner, &boundaries[boundary], limits),
        &MESSAGES,
    )
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
