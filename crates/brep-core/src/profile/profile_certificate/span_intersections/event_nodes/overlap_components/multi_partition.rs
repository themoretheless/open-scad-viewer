//! One complete partition against an explicitly owned third-source census.
use super::*;
use std::cmp::Ordering;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OriginalOverlapContactRef {
    pub source: usize,
    pub contact: usize,
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalOverlapPartition {
    overlap: Arc<OwnedOriginalOverlapComponents>,
    component: usize,
    sources: Vec<Curve>,
    width: f64,
    limits: ParameterOrderLimits,
    owners: Vec<Arc<OwnedOriginalOverlapContacts>>,
    groups: Vec<Vec<OriginalOverlapContactRef>>,
    segments: Vec<[OriginalOverlapSplit; 2]>,
}
fn node<'a>(
    owners: &'a [Arc<OwnedOriginalOverlapContacts>],
    r: OriginalOverlapContactRef,
    side: usize,
) -> &'a AlgebraicEventNode {
    let o = &owners[r.source];
    &o.contacts()[side].nodes()[o.cuts()[r.contact].nodes[side]]
}
fn derive_partition(
    o: &OwnedOriginalOverlapPartition,
) -> Result<(
    Vec<Vec<OriginalOverlapContactRef>>,
    Vec<[OriginalOverlapSplit; 2]>,
)> {
    if o.limits.max_refinements > 512
        || !(1..=128).contains(&o.limits.max_resultant_size)
        || o.sources.len() > 32
        || o.sources.len() != o.owners.len()
        || !o.width.is_finite()
        || !(1e-18..=1e-3).contains(&o.width)
    {
        return Err(refused("Invalid common-carrier third-source census"));
    }
    o.overlap.reverify_source()?;
    let c = o
        .overlap
        .components()
        .get(o.component)
        .ok_or_else(|| refused("Unknown common-carrier component"))?;
    let mut count = 0usize;
    for (i, owner) in o.owners.iter().enumerate() {
        if !Arc::ptr_eq(owner.overlap(), &o.overlap) || owner.component() != o.component {
            return Err(refused(
                "Common-carrier partition rebound to another context",
            ));
        }
        owner.reverify_source()?;
        for pair in owner.contacts() {
            if pair.sources()[1] != o.sources[i] || pair.census().maximum_parameter_width != o.width
            {
                return Err(refused(
                    "Common-carrier partition omits or rebinds a third-source census",
                ));
            }
        }
        count = count.saturating_add(owner.cuts().len());
    }
    if count > 4096 {
        return Err(refused("Common-carrier global contact count exceeded"));
    }
    let mut work = 0usize;
    let mut groups: Vec<Vec<OriginalOverlapContactRef>> = Vec::new();
    for (source, owner) in o.owners.iter().enumerate() {
        for contact in 0..owner.cuts().len() {
            let r = OriginalOverlapContactRef { source, contact };
            let mut pos = 0;
            let mut joined = false;
            while pos < groups.len() {
                work += 1;
                if work > 65536 {
                    return Err(refused("Common-carrier global ordering budget exceeded"));
                }
                let previous = groups[pos][0];
                let a = parameter_cmp(
                    node(&o.owners, previous, 0),
                    0,
                    node(&o.owners, r, 0),
                    0,
                    o.limits,
                )?;
                let b = parameter_cmp(
                    node(&o.owners, previous, 1),
                    0,
                    node(&o.owners, r, 1),
                    0,
                    o.limits,
                )?;
                if (a == Ordering::Equal) != (b == Ordering::Equal) {
                    return Err(refused(
                        "Common-carrier clocks disagree on global cut identity",
                    ));
                }
                if a == Ordering::Equal {
                    if o.owners[previous.source].cuts()[previous.contact].endpoint
                        != owner.cuts()[contact].endpoint
                    {
                        return Err(refused("Common-carrier global endpoint identity disagrees"));
                    }
                    groups[pos].push(r);
                    joined = true;
                    break;
                }
                if (a == Ordering::Greater) == (c.directions[0] > 0) {
                    break;
                }
                pos += 1;
            }
            if !joined {
                groups.insert(pos, vec![r]);
            }
        }
    }
    let mut bounds = vec![OriginalOverlapSplit::Endpoint(0)];
    for (g, refs) in groups.iter().enumerate() {
        let r = refs[0];
        if o.owners[r.source].cuts()[r.contact].endpoint.is_none() {
            bounds.push(OriginalOverlapSplit::ContactGroup(g));
        }
    }
    bounds.push(OriginalOverlapSplit::Endpoint(1));
    let segments = bounds.windows(2).map(|b| [b[0], b[1]]).collect();
    Ok((groups, segments))
}
impl OwnedOriginalOverlapPartition {
    pub fn overlap(&self) -> &Arc<OwnedOriginalOverlapComponents> {
        &self.overlap
    }
    pub fn component(&self) -> usize {
        self.component
    }
    pub fn sources(&self) -> &[Curve] {
        &self.sources
    }
    pub fn owners(&self) -> &[Arc<OwnedOriginalOverlapContacts>] {
        &self.owners
    }
    pub fn groups(&self) -> &[Vec<OriginalOverlapContactRef>] {
        &self.groups
    }
    pub fn segments(&self) -> &[[OriginalOverlapSplit; 2]] {
        &self.segments
    }
    pub fn split_parameter(
        &self,
        split: OriginalOverlapSplit,
        side: usize,
    ) -> Result<(&AlgebraicEventPosition, usize)> {
        if side > 1 {
            return Err(refused("Invalid common-carrier source"));
        }
        match split {
            OriginalOverlapSplit::Endpoint(i) => Ok((
                &self.overlap.components()[self.component]
                    .endpoints
                    .get(i)
                    .ok_or_else(|| refused("Unknown overlap endpoint"))?
                    .exact,
                side,
            )),
            OriginalOverlapSplit::ContactGroup(g) => {
                let r = *self
                    .groups
                    .get(g)
                    .and_then(|v| v.first())
                    .ok_or_else(|| refused("Unknown global overlap contact group"))?;
                Ok((&node(&self.owners, r, side).exact, 0))
            }
        }
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (groups, segments) = derive_partition(self)?;
        if groups != self.groups || segments != self.segments {
            return Err(refused("Original common-carrier global partition changed"));
        }
        Ok(())
    }
}
/// Automatically own contacts with EVERY supplied original third source.
/// The census is relative to this explicit list; it does not claim complete
/// global Boolean topology or handle third-source common components yet.
pub fn own_original_overlap_partition(
    overlap: Arc<OwnedOriginalOverlapComponents>,
    component: usize,
    sources: &[Curve],
    width: f64,
    limits: ParameterOrderLimits,
) -> Result<OwnedOriginalOverlapPartition> {
    if sources.len() > 32 || !width.is_finite() || !(1e-18..=1e-3).contains(&width) {
        return Err(refused("Invalid common-carrier third-source census"));
    }
    overlap.reverify_source()?;
    if component >= overlap.components().len() {
        return Err(refused("Unknown original common component"));
    }
    let mut owners = Vec::new();
    for third in sources {
        let make = |side: usize| -> Result<Arc<CurveEventNodes>> {
            let a = &overlap.owner().sources()[side];
            let census = certify_span_intersections(a, third, false, width)?;
            Ok(Arc::new(own_curve_event_nodes(a, third, &census)?))
        };
        owners.push(Arc::new(own_original_overlap_contacts(
            overlap.clone(),
            component,
            [make(0)?, make(1)?],
            limits,
        )?));
    }
    let mut out = OwnedOriginalOverlapPartition {
        overlap,
        component,
        sources: sources.to_vec(),
        width,
        limits,
        owners,
        groups: Vec::new(),
        segments: Vec::new(),
    };
    (out.groups, out.segments) = derive_partition(&out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn global_partition_replay_rejects_missing_groups_segments_and_sources() {
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![2., 0.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![3., 0.], vec![1., 0.]]).unwrap();
        let t = Curve::from_polyline(vec![vec![1.5, -1.], vec![1.5, 1.]]).unwrap();
        let census = certify_span_intersections(&a, &b, false, 1e-12).unwrap();
        let overlap = Arc::new(
            own_original_overlap_components(Arc::new(
                own_curve_event_nodes(&a, &b, &census).unwrap(),
            ))
            .unwrap(),
        );
        let x = own_original_overlap_partition(
            overlap,
            0,
            &[t],
            1e-12,
            ParameterOrderLimits::default(),
        )
        .unwrap();
        x.reverify_source().unwrap();
        let mut bad = x.clone();
        bad.groups.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.segments.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.sources.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.groups[0][0].source = 99;
        assert!(bad.reverify_source().is_err());
    }
}
