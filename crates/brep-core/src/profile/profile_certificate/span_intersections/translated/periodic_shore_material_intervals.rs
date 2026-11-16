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
const MAX_ALLOWANCE: usize = 1000000;
const MAX_REFINEMENTS: usize = 512;
const MAX_RESULTANT_SIZE: usize = 128;
type ParameterValue = arithmetic::branch_neighbourhood::ParameterValue;
/// One translated source piece against one native shore.
struct ShoreView<'a> {
    shore: usize,
    s: &'a OriginalPeriodicNativeShore,
    /// The piece's cell net translated by the piece offset and period shift.
    net: Vec<[Q; 3]>,
    domain: &'a [Q; 2],
    /// The two cuts that bound the piece inside its cell.
    bounds: &'a [ParameterValue; 2],
    /// Fixed coordinate of the shore.
    level: &'a Q,
}
/// The shore owner and the contact events charged so far.
struct ContactCensus<'a> {
    shores: &'a OwnedPeriodicNativeChartShores,
    parameters: ParameterOrderLimits,
    max_contacts: usize,
    events: usize,
}
impl ContactCensus<'_> {
    fn charge(&mut self) -> Result<()> {
        self.events = self
            .events
            .checked_add(1)
            .filter(|n| *n <= self.max_contacts)
            .ok_or_else(|| refused("Complete native shore contact census exceeds allowance"))?;
        Ok(())
    }
    /// Isolated crossings of the shore that lie between the piece's cuts,
    /// admitted as cuts of the shore.
    fn finite(
        &mut self,
        view: &ShoreView<'_>,
        roots: Vec<arithmetic::shore_events::ShoreRoot>,
    ) -> Result<OriginalPeriodicShoreContactKind> {
        let l = self.parameters;
        let mut cuts = Vec::new();
        for root in roots {
            let order = |bound: &ParameterValue| {
                arithmetic::selected_winding::compare(
                    &root.primitive,
                    &root.root,
                    view.domain,
                    bound,
                    l.max_refinements,
                    l.max_resultant_size,
                )
            };
            if order(&view.bounds[0])? == Ordering::Less
                || order(&view.bounds[1])? == Ordering::Greater
            {
                continue;
            }
            self.charge()?;
            if let Some(cut) = self.shores.admit_cut(
                view.shore,
                &root.primitive,
                &root.root,
                &root.position[1 - view.s.fixed_axis],
            )? {
                cuts.push(cut)
            }
        }
        Ok(OriginalPeriodicShoreContactKind::Finite { cuts })
    }
    /// The whole piece lies on the shore: proves its monotone traversal and
    /// admits both of its ends as cuts of the shore.
    fn coincident(&mut self, view: &ShoreView<'_>) -> Result<OriginalPeriodicShoreContactKind> {
        let l = self.parameters;
        let fixed = view.s.fixed_axis;
        let (_, direction) = arithmetic::shore_carrier::prove(
            &[(view.net.clone(), view.domain.clone())],
            view.bounds,
            fixed,
            view.level,
            l.max_refinements,
            l.max_resultant_size,
        )?;
        let mut ends = [None, None];
        for (end, b) in view.bounds.iter().enumerate() {
            self.charge()?;
            let uv = arithmetic::shore_events::translated_point(
                &view.net,
                view.domain,
                &b.value,
                &b.primitive,
                &b.interval,
                &[Q::zero(), Q::zero()],
                l.max_refinements,
            )?;
            ends[end] =
                self.shores
                    .admit_cut(view.shore, &b.primitive, &b.interval, &uv[1 - fixed])?;
        }
        Ok(OriginalPeriodicShoreContactKind::Coincident { ends, direction })
    }
    /// Complete contact of one translated piece with one shore.
    fn contact(&mut self, view: &ShoreView<'_>) -> Result<OriginalPeriodicShoreContactKind> {
        let roots = arithmetic::shore_events::produce(
            &view.net,
            view.domain,
            view.s.fixed_axis,
            view.level,
            &Q::new(1.into(), 1024.into()),
            self.parameters.max_refinements,
        )?;
        match roots {
            Some(roots) => self.finite(view, roots),
            None => self.coincident(view),
        }
    }
}
/// The two cuts of one piece as exact parameter values.
fn piece_bounds(
    cover: &OwnedOriginalPeriodicPieces,
    p: &OriginalPeriodicPiece,
) -> Result<[ParameterValue; 2]> {
    let bound = |end: usize| -> Result<ParameterValue> {
        let value = &cover.cuts()[p.source][p.cuts[end]].value;
        Ok(ParameterValue {
            primitive: decode_polynomial(&value.primitive)?,
            value: decode_polynomial(&value.parameter)?,
            interval: [
                decode_ratio(&value.interval[0])?,
                decode_ratio(&value.interval[1])?,
            ],
        })
    };
    Ok([bound(0)?, bound(1)?])
}
fn derive(
    o: &OwnedPeriodicShoreMaterialIntervals,
) -> Result<(Vec<OriginalPeriodicShoreContact>, usize)> {
    let _replay = super::periodic_replay::scope();
    if [o.limits.max_source_views, o.limits.max_contacts]
        .iter()
        .any(|n| !(1..=MAX_ALLOWANCE).contains(n))
        || o.parameters.max_refinements > MAX_REFINEMENTS
        || !(1..=MAX_RESULTANT_SIZE).contains(&o.parameters.max_resultant_size)
    {
        return Err(refused(
            "Invalid complete shore material interval allowance",
        ));
    }
    super::periodic_replay::verify_arc(&o.owner, |owner| owner.reverify_source())?;
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
    let mut chart = [[Q::zero(), Q::zero()], [Q::zero(), Q::zero()]];
    for (exact, bounds) in chart.iter_mut().zip(cover.chart()) {
        for (q, v) in exact.iter_mut().zip(bounds) {
            *q = Q::from_float(v)
                .ok_or_else(|| refused("Invalid complete shore material interval allowance"))?;
        }
    }
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
    let mut census = ContactCensus {
        shores,
        parameters: o.parameters,
        max_contacts: o.limits.max_contacts,
        events: 0,
    };
    let mut out = Vec::with_capacity(views);
    for (shore, s) in shores.shores().iter().enumerate() {
        for (piece, p) in cover.pieces().iter().enumerate() {
            let c = &original[p.source][p.cell];
            let bounds = piece_bounds(cover, p)?;
            for &u in &shifts[0] {
                for &v in &shifts[1] {
                    let offset = [
                        decode_ratio(&p.translation[0])? + Q::from_integer(u.into()) * &periods[0],
                        decode_ratio(&p.translation[1])? + Q::from_integer(v.into()) * &periods[1],
                    ];
                    let view = ShoreView {
                        shore,
                        s,
                        net: arithmetic::shore_events::translated_net(&c.net, &offset)?,
                        domain: &c.domain,
                        bounds: &bounds,
                        level: &chart[s.fixed_axis][s.end],
                    };
                    out.push(OriginalPeriodicShoreContact {
                        shore,
                        piece,
                        period_shift: [u, v],
                        kind: census.contact(&view)?,
                    });
                }
            }
        }
    }
    if out.len() != views {
        return Err(refused("Incomplete native shore source-view census"));
    }
    Ok((out, census.events))
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
