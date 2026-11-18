//! Full original-curve partition at chart shore roots, without rounded subcurves.
use super::super::super::super::algebraic_intersections::{decode_polynomial, decode_ratio};
use super::*;
use std::cmp::Ordering;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalChartCutOrigin {
    Knot { boundary: usize },
    Event { event: usize },
    Coincident { cell: usize, endpoint: usize },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalChartCut {
    pub value: OriginalSourceParameter,
    pub origins: Vec<OriginalChartCutOrigin>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalChartRegion {
    Below,
    LowerShore,
    Interior,
    UpperShore,
    Above,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalChartInterval {
    pub source: usize,
    pub cell: usize,
    pub cuts: [usize; 2],
    pub witness: ExactRatio,
    /// None means the axis was not selected as periodic, not unknown geometry.
    pub regions: [Option<OriginalChartRegion>; 2],
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalChartIntervals {
    owner: Arc<OwnedOriginalChartEvents>,
    limits: ParameterOrderLimits,
    cuts: Vec<Vec<OriginalChartCut>>,
    intervals: Vec<OriginalChartInterval>,
}
pub(super) fn rational(q: &Q) -> OriginalSourceParameter {
    OriginalSourceParameter {
        primitive: polynomial(&vec![Q::zero(), Q::one()]),
        interval: [ratio(&Q::zero()), ratio(&Q::zero())],
        parameter: polynomial(&if q.is_zero() {
            Vec::new()
        } else {
            vec![q.clone()]
        }),
    }
}
pub(super) fn interior(a: &OriginalSourceParameter, b: &OriginalSourceParameter, steps: usize) -> Result<Q> {
    let decode = |v: &OriginalSourceParameter| -> Result<(Vec<Q>, Vec<Q>, [Q; 2])> {
        Ok((
            decode_polynomial(&v.primitive)?,
            decode_polynomial(&v.parameter)?,
            [decode_ratio(&v.interval[0])?, decode_ratio(&v.interval[1])?],
        ))
    };
    let (f, p, mut x) = decode(a)?;
    let (g, q, mut y) = decode(b)?;
    for step in 0..=steps {
        let lo = arithmetic::parameter_order::range(&p, &x)[1].clone();
        let hi = arithmetic::parameter_order::range(&q, &y)[0].clone();
        if lo < hi {
            return Ok((lo + hi) / Q::from_integer(2.into()));
        }
        if step < steps {
            arithmetic::parameter_order::refine(&f, &mut x)?;
            arithmetic::parameter_order::refine(&g, &mut y)?;
        }
    }
    Err(refused(
        "Original chart interval has no separated rational interior within budget",
    ))
}
const MAX_REFINEMENTS: usize = 512;
const MAX_RESULTANT_SIZE: usize = 128;
const MAX_CUTS: usize = 32768;
type RawCuts = Vec<Vec<(OriginalSourceParameter, OriginalChartCutOrigin)>>;
/// Every knot, chart event and coincident cell end of every source, unordered.
fn raw_cuts(owner: &OwnedOriginalChartEvents, original: &[Vec<Cell>]) -> Result<RawCuts> {
    let mut raw = vec![Vec::new(); owner.sources().len()];
    let mut total = 0usize;
    for (source, cs) in original.iter().enumerate() {
        for (boundary, q) in atomic_cuts::knots(cs).enumerate() {
            raw[source].push((rational(q), OriginalChartCutOrigin::Knot { boundary }));
            total += 1;
        }
    }
    for (event, e) in owner.events().iter().enumerate() {
        raw[e.source].push((e.parameter.clone(), OriginalChartCutOrigin::Event { event }));
        total += 1;
    }
    for (cell, c) in owner.coincident_cells().iter().enumerate() {
        for endpoint in 0..2 {
            raw[c.source].push((
                rational(&decode_ratio(&c.domain[endpoint])?),
                OriginalChartCutOrigin::Coincident { cell, endpoint },
            ));
            total += 1;
        }
    }
    if total > MAX_CUTS {
        return Err(refused(
            "Original chart interval cut census allowance exceeded",
        ));
    }
    Ok(raw)
}
/// Where a coordinate lies against one chart axis.
fn region_of(q: &Q, bounds: &[Q; 2]) -> OriginalChartRegion {
    match (q.cmp(&bounds[0]), q.cmp(&bounds[1])) {
        (Ordering::Less, _) => OriginalChartRegion::Below,
        (Ordering::Equal, _) => OriginalChartRegion::LowerShore,
        (_, Ordering::Equal) => OriginalChartRegion::UpperShore,
        (_, Ordering::Greater) => OriginalChartRegion::Above,
        _ => OriginalChartRegion::Interior,
    }
}
/// Ordered cuts of the sources and the intervals between them.
struct IntervalCensus<'a> {
    owner: &'a OwnedOriginalChartEvents,
    original: &'a [Vec<Cell>],
    chart: [[Q; 2]; 2],
    l: ParameterOrderLimits,
    work: usize,
}
impl IntervalCensus<'_> {
    /// A witness on a chart shore must belong to a proved coincident cell.
    fn verify_shore(
        &self,
        source: usize,
        cell: usize,
        axis: usize,
        region: &OriginalChartRegion,
    ) -> Result<()> {
        let shore = match region {
            OriginalChartRegion::LowerShore => 0,
            OriginalChartRegion::UpperShore => 1,
            _ => return Ok(()),
        };
        let coincident =
            self.owner.coincident_cells().iter().any(|x| {
                x.source == source && x.cell == cell && x.axis == axis && x.shore == shore
            });
        if !coincident {
            return Err(refused(
                "A finite chart root lies inside an allegedly root-free interval",
            ));
        }
        Ok(())
    }
    /// The interval between two consecutive cuts: its owning cell, an
    /// interior witness and the chart region of the witness on wrapped axes.
    fn interval(
        &mut self,
        source: usize,
        values: &[OriginalChartCut],
        cut: usize,
    ) -> Result<OriginalChartInterval> {
        let cs = &self.original[source];
        let ends = [&values[cut].value, &values[cut + 1].value];
        let cell = atomic_cuts::owning_cell(
            cs,
            ends,
            self.l,
            &mut self.work,
            [
                "Original chart cell coverage allowance exceeded",
                "Original chart interval does not own one actual knot cell",
            ],
        )?;
        let c = &cs[cell];
        let witness = interior(ends[0], ends[1], self.l.max_refinements)?;
        let local = (&witness - &c.domain[0]) / (&c.domain[1] - &c.domain[0]);
        let point = arithmetic::shore_events::point(&c.net, &local)?;
        let mut regions = [None, None];
        for axis in 0..2 {
            if !self.owner.wrapped()[axis] {
                continue;
            }
            let region = region_of(&point[axis], &self.chart[axis]);
            self.verify_shore(source, cell, axis, &region)?;
            regions[axis] = Some(region);
        }
        Ok(OriginalChartInterval {
            source,
            cell,
            cuts: [cut, cut + 1],
            witness: ratio(&witness),
            regions,
        })
    }
}
type ChartIntervals = (Vec<Vec<OriginalChartCut>>, Vec<OriginalChartInterval>);
fn derive(owner: &OwnedOriginalChartEvents, l: ParameterOrderLimits) -> Result<ChartIntervals> {
    if l.max_refinements > MAX_REFINEMENTS
        || !(1..=MAX_RESULTANT_SIZE).contains(&l.max_resultant_size)
    {
        return Err(refused("Invalid original chart interval comparison limits"));
    }
    owner.reverify_source()?;
    let original = owner
        .sources()
        .iter()
        .map(cells)
        .collect::<Result<Vec<_>>>()?;
    let raw = raw_cuts(owner, &original)?;
    let mut work = 0usize;
    let mut cuts = Vec::new();
    for input in raw {
        let ordered = atomic_cuts::merge_ordered(
            input,
            l,
            &mut work,
            "Original chart cut ordering allowance exceeded",
        )?;
        cuts.push(
            ordered
                .into_iter()
                .map(|(value, origins)| OriginalChartCut { value, origins })
                .collect::<Vec<_>>(),
        );
    }
    let mut chart = [[Q::zero(), Q::zero()], [Q::zero(), Q::zero()]];
    for (exact, bounds) in chart.iter_mut().zip(owner.chart()) {
        for (q, v) in exact.iter_mut().zip(bounds) {
            *q = Q::from_float(v).ok_or_else(|| refused("Original chart bound is not finite"))?;
        }
    }
    let mut census = IntervalCensus {
        owner,
        original: &original,
        chart,
        l,
        work,
    };
    let mut intervals = Vec::new();
    for (source, values) in cuts.iter().enumerate() {
        let parameters = values.iter().map(|c| &c.value).collect::<Vec<_>>();
        if !atomic_cuts::spans_domain(&parameters, &original[source], l)? {
            return Err(refused(
                "Original chart intervals do not cover the full source domain",
            ));
        }
        for cut in 0..values.len() - 1 {
            intervals.push(census.interval(source, values, cut)?);
        }
    }
    Ok((cuts, intervals))
}
impl OwnedOriginalChartIntervals {
    pub fn owner(&self) -> &Arc<OwnedOriginalChartEvents> {
        &self.owner
    }
    pub fn cuts(&self) -> &[Vec<OriginalChartCut>] {
        &self.cuts
    }
    pub fn intervals(&self) -> &[OriginalChartInterval] {
        &self.intervals
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (c, i) = derive(&self.owner, self.limits)?;
        if c != self.cuts || i != self.intervals {
            return Err(refused(
                "Original chart source partition or root-free classification changed",
            ));
        }
        Ok(())
    }
}
pub fn own_original_chart_intervals(
    owner: Arc<OwnedOriginalChartEvents>,
    limits: ParameterOrderLimits,
) -> Result<OwnedOriginalChartIntervals> {
    let (cuts, intervals) = derive(&owner, limits)?;
    Ok(OwnedOriginalChartIntervals {
        owner,
        limits,
        cuts,
        intervals,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_rejects_cut_origin_interval_region_and_cell_changes() {
        let a = Curve::from_polyline(vec![vec![-1., 0.], vec![1., 1.]]).unwrap();
        let e = Arc::new(
            own_original_chart_events(&[a], [[0., 2.], [-2., 2.]], [true, false], 1e-12, 192)
                .unwrap(),
        );
        let x = own_original_chart_intervals(e, ParameterOrderLimits::default()).unwrap();
        x.reverify_source().unwrap();
        let mut bad = x.clone();
        bad.cuts[0][1].origins.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.intervals.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.intervals[0].regions[0] = Some(OriginalChartRegion::Interior);
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.intervals[0].cell = 99;
        assert!(bad.reverify_source().is_err());
    }
}
