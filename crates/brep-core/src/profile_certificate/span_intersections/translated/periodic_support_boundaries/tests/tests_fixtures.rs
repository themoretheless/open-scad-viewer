    use super::*;
    fn q(n: i64) -> ExactRatio {
        [n.to_string(), "1".into()]
    }
    pub(in crate::profile_certificate::span_intersections::translated) fn original(
        surface: &Surface,
        wrapped: [bool; 2],
        curves: &[Curve],
        uses: &[Vec<OriginalPeriodicSourceUse>],
    ) -> Arc<OwnedOriginalPeriodicWires> {
        let chart = [
            [
                surface.knots_u[surface.degree_u],
                surface.knots_u[surface.control_points.len()],
            ],
            [
                surface.knots_v[surface.degree_v],
                surface.knots_v[surface.control_points[0].len()],
            ],
        ];
        let cover = Arc::new(
            own_original_periodic_pieces(
                curves,
                chart,
                wrapped,
                1e-12,
                ParameterOrderLimits::default(),
                64,
            )
            .unwrap(),
        );
        let support = Arc::new(
            bind_original_periodic_pieces_to_surface(
                cover,
                surface,
                ParameterOrderLimits::default(),
            )
            .unwrap(),
        );
        Arc::new(own_original_periodic_wires(support, uses).unwrap())
    }
    pub(in crate::profile_certificate::span_intersections::translated) fn graph(
        wires: Arc<OwnedOriginalPeriodicWires>,
    ) -> Arc<OwnedPeriodicWireJumps> {
        graph_with_pair_limits(wires, OriginalAtlasPairLimits::default())
    }
    pub(in crate::profile_certificate::span_intersections::translated) fn graph_with_pair_limits(
        wires: Arc<OwnedOriginalPeriodicWires>,
        pair_limits: OriginalAtlasPairLimits,
    ) -> Arc<OwnedPeriodicWireJumps> {
        let _replay = super::super::periodic_replay::scope();
        let pieces = wires.surface().pieces().clone();
        let n = pieces.pieces().len();
        let count = n * (n + 1) / 2;
        let copies = 3usize.pow(pieces.wrapped().iter().filter(|w| **w).count() as u32);
        let embedding = Arc::new(
            own_original_periodic_embedding(
                wires,
                4096,
                OriginalInjectiveAtlasLimits::default(),
                pair_limits,
            )
            .unwrap(),
        );
        let base = Arc::new(
            own_periodic_arrangement_cuts(
                pieces,
                1e-12,
                ParameterOrderLimits::default(),
                count.max(1),
            )
            .unwrap(),
        );
        let seams = Arc::new(
            own_periodic_seam_contacts(
                embedding,
                1e-12,
                ParameterOrderLimits::default(),
                (count * (copies - 1)).max(1),
            )
            .unwrap(),
        );
        let combined = Arc::new(
            own_periodic_quotient_arrangement_cuts(
                base,
                seams,
                ParameterOrderLimits::default(),
                (count * copies).max(1),
            )
            .unwrap(),
        );
        let aligned = Arc::new(
            own_periodic_aligned_intervals(combined, ParameterOrderLimits::default(), 100000)
                .unwrap(),
        );
        let graph = Arc::new(
            own_periodic_quotient_graph(aligned, ParameterOrderLimits::default()).unwrap(),
        );
        let charts = Arc::new(own_periodic_vertex_charts(graph, 192, 100000).unwrap());
        Arc::new(own_periodic_wire_jumps(charts, 100000).unwrap())
    }
    #[test]
    fn untrimmed_support_keeps_its_base_fill_without_authored_wires_or_fake_torus_edges() {
        let plane = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1., 1.]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let input = original(&plane, [false, false], &[], &[]);
        let support = own_periodic_support_boundaries(
            input,
            1e-12,
            ParameterOrderLimits::default(),
            64,
            100000,
        )
        .unwrap();
        assert!(support.original().wires().is_empty());
        assert_eq!(support.boundaries().len(), 4);
        assert_eq!(
            support.roles(),
            &[OriginalPeriodicSupportWireRole::NaturalSupport]
        );
        let operand = OriginalPeriodicOperand {
            wire_weights: vec![],
            base_winding: "1".into(),
            fill: OriginalPeriodicFillRule::NonZero,
        };
        let lifted = support.lift_operand(&operand).unwrap();
        assert_eq!(lifted.wire_weights, [0]);
        assert_eq!(lifted.base_winding, "1");
        let surface = nurbs_core::primitives::torus([0.; 3], 3., 1., 1.).unwrap();
        let input = original(&surface, [true, true], &[], &[]);
        let support = own_periodic_support_boundaries(
            input.clone(),
            1e-12,
            ParameterOrderLimits::default(),
            64,
            1,
        )
        .unwrap();
        assert!(support.boundaries().is_empty());
        assert!(support.roles().is_empty());
        assert_eq!(support.work(), 0);
        assert!(Arc::ptr_eq(support.augmented(), &input));
        assert!(
            support
                .lift_operand(&operand)
                .unwrap()
                .wire_weights
                .is_empty()
        );
        support.reverify_source().unwrap();
    }
    #[test]
    fn augmented_natural_sources_reach_actual_embedding_complete_arrangement_and_unchanged_authored_fill()
     {
        let surface = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 1., 2., 3., 4., 5.],
            knots_v: vec![10., 10., 14., 14.],
            control_points: (0..3)
                .map(|i| vec![vec![i as f64, 0., 0.], vec![i as f64, 1., 0.]])
                .collect(),
            weights: vec![vec![1., 1.], vec![2., 2.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        let curve = Curve::from_polyline(vec![
            vec![2.25, 11.],
            vec![2.75, 11.],
            vec![2.75, 13.],
            vec![2.25, 13.],
            vec![2.25, 11.],
        ])
        .unwrap();
        let input = original(
            &surface,
            [false, false],
            &[curve],
            &[vec![OriginalPeriodicSourceUse {
                source: 0,
                reversed: false,
            }]],
        );
        let support = own_periodic_support_boundaries(
            input,
            1e-12,
            ParameterOrderLimits::default(),
            64,
            100000,
        )
        .unwrap();
        let jumps = graph(support.augmented().clone());
        assert_eq!(jumps.owner().owner().edges().len(), 8);
        assert_eq!(jumps.wire_fluxes().len(), 2);
        let a = [["17".into(), "8".into()], ["41".into(), "4".into()]];
        let b = [["5".into(), "2".into()], ["12".into(), "1".into()]];
        let base = Arc::new(
            own_periodic_relative_winding(
                jumps.clone(),
                &[a.clone(), a.clone()],
                ParameterOrderLimits::default(),
                1000,
            )
            .unwrap(),
        );
        let operand = support
            .lift_operand(&OriginalPeriodicOperand {
                wire_weights: vec![1],
                base_winding: "0".into(),
                fill: OriginalPeriodicFillRule::NonZero,
            })
            .unwrap();
        assert_eq!(operand.wire_weights, [1, 0]);
        let rule = Arc::new(
            own_periodic_material_rule(base, &[operand], &[OriginalPeriodicBooleanOp::Operand(0)])
                .unwrap(),
        );
        let query = Arc::new(
            own_periodic_relative_winding(jumps, &[a, b], ParameterOrderLimits::default(), 1000)
                .unwrap(),
        );
        assert_eq!(query.deltas(), &["1", "0"]);
        let sample = own_periodic_material_sample(rule, query).unwrap();
        assert!(sample.selected());
        assert_eq!(sample.windings(), &["1"]);
        sample.reverify_source().unwrap();
    }
    #[test]
    fn rational_uv_world_query_preserves_nonbinary_values_and_native_endpoint_conventions() {
        let surface = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 1., 2., 3., 4., 5.],
            knots_v: vec![10., 10., 14., 14.],
            control_points: (0..3)
                .map(|i| vec![vec![i as f64, 0., 0.], vec![i as f64, 1., 0.]])
                .collect(),
            weights: vec![vec![1., 1.], vec![2., 2.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        let o = own_periodic_support_boundaries(
            original(&surface, [false, false], &[], &[]),
            1e-12,
            ParameterOrderLimits::default(),
            64,
            100000,
        )
        .unwrap();
        let third = ["1".into(), "3".into()];
        // At u=7/3 the unweighted basis is (4,13,1)/18 and the
        // homogeneous weighted x numerator/denominator is 28/31.
        assert_eq!(
            o.world_uv_point(&[["7".into(), "3".into()], ["34".into(), "3".into()]])
                .unwrap(),
            [["28".into(), "31".into()], third, q(0)]
        );
        for b in o.boundaries() {
            let varying = 1 - b.fixed_axis;
            let mut uv = [q(0), q(0)];
            uv[b.fixed_axis] = b.level.clone();
            uv[varying] = if varying == 0 {
                ["5".into(), "2".into()]
            } else {
                q(12)
            };
            let i = o.boundaries().iter().position(|v| v == b).unwrap();
            assert_eq!(
                o.world_uv_point(&uv).unwrap(),
                o.world_point(i, &uv[varying]).unwrap()
            );
        }
        assert!(o.world_uv_point(&[q(1), q(12)]).is_err());
        assert!(o.world_uv_point(&[q(2), ["1".into(), "0".into()]]).is_err());
    }
    #[test]
    fn rational_uv_world_query_uses_proved_seams_for_positive_and_negative_cover_tiles() {
        let rim = Curve::from_polyline(vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![1., 1., 0.],
            vec![0., 1., 0.],
            vec![0., 0., 0.],
        ])
        .unwrap();
        let s = nurbs_core::surface::extrude(&rim, [0., 0., 1.]).unwrap();
        let o = own_periodic_support_boundaries(
            original(&s, [true, false], &[], &[]),
            1e-12,
            ParameterOrderLimits::default(),
            64,
            100000,
        )
        .unwrap();
        let z = ["1".into(), "3".into()];
        for numerator in [1, 8001, -7999] {
            assert_eq!(
                o.world_uv_point(&[[numerator.to_string(), "2".into()], z.clone()])
                    .unwrap(),
                [["1".into(), "2".into()], q(0), z.clone()]
            );
        }
        assert_eq!(
            o.world_uv_point(&[q(0), z.clone()]).unwrap(),
            o.world_uv_point(&[q(4), z]).unwrap()
        );
        assert!(o.world_uv_point(&[q(0), q(2)]).is_err());
    }
    #[test]
    fn nonclamped_fixed_boundary_retains_exact_world_weights_and_four_native_source_sides() {
        let surface = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 1., 2., 3., 4., 5.],
            knots_v: vec![10., 10., 14., 14.],
            control_points: (0..3)
                .map(|i| vec![vec![i as f64, 0., 0.], vec![i as f64, 1., 0.]])
                .collect(),
            weights: vec![vec![1., 1.], vec![2., 2.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        let curve = Curve::from_polyline(vec![
            vec![2.25, 11.],
            vec![2.75, 11.],
            vec![2.75, 13.],
            vec![2.25, 13.],
            vec![2.25, 11.],
        ])
        .unwrap();
        let original = original(
            &surface,
            [false, false],
            &[curve],
            &[vec![OriginalPeriodicSourceUse {
                source: 0,
                reversed: false,
            }]],
        );
        assert!(
            own_periodic_support_boundaries(
                original.clone(),
                1e-12,
                ParameterOrderLimits::default(),
                64,
                1
            )
            .is_err()
        );
        let o = own_periodic_support_boundaries(
            original,
            1e-12,
            ParameterOrderLimits::default(),
            64,
            100000,
        )
        .unwrap();
        o.reverify_source().unwrap();
        assert_eq!(o.boundaries().len(), 4);
        assert_eq!(o.augmented().wires().len(), 2);
        let index = o
            .boundaries()
            .iter()
            .position(|b| b.fixed_axis == 0 && b.end == 0)
            .unwrap();
        let b = &o.boundaries()[index];
        assert_eq!(b.native_domain, [q(10), q(14)]);
        assert_eq!(b.world.knots, vec![q(10), q(10), q(14), q(14)]);
        assert_eq!(
            b.world.controls[0],
            [q(1), q(0), q(0), ["3".into(), "2".into()]]
        );
        assert_eq!(
            o.world_point(index, &q(12)).unwrap(),
            [["2".into(), "3".into()], ["1".into(), "2".into()], q(0)]
        );
        assert!(o.world_point(index, &q(15)).is_err());
        let p = OriginalPeriodicOperand {
            wire_weights: vec![7],
            base_winding: "-2".into(),
            fill: OriginalPeriodicFillRule::EvenOdd,
        };
        let lifted = o.lift_operand(&p).unwrap();
        assert_eq!(lifted.wire_weights, [7, 0]);
        assert_eq!(lifted.base_winding, "-2");
        assert_eq!(lifted.fill, p.fill);
        let mut bad = o.clone();
        bad.boundaries[0].world.controls[0][0] = q(99);
        assert!(bad.reverify_source().is_err());
        let mut bad = o.clone();
        bad.roles[0] = OriginalPeriodicSupportWireRole::NaturalSupport;
        assert!(bad.reverify_source().is_err());
        let mut bad = o;
        bad.work += 1;
        assert!(bad.reverify_source().is_err());
    }
    #[test]
    fn periodic_tube_adds_both_natural_rims_with_zero_material_weights_and_keeps_signed_source_bits()
     {
        let mut curve = Curve::from_polyline(vec![vec![0., 0.5], vec![4., 0.5]]).unwrap();
        curve.control_points[0][0] = -0.;
        let jumps = super::super::periodic_wire_jumps::tests::own(
            &[curve],
            &[vec![OriginalPeriodicSourceUse {
                source: 0,
                reversed: false,
            }]],
        );
        let input = jumps
            .owner()
            .owner()
            .seam_contacts()
            .embedding()
            .wires()
            .clone();
        let o = own_periodic_support_boundaries(
            input,
            1e-12,
            ParameterOrderLimits::default(),
            64,
            100000,
        )
        .unwrap();
        assert_eq!(o.boundaries().len(), 2);
        assert_eq!(o.augmented().wires().len(), 3);
        assert_eq!(
            o.augmented().surface().pieces().sources()[0].control_points[0][0].to_bits(),
            (-0.0f64).to_bits()
        );
        assert_eq!(o.augmented().wires()[1].homology, [Some("1".into()), None]);
        assert_eq!(o.augmented().wires()[2].homology, [Some("-1".into()), None]);
        for (i, b) in o.boundaries().iter().enumerate() {
            assert_eq!(b.fixed_axis, 1);
            assert_eq!(
                o.world_point(i, &q(2)).unwrap(),
                [q(1), q(1), q(b.end as i64)]
            );
        }
        assert_eq!(
            o.lift_operand(&OriginalPeriodicOperand {
                wire_weights: vec![1],
                base_winding: "0".into(),
                fill: OriginalPeriodicFillRule::NonZero
            })
            .unwrap()
            .wire_weights,
            [1, 0, 0]
        );
    }
    #[test]
    fn closed_torus_has_no_fake_natural_edges_and_repeated_endpoint_basis_uses_the_original_left_limit()
     {
        let surface = nurbs_core::primitives::torus([0.; 3], 3., 1., 1.).unwrap();
        let curve = Curve::from_polyline(vec![vec![0., 2.], vec![1., 2.]]).unwrap();
        let input = original(
            &surface,
            [true, true],
            &[curve],
            &[vec![OriginalPeriodicSourceUse {
                source: 0,
                reversed: false,
            }]],
        );
        let o = own_periodic_support_boundaries(
            input.clone(),
            1e-12,
            ParameterOrderLimits::default(),
            64,
            100000,
        )
        .unwrap();
        assert!(o.boundaries().is_empty());
        assert!(Arc::ptr_eq(o.augmented(), &input));
        let knots = [0, 0, 1, 2, 3, 3, 4, 4].map(|n| Q::from_integer(n.into()));
        let mut work = 0;
        let values = basis(2, &knots, &Q::from_integer(3.into()), &mut work, 1000).unwrap();
        assert_eq!(
            values,
            vec![Q::zero(), Q::zero(), Q::zero(), Q::one(), Q::zero()]
        );
    }
