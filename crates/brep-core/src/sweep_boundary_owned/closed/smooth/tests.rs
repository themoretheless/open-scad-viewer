use super::*;
use nurbs_core::{
    progressive_sweep::{Spacing, constant_vector_law},
    surface::Surface,
};
fn material() -> (LoftCap, Vec<Curve>) {
    let r = 1. / 128.;
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-1., -1., 3., 3.],
        knots_v: vec![2., 2., 6., 6.],
        control_points: vec![
            vec![vec![0., -r, -r], vec![0., -r, r]],
            vec![vec![0., r, -r], vec![0., r, r]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let signs = [
        [1., 0.],
        [1., 1.],
        [0., 1.],
        [-1., 1.],
        [-1., 0.],
        [-1., -1.],
        [0., -1.],
        [1., -1.],
        [1., 0.],
    ];
    let curves = (0..4)
        .map(|i| Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: signs[2 * i..=2 * i + 2]
                .iter()
                .map(|p| vec![0., r * p[0], r * p[1]])
                .collect(),
            weights: vec![1., 0.5, 1.],
            periodic: false,
        })
        .collect::<Vec<_>>();
    let trims = (0..4)
        .map(|i| Curve {
            degree: 2,
            knots: vec![7., 7., 7., 11., 11., 11.],
            control_points: signs[2 * i..=2 * i + 2]
                .iter()
                .map(|p| vec![1. + 2. * p[0], 4. + 2. * p[1]])
                .collect(),
            weights: vec![1., 0.5, 1.],
            periodic: false,
        })
        .collect();
    (
        LoftCap {
            surface: s,
            trims: vec![trims],
        },
        curves,
    )
}
fn limits() -> SmoothClosedLimits {
    SmoothClosedLimits {
        source: super::super::tests::limits().into(),
        original_smoothness: None,
        candidate: AffineStationLimits {
            quantum: 2f64.powi(-18),
            tolerance: 0.03,
            work: 1000000,
            matrix_cells: 100000,
        },
        smoothness: crate::sweep_smoothness::complete::Limits {
            order: 2,
            exact_work: 16000000,
            agreement_work: 1000000,
            regularity_cells: 100000,
        },
        atlas: IdealLimits {
            cells: 1000000,
            exact_work: 1000000,
            projection: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        },
        binding_work: 1000000,
        orientation_cells: 100000,
    }
}
#[test]
fn original_spatial_material_and_actual_g2_solid_have_independent_global_certificates() {
    let (material, profiles) = material();
    let original = (
        material.surface.clone(),
        material.trims.clone(),
        profiles.clone(),
    );
    let path = super::super::tests::path(false, true);
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        super::super::tests::options(true, Spacing::Parameter),
    )
    .unwrap()
    .with_spatial_rmf_error_limits(1024, 100000, 1000000)
    .unwrap();
    let body = match construct_smooth_closed_with_ideal_atlas(
        &sweep,
        &[4],
        33,
        &material,
        1e-8,
        0.05,
        &[],
        limits(),
    )
    .unwrap()
    {
        SmoothClosedAttempt::Certified(b) => b,
        SmoothClosedAttempt::Refused(r) => panic!(
            "smooth refused {:?}; original={} E={:?} d={:?} binding={:?} smooth={:?} volume={:?} orientation={:?}",
            r.reason,
            r.original.certified,
            r.boundary_error_upper,
            r.displacement_upper,
            r.binding
                .as_ref()
                .map(|b| (b.certified, b.faces, b.exact_work, b.reason)),
            r.smoothness.as_ref().map(|s| (
                s.g2_certified,
                s.extraction_complete,
                s.regularity.all_faces_regular
            )),
            r.volume,
            r.orientation
        ),
    };
    let r = body.report();
    assert!(
        r.certified
            && r.original.certified
            && r.continuous_bound
            && r.within_budget
            && r.topology_preserved
            && r.shell_roles_certified
    );
    assert!(r.smoothness.as_ref().unwrap().g2_certified);
    assert!(r.volume.as_ref().unwrap().embedding_certified);
    assert_eq!(r.binding.as_ref().unwrap().faces, body.model().faces.len());
    assert_eq!(body.model().faces.len(), 128);
    assert_eq!(r.orientation.as_ref().unwrap().sign, Some(1));
    assert!(r.boundary_error_upper.unwrap() > r.original.boundary_error_upper.unwrap());
    assert_eq!(
        (
            material.surface.clone(),
            material.trims.clone(),
            profiles.clone()
        ),
        original
    );
    eprintln!(
        "constructor-owned G2 Solid E={:?} d={:?} original={:?} actual={:?} binding={} orientation={}",
        r.boundary_error_upper,
        r.displacement_upper,
        r.original.volume,
        r.volume,
        r.binding.as_ref().unwrap().exact_work,
        r.orientation_cells
    );
    #[cfg(feature = "codec")]
    if let Ok(destination) = std::env::var("OSV_SMOOTH_CLOSED_FIXTURE") {
        let l = limits();
        let original = l.source;
        let sm = l.smoothness;
        let c = l.candidate;
        let a = l.atlas;
        let packet = value_codec::json!({"op":"brep_sweep_smooth_closed_material_construct",
            "source":{"profiles":profiles,"path":path,"scale":scale,"twist":twist,"orientation":"rmf","spacing":"parameter",
                "normal":[0.,0.,1.],"initial_sections":33,"max_sections":129,"max_deviation":0.05,
                "rmf_transport_steps":1024,"error_max_cells":100000,"error_max_products":1000000},
            "material":{"surface":material.surface,"trims":material.trims},"loopSizes":[4],"sections":33,
            "toleranceUv":1e-8,"maxDeviation":0.05,"sharpStations":[],
            "idealBody":{"method":"atlas","projection":original.ideal.projection,"maxCells":original.ideal.cells,"maxExactWork":original.ideal.exact_work},
            "limits":{"sourceCells":original.source_cells,"sourceProducts":original.source_products,"sourceExactWork":original.source_exact_work,
                "maxWallFaces":original.wall_faces,"material":{"maxPairs":original.material.pairs,"maxCells":original.material.cells,"maxDomainCells":original.material.domain_cells}},
            "smoothing":{"quantum":c.quantum,"maxDisplacement":c.tolerance,"maxWork":c.work,"maxMatrixCells":c.matrix_cells,
                "maxBindingWork":l.binding_work,"maxOrientationCells":l.orientation_cells,"order":sm.order,"maxExactWork":sm.exact_work,
                "maxAgreementWork":sm.agreement_work,"maxRegularityCells":sm.regularity_cells,
                "atlas":{"projection":a.projection,"maxCells":a.cells,"maxExactWork":a.exact_work}}});
        std::fs::write(destination, value_codec::to_string(&packet).unwrap()).unwrap();
    }
}

fn reverse_wire(wire: &[Curve]) -> Vec<Curve> {
    wire.iter().rev().map(|c| c.reverse().unwrap()).collect()
}
fn qualified_mode(mode: usize, count: usize, hole: bool, reverse: bool) {
    let (mut material, mut profiles) = material();
    let mut sizes = vec![4];
    if hole {
        let mut inner = profiles.clone();
        for c in &mut inner {
            for p in &mut c.control_points {
                for x in p {
                    *x *= 0.5;
                }
            }
        }
        profiles.extend(reverse_wire(&inner));
        let mut uv = material.trims[0].clone();
        for c in &mut uv {
            for p in &mut c.control_points {
                p[0] = 1. + (p[0] - 1.) * 0.5;
                p[1] = 4. + (p[1] - 4.) * 0.5;
            }
        }
        material.trims.push(reverse_wire(&uv));
        sizes.push(4);
    }
    if reverse {
        profiles = sizes
            .iter()
            .scan(0, |offset, size| {
                let a = *offset;
                *offset += size;
                Some(reverse_wire(&profiles[a..*offset]))
            })
            .flatten()
            .collect();
        for w in &mut material.trims {
            *w = reverse_wire(w);
        }
    }
    let path = super::super::tests::path(false, true);
    let scalar = |p: [f64; 5], a, b| Curve {
        degree: 4,
        knots: [vec![a; 5], vec![b; 5]].concat(),
        control_points: p.map(|x| vec![x, 0., 0.]).to_vec(),
        weights: vec![1.; 5],
        periodic: false,
    };
    let scale = if mode == 0 {
        constant_vector_law([1., 0., 0.]).unwrap()
    } else {
        scalar([1., 1., 1.125, 1., 1.], 2., 6.)
    };
    let twist = if mode == 0 {
        constant_vector_law([0.; 3]).unwrap()
    } else {
        scalar([0., 0., 1. / 64., 0., 0.], 7., 11.)
    };
    let axes = constant_vector_law([1., 1.125, 0.875]).unwrap();
    let center = constant_vector_law([1. / 1024., 1. / 2048., 0.]).unwrap();
    let axis = Curve {
        degree: 8,
        knots: [vec![2.; 9], vec![6.; 9]].concat(),
        control_points: path
            .control_points
            .windows(2)
            .map(|p| (0..3).map(|k| 9. * (p[1][k] - p[0][k])).collect())
            .collect(),
        weights: vec![1.; 9],
        periodic: false,
    };
    let normal = constant_vector_law([0., 0., 1.]).unwrap();
    let mut guide = path.clone();
    for p in &mut guide.control_points {
        p[2] += 1.;
    }
    let mut options = super::super::tests::options(true, Spacing::Parameter);
    if mode == 1 {
        options.orientation = nurbs_core::progressive_sweep::Orientation::Fixed;
    }
    if mode == 2 {
        options.orientation = nurbs_core::progressive_sweep::Orientation::FixedNormal;
    }
    let before = (
        material.surface.clone(),
        material.trims.clone(),
        profiles.clone(),
        path.clone(),
        scale.clone(),
        twist.clone(),
    );
    let mut sweep = MultiSweep::new(&profiles, &path, &scale, &twist, options).unwrap();
    if mode == 0 {
        sweep = sweep
            .with_spatial_rmf_error_limits(1024, 100000, 1000000)
            .unwrap();
    }
    if mode == 1 {
        sweep = sweep.with_frame_laws(&axis, &normal).unwrap();
    }
    if mode == 2 {
        sweep = sweep.with_orientation_guide(&guide).unwrap();
    }
    if mode > 0 {
        sweep = sweep.with_affine_laws(&axes, &center).unwrap();
    }
    let b = match construct_smooth_closed_with_ideal_atlas(
        &sweep,
        &sizes,
        count,
        &material,
        1e-8,
        0.05,
        &[],
        SmoothClosedLimits {
            original_smoothness: Some(
                nurbs_core::progressive_sweep::OriginalBoundarySmoothnessLimits {
                    order: 2,
                    cells: 100000,
                    exact_work: 1000000,
                    products: 1000000,
                },
            ),
            ..limits()
        },
    )
    .unwrap()
    {
        SmoothClosedAttempt::Certified(b) => b,
        SmoothClosedAttempt::Refused(r) => panic!(
            "mode {mode}/{count}/hole={hole}/reverse={reverse} refused {:?}; source={:?} E={:?} candidate={:?} binding={:?} smooth={:?} atlas={:?} orientation={:?}",
            r.reason,
            r.original.reason,
            r.boundary_error_upper,
            r.candidate_reason,
            r.binding
                .as_ref()
                .map(|p| (p.faces, p.exact_work, p.reason)),
            r.smoothness.as_ref().map(|p| (
                p.g2_certified,
                p.exact_work,
                p.regularity.all_faces_regular
            )),
            r.volume,
            r.orientation
        ),
    };
    let r = b.report();
    assert!(
        r.certified
            && r.original.certified
            && r.topology_preserved
            && r.shell_roles_certified
            && r.continuous_bound
    );
    assert!(r.smoothness.as_ref().unwrap().g2_certified);
    assert!(
        r.original
            .smoothness
            .as_ref()
            .unwrap()
            .differential_compatibility_certified
    );
    assert_eq!(b.model().shells.len(), sizes.len());
    assert_eq!(b.model().bodies[0].inner_shells.len(), usize::from(hole));
    assert_eq!(
        b.model().faces.len(),
        sizes.iter().sum::<usize>() * (count - 1)
    );
    assert_eq!(
        (
            material.surface.clone(),
            material.trims.clone(),
            profiles.clone(),
            path.clone(),
            scale.clone(),
            twist.clone()
        ),
        before
    );
    eprintln!(
        "owned G2 mode={mode}/{count} hole={hole} reversed={reverse}: E={:?} actual={:?} seams={}",
        r.boundary_error_upper,
        r.volume,
        r.smoothness.as_ref().unwrap().edge_ids.len()
    );
}
#[test]
fn spatial_owned_g2_supports_forty_nonbinary_station_intervals() {
    qualified_mode(0, 41, false, false);
}
#[test]
fn authored_frame_scale_twist_affine_owned_g2_solid() {
    qualified_mode(1, 33, false, false);
}
#[test]
fn guided_frame_scale_twist_affine_owned_g2_solid() {
    qualified_mode(2, 129, false, false);
}
#[test]
fn hollow_spatial_owned_g2_has_correct_material_shell_roles() {
    qualified_mode(0, 33, true, false);
}
#[test]
fn reversed_original_hole_and_outer_keep_owned_g2_orientation() {
    qualified_mode(0, 33, true, true);
}

fn body(attempt: SmoothClosedAttempt) -> Box<SmoothClosedBody> {
    match attempt {
        SmoothClosedAttempt::Certified(b) => b,
        SmoothClosedAttempt::Refused(r) => panic!(
            "owned smooth refusal {:?} source={:?} E={:?} binding={:?} smooth={:?} volume={:?}",
            r.reason,
            r.original.reason,
            r.boundary_error_upper,
            r.binding.as_ref().map(|p| p.reason),
            r.smoothness.as_ref().map(|p| (
                p.g1_certified,
                p.g2_certified,
                p.proof.unresolved_seams.clone()
            )),
            r.volume
        ),
    }
}
#[test]
fn every_zero_or_one_short_owner_stage_returns_no_partial_solid() {
    let (material, profiles) = material();
    let path = super::super::tests::path(false, true);
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        super::super::tests::options(true, Spacing::Parameter),
    )
    .unwrap()
    .with_spatial_rmf_error_limits(1024, 100000, 1000000)
    .unwrap();
    let full = body(
        construct_smooth_closed_with_ideal_atlas(
            &sweep,
            &[4],
            33,
            &material,
            1e-8,
            0.05,
            &[],
            limits(),
        )
        .unwrap(),
    );
    let r = full.report();
    let l = limits();
    let mut cases = Vec::new();
    for work in [0, r.candidate_work - 1] {
        cases.push(SmoothClosedLimits {
            candidate: AffineStationLimits {
                work,
                ..l.candidate
            },
            ..l
        });
    }
    for matrix_cells in [0, r.matrix_cells - 1] {
        cases.push(SmoothClosedLimits {
            candidate: AffineStationLimits {
                matrix_cells,
                ..l.candidate
            },
            ..l
        });
    }
    for binding_work in [0, r.binding.as_ref().unwrap().exact_work - 1] {
        cases.push(SmoothClosedLimits { binding_work, ..l });
    }
    for exact_work in [0, r.smoothness.as_ref().unwrap().exact_work - 1] {
        cases.push(SmoothClosedLimits {
            smoothness: crate::sweep_smoothness::complete::Limits {
                exact_work,
                ..l.smoothness
            },
            ..l
        });
    }
    for regularity_cells in [0, r.smoothness.as_ref().unwrap().regularity.cells - 1] {
        cases.push(SmoothClosedLimits {
            smoothness: crate::sweep_smoothness::complete::Limits {
                regularity_cells,
                ..l.smoothness
            },
            ..l
        });
    }
    cases.push(SmoothClosedLimits {
        smoothness: crate::sweep_smoothness::complete::Limits {
            agreement_work: 0,
            ..l.smoothness
        },
        ..l
    });
    for cells in [0, r.volume.as_ref().unwrap().cells - 1] {
        cases.push(SmoothClosedLimits {
            atlas: IdealLimits { cells, ..l.atlas },
            ..l
        });
    }
    for exact_work in [0, r.volume.as_ref().unwrap().exact_work - 1] {
        cases.push(SmoothClosedLimits {
            atlas: IdealLimits {
                exact_work,
                ..l.atlas
            },
            ..l
        });
    }
    for orientation_cells in [0, r.orientation_cells - 1] {
        cases.push(SmoothClosedLimits {
            orientation_cells,
            ..l
        });
    }
    cases.push(SmoothClosedLimits {
        source: OriginalClosedLimits {
            source_products: 0,
            ..l.source
        },
        ..l
    });
    cases.push(SmoothClosedLimits {
        source: OriginalClosedLimits {
            source_exact_work: 0,
            ..l.source
        },
        ..l
    });
    cases.push(SmoothClosedLimits {
        source: OriginalClosedLimits {
            ideal: IdealLimits {
                cells: 0,
                ..l.source.ideal
            },
            ..l.source
        },
        ..l
    });
    cases.push(SmoothClosedLimits {
        source: OriginalClosedLimits {
            ideal: IdealLimits {
                exact_work: 0,
                ..l.source.ideal
            },
            ..l.source
        },
        ..l
    });
    for (i, l) in cases.iter().copied().enumerate() {
        let attempt = construct_smooth_closed_with_ideal_atlas(
            &sweep,
            &[4],
            33,
            &material,
            1e-8,
            0.05,
            &[],
            l,
        )
        .unwrap();
        let SmoothClosedAttempt::Refused(r) = attempt else {
            panic!("short owner stage {i} returned a Solid");
        };
        assert!(
            !r.certified && !r.topology_preserved && !r.shell_roles_certified && r.reason.is_some()
        );
        if let Some(b) = &r.binding {
            assert!(b.exact_work <= l.binding_work);
        }
        if let Some(v) = &r.volume {
            assert!(v.cells <= l.atlas.cells && v.exact_work <= l.atlas.exact_work);
        }
        assert!(r.orientation_cells <= l.orientation_cells);
        eprintln!("short smooth owner stage {i}: {:?}", r.reason);
    }
    for (error_budget, delta_budget) in [
        (
            r.boundary_error_upper.unwrap().next_down(),
            l.candidate.tolerance,
        ),
        (0.05, r.displacement_upper.unwrap().next_down()),
    ] {
        let l = SmoothClosedLimits {
            candidate: AffineStationLimits {
                tolerance: delta_budget,
                ..l.candidate
            },
            ..l
        };
        assert!(matches!(
            construct_smooth_closed_with_ideal_atlas(
                &sweep,
                &[4],
                33,
                &material,
                1e-8,
                error_budget,
                &[],
                l
            )
            .unwrap(),
            SmoothClosedAttempt::Refused(_)
        ));
    }
}

#[test]
fn geometric_g1_is_preserved_without_claiming_g2_at_a_curvature_jump() {
    let (mut material, mut profiles) = material();
    profiles[0].weights[1] = 0.75;
    material.trims[0][0].weights[1] = 0.75;
    let path = super::super::tests::path(false, true);
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        super::super::tests::options(true, Spacing::Parameter),
    )
    .unwrap()
    .with_spatial_rmf_error_limits(1024, 100000, 1000000)
    .unwrap();
    let mut l = limits();
    l.smoothness.order = 1;
    let b = body(
        construct_smooth_closed_with_ideal_atlas(&sweep, &[4], 33, &material, 1e-8, 0.05, &[], l)
            .unwrap(),
    );
    assert!(b.report().certified && b.report().smoothness.as_ref().unwrap().g1_certified);
    assert!(!b.report().smoothness.as_ref().unwrap().g2_certified);
    l.smoothness.order = 2;
    let SmoothClosedAttempt::Refused(r) =
        construct_smooth_closed_with_ideal_atlas(&sweep, &[4], 33, &material, 1e-8, 0.05, &[], l)
            .unwrap()
    else {
        panic!("a genuine original curvature jump received G2 Solid credit");
    };
    assert_eq!(r.reason, Some("complete-boundary-smoothness-unproved"));
    assert!(!r.smoothness.as_ref().unwrap().g2_certified);
}

#[test]
fn exact_wall_binding_refuses_sub_tolerance_geometry_weights_and_domain_changes() {
    let (material, profiles) = material();
    let path = super::super::tests::path(false, true);
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        super::super::tests::options(true, Spacing::Parameter),
    )
    .unwrap()
    .with_spatial_rmf_error_limits(1024, 100000, 1000000)
    .unwrap();
    let PreparedAttempt::Ready(p) =
        prepare_closed_source(&sweep, &[4], 33, &material, 1e-8, 0.05, limits().source).unwrap()
    else {
        panic!("source preparation failed");
    };
    let c = affine_station_walls(&p.profiles, &p.sections, &[], true, limits().candidate).unwrap();
    let poses = c.poses.unwrap();
    let origin = c.source_origin.unwrap();
    let model = crate::analytic::periodic_material_loft_surfaces(
        c.sections.as_ref().unwrap(),
        c.sides.as_ref().unwrap(),
    )
    .unwrap();
    assert!(
        binding(&model, &p.profiles, &poses, origin, 1000000)
            .unwrap()
            .certified
    );
    for gauge in 1_u8..8 {
        let mut chart = model.clone();
        for face in 0..chart.faces.len() {
            let outer = chart.faces[face].outer;
            let s = &mut chart.faces[face].surface;
            if gauge & 4 != 0 {
                std::mem::swap(&mut s.degree_u, &mut s.degree_v);
                std::mem::swap(&mut s.knots_u, &mut s.knots_v);
                let old = std::mem::take(&mut s.control_points);
                s.control_points = (0..old[0].len())
                    .map(|j| old.iter().map(|r| r[j].clone()).collect())
                    .collect();
                let old = std::mem::take(&mut s.weights);
                s.weights = (0..old[0].len())
                    .map(|j| old.iter().map(|r| r[j]).collect())
                    .collect();
            }
            if gauge & 1 != 0 {
                s.control_points.reverse();
                s.weights.reverse();
            }
            if gauge & 2 != 0 {
                for r in &mut s.control_points {
                    r.reverse();
                }
                for r in &mut s.weights {
                    r.reverse();
                }
            }
            for c in &mut chart.loops[outer].coedges {
                for p in &mut c.pcurve.control_points {
                    if gauge & 4 != 0 {
                        p.swap(0, 1);
                    }
                    if gauge & 1 != 0 {
                        p[0] = 1. - p[0];
                    }
                    if gauge & 2 != 0 {
                        p[1] = 1. - p[1];
                    }
                }
            }
            if gauge.count_ones() % 2 != 0 {
                // Keep native outer UV winding and world coedge ownership
                // after reflecting the chart. Face-use sense changes below.
                let wire = &mut chart.loops[outer];
                wire.coedges.reverse();
                for c in &mut wire.coedges {
                    c.reversed = !c.reversed;
                    c.pcurve = c.pcurve.reverse().unwrap();
                }
            }
        }
        for shell in &mut chart.shells {
            for u in &mut shell.faces {
                u.reversed ^= gauge.count_ones() % 2 != 0;
            }
        }
        chart.validate().unwrap();
        let proof = binding(&chart, &p.profiles, &poses, origin, 1000000).unwrap();
        assert!(proof.certified, "gauge {gauge}: {}", proof.reason);
        assert_eq!(proof.reparameterized_faces(), model.faces.len());
        assert_eq!(
            proof.orientation_reversed(0),
            Some(gauge.count_ones() % 2 != 0)
        );
        let smooth =
            crate::sweep_smoothness::complete::inspect(&chart, limits().smoothness).unwrap();
        assert!(smooth.g2_certified, "complete actual gauge {gauge}");
        if gauge == 5 {
            let short = binding(&chart, &p.profiles, &poses, origin, proof.exact_work - 1).unwrap();
            assert!(!short.certified && short.exact_work < proof.exact_work);
        }
    }
    let mut permuted = model.clone();
    let n = permuted.faces.len();
    permuted.faces.reverse();
    for shell in &mut permuted.shells {
        for u in &mut shell.faces {
            u.face = n - 1 - u.face;
        }
    }
    permuted.validate().unwrap();
    let proof = binding(&permuted, &p.profiles, &poses, origin, 1000000).unwrap();
    assert!(proof.certified, "{}", proof.reason);
    assert_eq!(proof.faces, n);
    for max in [0, proof.exact_work - 1] {
        let short = binding(&permuted, &p.profiles, &poses, origin, max).unwrap();
        assert!(!short.certified && short.exact_work <= max);
    }
    let mut duplicate = p.profiles.clone();
    duplicate[0][1] = duplicate[0][0].clone();
    assert!(
        !binding(&permuted, &duplicate, &poses, origin, 1000000)
            .unwrap()
            .certified
    );
    permuted.faces[0].surface.control_points[1][2][0] += 1e-10;
    permuted.validate().unwrap();
    assert!(
        !binding(&permuted, &p.profiles, &poses, origin, 1000000)
            .unwrap()
            .certified
    );
    for which in 0..3 {
        let mut changed = model.clone();
        if which == 0 {
            changed.faces[0].surface.control_points[1][2][0] += 1e-10;
        }
        if which == 1 {
            changed.faces[0].surface.weights[1][2] += 1e-10;
        }
        if which == 2 {
            let wire = changed.faces[0].outer;
            changed.loops[wire].coedges[0].pcurve.control_points[0][0] += 1e-12;
        }
        changed.validate().unwrap();
        assert!(
            !binding(&changed, &p.profiles, &poses, origin, 1000000)
                .unwrap()
                .certified
        );
    }
    let mut wrong = poses.clone();
    wrong[0][2][3][0] += 1e-12;
    assert!(
        !binding(&model, &p.profiles, &wrong, origin, 1000000)
            .unwrap()
            .certified
    );
}

#[test]
fn reversed_carrier_chart_uses_world_jacobian_and_uv_winding_for_orientation() {
    let (mut material, profiles) = material();
    material.surface.control_points.reverse();
    material.surface.weights.reverse();
    for c in &mut material.trims[0] {
        for p in &mut c.control_points {
            p[0] = 2. - p[0];
        }
    }
    let path = super::super::tests::path(false, true);
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        super::super::tests::options(true, Spacing::Parameter),
    )
    .unwrap()
    .with_spatial_rmf_error_limits(1024, 100000, 1000000)
    .unwrap();
    let b = body(
        construct_smooth_closed_with_ideal_atlas(
            &sweep,
            &[4],
            33,
            &material,
            1e-8,
            0.05,
            &[],
            limits(),
        )
        .unwrap(),
    );
    assert_eq!(b.report().orientation.as_ref().unwrap().sign, Some(-1));
    assert_eq!(
        b.report().original.material.trim.as_ref().unwrap().winding,
        vec![Some(-1)]
    );
    assert!(b.model().shells[0].faces.iter().all(|u| !u.reversed));
}

#[test]
fn nonplanar_filled_carrier_and_nonplanar_original_boundary_have_owned_global_g2() {
    let r = 1. / 128.;
    let h = 1. / 65536.;
    let grid = [-3., 3.];
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-3., -3., 3., 3.],
        knots_v: vec![-3., -3., 3., 3.],
        control_points: grid
            .iter()
            .map(|u| grid.iter().map(|v| vec![h * u * v, r * u, r * v]).collect())
            .collect(),
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let signs = [
        [3., 0.],
        [3., 3.],
        [0., 3.],
        [-3., 3.],
        [-3., 0.],
        [-3., -3.],
        [0., -3.],
        [3., -3.],
        [3., 0.],
    ];
    let elevate = |p: [f64; 3]| {
        [
            p[0],
            (p[0] + p[1]) / 2.,
            (p[0] + 4. * p[1] + p[2]) / 6.,
            (p[1] + p[2]) / 2.,
            p[2],
        ]
    };
    let product = |u: [f64; 3], v: [f64; 3]| {
        [
            u[0] * v[0],
            (u[0] * v[1] + u[1] * v[0]) / 2.,
            (u[0] * v[2] + 4. * u[1] * v[1] + u[2] * v[0]) / 6.,
            (u[1] * v[2] + u[2] * v[1]) / 2.,
            u[2] * v[2],
        ]
    };
    let mut profiles = Vec::new();
    let mut trims = Vec::new();
    for i in 0..4 {
        let uv = &signs[2 * i..=2 * i + 2];
        let u = [uv[0][0], uv[1][0], uv[2][0]];
        let v = [uv[0][1], uv[1][1], uv[2][1]];
        let x = product(u, v);
        let y = elevate(u);
        let z = elevate(v);
        profiles.push(Curve {
            degree: 4,
            knots: [vec![0.; 5], vec![1.; 5]].concat(),
            control_points: (0..5).map(|j| vec![h * x[j], r * y[j], r * z[j]]).collect(),
            weights: vec![1.; 5],
            periodic: false,
        });
        trims.push(Curve {
            degree: 2,
            knots: vec![7., 7., 7., 11., 11., 11.],
            control_points: uv.iter().map(|p| p.to_vec()).collect(),
            weights: vec![1.; 3],
            periodic: false,
        });
    }
    let material = LoftCap {
        surface,
        trims: vec![trims],
    };
    let path = super::super::tests::path(false, true);
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        super::super::tests::options(true, Spacing::Parameter),
    )
    .unwrap()
    .with_spatial_rmf_error_limits(1024, 100000, 1000000)
    .unwrap();
    let b = body(
        construct_smooth_closed_with_ideal_atlas(
            &sweep,
            &[4],
            33,
            &material,
            1e-8,
            0.05,
            &[],
            limits(),
        )
        .unwrap(),
    );
    assert!(
        b.report().certified
            && b.report().topology_preserved
            && b.report().smoothness.as_ref().unwrap().g2_certified
    );
    eprintln!(
        "nonplanar original material G2 Solid: E={:?}, actual={:?}, source={:?}",
        b.report().boundary_error_upper,
        b.report().volume,
        b.report().original.volume
    );
}

use nurbs_core as nurbs;
#[path="../../../../../nurbs-core/tests/support/arc_bishop_source.rs"]
mod arc_source_case;
#[test]
fn geometric_arc_source_clock_has_complete_original_and_actual_g2_solid_with_global_atlases(){
    let (profiles,surface,trims)=arc_source_case::material();let material=LoftCap{surface,trims};
    let path=arc_source_case::path(true);
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
    let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,super::super::tests::options(true,Spacing::ArcLength{tolerance:0.001,max_cells:100000})).unwrap()
        .with_spatial_rmf_error_limits(4096,100000,1000000).unwrap();
    let mut l=limits();
    l.source.ideal.cells=1000000;
    l.original_smoothness=Some(nurbs_core::progressive_sweep::OriginalBoundarySmoothnessLimits{order:2,cells:100000,exact_work:1000000,products:1000000});
    let attempt=construct_smooth_closed_with_ideal_atlas(&sweep,&[4],65,&material,1e-8,0.05,&[],l).unwrap();
    if let SmoothClosedAttempt::Refused(r)=&attempt {eprintln!("original diagnostic {:?}, cells={} work={} smooth={:?}",r.original.volume,r.original.cells,r.original.exact_work,r.original.smoothness);}
    let b=body(attempt);
    let r=b.report();assert!(r.certified && r.original.certified && r.topology_preserved && r.shell_roles_certified && r.continuous_bound);
    assert!(r.original.g2_certified());
    assert!(r.smoothness.as_ref().unwrap().g2_certified);
    assert!(r.original.volume.as_ref().unwrap().embedding_certified && r.volume.as_ref().unwrap().embedding_certified);
    eprintln!("geometric clock original/actual G2 Solid; faces={} E={:?} source={:?} actual={:?}",b.model().faces.len(),r.boundary_error_upper,r.original.volume,r.volume);
}
