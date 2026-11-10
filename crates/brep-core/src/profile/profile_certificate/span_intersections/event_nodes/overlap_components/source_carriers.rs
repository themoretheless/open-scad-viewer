//! Unified exact source cuts and atomic intervals, with common-edge aliases.
use super::super::super::super::algebraic_intersections::{decode_polynomial, decode_ratio};
use super::*;
use std::cmp::Ordering;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalSourceParameter {
    pub primitive: ExactPolynomial,
    pub interval: [ExactRatio; 2],
    pub parameter: ExactPolynomial,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalSourceCutOrigin {
    Knot {
        boundary: usize,
    },
    Pair {
        pair: usize,
        cut: usize,
        side: usize,
    },
    Common {
        partition: usize,
        cut: usize,
        side: usize,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalSourceCut {
    pub value: OriginalSourceParameter,
    pub origins: Vec<OriginalSourceCutOrigin>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OriginalSourceCommonUse {
    pub edge: usize,
    pub partition: usize,
    pub segment: usize,
    pub side: usize,
    pub reversed: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalUnifiedCarrier {
    pub source: usize,
    pub cell: usize,
    pub cuts: [usize; 2],
    pub common_edge: Option<usize>,
    pub common_uses: Vec<OriginalSourceCommonUse>,
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalSourceCarriers {
    dictionary: Arc<OwnedOriginalCommonDictionary>,
    pairs: Vec<Arc<OwnedOriginalPairCuts>>,
    limits: ParameterOrderLimits,
    cuts: Vec<Vec<OriginalSourceCut>>,
    carriers: Vec<OriginalUnifiedCarrier>,
}
fn value(exact: &AlgebraicEventPosition, side: usize) -> OriginalSourceParameter {
    OriginalSourceParameter {
        primitive: exact.primitive.clone(),
        interval: exact.isolating_interval.clone(),
        parameter: exact.parameters[side].clone(),
    }
}
fn rational(q: &Q) -> OriginalSourceParameter {
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
pub(super) fn compare(
    a: &OriginalSourceParameter,
    b: &OriginalSourceParameter,
    l: ParameterOrderLimits,
) -> Result<Ordering> {
    arithmetic::parameter_order::compare(
        &decode_polynomial(&a.primitive)?,
        &decode_polynomial(&a.parameter)?,
        &[decode_ratio(&a.interval[0])?, decode_ratio(&a.interval[1])?],
        &decode_polynomial(&b.primitive)?,
        &decode_polynomial(&b.parameter)?,
        &[decode_ratio(&b.interval[0])?, decode_ratio(&b.interval[1])?],
        l.max_refinements,
        l.max_resultant_size,
    )
}
fn derive(
    o: &OwnedOriginalSourceCarriers,
) -> Result<(Vec<Vec<OriginalSourceCut>>, Vec<OriginalUnifiedCarrier>)> {
    if o.limits.max_refinements > 512 || !(1..=128).contains(&o.limits.max_resultant_size) {
        return Err(refused("Invalid unified original source cut limits"));
    }
    o.dictionary.reverify_source()?;
    if o.pairs.len() != o.dictionary.owners().len() {
        return Err(refused("Unified source cuts omit an original pair census"));
    }
    let sources = o.dictionary.sources();
    let mut raw = vec![Vec::new(); sources.len()];
    let mut source_cells = Vec::new();
    let mut total = 0usize;
    for (source, curve) in sources.iter().enumerate() {
        let cells = cells(curve)?;
        for (boundary, q) in cells
            .iter()
            .map(|c| &c.domain[0])
            .chain(cells.last().map(|c| &c.domain[1]))
            .enumerate()
        {
            raw[source].push((rational(q), OriginalSourceCutOrigin::Knot { boundary }));
            total += 1;
            if total > 8192 {
                return Err(refused("Unified original source cut allowance exceeded"));
            }
        }
        source_cells.push(cells);
    }
    for (pair, owner) in o.pairs.iter().enumerate() {
        if !Arc::ptr_eq(owner.owner(), o.dictionary.owners()[pair].owner()) {
            return Err(refused("Unified original pair cut owner rebound"));
        }
        owner.reverify_source()?;
        for (cut, c) in owner.cuts().iter().enumerate() {
            for side in 0..2 {
                let source = o.dictionary.pairs()[pair][side];
                raw[source].push((
                    value(&c.node.exact, side),
                    OriginalSourceCutOrigin::Pair { pair, cut, side },
                ));
                total += 1;
                if total > 8192 {
                    return Err(refused("Unified original source cut allowance exceeded"));
                }
            }
        }
    }
    for (partition, p) in o.dictionary.partitions().iter().enumerate() {
        for (cut, c) in p.owner.cuts().iter().enumerate() {
            for side in 0..2 {
                let source = o.dictionary.pairs()[p.pair][side];
                raw[source].push((
                    value(&c.exact, side),
                    OriginalSourceCutOrigin::Common {
                        partition,
                        cut,
                        side,
                    },
                ));
                total += 1;
                if total > 8192 {
                    return Err(refused("Unified original source cut allowance exceeded"));
                }
            }
        }
    }
    if total > 8192 {
        return Err(refused("Unified original source cut allowance exceeded"));
    }
    let mut work = 0usize;
    let mut cuts = Vec::new();
    for input in raw {
        let mut ordered: Vec<OriginalSourceCut> = Vec::new();
        for (value, origin) in input {
            let mut pos = 0;
            let mut joined = false;
            while pos < ordered.len() {
                work += 1;
                if work > 131072 {
                    return Err(refused(
                        "Unified original source ordering allowance exceeded",
                    ));
                }
                match compare(&ordered[pos].value, &value, o.limits)? {
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
                    OriginalSourceCut {
                        value,
                        origins: vec![origin],
                    },
                );
            }
        }
        cuts.push(ordered);
    }
    let mut carriers = Vec::new();
    for (source, source_cuts) in cuts.iter().enumerate() {
        let cells = &source_cells[source];
        if source_cuts.len() < 2
            || compare(
                &source_cuts[0].value,
                &rational(&cells[0].domain[0]),
                o.limits,
            )? != Ordering::Equal
            || compare(
                &source_cuts.last().unwrap().value,
                &rational(&cells.last().unwrap().domain[1]),
                o.limits,
            )? != Ordering::Equal
        {
            return Err(refused(
                "Unified original source cuts do not cover its full natural domain",
            ));
        }
        for cut in 0..source_cuts.len() - 1 {
            let mut selected = Vec::new();
            for (cell, c) in cells.iter().enumerate() {
                work += 1;
                if work > 262144 {
                    return Err(refused(
                        "Unified original source cell coverage allowance exceeded",
                    ));
                }

                if compare(&source_cuts[cut].value, &rational(&c.domain[0]), o.limits)?
                    != Ordering::Less
                    && compare(
                        &source_cuts[cut + 1].value,
                        &rational(&c.domain[1]),
                        o.limits,
                    )? != Ordering::Greater
                {
                    selected.push(cell);
                }
            }
            if selected.len() != 1 {
                return Err(refused(
                    "Unified original interval does not own one source knot cell",
                ));
            }
            carriers.push(OriginalUnifiedCarrier {
                source,
                cell: selected[0],
                cuts: [cut, cut + 1],
                common_edge: None,
                common_uses: Vec::new(),
            });
        }
    }
    for (edge, group) in o.dictionary.edges().iter().enumerate() {
        for alias in &group.uses {
            let p = &o.dictionary.partitions()[alias.partition];
            let bounds = p.owner.segments()[alias.segment].cuts;
            for side in 0..2 {
                let source = o.dictionary.pairs()[p.pair][side];
                let a = value(&p.owner.cuts()[bounds[0]].exact, side);
                let b = value(&p.owner.cuts()[bounds[1]].exact, side);
                let reversed = compare(&a, &b, o.limits)? == Ordering::Greater;
                let endpoints = if reversed { [&b, &a] } else { [&a, &b] };
                let mut selected = Vec::new();
                for (i, c) in carriers
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| c.source == source)
                {
                    work += 1;
                    if work > 262144 {
                        return Err(refused(
                            "Unified original common alias matching allowance exceeded",
                        ));
                    }
                    if compare(&cuts[source][c.cuts[0]].value, endpoints[0], o.limits)?
                        == Ordering::Equal
                        && compare(&cuts[source][c.cuts[1]].value, endpoints[1], o.limits)?
                            == Ordering::Equal
                    {
                        selected.push(i);
                    }
                }
                if selected.len() != 1 {
                    return Err(refused(
                        "Original common edge alias is not one complete atomic source interval",
                    ));
                }
                let c = &mut carriers[selected[0]];
                let direction = reversed ^ alias.reversed;
                if c.common_edge.is_some_and(|e| e != edge)
                    || c.common_uses.iter().any(|u| u.reversed != direction)
                {
                    return Err(refused(
                        "Original atomic carrier common identity or orientation disagrees",
                    ));
                }
                c.common_edge = Some(edge);
                c.common_uses.push(OriginalSourceCommonUse {
                    edge,
                    partition: alias.partition,
                    segment: alias.segment,
                    side,
                    reversed: direction,
                });
            }
        }
    }
    Ok((cuts, carriers))
}
impl OwnedOriginalSourceCarriers {
    pub fn dictionary(&self) -> &Arc<OwnedOriginalCommonDictionary> {
        &self.dictionary
    }
    pub fn pair_cuts(&self) -> &[Arc<OwnedOriginalPairCuts>] {
        &self.pairs
    }
    pub fn cuts(&self) -> &[Vec<OriginalSourceCut>] {
        &self.cuts
    }
    pub fn carriers(&self) -> &[OriginalUnifiedCarrier] {
        &self.carriers
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (cuts, carriers) = derive(self)?;
        if cuts != self.cuts || carriers != self.carriers {
            return Err(refused(
                "Unified original source cuts or common carrier aliases changed",
            ));
        }
        Ok(())
    }
}
pub fn own_original_source_carriers(
    dictionary: Arc<OwnedOriginalCommonDictionary>,
    limits: ParameterOrderLimits,
) -> Result<OwnedOriginalSourceCarriers> {
    dictionary.reverify_source()?;
    let pairs = dictionary
        .owners()
        .iter()
        .map(|o| own_original_pair_cuts(o.owner().clone(), limits).map(Arc::new))
        .collect::<Result<Vec<_>>>()?;
    let mut out = OwnedOriginalSourceCarriers {
        dictionary,
        pairs,
        limits,
        cuts: Vec::new(),
        carriers: Vec::new(),
    };
    (out.cuts, out.carriers) = derive(&out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_rejects_missing_native_cut_carrier_and_common_alias() {
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![2., 0.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![2., 0.], vec![0., 0.]]).unwrap();
        let dictionary = Arc::new(
            own_original_common_dictionary(&[a, b], 1e-12, ParameterOrderLimits::default())
                .unwrap(),
        );
        let x = own_original_source_carriers(dictionary, ParameterOrderLimits::default()).unwrap();
        x.reverify_source().unwrap();
        let mut bad = x.clone();
        bad.cuts[0].pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.cuts[0][0].origins.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.carriers.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.carriers[0].common_edge = None;
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.carriers[0].common_uses[0].reversed = true;
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.carriers[0].cell = 99;
        assert!(bad.reverify_source().is_err());
    }
}
