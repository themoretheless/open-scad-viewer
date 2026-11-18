//! Exact point winding of each retained original boundary component.
use super::super::super::algebraic_intersections::decode_ratio;
use super::*;
use std::sync::Arc;
#[derive(Clone, Debug)]
pub struct OwnedOriginalBoundaryWinding {
    owner: Arc<OwnedOriginalFilledBoundaries>,
    point: [ExactRatio; 2],
    limits: ParameterOrderLimits,
    windings: Vec<i64>,
}
fn derive(
    owner: &OwnedOriginalFilledBoundaries,
    point: &[ExactRatio; 2],
    limits: ParameterOrderLimits,
) -> Result<Vec<i64>> {
    owner.reverify_source()?;
    evaluate_verified(owner, point, limits)
}
pub(super) fn evaluate_verified(
    owner: &OwnedOriginalFilledBoundaries,
    point: &[ExactRatio; 2],
    limits: ParameterOrderLimits,
) -> Result<Vec<i64>> {
    if limits.max_refinements > 512 || !(1..=128).contains(&limits.max_resultant_size) {
        return Err(refused(
            "Invalid selected original boundary winding allowance",
        ));
    }
    let point = [decode_ratio(&point[0])?, decode_ratio(&point[1])?];
    let carriers = owner.cycles().graph().owner();
    let mut contribution = vec![None; carriers.carriers().len()];
    let mut windings = Vec::new();
    for boundary in owner.boundaries() {
        let mut total = 0i64;
        for &h in boundary {
            let index = h / 2;
            if contribution[index].is_none() {
                let c = &carriers.carriers()[index];
                let bounds = [
                    automatic_boundaries::bound(carriers, c.source, &c.cuts[0])?,
                    automatic_boundaries::bound(carriers, c.source, &c.cuts[1])?,
                ];
                let mut value = 0i64;
                for cell in cells(&carriers.atlas().sources()[c.source])? {
                    value = value
                        .checked_add(arithmetic::selected_winding::contribution(
                            &cell.net,
                            &cell.domain,
                            &bounds,
                            &point,
                            limits.max_refinements,
                            limits.max_resultant_size,
                        )?)
                        .ok_or_else(|| refused("Original carrier ray sum overflow"))?;
                }
                contribution[index] = Some(value);
            }
            let value = contribution[index].unwrap();
            total = total
                .checked_add(if h % 2 == 0 { value } else { -value })
                .ok_or_else(|| refused("Original boundary winding total overflow"))?;
        }
        windings.push(total);
    }
    Ok(windings)
}
impl OwnedOriginalBoundaryWinding {
    pub fn owner(&self) -> &Arc<OwnedOriginalFilledBoundaries> {
        &self.owner
    }
    pub fn point(&self) -> &[ExactRatio; 2] {
        &self.point
    }
    pub fn windings(&self) -> &[i64] {
        &self.windings
    }
    pub fn reverify_source(&self) -> Result<()> {
        if derive(&self.owner, &self.point, self.limits)? != self.windings {
            return Err(refused("Selected original boundary point winding changed"));
        }
        Ok(())
    }
}
/// Complete original ray roots are compared to the actual algebraic cuts.
/// Artificial cut roots use exact one-sided germs, including cuts on the ray.
/// Query points on contributing original source images refuse, even if the
/// particular source portion is not retained. No rounded subcurve is extracted.
pub fn own_original_boundary_winding(
    owner: Arc<OwnedOriginalFilledBoundaries>,
    point: [ExactRatio; 2],
    limits: ParameterOrderLimits,
) -> Result<OwnedOriginalBoundaryWinding> {
    let windings = derive(&owner, &point, limits)?;
    Ok(OwnedOriginalBoundaryWinding {
        owner,
        point,
        limits,
        windings,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_boundary_query_replays_point_limits_and_winding_claim() {
        let points = [[0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]];
        let wire = points
            .windows(2)
            .map(|p| Curve::from_polyline(vec![p[0].to_vec(), p[1].to_vec()]).unwrap())
            .collect();
        let owner = Arc::new(
            assemble_original_filled_boundaries(&[wire], &[1], 1e-12, Default::default()).unwrap(),
        );
        let q = |a: &str, b: &str| [a.to_string(), b.to_string()];
        let w =
            own_original_boundary_winding(owner, [q("1", "2"), q("1", "2")], Default::default())
                .unwrap();
        w.reverify_source().unwrap();
        let mut bad = w.clone();
        bad.windings[0] = 0;
        assert!(bad.reverify_source().is_err());
        let mut bad = w.clone();
        bad.point[0] = q("2", "1");
        assert!(bad.reverify_source().is_err());
        let mut bad = w;
        bad.limits.max_refinements = 513;
        assert!(bad.reverify_source().is_err());
    }
}
