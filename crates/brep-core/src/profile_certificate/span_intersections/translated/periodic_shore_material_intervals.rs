//! Constant chart-inside material germs on actual open native frame intervals.
//! The complete original-source contact census is separate from point labels.
use super::*;
use crate::profile_certificate::algebraic_intersections::{decode_polynomial, decode_ratio};
use std::{cmp::Ordering, sync::Arc};
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OriginalPeriodicShoreContactKind {
    Finite {
        cuts: Vec<usize>,
    },
    /// Every active source cell is identically on the shore, with a complete
    /// derivative-root census proving regular monotone traversal on its cuts.
    Coincident {
        ends: [Option<usize>; 2],
        direction: i8,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicShoreContact {
    pub shore: usize,
    pub piece: usize,
    pub period_shift: [i8; 2],
    pub kind: OriginalPeriodicShoreContactKind,
}
#[derive(Clone, Copy, Debug)]
pub struct PeriodicShoreMaterialIntervalLimits {
    pub max_source_views: usize,
    pub max_contacts: usize,
}
impl Default for PeriodicShoreMaterialIntervalLimits {
    fn default() -> Self {
        Self {
            max_source_views: 1000000,
            max_contacts: 1000000,
        }
    }
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicShoreMaterialIntervals {
    owner: Arc<OwnedPeriodicShoreMaterialSamples>,
    parameters: ParameterOrderLimits,
    limits: PeriodicShoreMaterialIntervalLimits,
    contacts: Vec<OriginalPeriodicShoreContact>,
    events: usize,
}
fn derive(
    o: &OwnedPeriodicShoreMaterialIntervals,
) -> Result<(Vec<OriginalPeriodicShoreContact>, usize)> {
    let _replay = super::periodic_replay::scope();
    if [o.limits.max_source_views, o.limits.max_contacts]
        .iter()
        .any(|n| !(1..=1000000).contains(n))
        || o.parameters.max_refinements > 512
        || !(1..=128).contains(&o.parameters.max_resultant_size)
    {
        return Err(refused(
            "Invalid complete shore material interval allowance",
        ));
    }
    o.owner.reverify_source()?;
    let shores = o.owner.owner();
    let corners = shores.owner();
    let copies = corners.owner();
    let incidence = copies.owner();
    let traces = incidence.owner();
    let regions = traces.owner();
    let lifts = regions.owner().owner();
    let selection = lifts.owner().owner();
    let graph = selection.rule().material().base().owner().owner().owner();
    let cover = graph.owner().owner().pieces();
    let chart = cover.chart().map(|d| d.map(|v| Q::from_float(v).unwrap()));
    let periods = chart.clone().map(|d| d[1].clone() - &d[0]);
    let shifts: [Vec<i8>; 2] = cover
        .wrapped()
        .map(|wrapped| if wrapped { vec![-1, 0, 1] } else { vec![0] });
    let views = cover
        .pieces()
        .len()
        .checked_mul(shifts[0].len())
        .and_then(|n| n.checked_mul(shifts[1].len()))
        .and_then(|n| n.checked_mul(shores.shores().len()))
        .filter(|n| *n <= o.limits.max_source_views)
        .ok_or_else(|| refused("Complete native shore source-view census exceeds allowance"))?;
    let original = cover
        .sources()
        .iter()
        .map(cells)
        .collect::<Result<Vec<_>>>()?;
    let mut out = Vec::with_capacity(views);
    let mut events = 0usize;
    let mut charge = || -> Result<()> {
        events = events
            .checked_add(1)
            .filter(|n| *n <= o.limits.max_contacts)
            .ok_or_else(|| refused("Complete native shore contact census exceeds allowance"))?;
        Ok(())
    };
    for (shore, s) in shores.shores().iter().enumerate() {
        for (piece, p) in cover.pieces().iter().enumerate() {
            let c = &original[p.source][p.cell];
            let bound = |end: usize| -> Result<arithmetic::branch_neighbourhood::ParameterValue> {
                let value = &cover.cuts()[p.source][p.cuts[end]].value;
                Ok(arithmetic::branch_neighbourhood::ParameterValue {
                    primitive: decode_polynomial(&value.primitive)?,
                    value: decode_polynomial(&value.parameter)?,
                    interval: [
                        decode_ratio(&value.interval[0])?,
                        decode_ratio(&value.interval[1])?,
                    ],
                })
            };
            let bounds = [bound(0)?, bound(1)?];
            for &u in &shifts[0] {
                for &v in &shifts[1] {
                    let offset = [
                        decode_ratio(&p.translation[0])? + Q::from_integer(u.into()) * &periods[0],
                        decode_ratio(&p.translation[1])? + Q::from_integer(v.into()) * &periods[1],
                    ];
                    let net = arithmetic::shore_events::translated_net(&c.net, &offset)?;
                    let roots = arithmetic::shore_events::produce(
                        &net,
                        &c.domain,
                        s.fixed_axis,
                        &chart[s.fixed_axis][s.end],
                        &Q::new(1.into(), 1024.into()),
                        o.parameters.max_refinements,
                    )?;
                    let kind = if let Some(roots) = roots {
                        let mut cuts = Vec::new();
                        for root in roots {
                            if arithmetic::selected_winding::compare(
                                &root.primitive,
                                &root.root,
                                &c.domain,
                                &bounds[0],
                                o.parameters.max_refinements,
                                o.parameters.max_resultant_size,
                            )? == Ordering::Less
                                || arithmetic::selected_winding::compare(
                                    &root.primitive,
                                    &root.root,
                                    &c.domain,
                                    &bounds[1],
                                    o.parameters.max_refinements,
                                    o.parameters.max_resultant_size,
                                )? == Ordering::Greater
                            {
                                continue;
                            }
                            charge()?;
                            if let Some(cut) = shores.admit_cut(
                                shore,
                                &root.primitive,
                                &root.root,
                                &root.position[1 - s.fixed_axis],
                            )? {
                                cuts.push(cut)
                            }
                        }
                        OriginalPeriodicShoreContactKind::Finite { cuts }
                    } else {
                        let (_, direction) = arithmetic::shore_carrier::prove(
                            &[(net.clone(), c.domain.clone())],
                            &bounds,
                            s.fixed_axis,
                            &chart[s.fixed_axis][s.end],
                            o.parameters.max_refinements,
                            o.parameters.max_resultant_size,
                        )?;
                        let mut ends = [None, None];
                        for end in 0..2 {
                            charge()?;
                            let b = &bounds[end];
                            let uv = arithmetic::shore_events::translated_point(
                                &net,
                                &c.domain,
                                &b.value,
                                &b.primitive,
                                &b.interval,
                                &[Q::zero(), Q::zero()],
                                o.parameters.max_refinements,
                            )?;
                            ends[end] = shores.admit_cut(
                                shore,
                                &b.primitive,
                                &b.interval,
                                &uv[1 - s.fixed_axis],
                            )?;
                        }
                        OriginalPeriodicShoreContactKind::Coincident { ends, direction }
                    };
                    out.push(OriginalPeriodicShoreContact {
                        shore,
                        piece,
                        period_shift: [u, v],
                        kind,
                    });
                }
            }
        }
    }
    if out.len() != views {
        return Err(refused("Incomplete native shore source-view census"));
    }
    Ok((out, events))
}
impl OwnedPeriodicShoreMaterialIntervals {
    pub fn owner(&self) -> &Arc<OwnedPeriodicShoreMaterialSamples> {
        &self.owner
    }
    pub fn contacts(&self) -> &[OriginalPeriodicShoreContact] {
        &self.contacts
    }
    pub fn contact_events(&self) -> usize {
        self.events
    }
    /// The chart-inside MATERIAL GERM is constant over this whole OPEN native
    /// interval. Endpoint strata are retained separately by the owned frame.
    pub fn selected(&self, shore: usize, interval: usize) -> Result<bool> {
        self.owner
            .samples()
            .iter()
            .find(|s| s.shore == shore && s.interval == interval)
            .map(|s| s.selected)
            .ok_or_else(|| refused("Unknown certified native material interval"))
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (contacts, events) = derive(self)?;
        if contacts != self.contacts || events != self.events {
            return Err(refused(
                "Native shore complete contact census or material interval proof changed",
            ));
        }
        Ok(())
    }
}
/// Full finite source-shore contacts and monotone coincident image endpoints
/// occur only at the owned frame cuts. Between these cuts, a one-sided material
/// germ cannot change: all authored source curves are continuous algebraic cells
/// and all operand labels descend through the actual support periods. The
/// owner's cleared-leg or source-side point witness fixes that constant label.
/// Critical coincident carriers that lack a monotone proof refuse; no sampling,
/// empty search or finite budget may stand in for this completeness proof.
/// This does not publish physical face cycles or sew world-space edges.
pub fn own_periodic_shore_material_intervals(
    owner: Arc<OwnedPeriodicShoreMaterialSamples>,
    parameters: ParameterOrderLimits,
    limits: PeriodicShoreMaterialIntervalLimits,
) -> Result<OwnedPeriodicShoreMaterialIntervals> {
    let mut o = OwnedPeriodicShoreMaterialIntervals {
        owner,
        parameters,
        limits,
        contacts: Vec::new(),
        events: 0,
    };
    let (contacts, events) = derive(&o)?;
    o.contacts = contacts;
    o.events = events;
    Ok(o)
}
