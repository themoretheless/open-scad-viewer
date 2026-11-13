//! Exact closed authored wire chains on an admitted periodic source surface.
use super::super::super::super::algebraic_intersections::decode_ratio;
use super::*;
use num_bigint::BigInt;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicSourceUse {
    pub source: usize,
    pub reversed: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicWireUse {
    pub piece: usize,
    pub reversed: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalPeriodicJoinKind {
    SourceTransition { transition: usize, reversed: bool },
    SourceJunction,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicWireJoin {
    pub uses: [usize; 2],
    pub kind: OriginalPeriodicJoinKind,
    /// Canonical UV after-before in whole periods.
    pub chart_delta: [String; 2],
    /// Original (untranslated) source start-after minus end-before in periods.
    pub native_period_delta: [String; 2],
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicWire {
    pub uses: Vec<OriginalPeriodicWireUse>,
    pub joins: Vec<OriginalPeriodicWireJoin>,
    /// Oriented period count; None means an unwrapped axis.
    pub homology: [Option<String>; 2],
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalPeriodicWires {
    surface: Arc<OwnedOriginalPeriodicSurface>,
    sources: Vec<Vec<OriginalPeriodicSourceUse>>,
    wires: Vec<OriginalPeriodicWire>,
}
fn rational_point(p: &OriginalPeriodicPoint) -> Result<[Q; 2]> {
    let mut out = [Q::zero(), Q::zero()];
    for axis in 0..2 {
        let n = &p.coordinates[axis][0];
        let d = &p.coordinates[axis][1];
        if n.len() > 1 || d.len() != 1 {
            return Err(refused(
                "Periodic source junction is not an original rational domain endpoint",
            ));
        }
        let den = decode_ratio(&d[0])?;
        if den.is_zero() {
            return Err(refused("Periodic source junction denominator vanishes"));
        }
        out[axis] = if n.is_empty() {
            Q::zero()
        } else {
            decode_ratio(&n[0])?
        } / den;
    }
    Ok(out)
}
const MAX_WIRES: usize = 32;
const MAX_WIRE_USES: usize = 16384;
/// Kind of one join with its chart and native period deltas.
type JoinDeltas = (OriginalPeriodicJoinKind, [BigInt; 2], [BigInt; 2]);
fn zero_delta() -> [BigInt; 2] {
    [BigInt::from(0), BigInt::from(0)]
}
/// Every original source curve must be used by exactly one wire.
fn require_complete_grouping(
    sources: &[Vec<OriginalPeriodicSourceUse>],
    count: usize,
) -> Result<()> {
    if sources.len() > MAX_WIRES || sources.iter().any(Vec::is_empty) {
        return Err(refused(
            "Periodic wire source grouping must contain nonempty bounded wires",
        ));
    }
    let mut used = vec![false; count];
    for use_ in sources.iter().flatten() {
        if use_.source >= count || used[use_.source] {
            return Err(refused(
                "Periodic wire grouping omits, repeats or rebinds a source curve",
            ));
        }
        used[use_.source] = true;
    }
    if used.iter().any(|v| !v) {
        return Err(refused(
            "Periodic wire grouping omits an original source curve",
        ));
    }
    Ok(())
}
/// The owned periodic pieces indexed for chaining them into closed wires.
struct Chain<'a> {
    owner: &'a OwnedOriginalPeriodicPieces,
    periods: [Q; 2],
    wrapped: [bool; 2],
    /// Pieces of every source in natural parameter order.
    by_source: Vec<Vec<usize>>,
    /// Owned transition between two consecutive pieces of one source.
    transition_index: std::collections::BTreeMap<[usize; 2], usize>,
}
impl<'a> Chain<'a> {
    fn new(owner: &'a OwnedOriginalPeriodicPieces) -> Result<Self> {
        let mut by_source = vec![Vec::new(); owner.sources().len()];
        for (piece, p) in owner.pieces().iter().enumerate() {
            by_source[p.source].push(piece);
        }
        let mut periods = [Q::zero(), Q::zero()];
        for (period, bounds) in periods.iter_mut().zip(owner.chart()) {
            let (Some(low), Some(high)) = (Q::from_float(bounds[0]), Q::from_float(bounds[1]))
            else {
                return Err(refused("Periodic wire chart bounds are not finite"));
            };
            *period = high - low;
        }
        let mut transition_index = std::collections::BTreeMap::new();
        for (index, t) in owner.transitions().iter().enumerate() {
            if transition_index.insert(t.pieces, index).is_some() {
                return Err(refused(
                    "Original periodic transition ownership is duplicated",
                ));
            }
        }
        Ok(Self {
            owner,
            periods,
            wrapped: owner.wrapped(),
            by_source,
            transition_index,
        })
    }
    /// Piece uses of one wire: each source's pieces in traversal order.
    fn uses(&self, wire: &[OriginalPeriodicSourceUse]) -> Result<Vec<OriginalPeriodicWireUse>> {
        let mut uses = Vec::new();
        for source in wire {
            let pieces = &self.by_source[source.source];
            if pieces.is_empty() {
                return Err(refused("Periodic wire source has no owned original pieces"));
            }
            let ordered: Vec<usize> = if source.reversed {
                pieces.iter().rev().copied().collect()
            } else {
                pieces.clone()
            };
            uses.extend(ordered.into_iter().map(|piece| OriginalPeriodicWireUse {
                piece,
                reversed: source.reversed,
            }));
        }
        if uses.len() > MAX_WIRE_USES {
            return Err(refused("Periodic wire coedge allowance exceeded"));
        }
        Ok(uses)
    }
    /// The owned transition two consecutive uses cross, if they are adjacent
    /// pieces of one source traversed in the same direction.
    fn transition(&self, a: OriginalPeriodicWireUse, b: OriginalPeriodicWireUse) -> Option<usize> {
        let key = match (a.reversed, b.reversed) {
            (false, false) => [a.piece, b.piece],
            (true, true) => [b.piece, a.piece],
            _ => return None,
        };
        self.transition_index.get(&key).copied()
    }
    /// An internal native cut is already proved in its actual root
    /// extension. Its original endpoint is continuous; only its
    /// canonical chart representative changes by whole periods.
    fn transition_join(&self, transition: usize, reversed: bool) -> Result<JoinDeltas> {
        let t = &self.owner.transitions()[transition];
        let mut delta = zero_delta();
        for axis in 0..2 {
            let d = t.period_delta[axis]
                .parse::<BigInt>()
                .map_err(|_| refused("Invalid periodic transition integer"))?;
            delta[axis] = if reversed { d } else { -d };
        }
        let kind = OriginalPeriodicJoinKind::SourceTransition {
            transition,
            reversed,
        };
        Ok((kind, delta, zero_delta()))
    }
    /// Different original curves (or source closure) meet at their
    /// rational original natural-domain endpoints, not a midpoint.
    fn junction_join(
        &self,
        a: OriginalPeriodicWireUse,
        b: OriginalPeriodicWireUse,
    ) -> Result<JoinDeltas> {
        let owner = self.owner;
        let end = owner.endpoint_verified(a.piece, if a.reversed { 0 } else { 1 })?;
        let start = owner.endpoint_verified(b.piece, if b.reversed { 1 } else { 0 })?;
        let end = rational_point(&end)?;
        let start = rational_point(&start)?;
        let pa = &owner.pieces()[a.piece];
        let pb = &owner.pieces()[b.piece];
        let mut delta = zero_delta();
        let mut native = zero_delta();
        for axis in 0..2 {
            let difference = &start[axis] - &end[axis];
            if !self.wrapped[axis] {
                if !difference.is_zero() {
                    return Err(refused(
                        "Periodic wire source endpoints disagree on an unwrapped coordinate",
                    ));
                }
                continue;
            }
            let d = &difference / &self.periods[axis];
            if !d.is_integer() {
                return Err(refused(
                    "Periodic wire source endpoints do not meet modulo the actual chart period",
                ));
            }
            delta[axis] = d.to_integer();
            if delta[axis] < BigInt::from(-1) || delta[axis] > BigInt::from(1) {
                return Err(refused(
                    "Periodic wire source junction leaves its admitted closed chart",
                ));
            }
            let original_difference = difference - decode_ratio(&pb.translation[axis])?
                + decode_ratio(&pa.translation[axis])?;
            let d = original_difference / &self.periods[axis];
            if !d.is_integer() {
                return Err(refused(
                    "Periodic native wire source lift has a fractional period jump",
                ));
            }
            native[axis] = d.to_integer();
        }
        Ok((OriginalPeriodicJoinKind::SourceJunction, delta, native))
    }
    /// One closed wire: its uses, the join after each use, and its homology
    /// as the total native period count on each wrapped axis.
    fn wire(&self, wire: &[OriginalPeriodicSourceUse]) -> Result<OriginalPeriodicWire> {
        let uses = self.uses(wire)?;
        let mut joins = Vec::new();
        let mut homology = zero_delta();
        for i in 0..uses.len() {
            let next = (i + 1) % uses.len();
            let (a, b) = (uses[i], uses[next]);
            let (kind, chart_delta, native_period_delta) = match self.transition(a, b) {
                Some(transition) => self.transition_join(transition, a.reversed)?,
                None => self.junction_join(a, b)?,
            };
            for axis in 0..2 {
                homology[axis] -= &native_period_delta[axis];
            }
            joins.push(OriginalPeriodicWireJoin {
                uses: [i, next],
                kind,
                chart_delta: chart_delta.map(|n| n.to_string()),
                native_period_delta: native_period_delta.map(|n| n.to_string()),
            });
        }
        Ok(OriginalPeriodicWire {
            uses,
            joins,
            homology: std::array::from_fn(|axis| {
                self.wrapped[axis].then(|| homology[axis].to_string())
            }),
        })
    }
}
fn derive(
    surface: &Arc<OwnedOriginalPeriodicSurface>,
    sources: &[Vec<OriginalPeriodicSourceUse>],
) -> Result<Vec<OriginalPeriodicWire>> {
    crate::profile_certificate::span_intersections::translated::periodic_replay::verify_arc(
        surface,
        |x| x.reverify_source(),
    )?;
    let owner = surface.pieces();
    require_complete_grouping(sources, owner.sources().len())?;
    let chain = Chain::new(owner)?;
    sources.iter().map(|wire| chain.wire(wire)).collect()
}
impl OwnedOriginalPeriodicWires {
    pub fn surface(&self) -> &Arc<OwnedOriginalPeriodicSurface> {
        &self.surface
    }
    pub fn source_uses(&self) -> &[Vec<OriginalPeriodicSourceUse>] {
        &self.sources
    }
    pub fn wires(&self) -> &[OriginalPeriodicWire] {
        &self.wires
    }
    pub fn reverify_source(&self) -> Result<()> {
        let _replay =
            crate::profile_certificate::span_intersections::translated::periodic_replay::scope();
        if derive(&self.surface, &self.sources)? != self.wires {
            return Err(refused(
                "Original periodic wire closure orientation or homology changed",
            ));
        }
        Ok(())
    }
}
/// Complete source coverage and exact C0 wire closure on the actual periodic
/// surface. Self-intersections, region separation and manifold faces need
/// independent common-contact, embedding and material-region admission.
pub fn own_original_periodic_wires(
    surface: Arc<OwnedOriginalPeriodicSurface>,
    sources: &[Vec<OriginalPeriodicSourceUse>],
) -> Result<OwnedOriginalPeriodicWires> {
    let _replay =
        crate::profile_certificate::span_intersections::translated::periodic_replay::scope();
    let wires = derive(&surface, sources)?;
    Ok(OwnedOriginalPeriodicWires {
        surface,
        sources: sources.to_vec(),
        wires,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_rejects_join_omission_period_count_and_wire_use_orientation() {
        let c = Curve::from_polyline(vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![1., 1., 0.],
            vec![0., 1., 0.],
            vec![0., 0., 0.],
        ])
        .unwrap();
        let surface = nurbs_core::surface::extrude(&c, [0., 0., 1.]).unwrap();
        let a = Curve::from_polyline(vec![vec![0.5, 0.5], vec![4.5, 0.5]]).unwrap();
        let pieces = Arc::new(
            own_original_periodic_pieces(
                &[a],
                [[0., 4.], [0., 1.]],
                [true, false],
                1e-12,
                ParameterOrderLimits::default(),
                32,
            )
            .unwrap(),
        );
        let surface = Arc::new(
            bind_original_periodic_pieces_to_surface(
                pieces,
                &surface,
                ParameterOrderLimits::default(),
            )
            .unwrap(),
        );
        let x = own_original_periodic_wires(
            surface,
            &[vec![OriginalPeriodicSourceUse {
                source: 0,
                reversed: false,
            }]],
        )
        .unwrap();
        x.reverify_source().unwrap();
        let mut bad = x.clone();
        bad.wires[0].joins.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.wires[0].homology[0] = Some("0".into());
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.wires[0].uses[0].reversed = true;
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.wires[0].joins[0].chart_delta[0] = "3".into();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.sources[0].clear();
        assert!(bad.reverify_source().is_err());
    }
}
