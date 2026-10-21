//! Exact cut propagation and whole-interval counterpart alignment.
use super::*;
use periodic_events::{common_clip::lift_value, compare};
use std::{cmp::Ordering, sync::Arc};
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OriginalPeriodicAlignedOrigin {
    Seed {
        cut: usize,
    },
    Propagated {
        pair: usize,
        clip: usize,
        side: usize,
        source: OriginalSourceParameter,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicAlignedCut {
    pub value: OriginalSourceParameter,
    pub origins: Vec<OriginalPeriodicAlignedOrigin>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicIntervalBinding {
    pub pair: usize,
    pub clip: usize,
    pub side: usize,
    pub source: OriginalSourceParameter,
    pub node: AlgebraicEventNode,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicIntervalAlias {
    pub pair: usize,
    pub clip: usize,
    pub side: usize,
    /// Piece-local consecutive cut interval indices, in source/target order.
    pub pieces: [usize; 2],
    pub intervals: [usize; 2],
    pub reversed: bool,
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicAlignedIntervals {
    owner: Arc<OwnedPeriodicArrangementCuts>,
    limits: ParameterOrderLimits,
    max_lifts: usize,
    cuts: Vec<Vec<OriginalPeriodicAlignedCut>>,
    bindings: Vec<OriginalPeriodicIntervalBinding>,
    aliases: Vec<OriginalPeriodicIntervalAlias>,
    lifts: usize,
}
fn value(n: &AlgebraicEventNode, side: usize) -> OriginalSourceParameter {
    OriginalSourceParameter {
        primitive: n.exact.primitive.clone(),
        interval: n.exact.isolating_interval.clone(),
        parameter: n.exact.parameters[side].clone(),
    }
}
fn inside(
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
fn insert(
    cuts: &mut Vec<OriginalPeriodicAlignedCut>,
    v: OriginalSourceParameter,
    l: ParameterOrderLimits,
) -> Result<bool> {
    for i in 0..cuts.len() {
        match compare(&v, &cuts[i].value, l)? {
            Ordering::Equal => return Ok(false),
            Ordering::Less => {
                cuts.insert(
                    i,
                    OriginalPeriodicAlignedCut {
                        value: v,
                        origins: Vec::new(),
                    },
                );
                return Ok(true);
            }
            Ordering::Greater => {}
        }
    }
    cuts.push(OriginalPeriodicAlignedCut {
        value: v,
        origins: Vec::new(),
    });
    Ok(true)
}
fn index(
    cuts: &[OriginalPeriodicAlignedCut],
    v: &OriginalSourceParameter,
    l: ParameterOrderLimits,
) -> Result<usize> {
    for (i, c) in cuts.iter().enumerate() {
        if compare(&c.value, v, l)? == Ordering::Equal {
            return Ok(i);
        }
    }
    Err(refused("Exact common counterpart cut is missing"))
}
type Data = (
    Vec<Vec<OriginalPeriodicAlignedCut>>,
    Vec<OriginalPeriodicIntervalBinding>,
    Vec<OriginalPeriodicIntervalAlias>,
    usize,
);
fn derive(o: &OwnedPeriodicAlignedIntervals) -> Result<Data> {
    if o.max_lifts == 0
        || o.max_lifts > 100000
        || o.limits.max_refinements > 512
        || !(1..=128).contains(&o.limits.max_resultant_size)
    {
        return Err(refused(
            "Invalid complete periodic cut propagation allowance",
        ));
    }
    super::periodic_replay::verify(&o.owner)?;
    let mut cuts = o
        .owner
        .cuts()
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(cut, c)| OriginalPeriodicAlignedCut {
                    value: c.value.clone(),
                    origins: vec![OriginalPeriodicAlignedOrigin::Seed { cut }],
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut contexts = Vec::new();
    for pair in o.owner.pairs() {
        let c = pair.owner().census();
        let (_, x, y) = context(
            &c.sources[0],
            &c.sources[1],
            &c.offsets,
            false,
            c.certificate.maximum_parameter_width,
        )?;
        contexts.push((x, y));
    }
    let mut lifts = 0;
    let bindings = loop {
        let mut changed = false;
        let mut round = Vec::new();
        for (pair, p) in o.owner.pairs().iter().enumerate() {
            let pieces = p.owner().pair();
            for (clip, c) in p.clips().iter().enumerate() {
                if c.point_only {
                    continue;
                }
                for side in 0..2 {
                    let source = pieces[side];
                    let target = pieces[1 - side];
                    for cut in cuts[source].clone() {
                        if !inside(
                            &cut.value,
                            &value(&c.endpoints[0], side),
                            &value(&c.endpoints[1], side),
                            o.limits,
                        )? {
                            continue;
                        }
                        if lifts >= o.max_lifts {
                            return Err(refused(
                                "Complete periodic common cut propagation exhausted its lift allowance",
                            ));
                        }
                        lifts += 1;
                        let nodes = lift_value(
                            p.owner(),
                            c.pair,
                            side,
                            &cut.value,
                            &contexts[pair].0,
                            &contexts[pair].1,
                        )?;
                        let node = nodes
                            .into_iter()
                            .next()
                            .ok_or_else(|| refused("Missing unique periodic cut counterpart"))?;
                        let target_value = value(&node, 1 - side);
                        if !inside(
                            &target_value,
                            &value(&c.endpoints[0], 1 - side),
                            &value(&c.endpoints[1], 1 - side),
                            o.limits,
                        )? {
                            return Err(refused(
                                "Common cut counterpart escaped its proved clipped component",
                            ));
                        }
                        changed |= insert(&mut cuts[target], target_value, o.limits)?;
                        round.push(OriginalPeriodicIntervalBinding {
                            pair,
                            clip,
                            side,
                            source: cut.value,
                            node,
                        });
                    }
                }
            }
        }
        if !changed {
            break round;
        }
    };
    for b in &bindings {
        let pieces = o.owner.pairs()[b.pair].owner().pair();
        let target = pieces[1 - b.side];
        let i = index(&cuts[target], &value(&b.node, 1 - b.side), o.limits)?;
        let origin = OriginalPeriodicAlignedOrigin::Propagated {
            pair: b.pair,
            clip: b.clip,
            side: b.side,
            source: b.source.clone(),
        };
        if !cuts[target][i].origins.contains(&origin) {
            cuts[target][i].origins.push(origin);
        }
    }
    let mut aliases = Vec::new();
    for (pair, p) in o.owner.pairs().iter().enumerate() {
        let pieces = p.owner().pair();
        for (clip, c) in p.clips().iter().enumerate() {
            if c.point_only {
                continue;
            }
            for side in 0..2 {
                let source = pieces[side];
                let target = pieces[1 - side];
                for i in 0..cuts[source].len() - 1 {
                    let a = &cuts[source][i].value;
                    let b = &cuts[source][i + 1].value;
                    if !inside(
                        a,
                        &value(&c.endpoints[0], side),
                        &value(&c.endpoints[1], side),
                        o.limits,
                    )? || !inside(
                        b,
                        &value(&c.endpoints[0], side),
                        &value(&c.endpoints[1], side),
                        o.limits,
                    )? {
                        continue;
                    }
                    let target_index = |v: &OriginalSourceParameter| -> Result<usize> {
                        let binding = bindings
                            .iter()
                            .find(|r| {
                                r.pair == pair && r.clip == clip && r.side == side && r.source == *v
                            })
                            .ok_or_else(|| refused("Missing complete common endpoint binding"))?;
                        index(&cuts[target], &value(&binding.node, 1 - side), o.limits)
                    };
                    let x = target_index(a)?;
                    let y = target_index(b)?;
                    if x.abs_diff(y) != 1 {
                        return Err(refused(
                            "Common source interval does not match one WHOLE counterpart interval",
                        ));
                    }
                    aliases.push(OriginalPeriodicIntervalAlias {
                        pair,
                        clip,
                        side,
                        pieces: [source, target],
                        intervals: [i, x.min(y)],
                        reversed: x > y,
                    });
                }
            }
        }
    }
    Ok((cuts, bindings, aliases, lifts))
}
impl OwnedPeriodicAlignedIntervals {
    pub fn owner(&self) -> &Arc<OwnedPeriodicArrangementCuts> {
        &self.owner
    }
    pub fn cuts(&self) -> &[Vec<OriginalPeriodicAlignedCut>] {
        &self.cuts
    }
    pub fn bindings(&self) -> &[OriginalPeriodicIntervalBinding] {
        &self.bindings
    }
    pub fn aliases(&self) -> &[OriginalPeriodicIntervalAlias] {
        &self.aliases
    }
    pub fn lifts(&self) -> usize {
        self.lifts
    }
    pub fn reverify_source(&self) -> Result<()> {
        let _replay = super::periodic_replay::scope();
        let (c, b, a, w) = derive(self)?;
        if c != self.cuts || b != self.bindings || a != self.aliases || w != self.lifts {
            return Err(refused(
                "Periodic whole-interval alignment proof or work census changed",
            ));
        }
        Ok(())
    }
}
pub fn own_periodic_aligned_intervals(
    owner: Arc<OwnedPeriodicArrangementCuts>,
    limits: ParameterOrderLimits,
    max_lifts: usize,
) -> Result<OwnedPeriodicAlignedIntervals> {
    let _replay = super::periodic_replay::scope();
    let mut o = OwnedPeriodicAlignedIntervals {
        owner,
        limits,
        max_lifts,
        cuts: Vec::new(),
        bindings: Vec::new(),
        aliases: Vec::new(),
        lifts: 0,
    };
    let (c, b, a, w) = derive(&o)?;
    o.cuts = c;
    o.bindings = b;
    o.aliases = a;
    o.lifts = w;
    Ok(o)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn seeds(sources: &[Curve], max: usize) -> Arc<OwnedPeriodicArrangementCuts> {
        let pieces = Arc::new(
            own_original_periodic_pieces(
                sources,
                [[0., 1.], [0., 1.]],
                [false, false],
                1e-12,
                ParameterOrderLimits::default(),
                16,
            )
            .unwrap(),
        );
        Arc::new(
            own_periodic_arrangement_cuts(pieces, 1e-12, ParameterOrderLimits::default(), max)
                .unwrap(),
        )
    }
    #[test]
    fn nonlinear_counterparts_align_through_joint_sixth_root_original_clocks() {
        let a = Curve {
            degree: 2,
            knots: vec![2., 2., 2., 6., 6., 6.],
            control_points: vec![vec![0., 0.5], vec![0., 0.5], vec![1., 0.5]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let b = Curve {
            degree: 3,
            knots: vec![3., 3., 3., 3., 9., 9., 9., 9.],
            control_points: vec![vec![0., 0.5], vec![0., 0.5], vec![0., 0.5], vec![1., 0.5]],
            weights: vec![1.; 4],
            periodic: false,
        };
        let c = Curve::from_polyline(vec![vec![0.5, 0.], vec![0.5, 1.]]).unwrap();
        let owner = seeds(&[a, b, c], 6);
        assert!(
            own_periodic_aligned_intervals(owner.clone(), ParameterOrderLimits::default(), 1)
                .is_err()
        );
        let o =
            own_periodic_aligned_intervals(owner, ParameterOrderLimits::default(), 512).unwrap();
        o.reverify_source().unwrap();
        assert!(o.cuts().iter().all(|c| c.len() == 3));
        let aliases = o
            .aliases()
            .iter()
            .filter(|a| a.pair == 1 && a.side == 0)
            .collect::<Vec<_>>();
        assert_eq!(aliases.len(), 2);
        assert_eq!(aliases[0].intervals, [0, 0]);
        assert_eq!(aliases[1].intervals, [1, 1]);
        assert!(
            o.bindings()
                .iter()
                .any(|b| b.pair == 1 && b.node.exact.primitive.len() >= 7)
        );
        assert!(o.lifts() > 0);
    }
    #[test]
    fn reversed_whole_intervals_map_in_opposite_order_and_keep_provenance() {
        let a = Curve::from_polyline(vec![vec![0., 0.5], vec![1., 0.5]]).unwrap();
        let b = Curve::from_polyline(vec![vec![1., 0.5], vec![0., 0.5]]).unwrap();
        let c = Curve::from_polyline(vec![vec![0.25, 0.], vec![0.25, 1.]]).unwrap();
        let o = own_periodic_aligned_intervals(
            seeds(&[a, b, c], 6),
            ParameterOrderLimits::default(),
            512,
        )
        .unwrap();
        o.reverify_source().unwrap();
        let aliases = o
            .aliases()
            .iter()
            .filter(|a| a.pair == 1 && a.side == 0)
            .collect::<Vec<_>>();
        assert_eq!(aliases.len(), 2);
        assert_eq!(aliases[0].intervals, [0, 1]);
        assert_eq!(aliases[1].intervals, [1, 0]);
        assert!(aliases.iter().all(|a| a.reversed));
        assert!(
            o.cuts()[1][1]
                .origins
                .iter()
                .any(|p| matches!(p, OriginalPeriodicAlignedOrigin::Propagated { pair: 1, .. }))
        );
        let mut bad = o.clone();
        bad.aliases[0].intervals[1] = 99;
        assert!(bad.reverify_source().is_err());
        let mut bad = o.clone();
        bad.bindings.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = o.clone();
        bad.lifts += 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = o;
        bad.cuts[1][1].origins.clear();
        assert!(bad.reverify_source().is_err());
    }
}
