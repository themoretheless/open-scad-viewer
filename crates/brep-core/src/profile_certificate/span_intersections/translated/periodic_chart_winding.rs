//! Exact chart-plane cycle winding, including algebraic native frame cuts.
use super::*;
use crate::profile_certificate::algebraic_intersections::{decode_polynomial, decode_ratio};
use std::sync::Arc;
#[derive(Clone, Copy, Debug)]
pub struct PeriodicChartWindingLimits {
    pub parameters: ParameterOrderLimits,
    pub max_prepared_edges: usize,
    pub max_query_edges: usize,
}
impl Default for PeriodicChartWindingLimits {
    fn default() -> Self {
        Self {
            parameters: ParameterOrderLimits::default(),
            max_prepared_edges: 1000000,
            max_query_edges: 1000000,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Bound {
    primitive: Vec<Q>,
    value: Vec<Q>,
    interval: [Q; 2],
}
impl Bound {
    fn decode(p: &OriginalSourceParameter) -> Result<Self> {
        Ok(Self {
            primitive: decode_polynomial(&p.primitive)?,
            value: decode_polynomial(&p.parameter)?,
            interval: [decode_ratio(&p.interval[0])?, decode_ratio(&p.interval[1])?],
        })
    }
    fn arithmetic(&self) -> arithmetic::branch_neighbourhood::ParameterValue {
        arithmetic::branch_neighbourhood::ParameterValue {
            primitive: self.primitive.clone(),
            value: self.value.clone(),
            interval: self.interval.clone(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Prepared {
    Source {
        net: Vec<[Q; 3]>,
        domain: [Q; 2],
        bounds: [Bound; 2],
        direction: i8,
    },
    Frame {
        shore: usize,
        interval: usize,
    },
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicChartWinding {
    owner: Arc<OwnedPeriodicChartCycles>,
    limits: PeriodicChartWindingLimits,
    prepared: Vec<Prepared>,
}
fn shores(o: &OwnedPeriodicChartCycles) -> &Arc<OwnedPeriodicNativeChartShores> {
    o.owner().owner().owner()
}
fn validate(l: PeriodicChartWindingLimits) -> Result<()> {
    if [l.max_prepared_edges, l.max_query_edges]
        .iter()
        .any(|n| !(1..=1000000).contains(n))
        || l.parameters.max_refinements > 512
        || !(1..=128).contains(&l.parameters.max_resultant_size)
    {
        return Err(refused("Invalid exact chart winding allowance"));
    }
    Ok(())
}
fn prepare(
    owner: &OwnedPeriodicChartCycles,
    limits: PeriodicChartWindingLimits,
) -> Result<Vec<Prepared>> {
    validate(limits)?;
    if owner.edges().len() > limits.max_prepared_edges {
        return Err(refused(
            "Complete chart winding preparation exceeds allowance",
        ));
    }
    let corners = shores(owner).owner();
    let copies = corners.owner();
    let incidence = copies.owner();
    let traces = incidence.owner();
    let regions = traces.owner();
    let lifts = regions.owner().owner();
    let graph = lifts
        .owner()
        .owner()
        .rule()
        .material()
        .base()
        .owner()
        .owner()
        .owner();
    let cover = graph.owner().owner().pieces();
    let original = cover
        .sources()
        .iter()
        .map(cells)
        .collect::<Result<Vec<_>>>()?;
    let mut out = Vec::with_capacity(owner.edges().len());
    for edge in owner.edges() {
        out.push(match edge.kind {
            OriginalPeriodicChartCycleEdgeKind::SourceTrace(trace) => {
                let p = &traces
                    .traces()
                    .get(trace)
                    .ok_or_else(|| refused("Chart winding lost original selected trace"))?
                    .original;
                let c = &original[p.source][p.cell];
                // Use canonical PIECE offsets, never a walk's accumulated deck lift.
                let offset = &cover.pieces()[p.piece].translation;
                let offset = [decode_ratio(&offset[0])?, decode_ratio(&offset[1])?];
                let net = arithmetic::shore_events::translated_net(&c.net, &offset)?;
                let mut bounds = [
                    Bound::decode(&p.parameters[0])?,
                    Bound::decode(&p.parameters[1])?,
                ];
                if p.direction < 0 {
                    bounds.reverse()
                }
                if p.direction != 1 && p.direction != -1 {
                    return Err(refused("Invalid canonical trace winding direction"));
                }
                Prepared::Source {
                    net,
                    domain: c.domain.clone(),
                    bounds,
                    direction: p.direction,
                }
            }
            OriginalPeriodicChartCycleEdgeKind::NativeShore { shore, interval } => {
                Prepared::Frame { shore, interval }
            }
        });
    }
    Ok(out)
}
impl OwnedPeriodicChartWinding {
    pub(super) fn from_fresh_cycles(
        proof: &super::periodic_chart_cycles::FreshChartCycles<'_>,
        limits: PeriodicChartWindingLimits,
    ) -> Result<Self> {
        let prepared = prepare(proof.owner(), limits)?;
        Ok(Self {
            owner: proof.owner().clone(),
            limits,
            prepared,
        })
    }
    pub fn owner(&self) -> &Arc<OwnedPeriodicChartCycles> {
        &self.owner
    }
    fn query(&self, cycle: usize, point: &[Q; 2]) -> Result<Option<i64>> {
        let walk = self
            .owner
            .walks()
            .get(cycle)
            .ok_or_else(|| refused("Unknown exact chart winding cycle"))?;
        self.query_edges(walk, point)
    }
    // Only sibling source-owned loop decompositions may select these edges.
    pub(super) fn query_edges(&self, walk: &[usize], point: &[Q; 2]) -> Result<Option<i64>> {
        if walk.len() > self.limits.max_query_edges {
            return Err(refused("Complete chart cycle ray census exceeds allowance"));
        }
        let mut total = 0i64;
        for &edge in walk {
            let value = match self
                .prepared
                .get(edge)
                .ok_or_else(|| refused("Unknown original winding edge"))?
            {
                Prepared::Source {
                    net,
                    domain,
                    bounds,
                    direction,
                } => {
                    let value = arithmetic::selected_winding::contribution_status(
                        net,
                        domain,
                        &[bounds[0].arithmetic(), bounds[1].arithmetic()],
                        point,
                        self.limits.parameters.max_refinements,
                        self.limits.parameters.max_resultant_size,
                    )?;
                    match value {
                        Some(v) => Some(if *direction == 1 {
                            v
                        } else {
                            v.checked_neg()
                                .ok_or_else(|| refused("Canonical source winding sign overflow"))?
                        }),
                        None => None,
                    }
                }
                Prepared::Frame { shore, interval } => shores(&self.owner).ray_contribution(
                    *shore,
                    *interval,
                    point,
                    self.limits.parameters,
                )?,
            };
            let Some(value) = value else { return Ok(None) };
            total = total
                .checked_add(value)
                .ok_or_else(|| refused("Exact local cycle winding overflow"))?;
        }
        Ok(Some(total))
    }
    /// Winding in the ORIGINAL chart plane, with no periodic normalization.
    /// None proves a point on this cycle; arithmetic/budget failures are errors.
    /// This immutable owned batch was source-verified at construction.
    pub fn winding(&self, cycle: usize, point: &[ExactRatio; 2]) -> Result<Option<i64>> {
        self.query(cycle, &[decode_ratio(&point[0])?, decode_ratio(&point[1])?])
    }
    /// Complete per-cycle vector; None proves contact with at least one cycle.
    pub fn windings(&self, point: &[ExactRatio; 2]) -> Result<Option<Vec<i64>>> {
        if self.prepared.len() > self.limits.max_query_edges {
            return Err(refused("Complete all-cycle ray census exceeds allowance"));
        }
        let p = [decode_ratio(&point[0])?, decode_ratio(&point[1])?];
        let mut out = Vec::with_capacity(self.owner.walks().len());
        for cycle in 0..self.owner.walks().len() {
            let Some(w) = self.query(cycle, &p)? else {
                return Ok(None);
            };
            out.push(w)
        }
        Ok(Some(out))
    }
    /// Regularized material in the chart plane, without period normalization.
    /// None proves contact with a retained material boundary; canceled or neutral
    /// authored sources are not falsely promoted to material boundaries.
    pub fn material(&self, point: &[ExactRatio; 2]) -> Result<Option<bool>> {
        let Some(values) = self.windings(point)? else {
            return Ok(None);
        };
        let sum = values.into_iter().try_fold(0i64, |a, b| {
            a.checked_add(b)
                .ok_or_else(|| refused("Chart material winding sum overflow"))
        })?;
        match sum {
            0 => Ok(Some(false)),
            1 => Ok(Some(true)),
            _ => Err(refused(
                "Chart boundary winding is not a regularized material characteristic",
            )),
        }
    }
    pub fn reverify_source(&self) -> Result<()> {
        let _replay = super::periodic_replay::scope();
        super::periodic_replay::verify_arc(&self.owner, |owner| owner.reverify_source())?;
        if prepare(&self.owner, self.limits)? != self.prepared {
            return Err(refused(
                "Chart winding original cells,cuts or frame expressions changed",
            ));
        }
        Ok(())
    }
}
/// A pinned immutable original-source winding batch. Independently replay its
/// complete source using reverify_source; point queries reuse this validated
/// preparation. No source or frame bounds are fitted or rounded.
pub fn own_periodic_chart_winding(
    owner: Arc<OwnedPeriodicChartCycles>,
    limits: PeriodicChartWindingLimits,
) -> Result<OwnedPeriodicChartWinding> {
    let _replay = super::periodic_replay::scope();
    validate(limits)?;
    super::periodic_replay::verify_arc(&owner, |owner| owner.reverify_source())?;
    let prepared = prepare(&owner, limits)?;
    Ok(OwnedPeriodicChartWinding {
        owner,
        limits,
        prepared,
    })
}

#[cfg(test)]
pub(super) fn check_batch(o: &OwnedPeriodicChartWinding) {
    let point = [["-1".into(), "1".into()], ["1".into(), "2".into()]];
    let mut limited = o.clone();
    limited.limits.max_query_edges = 1;
    assert!(limited.windings(&point).is_err());
    let mut changed = o.clone();
    changed.prepared[0] = Prepared::Frame {
        shore: usize::MAX,
        interval: 0,
    };
    assert!(changed.reverify_source().is_err());
}
