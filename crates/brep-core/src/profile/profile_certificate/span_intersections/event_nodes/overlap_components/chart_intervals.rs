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
fn derive(
    owner: &OwnedOriginalChartEvents,
    l: ParameterOrderLimits,
) -> Result<(Vec<Vec<OriginalChartCut>>, Vec<OriginalChartInterval>)> {
    if l.max_refinements > 512 || !(1..=128).contains(&l.max_resultant_size) {
        return Err(refused("Invalid original chart interval comparison limits"));
    }
    owner.reverify_source()?;
    let original = owner
        .sources()
        .iter()
        .map(cells)
        .collect::<Result<Vec<_>>>()?;
    let mut raw = vec![Vec::new(); owner.sources().len()];
    let mut total = 0usize;
    for (source, cs) in original.iter().enumerate() {
        for (boundary, q) in cs
            .iter()
            .map(|c| &c.domain[0])
            .chain(cs.last().map(|c| &c.domain[1]))
            .enumerate()
        {
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
    if total > 32768 {
        return Err(refused(
            "Original chart interval cut census allowance exceeded",
        ));
    }
    let mut cuts = Vec::new();
    let mut work = 0usize;
    for input in raw {
        let mut ordered: Vec<OriginalChartCut> = Vec::new();
        for (value, origin) in input {
            let mut pos = 0;
            let mut joined = false;
            while pos < ordered.len() {
                work += 1;
                if work > 131072 {
                    return Err(refused("Original chart cut ordering allowance exceeded"));
                }
                match source_carriers::compare(&ordered[pos].value, &value, l)? {
                    Ordering::Equal => {
                        ordered[pos].origins.push(origin);
                        joined = true;
                        break;
                    }
                    Ordering::Greater => break,
                    Ordering::Less => pos += 1,
                }
            }
            if !joined {
                ordered.insert(
                    pos,
                    OriginalChartCut {
                        value,
                        origins: vec![origin],
                    },
                );
            }
        }
        cuts.push(ordered);
    }
    let mut intervals = Vec::new();
    let chart = owner.chart().map(|a| a.map(|q| Q::from_float(q).unwrap()));
    for (source, values) in cuts.iter().enumerate() {
        let cs = &original[source];
        if values.len() < 2
            || source_carriers::compare(&values[0].value, &rational(&cs[0].domain[0]), l)?
                != Ordering::Equal
            || source_carriers::compare(
                &values.last().unwrap().value,
                &rational(&cs.last().unwrap().domain[1]),
                l,
            )? != Ordering::Equal
        {
            return Err(refused(
                "Original chart intervals do not cover the full source domain",
            ));
        }
        for cut in 0..values.len() - 1 {
            let mut found = Vec::new();
            for (cell, c) in cs.iter().enumerate() {
                work += 1;
                if work > 262144 {
                    return Err(refused("Original chart cell coverage allowance exceeded"));
                }
                if source_carriers::compare(&values[cut].value, &rational(&c.domain[0]), l)?
                    != Ordering::Less
                    && source_carriers::compare(&values[cut + 1].value, &rational(&c.domain[1]), l)?
                        != Ordering::Greater
                {
                    found.push(cell);
                }
            }
            if found.len() != 1 {
                return Err(refused(
                    "Original chart interval does not own one actual knot cell",
                ));
            }
            let cell = found[0];
            let c = &cs[cell];
            let witness = interior(
                &values[cut].value,
                &values[cut + 1].value,
                l.max_refinements,
            )?;
            let local = (&witness - &c.domain[0]) / (&c.domain[1] - &c.domain[0]);
            let point = arithmetic::shore_events::point(&c.net, &local)?;
            let mut regions = [None, None];
            for axis in 0..2 {
                if !owner.wrapped()[axis] {
                    continue;
                }
                let region = match (
                    point[axis].cmp(&chart[axis][0]),
                    point[axis].cmp(&chart[axis][1]),
                ) {
                    (Ordering::Less, _) => OriginalChartRegion::Below,
                    (Ordering::Equal, _) => OriginalChartRegion::LowerShore,
                    (_, Ordering::Equal) => OriginalChartRegion::UpperShore,
                    (_, Ordering::Greater) => OriginalChartRegion::Above,
                    _ => OriginalChartRegion::Interior,
                };
                let shore = match region {
                    OriginalChartRegion::LowerShore => Some(0),
                    OriginalChartRegion::UpperShore => Some(1),
                    _ => None,
                };
                if let Some(shore) = shore {
                    if !owner.coincident_cells().iter().any(|x| {
                        x.source == source && x.cell == cell && x.axis == axis && x.shore == shore
                    }) {
                        return Err(refused(
                            "A finite chart root lies inside an allegedly root-free interval",
                        ));
                    }
                }
                regions[axis] = Some(region);
            }
            intervals.push(OriginalChartInterval {
                source,
                cell,
                cuts: [cut, cut + 1],
                witness: ratio(&witness),
                regions,
            });
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
