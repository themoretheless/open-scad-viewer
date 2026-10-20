//! Source-bound one-sided witnesses for local cycle orientation.
//! Does not assert whole-cycle containment, patch ownership or manifold sewing.
use super::*;
use crate::profile_certificate::algebraic_intersections::{decode_ratio, ratio};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeriodicChartCycleOrientation {
    Positive,
    Negative,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicChartCycleSides {
    pub cycle: usize,
    pub edge: usize,
    /// Left and right of this actual oriented boundary edge.
    pub points: [[ExactRatio; 2]; 2],
    /// Complete exact local-cycle winding vectors at the two witnesses.
    pub windings: [Vec<i64>; 2],
    pub orientation: PeriodicChartCycleOrientation,
}
#[derive(Clone, Copy, Debug)]
pub struct PeriodicChartCycleSideLimits {
    pub max_cycles: usize,
    pub max_query_work: usize,
    pub max_levels: usize,
}
impl Default for PeriodicChartCycleSideLimits {
    fn default() -> Self {
        Self {
            max_cycles: 100000,
            max_query_work: 1000000,
            max_levels: 128,
        }
    }
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicChartCycleSides {
    owner: Arc<OwnedPeriodicChartWinding>,
    limits: PeriodicChartCycleSideLimits,
    sides: Vec<OriginalPeriodicChartCycleSides>,
}
fn inside(p: &[Q; 2], chart: &[[Q; 2]; 2]) -> bool {
    (0..2).all(|a| chart[a][0] < p[a] && p[a] < chart[a][1])
}
fn derive(o: &OwnedPeriodicChartCycleSides) -> Result<Vec<OriginalPeriodicChartCycleSides>> {
    let _replay = super::periodic_replay::scope();
    let limits = o.limits;
    if !(1..=1000000).contains(&limits.max_cycles)
        || !(1..=1000000).contains(&limits.max_query_work)
        || !(1..=512).contains(&limits.max_levels)
    {
        return Err(refused("Invalid complete cycle side witness allowance"));
    }
    o.owner.reverify_source()?;
    let cycles = o.owner.owner();
    let samples = cycles.owner().owner();
    let shores = samples.owner();
    let traces = shores.owner().owner().owner().owner();
    let regions = traces.owner();
    let lifts = regions.owner().owner();
    let selection = lifts.owner().owner();
    let cover = selection.rule().support().augmented().surface().pieces();
    let chart = cover.chart().map(|r| r.map(|v| Q::from_float(v).unwrap()));
    if cycles.walks().len() > limits.max_cycles {
        return Err(refused("Complete cycle side census exceeds allowance"));
    }
    let mut work = cycles
        .walks()
        .len()
        .checked_mul(2)
        .and_then(|n| n.checked_mul(cycles.edges().len()))
        .and_then(|n| n.checked_add(samples.samples().len()))
        .ok_or_else(|| refused("Cycle side winding work overflow"))?;
    if work > limits.max_query_work {
        return Err(refused(
            "Complete cycle side winding work exceeds allowance",
        ));
    }
    let sample_index = samples
        .samples()
        .iter()
        .map(|s| ((s.shore, s.interval), s))
        .collect::<BTreeMap<_, _>>();
    let mut result = Vec::with_capacity(cycles.walks().len());
    for (cycle, walk) in cycles.walks().iter().enumerate() {
        let &edge = walk
            .first()
            .ok_or_else(|| refused("Empty chart boundary walk"))?;
        let points = match cycles.edges()[edge].kind {
            OriginalPeriodicChartCycleEdgeKind::SourceTrace(trace) => {
                let p = &traces
                    .traces()
                    .get(trace)
                    .ok_or_else(|| refused("Cycle side lost its original source trace"))?
                    .original;
                let side = selection
                    .sides()
                    .get(p.edge)
                    .ok_or_else(|| refused("Cycle side lost its original physical side proof"))?;
                if p.dart / 2 != p.edge || side.probes().edge() != p.edge {
                    return Err(refused(
                        "Cycle side rebound its original directed physical edge",
                    ));
                }
                let graph = side.probes().owner().owner().owner();
                let carrier = graph.edges()[p.edge]
                    .carriers
                    .first()
                    .ok_or_else(|| refused("Cycle side lost its original canonical carrier"))?;
                if (
                    carrier.piece,
                    carrier.source,
                    carrier.cell,
                    carrier.interval,
                ) != (p.piece, p.source, p.cell, p.interval)
                {
                    return Err(refused(
                        "Cycle side probe belongs to a different canonical source image",
                    ));
                }
                let left = p.dart % 2;
                if side.selected()[left] != true || side.selected()[1 - left] != false {
                    return Err(refused(
                        "Cycle side is not the actual selected material boundary",
                    ));
                }
                let q = side
                    .probes()
                    .lifted_queries()
                    .iter()
                    .map(|p| Ok([decode_ratio(&p[0])?, decode_ratio(&p[1])?]))
                    .collect::<Result<Vec<_>>>()?;
                let center = std::array::from_fn::<_, 2, _>(|a| {
                    (&q[0][a] + &q[1][a]) / Q::from_integer(2.into())
                });
                // A retained non-frame trace must have an interior center.
                // Shrinking the already certified transverse segment preserves
                // its unique center contact and the actual material germs.
                if !inside(&center, &chart) {
                    return Err(refused(
                        "Source cycle witness needs an interior original chart center",
                    ));
                }
                let mut points = [q[left].clone(), q[1 - left].clone()];
                let mut found = false;
                for _ in 0..limits.max_levels {
                    work = work
                        .checked_add(1)
                        .filter(|n| *n <= limits.max_query_work)
                        .ok_or_else(|| refused("Cycle side refinement work exhausted"))?;
                    if points.iter().all(|p| inside(p, &chart)) {
                        found = true;
                        break;
                    }
                    for p in &mut points {
                        for a in 0..2 {
                            p[a] = (&p[a] + &center[a]) / Q::from_integer(2.into());
                        }
                    }
                }
                if !found {
                    return Err(refused(
                        "Interior original cycle side witnesses exceed refinement allowance",
                    ));
                }
                points
            }
            OriginalPeriodicChartCycleEdgeKind::NativeShore { shore, interval } => {
                let s = shores
                    .shores()
                    .get(shore)
                    .ok_or_else(|| refused("Cycle side lost its native shore"))?;
                let sample = sample_index.get(&(shore, interval)).ok_or_else(|| {
                    refused("Cycle side lost its complete native interval material germ")
                })?;
                if !sample.selected {
                    return Err(refused(
                        "Retained frame interval has no actual inside material",
                    ));
                }
                let center = [
                    decode_ratio(&sample.center[0])?,
                    decode_ratio(&sample.center[1])?,
                ];
                let left = [
                    decode_ratio(&sample.probe[0])?,
                    decode_ratio(&sample.probe[1])?,
                ];
                if !inside(&left, &chart)
                    || left[1 - s.fixed_axis] != center[1 - s.fixed_axis]
                    || center[s.fixed_axis] != chart[s.fixed_axis][s.end]
                {
                    return Err(refused(
                        "Frame cycle side is not its owned normal inside germ",
                    ));
                }
                let mut right = left.clone();
                right[s.fixed_axis] =
                    &center[s.fixed_axis] * Q::from_integer(2.into()) - &left[s.fixed_axis];
                if !(if s.end == 0 {
                    right[s.fixed_axis] < chart[s.fixed_axis][0]
                } else {
                    right[s.fixed_axis] > chart[s.fixed_axis][1]
                }) {
                    return Err(refused(
                        "Frame cycle right witness is not strictly outside the chart",
                    ));
                }
                [left, right]
            }
        }
        .map(|p| p.map(|q| ratio(&q)));
        let mut values = Vec::with_capacity(2);
        for p in &points {
            values.push(
                o.owner.windings(p)?.ok_or_else(|| {
                    refused("Owned cycle side witness touches a retained boundary")
                })?,
            );
        }
        let windings = [values.remove(0), values.remove(0)];
        // The certified transverse normal segment crosses this one original
        // edge. Other cycle values must remain unchanged, including at contacts.
        for i in 0..cycles.walks().len() {
            if windings[0][i].checked_sub(windings[1][i]) != Some(if i == cycle { 1 } else { 0 }) {
                return Err(refused(
                    "Cycle side winding jump disagrees with its owned transverse germ",
                ));
            }
        }
        for (side, values) in windings.iter().enumerate() {
            let sum = values
                .iter()
                .try_fold(0i64, |n, v| n.checked_add(*v))
                .ok_or_else(|| refused("Cycle side material winding overflow"))?;
            if sum != if side == 0 { 1 } else { 0 } {
                return Err(refused(
                    "Cycle side winding does not reproduce actual material germs",
                ));
            }
        }
        let orientation = match [windings[0][cycle], windings[1][cycle]] {
            [1, 0] => PeriodicChartCycleOrientation::Positive,
            [0, -1] => PeriodicChartCycleOrientation::Negative,
            _ => {
                return Err(refused(
                    "Cycle orientation needs further owned fan decomposition",
                ));
            }
        };
        result.push(OriginalPeriodicChartCycleSides {
            cycle,
            edge,
            points,
            windings,
            orientation,
        });
    }
    Ok(result)
}
impl OwnedPeriodicChartCycleSides {
    pub fn owner(&self) -> &Arc<OwnedPeriodicChartWinding> {
        &self.owner
    }
    pub fn sides(&self) -> &[OriginalPeriodicChartCycleSides] {
        &self.sides
    }
    pub fn reverify_source(&self) -> Result<()> {
        if derive(self)? != self.sides {
            return Err(refused(
                "Original cycle side witness or orientation changed",
            ));
        }
        Ok(())
    }
}
pub fn own_periodic_chart_cycle_sides(
    owner: Arc<OwnedPeriodicChartWinding>,
    limits: PeriodicChartCycleSideLimits,
) -> Result<OwnedPeriodicChartCycleSides> {
    let mut o = OwnedPeriodicChartCycleSides {
        owner,
        limits,
        sides: Vec::new(),
    };
    o.sides = derive(&o)?;
    Ok(o)
}

#[cfg(test)]
pub(super) fn check_batch(winding: &OwnedPeriodicChartWinding) {
    let o = own_periodic_chart_cycle_sides(
        Arc::new(winding.clone()),
        PeriodicChartCycleSideLimits::default(),
    )
    .unwrap();
    assert_eq!(o.sides().len(), winding.owner().walks().len());
    let mut seen = std::collections::BTreeSet::new();
    let strict = winding
        .owner()
        .walks()
        .iter()
        .flatten()
        .all(|&e| seen.insert(winding.owner().edges()[e].vertices[0]));
    let patches = own_periodic_chart_strict_patches(
        Arc::new(o.clone()),
        PeriodicChartStrictPatchLimits::default(),
    );
    if strict {
        let patches = patches.unwrap();
        assert_eq!(patches.patches().len(), o.sides().len());
        assert!(patches.patches().iter().all(|p| p.holes.is_empty()));
    } else {
        assert!(patches.is_err());
    }

    assert!(
        o.sides()
            .iter()
            .all(|s| s.orientation == PeriodicChartCycleOrientation::Positive)
    );
    if o.sides().len() == 1
        && winding.owner().coincident_traces().is_empty()
        && winding.owner().edges().iter().all(|e| {
            matches!(
                e.kind,
                OriginalPeriodicChartCycleEdgeKind::NativeShore { .. }
            )
        })
    {
        let mut bad = o.clone();
        bad.sides[0].points[0][0] = ["999".into(), "1".into()];
        assert!(bad.reverify_source().is_err());
        assert!(
            own_periodic_chart_cycle_sides(
                o.owner.clone(),
                PeriodicChartCycleSideLimits {
                    max_query_work: 1,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_hole_has_distinct_positive_outer_and_negative_inner_source_germs() {
        use super::super::{
            periodic_chart_shores::tests as frame, periodic_support_regions::tests as fixture,
        };
        let signatures =
            fixture::signatures(super::super::periodic_support_material::tests::with_hole());
        let path = fixture::connection(&signatures, 0, 1, &[]);
        let regions = Arc::new(own_periodic_support_regions(signatures, &[path], 100000).unwrap());
        let shores = Arc::new(frame::shores(frame::corners(regions)));
        let samples = Arc::new(
            own_periodic_shore_material_samples(
                shores,
                ParameterOrderLimits::default(),
                PeriodicShoreMaterialSampleLimits::default(),
            )
            .unwrap(),
        );
        let intervals = Arc::new(
            own_periodic_shore_material_intervals(
                samples,
                ParameterOrderLimits::default(),
                PeriodicShoreMaterialIntervalLimits::default(),
            )
            .unwrap(),
        );
        let winding = Arc::new(
            construct_periodic_chart_winding(
                intervals,
                PeriodicChartCycleLimits::default(),
                PeriodicChartWindingLimits::default(),
            )
            .unwrap(),
        );
        let o = own_periodic_chart_cycle_sides(winding, PeriodicChartCycleSideLimits::default())
            .unwrap();
        assert_eq!(o.sides().len(), 2);
        assert_eq!(
            o.sides()
                .iter()
                .filter(|s| s.orientation == PeriodicChartCycleOrientation::Positive)
                .count(),
            1
        );
        assert_eq!(
            o.sides()
                .iter()
                .filter(|s| s.orientation == PeriodicChartCycleOrientation::Negative)
                .count(),
            1
        );
        o.reverify_source().unwrap();
        let patches = own_periodic_chart_strict_patches(
            Arc::new(o.clone()),
            PeriodicChartStrictPatchLimits::default(),
        )
        .unwrap();
        assert_eq!(patches.patches().len(), 1);
        assert_eq!(patches.patches()[0].holes.len(), 1);
        assert_eq!(
            patches.parents()[patches.patches()[0].holes[0]],
            Some(patches.patches()[0].outer)
        );
        patches.reverify_source().unwrap();
        super::super::periodic_chart_strict_patches::check_tampering(&patches);

        let mut bad = o.clone();
        bad.sides[0].orientation = match bad.sides[0].orientation {
            PeriodicChartCycleOrientation::Positive => PeriodicChartCycleOrientation::Negative,
            PeriodicChartCycleOrientation::Negative => PeriodicChartCycleOrientation::Positive,
        };
        assert!(bad.reverify_source().is_err());
    }
}
