//! A source-owned partition combining isolated contacts and common intervals.
use super::*;
use std::cmp::Ordering;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalCommonCutOrigin {
    BaseEndpoint(usize),
    ThirdCut {
        source: usize,
        cut: usize,
    },
    ClipBoundary {
        source: usize,
        clip: usize,
        boundary: usize,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalCommonCut {
    pub exact: AlgebraicEventPosition,
    pub origins: Vec<OriginalCommonCutOrigin>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OriginalCommonContributor {
    pub source: usize,
    pub clip: usize,
    /// Increasing third natural parameter relative to increasing A parameter.
    pub direction: i8,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalCommonSegment {
    pub cuts: [usize; 2],
    pub contributors: Vec<OriginalCommonContributor>,
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalCommonPartition {
    base: Arc<OwnedOriginalOverlapComponents>,
    component: usize,
    sources: Vec<Curve>,
    owners: Vec<Arc<OwnedOriginalCommonClips>>,
    width: f64,
    limits: ParameterOrderLimits,
    cuts: Vec<OriginalCommonCut>,
    segments: Vec<OriginalCommonSegment>,
}
fn cmp(
    a: &AlgebraicEventPosition,
    side: usize,
    b: &AlgebraicEventPosition,
    other: usize,
    l: ParameterOrderLimits,
) -> Result<Ordering> {
    // Only an adapter to the existing exact value comparator. Geometry and
    // selected-root authority come from the source owners, never these wrappers.
    let node = |exact: &AlgebraicEventPosition| AlgebraicEventNode {
        source_cells: [0, 0],
        pair: 0,
        factor: None,
        root: 0,
        stationary: false,
        exact: exact.clone(),
    };
    parameter_cmp(&node(a), side, &node(b), other, l)
}
fn derive(
    o: &OwnedOriginalCommonPartition,
) -> Result<(Vec<OriginalCommonCut>, Vec<OriginalCommonSegment>)> {
    if o.sources.len() > 32
        || o.sources.len() != o.owners.len()
        || !o.width.is_finite()
        || !(1e-18..=1e-3).contains(&o.width)
        || o.limits.max_refinements > 512
        || !(1..=128).contains(&o.limits.max_resultant_size)
    {
        return Err(refused(
            "Invalid original common partition census or limits",
        ));
    }
    o.base.reverify_source()?;
    let base = o
        .base
        .components()
        .get(o.component)
        .ok_or_else(|| refused("Unknown original common partition component"))?;
    let ends = if base.directions[0] > 0 {
        [0, 1]
    } else {
        [1, 0]
    };
    let mut raw = ends
        .iter()
        .map(|&i| {
            (
                base.endpoints[i].exact.clone(),
                OriginalCommonCutOrigin::BaseEndpoint(i),
            )
        })
        .collect::<Vec<_>>();
    for (source, owner) in o.owners.iter().enumerate() {
        if !Arc::ptr_eq(owner.base(), &o.base)
            || owner.component() != o.component
            || owner.third().owner().sources()[1] != o.sources[source]
            || owner.third().owner().census().maximum_parameter_width != o.width
        {
            return Err(refused("Original common partition lost a source context"));
        }
        owner.reverify_source()?;
        // Include all pair candidates: isolated contacts and overlap endpoints,
        // including singleton touches which have no positive common interval.
        for (cut, c) in owner.third_cuts().cuts().iter().enumerate() {
            let lo = cmp(
                &c.node.exact,
                0,
                &base.endpoints[ends[0]].exact,
                0,
                o.limits,
            )?;
            let hi = cmp(
                &c.node.exact,
                0,
                &base.endpoints[ends[1]].exact,
                0,
                o.limits,
            )?;
            if lo != Ordering::Less && hi != Ordering::Greater {
                let exact = common_clip::lifted(
                    owner.third_cuts().clone(),
                    cut,
                    &o.base.owner().sources()[1],
                    base,
                    o.width,
                    o.limits,
                )?;
                raw.push((exact, OriginalCommonCutOrigin::ThirdCut { source, cut }));
            }
        }
        for (clip, c) in owner.clips().iter().enumerate() {
            for boundary in 0..2 {
                raw.push((
                    c.boundaries[boundary].positions[0].clone(),
                    OriginalCommonCutOrigin::ClipBoundary {
                        source,
                        clip,
                        boundary,
                    },
                ));
            }
        }
        if raw.len() > 4096 {
            return Err(refused("Original common partition cut allowance exceeded"));
        }
    }
    let mut work = 0usize;
    let mut cuts: Vec<OriginalCommonCut> = Vec::new();
    for (exact, origin) in raw {
        let mut pos = 0;
        let mut joined = false;
        while pos < cuts.len() {
            work += 1;
            if work > 65536 {
                return Err(refused(
                    "Original common partition ordering allowance exceeded",
                ));
            }
            let a = cmp(&cuts[pos].exact, 0, &exact, 0, o.limits)?;
            if a == Ordering::Equal {
                if cmp(&cuts[pos].exact, 1, &exact, 1, o.limits)? != Ordering::Equal {
                    return Err(refused("Original common partition source clocks disagree"));
                }
                cuts[pos].origins.push(origin);
                joined = true;
                break;
            }
            if a == Ordering::Greater {
                break;
            }
            pos += 1;
        }
        if !joined {
            cuts.insert(
                pos,
                OriginalCommonCut {
                    exact,
                    origins: vec![origin],
                },
            );
        }
    }
    let mut segments = Vec::new();
    for index in 0..cuts.len() - 1 {
        let mut contributors = Vec::new();
        for (source, owner) in o.owners.iter().enumerate() {
            for (clip, c) in owner.clips().iter().enumerate() {
                work += 1;
                if work > 131072 {
                    return Err(refused(
                        "Original common partition coverage allowance exceeded",
                    ));
                }
                let lower = &c.boundaries[0].positions[0];
                let upper = &c.boundaries[1].positions[0];
                let lo = cmp(&cuts[index].exact, 0, lower, 0, o.limits)?;
                let hi = cmp(&cuts[index + 1].exact, 0, upper, 0, o.limits)?;
                if lo != Ordering::Less && hi != Ordering::Greater {
                    let third = &owner.third().components()[c.third_component];
                    contributors.push(OriginalCommonContributor {
                        source,
                        clip,
                        direction: third.directions[0] * third.directions[1],
                    });
                } else if cmp(&cuts[index].exact, 0, upper, 0, o.limits)? == Ordering::Less
                    && cmp(&cuts[index + 1].exact, 0, lower, 0, o.limits)? == Ordering::Greater
                {
                    return Err(refused(
                        "Original common partition misses a contributor boundary",
                    ));
                }
            }
        }
        segments.push(OriginalCommonSegment {
            cuts: [index, index + 1],
            contributors,
        });
    }
    Ok((cuts, segments))
}
impl OwnedOriginalCommonPartition {
    pub fn base(&self) -> &Arc<OwnedOriginalOverlapComponents> {
        &self.base
    }
    pub fn component(&self) -> usize {
        self.component
    }
    pub fn sources(&self) -> &[Curve] {
        &self.sources
    }
    pub fn owners(&self) -> &[Arc<OwnedOriginalCommonClips>] {
        &self.owners
    }
    pub fn cuts(&self) -> &[OriginalCommonCut] {
        &self.cuts
    }
    pub fn segments(&self) -> &[OriginalCommonSegment] {
        &self.segments
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (cuts, segments) = derive(self)?;
        if cuts != self.cuts || segments != self.segments {
            return Err(refused(
                "Original common partition cuts or contributors changed",
            ));
        }
        Ok(())
    }
}
pub fn own_original_common_partition(
    base: Arc<OwnedOriginalOverlapComponents>,
    component: usize,
    sources: &[Curve],
    width: f64,
    limits: ParameterOrderLimits,
) -> Result<OwnedOriginalCommonPartition> {
    if sources.len() > 32 {
        return Err(refused(
            "Original common partition source allowance exceeded",
        ));
    }
    let mut owners = Vec::new();
    for source in sources {
        owners.push(Arc::new(own_original_common_clips(
            base.clone(),
            component,
            source,
            width,
            limits,
        )?));
    }
    let mut out = OwnedOriginalCommonPartition {
        base,
        component,
        sources: sources.to_vec(),
        owners,
        width,
        limits,
        cuts: Vec::new(),
        segments: Vec::new(),
    };
    (out.cuts, out.segments) = derive(&out)?;
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_rejects_missing_cut_origin_segment_and_contributor_direction() {
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![3., 0.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![2.5, 0.], vec![0.5, 0.]]).unwrap();
        let t = Curve::from_polyline(vec![vec![2., 0.], vec![1., 0.]]).unwrap();
        let census = certify_span_intersections(&a, &b, false, 1e-12).unwrap();
        let base = Arc::new(
            own_original_overlap_components(Arc::new(
                own_curve_event_nodes(&a, &b, &census).unwrap(),
            ))
            .unwrap(),
        );
        let x =
            own_original_common_partition(base, 0, &[t], 1e-12, ParameterOrderLimits::default())
                .unwrap();
        x.reverify_source().unwrap();
        let mut bad = x.clone();
        bad.cuts[1].origins.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.segments.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.segments[1].contributors.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.segments[1].contributors[0].direction = 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.sources.clear();
        assert!(bad.reverify_source().is_err());
    }
}
