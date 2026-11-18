//! Unified exact source cuts and atomic intervals, with common-edge aliases.
use super::super::super::super::algebraic_intersections::{decode_polynomial, decode_ratio};
use super::*;
use chart_intervals::rational;
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
pub(super) fn compare(
    a: &OriginalSourceParameter,
    b: &OriginalSourceParameter,
    l: ParameterOrderLimits,
) -> Result<Ordering> {
    arithmetic::parameter_order::compare(
        arithmetic::parameter_order::ParameterRef::new(
            &decode_polynomial(&a.primitive)?,
            &decode_polynomial(&a.parameter)?,
            &[decode_ratio(&a.interval[0])?, decode_ratio(&a.interval[1])?],
        ),
        arithmetic::parameter_order::ParameterRef::new(
            &decode_polynomial(&b.primitive)?,
            &decode_polynomial(&b.parameter)?,
            &[decode_ratio(&b.interval[0])?, decode_ratio(&b.interval[1])?],
        ),
        l.max_refinements,
        l.max_resultant_size,
    )
}
const MAX_REFINEMENTS: usize = 512;
const MAX_RESULTANT_SIZE: usize = 128;
const MAX_CUT_PROPOSALS: usize = 8192;
/// Allowance on the running work total after matching common edge aliases.
const ALIAS_MATCHING_WORK: usize = 262144;
type RawCut = (OriginalSourceParameter, OriginalSourceCutOrigin);
/// Knot cells of every source, indexed by source.
type SourceCells = Vec<Vec<Cell>>;
/// Unordered cut proposals of every source, bounded in total.
struct Proposals {
    raw: Vec<Vec<RawCut>>,
    total: usize,
}
impl Proposals {
    fn push(
        &mut self,
        source: usize,
        value: OriginalSourceParameter,
        origin: OriginalSourceCutOrigin,
    ) -> Result<()> {
        self.raw[source].push((value, origin));
        self.total += 1;
        if self.total > MAX_CUT_PROPOSALS {
            return Err(refused("Unified original source cut allowance exceeded"));
        }
        Ok(())
    }
}
/// Proposes every knot, pair cut and common partition cut on its source, and
/// returns the proposals together with each source's knot cells.
fn propose(o: &OwnedOriginalSourceCarriers) -> Result<(Vec<Vec<RawCut>>, SourceCells)> {
    let sources = o.dictionary.sources();
    let mut out = Proposals {
        raw: vec![Vec::new(); sources.len()],
        total: 0,
    };
    let mut source_cells = Vec::new();
    for (source, curve) in sources.iter().enumerate() {
        let cells = cells(curve)?;
        for (boundary, q) in atomic_cuts::knots(&cells).enumerate() {
            out.push(
                source,
                rational(q),
                OriginalSourceCutOrigin::Knot { boundary },
            )?;
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
                out.push(
                    o.dictionary.pairs()[pair][side],
                    value(&c.node.exact, side),
                    OriginalSourceCutOrigin::Pair { pair, cut, side },
                )?;
            }
        }
    }
    for (partition, p) in o.dictionary.partitions().iter().enumerate() {
        for (cut, c) in p.owner.cuts().iter().enumerate() {
            for side in 0..2 {
                out.push(
                    o.dictionary.pairs()[p.pair][side],
                    value(&c.exact, side),
                    OriginalSourceCutOrigin::Common {
                        partition,
                        cut,
                        side,
                    },
                )?;
            }
        }
    }
    Ok((out.raw, source_cells))
}
/// One carrier per interval between adjacent cuts, each owned by one knot cell.
fn atomic_carriers(
    cuts: &[Vec<OriginalSourceCut>],
    source_cells: &[Vec<Cell>],
    limits: ParameterOrderLimits,
    work: &mut usize,
) -> Result<Vec<OriginalUnifiedCarrier>> {
    let mut carriers = Vec::new();
    for (source, source_cuts) in cuts.iter().enumerate() {
        let cells = &source_cells[source];
        let values = source_cuts.iter().map(|c| &c.value).collect::<Vec<_>>();
        if !atomic_cuts::spans_domain(&values, cells, limits)? {
            return Err(refused(
                "Unified original source cuts do not cover its full natural domain",
            ));
        }
        for cut in 0..source_cuts.len() - 1 {
            let cell = atomic_cuts::owning_cell(
                cells,
                [&source_cuts[cut].value, &source_cuts[cut + 1].value],
                limits,
                work,
                [
                    "Unified original source cell coverage allowance exceeded",
                    "Unified original interval does not own one source knot cell",
                ],
            )?;
            carriers.push(OriginalUnifiedCarrier {
                source,
                cell,
                cuts: [cut, cut + 1],
                common_edge: None,
                common_uses: Vec::new(),
            });
        }
    }
    Ok(carriers)
}
/// The single carrier of `source` whose two cuts are exactly `endpoints`.
fn carrier_between(
    carriers: &[OriginalUnifiedCarrier],
    cuts: &[Vec<OriginalSourceCut>],
    source: usize,
    endpoints: [&OriginalSourceParameter; 2],
    limits: ParameterOrderLimits,
    work: &mut usize,
) -> Result<usize> {
    let mut selected = Vec::new();
    for (i, c) in carriers
        .iter()
        .enumerate()
        .filter(|(_, c)| c.source == source)
    {
        *work += 1;
        if *work > ALIAS_MATCHING_WORK {
            return Err(refused(
                "Unified original common alias matching allowance exceeded",
            ));
        }
        if compare(&cuts[source][c.cuts[0]].value, endpoints[0], limits)? == Ordering::Equal
            && compare(&cuts[source][c.cuts[1]].value, endpoints[1], limits)? == Ordering::Equal
        {
            selected.push(i);
        }
    }
    match selected[..] {
        [i] => Ok(i),
        _ => Err(refused(
            "Original common edge alias is not one complete atomic source interval",
        )),
    }
}
/// Marks each carrier that realizes a common edge on either side of its pair,
/// requiring one edge identity and one orientation per carrier.
fn bind_common_aliases(
    o: &OwnedOriginalSourceCarriers,
    cuts: &[Vec<OriginalSourceCut>],
    carriers: &mut [OriginalUnifiedCarrier],
    work: &mut usize,
) -> Result<()> {
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
                let index = carrier_between(carriers, cuts, source, endpoints, o.limits, work)?;
                let c = &mut carriers[index];
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
    Ok(())
}
fn derive(
    o: &OwnedOriginalSourceCarriers,
) -> Result<(Vec<Vec<OriginalSourceCut>>, Vec<OriginalUnifiedCarrier>)> {
    if o.limits.max_refinements > MAX_REFINEMENTS
        || !(1..=MAX_RESULTANT_SIZE).contains(&o.limits.max_resultant_size)
    {
        return Err(refused("Invalid unified original source cut limits"));
    }
    o.dictionary.reverify_source()?;
    if o.pairs.len() != o.dictionary.owners().len() {
        return Err(refused("Unified source cuts omit an original pair census"));
    }
    let (raw, source_cells) = propose(o)?;
    let mut work = 0usize;
    let mut cuts = Vec::new();
    for input in raw {
        let ordered = atomic_cuts::merge_ordered(
            input,
            o.limits,
            &mut work,
            "Unified original source ordering allowance exceeded",
        )?;
        cuts.push(
            ordered
                .into_iter()
                .map(|(value, origins)| OriginalSourceCut { value, origins })
                .collect::<Vec<_>>(),
        );
    }
    let mut carriers = atomic_carriers(&cuts, &source_cells, o.limits, &mut work)?;
    bind_common_aliases(o, &cuts, &mut carriers, &mut work)?;
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
