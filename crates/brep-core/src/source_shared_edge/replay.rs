//! Fresh composition and endpoint proof from typed original mapping inputs.
use super::*;
pub struct ReplayLimits {
    /// Independently bounded root mapping for each original use.
    pub mapping_cells_per_use: usize,
    pub exact_work: u64,
    pub driver_cells: usize,
}
impl SharedEdge {
    pub fn requalify(&self, limits: ReplayLimits) -> Result<Report> {
        self.requalify_with_uses(self.uses.each_ref(), limits)
    }
    /// Substitute source uses (for root refinement), then replay every original
    /// root, correspondence and endpoint gate. No cached certificate is reused.
    pub fn requalify_with_uses(
        &self,
        fragments: [&Fragment; 2],
        limits: ReplayLimits,
    ) -> Result<Report> {
        if !(1..=100000).contains(&limits.mapping_cells_per_use)
            || !(1..=100_000_000).contains(&limits.exact_work)
            || limits.driver_cells > 100000
        {
            return Err(Error::new(
                "BREP_SOURCE_EDGE_RESTORE",
                "Invalid original shared-edge definition or budget",
            ));
        }
        let uses = [
            fragments[0].requalify(limits.mapping_cells_per_use)?,
            fragments[1].requalify(limits.mapping_cells_per_use)?,
        ];
        match &self.recipe {
            Recipe::Direct {
                world_reversed,
                cutters,
                planes,
            } => qualify_with_cutters_and_planes(
                &self.world,
                uses.each_ref(),
                *world_reversed,
                cutters.each_ref().map(Option::as_ref),
                *planes,
                limits.exact_work,
                limits.driver_cells,
            ),
            Recipe::Mapped {
                ranges,
                planes,
                candidates,
            } => crate::source_mapped_edge::qualify_with_candidates(
                &self.world,
                uses.each_ref(),
                *ranges,
                *planes,
                *candidates,
                limits.exact_work,
                limits.driver_cells,
            ),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use nurbs_core::surface::Surface;
    fn fixture(mapped: bool) -> SharedEdge {
        let surface = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let uv = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        let world = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
        let a = Fragment::new(
            &surface,
            &uv,
            Endpoint::Parameter(0.),
            Endpoint::Parameter(1.),
        )
        .unwrap();
        let b = Fragment::new(
            &surface,
            &uv,
            Endpoint::Parameter(1.),
            Endpoint::Parameter(0.),
        )
        .unwrap();
        let r = if mapped {
            crate::source_mapped_edge::qualify(
                &world,
                [&a, &b],
                [[[0., 1.], [1., 1.]]; 2],
                [None; 2],
                100000000,
                0,
            )
        } else {
            qualify(&world, [&a, &b], [false; 2], 100000000)
        }
        .unwrap();
        r.edge.expect(r.reason)
    }
    fn limits(work: u64) -> ReplayLimits {
        ReplayLimits {
            mapping_cells_per_use: 10000,
            exact_work: work,
            driver_cells: 10000,
        }
    }
    #[test]
    fn direct_and_mapped_native_replay_match_wire_work_and_ignore_cached_maps() {
        for mapped in [false, true] {
            let mut edge = fixture(mapped);
            let original = edge.clone();
            edge.reversed = [true; 2];
            edge.ranges = Some([[[99.; 2]; 2]; 2]);
            for work in [1, 100000000] {
                let native = edge.requalify(limits(work)).unwrap();
                let pristine = original.requalify(limits(work)).unwrap();
                assert_eq!(
                    (
                        native.work_used,
                        native.driver_cells,
                        native.root_checks,
                        native.reason
                    ),
                    (
                        pristine.work_used,
                        pristine.driver_cells,
                        pristine.root_checks,
                        pristine.reason
                    )
                );
                assert_eq!(native.edge.is_some(), pristine.edge.is_some());
                if let (Some(a), Some(b)) = (&native.edge, &pristine.edge) {
                    assert!(a.same_definition(b));
                }
                #[cfg(feature = "codec")]
                {
                    let wire = crate::source_shared_edge_restore::restore(
                        original.definition(),
                        limits(work),
                    )
                    .unwrap();
                    assert_eq!(
                        (
                            native.work_used,
                            native.driver_cells,
                            native.root_checks,
                            native.reason
                        ),
                        (
                            wire.work_used,
                            wire.driver_cells,
                            wire.root_checks,
                            wire.reason
                        )
                    );
                    assert_eq!(
                        native.edge.as_ref().map(SharedEdge::definition),
                        wire.edge.as_ref().map(SharedEdge::definition)
                    );
                }
                if let Some(fresh) = native.edge {
                    assert_ne!(fresh.reversed(), edge.reversed());
                }
            }
            edge.world.control_points[1][2] = 1.;
            assert!(edge.requalify(limits(100000000)).unwrap().edge.is_none());
            assert!(edge.requalify(limits(0)).is_err());
        }
    }
}
