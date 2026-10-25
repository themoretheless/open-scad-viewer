use crate::{LoftCap, sweep_boundary_owned::*};
use nurbs_core::{
    curve::Curve,
    progressive_sweep::{MultiSweep, Options, Orientation, Spacing, constant_vector_law},
    surface::Surface,
};
fn cap(corners: Vec<Vec<f64>>) -> LoftCap {
    let xy = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    LoftCap {
        surface: Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![corners[0].clone(), corners[3].clone()],
                vec![corners[1].clone(), corners[2].clone()],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        },
        trims: vec![
            (0..4)
                .map(|i| Curve {
                    degree: 1,
                    knots: vec![0., 0., 1., 1.],
                    control_points: vec![xy[i].to_vec(), xy[(i + 1) % 4].to_vec()],
                    weights: vec![1.; 2],
                    periodic: false,
                })
                .collect(),
        ],
    }
}
fn limits() -> Limits {
    Limits {
        source_cells: 100000,
        source_products: 1000000,
        source_exact_work: 1000000,
        regularity_cells: 10000,
        linear_cells: 10000,
        volume: crate::volume_validity::Limits {
            boundary: crate::boundary_embedding::Limits {
                exact_work: 1000000,
                trim_pairs: 10000,
                trim_cells: 100000,
                trim_domain_cells: 1000000,
                spans: 1000,
                contacts: crate::face_contacts::Limits {
                    pairs: 10000,
                    cells: 100000,
                    domain_cells: 1000000,
                    cells_per_pair: 1000,
                    domain_cells_per_pair: 10000,
                },
            },
            nesting_pairs: 100,
            nesting_cells: 100000,
            nesting_domain_cells: 1000000,
            orientation_cells: 100000,
            orientation_domain_cells: 1000000,
            orientation_spans: 100,
        },
    }
}
#[test]
fn original_moving_authored_frame_and_saddle_caps_receive_a_joint_native_certificate() {
    let corners = vec![
        vec![0., 0., 0.],
        vec![1., 0., 0.],
        vec![1., 1., 1. / 64.],
        vec![0., 1., 0.],
    ];
    let profiles = (0..4)
        .map(|i| {
            nurbs_core::primitives::line(
                corners[i].clone().try_into().unwrap(),
                corners[(i + 1) % 4].clone().try_into().unwrap(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let path = nurbs_core::primitives::line([0.; 3], [0., 0., 64.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let axis = constant_vector_law([0., 0., 1.]).unwrap();
    // Nonconstant interior frame with exact endpoint bases: the emitted JSON
    // fixture can be replayed across native hosts and WASM without importing
    // rounded cap coordinates from a different host's hypot implementation.
    let normal = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1., 0., 0.], vec![1., 0.125, 0.], vec![1., 0., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let source = cap(corners);
    let options = Options {
        normal: [1., 0., 0.],
        orientation: Orientation::Fixed,
        spacing: Spacing::Parameter,
        initial_sections: 5,
        max_sections: 5,
        max_deviation: 0.01,
    };
    let sweep = MultiSweep::new(&profiles, &path, &scale, &twist, options)
        .unwrap()
        .with_frame_laws(&axis, &normal)
        .unwrap();
    let rows = sweep.sections_at(5).unwrap();
    let retained = [
        cap(rows
            .iter()
            .map(|r| r[0].control_points[0].clone())
            .collect()),
        cap(rows
            .iter()
            .map(|r| r[4].control_points[0].clone())
            .collect()),
    ];
    assert!(
        construct(
            &sweep,
            &[usize::MAX, 1],
            5,
            [&source, &source],
            [&retained[0], &retained[1]],
            1e-8,
            0.01,
            limits()
        )
        .is_err()
    );
    let attempt = construct(
        &sweep,
        &[4],
        5,
        [&source, &source],
        [&retained[0], &retained[1]],
        1e-8,
        0.01,
        limits(),
    )
    .unwrap();
    let body = match attempt {
        Attempt::Certified(body) => body,
        Attempt::Refused(report) => panic!(
            "reason={:?} wall={:?} caps={:?} regular={:?} volume={:?}",
            report.reason,
            report.wall,
            report.cap_errors,
            report.regularity.map(|r| r.unresolved_faces),
            report.volume.map(|r| (
                r.boundary.agreement.all_equal,
                r.boundary.intersections.faces.all_faces_injective,
                r.boundary
                    .intersections
                    .pairs
                    .pairs
                    .iter()
                    .filter(|p| p.reason == "pair-unresolved")
                    .map(|p| p.faces)
                    .collect::<Vec<_>>()
            ))
        ),
    };
    assert!(
        body.report().continuous_bound && body.report().within_budget && body.report().certified
    );
    assert!(body.report().boundary_error_upper.unwrap() <= 0.01);
    assert_eq!(body.report().caps.len(), 2);
    let ideal_limits=IdealLimits {projection:[[1.,0.,0.],[0.,1.,0.],[0.,0.,1./64.]],cells:100000,exact_work:1000000};
    let stronger=construct_with_ideal(&sweep,&[4],5,[&source,&source],[&retained[0],&retained[1]],1e-8,0.01,limits(),ideal_limits).unwrap();
    let Attempt::Certified(stronger)=stronger else {panic!("Original body embedding was not certified");};
    let proof=stronger.report().ideal_volume.as_ref().unwrap();
    assert!(proof.embedding_certified&&proof.regularity_certified&&proof.parameter_topology_preserved);
    assert_eq!(stronger.model(),body.model());
    let short=construct_with_ideal(&sweep,&[4],5,[&source,&source],[&retained[0],&retained[1]],1e-8,0.01,limits(),IdealLimits {cells:proof.cells-1,..ideal_limits}).unwrap();
    assert!(matches!(short,Attempt::Refused(_)));

    #[cfg(feature = "codec")]
    if let Ok(path_out) = std::env::var("OSV_GENERAL_CERTIFICATE_FIXTURE") {
        emit_fixture(
            &path_out,
            &source,
            &retained,
            &profiles,
            [&path, &scale, &twist, &axis, &normal],
            body.model(),
        );
    }

    assert!(
        body.report()
            .volume
            .as_ref()
            .unwrap()
            .boundary
            .intersections
            .pairs
            .pairs
            .iter()
            .any(|p| matches!(
                p.boundary,
                Some(crate::face_contacts::SharedBoundary::JoinedSourceCharts(_))
            ))
    );
    let original = (
        profiles.clone(),
        source.surface.clone(),
        retained[0].surface.clone(),
        retained[1].surface.clone(),
    );
    let mut changed = source.clone();
    changed.surface.control_points[1][1][2] = changed.surface.control_points[1][1][2].next_up();
    let altered = construct(
        &sweep,
        &[4],
        5,
        [&changed, &source],
        [&retained[0], &retained[1]],
        1e-8,
        0.01,
        limits(),
    )
    .unwrap();
    assert!(matches!(altered, Attempt::Refused(_)));
    assert_eq!(
        (
            profiles.clone(),
            source.surface.clone(),
            retained[0].surface.clone(),
            retained[1].surface.clone()
        ),
        original
    );

    let nonunit = |mut c: LoftCap| {
        c.surface.knots_u = vec![2., 2., 4., 4.];
        c.surface.knots_v = vec![2., 2., 4., 4.];
        for p in c
            .trims
            .iter_mut()
            .flatten()
            .flat_map(|c| &mut c.control_points)
        {
            for value in p {
                *value = 2. + 2. * (*value);
            }
        }
        c
    };
    let source_nonunit = nonunit(source.clone());
    let retained_nonunit = [nonunit(retained[0].clone()), nonunit(retained[1].clone())];
    let rebuilt = construct(
        &sweep,
        &[4],
        5,
        [&source_nonunit, &source_nonunit],
        [&retained_nonunit[0], &retained_nonunit[1]],
        1e-8,
        0.01,
        limits(),
    )
    .unwrap();
    assert!(
        matches!(rebuilt, Attempt::Certified(_)),
        "Exact nonunit cap domains must be supported"
    );
    for cells in [0, 1, body.report().source_cells - 1] {
        let limited = construct(
            &sweep,
            &[4],
            5,
            [&source, &source],
            [&retained[0], &retained[1]],
            1e-8,
            0.01,
            Limits {
                source_cells: cells,
                ..limits()
            },
        )
        .unwrap();
        let Attempt::Refused(r) = limited else {
            panic!("short source allowance accepted")
        };
        assert!(!r.certified && r.source_cells <= cells);
    }
}

#[cfg(feature = "codec")]
fn emit_fixture(
    path: &str,
    source: &LoftCap,
    retained: &[LoftCap; 2],
    profiles: &[Curve],
    curves: [&Curve; 5],
    model: &crate::Model,
) {
    use value_codec::json;
    let [path_curve, scale, twist, axis, normal] = curves;
    let cap = |c: &LoftCap| json!({"surface":c.surface,"trims":c.trims});
    let l = limits();
    let v = l.volume;
    let b = v.boundary;
    let c = b.contacts;
    let volume = json!({"maxExactWork":b.exact_work,"maxTrimPairs":b.trim_pairs,"maxTrimCells":b.trim_cells,
        "maxTrimDomainCells":b.trim_domain_cells,"maxSpans":b.spans,"maxPairs":c.pairs,"maxCells":c.cells,
        "maxDomainCells":c.domain_cells,"cellsPerPair":c.cells_per_pair,"domainCellsPerPair":c.domain_cells_per_pair,
        "nestingPairs":v.nesting_pairs,"nestingCells":v.nesting_cells,"nestingDomainCells":v.nesting_domain_cells,
        "orientationCells":v.orientation_cells,"orientationDomainCells":v.orientation_domain_cells,"orientationSpans":v.orientation_spans});
    let request = json!({"op":"brep_sweep_boundary_construct","source":{"profiles":profiles,"path":path_curve,
        "scale":scale,"twist":twist,"frame_axis":axis,"frame_normal":normal,"normal":[1.,0.,0.],
        "orientation":"authored","spacing":"parameter","initial_sections":5,"max_sections":5,"max_deviation":0.01},
        "loopSizes":[4],"sections":5,"sourceCaps":[cap(source),cap(source)],"retainedCaps":[cap(&retained[0]),cap(&retained[1])],
        "toleranceUv":1e-8,"maxDeviation":0.01,"limits":{"sourceCells":l.source_cells,"sourceProducts":l.source_products,
            "sourceExactWork":l.source_exact_work,"regularityCells":l.regularity_cells,"linearCells":l.linear_cells,"volume":volume}});
    let common = json!({"op":"brep_boundary_certificate","reference":model,"retained":model,"toleranceUv":1e-8,
        "maxDeviation":0.,"limits":{"correspondenceCells":10000,"correspondenceProducts":1000000,
            "regularityCells":10000,"linearCellsPerBoundary":10000,"referenceVolume":volume,"retainedVolume":volume}});
    std::fs::write(
        path,
        value_codec::to_string(&json!({"sweep":request,"common":common})).unwrap(),
    )
    .unwrap();
}

#[test]
fn moving_frame_guide_affine_weights_and_station_policy_matrix() {
    let mut checked = 0;
    for guided in [false, true] {
        for affine in [false, true] {
            for rational in [false, true] {
                for arc in [false, true] {
                    let id =
                        format!("guide={guided}/affine={affine}/rational={rational}/arc={arc}");
                    let corners = vec![
                        vec![0., 0., 0.],
                        vec![1., 0., 0.],
                        vec![1., 1., 1. / 64.],
                        vec![0., 1., 0.],
                    ];
                    let mut source = cap(corners.clone());
                    let weights = if rational { [1., 2., 4., 2.] } else { [1.; 4] };
                    source.surface.weights =
                        vec![vec![weights[0], weights[3]], vec![weights[1], weights[2]]];
                    let profiles = (0..4)
                        .map(|i| Curve {
                            degree: 1,
                            knots: vec![0., 0., 1., 1.],
                            control_points: vec![corners[i].clone(), corners[(i + 1) % 4].clone()],
                            weights: vec![weights[i], weights[(i + 1) % 4]],
                            periodic: false,
                        })
                        .collect::<Vec<_>>();
                    let path = nurbs_core::primitives::line([0.; 3], [0., 0., 64.]).unwrap();
                    let scale = constant_vector_law([1., 0., 0.]).unwrap();
                    let twist = constant_vector_law([0.; 3]).unwrap();
                    let axis = constant_vector_law([0., 0., 1.]).unwrap();
                    let normal =
                        nurbs_core::primitives::line([1., 0., 0.], [1., 0.125, 0.]).unwrap();
                    let guide =
                        nurbs_core::primitives::line([1., 0., 0.], [1., 0.125, 64.]).unwrap();
                    let axes = constant_vector_law([1.25, 0.75, 1.]).unwrap();
                    let center = constant_vector_law([0.125, -0.25, 0.]).unwrap();
                    let options = Options {
                        normal: [1., 0., 0.],
                        orientation: if guided {
                            Orientation::RotationMinimizing
                        } else {
                            Orientation::Fixed
                        },
                        spacing: if arc {
                            Spacing::ArcLength {
                                tolerance: 1e-8,
                                max_cells: 10000,
                            }
                        } else {
                            Spacing::Parameter
                        },
                        initial_sections: 5,
                        max_sections: 5,
                        max_deviation: 0.01,
                    };
                    let sweep = MultiSweep::new(&profiles, &path, &scale, &twist, options).unwrap();
                    let mut sweep = if guided {
                        sweep.with_orientation_guide(&guide).unwrap()
                    } else {
                        sweep.with_frame_laws(&axis, &normal).unwrap()
                    };
                    if affine {
                        sweep = sweep.with_affine_laws(&axes, &center).unwrap();
                    }
                    let rows = sweep.sections_at(5).unwrap();
                    let mut retained = [
                        cap(rows
                            .iter()
                            .map(|r| r[0].control_points[0].clone())
                            .collect()),
                        cap(rows
                            .iter()
                            .map(|r| r[4].control_points[0].clone())
                            .collect()),
                    ];
                    for c in &mut retained {
                        c.surface.weights = source.surface.weights.clone();
                    }
                    let attempt = construct(
                        &sweep,
                        &[4],
                        5,
                        [&source, &source],
                        [&retained[0], &retained[1]],
                        1e-8,
                        0.01,
                        limits(),
                    )
                    .unwrap();
                    match attempt {
                        Attempt::Certified(body) => {
                            assert!(body.report().boundary_error_upper.unwrap() <= 0.01, "{id}");
                            checked += 1;
                        }
                        Attempt::Refused(r) => panic!(
                            "{id}: {:?}; wall={:?} caps={:?} volume={:?}",
                            r.reason,
                            r.wall.error_certificate_reason,
                            r.cap_errors,
                            r.volume.map(|v| v
                                .boundary
                                .intersections
                                .pairs
                                .pairs
                                .iter()
                                .filter(|p| p.reason == "pair-unresolved")
                                .map(|p| p.faces)
                                .collect::<Vec<_>>())
                        ),
                    }
                }
            }
        }
    }
    assert_eq!(checked, 16);
}

#[test]
fn quadratic_filled_caps_glue_to_ruled_walls_without_degree_elevation() {
    let mut source = cap(vec![
        vec![0., 0., 0.],
        vec![1., 0., 0.],
        vec![1., 1., 1. / 64.],
        vec![0., 1., 0.],
    ]);
    source.surface =
        nurbs_core::surface_edit::elevate(&source.surface, nurbs_core::surface::Axis::U, 2)
            .unwrap();
    source.surface =
        nurbs_core::surface_edit::elevate(&source.surface, nurbs_core::surface::Axis::V, 2)
            .unwrap();
    source.surface.control_points[1][1][2] += 0.125;
    let p = &source.surface.control_points;
    let curve = |control_points| Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points,
        weights: vec![1.; 3],
        periodic: false,
    };
    let profiles = vec![
        curve((0..3).map(|i| p[i][0].clone()).collect()),
        curve(p[2].clone()),
        curve((0..3).rev().map(|i| p[i][2].clone()).collect()),
        curve(p[0].iter().rev().cloned().collect()),
    ];
    let path = nurbs_core::primitives::line([0.; 3], [0., 0., 64.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let axis = constant_vector_law([0., 0., 1.]).unwrap();
    let normal = nurbs_core::primitives::line([1., 0., 0.], [1., 0.125, 0.]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [1., 0., 0.],
            orientation: Orientation::Fixed,
            spacing: Spacing::Parameter,
            initial_sections: 5,
            max_sections: 5,
            max_deviation: 0.01,
        },
    )
    .unwrap()
    .with_frame_laws(&axis, &normal)
    .unwrap();
    let rows = sweep.sections_at(5).unwrap();
    let caps = (0..2)
        .map(|endpoint| {
            let mut c = source.clone();
            c.surface = sweep
                .endpoint_surface_image(&source.surface, endpoint, 100000, 1000000)
                .unwrap()
                .surface
                .unwrap();
            let j = endpoint * 4;
            for i in 0..3 {
                c.surface.control_points[i][0] = rows[0][j].control_points[i].clone();
                c.surface.control_points[2][i] = rows[1][j].control_points[i].clone();
                c.surface.control_points[2 - i][2] = rows[2][j].control_points[i].clone();
                c.surface.control_points[0][2 - i] = rows[3][j].control_points[i].clone();
            }
            c
        })
        .collect::<Vec<_>>();
    let attempt = construct(
        &sweep,
        &[4],
        5,
        [&source, &source],
        [&caps[0], &caps[1]],
        1e-8,
        0.01,
        limits(),
    )
    .unwrap();
    let body = match attempt {
        Attempt::Certified(body) => body,
        Attempt::Refused(r) => panic!(
            "{:?} volume={:?}",
            r.reason,
            r.volume.map(|v| v
                .boundary
                .intersections
                .pairs
                .pairs
                .iter()
                .filter(|p| p.reason == "pair-unresolved")
                .map(|p| p.faces)
                .collect::<Vec<_>>())
        ),
    };
    assert!(body.report().continuous_bound && body.report().within_budget);
    assert!(
        body.model()
            .faces
            .iter()
            .any(|f| f.surface.degree_u == 2 && f.surface.degree_v == 2)
    );
    let mut changed = source.clone();
    changed.surface.control_points[1][1][2] += 0.25;
    let rejected = construct(
        &sweep,
        &[4],
        5,
        [&changed, &source],
        [&caps[0], &caps[1]],
        1e-8,
        0.01,
        limits(),
    )
    .unwrap();
    let Attempt::Refused(r) = rejected else {
        panic!("Unchanged cap contour hid an interior displacement");
    };
    assert!(r.continuous_bound && !r.within_budget && !r.certified);
    assert!(r.boundary_error_upper.unwrap() >= 0.25);
    assert_eq!(r.reason, Some("boundary-error-budget-exceeded"));
}

#[test]
fn spatial_bishop_transport_and_saddle_caps_work_at_six_nondyadic_stations() {
    let corners = vec![
        vec![0., 0., 0.],
        vec![1., 0., 0.],
        vec![1., 1., 1. / 64.],
        vec![0., 1., 0.],
    ];
    let source = cap(corners.clone());
    let profiles = (0..4)
        .map(|i| Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![corners[i].clone(), corners[(i + 1) % 4].clone()],
            weights: vec![1.; 2],
            periodic: false,
        })
        .collect::<Vec<_>>();
    let path = nurbs_core::paths::bezier(
        vec![
            vec![0.; 3],
            vec![0.0625, 0.03125, 64. / 3.],
            vec![0.125, 0., 128. / 3.],
            vec![0.1875, 0.03125, 64.],
        ],
        None,
    )
    .unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [1., 0., 0.],
            orientation: Orientation::RotationMinimizing,
            spacing: Spacing::Parameter,
            initial_sections: 6,
            max_sections: 6,
            max_deviation: 0.01,
        },
    )
    .unwrap()
    .with_spatial_rmf_error_limits(512, 100000, 1000000)
    .unwrap();
    let rows = sweep.sections_at(6).unwrap();
    let retained = [
        cap(rows
            .iter()
            .map(|r| r[0].control_points[0].clone())
            .collect()),
        cap(rows
            .iter()
            .map(|r| r[5].control_points[0].clone())
            .collect()),
    ];
    let attempt = construct(
        &sweep,
        &[4],
        6,
        [&source, &source],
        [&retained[0], &retained[1]],
        1e-8,
        0.01,
        limits(),
    )
    .unwrap();
    let body = match attempt {
        Attempt::Certified(b) => b,
        Attempt::Refused(r) => panic!(
            "reason={:?} wall={:?} cap={:?} volume={:?}",
            r.reason,
            r.wall,
            r.cap_errors,
            r.volume.map(|v| v
                .boundary
                .intersections
                .pairs
                .pairs
                .iter()
                .filter(|p| p.reason == "pair-unresolved")
                .map(|p| p.faces)
                .collect::<Vec<_>>())
        ),
    };
    assert!(body.report().continuous_bound && body.report().within_budget);
    let ideal=IdealLimits{projection:[[1.,0.,0.],[0.,1.,0.],[0.,0.,1./64.]],cells:100000,exact_work:1000000};
    let stronger=construct_with_ideal(&sweep,&[4],6,[&source,&source],[&retained[0],&retained[1]],1e-8,0.01,limits(),ideal).unwrap();
    let Attempt::Certified(stronger)=stronger else {panic!("Original spatial Bishop volume refused");};
    assert!(stronger.report().ideal_volume.as_ref().unwrap().embedding_certified);assert_eq!(stronger.model(),body.model());

    assert!(
        body.model()
            .faces
            .iter()
            .any(|f| f.surface.knots_v.contains(&0.2))
    );
    assert!(body.report().source_cells <= limits().source_cells);
}

#[test]
fn original_volume_preserves_a_hole_only_after_complete_native_trim_binding() {
    let mut source=cap(vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![1.,1.,0.],vec![0.,1.,0.]]);
    let hole=[[0.25,0.25],[0.25,0.75],[0.75,0.75],[0.75,0.25]];
    source.trims.push((0..4).map(|i|Curve {degree:1,knots:vec![0.,0.,1.,1.],
        control_points:vec![hole[i].to_vec(),hole[(i+1)%4].to_vec()],weights:vec![1.;2],periodic:false}).collect());
    let profiles=source.trims.iter().flatten().map(|c| {
        let a=&c.control_points[0];let b=&c.control_points[1];
        nurbs_core::primitives::line([a[0],a[1],0.],[b[0],b[1],0.]).unwrap()
    }).collect::<Vec<_>>();
    let path=nurbs_core::primitives::line([0.;3],[0.,0.,64.]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
    let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,Options{normal:[1.,0.,0.],orientation:Orientation::Fixed,
        spacing:Spacing::Parameter,initial_sections:5,max_sections:5,max_deviation:0.01}).unwrap();
    let mut end=source.clone();for p in end.surface.control_points.iter_mut().flatten() {p[2]+=64.;}
    let ideal=IdealLimits{projection:[[1.,0.,0.],[0.,1.,0.],[0.,0.,1./64.]],cells:100000,exact_work:1000000};
    let attempt=construct_with_ideal(&sweep,&[4,4],5,[&source,&source],[&source,&end],1e-8,0.01,limits(),ideal).unwrap();
    let body=match attempt {Attempt::Certified(body)=>body,Attempt::Refused(r)=>panic!("Hollow source: {:?}",r.reason)};
    assert!(body.report().ideal_volume.as_ref().unwrap().parameter_topology_preserved);
    assert_eq!(body.model().bodies.len(),1);assert_eq!(body.model().faces.iter().filter(|f|!f.holes.is_empty()).count(),2);
    let mut omitted=source.clone();omitted.trims.pop();
    assert!(matches!(construct_with_ideal(&sweep,&[4,4],5,[&omitted,&source],[&source,&end],1e-8,0.01,limits(),ideal).unwrap(),Attempt::Refused(_)));
}
