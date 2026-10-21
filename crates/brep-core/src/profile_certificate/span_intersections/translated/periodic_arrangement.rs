//! Complete same-chart pair census and source-owned periodic interval seeds.
use super::*;
use periodic_events::compare;
use std::{cmp::Ordering, sync::Arc};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalPeriodicArrangementOrigin {
    Boundary {
        end: usize,
    },
    Event {
        pair: usize,
        event: usize,
        side: usize,
    },
    Common {
        pair: usize,
        clip: usize,
        end: usize,
        side: usize,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicArrangementCut {
    pub value: OriginalSourceParameter,
    pub origins: Vec<OriginalPeriodicArrangementOrigin>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicArrangementUse {
    pub pair: usize,
    pub clip: usize,
    pub side: usize,
    pub reversed: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicArrangementInterval {
    pub piece: usize,
    pub source: usize,
    pub cell: usize,
    pub cuts: [usize; 2],
    pub common_uses: Vec<OriginalPeriodicArrangementUse>,
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicArrangementCuts {
    pieces: Arc<OwnedOriginalPeriodicPieces>,
    pairs: Vec<Arc<OwnedPeriodicCommonClips>>,
    seams: Option<Arc<OwnedPeriodicSeamContacts>>,
    limits: ParameterOrderLimits,
    max_pairs: usize,
    cuts: Vec<Vec<OriginalPeriodicArrangementCut>>,
    intervals: Vec<OriginalPeriodicArrangementInterval>,
}
fn value(n: &AlgebraicEventNode, side: usize) -> OriginalSourceParameter {
    OriginalSourceParameter {
        primitive: n.exact.primitive.clone(),
        interval: n.exact.isolating_interval.clone(),
        parameter: n.exact.parameters[side].clone(),
    }
}
fn insert(
    cuts: &mut Vec<OriginalPeriodicArrangementCut>,
    value: OriginalSourceParameter,
    origin: OriginalPeriodicArrangementOrigin,
    limits: ParameterOrderLimits,
) -> Result<()> {
    for i in 0..cuts.len() {
        match compare(&value, &cuts[i].value, limits)? {
            Ordering::Equal => {
                if !cuts[i].origins.contains(&origin) {
                    cuts[i].origins.push(origin);
                }
                return Ok(());
            }
            Ordering::Less => {
                cuts.insert(
                    i,
                    OriginalPeriodicArrangementCut {
                        value,
                        origins: vec![origin],
                    },
                );
                return Ok(());
            }
            Ordering::Greater => {}
        }
    }
    cuts.push(OriginalPeriodicArrangementCut {
        value,
        origins: vec![origin],
    });
    Ok(())
}
fn derive(
    o: &OwnedPeriodicArrangementCuts,
) -> Result<(
    Vec<Vec<OriginalPeriodicArrangementCut>>,
    Vec<OriginalPeriodicArrangementInterval>,
)> {
    let n = o.pieces.pieces().len();
    let count = n
        .checked_add(1)
        .and_then(|v| n.checked_mul(v))
        .map(|v| v / 2)
        .ok_or_else(|| refused("Periodic pair count overflow"))?;
    let total = count
        .checked_add(o.seams.as_ref().map_or(0, |s| s.pairs().len()))
        .ok_or_else(|| refused("Combined periodic pair count overflow"))?;
    if o.max_pairs == 0
        || o.max_pairs > 4096
        || total > o.max_pairs
        || o.pairs.len() != total
        || o.limits.max_refinements > 512
        || !(1..=128).contains(&o.limits.max_resultant_size)
    {
        return Err(refused(
            "Incomplete periodic pair census or invalid arrangement allowance",
        ));
    }
    super::periodic_replay::verify_arc(&o.pieces, |p| p.reverify_source())?;
    let mut ordinal = 0;
    for a in 0..n {
        for b in a..n {
            let p = &o.pairs[ordinal];
            if p.owner().pair() != [a, b]
                || !Arc::ptr_eq(p.owner().pieces(), &o.pieces)
                || p.owner()
                    .second_translation_delta()
                    .iter()
                    .any(|v| !algebraic_intersections::decode_ratio(v).is_ok_and(|q| q.is_zero()))
            {
                return Err(refused("Zero-shift chart pair census reordered or shifted"));
            }
            ordinal += 1;
        }
    }
    if let Some(seams) = &o.seams {
        if !Arc::ptr_eq(seams.embedding().wires().surface().pieces(), &o.pieces) {
            return Err(refused(
                "Actual seam contacts and chart arrangement use different source covers",
            ));
        }
        super::periodic_replay::verify_arc(seams, |s| s.reverify_source())?;
        for (i, p) in seams.pairs().iter().enumerate() {
            if !Arc::ptr_eq(&o.pairs[count + i], &p.owner) {
                return Err(refused(
                    "Combined arrangement lost actual seam pair provenance",
                ));
            }
        }
    }
    let mut cuts = vec![Vec::new(); n];
    for (piece, p) in o.pieces.pieces().iter().enumerate() {
        for end in 0..2 {
            insert(
                &mut cuts[piece],
                o.pieces.cuts()[p.source][p.cuts[end]].value.clone(),
                OriginalPeriodicArrangementOrigin::Boundary { end },
                o.limits,
            )?;
        }
    }
    for (index, pair) in o.pairs.iter().enumerate() {
        let events = pair.owner();
        let [a, b] = events.pair();
        if !Arc::ptr_eq(events.pieces(), &o.pieces) {
            return Err(refused(
                "Combined contact uses unrelated original source cover",
            ));
        }
        super::periodic_replay::verify_arc(pair, |p| p.reverify_source())?;
        for (event, node) in events.events().iter().enumerate() {
            if node.in_both_closed_pieces() {
                for (side, piece) in [a, b].into_iter().enumerate() {
                    insert(
                        &mut cuts[piece],
                        value(&node.node, side),
                        OriginalPeriodicArrangementOrigin::Event {
                            pair: index,
                            event,
                            side,
                        },
                        o.limits,
                    )?;
                }
            }
        }
        for (clip, c) in pair.clips().iter().enumerate() {
            for end in 0..2 {
                for (side, piece) in [a, b].into_iter().enumerate() {
                    insert(
                        &mut cuts[piece],
                        value(&c.endpoints[end], side),
                        OriginalPeriodicArrangementOrigin::Common {
                            pair: index,
                            clip,
                            end,
                            side,
                        },
                        o.limits,
                    )?;
                }
            }
        }
    }
    let mut intervals = Vec::new();
    for (piece, p) in o.pieces.pieces().iter().enumerate() {
        if cuts[piece].len() < 2 {
            return Err(refused(
                "Periodic source piece lost its closed boundary cuts",
            ));
        }
        for i in 0..cuts[piece].len() - 1 {
            let mut uses = Vec::new();
            for (pair, owner) in o.pairs.iter().enumerate() {
                for (side, id) in owner.owner().pair().into_iter().enumerate() {
                    if id != piece {
                        continue;
                    }
                    for (clip, c) in owner.clips().iter().enumerate() {
                        if c.point_only {
                            continue;
                        }
                        let a = value(&c.endpoints[0], side);
                        let b = value(&c.endpoints[1], side);
                        let reversed = compare(&a, &b, o.limits)? == Ordering::Greater;
                        let (lo, hi) = if reversed { (&b, &a) } else { (&a, &b) };
                        if compare(&cuts[piece][i].value, lo, o.limits)? != Ordering::Less
                            && compare(&cuts[piece][i + 1].value, hi, o.limits)?
                                != Ordering::Greater
                        {
                            uses.push(OriginalPeriodicArrangementUse {
                                pair,
                                clip,
                                side,
                                reversed,
                            });
                        }
                    }
                }
            }
            intervals.push(OriginalPeriodicArrangementInterval {
                piece,
                source: p.source,
                cell: p.cell,
                cuts: [i, i + 1],
                common_uses: uses,
            });
        }
    }
    Ok((cuts, intervals))
}
impl OwnedPeriodicArrangementCuts {
    pub fn pieces(&self) -> &Arc<OwnedOriginalPeriodicPieces> {
        &self.pieces
    }
    pub fn seam_contacts(&self) -> Option<&Arc<OwnedPeriodicSeamContacts>> {
        self.seams.as_ref()
    }
    pub fn pairs(&self) -> &[Arc<OwnedPeriodicCommonClips>] {
        &self.pairs
    }
    pub fn cuts(&self) -> &[Vec<OriginalPeriodicArrangementCut>] {
        &self.cuts
    }
    pub fn intervals(&self) -> &[OriginalPeriodicArrangementInterval] {
        &self.intervals
    }
    pub fn reverify_source(&self) -> Result<()> {
        let _replay = super::periodic_replay::scope();
        let (c, i) = derive(self)?;
        if c != self.cuts || i != self.intervals {
            return Err(refused(
                "Periodic arrangement cut or interval provenance changed",
            ));
        }
        Ok(())
    }
}
/// Complete unordered pair census INCLUDING self-piece obligations. Existing
/// exact common/finite algorithms must admit EVERY pair; no skipped unknowns.
/// These are source interval seeds, not a physical quotient graph: third-source
/// cuts still need propagation through all common counterparts before aliasing.
pub fn own_periodic_arrangement_cuts(
    pieces: Arc<OwnedOriginalPeriodicPieces>,
    width: f64,
    limits: ParameterOrderLimits,
    max_pairs: usize,
) -> Result<OwnedPeriodicArrangementCuts> {
    let _replay = super::periodic_replay::scope();
    let n = pieces.pieces().len();
    let count = n
        .checked_add(1)
        .and_then(|v| n.checked_mul(v))
        .map(|v| v / 2)
        .ok_or_else(|| refused("Periodic pair count overflow"))?;
    if max_pairs == 0 || max_pairs > 4096 || count > max_pairs {
        return Err(refused("Complete periodic pair census exceeds allowance"));
    }
    super::periodic_replay::verify_arc(&pieces, |p| p.reverify_source())?;
    let mut pairs = Vec::new();
    for a in 0..n {
        for b in a..n {
            pairs.push(Arc::new(own_periodic_common_clips(Arc::new(
                own_periodic_pair_events(pieces.clone(), [a, b], width, limits)?,
            ))?));
        }
    }
    let mut o = OwnedPeriodicArrangementCuts {
        pieces,
        pairs,
        seams: None,
        limits,
        max_pairs,
        cuts: Vec::new(),
        intervals: Vec::new(),
    };
    let (cuts, intervals) = derive(&o)?;
    o.cuts = cuts;
    o.intervals = intervals;
    Ok(o)
}
/// Joins the complete zero-shift census with EVERY actual-support seam pair.
/// Re-derives all source cuts and intervals rather than appending unmatched
/// physical vertices to a graph whose source intervals were never split.
pub fn own_periodic_quotient_arrangement_cuts(
    chart: Arc<OwnedPeriodicArrangementCuts>,
    seams: Arc<OwnedPeriodicSeamContacts>,
    limits: ParameterOrderLimits,
    max_pairs: usize,
) -> Result<OwnedPeriodicArrangementCuts> {
    let _replay = super::periodic_replay::scope();
    if chart.seams.is_some() {
        return Err(refused("Expected an unquotiented chart arrangement"));
    }
    if !Arc::ptr_eq(chart.pieces(), seams.embedding().wires().surface().pieces()) {
        return Err(refused(
            "Quotient arrangement requires the identical actual source cover",
        ));
    }
    let total = chart
        .pairs
        .len()
        .checked_add(seams.pairs().len())
        .ok_or_else(|| refused("Combined periodic pair count overflow"))?;
    if max_pairs == 0 || max_pairs > 4096 || total > max_pairs {
        return Err(refused(
            "Complete combined chart/seam census exceeds allowance",
        ));
    }
    super::periodic_replay::verify(&chart)?;
    super::periodic_replay::verify_arc(&seams, |s| s.reverify_source())?;
    let mut pairs = chart.pairs.clone();
    pairs.extend(seams.pairs().iter().map(|p| p.owner.clone()));
    let mut o = OwnedPeriodicArrangementCuts {
        pieces: chart.pieces.clone(),
        pairs,
        seams: Some(seams),
        limits,
        max_pairs,
        cuts: Vec::new(),
        intervals: Vec::new(),
    };
    let (cuts, intervals) = derive(&o)?;
    o.cuts = cuts;
    o.intervals = intervals;
    Ok(o)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn pieces(a: Curve, b: Curve) -> Arc<OwnedOriginalPeriodicPieces> {
        Arc::new(
            own_original_periodic_pieces(
                &[a, b],
                [[0., 1.], [0., 1.]],
                [false, false],
                1e-12,
                ParameterOrderLimits::default(),
                16,
            )
            .unwrap(),
        )
    }
    #[test]
    fn complete_crossing_pair_cover_splits_both_original_clocks_and_preserves_origins() {
        let a = Curve::from_polyline(vec![vec![0., 0.5], vec![1., 0.5]]).unwrap();
        let b = Curve::from_polyline(vec![vec![0.5, 0.], vec![0.5, 1.]]).unwrap();
        let cover = pieces(a, b);
        assert!(
            own_periodic_arrangement_cuts(cover.clone(), 1e-12, ParameterOrderLimits::default(), 2)
                .is_err()
        );
        let o = own_periodic_arrangement_cuts(cover, 1e-12, ParameterOrderLimits::default(), 3)
            .unwrap();
        o.reverify_source().unwrap();
        assert_eq!(o.pairs().len(), 3);
        assert_eq!(o.intervals().len(), 4);
        assert!(o.cuts().iter().all(|c| c.len() == 3));
        for side in 0..2 {
            assert!(o.cuts()[side][1].origins.contains(
                &OriginalPeriodicArrangementOrigin::Event {
                    pair: 1,
                    event: 0,
                    side
                }
            ));
        }
        let mut bad = o.clone();
        bad.pairs.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = o.clone();
        bad.cuts[0][1].origins.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = o;
        bad.intervals[0].cell = 99;
        assert!(bad.reverify_source().is_err());
    }
    #[test]
    fn reversed_common_uses_remain_source_owned_without_duplicate_cut_values() {
        let a = Curve::from_polyline(vec![vec![0., 0.5], vec![1., 0.5]]).unwrap();
        let b = Curve::from_polyline(vec![vec![1., 0.5], vec![0., 0.5]]).unwrap();
        let o =
            own_periodic_arrangement_cuts(pieces(a, b), 1e-12, ParameterOrderLimits::default(), 3)
                .unwrap();
        o.reverify_source().unwrap();
        assert_eq!(o.intervals().len(), 2);
        assert!(o.cuts().iter().all(|c| c.len() == 2));
        let first = o.intervals().iter().find(|i| i.source == 0).unwrap();
        let second = o.intervals().iter().find(|i| i.source == 1).unwrap();
        assert!(
            first
                .common_uses
                .iter()
                .any(|u| u.pair == 1 && u.side == 0 && !u.reversed)
        );
        assert!(
            second
                .common_uses
                .iter()
                .any(|u| u.pair == 1 && u.side == 1 && u.reversed)
        );
        assert!(o.cuts()[0][0].origins.len() > 1);
    }
}

#[cfg(test)]
pub(super) fn corrupt_for_replay(
    owner: &Arc<OwnedPeriodicArrangementCuts>,
) -> Arc<OwnedPeriodicArrangementCuts> {
    let mut bad = owner.as_ref().clone();
    bad.intervals.clear();
    Arc::new(bad)
}
