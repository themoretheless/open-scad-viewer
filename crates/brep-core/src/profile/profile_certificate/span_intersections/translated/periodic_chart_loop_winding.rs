//! Exact winding of decomposed ORIGINAL chart edges; no patch authority.
use super::*;
use crate::profile_certificate::algebraic_intersections::decode_ratio;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct OwnedPeriodicChartLoopWinding {
    owner: Arc<OwnedPeriodicChartBoundaryLoops>,
    winding: OwnedPeriodicChartWinding,
    limits: PeriodicChartWindingLimits,
}
impl OwnedPeriodicChartLoopWinding {
    pub fn owner(&self) -> &Arc<OwnedPeriodicChartBoundaryLoops> {
        &self.owner
    }
    /// None proves contact with this original-edge loop. No period reduction.
    pub fn winding(&self, boundary: usize, point: &[ExactRatio; 2]) -> Result<Option<i64>> {
        let boundary = self
            .owner
            .loops()
            .get(boundary)
            .ok_or_else(|| refused("Unknown original chart boundary loop"))?;
        self.winding.query_edges(
            &boundary.edges,
            &[decode_ratio(&point[0])?, decode_ratio(&point[1])?],
        )
    }
    /// Complete loop vector; errors never become an exterior or boundary label.
    pub fn windings(&self, point: &[ExactRatio; 2]) -> Result<Option<Vec<i64>>> {
        if self.owner.edge_locations().len() > self.limits.max_query_edges {
            return Err(refused("Complete decomposed loop query exceeds allowance"));
        }
        let p = [decode_ratio(&point[0])?, decode_ratio(&point[1])?];
        let mut values = Vec::with_capacity(self.owner.loops().len());
        for boundary in self.owner.loops() {
            let Some(v) = self.winding.query_edges(&boundary.edges, &p)? else {
                return Ok(None);
            };
            values.push(v);
        }
        Ok(Some(values))
    }
    pub fn material(&self, point: &[ExactRatio; 2]) -> Result<Option<bool>> {
        let Some(values) = self.windings(point)? else {
            return Ok(None);
        };
        let total = values.into_iter().try_fold(0i64, |a, b| {
            a.checked_add(b)
                .ok_or_else(|| refused("Decomposed chart winding overflow"))
        })?;
        match total {
            0 => Ok(Some(false)),
            1 => Ok(Some(true)),
            _ => Err(refused(
                "Decomposed loops do not yield a regularized material characteristic",
            )),
        }
    }
    pub fn reverify_source(&self) -> Result<()> {
        let _replay = super::periodic_replay::scope();
        if !Arc::ptr_eq(self.owner.owner(), self.winding.owner()) {
            return Err(refused(
                "Loop winding belongs to another original chart owner",
            ));
        }
        super::periodic_replay::verify_arc(&self.owner, |owner| owner.reverify_source())?;
        self.winding.reverify_source()
    }
}
pub fn own_periodic_chart_loop_winding(
    owner: Arc<OwnedPeriodicChartBoundaryLoops>,
    limits: PeriodicChartWindingLimits,
) -> Result<OwnedPeriodicChartLoopWinding> {
    let _replay = super::periodic_replay::scope();
    super::periodic_replay::verify_arc(&owner, |owner| owner.reverify_source())?;
    let winding = own_periodic_chart_winding(owner.owner().clone(), limits)?;
    Ok(OwnedPeriodicChartLoopWinding {
        owner,
        winding,
        limits,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn q(x: i32, y: i32, denominator: i32) -> [ExactRatio; 2] {
        [x, y].map(|v| {
            crate::profile_certificate::algebraic_intersections::ratio(&Q::new(
                v.into(),
                denominator.into(),
            ))
        })
    }
    fn check(original: OwnedPeriodicChartWinding, queries: Vec<([ExactRatio; 2], Option<bool>)>) {
        let _replay = super::super::periodic_replay::scope();
        let loops = Arc::new(
            own_periodic_chart_boundary_loops(original.owner().clone(), Default::default())
                .unwrap(),
        );
        let o = own_periodic_chart_loop_winding(loops.clone(), Default::default()).unwrap();
        assert_eq!(o.owner().loops().len(), 2);
        for (p, expected) in queries {
            assert_eq!(o.material(&p).unwrap(), expected);
            assert_eq!(o.material(&p).unwrap(), original.material(&p).unwrap());
            if let Some(values) = o.windings(&p).unwrap() {
                let old = original.windings(&p).unwrap().unwrap();
                for (walk, &value) in old.iter().enumerate() {
                    let total: i64 = o
                        .owner()
                        .loops()
                        .iter()
                        .zip(&values)
                        .filter(|(l, _)| l.source_walk == walk)
                        .map(|(_, v)| *v)
                        .sum();
                    assert_eq!(total, value);
                }
            }
        }
        assert!(o.winding(usize::MAX, &q(1, 1, 4)).is_err());
        assert!(
            o.material(&[["2".into(), "4".into()], ["1".into(), "4".into()]])
                .is_err()
        );
        let low = own_periodic_chart_loop_winding(
            loops,
            PeriodicChartWindingLimits {
                max_query_edges: 1,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(low.material(&q(1, 1, 4)).is_err());
        o.reverify_source().unwrap();
    }
    #[test]
    fn actual_touching_components_preserve_each_original_cycle_winding() {
        let w = super::super::periodic_chart_cycles::actual_fan_tests::build(
            super::super::periodic_support_regions::tests::discs(true),
            2,
        );
        check(
            w,
            vec![
                (q(2, 2, 8), Some(true)),
                (q(6, 6, 8), Some(true)),
                (q(2, 6, 8), Some(false)),
                (q(4, 4, 8), None),
            ],
        );
    }
    #[test]
    fn actual_tangent_hole_keeps_negative_inner_loop_and_boundary_contacts() {
        let w = super::super::periodic_chart_cycles::actual_fan_tests::build(
            super::super::periodic_support_material::tests::tangent_hole(),
            1,
        );
        check(
            w,
            vec![
                (q(2, 1, 4), Some(false)),
                (q(0, 2, 4), None),
                (q(-4, 1, 4), Some(false)),
                (q(2, 0, 4), None),
            ],
        );
    }
}
