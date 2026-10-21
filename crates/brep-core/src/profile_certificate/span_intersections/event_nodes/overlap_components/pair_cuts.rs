//! Complete exact split candidates from isolated contacts and common boundaries.
use super::*;
use std::cmp::Ordering;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalPairCutOrigin {
    Isolated { node: usize },
    OverlapEndpoint { pair: usize, endpoint: usize },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPairCut {
    pub node: AlgebraicEventNode,
    pub origins: Vec<OriginalPairCutOrigin>,
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalPairCuts {
    owner: Arc<CurveEventNodes>,
    limits: ParameterOrderLimits,
    cuts: Vec<OriginalPairCut>,
}
fn derive_cuts(owner: &CurveEventNodes, l: ParameterOrderLimits) -> Result<Vec<OriginalPairCut>> {
    if l.max_refinements > 512 || !(1..=128).contains(&l.max_resultant_size) {
        return Err(refused("Invalid original pair-cut comparison budget"));
    }
    owner.reverify_source()?;
    let mut raw = owner
        .nodes()
        .iter()
        .enumerate()
        .map(|(node, n)| (n.clone(), OriginalPairCutOrigin::Isolated { node }))
        .collect::<Vec<_>>();
    for (pair, ends) in owner.overlap_endpoints()? {
        for (endpoint, node) in ends.into_iter().enumerate() {
            raw.push((
                node,
                OriginalPairCutOrigin::OverlapEndpoint { pair, endpoint },
            ));
        }
    }
    if raw.len() > 4096 {
        return Err(refused("Original pair-cut census allowance exceeded"));
    }
    let mut out: Vec<OriginalPairCut> = Vec::new();
    let mut work = 0;
    for (node, origin) in raw {
        let mut pos = 0;
        let mut joined = false;
        while pos < out.len() {
            work += 1;
            if work > 65536 {
                return Err(refused("Original pair-cut ordering allowance exceeded"));
            }
            let a = parameter_cmp(&out[pos].node, 0, &node, 0, l)?;
            let order = if a == Ordering::Equal {
                parameter_cmp(&out[pos].node, 1, &node, 1, l)?
            } else {
                a
            };
            if order == Ordering::Equal {
                out[pos].origins.push(origin);
                joined = true;
                break;
            }
            if order == Ordering::Greater {
                break;
            }
            pos += 1;
        }
        if !joined {
            out.insert(
                pos,
                OriginalPairCut {
                    node,
                    origins: vec![origin],
                },
            );
        }
    }
    Ok(out)
}
impl OwnedOriginalPairCuts {
    pub fn owner(&self) -> &Arc<CurveEventNodes> {
        &self.owner
    }
    /// Ordered by first natural parameter, then second. Equal spatial points
    /// with different original traversals intentionally remain different cuts.
    pub fn cuts(&self) -> &[OriginalPairCut] {
        &self.cuts
    }
    pub fn reverify_source(&self) -> Result<()> {
        if self.cuts != derive_cuts(&self.owner, self.limits)? {
            return Err(refused(
                "Original pair-cut source census or provenance changed",
            ));
        }
        Ok(())
    }
}
/// Own all isolated/stationary events AND every original overlap boundary.
/// Equality requires both exact original parameters; no midpoint/box merging.
pub fn own_original_pair_cuts(
    owner: Arc<CurveEventNodes>,
    limits: ParameterOrderLimits,
) -> Result<OwnedOriginalPairCuts> {
    let cuts = derive_cuts(&owner, limits)?;
    Ok(OwnedOriginalPairCuts {
        owner,
        limits,
        cuts,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_rejects_cut_provenance_omission_and_primitive_changes() {
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.], vec![2., 0.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![0.5, 0.], vec![1.5, 0.]]).unwrap();
        let c = certify_span_intersections(&a, &b, false, 1e-12).unwrap();
        let x = own_original_pair_cuts(
            Arc::new(own_curve_event_nodes(&a, &b, &c).unwrap()),
            ParameterOrderLimits::default(),
        )
        .unwrap();
        x.reverify_source().unwrap();
        let mut bad = x.clone();
        bad.cuts.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.cuts[0].origins.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.cuts[0].node.exact.parameters[0].clear();
        assert!(bad.reverify_source().is_err());
    }
}
