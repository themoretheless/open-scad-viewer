//! Source-bound chart-inside POINT labels on every native frame interval.
//! A point label is not a proof of uniformity on that interval or a physical face.
use super::*;
use crate::profile_certificate::algebraic_intersections::{decode_ratio, ratio};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalPeriodicShoreSampleOrigin {
    OwnedSide {
        row: usize,
        side: usize,
    },
    /// The closed normal leg from center to probe avoids ALL original sources.
    ClearedNormalLeg,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicShoreMaterialSample {
    pub shore: usize,
    pub interval: usize,
    pub center: [ExactRatio; 2],
    /// An actual strictly interior point in the original chart.
    pub probe: [ExactRatio; 2],
    pub origin: OriginalPeriodicShoreSampleOrigin,
    pub windings: Vec<String>,
    pub operand_inside: Vec<bool>,
    pub selected: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct PeriodicShoreMaterialSampleLimits {
    pub max_intervals: usize,
    pub max_queries: usize,
    pub max_levels: usize,
    pub max_clearance_work: usize,
    pub max_winding_work: usize,
}
impl Default for PeriodicShoreMaterialSampleLimits {
    fn default() -> Self {
        Self {
            max_intervals: 100000,
            max_queries: 1000000,
            max_levels: 128,
            max_clearance_work: 100000,
            max_winding_work: 100000,
        }
    }
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicShoreMaterialSamples {
    owner: Arc<OwnedPeriodicNativeChartShores>,
    parameters: ParameterOrderLimits,
    limits: PeriodicShoreMaterialSampleLimits,
    samples: Vec<OriginalPeriodicShoreMaterialSample>,
    queries: usize,
}
struct Work {
    used: usize,
    cap: usize,
}
impl Work {
    fn charge(&mut self) -> Result<()> {
        self.used = self
            .used
            .checked_add(1)
            .filter(|n| *n <= self.cap)
            .ok_or_else(|| refused("Native shore material sample query allowance exhausted"))?;
        Ok(())
    }
}
fn inside(p: &[Q; 2], chart: &[[Q; 2]; 2]) -> bool {
    (0..2).all(|a| p[a] > chart[a][0] && p[a] < chart[a][1])
}
fn record(
    shore: usize,
    interval: usize,
    center: [Q; 2],
    probe: [Q; 2],
    origin: OriginalPeriodicShoreSampleOrigin,
    sample: &OwnedPeriodicMaterialSample,
) -> OriginalPeriodicShoreMaterialSample {
    OriginalPeriodicShoreMaterialSample {
        shore,
        interval,
        center: center.map(|q| ratio(&q)),
        probe: probe.map(|q| ratio(&q)),
        origin,
        windings: sample.windings().to_vec(),
        operand_inside: sample.operand_inside().to_vec(),
        selected: sample.selected(),
    }
}
fn derive(
    o: &OwnedPeriodicShoreMaterialSamples,
) -> Result<(Vec<OriginalPeriodicShoreMaterialSample>, usize)> {
    let _replay = super::periodic_replay::scope();
    let l = o.limits;
    if [
        l.max_intervals,
        l.max_queries,
        l.max_clearance_work,
        l.max_winding_work,
    ]
    .iter()
    .any(|n| !(1..=1000000).contains(n))
        || !(1..=512).contains(&l.max_levels)
        || o.parameters.max_refinements > 512
        || !(1..=128).contains(&o.parameters.max_resultant_size)
        || o.owner.interval_count() > l.max_intervals
    {
        return Err(refused("Invalid native shore material sample allowance"));
    }
    let labels = super::periodic_material::MaterialView::for_shores(&o.owner, o.parameters)?;
    let corners = o.owner.owner();
    let copies = corners.owner();
    let incidence = copies.owner();
    let traces = incidence.owner();
    let regions = traces.owner();
    let lifts = regions.owner().owner();
    let selection = lifts.owner().owner();
    let cover = selection.rule().support().augmented().surface().pieces();
    let chart = cover.chart().map(|d| d.map(|v| Q::from_float(v).unwrap()));
    let material = selection.rule().material();
    if !Arc::ptr_eq(labels.rule(), material) {
        return Err(refused(
            "Shore material batch belongs to a different actual rule",
        ));
    }
    let mut work = Work {
        used: 0,
        cap: l.max_queries,
    };
    let mut records = Vec::with_capacity(o.owner.interval_count());
    for (shore, s) in o.owner.shores().iter().enumerate() {
        for interval in 0..s.intervals.len() {
            let mut owned = None;
            // An existing source-side certificate handles a coincident carrier
            // without ever querying winding on the source or on the chart shore.
            for (row, side) in selection.sides().iter().enumerate() {
                work.charge()?;
                let probes = side.probes();
                let lifted = probes
                    .lifted_queries()
                    .iter()
                    .map(|p| Ok([decode_ratio(&p[0])?, decode_ratio(&p[1])?]))
                    .collect::<Result<Vec<_>>>()?;
                let center = std::array::from_fn::<_, 2, _>(|a| {
                    (&lifted[0][a] + &lifted[1][a]) / Q::from_integer(2.into())
                });
                let fixed = s.fixed_axis;
                let source_end = if center[fixed] == chart[fixed][0] {
                    Some(0)
                } else if center[fixed] == chart[fixed][1] {
                    Some(1)
                } else {
                    None
                };
                let Some(source_end) = source_end else {
                    continue;
                };
                if source_end != s.end && !cover.wrapped()[fixed] {
                    continue;
                }
                if !o
                    .owner
                    .interior_contains(shore, interval, &center[1 - fixed])?
                {
                    continue;
                }
                for k in 0..2 {
                    let delta = &lifted[k][fixed] - &center[fixed];
                    if (s.end == 0 && delta <= Q::zero()) || (s.end == 1 && delta >= Q::zero()) {
                        continue;
                    }
                    let mut transported = lifted[k].clone();
                    transported[fixed] += &chart[fixed][s.end] - &center[fixed];
                    if !inside(&transported, &chart) {
                        continue;
                    }
                    let Some(sample) = side.samples()[k].as_ref() else {
                        continue;
                    };
                    let actual_query = sample
                        .query()
                        .path()
                        .last()
                        .ok_or_else(|| refused("Owned shore sample lost its actual query"))?;
                    for axis in 0..2 {
                        if decode_ratio(&actual_query[axis])? != transported[axis] {
                            return Err(refused(
                                "Transported shore probe does not equal its actual canonical material query",
                            ));
                        }
                    }
                    // Transport of a selected material label requires the full
                    // actual Surface seam proof and descending operand periods,
                    // both already replayed by the support/selection owner.
                    let mut target_center = center.clone();
                    target_center[fixed] = chart[fixed][s.end].clone();
                    owned = Some(record(
                        shore,
                        interval,
                        target_center,
                        transported,
                        OriginalPeriodicShoreSampleOrigin::OwnedSide { row, side: k },
                        sample,
                    ));
                    break;
                }
                if owned.is_some() {
                    break;
                }
            }
            if let Some(sample) = owned {
                records.push(sample);
                continue;
            }
            let center = o.owner.interior_uv(shore, interval, l.max_levels)?;
            let center_dto = center.clone().map(|q| ratio(&q));
            work.charge()?;
            if !labels.clear(
                &[center_dto.clone(), center_dto.clone()],
                l.max_clearance_work,
            )? {
                return Err(refused(
                    "Shore sample lies on an original source and needs a locally rebound owned side probe",
                ));
            }
            let fixed = s.fixed_axis;
            let mut epsilon = (&chart[fixed][1] - &chart[fixed][0]) / Q::from_integer(2.into());
            let mut found = None;
            for _ in 0..l.max_levels {
                let mut probe = center.clone();
                if s.end == 0 {
                    probe[fixed] += &epsilon
                } else {
                    probe[fixed] -= &epsilon
                }
                if !inside(&probe, &chart) {
                    return Err(refused(
                        "Native shore normal sample escaped the strict original chart",
                    ));
                }
                let probe_dto = probe.clone().map(|q| ratio(&q));
                work.charge()?;
                if labels.clear(
                    &[center_dto.clone(), probe_dto.clone()],
                    l.max_clearance_work,
                )? {
                    work.charge()?;
                    let sample = labels.sample(
                        &[material.base().path()[0].clone(), probe_dto],
                        l.max_winding_work,
                    )?;
                    found = Some(record(
                        shore,
                        interval,
                        center.clone(),
                        probe,
                        OriginalPeriodicShoreSampleOrigin::ClearedNormalLeg,
                        &sample,
                    ));
                    break;
                }
                epsilon /= Q::from_integer(2.into());
            }
            records.push(found.ok_or_else(|| {
                refused("Native shore inside probe is unresolved within allowance")
            })?);
        }
    }
    if records.len() != o.owner.interval_count() {
        return Err(refused("Native shore material point census is incomplete"));
    }
    Ok((records, work.used))
}
impl OwnedPeriodicShoreMaterialSamples {
    pub fn owner(&self) -> &Arc<OwnedPeriodicNativeChartShores> {
        &self.owner
    }
    pub fn samples(&self) -> &[OriginalPeriodicShoreMaterialSample] {
        &self.samples
    }
    pub fn queries(&self) -> usize {
        self.queries
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (samples, queries) = derive(self)?;
        if samples != self.samples || queries != self.queries {
            return Err(refused(
                "Native shore material point label or source proof changed",
            ));
        }
        Ok(())
    }
}
/// One certified chart-inside POINT per native frame interval. Does not assert
/// material uniformity over the interval, local cycles, or physical face sewing.
pub fn own_periodic_shore_material_samples(
    owner: Arc<OwnedPeriodicNativeChartShores>,
    parameters: ParameterOrderLimits,
    limits: PeriodicShoreMaterialSampleLimits,
) -> Result<OwnedPeriodicShoreMaterialSamples> {
    let mut o = OwnedPeriodicShoreMaterialSamples {
        owner,
        parameters,
        limits,
        samples: Vec::new(),
        queries: 0,
    };
    let (samples, queries) = derive(&o)?;
    o.samples = samples;
    o.queries = queries;
    Ok(o)
}
#[cfg(test)]
mod tests {
    fn q(n: i64, d: i64) -> ExactRatio {
        [n.to_string(), d.to_string()]
    }
    use super::super::periodic_chart_shores::tests as frame;
    use super::super::periodic_support_regions::tests as fixtures;
    use super::*;
    fn samples(shores: OwnedPeriodicNativeChartShores) -> OwnedPeriodicShoreMaterialSamples {
        own_periodic_shore_material_samples(
            Arc::new(shores),
            ParameterOrderLimits::default(),
            PeriodicShoreMaterialSampleLimits::default(),
        )
        .unwrap()
    }
    fn qualify(o: &OwnedPeriodicShoreMaterialSamples) -> OwnedPeriodicChartCycles {
        let intervals = own_periodic_shore_material_intervals(
            Arc::new(o.clone()),
            ParameterOrderLimits::default(),
            PeriodicShoreMaterialIntervalLimits::default(),
        )
        .unwrap();
        for sample in o.samples() {
            assert_eq!(
                intervals.selected(sample.shore, sample.interval).unwrap(),
                sample.selected
            );
        }
        assert!(intervals.selected(99, 0).is_err());
        let winding = construct_periodic_chart_winding(
            Arc::new(intervals),
            PeriodicChartCycleLimits::default(),
            PeriodicChartWindingLimits::default(),
        )
        .unwrap();
        super::super::periodic_chart_cycle_sides::check_batch(&winding);
        for sample in o.samples() {
            let values = winding.windings(&sample.probe).unwrap().unwrap();
            assert_eq!(values.iter().sum::<i64>(), i64::from(sample.selected));
            assert_eq!(
                winding.material(&sample.probe).unwrap(),
                Some(sample.selected)
            );
        }
        let outside = [["-1".into(), "1".into()], ["1".into(), "2".into()]];
        assert!(
            winding
                .windings(&outside)
                .unwrap()
                .unwrap()
                .iter()
                .all(|&w| w == 0)
        );
        assert_eq!(winding.material(&outside).unwrap(), Some(false));
        if winding.owner().edges().len() == 4
            && winding.owner().edges().iter().all(|e| {
                matches!(
                    e.kind,
                    OriginalPeriodicChartCycleEdgeKind::NativeShore { .. }
                )
            })
        {
            let corner = o.owner().owner().corners()[0].uv.clone();
            assert_eq!(winding.windings(&corner).unwrap(), None);
            assert_eq!(winding.material(&corner).unwrap(), None);
        }
        if o.owner()
            .shores()
            .iter()
            .all(|s| s.role == OriginalPeriodicNativeShoreRole::NaturalSupport)
        {
            super::super::periodic_chart_winding::check_batch(&winding);
        }
        winding.owner().as_ref().clone()
    }
    fn labels(o: &OwnedPeriodicShoreMaterialSamples, shore: usize) -> Vec<bool> {
        o.samples()
            .iter()
            .filter(|p| p.shore == shore)
            .map(|p| p.selected)
            .collect()
    }
    #[test]
    fn actual_plane_point_labels_replay_and_refuse_cache_and_work_changes_and_complete_interval_germs()
     {
        let sig = fixtures::signatures(super::super::periodic_support_material::tests::full_plane(
            true,
        ));
        let shores = Arc::new(frame::shores(frame::corners(Arc::new(
            own_periodic_support_regions(sig, &[], 100000).unwrap(),
        ))));
        assert!(
            own_periodic_shore_material_samples(
                shores.clone(),
                ParameterOrderLimits::default(),
                PeriodicShoreMaterialSampleLimits {
                    max_intervals: 3,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(
            own_periodic_shore_material_samples(
                shores.clone(),
                ParameterOrderLimits::default(),
                PeriodicShoreMaterialSampleLimits {
                    max_queries: 1,
                    ..Default::default()
                }
            )
            .is_err()
        );
        let o = own_periodic_shore_material_samples(
            shores,
            ParameterOrderLimits::default(),
            PeriodicShoreMaterialSampleLimits::default(),
        )
        .unwrap();
        let chart = qualify(&o);
        super::super::periodic_chart_cycles::check_tampering(&chart);
        assert_eq!(chart.walks().len(), 1);
        assert_eq!(chart.edges().len(), 4);
        assert_eq!(chart.walks().iter().map(Vec::len).sum::<usize>(), 4);
        assert_eq!(o.samples().len(), 4);
        assert!(o.samples().iter().all(|p| p.selected
            && matches!(
                p.origin,
                OriginalPeriodicShoreSampleOrigin::OwnedSide { .. }
            )));
        let counterfeit = frame::corrupt_direction(o.owner());
        assert!(
            super::super::periodic_material::MaterialView::for_shores(
                &counterfeit,
                ParameterOrderLimits::default()
            )
            .is_err()
        );
        let q = |n: i64, d: i64| Q::new(n.into(), d.into());
        let f = vec![Q::zero(), Q::one()];
        let root = [Q::zero(), Q::zero()];
        assert!(
            o.owner()
                .admit_cut(0, &f, &root, &[vec![q(1, 2)], vec![Q::one()]])
                .is_err()
        );
        assert_eq!(
            o.owner()
                .admit_cut(0, &f, &root, &[vec![Q::zero()], vec![Q::one()]])
                .unwrap(),
            Some(0)
        );
        assert_eq!(
            o.owner()
                .admit_cut(0, &f, &root, &[vec![q(-1, 1)], vec![Q::one()]])
                .unwrap(),
            None
        );
        assert!(
            own_periodic_shore_material_intervals(
                Arc::new(o.clone()),
                ParameterOrderLimits::default(),
                PeriodicShoreMaterialIntervalLimits {
                    max_contacts: 0,
                    ..Default::default()
                }
            )
            .is_err()
        );
        o.reverify_source().unwrap();
        let mut bad = o.clone();
        bad.samples[0].selected = false;
        assert!(bad.reverify_source().is_err());
    }
    #[test]
    fn actual_cylinder_bands_label_both_periodic_sides_and_keep_natural_exterior_empty_and_complete_interval_germs()
     {
        let sig = Arc::new(
            own_periodic_support_signatures(
                fixtures::cylinder_bands(),
                ParameterOrderLimits::default(),
                10000,
            )
            .unwrap(),
        );
        let r = construct_periodic_support_regions(
            sig,
            ParameterOrderLimits::default(),
            PeriodicRegionSearchLimits::default(),
        )
        .unwrap();
        assert_eq!(r.stop(), None);
        let o = samples(frame::shores(frame::corners(Arc::new(
            r.regions().unwrap().clone(),
        ))));
        let chart = qualify(&o);
        assert_eq!(chart.walks().len(), 2);
        assert_eq!(chart.edges().len(), 8);
        assert_eq!(chart.walks().iter().map(Vec::len).sum::<usize>(), 8);
        assert_eq!(o.samples().len(), 12);
        assert_eq!(labels(&o, 0), vec![false, true, false, true, false]);
        assert_eq!(labels(&o, 1), labels(&o, 0));
        assert_eq!(labels(&o, 2), vec![false]);
        assert_eq!(labels(&o, 3), vec![false]);
        assert!(
            o.samples()
                .iter()
                .filter(|p| p.shore < 2)
                .all(|p| p.origin == OriginalPeriodicShoreSampleOrigin::ClearedNormalLeg)
        );
    }
    #[test]
    fn actual_one_sided_coincident_rectangle_has_different_chart_inside_labels_across_seam_and_complete_interval_germs()
     {
        let o = samples(frame::shores(frame::cylinder_profile(false)));
        let chart = qualify(&o);
        assert_eq!(chart.walks().len(), 1);
        assert_eq!(chart.edges().len(), 4);
        assert_eq!(chart.walks().iter().map(Vec::len).sum::<usize>(), 4);
        assert_eq!(o.samples().len(), 8);
        assert_eq!(labels(&o, 0), vec![false, true, false]);
        assert_eq!(labels(&o, 1), vec![false, false, false]);
        for shore in 0..2 {
            let p = o
                .samples()
                .iter()
                .find(|p| p.shore == shore && p.interval == 1)
                .unwrap();
            assert!(matches!(
                p.origin,
                OriginalPeriodicShoreSampleOrigin::OwnedSide { .. }
            ));
        }
    }
    #[test]
    fn actual_irrational_quadratic_contacts_have_strict_inside_point_labels_without_rounded_cuts_and_complete_interval_germs()
     {
        let o = samples(frame::shores(frame::cylinder_profile(true)));
        let chart = qualify(&o);
        assert_eq!(chart.walks().len(), 2);
        assert_eq!(chart.edges().len(), 6);
        assert_eq!(chart.walks().iter().map(Vec::len).sum::<usize>(), 6);
        assert_eq!(o.samples().len(), 8);
        assert_eq!(labels(&o, 0), vec![false, true, false]);
        assert_eq!(labels(&o, 1), labels(&o, 0));
        assert!(
            o.samples()
                .iter()
                .filter(|p| p.shore < 2)
                .all(|p| p.origin == OriginalPeriodicShoreSampleOrigin::ClearedNormalLeg)
        );
    }
    #[test]
    fn actual_zero_source_torus_point_labels_use_no_invented_sources_and_complete_interval_germs() {
        let surface = nurbs_core::primitives::torus([0.; 3], 3., 1., 1.).unwrap();
        let input = super::super::periodic_support_boundaries::tests::original(
            &surface,
            [true, true],
            &[],
            &[],
        );
        let support = Arc::new(
            own_periodic_support_boundaries(
                input,
                1e-12,
                ParameterOrderLimits::default(),
                64,
                100000,
            )
            .unwrap(),
        );
        let jumps = super::super::periodic_support_boundaries::tests::graph_with_pair_limits(
            support.augmented().clone(),
            OriginalAtlasPairLimits {
                max_span_work: 1000000,
                refinement_depth: 4,
                ..Default::default()
            },
        );
        let a = [q(1, 2), q(1, 2)];
        let base = Arc::new(
            own_periodic_relative_winding(
                jumps,
                &[a.clone(), a],
                ParameterOrderLimits::default(),
                1000,
            )
            .unwrap(),
        );
        let rule = Arc::new(
            own_periodic_support_material_rule(
                support,
                base,
                &[OriginalPeriodicOperand {
                    wire_weights: vec![],
                    base_winding: "1".into(),
                    fill: OriginalPeriodicFillRule::NonZero,
                }],
                &[OriginalPeriodicBooleanOp::Operand(0)],
            )
            .unwrap(),
        );
        let sig =
            fixtures::signatures(Arc::new(own_periodic_support_selection(rule, &[]).unwrap()));
        let o = frame::shores(frame::corners(Arc::new(
            own_periodic_support_regions(sig, &[], 10000).unwrap(),
        )));
        let o = samples(o);
        let chart = qualify(&o);
        assert_eq!(chart.walks().len(), 1);
        assert_eq!(chart.edges().len(), 4);
        assert_eq!(chart.walks().iter().map(Vec::len).sum::<usize>(), 4);
        assert_eq!(o.samples().len(), 4);
        assert!(o.samples().iter().all(|p| p.selected
            && p.origin == OriginalPeriodicShoreSampleOrigin::ClearedNormalLeg
            && p.windings == ["1"]));
    }
}
