//! Genuine native-domain layout corners, distinct from authored material trim.
use super::*;
use crate::profile_certificate::algebraic_intersections::ratio;
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicNativeChartCorner {
    pub ends: [usize; 2],
    pub uv: [ExactRatio; 2],
    pub world: [ExactRatio; 3],
    pub existing_copy: Option<usize>,
    pub physical_vertex: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicNativeCornerVertex {
    /// Wrapped ends are0; unwrapped ends retain their actual native side.
    pub quotient_key: [usize; 2],
    /// ORIGINAL graph vertex ID, not an index in selected incidence.vertices.
    pub source_vertex: Option<usize>,
    pub corners: Vec<usize>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Data {
    corners: Vec<OriginalPeriodicNativeChartCorner>,
    vertices: Vec<OriginalPeriodicNativeCornerVertex>,
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicNativeChartCorners {
    owner: Arc<OwnedPeriodicSourceChartCopies>,
    max_work_per_corner: usize,
    data: Data,
}
fn derive(o: &OwnedPeriodicNativeChartCorners) -> Result<Data> {
    let _replay = super::periodic_replay::scope();
    if !(1..=1000000).contains(&o.max_work_per_corner) {
        return Err(refused("Invalid exact native corner work allowance"));
    }
    o.owner.reverify_source()?;
    let incidence = o.owner.owner();
    let traces = incidence.owner();
    let regions = traces.owner();
    let lifts = regions.owner().owner();
    let cycles = lifts.owner();
    let source = cycles.owner().rule().support().augmented().surface();
    let cover = source.pieces();
    let wrapped = cover.wrapped();
    for axis in 0..2 {
        if wrapped[axis] && source.seam_proofs()[axis].is_none() {
            return Err(refused(
                "Native corner identification lacks actual Surface seam identity",
            ));
        }
    }
    let chart = cover.chart().map(|d| d.map(|v| Q::from_float(v).unwrap()));
    let layout = [[0usize, 0usize], [1, 0], [1, 1], [0, 1]];
    let key =
        |ends: [usize; 2]| std::array::from_fn(|axis| if wrapped[axis] { 0 } else { ends[axis] });
    let mut existing = [None; 4];
    let mut sources = BTreeMap::new();
    let mut source_keys = BTreeMap::new();
    for (copy, c) in o.owner.copies().iter().enumerate() {
        let [Some(u), Some(v)] = c.bounds else {
            continue;
        };
        let ends = [u, v];
        let i = layout
            .iter()
            .position(|e| *e == ends)
            .ok_or_else(|| refused("Original corner copy has invalid native side flags"))?;
        if existing[i].replace(copy).is_some() {
            return Err(refused(
                "Original source contains multiple canonical copies at one native corner",
            ));
        }
        let k = key(ends);
        if sources
            .insert(k, c.source_vertex)
            .is_some_and(|old| old != c.source_vertex)
        {
            return Err(refused(
                "Original corner quotient has distinct unproved source vertices",
            ));
        }
        if source_keys
            .insert(c.source_vertex, k)
            .is_some_and(|old| old != k)
        {
            return Err(refused(
                "Original corner identity changes an unwrapped native side",
            ));
        }
    }
    let mut vertex_map = BTreeMap::new();
    let mut vertices: Vec<OriginalPeriodicNativeCornerVertex> = Vec::new();
    let mut corners: Vec<OriginalPeriodicNativeChartCorner> = Vec::with_capacity(4);
    for (i, ends) in layout.into_iter().enumerate() {
        let k = key(ends);
        let physical_vertex = if let Some(&v) = vertex_map.get(&k) {
            v
        } else {
            let v = vertices.len();
            vertex_map.insert(k, v);
            vertices.push(OriginalPeriodicNativeCornerVertex {
                quotient_key: k,
                source_vertex: sources.get(&k).copied(),
                corners: Vec::new(),
            });
            v
        };
        let uv = [chart[0][ends[0]].clone(), chart[1][ends[1]].clone()];
        // A rational native corner is evaluated in the exact alpha=0 field.
        // This calls pure arithmetic after ONE fresh immutable owner replay.
        let value = arithmetic::surface_expression::evaluate(
            source.surface(),
            &vec![Q::zero(), Q::one()],
            &[Q::zero(), Q::zero()],
            &[
                [vec![uv[0].clone()], vec![Q::one()]],
                [vec![uv[1].clone()], vec![Q::one()]],
            ],
            128,
            o.max_work_per_corner,
        )?;
        let mut world = [Q::zero(), Q::zero(), Q::zero()];
        for axis in 0..3 {
            if value[axis].iter().any(|p| p.len() > 1) {
                return Err(refused(
                    "Native rational corner did not remain in its exact constant field",
                ));
            }
            let n = value[axis][0].first().cloned().unwrap_or_else(Q::zero);
            let d = value[axis][1].first().cloned().unwrap_or_else(Q::zero);
            if d <= Q::zero() {
                return Err(refused(
                    "Native rational corner has a nonpositive exact denominator",
                ));
            }
            world[axis] = n / d;
        }
        // Equality is a consistency check AFTER whole-seam source authority;
        // it is never the criterion for identifying physical vertices.
        if let Some(&other) = vertices[physical_vertex].corners.first() {
            if corners[other].world != world.clone().map(|q| ratio(&q)) {
                return Err(refused("Actual native seam corner values disagree"));
            }
        }
        vertices[physical_vertex].corners.push(i);
        corners.push(OriginalPeriodicNativeChartCorner {
            ends,
            uv: uv.map(|q| ratio(&q)),
            world: world.map(|q| ratio(&q)),
            existing_copy: existing[i],
            physical_vertex,
        });
    }
    Ok(Data { corners, vertices })
}
impl OwnedPeriodicNativeChartCorners {
    pub fn owner(&self) -> &Arc<OwnedPeriodicSourceChartCopies> {
        &self.owner
    }
    pub fn corners(&self) -> &[OriginalPeriodicNativeChartCorner] {
        &self.data.corners
    }
    pub fn vertices(&self) -> &[OriginalPeriodicNativeCornerVertex] {
        &self.data.vertices
    }
    pub fn reverify_source(&self) -> Result<()> {
        if derive(self)? != self.data {
            return Err(refused(
                "Native corner source,exact coordinates or seam quotient changed",
            ));
        }
        Ok(())
    }
}
pub fn own_periodic_native_chart_corners(
    owner: Arc<OwnedPeriodicSourceChartCopies>,
    max_work_per_corner: usize,
) -> Result<OwnedPeriodicNativeChartCorners> {
    let mut o = OwnedPeriodicNativeChartCorners {
        owner,
        max_work_per_corner,
        data: Data {
            corners: Vec::new(),
            vertices: Vec::new(),
        },
    };
    o.data = derive(&o)?;
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::super::periodic_support_regions::tests as fixtures;
    use super::*;
    fn copies(regions: Arc<OwnedPeriodicSupportRegions>) -> Arc<OwnedPeriodicSourceChartCopies> {
        let traces = Arc::new(
            own_periodic_support_traces(regions, ParameterOrderLimits::default(), 1000).unwrap(),
        );
        let incidence = Arc::new(own_periodic_support_incidence(traces, 1000).unwrap());
        Arc::new(
            own_periodic_source_chart_copies(
                incidence,
                ParameterOrderLimits::default(),
                PeriodicSourceChartCopyLimits::default(),
            )
            .unwrap(),
        )
    }
    fn q(n: i64) -> ExactRatio {
        [n.to_string(), "1".into()]
    }
    #[test]
    fn native_chart_corners_preserve_actual_plane_ids_and_reject_geometry_or_quotient_forgery() {
        let signatures = fixtures::signatures(
            super::super::periodic_support_material::tests::full_plane(true),
        );
        let source = copies(Arc::new(
            own_periodic_support_regions(signatures, &[], 100000).unwrap(),
        ));
        assert!(own_periodic_native_chart_corners(source.clone(), 1).is_err());
        let o = own_periodic_native_chart_corners(source, 100000).unwrap();
        assert_eq!((o.corners().len(), o.vertices().len()), (4, 4));
        assert!(
            o.vertices()
                .iter()
                .all(|v| v.source_vertex.is_some() && v.corners.len() == 1)
        );
        for c in o.corners() {
            assert!(c.existing_copy.is_some());
            assert_eq!(c.world, [q(c.ends[0] as i64), q(c.ends[1] as i64), q(0)]);
        }
        let mut changed = o.clone();
        changed.data.corners[0].world[0] = q(999);
        assert!(changed.reverify_source().is_err());
        let mut changed = o.clone();
        changed.data.corners[0].existing_copy = None;
        assert!(changed.reverify_source().is_err());
        let mut changed = o.clone();
        changed.data.vertices[0].quotient_key = [1, 1];
        assert!(changed.reverify_source().is_err());
        let mut changed = o.clone();
        changed.data.vertices[0].source_vertex = Some(usize::MAX);
        assert!(changed.reverify_source().is_err());
    }
    #[test]
    fn native_chart_corners_use_actual_cylinder_seams_and_original_graph_vertex_namespace() {
        let signatures = Arc::new(
            own_periodic_support_signatures(
                fixtures::cylinder_bands(),
                ParameterOrderLimits::default(),
                10000,
            )
            .unwrap(),
        );
        let search = construct_periodic_support_regions(
            signatures,
            ParameterOrderLimits::default(),
            PeriodicRegionSearchLimits::default(),
        )
        .unwrap();
        assert_eq!(search.stop(), None);
        let o = own_periodic_native_chart_corners(
            copies(Arc::new(search.regions().unwrap().clone())),
            100000,
        )
        .unwrap();
        assert_eq!((o.corners().len(), o.vertices().len()), (4, 2));
        assert!(
            o.vertices()
                .iter()
                .all(|v| v.source_vertex.is_some() && v.corners.len() == 2)
        );
        let selected = o
            .owner()
            .owner()
            .vertices()
            .iter()
            .map(|v| v.source_vertex)
            .collect::<Vec<_>>();
        assert!(
            o.vertices()
                .iter()
                .all(|v| !selected.contains(&v.source_vertex.unwrap()))
        );
        for c in o.corners() {
            assert!(c.existing_copy.is_some());
            assert_eq!(c.world, [q(0), q(0), q(c.ends[1] as i64)]);
        }
    }
    #[test]
    fn native_chart_corners_anchor_an_actual_full_torus_without_fake_original_trim_sources() {
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
        let a = [["1".into(), "2".into()], ["1".into(), "2".into()]];
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
        let signatures =
            fixtures::signatures(Arc::new(own_periodic_support_selection(rule, &[]).unwrap()));
        let regions = Arc::new(own_periodic_support_regions(signatures, &[], 10000).unwrap());
        assert_eq!(regions.components().len(), 1);
        assert!(regions.components()[0].boundary_walks.is_empty());
        let source = copies(regions);
        assert!(source.copies().is_empty());
        let o = own_periodic_native_chart_corners(source, 100000).unwrap();
        assert_eq!((o.corners().len(), o.vertices().len()), (4, 1));
        assert_eq!(o.vertices()[0].source_vertex, None);
        assert_eq!(o.vertices()[0].quotient_key, [0, 0]);
        assert_eq!(o.vertices()[0].corners, vec![0, 1, 2, 3]);
        for c in o.corners() {
            assert_eq!(c.existing_copy, None);
            assert_eq!(c.world, [q(4), q(0), q(0)]);
        }
        let source = o.owner().owner().owner().owner();
        assert!(source.owner().owner().walks().is_empty());
    }
}
