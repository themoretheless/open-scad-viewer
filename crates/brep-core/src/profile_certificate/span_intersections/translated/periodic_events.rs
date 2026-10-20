//! Exact finite-event clipping against source-owned periodic piece boundaries.
pub(super) mod common_clip;
use super::*;
use crate::profile_certificate::algebraic_intersections::{decode_polynomial, decode_ratio};
pub use common_clip::{
    OriginalPeriodicCommonClip, OwnedPeriodicCommonClips, own_periodic_common_clips,
};
use std::{cmp::Ordering, sync::Arc};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalPieceEventLocation {
    Before,
    Start,
    Interior,
    End,
    After,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPieceEvent {
    pub node: AlgebraicEventNode,
    pub locations: [OriginalPieceEventLocation; 2],
}
impl OriginalPieceEvent {
    pub fn in_both_closed_pieces(&self) -> bool {
        self.locations.iter().all(|v| {
            matches!(
                v,
                OriginalPieceEventLocation::Start
                    | OriginalPieceEventLocation::Interior
                    | OriginalPieceEventLocation::End
            )
        })
    }
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicPairEvents {
    pieces: Arc<OwnedOriginalPeriodicPieces>,
    pair: [usize; 2],
    second_translation_delta: [ExactRatio; 2],
    census: OwnedTranslatedSpanIntersections,
    limits: ParameterOrderLimits,
    events: Vec<OriginalPieceEvent>,
    common_endpoints: Vec<(usize, [OriginalPieceEvent; 2])>,
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
fn classify(o: &OwnedPeriodicPairEvents, node: AlgebraicEventNode) -> Result<OriginalPieceEvent> {
    let mut locations = [OriginalPieceEventLocation::Interior; 2];
    for side in 0..2 {
        let piece = &o.pieces.pieces()[o.pair[side]];
        let cuts = &o.pieces.cuts()[piece.source];
        let value = OriginalSourceParameter {
            primitive: node.exact.primitive.clone(),
            interval: node.exact.isolating_interval.clone(),
            parameter: node.exact.parameters[side].clone(),
        };
        let lo = compare(&value, &cuts[piece.cuts[0]].value, o.limits)?;
        let hi = compare(&value, &cuts[piece.cuts[1]].value, o.limits)?;
        locations[side] = if lo == Ordering::Less {
            OriginalPieceEventLocation::Before
        } else if lo == Ordering::Equal {
            OriginalPieceEventLocation::Start
        } else if hi == Ordering::Less {
            OriginalPieceEventLocation::Interior
        } else if hi == Ordering::Equal {
            OriginalPieceEventLocation::End
        } else {
            OriginalPieceEventLocation::After
        };
    }
    Ok(OriginalPieceEvent { node, locations })
}
fn offsets(
    a: &OriginalPeriodicPiece,
    b: &OriginalPeriodicPiece,
    delta: &[ExactRatio; 2],
) -> Result<[[ExactRatio; 2]; 2]> {
    Ok([
        a.translation.clone(),
        [
            ratio(&(decode_ratio(&b.translation[0])? + decode_ratio(&delta[0])?)),
            ratio(&(decode_ratio(&b.translation[1])? + decode_ratio(&delta[1])?)),
        ],
    ])
}
fn derive(
    o: &OwnedPeriodicPairEvents,
) -> Result<(
    Vec<OriginalPieceEvent>,
    Vec<(usize, [OriginalPieceEvent; 2])>,
)> {
    if o.limits.max_refinements > 512
        || !(1..=128).contains(&o.limits.max_resultant_size)
        || o.pair.iter().any(|i| *i >= o.pieces.pieces().len())
    {
        return Err(refused(
            "Invalid exact periodic event piece reference or limits",
        ));
    }
    o.pieces.reverify_source()?;
    let a = &o.pieces.pieces()[o.pair[0]];
    let b = &o.pieces.pieces()[o.pair[1]];
    let (key, _, _) = context(
        &o.pieces.sources()[a.source],
        &o.pieces.sources()[b.source],
        &offsets(a, b, &o.second_translation_delta)?,
        false,
        o.census.certificate.maximum_parameter_width,
    )?;
    if o.census.certificate.source != key {
        return Err(refused(
            "Periodic event census does not belong to actual translated pieces",
        ));
    }
    let events = o
        .census
        .exact_nodes()?
        .into_iter()
        .map(|n| classify(o, n))
        .collect::<Result<Vec<_>>>()?;
    let common = o
        .census
        .exact_overlap_endpoints()?
        .into_iter()
        .map(|(pair, [a, b])| Ok((pair, [classify(o, a)?, classify(o, b)?])))
        .collect::<Result<Vec<_>>>()?;
    Ok((events, common))
}
impl OwnedPeriodicPairEvents {
    pub fn pieces(&self) -> &Arc<OwnedOriginalPeriodicPieces> {
        &self.pieces
    }
    pub fn pair(&self) -> [usize; 2] {
        self.pair
    }
    pub fn second_translation_delta(&self) -> &[ExactRatio; 2] {
        &self.second_translation_delta
    }
    pub fn census(&self) -> &OwnedTranslatedSpanIntersections {
        &self.census
    }
    /// Complete finite source event census, retaining outside classifications.
    pub fn events(&self) -> &[OriginalPieceEvent] {
        &self.events
    }
    /// Membership of ORIGINAL common endpoints only. A common interval may cross
    /// both pieces even if neither original endpoint lies in both: this does
    /// not substitute for clipped common-interval construction.
    pub fn common_endpoints(&self) -> &[(usize, [OriginalPieceEvent; 2])] {
        &self.common_endpoints
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (a, b) = derive(self)?;
        if a != self.events || b != self.common_endpoints {
            return Err(refused(
                "Periodic event membership or complete source census changed",
            ));
        }
        Ok(())
    }
}
pub fn own_periodic_pair_events(
    pieces: Arc<OwnedOriginalPeriodicPieces>,
    pair: [usize; 2],
    width: f64,
    limits: ParameterOrderLimits,
) -> Result<OwnedPeriodicPairEvents> {
    own_periodic_pair_events_translated(
        pieces,
        pair,
        width,
        limits,
        &[ratio(&Q::zero()), ratio(&Q::zero())],
    )
}
/// Abstract translated UV view only; a nonzero shift proves no physical seam
/// unless separately bound to an actual qualified surface/periodic cover.
pub fn own_periodic_pair_events_translated(
    pieces: Arc<OwnedOriginalPeriodicPieces>,
    pair: [usize; 2],
    width: f64,
    limits: ParameterOrderLimits,
    delta: &[ExactRatio; 2],
) -> Result<OwnedPeriodicPairEvents> {
    if pair.iter().any(|i| *i >= pieces.pieces().len()) {
        return Err(refused(
            "Periodic event piece index outside actual source cover",
        ));
    }
    pieces.reverify_source()?;
    let a = &pieces.pieces()[pair[0]];
    let b = &pieces.pieces()[pair[1]];
    let census = own_translated_span_intersections(
        &pieces.sources()[a.source],
        &pieces.sources()[b.source],
        &offsets(a, b, delta)?,
        false,
        width,
    )?;
    let mut owner = OwnedPeriodicPairEvents {
        pieces,
        pair,
        second_translation_delta: delta.clone(),
        census,
        limits,
        events: Vec::new(),
        common_endpoints: Vec::new(),
    };
    let (events, common_endpoints) = derive(&owner)?;
    owner.events = events;
    owner.common_endpoints = common_endpoints;
    Ok(owner)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn pieces(a: Curve, b: Curve) -> Arc<OwnedOriginalPeriodicPieces> {
        Arc::new(
            own_original_periodic_pieces(
                &[a, b],
                [[0., 1.], [0., 1.]],
                [true, false],
                1e-12,
                ParameterOrderLimits::default(),
                64,
            )
            .unwrap(),
        )
    }
    #[test]
    fn crossing_common_interval_is_preserved_when_original_endpoints_are_outside() {
        let a = Curve::from_polyline(vec![vec![0., 0.5], vec![3., 0.5]]).unwrap();
        let cover = pieces(a.clone(), a);
        let second = cover.pieces().iter().position(|p| p.source == 1).unwrap();
        let owner = own_periodic_pair_events(
            cover,
            [1, second + 1],
            1e-12,
            ParameterOrderLimits::default(),
        )
        .unwrap();
        owner.reverify_source().unwrap();
        assert!(owner.events().is_empty());
        assert_eq!(owner.common_endpoints().len(), 1);
        let ends = &owner.common_endpoints()[0].1;
        assert_eq!(ends[0].locations, [OriginalPieceEventLocation::Before; 2]);
        assert_eq!(ends[1].locations, [OriginalPieceEventLocation::After; 2]);
        assert!(ends.iter().all(|p| !p.in_both_closed_pieces()));
        assert_eq!(
            owner.census().reverify_source().unwrap().overlaps().len(),
            1
        );
    }
    #[test]
    fn finite_piece_census_retains_outside_traversals_and_rejects_cache_damage() {
        let a = Curve::from_polyline(vec![vec![0., 0.5], vec![2., 0.5], vec![0., 0.5]]).unwrap();
        let b = Curve::from_polyline(vec![vec![0.5, 0.], vec![0.5, 1.]]).unwrap();
        let cover = pieces(a, b);
        let b = cover.pieces().iter().position(|p| p.source == 1).unwrap();
        let owner = own_periodic_pair_events(cover, [0, b], 1e-12, ParameterOrderLimits::default())
            .unwrap();
        owner.reverify_source().unwrap();
        assert_eq!(owner.events().len(), 2);
        assert_eq!(
            owner
                .events()
                .iter()
                .filter(|n| n.in_both_closed_pieces())
                .count(),
            1
        );
        assert!(
            owner
                .events()
                .iter()
                .any(|n| n.locations[0] == OriginalPieceEventLocation::After)
        );
        let mut bad = owner.clone();
        bad.events.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.events[0].locations[0] = OriginalPieceEventLocation::Before;
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.pair[0] = 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = owner;
        bad.limits.max_refinements = 513;
        assert!(bad.reverify_source().is_err());
    }
    #[test]
    fn irrational_shore_event_is_exact_start_of_its_actual_piece() {
        let a = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.5], vec![0., 0.5], vec![2., 0.5]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let b = Curve::from_polyline(vec![vec![0., 0.], vec![0., 1.]]).unwrap();
        let cover = pieces(a, b);
        let b = cover.pieces().iter().position(|p| p.source == 1).unwrap();
        let owner = own_periodic_pair_events(cover, [1, b], 1e-12, ParameterOrderLimits::default())
            .unwrap();
        owner.reverify_source().unwrap();
        assert_eq!(owner.events().len(), 1);
        assert_eq!(
            owner.events()[0].locations,
            [
                OriginalPieceEventLocation::Start,
                OriginalPieceEventLocation::Interior
            ]
        );
        assert!(owner.events()[0].in_both_closed_pieces());
        assert_ne!(
            owner.events()[0].node.exact.isolating_interval[0],
            owner.events()[0].node.exact.isolating_interval[1]
        );
    }
}
