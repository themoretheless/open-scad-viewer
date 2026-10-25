use super::*;
fn ring(radius: f64) -> Vec<Curve> {
    (0..4)
        .map(|quadrant| {
            let xy = [[radius, 0.], [radius, radius], [0., radius]];
            Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: xy
                    .into_iter()
                    .map(|[x, y]| {
                        let (x, y) = match quadrant {
                            0 => (x, y),
                            1 => (-y, x),
                            2 => (-x, -y),
                            _ => (y, -x),
                        };
                        vec![x, y, 0.]
                    })
                    .collect(),
                weights: vec![1., 1., 2.],
                periodic: false,
            }
        })
        .collect()
}
fn material(hole: bool) -> (Vec<Vec<Curve>>, LoftCap) {
    let mut profiles = vec![ring(2.)];
    if hole {
        profiles.push(
            ring(1.)
                .into_iter()
                .rev()
                .map(|c| c.reverse().unwrap())
                .collect(),
        );
    }
    let trims = profiles
        .iter()
        .map(|r| {
            r.iter()
                .map(|c| {
                    let mut c = c.clone();
                    for p in &mut c.control_points {
                        p.pop();
                    }
                    c
                })
                .collect()
        })
        .collect();
    let q = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-2., -2., 2., 2.],
        knots_v: vec![-2., -2., 2., 2.],
        control_points: vec![
            vec![vec![-2., -2., 0.], vec![-2., 2., 0.]],
            vec![vec![2., -2., 0.], vec![2., 2., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    (profiles, LoftCap { surface: q, trims })
}
fn motion() -> Vec<RationalPiece> {
    vec![RationalPiece {
        controls: [1., 1.25, 1.5, 1.5, 1.25, 1.]
            .into_iter()
            .zip([0., 0., 0., 1., 1., 1.])
            .map(|(a, z)| [[a, 0., 0.], [0., a, 0.], [0.; 3], [0., 0., z]])
            .collect(),
        weights: vec![1.; 6],
    }]
}
fn proposal() -> fibered_volume::Proposal {
    fibered_volume::Proposal {
        source_normal: [0., 0., 1.],
        world_rows: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
    }
}
fn limits() -> FiberedBodyLimits {
    FiberedBodyLimits {
        volume: fibered_volume::Limits {
            cells: 100000,
            exact_work: 1000000,
        },
        material_pairs: 100000,
        material_cells: 100000,
        material_domain_cells: 1000000,
        profile_cells: 100000,
        source_trace_work: 1000000,
        binding_rows: 1000000,
        agreement_work: 1000000,
        regularity_cells: 100000,
        normal_cells: 100000,
        smoothness_work: 1000000,
        order: 2,
    }
}
fn body(attempt: FiberedBodyAttempt) -> Box<FiberedBody> {
    match attempt {
        FiberedBodyAttempt::Certified(b) => b,
        FiberedBodyAttempt::Refused(r) => panic!(
            "refused {:?}; normal={} regular={:?}, volume={:?} trace={} smooth={}",
            r.reason,
            r.normal_cells,
            r.regularity.as_ref().map(|x| &x.unresolved_faces),
            r.volume.as_ref().map(|x| x.embedding_certified),
            r.source_trace_work,
            r.smoothness_work
        ),
    }
}
#[test]
fn original_filled_flat_end_caps_and_walls_have_global_source_ownership_and_complete_g2() {
    let (p, m) = material(false);
    let motion = motion();
    let b =
        body(construct_fibered_body(&p, &m, [0.; 3], &motion, proposal(), 1e-8, limits()).unwrap());
    let r = b.report();
    assert!(
        r.certified
            && r.source_g1
            && r.source_g2
            && r.topology_preserved
            && r.absence_of_self_intersections
            && r.source_boundary_exact
    );
    assert_eq!(r.boundary_error_upper, Some(0.));
    assert_eq!(r.inspected_profile_joins, 4);
    assert_eq!(b.model().faces.len(), 6);
    assert_eq!(
        r.volume.as_ref().unwrap().known_singular_parameter_ends,
        [true; 2]
    );
    assert!(r.regularity.as_ref().unwrap().all_faces_regular);
    let before = b.model().clone();
    let a = body(
        verify_fibered_body(
            &p,
            &m,
            [0.; 3],
            &motion,
            proposal(),
            1e-8,
            limits(),
            &before,
        )
        .unwrap(),
    );
    assert_eq!(a.model(), &before);
    assert_eq!(before, *b.model());
    eprintln!(
        "flat physical caps source/actual G2; normals={} regularity={} binding={} trace={} smooth={}",
        r.normal_cells,
        r.regularity.as_ref().unwrap().cells,
        r.binding_rows,
        r.source_trace_work,
        r.smoothness_work
    );
}
#[test]
fn an_inner_wall_tangent_cusp_is_c0_and_cannot_be_promoted_to_g1_or_g2() {
    let (p, m) = material(true);
    let motion = motion();
    let mut l = limits();
    l.order = 0;
    let b = body(construct_fibered_body(&p, &m, [0.; 3], &motion, proposal(), 1e-8, l).unwrap());
    assert!(b.report().topology_preserved);
    assert!(!b.report().source_g1 && !b.report().source_g2);
    assert_eq!(b.report().seam_continuity, "C0");
    for order in [1, 2] {
        l.order = order;
        let x = construct_fibered_body(&p, &m, [0.; 3], &motion, proposal(), 1e-8, l).unwrap();
        let FiberedBodyAttempt::Refused(r) = x else {
            panic!("cusp got G{order} Solid")
        };
        assert!(!r.certified && !r.source_g1 && !r.source_g2);
        assert_eq!(
            r.reason,
            Some("complete-cap-wall-oriented-regularity-unproved")
        );
    }
}
#[test]
fn source_binding_budget_failures_mutations_and_invalid_tail_inputs_never_return_partial_models() {
    let (p, m) = material(false);
    let motion = motion();
    let b =
        body(construct_fibered_body(&p, &m, [0.; 3], &motion, proposal(), 1e-8, limits()).unwrap());
    for stage in 0..7 {
        let mut l = limits();
        match stage {
            0 => l.volume.exact_work = 0,
            1 => l.volume.cells = b.report().volume.as_ref().unwrap().cells - 1,
            2 => l.source_trace_work = b.report().source_trace_work - 1,
            3 => l.binding_rows = b.report().binding_rows - 1,
            4 => l.agreement_work = b.report().agreement.as_ref().unwrap().work - 1,
            5 => l.smoothness_work = b.report().smoothness_work - 1,
            6 => l.normal_cells = b.report().normal_cells - 1,
            _ => unreachable!(),
        }
        let FiberedBodyAttempt::Refused(r) =
            construct_fibered_body(&p, &m, [0.; 3], &motion, proposal(), 1e-8, l).unwrap()
        else {
            panic!("short stage {stage} returned Solid")
        };
        assert!(!r.certified && !r.source_g1 && !r.source_g2 && !r.topology_preserved);
    }
    let mut changed = b.model().clone();
    changed.faces[0].surface.control_points[1][1][0] =
        changed.faces[0].surface.control_points[1][1][0].next_up();
    let before = changed.clone();
    assert!(matches!(
        verify_fibered_body(
            &p,
            &m,
            [0.; 3],
            &motion,
            proposal(),
            1e-8,
            limits(),
            &changed
        )
        .unwrap(),
        FiberedBodyAttempt::Refused(_)
    ));
    assert_eq!(changed, before);
    let mut l = limits();
    l.volume.cells = 100001;
    l.material_pairs = 0;
    assert!(construct_fibered_body(&p, &m, [0.; 3], &motion, proposal(), 1e-8, l).is_err());
}

#[test]
fn rational_weights_oblique_world_planes_and_descending_fibers_keep_original_and_actual_g2() {
    let (p, m) = material(false);
    for descending in [false, true] {
        let mut motion = motion();
        motion[0].weights = vec![1., 0.3, 2., 0.7, 3., 2.];
        for (pose, b) in motion[0]
            .controls
            .iter_mut()
            .zip([0., 0.25, 0.5, 0.5, 0.25, 0.])
        {
            pose[0][2] = pose[0][0];
            pose[3][0] = b;
            pose[3][2] += b;
            if descending {
                pose[3][0] = -pose[3][0];
                pose[3][2] = -pose[3][2];
            }
        }
        let q = fibered_volume::Proposal {
            source_normal: [0., 0., 0.7],
            world_rows: [[1., 0., 0.], [0., 1., 0.], [-0.3, 0., 0.3]],
        };
        let b = body(construct_fibered_body(&p, &m, [0.; 3], &motion, q, 1e-8, limits()).unwrap());
        assert!(b.report().source_g2);
        assert_eq!(
            b.report().volume.as_ref().unwrap().height_direction,
            Some(if descending { -1 } else { 1 })
        );
        let a = body(
            verify_fibered_body(&p, &m, [0.; 3], &motion, q, 1e-8, limits(), b.model()).unwrap(),
        );
        assert_eq!(a.model(), b.model());
        eprintln!(
            "rational oblique descending={descending} source/actual caps G2 verified, normal cells={}",
            b.report().normal_cells
        );
    }
}
#[test]
fn sharp_profile_corners_remain_explicit_c0_despite_flat_end_cap_jets() {
    let (_, mut m) = material(false);
    let xy = [[-1., -1.], [1., -1.], [1., 1.], [-1., 1.]];
    let ring = (0..4)
        .map(|i| {
            nurbs_core::primitives::line(
                [xy[i][0], xy[i][1], 0.],
                [xy[(i + 1) % 4][0], xy[(i + 1) % 4][1], 0.],
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    m.trims = vec![
        ring.iter()
            .map(|c| {
                let mut c = c.clone();
                for p in &mut c.control_points {
                    p.pop();
                }
                c
            })
            .collect(),
    ];
    let profiles = vec![ring];
    let motion = motion();
    let mut l = limits();
    l.order = 0;
    let b =
        body(construct_fibered_body(&profiles, &m, [0.; 3], &motion, proposal(), 1e-8, l).unwrap());
    assert!(b.report().certified && !b.report().source_g1 && !b.report().source_g2);
    for order in [1, 2] {
        l.order = order;
        let FiberedBodyAttempt::Refused(r) =
            construct_fibered_body(&profiles, &m, [0.; 3], &motion, proposal(), 1e-8, l).unwrap()
        else {
            panic!("sharp corners got G{order}")
        };
        assert_eq!(r.reason, Some("original-profile-geometric-jets-unproved"));
    }
}

fn three_pieces() -> Vec<RationalPiece> {
    let scales = [
        [1., 1.25, 1.5, 1.5, 1.5, 1.5],
        [1.5; 6],
        [1.5, 1.5, 1.5, 1.5, 1.25, 1.],
    ];
    let heights = [
        [0., 0., 0., 0.5, 0.75, 1.],
        [1., 1.25, 1.5, 1.5, 1.75, 2.],
        [2., 2.25, 2.5, 3., 3., 3.],
    ];
    scales
        .into_iter()
        .zip(heights)
        .map(|(a, h)| RationalPiece {
            controls: a
                .into_iter()
                .zip(h)
                .map(|(a, z)| [[a, 0., 0.], [0., a, 0.], [0.; 3], [0., 0., z]])
                .collect(),
            weights: vec![1.; 6],
        })
        .collect()
}
#[test]
fn all_original_pose_seams_are_owned_even_at_nonbinary_global_station_parameters() {
    let (p, m) = material(false);
    let motion = three_pieces();
    let b =
        body(construct_fibered_body(&p, &m, [0.; 3], &motion, proposal(), 1e-8, limits()).unwrap());
    assert!(b.report().source_g2);
    assert_eq!(b.report().inspected_motion_joins, 2);
    assert_eq!(b.model().faces.len(), 14);
    let a = body(
        verify_fibered_body(
            &p,
            &m,
            [0.; 3],
            &motion,
            proposal(),
            1e-8,
            limits(),
            b.model(),
        )
        .unwrap(),
    );
    assert_eq!(a.model(), b.model());
    let mut g1 = motion;
    g1[1].controls[2][0][0] = 1.625;
    let FiberedBodyAttempt::Refused(r) =
        construct_fibered_body(&p, &m, [0.; 3], &g1, proposal(), 1e-8, limits()).unwrap()
    else {
        panic!("C1-only original pose seam got G2")
    };
    assert_eq!(r.reason, Some("original-multipiece-motion-jets-unproved"));
    let mut l = limits();
    l.order = 1;
    let b = body(construct_fibered_body(&p, &m, [0.; 3], &g1, proposal(), 1e-8, l).unwrap());
    assert!(b.report().source_g1 && !b.report().source_g2);
}
