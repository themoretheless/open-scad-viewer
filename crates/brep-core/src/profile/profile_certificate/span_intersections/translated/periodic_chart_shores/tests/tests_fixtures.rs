    use super::super::periodic_support_regions::tests as fixtures;
    use super::*;
    pub(in super::super) fn corrupt_direction(
        o: &OwnedPeriodicNativeChartShores,
    ) -> OwnedPeriodicNativeChartShores {
        let mut bad = o.clone();
        bad.data.shores[0].direction *= -1;
        bad
    }
    fn q(n: i64, d: i64) -> ExactRatio {
        [n.to_string(), d.to_string()]
    }
    pub(in super::super) fn corners(
        regions: Arc<OwnedPeriodicSupportRegions>,
    ) -> Arc<OwnedPeriodicNativeChartCorners> {
        let _replay = super::super::periodic_replay::scope();
        let traces = Arc::new(
            own_periodic_support_traces(regions, ParameterOrderLimits::default(), 1000).unwrap(),
        );
        let incidence = Arc::new(own_periodic_support_incidence(traces, 1000).unwrap());
        let copies = Arc::new(
            own_periodic_source_chart_copies(
                incidence,
                ParameterOrderLimits::default(),
                PeriodicSourceChartCopyLimits::default(),
            )
            .unwrap(),
        );
        Arc::new(own_periodic_native_chart_corners(copies, 100000).unwrap())
    }
    pub(in super::super) fn shores(
        c: Arc<OwnedPeriodicNativeChartCorners>,
    ) -> OwnedPeriodicNativeChartShores {
        own_periodic_native_chart_shores(
            c,
            ParameterOrderLimits::default(),
            PeriodicNativeShoreLimits::default(),
        )
        .unwrap()
    }
    #[test]
    fn native_chart_shores_order_actual_plane_sides_and_refuse_cache_or_budget_changes() {
        let sig = fixtures::signatures(super::super::periodic_support_material::tests::full_plane(
            true,
        ));
        let c = corners(Arc::new(
            own_periodic_support_regions(sig, &[], 100000).unwrap(),
        ));
        let limits = PeriodicNativeShoreLimits::default();
        assert!(
            own_periodic_native_chart_shores(
                c.clone(),
                ParameterOrderLimits::default(),
                PeriodicNativeShoreLimits {
                    max_endpoint_uses: 1,
                    ..limits
                }
            )
            .is_err()
        );
        assert!(
            own_periodic_native_chart_shores(
                c.clone(),
                ParameterOrderLimits::default(),
                PeriodicNativeShoreLimits {
                    max_intervals: 3,
                    ..limits
                }
            )
            .is_err()
        );
        assert!(
            own_periodic_native_chart_shores(
                c.clone(),
                ParameterOrderLimits::default(),
                PeriodicNativeShoreLimits {
                    max_comparisons: 1,
                    ..limits
                }
            )
            .is_err()
        );
        let o = shores(c);
        assert_eq!(o.interval_count(), 4);
        assert_eq!(o.shores().len(), 4);
        assert!(o.shores().iter().all(|s| s.role
            == OriginalPeriodicNativeShoreRole::NaturalSupport
            && s.points.len() == 2));
        assert_eq!(
            o.shores().iter().map(|s| s.direction).collect::<Vec<_>>(),
            vec![-1, 1, 1, -1]
        );
        assert_eq!(
            o.point(0, 0, &q(1, 2)).unwrap(),
            [q(0, 1), q(1, 2), q(0, 1)]
        );
        assert!(o.point(0, 0, &q(2, 1)).is_err());
        let mut changed = o.clone();
        changed.data.shores[0].role = OriginalPeriodicNativeShoreRole::AuxiliarySeam;
        assert!(changed.reverify_source().is_err());
        let mut changed = o.clone();
        changed.data.shores[0].points[0] = OriginalPeriodicNativeShorePoint::Corner(usize::MAX);
        assert!(changed.reverify_source().is_err());
        let mut changed = o.clone();
        changed.data.shores[0].intervals[0].swap(0, 1);
        assert!(changed.reverify_source().is_err());
    }
    #[test]
    fn native_chart_shores_partition_actual_cylinder_bands_in_original_surface_clocks() {
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
        let o = shores(corners(Arc::new(r.regions().unwrap().clone())));
        assert_eq!(o.interval_count(), 12);
        for s in o.shores() {
            assert_eq!(s.intervals.len(), if s.fixed_axis == 0 { 5 } else { 1 });
            assert_eq!(
                s.role,
                if s.fixed_axis == 0 {
                    OriginalPeriodicNativeShoreRole::AuxiliarySeam
                } else {
                    OriginalPeriodicNativeShoreRole::NaturalSupport
                }
            );
        }
        assert_eq!(
            o.point(2, 0, &q(3, 2)).unwrap(),
            [q(1, 1), q(1, 2), q(0, 1)]
        );
    }
    pub(in super::super) fn cylinder_profile(curved: bool) -> Arc<OwnedPeriodicNativeChartCorners> {
        let _replay = super::super::periodic_replay::scope();
        let rim = Curve::from_polyline(vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![1., 1., 0.],
            vec![0., 1., 0.],
            vec![0., 0., 0.],
        ])
        .unwrap();
        let surface = nurbs_core::surface::extrude(&rim, [0., 0., 1.]).unwrap();
        let curves = if curved {
            vec![
                Curve {
                    degree: 2,
                    knots: vec![0., 0., 0., 1., 1., 1.],
                    control_points: vec![vec![0.125, 0.25], vec![-0.375, 0.5], vec![0.125, 0.75]],
                    weights: vec![1.; 3],
                    periodic: false,
                },
                Curve::from_polyline(vec![vec![0.125, 0.75], vec![0.125, 0.25]]).unwrap(),
            ]
        } else {
            vec![
                Curve::from_polyline(vec![
                    vec![0., 0.25],
                    vec![0.5, 0.25],
                    vec![0.5, 0.75],
                    vec![0., 0.75],
                    vec![0., 0.25],
                ])
                .unwrap(),
            ]
        };
        let uses = vec![
            (0..curves.len())
                .map(|source| OriginalPeriodicSourceUse {
                    source,
                    reversed: false,
                })
                .collect(),
        ];
        let input = super::super::periodic_support_boundaries::tests::original(
            &surface,
            [true, false],
            &curves,
            &uses,
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
        let jumps =
            super::super::periodic_support_boundaries::tests::graph(support.augmented().clone());
        let a = [q(2, 1), q(1, 2)];
        let base = Arc::new(
            own_periodic_relative_winding(
                jumps.clone(),
                &[a.clone(), a.clone()],
                ParameterOrderLimits::default(),
                10000,
            )
            .unwrap(),
        );
        let rule = Arc::new(
            own_periodic_support_material_rule(
                support.clone(),
                base,
                &[OriginalPeriodicOperand {
                    wire_weights: vec![1],
                    base_winding: "0".into(),
                    fill: OriginalPeriodicFillRule::NonZero,
                }],
                &[OriginalPeriodicBooleanOp::Operand(0)],
            )
            .unwrap(),
        );
        let mut sides = Vec::new();
        for edge in 0..jumps.owner().owner().edges().len() {
            let p = Arc::new(
                own_periodic_support_side_probes(
                    support.clone(),
                    jumps.clone(),
                    edge,
                    q(1, 1024),
                    ParameterOrderLimits::default(),
                )
                .unwrap(),
            );
            let samples = std::array::from_fn(|s| {
                if p.exterior()[s] {
                    None
                } else {
                    Some(Arc::new(
                        own_periodic_material_sample(
                            rule.material().clone(),
                            Arc::new(
                                own_periodic_relative_winding(
                                    jumps.clone(),
                                    &[a.clone(), p.queries()[s].clone()],
                                    ParameterOrderLimits::default(),
                                    10000,
                                )
                                .unwrap(),
                            ),
                        )
                        .unwrap(),
                    ))
                }
            });
            sides.push(Arc::new(
                own_periodic_support_sides(p, rule.clone(), samples).unwrap(),
            ));
        }
        let sig = fixtures::signatures(Arc::new(
            own_periodic_support_selection(rule, &sides).unwrap(),
        ));
        corners(Arc::new(
            own_periodic_support_regions(sig, &[], 100000).unwrap(),
        ))
    }
    #[test]
    fn native_chart_shores_transport_one_sided_coincident_contour_points_through_actual_seam() {
        let o = shores(cylinder_profile(false));
        assert_eq!(o.interval_count(), 8);
        assert!(o.shores()[0].points.iter().all(|p| !matches!(
            p,
            OriginalPeriodicNativeShorePoint::SourceCopy {
                transported: true,
                ..
            }
        )));
        assert_eq!(
            o.shores()[1]
                .points
                .iter()
                .filter(|p| matches!(
                    p,
                    OriginalPeriodicNativeShorePoint::SourceCopy {
                        transported: true,
                        ..
                    }
                ))
                .count(),
            2
        );
        assert_eq!(
            o.point(1, 1, &q(1, 2)).unwrap(),
            [q(0, 1), q(0, 1), q(1, 2)]
        );
    }
    #[test]
    fn native_chart_shores_order_actual_irrational_quadratic_contacts_without_rounding_cuts() {
        // u=t^2-t+1/8 and v=1/4+t/2 cross U=0 at two irrational clocks.
        let o = shores(cylinder_profile(true));
        assert_eq!(o.interval_count(), 8);
        for s in &o.shores()[..2] {
            assert_eq!(s.points.len(), 4);
            assert!(
                s.points[1..3]
                    .iter()
                    .all(|p| matches!(p, OriginalPeriodicNativeShorePoint::SourceCopy { .. }))
            );
        }
        assert_eq!(
            o.point(0, 1, &q(1, 2)).unwrap(),
            [q(0, 1), q(0, 1), q(1, 2)]
        );
        assert!(o.point(0, 1, &q(1, 4)).is_err());
    }
    pub(in super::super) fn full_torus() -> OwnedPeriodicNativeChartShores {
        let _replay = super::super::periodic_replay::scope();
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
        let o = shores(corners(Arc::new(
            own_periodic_support_regions(sig, &[], 10000).unwrap(),
        )));
        o
    }
    #[test]
    fn native_chart_shores_cover_actual_zero_source_torus_with_only_auxiliary_frame_edges() {
        let o = full_torus();
        assert_eq!(o.interval_count(), 4);
        assert!(o.shores().iter().all(|s| {
            s.role == OriginalPeriodicNativeShoreRole::AuxiliarySeam
                && s.points
                    .iter()
                    .all(|p| matches!(p, OriginalPeriodicNativeShorePoint::Corner(_)))
        }));
        assert!(o.owner().owner().copies().is_empty());
    }
