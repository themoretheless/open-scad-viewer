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
    match selected.pop() {
        Some(only) if selected.is_empty() => Ok(only),
        _ => Err(refused(
            "Original common clipping boundary has no unique target traversal",
        )),
    }
}
const MAX_REFINEMENTS: usize = 512;
const MAX_RESULTANT_SIZE: usize = 128;
const MAX_CLIPS: usize = 4096;
/// Limits are valid and all four owners replay on one shared source.
fn verify_context(o: &OwnedOriginalCommonClips) -> Result<()> {
    if !o.width.is_finite()
        || !(1e-18..=1e-3).contains(&o.width)
        || o.limits.max_refinements > MAX_REFINEMENTS
        || !(1..=MAX_RESULTANT_SIZE).contains(&o.limits.max_resultant_size)
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
    Ok(())
}
/// Endpoint indices of a component in increasing order on the shared clock.
fn ordered_ends(c: &OriginalOverlapComponent) -> [usize; 2] {
    if c.directions[0] > 0 { [0, 1] } else { [1, 0] }
}
/// The base component clipped against one third component.
struct ClipPair<'a> {
    o: &'a OwnedOriginalCommonClips,
    base: &'a OriginalOverlapComponent,
    third: &'a OriginalOverlapComponent,
}
impl ClipPair<'_> {
    fn node(&self, origin: OriginalCommonClipOrigin) -> &AlgebraicEventNode {
        match origin {
            OriginalCommonClipOrigin::BaseEndpoint(i) => &self.base.endpoints[i],
            OriginalCommonClipOrigin::ThirdEndpoint(i) => &self.third.endpoints[i],
        }
    }
    /// The later lower end and the earlier upper end, when they bound a
    /// nonempty common interval.
    fn bounds(&self) -> Result<Option<[OriginalCommonClipOrigin; 2]>> {
        let (be, te) = (ordered_ends(self.base), ordered_ends(self.third));
        let pick = |end: usize, keep_base: Ordering| -> Result<OriginalCommonClipOrigin> {
            let order = parameter_cmp(
                &self.base.endpoints[be[end]],
                0,
                &self.third.endpoints[te[end]],
                0,
                self.o.limits,
            )?;
            Ok(if order != keep_base {
                OriginalCommonClipOrigin::BaseEndpoint(be[end])
            } else {
                OriginalCommonClipOrigin::ThirdEndpoint(te[end])
            })
        };
        let lower = pick(0, Ordering::Less)?;
        let upper = pick(1, Ordering::Greater)?;
        let nonempty = parameter_cmp(self.node(lower), 0, self.node(upper), 0, self.o.limits)?
            == Ordering::Less;
        Ok(nonempty.then_some([lower, upper]))
    }
    /// A clip end with its exact position on both clocks: its own endpoint
    /// and its lift onto the other component.
    fn boundary(&self, origin: OriginalCommonClipOrigin) -> Result<OriginalCommonClipBoundary> {
        let o = self.o;
        let positions = match origin {
            OriginalCommonClipOrigin::BaseEndpoint(i) => {
                let cut = endpoint_cut(&o.base_cuts, self.base.pair, i)?;
                [
                    self.base.endpoints[i].exact.clone(),
                    lifted(
                        o.base_cuts.clone(),
                        cut,
                        &o.third.owner().sources()[1],
                        self.third,
                        o.width,
                        o.limits,
                    )?,
                ]
            }
            OriginalCommonClipOrigin::ThirdEndpoint(i) => {
                let cut = endpoint_cut(&o.third_cuts, self.third.pair, i)?;
                [
                    lifted(
                        o.third_cuts.clone(),
                        cut,
                        &o.base.owner().sources()[1],
                        self.base,
                        o.width,
                        o.limits,
                    )?,
                    self.third.endpoints[i].exact.clone(),
                ]
            }
        };
        Ok(OriginalCommonClipBoundary { origin, positions })
    }
}
fn derive(o: &OwnedOriginalCommonClips) -> Result<Vec<OriginalCommonClip>> {
    verify_context(o)?;
    let base = o
        .base
        .components()
        .get(o.component)
        .ok_or_else(|| refused("Unknown base common component"))?;
    let mut out = Vec::new();
    for (third_component, third) in o.third.components().iter().enumerate() {
        let pair = ClipPair { o, base, third };
        let Some([lower, upper]) = pair.bounds()? else {
            continue;
        };
        out.push(OriginalCommonClip {
            third_component,
            boundaries: [pair.boundary(lower)?, pair.boundary(upper)?],
        });
        if out.len() > MAX_CLIPS {
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
