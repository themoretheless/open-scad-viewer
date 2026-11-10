//! Exact chart-local relative winding; no implicit outside on a periodic surface.
use super::*;
use crate::profile_certificate::algebraic_intersections::{decode_polynomial, decode_ratio};
use num_bigint::BigInt;
use std::sync::Arc;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicPathCrossing {
    pub segment: usize,
    pub piece: usize,
    pub source: usize,
    pub wire: usize,
    pub delta: i64,
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicRelativeWinding {
    owner: Arc<OwnedPeriodicWireJumps>,
    path: Vec<[ExactRatio; 2]>,
    limits: ParameterOrderLimits,
    max_work: usize,
    crossings: Vec<OriginalPeriodicPathCrossing>,
    deltas: Vec<String>,
}
fn derive(
    o: &OwnedPeriodicRelativeWinding,
) -> Result<(Vec<OriginalPeriodicPathCrossing>, Vec<String>)> {
    if !(2..=4096).contains(&o.path.len())
        || !(1..=1000000).contains(&o.max_work)
        || o.limits.max_refinements > 512
        || !(1..=128).contains(&o.limits.max_resultant_size)
    {
        return Err(refused(
            "Invalid complete chart-local winding path allowance",
        ));
    }
    let graph = o.owner.owner().owner();
    let cover = graph.owner().owner().pieces();
    if (o.path.len() - 1)
        .checked_mul(cover.pieces().len())
        .is_none_or(|n| n > o.max_work)
    {
        return Err(refused(
            "Complete chart-local winding crossing census exceeds allowance",
        ));
    }
    super::periodic_replay::verify_arc(&o.owner, |x| x.reverify_source())?;
    let chart = cover.chart().map(|d| d.map(|v| Q::from_float(v).unwrap()));
    let path = o
        .path
        .iter()
        .map(|p| Ok([decode_ratio(&p[0])?, decode_ratio(&p[1])?]))
        .collect::<Result<Vec<_>>>()?;
    for p in &path {
        for axis in 0..2 {
            if p[axis] <= chart[axis][0] || p[axis] >= chart[axis][1] {
                return Err(refused(
                    "Relative winding path must stay strictly inside the original chart",
                ));
            }
        }
    }
    let wires = graph.seam_contacts().embedding().wires();
    let mut source_wire = vec![None; cover.sources().len()];
    for (wire, uses) in wires.source_uses().iter().enumerate() {
        for u in uses {
            if source_wire[u.source].replace((wire, u.reversed)).is_some() {
                return Err(refused(
                    "Relative winding source occurs in multiple authored wires",
                ));
            }
        }
    }
    let original = cover
        .sources()
        .iter()
        .map(cells)
        .collect::<Result<Vec<_>>>()?;
    let mut prepared = Vec::new();
    for p in cover.pieces() {
        let cell = &original[p.source][p.cell];
        let shift = [
            decode_ratio(&p.translation[0])?,
            decode_ratio(&p.translation[1])?,
        ];
        let net = cell
            .net
            .iter()
            .map(|v| {
                [
                    &v[0] + &shift[0] * &v[2],
                    &v[1] + &shift[1] * &v[2],
                    v[2].clone(),
                ]
            })
            .collect::<Vec<_>>();
        let bound = |i: usize| -> Result<arithmetic::branch_neighbourhood::ParameterValue> {
            let p = &cover.cuts()[p.source][p.cuts[i]].value;
            Ok(arithmetic::branch_neighbourhood::ParameterValue {
                primitive: decode_polynomial(&p.primitive)?,
                value: decode_polynomial(&p.parameter)?,
                interval: [decode_ratio(&p.interval[0])?, decode_ratio(&p.interval[1])?],
            })
        };
        prepared.push((net, cell.domain.clone(), [bound(0)?, bound(1)?]));
    }
    let mut totals = vec![BigInt::from(0); wires.wires().len()];
    let mut crossings = Vec::new();
    for (segment, points) in path.windows(2).enumerate() {
        for (piece, p) in cover.pieces().iter().enumerate() {
            let (wire, reversed) = source_wire[p.source]
                .ok_or_else(|| refused("Relative winding has no authored wire"))?;
            let (net, domain, bounds) = &prepared[piece];
            let value = arithmetic::selected_winding::segment(
                net,
                domain,
                bounds,
                &[points[0].clone(), points[1].clone()],
                o.limits.max_refinements,
                o.limits.max_resultant_size,
            )?
            .ok_or_else(|| {
                refused("Relative winding path vertex lies on an original selected source")
            })?;
            let delta = if reversed {
                value
                    .checked_neg()
                    .ok_or_else(|| refused("Relative winding sign overflow"))?
            } else {
                value
            };
            totals[wire] += BigInt::from(delta);
            crossings.push(OriginalPeriodicPathCrossing {
                segment,
                piece,
                source: p.source,
                wire,
                delta,
            });
        }
    }
    Ok((
        crossings,
        totals.into_iter().map(|v| v.to_string()).collect(),
    ))
}
impl OwnedPeriodicRelativeWinding {
    pub fn owner(&self) -> &Arc<OwnedPeriodicWireJumps> {
        &self.owner
    }
    pub fn path(&self) -> &[[ExactRatio; 2]] {
        &self.path
    }
    pub fn crossings(&self) -> &[OriginalPeriodicPathCrossing] {
        &self.crossings
    }
    pub fn deltas(&self) -> &[String] {
        &self.deltas
    }
    pub fn reverify_source(&self) -> Result<()> {
        let _replay = super::periodic_replay::scope();
        let (c, d) = derive(self)?;
        if c != self.crossings || d != self.deltas {
            return Err(refused(
                "Original relative winding path, crossing census or wire delta changed",
            ));
        }
        Ok(())
    }
}
/// Chart-local CHANGE per authored wire along a declared generic finite-segment path.
/// No base material value, period compatibility, Boolean predicate, connected
/// region or physical face is inferred from these relative values.
pub fn own_periodic_relative_winding(
    owner: Arc<OwnedPeriodicWireJumps>,
    path: &[[ExactRatio; 2]],
    limits: ParameterOrderLimits,
    max_work: usize,
) -> Result<OwnedPeriodicRelativeWinding> {
    let _replay = super::periodic_replay::scope();
    let mut o = OwnedPeriodicRelativeWinding {
        owner,
        path: path.to_vec(),
        limits,
        max_work,
        crossings: Vec::new(),
        deltas: Vec::new(),
    };
    let (c, d) = derive(&o)?;
    o.crossings = c;
    o.deltas = d;
    Ok(o)
}

/// A batch-local, source-verified census. No caller can construct its prepared data.
/// Ordinary returned certificates still replay through the independent legacy path.
pub(super) struct RelativeWindingView {
    owner: Arc<OwnedPeriodicWireJumps>,
    limits: ParameterOrderLimits,
    chart: [[Q; 2]; 2],
    periods: [Q; 2],
    wrapped: [bool; 2],
    source_wire: Vec<Option<(usize, bool)>>,
    sources: Vec<usize>,
    wire_count: usize,
    prepared: Vec<(
        Vec<[Q; 3]>,
        [Q; 2],
        [arithmetic::branch_neighbourhood::ParameterValue; 2],
    )>,
}
impl RelativeWindingView {
    #[allow(dead_code)]
    pub(super) fn new(
        owner: Arc<OwnedPeriodicWireJumps>,
        limits: ParameterOrderLimits,
    ) -> Result<Self> {
        Self::prepare(owner, limits, true)
    }
    pub(super) fn from_verified_material(
        proof: &super::periodic_material::VerifiedMaterialRule<'_>,
        limits: ParameterOrderLimits,
    ) -> Result<Self> {
        Self::prepare(proof.source_owner().clone(), limits, false)
    }
    fn prepare(
        owner: Arc<OwnedPeriodicWireJumps>,
        limits: ParameterOrderLimits,
        replay: bool,
    ) -> Result<Self> {
        if limits.max_refinements > 512 || !(1..=128).contains(&limits.max_resultant_size) {
            return Err(refused("Invalid batch winding parameter allowance"));
        }
        if replay {
            super::periodic_replay::verify_arc(&owner, |x| x.reverify_source())?;
        }
        let graph = owner.owner().owner();
        let cover = graph.owner().owner().pieces();
        let wires = graph.seam_contacts().embedding().wires();
        let mut source_wire = vec![None; cover.sources().len()];
        for (wire, uses) in wires.source_uses().iter().enumerate() {
            for u in uses {
                if source_wire[u.source].replace((wire, u.reversed)).is_some() {
                    return Err(refused(
                        "Relative winding source occurs in multiple authored wires",
                    ));
                }
            }
        }
        let original = cover
            .sources()
            .iter()
            .map(cells)
            .collect::<Result<Vec<_>>>()?;
        let mut prepared = Vec::new();
        for p in cover.pieces() {
            let cell = &original[p.source][p.cell];
            let shift = [
                decode_ratio(&p.translation[0])?,
                decode_ratio(&p.translation[1])?,
            ];
            let net = cell
                .net
                .iter()
                .map(|v| {
                    [
                        &v[0] + &shift[0] * &v[2],
                        &v[1] + &shift[1] * &v[2],
                        v[2].clone(),
                    ]
                })
                .collect::<Vec<_>>();
            let bound = |i: usize| -> Result<arithmetic::branch_neighbourhood::ParameterValue> {
                let p = &cover.cuts()[p.source][p.cuts[i]].value;
                Ok(arithmetic::branch_neighbourhood::ParameterValue {
                    primitive: decode_polynomial(&p.primitive)?,
                    value: decode_polynomial(&p.parameter)?,
                    interval: [decode_ratio(&p.interval[0])?, decode_ratio(&p.interval[1])?],
                })
            };
            prepared.push((net, cell.domain.clone(), [bound(0)?, bound(1)?]));
        }

        let chart = cover.chart().map(|d| d.map(|v| Q::from_float(v).unwrap()));
        let periods = chart.clone().map(|d| d[1].clone() - &d[0]);
        let wrapped = cover.wrapped();
        let sources = cover.pieces().iter().map(|p| p.source).collect();
        let wire_count = wires.wires().len();
        Ok(Self {
            owner,
            limits,
            chart,
            periods,
            wrapped,
            source_wire,
            sources,
            wire_count,
            prepared,
        })
    }
    pub(super) fn query(
        &self,
        path: &[[ExactRatio; 2]],
        max_work: usize,
    ) -> Result<OwnedPeriodicRelativeWinding> {
        if !(2..=4096).contains(&path.len())
            || !(1..=1000000).contains(&max_work)
            || (path.len() - 1)
                .checked_mul(self.prepared.len())
                .is_none_or(|n| n > max_work)
        {
            return Err(refused(
                "Complete batch winding crossing census exceeds allowance",
            ));
        }
        let decoded = path
            .iter()
            .map(|p| Ok([decode_ratio(&p[0])?, decode_ratio(&p[1])?]))
            .collect::<Result<Vec<_>>>()?;
        for p in &decoded {
            for axis in 0..2 {
                if p[axis] <= self.chart[axis][0] || p[axis] >= self.chart[axis][1] {
                    return Err(refused(
                        "Relative winding path must stay strictly inside the original chart",
                    ));
                }
            }
        }
        let (crossings, deltas) = self.compute(&decoded)?;
        Ok(OwnedPeriodicRelativeWinding {
            owner: self.owner.clone(),
            path: path.to_vec(),
            limits: self.limits,
            max_work,
            crossings,
            deltas,
        })
    }
    /// Closed geometry clearance against ALL authored pieces, even zero-weight
    /// operands. This does not itself assign a material value to a shore point.
    pub(super) fn clear(&self, endpoints: &[[ExactRatio; 2]; 2], max_work: usize) -> Result<bool> {
        use num_traits::ToPrimitive;
        if !(1..=1000000).contains(&max_work) {
            return Err(refused("Invalid all-source clearance work allowance"));
        }
        let points = endpoints
            .iter()
            .map(|p| Ok([decode_ratio(&p[0])?, decode_ratio(&p[1])?]))
            .collect::<Result<Vec<_>>>()?;
        for p in &points {
            for axis in 0..2 {
                if p[axis] < self.chart[axis][0] || p[axis] > self.chart[axis][1] {
                    return Err(refused(
                        "All-source clearance leg escaped the closed original chart",
                    ));
                }
            }
        }
        let ranges: [[BigInt; 2]; 2] = std::array::from_fn(|axis| {
            if self.wrapped[axis] {
                let lo = points[0][axis].clone().min(points[1][axis].clone());
                let hi = points[0][axis].clone().max(points[1][axis].clone());
                [
                    ((lo - &self.chart[axis][1]) / &self.periods[axis])
                        .ceil()
                        .to_integer(),
                    ((hi - &self.chart[axis][0]) / &self.periods[axis])
                        .floor()
                        .to_integer(),
                ]
            } else {
                [BigInt::from(0), BigInt::from(0)]
            }
        });
        let mut required = self.prepared.len();
        for r in &ranges {
            let count = (&r[1] - &r[0] + BigInt::from(1))
                .to_usize()
                .filter(|n| *n > 0)
                .ok_or_else(|| refused("All-source lattice count exceeds allowance"))?;
            required = required
                .checked_mul(count)
                .ok_or_else(|| refused("All-source lattice count overflow"))?;
        }
        if required > max_work {
            return Err(refused(
                "Complete all-source clearance census exceeds allowance",
            ));
        }
        for (net, domain, bounds) in &self.prepared {
            let mut u = ranges[0][0].clone();
            while u <= ranges[0][1] {
                let mut v = ranges[1][0].clone();
                while v <= ranges[1][1] {
                    let shift = [
                        Q::from_integer(u.clone()) * &self.periods[0],
                        Q::from_integer(v.clone()) * &self.periods[1],
                    ];
                    let translated = net
                        .iter()
                        .map(|p| {
                            [
                                &p[0] + &shift[0] * &p[2],
                                &p[1] + &shift[1] * &p[2],
                                p[2].clone(),
                            ]
                        })
                        .collect::<Vec<_>>();
                    if !arithmetic::selected_clearance::clear(
                        &translated,
                        domain,
                        bounds,
                        &[points[0].clone(), points[1].clone()],
                        self.limits.max_refinements,
                        self.limits.max_resultant_size,
                    )? {
                        return Ok(false);
                    }
                    v += BigInt::from(1);
                }
                u += BigInt::from(1);
            }
        }
        Ok(true)
    }
    fn compute(&self, path: &[[Q; 2]]) -> Result<(Vec<OriginalPeriodicPathCrossing>, Vec<String>)> {
        let mut totals = vec![BigInt::from(0); self.wire_count];
        let mut crossings = Vec::new();
        for (segment, points) in path.windows(2).enumerate() {
            for (piece, source) in self.sources.iter().enumerate() {
                let (wire, reversed) = self.source_wire[*source]
                    .ok_or_else(|| refused("Relative winding has no authored wire"))?;
                let (net, domain, bounds) = &self.prepared[piece];
                let value = arithmetic::selected_winding::segment(
                    net,
                    domain,
                    bounds,
                    &[points[0].clone(), points[1].clone()],
                    self.limits.max_refinements,
                    self.limits.max_resultant_size,
                )?
                .ok_or_else(|| {
                    refused("Relative winding path vertex lies on an original selected source")
                })?;
                let delta = if reversed {
                    value
                        .checked_neg()
                        .ok_or_else(|| refused("Relative winding sign overflow"))?
                } else {
                    value
                };
                totals[wire] += BigInt::from(delta);
                crossings.push(OriginalPeriodicPathCrossing {
                    segment,
                    piece,
                    source: *source,
                    wire,
                    delta,
                });
            }
        }
        Ok((
            crossings,
            totals.into_iter().map(|v| v.to_string()).collect(),
        ))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn p(x: &str, y: &str) -> [ExactRatio; 2] {
        [x, y].map(|s| {
            let (a, b) = s.split_once('/').unwrap_or((s, "1"));
            [a.to_string(), b.to_string()]
        })
    }
    #[test]
    fn essential_opposite_wires_retain_separate_relative_values_and_replay_original_sources() {
        let line = |a, b| Curve::from_polyline(vec![vec![a, 0.5], vec![b, 0.5]]).unwrap();
        let owner = Arc::new(super::super::periodic_wire_jumps::tests::own(
            &[line(0., 8.), line(4., 0.)],
            &[
                vec![OriginalPeriodicSourceUse {
                    source: 0,
                    reversed: false,
                }],
                vec![OriginalPeriodicSourceUse {
                    source: 1,
                    reversed: false,
                }],
            ],
        ));
        let path = [p("1/2", "1/4"), p("1/2", "3/4")];
        let o = own_periodic_relative_winding(
            owner.clone(),
            &path,
            ParameterOrderLimits::default(),
            100,
        )
        .unwrap();
        assert_eq!(o.deltas(), &["2", "-1"]);
        let view =
            RelativeWindingView::new(owner.clone(), ParameterOrderLimits::default()).unwrap();
        for points in [path.clone(), [path[1].clone(), path[0].clone()]] {
            let batched = view.query(&points, 100).unwrap();
            let independent = own_periodic_relative_winding(
                owner.clone(),
                &points,
                ParameterOrderLimits::default(),
                100,
            )
            .unwrap();
            assert_eq!(batched.crossings(), independent.crossings());
            assert_eq!(batched.deltas(), independent.deltas());
            batched.reverify_source().unwrap();
        }
        assert!(view.query(&path, 1).is_err());
        assert!(!view.clear(&path, 100).unwrap());
        assert!(view.clear(&[p("0", "1/4"), p("1/2", "1/4")], 100).unwrap());
        assert!(!view.clear(&[p("0", "1/2"), p("1/2", "1/4")], 100).unwrap());
        assert!(view.clear(&path, 1).is_err());
        assert!(view.clear(&[p("-1", "1/4"), path[0].clone()], 100).is_err());

        assert!(view.query(&[p("0", "1/4"), path[1].clone()], 100).is_err());
        assert!(
            view.query(&[path[0].clone(), p("1/2", "1/2")], 100)
                .is_err()
        );

        o.reverify_source().unwrap();
        let reversed = own_periodic_relative_winding(
            owner.clone(),
            &[path[1].clone(), path[0].clone()],
            ParameterOrderLimits::default(),
            100,
        )
        .unwrap();
        assert_eq!(reversed.deltas(), &["-2", "1"]);
        let mut bad = o.clone();
        bad.crossings.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = o.clone();
        bad.deltas[0] = "1".to_string();
        assert!(bad.reverify_source().is_err());
        let mut bad = o.clone();
        bad.path[1] = p("1/2", "1/3");
        assert!(bad.reverify_source().is_err());
        assert!(
            own_periodic_relative_winding(owner.clone(), &path, ParameterOrderLimits::default(), 1)
                .is_err()
        );
        assert!(
            own_periodic_relative_winding(
                owner.clone(),
                &[p("0", "1/4"), p("1/2", "3/4")],
                ParameterOrderLimits::default(),
                100
            )
            .is_err()
        );
        assert!(
            own_periodic_relative_winding(
                owner.clone(),
                &[p("1/2", "1/4"), p("1/2", "1/2")],
                ParameterOrderLimits::default(),
                100
            )
            .is_err()
        );
        let oblique = own_periodic_relative_winding(
            owner,
            &[p("1/2", "1/4"), p("3/4", "3/4")],
            ParameterOrderLimits::default(),
            100,
        )
        .unwrap();
        assert_eq!(oblique.deltas(), &["2", "-1"]);
        oblique.reverify_source().unwrap();
    }
}
