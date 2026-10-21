//! Complete retained original UV shore-use census and opposite-shore bijection.
use super::*;
use std::sync::Arc;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OriginalSeamUse {
    pub halfedge: usize,
    pub axis: usize,
    pub shore: usize,
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalSeamDictionary {
    regions: Arc<OwnedOriginalFilledRegions>,
    chart: Arc<OwnedOriginalChartVertices>,
    limits: ParameterOrderLimits,
    census: Vec<(usize, usize, usize, OriginalShoreMembership)>,
    uses: Vec<OriginalSeamUse>,
    pairs: Vec<[usize; 2]>,
}
type Dictionary = (
    Vec<(usize, usize, usize, OriginalShoreMembership)>,
    Vec<OriginalSeamUse>,
    Vec<[usize; 2]>,
);
fn derive(
    regions: &OwnedOriginalFilledRegions,
    chart: &Arc<OwnedOriginalChartVertices>,
    limits: ParameterOrderLimits,
) -> Result<Dictionary> {
    regions.reverify_source()?;
    chart.reverify_source()?;
    if !Arc::ptr_eq(regions.owner().cycles().graph(), chart.graph())
        || limits.max_refinements > 512
        || !(1..=128).contains(&limits.max_resultant_size)
    {
        return Err(refused(
            "Original seam dictionary has a foreign graph or invalid allowance",
        ));
    }
    let mut census = Vec::new();
    let mut uses = Vec::new();
    let mut receipts = Vec::new();
    for &h in regions.owner().retained() {
        for axis in 0..2 {
            if !chart.wrapped()[axis] {
                continue;
            }
            for shore in 0..2 {
                let class = classify_original_shore_carrier(
                    chart.clone(),
                    h / 2,
                    axis,
                    shore,
                    limits.max_refinements,
                )?
                .membership();
                census.push((h, axis, shore, class));
                match class {
                    OriginalShoreMembership::Absent => {}
                    OriginalShoreMembership::Mixed => {
                        return Err(refused(
                            "Original mixed shore carrier needs owned subdivision before pairing",
                        ));
                    }
                    OriginalShoreMembership::Entire => {
                        receipts.push(own_original_shore_carrier(
                            chart.clone(),
                            h / 2,
                            axis,
                            shore,
                            limits,
                        )?);
                        uses.push(OriginalSeamUse {
                            halfedge: h,
                            axis,
                            shore,
                        });
                    }
                }
            }
        }
    }
    let mut paired = vec![false; uses.len()];
    let mut pairs = Vec::new();
    for i in 0..uses.len() {
        if paired[i] {
            continue;
        }
        let keys = receipts[i].endpoint_keys();
        let candidates = (0..uses.len())
            .filter(|&j| {
                j != i
                    && !paired[j]
                    && uses[j].axis == uses[i].axis
                    && uses[j].shore != uses[i].shore
            })
            .filter(|&j| {
                let other = receipts[j].endpoint_keys();
                keys == other || keys == [other[1], other[0]]
            })
            .collect::<Vec<_>>();
        if candidates.len() != 1 {
            return Err(refused(
                "Original retained shore use lacks one unique complete opposite interval",
            ));
        }
        let j = candidates[0];
        own_original_shore_pair(receipts[i].clone(), receipts[j].clone())?.reverify_source()?;
        let travel =
            |k: usize| receipts[k].direction() * if uses[k].halfedge % 2 == 0 { 1 } else { -1 };
        if travel(i) != -travel(j) {
            return Err(refused(
                "Original seam material uses have incompatible traversal orientation",
            ));
        }
        paired[i] = true;
        paired[j] = true;
        pairs.push([i, j]);
    }
    Ok((census, uses, pairs))
}
impl OwnedOriginalSeamDictionary {
    pub fn regions(&self) -> &Arc<OwnedOriginalFilledRegions> {
        &self.regions
    }
    pub fn chart(&self) -> &Arc<OwnedOriginalChartVertices> {
        &self.chart
    }
    pub fn census(&self) -> &[(usize, usize, usize, OriginalShoreMembership)] {
        &self.census
    }
    pub fn uses(&self) -> &[OriginalSeamUse] {
        &self.uses
    }
    /// Indices into the complete retained original shore-use list.
    pub fn pairs(&self) -> &[[usize; 2]] {
        &self.pairs
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (census, uses, pairs) = derive(&self.regions, &self.chart, self.limits)?;
        if census != self.census || uses != self.uses || pairs != self.pairs {
            return Err(refused(
                "Original retained seam census or pair bijection changed",
            ));
        }
        Ok(())
    }
}
/// Classify every retained half-edge on every wrapped shore; all entire shore
/// images require one oppositely oriented mate. Mixed intervals explicitly
/// refuse pending owned subdivision. No failed proof becomes negative evidence.
/// This owns the UV pairing dictionary, not generic chart coverage, quotient
/// manifold links or actual periodic 3D source-surface agreement.
pub fn own_original_seam_dictionary(
    regions: Arc<OwnedOriginalFilledRegions>,
    chart: Arc<OwnedOriginalChartVertices>,
    limits: ParameterOrderLimits,
) -> Result<OwnedOriginalSeamDictionary> {
    let (census, uses, pairs) = derive(&regions, &chart, limits)?;
    Ok(OwnedOriginalSeamDictionary {
        regions,
        chart,
        limits,
        census,
        uses,
        pairs,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seam_dictionary_replays_complete_census_uses_pairs_and_allowance() {
        let points = [[0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]];
        let wire = points
            .windows(2)
            .map(|p| Curve::from_polyline(vec![p[0].to_vec(), p[1].to_vec()]).unwrap())
            .collect();
        let regions = Arc::new(
            assemble_original_filled_regions(
                &[wire],
                &[1],
                1e-12,
                Default::default(),
                Default::default(),
            )
            .unwrap(),
        );
        let chart = Arc::new(
            own_original_chart_vertices(
                regions.owner().cycles().graph().clone(),
                [[0., 1.], [0., 1.]],
                [true, false],
                Default::default(),
            )
            .unwrap(),
        );
        let d = own_original_seam_dictionary(regions, chart, Default::default()).unwrap();
        d.reverify_source().unwrap();
        let mut bad = d.clone();
        bad.census.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = d.clone();
        bad.uses.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = d.clone();
        bad.pairs[0] = [0, 0];
        assert!(bad.reverify_source().is_err());
        let mut bad = d;
        bad.limits.max_refinements = 513;
        assert!(bad.reverify_source().is_err());
    }
}
