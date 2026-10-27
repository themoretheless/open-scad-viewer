//! Budget, binding, continuity and carrier ownership regressions.
use super::*;
#[test]
fn every_zero_or_one_short_owner_stage_returns_no_partial_solid() {
    let (material, profiles) = material();
    let path = super::super::super::tests::path(false, true);
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        super::super::super::tests::options(true, Spacing::Parameter),
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
    let path = super::super::super::tests::path(false, true);
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        super::super::super::tests::options(true, Spacing::Parameter),
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
    let path = super::super::super::tests::path(false, true);
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        super::super::super::tests::options(true, Spacing::Parameter),
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
    let path = super::super::super::tests::path(false, true);
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        super::super::super::tests::options(true, Spacing::Parameter),
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
    let path = super::super::super::tests::path(false, true);
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        super::super::super::tests::options(true, Spacing::Parameter),
    )
    .unwrap()
    .with_spatial_rmf_error_limits(1024, 100000, 1000000)
    .unwrap();
    let b = body(
        construct_smooth_closed_with_ideal_atlas(
            &sweep,
            &[4],
            65,
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
