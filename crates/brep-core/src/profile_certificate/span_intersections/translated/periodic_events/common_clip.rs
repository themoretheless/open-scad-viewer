//! Complete closed-piece intersection of an admitted monotone common interval.
use super::*;
use crate::profile_certificate::algebraic_intersections::polynomial;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicCommonClip {
    pub pair: usize,
    /// Increasing ORIGINAL first-source parameter, not UV image ordering.
    pub endpoints: [AlgebraicEventNode; 2],
    pub point_only: bool,
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicCommonClips {
    owner: Arc<OwnedPeriodicPairEvents>,
    clips: Vec<OriginalPeriodicCommonClip>,
}
fn value(n: &AlgebraicEventNode, side: usize) -> OriginalSourceParameter {
    OriginalSourceParameter {
        primitive: n.exact.primitive.clone(),
        interval: n.exact.isolating_interval.clone(),
        parameter: n.exact.parameters[side].clone(),
    }
}
fn between(
    v: &OriginalSourceParameter,
    a: &OriginalSourceParameter,
    b: &OriginalSourceParameter,
    l: ParameterOrderLimits,
) -> Result<bool> {
    let (lo, hi) = if compare(a, b, l)? == Ordering::Greater {
        (b, a)
    } else {
        (a, b)
    };
    Ok(compare(v, lo, l)? != Ordering::Less && compare(v, hi, l)? != Ordering::Greater)
}
fn lift(
    o: &OwnedPeriodicPairEvents,
    pair: usize,
    side: usize,
    end: usize,
    x: &[Cell],
    y: &[Cell],
) -> Result<Vec<AlgebraicEventNode>> {
    let piece = &o.pieces.pieces()[o.pair[side]];
    lift_value(
        o,
        pair,
        side,
        &o.pieces.cuts()[piece.source][piece.cuts[end]].value,
        x,
        y,
    )
}
pub(in crate::profile_certificate) fn lift_value(
    o: &OwnedPeriodicPairEvents,
    pair: usize,
    side: usize,
    value: &OriginalSourceParameter,
    x: &[Cell],
    y: &[Cell],
) -> Result<Vec<AlgebraicEventNode>> {
    if side > 1 {
        return Err(refused("Invalid common cut source clock"));
    }
    let receipt = o
        .census
        .certificate
        .pairs
        .get(pair)
        .ok_or_else(|| refused("Missing original common cell pair"))?;
    if receipt.kind != "overlap" {
        return Err(refused("Cut lift requires proved common component"));
    }
    let target = if side == 0 {
        &y[receipt.second]
    } else {
        &x[receipt.first]
    };
    let source = if side == 0 {
        &x[receipt.first]
    } else {
        &y[receipt.second]
    };
    let f = decode_polynomial(&value.primitive)?;
    let parameter = decode_polynomial(&value.parameter)?;
    let root = [
        decode_ratio(&value.interval[0])?,
        decode_ratio(&value.interval[1])?,
    ];
    let coordinates = arithmetic::shore_events::translated_point(
        &source.net,
        &source.domain,
        &parameter,
        &f,
        &root,
        &[Q::zero(), Q::zero()],
        o.limits.max_refinements,
    )?;
    // Equality of the ACTUAL homogeneous polynomial expression and original
    // natural clock gives an exact identity correspondence. The common proof
    // already establishes monotonicity/uniqueness; no resultant is required.
    if source.net == target.net && source.domain == target.domain {
        return Ok(vec![AlgebraicEventNode {
            source_cells: [receipt.first, receipt.second],
            pair,
            factor: None,
            root: 0,
            stationary: false,
            exact: AlgebraicEventPosition {
                primitive: polynomial(&f),
                isolating_interval: root.map(|q| ratio(&q)),
                parameters: [polynomial(&parameter), polynomial(&parameter)],
                position: coordinates.map(|c| c.map(|p| polynomial(&p))),
            },
        }]);
    }
    let width = width(source, target, o.census.certificate.maximum_parameter_width);
    let preimages = arithmetic::algebraic_point_pullback::pullback(
        &f,
        &root,
        &coordinates,
        &target.net,
        &width,
    )?;
    let mut out = Vec::new();
    for (index, p) in preimages.into_iter().enumerate() {
        let original =
            arithmetic::event_coordinates::compose(&parameter, &p.source_primitive, &p.primitive)?;
        let target_parameter = arithmetic::event_coordinates::compose(
            &vec![
                target.domain[0].clone(),
                &target.domain[1] - &target.domain[0],
            ],
            &p.local_parameter,
            &p.primitive,
        )?;
        let parameters = if side == 0 {
            [original, target_parameter]
        } else {
            [target_parameter, original]
        };
        out.push(AlgebraicEventNode {
            source_cells: [receipt.first, receipt.second],
            pair,
            factor: None,
            root: index,
            stationary: false,
            exact: AlgebraicEventPosition {
                primitive: polynomial(&p.primitive),
                isolating_interval: p.interval.map(|q| ratio(&q)),
                parameters: parameters.map(|p| polynomial(&p)),
                position: p.position.map(|c| c.map(|p| polynomial(&p))),
            },
        });
    }
    // The caller limits a cut to the proved monotone common image before lift;
    // its counterpart on this actual target cell must be unique.
    if out.len() != 1 {
        return Err(refused(
            "Periodic common cut has no unique actual target-cell preimage",
        ));
    }
    Ok(out)
}
fn derive(o: &Arc<OwnedPeriodicPairEvents>) -> Result<Vec<OriginalPeriodicCommonClip>> {
    super::super::periodic_replay::verify_arc(o, |x| x.reverify_source())?;
    let (_, x, y) = context(
        &o.census.sources[0],
        &o.census.sources[1],
        &o.census.offsets,
        false,
        o.census.certificate.maximum_parameter_width,
    )?;
    let pieces = [&o.pieces.pieces()[o.pair[0]], &o.pieces.pieces()[o.pair[1]]];
    let mut clips = Vec::new();
    for (pair, ends) in &o.common_endpoints {
        let mut candidates = ends
            .iter()
            .filter(|n| n.in_both_closed_pieces())
            .map(|n| n.node.clone())
            .collect::<Vec<_>>();
        for side in 0..2 {
            for end in 0..2 {
                let cut = &o.pieces.cuts()[pieces[side].source][pieces[side].cuts[end]].value;
                if !between(
                    cut,
                    &value(&ends[0].node, side),
                    &value(&ends[1].node, side),
                    o.limits,
                )? {
                    continue;
                }
                for n in lift(o, *pair, side, end, &x, &y)? {
                    if classify(o, n.clone())?.in_both_closed_pieces() {
                        candidates.push(n);
                    }
                }
            }
        }
        let mut ordered: Vec<AlgebraicEventNode> = Vec::new();
        for n in candidates {
            let mut at = ordered.len();
            let mut duplicate = false;
            for (i, previous) in ordered.iter().enumerate() {
                match compare(&value(&n, 0), &value(previous, 0), o.limits)? {
                    Ordering::Equal => {
                        duplicate = true;
                        break;
                    }
                    Ordering::Less => {
                        at = i;
                        break;
                    }
                    Ordering::Greater => {}
                }
            }
            if !duplicate {
                ordered.insert(at, n);
            }
        }
        if ordered.len() > 2 {
            return Err(refused(
                "Monotone common interval has more than two clipped boundary values",
            ));
        }
        if let Some(first) = ordered.first() {
            let last = ordered.last().unwrap();
            clips.push(OriginalPeriodicCommonClip {
                pair: *pair,
                endpoints: [first.clone(), last.clone()],
                point_only: ordered.len() == 1,
            });
        }
    }
    Ok(clips)
}
impl OwnedPeriodicCommonClips {
    pub fn owner(&self) -> &Arc<OwnedPeriodicPairEvents> {
        &self.owner
    }
    pub fn clips(&self) -> &[OriginalPeriodicCommonClip] {
        &self.clips
    }
    pub fn reverify_source(&self) -> Result<()> {
        let _replay = super::super::periodic_replay::scope();
        if derive(&self.owner)? != self.clips {
            return Err(refused("Exact periodic clipped common interval changed"));
        }
        Ok(())
    }
}
pub fn own_periodic_common_clips(
    owner: Arc<OwnedPeriodicPairEvents>,
) -> Result<OwnedPeriodicCommonClips> {
    let _replay = super::super::periodic_replay::scope();
    let clips = derive(&owner)?;
    Ok(OwnedPeriodicCommonClips { owner, clips })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn cover(a: Curve, b: Curve) -> Arc<OwnedOriginalPeriodicPieces> {
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
    fn own(p: Arc<OwnedOriginalPeriodicPieces>, pair: [usize; 2]) -> OwnedPeriodicCommonClips {
        own_periodic_common_clips(Arc::new(
            own_periodic_pair_events(p, pair, 1e-12, ParameterOrderLimits::default()).unwrap(),
        ))
        .unwrap()
    }
    #[test]
    fn native_knot_singleton_survives_adjacent_common_cell_provenance() {
        let a = Curve::from_polyline(vec![vec![0., 0.5], vec![1., 0.5], vec![2., 0.5]]).unwrap();
        let p = Arc::new(
            own_original_periodic_pieces(
                &[a.clone(), a],
                [[0., 4.], [0., 1.]],
                [true, false],
                1e-12,
                ParameterOrderLimits::default(),
                64,
            )
            .unwrap(),
        );
        let second = p.pieces().iter().position(|p| p.source == 1).unwrap();
        let o = own(p, [0, second + 1]);
        o.reverify_source().unwrap();
        assert!(!o.clips().is_empty());
        assert!(o.clips().iter().all(|c| c.point_only));
        for c in o.clips() {
            assert_eq!(
                classify(o.owner(), c.endpoints[0].clone())
                    .unwrap()
                    .locations,
                [
                    OriginalPieceEventLocation::End,
                    OriginalPieceEventLocation::Start
                ]
            );
        }
    }
    #[test]
    fn common_crossing_with_both_old_endpoints_outside_gets_two_new_cuts() {
        let a = Curve::from_polyline(vec![vec![0., 0.5], vec![3., 0.5]]).unwrap();
        let p = cover(a.clone(), a);
        let second = p.pieces().iter().position(|p| p.source == 1).unwrap();
        let o = own(p, [1, second + 1]);
        o.reverify_source().unwrap();
        assert_eq!(o.clips().len(), 1);
        assert!(!o.clips()[0].point_only);
        for end in 0..2 {
            let n = &o.clips()[0].endpoints[end];
            let c = classify(o.owner(), n.clone()).unwrap();
            assert_eq!(
                c.locations,
                [if end == 0 {
                    OriginalPieceEventLocation::Start
                } else {
                    OriginalPieceEventLocation::End
                }; 2]
            );
        }
        let mut bad = o.clone();
        bad.clips[0].endpoints[0].exact.parameters[1].clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = o;
        bad.clips.clear();
        assert!(bad.reverify_source().is_err());
    }
    #[test]
    fn curved_reversed_common_clip_preserves_both_irrational_original_clocks() {
        let a = Curve {
            degree: 2,
            knots: vec![2., 2., 2., 6., 6., 6.],
            control_points: vec![vec![0., 0.25], vec![0., 0.5], vec![3., 0.75]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let mut b = a.clone();
        b.control_points.reverse();
        let p = cover(a, b);
        let second = p.pieces().iter().position(|p| p.source == 1).unwrap();
        let o = own(p, [1, second + 1]);
        o.reverify_source().unwrap();
        assert_eq!(o.clips().len(), 1);
        assert!(!o.clips()[0].point_only);
        let ends = &o.clips()[0].endpoints;
        assert_eq!(
            classify(o.owner(), ends[0].clone()).unwrap().locations,
            [
                OriginalPieceEventLocation::Start,
                OriginalPieceEventLocation::End
            ]
        );
        assert_eq!(
            classify(o.owner(), ends[1].clone()).unwrap().locations,
            [
                OriginalPieceEventLocation::End,
                OriginalPieceEventLocation::Start
            ]
        );
        assert_ne!(
            ends[0].exact.isolating_interval[0],
            ends[0].exact.isolating_interval[1]
        );
        assert_eq!(o.owner().pieces().sources()[0].domain(), [2., 6.]);
    }
}
