//! Complete opposite-shore contact census on an actually qualified support.
use super::*;
use crate::profile_certificate::algebraic_intersections::{
    decode_polynomial, decode_ratio, polynomial,
};
use std::sync::Arc;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicSeamNode {
    pub node: AlgebraicEventNode,
    /// For each shifted axis: actual first/second chart side, 0=lower,1=upper.
    pub shores: [Option<[usize; 2]>; 2],
    pub actual_positions: [[[ExactPolynomial; 2]; 2]; 2],
}
#[derive(Clone, Debug)]
pub struct OriginalPeriodicSeamPair {
    pub pieces: [usize; 2],
    pub shift: [i8; 2],
    pub owner: Arc<OwnedPeriodicCommonClips>,
    pub finite: Vec<(usize, OriginalPeriodicSeamNode)>,
    pub common: Vec<(usize, [OriginalPeriodicSeamNode; 2])>,
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicSeamContacts {
    embedding: Arc<OwnedOriginalPeriodicEmbedding>,
    width: f64,
    limits: ParameterOrderLimits,
    max_pairs: usize,
    pairs: Vec<OriginalPeriodicSeamPair>,
}
fn shifts(wrapped: [bool; 2]) -> Vec<[i8; 2]> {
    let mut out = Vec::new();
    for a in -1..=1 {
        for b in -1..=1 {
            if (a != 0 || b != 0) && (wrapped[0] || a == 0) && (wrapped[1] || b == 0) {
                out.push([a, b]);
            }
        }
    }
    out
}
fn count(n: usize, k: usize, max: usize) -> Result<usize> {
    let total = n
        .checked_add(1)
        .and_then(|v| n.checked_mul(v))
        .map(|v| v / 2)
        .and_then(|v| v.checked_mul(k))
        .ok_or_else(|| refused("Periodic seam pair count overflow"))?;
    if max == 0 || max > 4096 || total > max {
        return Err(refused(
            "Complete physical seam contact census exceeds pair allowance",
        ));
    }
    Ok(total)
}
fn delta(chart: [[f64; 2]; 2], shift: [i8; 2]) -> [ExactRatio; 2] {
    std::array::from_fn(|axis| {
        ratio(
            &((Q::from_float(chart[axis][1]).unwrap() - Q::from_float(chart[axis][0]).unwrap())
                * Q::from_integer(shift[axis].into())),
        )
    })
}
fn qualify(
    o: &OwnedPeriodicSeamContacts,
    pieces: [usize; 2],
    shift: [i8; 2],
    node: &AlgebraicEventNode,
) -> Result<OriginalPeriodicSeamNode> {
    let surface = o.embedding.wires().surface();
    let cover = surface.pieces();
    let f = decode_polynomial(&node.exact.primitive)?;
    let root = node
        .exact
        .isolating_interval
        .clone()
        .map(|r| decode_ratio(&r))
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let root: [Q; 2] = root
        .try_into()
        .map_err(|_| refused("Incomplete seam primitive root"))?;
    let mut actual = Vec::new();
    for side in 0..2 {
        let p = &cover.pieces()[pieces[side]];
        let original = cells(&cover.sources()[p.source])?;
        let cell = &original[p.cell];
        let parameter = decode_polynomial(&node.exact.parameters[side])?;
        actual.push(arithmetic::shore_events::translated_point(
            &cell.net,
            &cell.domain,
            &parameter,
            &f,
            &root,
            &[
                decode_ratio(&p.translation[0])?,
                decode_ratio(&p.translation[1])?,
            ],
            o.limits.max_refinements,
        )?);
    }
    let actual: [[[Vec<Q>; 2]; 2]; 2] = actual
        .try_into()
        .map_err(|_| refused("Incomplete original seam source positions"))?;
    let mut shores = [None; 2];
    let chart = cover.chart();
    let d = delta(chart, shift);
    for axis in 0..2 {
        let n = decode_polynomial(&node.exact.position[axis][0])?;
        let w = decode_polynomial(&node.exact.position[axis][1])?;
        if !arithmetic::event_coordinates::translated_equal(
            &n,
            &w,
            &actual[0][axis][0],
            &actual[0][axis][1],
            &Q::zero(),
            &f,
            &root,
        )? || !arithmetic::event_coordinates::translated_equal(
            &actual[0][axis][0],
            &actual[0][axis][1],
            &actual[1][axis][0],
            &actual[1][axis][1],
            &(-decode_ratio(&d[axis])?),
            &f,
            &root,
        )? {
            return Err(refused(
                "Physical seam node differs from actual ORIGINAL source views",
            ));
        }
        if shift[axis] != 0 {
            if !cover.wrapped()[axis] || surface.seam_proofs()[axis].is_none() {
                return Err(refused(
                    "Seam contact names an unqualified actual surface period",
                ));
            }
            let sides = if shift[axis] > 0 { [1, 0] } else { [0, 1] };
            for side in 0..2 {
                let shore = Q::from_float(chart[axis][sides[side]]).unwrap();
                if !arithmetic::event_coordinates::translated_equal(
                    &Vec::new(),
                    &vec![Q::one()],
                    &actual[side][axis][0],
                    &actual[side][axis][1],
                    &shore,
                    &f,
                    &root,
                )? {
                    return Err(refused(
                        "Period-separated contact is not on both actual opposite shores",
                    ));
                }
            }
            shores[axis] = Some(sides);
        }
    }
    Ok(OriginalPeriodicSeamNode {
        node: node.clone(),
        shores,
        actual_positions: actual.map(|point| point.map(|c| c.map(|p| polynomial(&p)))),
    })
}
fn qualified_pair(
    o: &OwnedPeriodicSeamContacts,
    pieces: [usize; 2],
    shift: [i8; 2],
    owner: Arc<OwnedPeriodicCommonClips>,
) -> Result<OriginalPeriodicSeamPair> {
    let cover = o.embedding.wires().surface().pieces();
    if owner.owner().pair() != pieces
        || !Arc::ptr_eq(owner.owner().pieces(), cover)
        || owner.owner().second_translation_delta() != &delta(cover.chart(), shift)
    {
        return Err(refused(
            "Seam contact proof belongs to unrelated source pieces or period",
        ));
    }
    owner.reverify_source()?;
    let mut finite = Vec::new();
    for (index, event) in owner.owner().events().iter().enumerate() {
        if event.in_both_closed_pieces() {
            finite.push((index, qualify(o, pieces, shift, &event.node)?));
        }
    }
    let mut common = Vec::new();
    for (clip, c) in owner.clips().iter().enumerate() {
        if !c.point_only {
            for axis in 0..2 {
                if shift[axis] == 0 {
                    continue;
                }
                let sides = if shift[axis] > 0 { [1, 0] } else { [0, 1] };
                for side in 0..2 {
                    let p = &cover.pieces()[pieces[side]];
                    let source = cells(&cover.sources()[p.source])?;
                    let cell = &source[p.cell];
                    let offset = decode_ratio(&p.translation[axis])?;
                    let shore = Q::from_float(cover.chart()[axis][sides[side]]).unwrap();
                    if cell
                        .net
                        .iter()
                        .any(|p| &p[axis] + &offset * &p[2] != &shore * &p[2])
                    {
                        return Err(refused(
                            "Positive seam common interval lacks actual source shore identity",
                        ));
                    }
                }
            }
        }
        common.push((
            clip,
            [
                qualify(o, pieces, shift, &c.endpoints[0])?,
                qualify(o, pieces, shift, &c.endpoints[1])?,
            ],
        ));
    }
    Ok(OriginalPeriodicSeamPair {
        pieces,
        shift,
        owner,
        finite,
        common,
    })
}
fn validate(o: &OwnedPeriodicSeamContacts) -> Result<()> {
    if !o.width.is_finite()
        || !(1e-18..=1e-3).contains(&o.width)
        || o.limits.max_refinements > 512
        || !(1..=128).contains(&o.limits.max_resultant_size)
    {
        return Err(refused("Invalid physical seam census allowances"));
    }
    let cover = o.embedding.wires().surface().pieces();
    let s = shifts(cover.wrapped());
    let expected = count(cover.pieces().len(), s.len(), o.max_pairs)?;
    if o.pairs.len() != expected {
        return Err(refused(
            "Physical seam census omits source pair/period obligations",
        ));
    }
    o.embedding.reverify_source()?;
    let mut index = 0;
    for a in 0..cover.pieces().len() {
        for b in a..cover.pieces().len() {
            for shift in &s {
                let pair = &o.pairs[index];
                if pair.pieces != [a, b]
                    || pair.shift != *shift
                    || pair
                        .owner
                        .owner()
                        .census()
                        .certificate()
                        .maximum_parameter_width
                        .to_bits()
                        != o.width.to_bits()
                {
                    return Err(refused(
                        "Physical seam source pairs or period shifts reordered/changed",
                    ));
                }
                let fresh = qualified_pair(o, [a, b], *shift, pair.owner.clone())?;
                if pair.finite != fresh.finite || pair.common != fresh.common {
                    return Err(refused(
                        "Actual seam node, shore or source position proof changed",
                    ));
                }
                index += 1;
            }
        }
    }
    Ok(())
}
impl OwnedPeriodicSeamContacts {
    pub fn embedding(&self) -> &Arc<OwnedOriginalPeriodicEmbedding> {
        &self.embedding
    }
    pub fn pairs(&self) -> &[OriginalPeriodicSeamPair] {
        &self.pairs
    }
    pub fn reverify_source(&self) -> Result<()> {
        validate(self)
    }
}
/// Complete nonzero adjacent-period UV contacts, bound to actual closed-chart
/// coverage, functional surface seam identities and global support embedding.
/// The zero-shift chart graph, seam cut propagation and quotient assembly are
/// retained as separate obligations; no physical faces are published here.
pub fn own_periodic_seam_contacts(
    embedding: Arc<OwnedOriginalPeriodicEmbedding>,
    width: f64,
    limits: ParameterOrderLimits,
    max_pairs: usize,
) -> Result<OwnedPeriodicSeamContacts> {
    let cover = embedding.wires().surface().pieces();
    let shifts = shifts(cover.wrapped());
    count(cover.pieces().len(), shifts.len(), max_pairs)?;
    embedding.reverify_source()?;
    let mut o = OwnedPeriodicSeamContacts {
        embedding: embedding.clone(),
        width,
        limits,
        max_pairs,
        pairs: Vec::new(),
    };
    for a in 0..cover.pieces().len() {
        for b in a..cover.pieces().len() {
            for shift in &shifts {
                let pair = Arc::new(own_periodic_common_clips(Arc::new(
                    own_periodic_pair_events_translated(
                        cover.clone(),
                        [a, b],
                        width,
                        limits,
                        &delta(cover.chart(), *shift),
                    )?,
                ))?);
                o.pairs.push(qualified_pair(&o, [a, b], *shift, pair)?);
            }
        }
    }
    validate(&o)?;
    Ok(o)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn embedded_loop() -> Arc<OwnedOriginalPeriodicEmbedding> {
        let rim = Curve::from_polyline(vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![1., 1., 0.],
            vec![0., 1., 0.],
            vec![0., 0., 0.],
        ])
        .unwrap();
        let surface = nurbs_core::surface::extrude(&rim, [0., 0., 1.]).unwrap();
        let source = Curve {
            degree: 2,
            knots: vec![2., 2., 2., 6., 6., 6.],
            control_points: vec![vec![0., 0.5], vec![0., 0.5], vec![4., 0.5]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let pieces = Arc::new(
            own_original_periodic_pieces(
                &[source],
                [[0., 4.], [0., 1.]],
                [true, false],
                1e-12,
                ParameterOrderLimits::default(),
                32,
            )
            .unwrap(),
        );
        let support = Arc::new(
            bind_original_periodic_pieces_to_surface(
                pieces,
                &surface,
                ParameterOrderLimits::default(),
            )
            .unwrap(),
        );
        let wires = Arc::new(
            own_original_periodic_wires(
                support,
                &[vec![OriginalPeriodicSourceUse {
                    source: 0,
                    reversed: false,
                }]],
            )
            .unwrap(),
        );
        Arc::new(
            own_original_periodic_embedding(
                wires,
                4096,
                OriginalInjectiveAtlasLimits::default(),
                OriginalAtlasPairLimits::default(),
            )
            .unwrap(),
        )
    }
    #[test]
    fn exact_actual_surface_seam_closes_one_original_nonlinear_source_loop() {
        let embedding = embedded_loop();
        assert!(
            own_periodic_seam_contacts(
                embedding.clone(),
                1e-12,
                ParameterOrderLimits::default(),
                1
            )
            .is_err()
        );
        let o = own_periodic_seam_contacts(embedding, 1e-12, ParameterOrderLimits::default(), 2)
            .unwrap();
        o.reverify_source().unwrap();
        assert_eq!(o.pairs().len(), 2);
        assert_eq!(o.pairs()[0].shift, [-1, 0]);
        assert_eq!(o.pairs()[1].shift, [1, 0]);
        assert_eq!(o.pairs().iter().map(|p| p.finite.len()).sum::<usize>(), 2);
        assert_eq!(o.pairs()[0].finite[0].1.shores, [Some([0, 1]), None]);
        assert_eq!(o.pairs()[1].finite[0].1.shores, [Some([1, 0]), None]);
        let first = &o.pairs()[0].finite[0].1.node.exact;
        assert_eq!(
            first.parameters[0],
            vec![["2".to_string(), "1".to_string()]]
        );
        assert_eq!(
            first.parameters[1],
            vec![["6".to_string(), "1".to_string()]]
        );
        let mut bad = o.clone();
        bad.pairs.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = o.clone();
        bad.pairs[0].shift = [0, 1];
        assert!(bad.reverify_source().is_err());
        let mut bad = o.clone();
        bad.pairs[0].finite[0].1.actual_positions[1][0][0].clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = o;
        bad.pairs[0].finite[0].1.shores[0] = None;
        assert!(bad.reverify_source().is_err());
    }
    #[test]
    fn all_nonzero_adjacent_period_combinations_are_required_and_unwrapped_axes_excluded() {
        assert!(shifts([false, false]).is_empty());
        assert_eq!(shifts([true, false]), vec![[-1, 0], [1, 0]]);
        let both = shifts([true, true]);
        assert_eq!(both.len(), 8);
        assert!(both.contains(&[-1, -1]));
        assert!(both.contains(&[1, 1]));
        assert!(!both.contains(&[0, 0]));
        assert_eq!(count(3, both.len(), 48).unwrap(), 48);
        assert!(count(3, both.len(), 47).is_err());
    }
}
