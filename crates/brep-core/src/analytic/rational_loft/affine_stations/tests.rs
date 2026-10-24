use super::*;
use nurbs_core::progressive_sweep::{
    MultiSweep, Options, Orientation, Spacing, constant_vector_law,
};

fn profiles(sharp: bool) -> Vec<Vec<Curve>> {
    let r = 1. / 128.;
    let p = [
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
    vec![
        (0..4)
            .map(|i| Curve {
                degree: if sharp { 1 } else { 2 },
                knots: if sharp {
                    vec![0., 0., 1., 1.]
                } else {
                    vec![0., 0., 0., 1., 1., 1.]
                },
                control_points: if sharp {
                    vec![p[2 * i], p[2 * i + 2]]
                } else {
                    p[2 * i..=2 * i + 2].to_vec()
                }
                .iter()
                .map(|p| vec![0., p[0] * r, p[1] * r])
                .collect(),
                weights: if sharp {
                    vec![1.; 2]
                } else {
                    vec![1., 0.5, 1.]
                },
                periodic: false,
            })
            .collect(),
    ]
}
fn sections(source: &[Vec<Curve>]) -> Vec<Vec<Vec<Curve>>> {
    sections_mode(source, 0)
}
fn sections_mode(source: &[Vec<Curve>], mode: usize) -> Vec<Vec<Vec<Curve>>> {
    let path = Curve {
        degree: 9,
        knots: [vec![-3.; 10], vec![5.; 10]].concat(),
        control_points: vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![2., 1., 0.],
            vec![3., 3., 1.],
            vec![2., 4., 6.],
            vec![-2., 4., -4.],
            vec![-3., 3., -1.],
            vec![-2., 1., 0.],
            vec![-1., 0., 0.],
            vec![0., 0., 0.],
        ],
        weights: vec![1.; 10],
        periodic: false,
    };
    let scalar = |x: [f64; 5], a: f64, b: f64| Curve {
        degree: 4,
        knots: [vec![a; 5], vec![b; 5]].concat(),
        control_points: x.map(|x| vec![x, 0., 0.]).to_vec(),
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
    let axes = constant_vector_law([1., 1.125, 0.875]).unwrap();
    let center = constant_vector_law([1. / 1024., 1. / 2048., 0.]).unwrap();
    let mut guide = path.clone();
    for p in &mut guide.control_points {
        p[2] += 1.;
    }
    let mut sweep = MultiSweep::new(
        &source[0],
        &path,
        &scale,
        &twist,
        Options {
            normal: [0., 0., 1.],
            orientation: if mode == 1 {
                Orientation::Fixed
            } else if mode == 2 {
                Orientation::FixedNormal
            } else {
                Orientation::RotationMinimizing
            },
            spacing: Spacing::Parameter,
            initial_sections: 33,
            max_sections: 129,
            max_deviation: 0.05,
        },
    )
    .unwrap();
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
    let rows = sweep.sections_at(33).unwrap();
    (0..33)
        .map(|i| vec![rows.iter().map(|r| r[i].clone()).collect()])
        .collect()
}
fn limits() -> AffineStationLimits {
    AffineStationLimits {
        quantum: 2f64.powi(-18),
        tolerance: 0.03,
        work: 1000000,
        matrix_cells: 100000,
    }
}
fn complete(model: &Model) -> crate::sweep_smoothness::complete::Report {
    crate::sweep_smoothness::complete::inspect(
        model,
        crate::sweep_smoothness::complete::Limits {
            order: 2,
            exact_work: 2000000,
            agreement_work: 1000000,
            regularity_cells: 100000,
        },
    )
    .unwrap()
}
#[test]
fn one_affine_spatial_motion_preserves_all_original_profile_and_closed_station_g2_joins() {
    let source = profiles(false);
    let rows = sections(&source);
    let before = (source.clone(), rows.clone());
    let c = affine_station_walls(&source, &rows, &[], true, limits()).unwrap();
    assert!(
        c.affine_regular && c.sides.is_some(),
        "{} E={:?} work={} cells={}",
        c.reason,
        c.displacement_upper,
        c.work,
        c.matrix_cells
    );
    let model = crate::analytic::periodic_material_loft_surfaces(
        c.sections.as_ref().unwrap(),
        c.sides.as_ref().unwrap(),
    )
    .unwrap();
    model.validate().unwrap();
    let s = complete(&model);
    eprintln!(
        "affine E={:?} work={} matrices={} edges={} complete={} links={} regular={} g1={} g2={} unresolved={:?} proofwork={}",
        c.displacement_upper,
        c.work,
        c.matrix_cells,
        s.edge_ids.len(),
        s.extraction_complete,
        s.vertex_links_complete,
        s.regularity.all_faces_regular,
        s.g1_certified,
        s.g2_certified,
        s.unresolved_edges,
        s.exact_work
    );
    eprintln!(
        "regularity cells={} unresolved={:?}; seam proof={} unresolved={:?}; reasons={:?}",
        s.regularity.cells,
        s.regularity.unresolved_faces,
        s.proof.certified,
        s.proof.unresolved_seams,
        s.proof
            .seams
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.certified)
            .map(|(i, p)| (i, p.exact_identity, p.regularity_certified, p.reason))
            .collect::<Vec<_>>()
    );
    assert_eq!(s.edge_ids.len(), model.edges.len());
    assert!(s.extraction_complete && s.vertex_links_complete && s.regularity.all_faces_regular);
    assert!(s.g1_certified && s.g2_certified);
    assert_eq!((source, rows), before);
}
#[test]
fn original_sharp_profile_is_not_rounded_into_a_smooth_profile() {
    let source = profiles(true);
    let rows = sections(&source);
    let c = affine_station_walls(&source, &rows, &[], true, limits()).unwrap();
    assert!(c.affine_regular && c.sides.is_some(), "{}", c.reason);
    let model = crate::analytic::periodic_material_loft_surfaces(
        c.sections.as_ref().unwrap(),
        c.sides.as_ref().unwrap(),
    )
    .unwrap();
    let s = complete(&model);
    assert!(s.extraction_complete && s.vertex_links_complete && s.regularity.all_faces_regular);
    assert!(!s.g1_certified && !s.g2_certified);
}
#[test]
fn bounded_affine_candidate_refuses_every_short_stage_and_changed_basis() {
    let source = profiles(false);
    let rows = sections(&source);
    let full = affine_station_walls(&source, &rows, &[], true, limits()).unwrap();
    assert!(full.sides.is_some(), "{}", full.reason);
    for l in [
        AffineStationLimits {
            work: 0,
            ..limits()
        },
        AffineStationLimits {
            work: full.work - 1,
            ..limits()
        },
        AffineStationLimits {
            matrix_cells: 0,
            ..limits()
        },
        AffineStationLimits {
            matrix_cells: full.matrix_cells - 1,
            ..limits()
        },
        AffineStationLimits {
            tolerance: full.displacement_upper.unwrap().next_down(),
            ..limits()
        },
    ] {
        let c = affine_station_walls(&source, &rows, &[], true, l).unwrap();
        assert!(
            c.sides.is_none() && c.sections.is_none() && c.poses.is_none(),
            "{}",
            c.reason
        );
    }
    for which in 0..3 {
        let mut bad = rows.clone();
        if which == 0 {
            bad[1][0][0].knots = vec![2., 2., 2., 5., 5., 5.];
        }
        if which == 1 {
            bad[1][0][0].weights[1] = 0.75;
        }
        if which == 2 {
            bad.last_mut().unwrap()[0][0].control_points[0][0] += 1e-8;
        }
        let c = affine_station_walls(&source, &bad, &[], true, limits()).unwrap();
        assert!(c.sides.is_none() && !c.affine_regular, "{}", c.reason);
    }
}
#[test]
fn singular_affine_section_has_no_positive_matrix_certificate() {
    let source = profiles(false);
    let rows = vec![
        vec![
            source[0]
                .iter()
                .map(|c| {
                    let mut c = c.clone();
                    for p in &mut c.control_points {
                        p[2] = 0.;
                    }
                    c
                })
                .collect()
        ];
        4
    ];
    let c = affine_station_walls(&source, &rows, &[], true, limits()).unwrap();
    assert!(c.sides.is_none() && !c.affine_regular, "{}", c.reason);
}

#[test]
fn invertible_station_samples_cannot_hide_a_singular_affine_motion_between_them() {
    let source = profiles(false);
    let mut mirrored = source.clone();
    for c in mirrored.iter_mut().flatten() {
        for p in &mut c.control_points {
            p[1] = -p[1];
        }
    }
    // Both sampled planar fits extend to nonsingular 3D matrices. Their
    // continuous quintic transition still collapses in the interval interior.
    let rows = vec![source.clone(), mirrored, source.clone()];
    let c = affine_station_walls(&source, &rows, &[], false, limits()).unwrap();
    assert!(
        c.sides.is_none() && !c.affine_regular && c.matrix_cells > 0,
        "{}",
        c.reason
    );
}

#[test]
fn a_nonplanar_cubic_original_profile_keeps_all_g2_joins_under_spatial_motion() {
    let r = 1. / 128.;
    let xy = [
        [[1., 0.], [1., 0.5], [0.5, 1.], [0., 1.]],
        [[0., 1.], [-0.5, 1.], [-1., 0.5], [-1., 0.]],
        [[-1., 0.], [-1., -0.5], [-0.5, -1.], [0., -1.]],
        [[0., -1.], [0.5, -1.], [1., -0.5], [1., 0.]],
    ];
    let source = vec![
        (0..4)
            .map(|i| {
                let a = if i % 2 == 0 { r / 16. } else { -r / 16. };
                Curve {
                    degree: 3,
                    knots: vec![-3., -3., -3., -3., 5., 5., 5., 5.],
                    control_points: (0..4)
                        .map(|j| vec![if j < 2 { a } else { -a }, xy[i][j][0] * r, xy[i][j][1] * r])
                        .collect(),
                    weights: vec![1.; 4],
                    periodic: false,
                }
            })
            .collect(),
    ];
    let rows = sections(&source);
    let original = (source.clone(), rows.clone());
    let c = affine_station_walls(&source, &rows, &[], true, limits()).unwrap();
    assert!(c.sides.is_some() && c.affine_regular, "{}", c.reason);
    let model = crate::analytic::periodic_material_loft_surfaces(
        c.sections.as_ref().unwrap(),
        c.sides.as_ref().unwrap(),
    )
    .unwrap();
    let l = crate::sweep_smoothness::complete::Limits {
        order: 2,
        exact_work: 4_000_000,
        agreement_work: 1_000_000,
        regularity_cells: 100000,
    };
    let report = crate::sweep_smoothness::complete::inspect(&model, l).unwrap();
    assert!(
        report.g2_certified && report.edge_ids.len() == model.edges.len(),
        "regular={} seams={} unresolved={:?}",
        report.regularity.all_faces_regular,
        report.proof.certified,
        report.proof.unresolved_seams
    );
    assert_eq!((source, rows), original);
    assert!(report.exact_work > 2_000_000 && report.exact_work <= 4_000_000);
    let short = crate::sweep_smoothness::complete::inspect(
        &model,
        crate::sweep_smoothness::complete::Limits {
            exact_work: report.exact_work - 1,
            ..l
        },
    )
    .unwrap();
    assert!(!short.g2_certified && short.exact_work <= report.exact_work - 1);
    assert!(
        crate::sweep_smoothness::complete::inspect(
            &model,
            crate::sweep_smoothness::complete::Limits {
                exact_work: nurbs_core::continuity::MAX_PROJECTIVE_SEAM_COLLECTION_WORK + 1,
                ..l
            }
        )
        .is_err()
    );
}

#[test]
fn declared_sharp_station_and_short_complete_boundary_budgets_cannot_receive_g2_credit() {
    let source = profiles(false);
    let rows = sections(&source);
    let c = affine_station_walls(&source, &rows, &[], true, limits()).unwrap();
    let model = crate::analytic::periodic_material_loft_surfaces(
        c.sections.as_ref().unwrap(),
        c.sides.as_ref().unwrap(),
    )
    .unwrap();
    let full = complete(&model);
    assert!(full.g2_certified);
    let l = crate::sweep_smoothness::complete::Limits {
        order: 2,
        exact_work: 2000000,
        agreement_work: 1000000,
        regularity_cells: 100000,
    };
    for limit in [
        crate::sweep_smoothness::complete::Limits { exact_work: 0, ..l },
        crate::sweep_smoothness::complete::Limits {
            exact_work: full.exact_work - 1,
            ..l
        },
        crate::sweep_smoothness::complete::Limits {
            agreement_work: 0,
            ..l
        },
        crate::sweep_smoothness::complete::Limits {
            regularity_cells: full.regularity.cells - 1,
            ..l
        },
    ] {
        let r = crate::sweep_smoothness::complete::inspect(&model, limit).unwrap();
        assert!(!r.g1_certified && !r.g2_certified);
    }
    for index in [0, 11, 32] {
        let c = affine_station_walls(&source, &rows, &[index], true, limits()).unwrap();
        assert!(c.sides.is_some(), "{}", c.reason);
        let sharp = crate::analytic::periodic_material_loft_surfaces(
            c.sections.as_ref().unwrap(),
            c.sides.as_ref().unwrap(),
        )
        .unwrap();
        let r = complete(&sharp);
        assert!(r.extraction_complete && !r.g1_certified && !r.g2_certified);
    }
}

fn qualify_frame(mode: usize) {
    let source = profiles(false);
    let rows = sections_mode(&source, mode);
    let before = (source.clone(), rows.clone());
    let c = affine_station_walls(&source, &rows, &[], true, limits()).unwrap();
    assert!(
        c.sides.is_some(),
        "mode={mode}: {} E={:?}",
        c.reason,
        c.displacement_upper
    );
    let model = crate::analytic::periodic_material_loft_surfaces(
        c.sections.as_ref().unwrap(),
        c.sides.as_ref().unwrap(),
    )
    .unwrap();
    let r = complete(&model);
    assert!(
        r.g2_certified && r.edge_ids.len() == model.edges.len(),
        "mode={mode}: regular={} proof={} edges={:?}",
        r.regularity.all_faces_regular,
        r.proof.certified,
        r.proof.unresolved_seams
    );
    assert_eq!((source, rows), before);
}
#[test]
fn authored_spatial_frame_scale_twist_and_affine_laws_keep_every_owned_g2_seam() {
    qualify_frame(1);
}
#[test]
fn guided_spatial_frame_scale_twist_and_affine_laws_keep_every_owned_g2_seam() {
    qualify_frame(2);
}

#[test]
fn actual_closed_quintic_filled_carrier_has_a_global_parameter_volume_atlas() {
    let source = profiles(false);
    let rows = sections(&source);
    let candidate = affine_station_walls(&source, &rows, &[], true, limits()).unwrap();
    assert!(candidate.sides.is_some(), "{}", candidate.reason);
    let r = 1. / 128.;
    let carrier = Surface {
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
    let before = carrier.clone();
    let atlas = nurbs_core::affine_volume::certify(
        &carrier,
        candidate.source_origin.unwrap(),
        candidate.poses.as_ref().unwrap(),
        true,
        [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        1000000,
        1000000,
    )
    .unwrap();
    eprintln!("actual closed quintic carrier atlas: {atlas:?}");
    assert!(
        atlas.embedding_certified
            && atlas.regularity_certified
            && atlas.parameter_topology_preserved,
        "{atlas:?}"
    );
    assert_eq!(carrier, before);
    let pieces = candidate.poses.as_ref().unwrap();
    let twice = [pieces.clone(), pieces.clone()].concat();
    let doubled = nurbs_core::affine_volume::certify(
        &carrier,
        candidate.source_origin.unwrap(),
        &twice,
        true,
        [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        100000,
        1000000,
    )
    .unwrap();
    assert!(
        !doubled.embedding_certified
            && !doubled.regularity_certified
            && !doubled.parameter_topology_preserved,
        "{doubled:?}"
    );
    eprintln!("literal double-cover affine volume refused: {doubled:?}");
}
