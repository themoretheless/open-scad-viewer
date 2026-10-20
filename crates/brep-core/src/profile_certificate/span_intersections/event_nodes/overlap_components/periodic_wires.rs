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
fn derive(
    surface: &OwnedOriginalPeriodicSurface,
    sources: &[Vec<OriginalPeriodicSourceUse>],
) -> Result<Vec<OriginalPeriodicWire>> {
    surface.reverify_source()?;
    let owner = surface.pieces();
    if sources.len() > 32 || sources.iter().any(Vec::is_empty) {
        return Err(refused(
            "Periodic wire source grouping must contain nonempty bounded wires",
        ));
    }
    let mut used = vec![false; owner.sources().len()];
    let mut by_source = vec![Vec::new(); used.len()];
    for (piece, p) in owner.pieces().iter().enumerate() {
        by_source[p.source].push(piece);
    }
    for wire in sources {
        for use_ in wire {
            if use_.source >= used.len() || used[use_.source] {
                return Err(refused(
                    "Periodic wire grouping omits, repeats or rebinds a source curve",
                ));
            }
            used[use_.source] = true;
        }
    }
    if used.iter().any(|v| !v) {
        return Err(refused(
            "Periodic wire grouping omits an original source curve",
        ));
    }
    let chart = owner.chart().map(|a| a.map(|q| Q::from_float(q).unwrap()));
    let periods = chart.each_ref().map(|a| &a[1] - &a[0]);
    let wrapped = owner.wrapped();
    let mut transition_index = std::collections::BTreeMap::new();
    for (index, t) in owner.transitions().iter().enumerate() {
        if transition_index.insert(t.pieces, index).is_some() {
            return Err(refused(
                "Original periodic transition ownership is duplicated",
            ));
        }
    }
    let mut wires = Vec::new();
    for wire in sources {
        let mut uses = Vec::new();
        for source in wire {
            if by_source[source.source].is_empty() {
                return Err(refused("Periodic wire source has no owned original pieces"));
            }
            let ordered = if source.reversed {
                by_source[source.source]
                    .iter()
                    .rev()
                    .copied()
                    .collect::<Vec<_>>()
            } else {
                by_source[source.source].clone()
            };
            uses.extend(ordered.into_iter().map(|piece| OriginalPeriodicWireUse {
                piece,
                reversed: source.reversed,
            }));
        }
        if uses.len() > 16384 {
            return Err(refused("Periodic wire coedge allowance exceeded"));
        }
        let mut joins = Vec::new();
        let mut homology = [BigInt::from(0), BigInt::from(0)];
        for i in 0..uses.len() {
            let next = (i + 1) % uses.len();
            let a = uses[i];
            let b = uses[next];
            let key = if !a.reversed && !b.reversed {
                Some([a.piece, b.piece])
            } else if a.reversed && b.reversed {
                Some([b.piece, a.piece])
            } else {
                None
            };
            let original = key
                .and_then(|key| transition_index.get(&key).copied())
                .map(|index| (index, &owner.transitions()[index]));
            let (kind, chart_delta, native_period_delta) = if let Some((transition, t)) = original {
                // An internal native cut is already proved in its actual root
                // extension. Its original endpoint is continuous; only its
                // canonical chart representative changes by whole periods.
                let mut delta = [BigInt::from(0), BigInt::from(0)];
                for axis in 0..2 {
                    let d = t.period_delta[axis]
                        .parse::<BigInt>()
                        .map_err(|_| refused("Invalid periodic transition integer"))?;
                    delta[axis] = if a.reversed { d } else { -d };
                }
                (
                    OriginalPeriodicJoinKind::SourceTransition {
                        transition,
                        reversed: a.reversed,
                    },
                    delta,
                    [BigInt::from(0), BigInt::from(0)],
                )
            } else {
                // Different original curves (or source closure) meet at their
                // rational original natural-domain endpoints, not a midpoint.
                let end = owner.endpoint_verified(a.piece, if a.reversed { 0 } else { 1 })?;
                let start = owner.endpoint_verified(b.piece, if b.reversed { 1 } else { 0 })?;
                let end = rational_point(&end)?;
                let start = rational_point(&start)?;
                let pa = &owner.pieces()[a.piece];
                let pb = &owner.pieces()[b.piece];
                let mut delta = [BigInt::from(0), BigInt::from(0)];
                let mut native = [BigInt::from(0), BigInt::from(0)];
                for axis in 0..2 {
                    let difference = &start[axis] - &end[axis];
                    if wrapped[axis] {
                        let d = &difference / &periods[axis];
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
                        let d = original_difference / &periods[axis];
                        if !d.is_integer() {
                            return Err(refused(
                                "Periodic native wire source lift has a fractional period jump",
                            ));
                        }
                        native[axis] = d.to_integer();
                    } else if !difference.is_zero() {
                        return Err(refused(
                            "Periodic wire source endpoints disagree on an unwrapped coordinate",
                        ));
                    }
                }
                (OriginalPeriodicJoinKind::SourceJunction, delta, native)
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
        wires.push(OriginalPeriodicWire {
            uses,
            joins,
            homology: std::array::from_fn(|axis| {
                if wrapped[axis] {
                    Some(homology[axis].to_string())
                } else {
                    None
                }
            }),
        });
    }
    Ok(wires)
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
