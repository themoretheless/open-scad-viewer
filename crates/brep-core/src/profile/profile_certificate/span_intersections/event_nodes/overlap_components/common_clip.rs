//! Exact native-parameter clipping of original common components with a third source.
use super::*;
use std::cmp::Ordering;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalCommonClipOrigin {
    BaseEndpoint(usize),
    ThirdEndpoint(usize),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalCommonClipBoundary {
    pub origin: OriginalCommonClipOrigin,
    /// Two joint extensions: [A,B] and [A,third]. No clock linearity is assumed.
    pub positions: [AlgebraicEventPosition; 2],
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalCommonClip {
    pub third_component: usize,
    /// Increasing original A natural parameter, not projected image order.
    pub boundaries: [OriginalCommonClipBoundary; 2],
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalCommonClips {
    base: Arc<OwnedOriginalOverlapComponents>,
    component: usize,
    third: Arc<OwnedOriginalOverlapComponents>,
    base_cuts: Arc<OwnedOriginalPairCuts>,
    third_cuts: Arc<OwnedOriginalPairCuts>,
    width: f64,
    limits: ParameterOrderLimits,
    clips: Vec<OriginalCommonClip>,
}
fn endpoint_cut(cuts: &OwnedOriginalPairCuts, pair: usize, endpoint: usize) -> Result<usize> {
    cuts.cuts()
        .iter()
        .position(|c| {
            c.origins
                .contains(&OriginalPairCutOrigin::OverlapEndpoint { pair, endpoint })
        })
        .ok_or_else(|| refused("Original common clip lost an owned endpoint"))
}
pub(super) fn lifted(
    cuts: Arc<OwnedOriginalPairCuts>,
    cut: usize,
    target: &Curve,
    interval: &OriginalOverlapComponent,
    width: f64,
    limits: ParameterOrderLimits,
) -> Result<AlgebraicEventPosition> {
    let o = own_original_point_pullback(cuts, cut, target, width)?;
    let ends = if interval.directions[1] > 0 {
        [0, 1]
    } else {
        [1, 0]
    };
    let mut selected = Vec::new();
    for p in o.preimages() {
        let n = AlgebraicEventNode {
            source_cells: [0, p.source_cell],
            pair: 0,
            factor: None,
            root: 0,
            stationary: false,
            exact: p.exact.clone(),
        };
        let lo = parameter_cmp(&n, 1, &interval.endpoints[ends[0]], 1, limits)?;
        let hi = parameter_cmp(&n, 1, &interval.endpoints[ends[1]], 1, limits)?;
        if lo != Ordering::Less && hi != Ordering::Greater {
            selected.push(p.exact.clone());
        }
    }
    if selected.len() != 1 {
        return Err(refused(
            "Original common clipping boundary has no unique target traversal",
        ));
    }
    Ok(selected.pop().unwrap())
}
fn derive(o: &OwnedOriginalCommonClips) -> Result<Vec<OriginalCommonClip>> {
    if !o.width.is_finite()
        || !(1e-18..=1e-3).contains(&o.width)
        || o.limits.max_refinements > 512
        || !(1..=128).contains(&o.limits.max_resultant_size)
    {
        return Err(refused("Invalid original common clip limits"));
    }
    o.base.reverify_source()?;
    o.third.reverify_source()?;
    o.base_cuts.reverify_source()?;
    o.third_cuts.reverify_source()?;
    if !Arc::ptr_eq(o.base_cuts.owner(), o.base.owner())
        || !Arc::ptr_eq(o.third_cuts.owner(), o.third.owner())
        || o.base.owner().sources()[0] != o.third.owner().sources()[0]
    {
        return Err(refused(
            "Original common clip contexts do not share the same source",
        ));
    }
    let base = o
        .base
        .components()
        .get(o.component)
        .ok_or_else(|| refused("Unknown base common component"))?;
    let be = if base.directions[0] > 0 {
        [0, 1]
    } else {
        [1, 0]
    };
    let mut out = Vec::new();
    for (third_component, third) in o.third.components().iter().enumerate() {
        let te = if third.directions[0] > 0 {
            [0, 1]
        } else {
            [1, 0]
        };
        let lower = if parameter_cmp(
            &base.endpoints[be[0]],
            0,
            &third.endpoints[te[0]],
            0,
            o.limits,
        )? != Ordering::Less
        {
            OriginalCommonClipOrigin::BaseEndpoint(be[0])
        } else {
            OriginalCommonClipOrigin::ThirdEndpoint(te[0])
        };
        let upper = if parameter_cmp(
            &base.endpoints[be[1]],
            0,
            &third.endpoints[te[1]],
            0,
            o.limits,
        )? != Ordering::Greater
        {
            OriginalCommonClipOrigin::BaseEndpoint(be[1])
        } else {
            OriginalCommonClipOrigin::ThirdEndpoint(te[1])
        };
        let node = |r| match r {
            OriginalCommonClipOrigin::BaseEndpoint(i) => &base.endpoints[i],
            OriginalCommonClipOrigin::ThirdEndpoint(i) => &third.endpoints[i],
        };
        if parameter_cmp(node(lower), 0, node(upper), 0, o.limits)? != Ordering::Less {
            continue;
        }
        let make = |origin| -> Result<OriginalCommonClipBoundary> {
            let positions = match origin {
                OriginalCommonClipOrigin::BaseEndpoint(i) => {
                    let cut = endpoint_cut(&o.base_cuts, base.pair, i)?;
                    [
                        base.endpoints[i].exact.clone(),
                        lifted(
                            o.base_cuts.clone(),
                            cut,
                            &o.third.owner().sources()[1],
                            third,
                            o.width,
                            o.limits,
                        )?,
                    ]
                }
                OriginalCommonClipOrigin::ThirdEndpoint(i) => {
                    let cut = endpoint_cut(&o.third_cuts, third.pair, i)?;
                    [
                        lifted(
                            o.third_cuts.clone(),
                            cut,
                            &o.base.owner().sources()[1],
                            base,
                            o.width,
                            o.limits,
                        )?,
                        third.endpoints[i].exact.clone(),
                    ]
                }
            };
            Ok(OriginalCommonClipBoundary { origin, positions })
        };
        out.push(OriginalCommonClip {
            third_component,
            boundaries: [make(lower)?, make(upper)?],
        });
        if out.len() > 4096 {
            return Err(refused("Original common clip component allowance exceeded"));
        }
    }
    Ok(out)
}
impl OwnedOriginalCommonClips {
    pub fn base(&self) -> &Arc<OwnedOriginalOverlapComponents> {
        &self.base
    }
    pub fn component(&self) -> usize {
        self.component
    }
    pub fn third(&self) -> &Arc<OwnedOriginalOverlapComponents> {
        &self.third
    }
    pub fn third_cuts(&self) -> &Arc<OwnedOriginalPairCuts> {
        &self.third_cuts
    }
    pub fn clips(&self) -> &[OriginalCommonClip] {
        &self.clips
    }
    pub fn reverify_source(&self) -> Result<()> {
        if self.clips != derive(self)? {
            return Err(refused(
                "Original common clipping census or boundary geometry changed",
            ));
        }
        Ok(())
    }
}
pub fn own_original_common_clips(
    base: Arc<OwnedOriginalOverlapComponents>,
    component: usize,
    third: &Curve,
    width: f64,
    limits: ParameterOrderLimits,
) -> Result<OwnedOriginalCommonClips> {
    base.reverify_source()?;
    let a = &base.owner().sources()[0];
    let census = certify_span_intersections(a, third, false, width)?;
    let third = Arc::new(own_original_overlap_components(Arc::new(
        own_curve_event_nodes(a, third, &census)?,
    ))?);
    let base_cuts = Arc::new(own_original_pair_cuts(base.owner().clone(), limits)?);
    let third_cuts = Arc::new(own_original_pair_cuts(third.owner().clone(), limits)?);
    let mut out = OwnedOriginalCommonClips {
        base,
        component,
        third,
        base_cuts,
        third_cuts,
        width,
        limits,
        clips: Vec::new(),
    };
    out.clips = derive(&out)?;
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn common_clip_replay_rejects_missing_interval_reversed_boundary_and_parameter() {
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![4., 0.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![3., 0.], vec![1., 0.]]).unwrap();
        let t = Curve::from_polyline(vec![vec![2., 0.], vec![4., 0.]]).unwrap();
        let c = certify_span_intersections(&a, &b, false, 1e-12).unwrap();
        let base = Arc::new(
            own_original_overlap_components(Arc::new(own_curve_event_nodes(&a, &b, &c).unwrap()))
                .unwrap(),
        );
        let x =
            own_original_common_clips(base, 0, &t, 1e-12, ParameterOrderLimits::default()).unwrap();
        x.reverify_source().unwrap();
        let mut bad = x.clone();
        bad.clips.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.clips[0].boundaries.swap(0, 1);
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.clips[0].boundaries[1].positions[1].parameters[1].clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.clips[0].third_component = 99;
        assert!(bad.reverify_source().is_err());
    }
}
