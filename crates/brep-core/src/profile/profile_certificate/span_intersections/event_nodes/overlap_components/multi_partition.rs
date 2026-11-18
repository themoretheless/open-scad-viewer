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
fn node(
    owners: &[Arc<OwnedOriginalOverlapContacts>],
    r: OriginalOverlapContactRef,
    side: usize,
) -> &AlgebraicEventNode {
    let o = &owners[r.source];
    &o.contacts()[side].nodes()[o.cuts()[r.contact].nodes[side]]
}
const MAX_REFINEMENTS: usize = 512;
const MAX_RESULTANT_SIZE: usize = 128;
const MAX_SOURCES: usize = 32;
const MAX_CONTACTS: usize = 4096;
const MAX_ORDERING_WORK: usize = 65536;
fn verify_limits(o: &OwnedOriginalOverlapPartition) -> Result<()> {
    if o.limits.max_refinements > MAX_REFINEMENTS
        || !(1..=MAX_RESULTANT_SIZE).contains(&o.limits.max_resultant_size)
        || o.sources.len() > MAX_SOURCES
        || o.sources.len() != o.owners.len()
        || !o.width.is_finite()
        || !(1e-18..=1e-3).contains(&o.width)
    {
        return Err(refused("Invalid common-carrier third-source census"));
    }
    Ok(())
}
/// Every owner is bound to this component, this source and this width, and
/// together they stay within the contact allowance.
fn verify_owners(o: &OwnedOriginalOverlapPartition) -> Result<()> {
    let mut count = 0usize;
    for (owner, source) in o.owners.iter().zip(&o.sources) {
        if !Arc::ptr_eq(owner.overlap(), &o.overlap) || owner.component() != o.component {
            return Err(refused(
                "Common-carrier partition rebound to another context",
            ));
        }
        owner.reverify_source()?;
        for pair in owner.contacts() {
            if pair.sources()[1] != *source || pair.census().maximum_parameter_width != o.width {
                return Err(refused(
                    "Common-carrier partition omits or rebinds a third-source census",
                ));
            }
        }
        count = count.saturating_add(owner.cuts().len());
    }
    if count > MAX_CONTACTS {
        return Err(refused("Common-carrier global contact count exceeded"));
    }
    Ok(())
}
/// Contacts of all third sources merged into groups ordered along the carrier.
struct Grouping<'a> {
    o: &'a OwnedOriginalOverlapPartition,
    /// The first clock runs with the carrier.
    forward: bool,
    work: usize,
    groups: Vec<Vec<OriginalOverlapContactRef>>,
}
impl Grouping<'_> {
    /// Order of two contacts on the first clock, after both clocks agree on
    /// whether they are one cut.
    fn relation(
        &self,
        previous: OriginalOverlapContactRef,
        r: OriginalOverlapContactRef,
    ) -> Result<Ordering> {
        let clock = |side: usize| {
            parameter_cmp(
                node(&self.o.owners, previous, side),
                0,
                node(&self.o.owners, r, side),
                0,
                self.o.limits,
            )
        };
        let a = clock(0)?;
        let b = clock(1)?;
        if (a == Ordering::Equal) != (b == Ordering::Equal) {
            return Err(refused(
                "Common-carrier clocks disagree on global cut identity",
            ));
        }
        Ok(a)
    }
    /// Joins the contact to the group of the same cut or opens a new group
    /// at its place in carrier order.
    fn place(&mut self, r: OriginalOverlapContactRef) -> Result<()> {
        let owners = &self.o.owners;
        let mut pos = 0;
        while pos < self.groups.len() {
            self.work += 1;
            if self.work > MAX_ORDERING_WORK {
                return Err(refused("Common-carrier global ordering budget exceeded"));
            }
            let previous = self.groups[pos][0];
            let order = self.relation(previous, r)?;
            if order == Ordering::Equal {
                if owners[previous.source].cuts()[previous.contact].endpoint
                    != owners[r.source].cuts()[r.contact].endpoint
                {
                    return Err(refused("Common-carrier global endpoint identity disagrees"));
                }
                self.groups[pos].push(r);
                return Ok(());
            }
            if (order == Ordering::Greater) == self.forward {
                break;
            }
            pos += 1;
        }
        self.groups.insert(pos, vec![r]);
        Ok(())
    }
}
/// Carrier segments between consecutive interior contact groups.
fn segments(
    o: &OwnedOriginalOverlapPartition,
    groups: &[Vec<OriginalOverlapContactRef>],
) -> Vec<[OriginalOverlapSplit; 2]> {
    let mut bounds = vec![OriginalOverlapSplit::Endpoint(0)];
    for (g, refs) in groups.iter().enumerate() {
        let r = refs[0];
        if o.owners[r.source].cuts()[r.contact].endpoint.is_none() {
            bounds.push(OriginalOverlapSplit::ContactGroup(g));
        }
    }
    bounds.push(OriginalOverlapSplit::Endpoint(1));
    bounds.windows(2).map(|b| [b[0], b[1]]).collect()
}
/// Contact groups in carrier order and the segments between them.
type Partition = (
    Vec<Vec<OriginalOverlapContactRef>>,
    Vec<[OriginalOverlapSplit; 2]>,
);
fn derive_partition(o: &OwnedOriginalOverlapPartition) -> Result<Partition> {
    verify_limits(o)?;
    o.overlap.reverify_source()?;
    let c = o
        .overlap
        .components()
        .get(o.component)
        .ok_or_else(|| refused("Unknown common-carrier component"))?;
    verify_owners(o)?;
    let mut grouping = Grouping {
        o,
        forward: c.directions[0] > 0,
        work: 0,
        groups: Vec::new(),
    };
    for (source, owner) in o.owners.iter().enumerate() {
        for contact in 0..owner.cuts().len() {
            grouping.place(OriginalOverlapContactRef { source, contact })?;
        }
    }
    let segments = segments(o, &grouping.groups);
    Ok((grouping.groups, segments))
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
