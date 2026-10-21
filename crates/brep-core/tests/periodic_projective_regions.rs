use brep_core::profile_certificate::{
    bind_periodic_regions_to_surface, regularize_projective_coherent_periodic_winding as regularize,
};
use nurbs_core::{
    curve::Curve,
    periodic_trim::{self as p, Piece, Report},
    surface::Surface,
};
#[test]
fn nonlinear_original_aliases_reach_owned_regions_and_actual_surface_binding() {
    let a = Curve {
        degree: 4,
        knots: vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.],
        control_points: vec![
            vec![0.5, 0.5],
            vec![0.75, 0.5],
            vec![0.5, 0.75],
            vec![0.25, 0.5],
            vec![0.5, 0.5],
        ],
        weights: vec![1.; 5],
        periodic: false,
    };
    let mut b = a.clone();
    b.weights = vec![1., 2., 4., 8., 16.];
    let part = |c: &Curve| Report {
        pieces: vec![Piece {
            source_domain: c.domain(),
            translation: [0.; 2],
            tile: [0; 2],
            curve: c.clone(),
        }],
        candidate_hulls_contained: false,
    };
    let pa = part(&a);
    let pb = part(&b);
    let receipt =
        p::verify_projective_partition_homotopy(&a, &b, &pa, &pb, [2., 1.], false, 1_000_000)
            .unwrap()
            .unwrap();
    let limits = p::ApproximateChartLimits {
        isotopy: p::IsotopyLimits {
            tolerance_uv: 0.01,
            max_deviation_cells: 40000,
            max_jet_cells: 256,
            max_pairs: 20000,
        },
        max_edges: 128,
        max_images: 256,
        max_cells: 30000,
        max_checks: 10000,
        tolerance_uv: 1e-8,
    };
    let sources = [a.clone(), b.clone()];
    let partitions = [pa.clone(), pb.clone()];
    let regions = regularize(
        &sources,
        &partitions,
        std::slice::from_ref(&receipt),
        [[0., 2.]; 2],
        [0.5, 0.55],
        limits,
        1e-7,
        1_000_000,
    )
    .unwrap();
    regions.reverify_source().unwrap();
    assert_eq!(regions.assembly().transfer.winding, Some(2));
    assert_eq!(regions.candidate_regions().components().len(), 1);
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 2., 2.],
        knots_v: vec![0., 0., 2., 2.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 2., 0.]],
            vec![vec![2., 0., 0.], vec![2., 2., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let bound = bind_periodic_regions_to_surface(
        &surface,
        &regions,
        [false, false],
        p::SurfaceCorrespondenceLimits {
            tolerance_uv: 0.01,
            tolerance_world: 1e-3,
            max_rectangles: 256,
            max_deviation_cells: 100000,
        },
    )
    .unwrap();
    bound.reverify_source().unwrap();
    assert_eq!(bound.surface(), &surface);
    let closed = brep_core::profile_certificate::assemble_periodic_surface_faces(
        &bound,
        brep_core::profile_certificate::PeriodicFaceLimits::default(),
    );
    assert!(
        closed
            .err()
            .unwrap()
            .message
            .contains("explicit chart partition")
    );

    assert!(
        regularize(
            &sources,
            &partitions,
            std::slice::from_ref(&receipt),
            [[0., 2.]; 2],
            [0.5, 0.55],
            limits,
            1e-7,
            0
        )
        .is_err()
    );
    let mut changed = partitions;
    changed[1].pieces[0].curve.weights[1] = 3.;
    assert!(
        regularize(
            &sources,
            &changed,
            &[receipt],
            [[0., 2.]; 2],
            [0.5, 0.55],
            limits,
            1e-7,
            1_000_000
        )
        .is_err()
    );
}

#[test]
fn nonlinear_seam_aliases_own_two_actual_surface_faces_and_one_shared_shore() {
    let a = Curve {
        degree: 4,
        knots: vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.],
        control_points: vec![
            vec![4., 0.5],
            vec![4.5625, 0.6875],
            vec![4.25, 0.75],
            vec![3.8125, 0.6875],
            vec![4., 0.5],
        ],
        weights: vec![1.; 5],
        periodic: false,
    };
    let mut b = a.clone();
    b.weights = vec![1., 3., 9., 27., 81.];
    let chart = [[0., 4.], [0., 1.]];
    // x_A-4=3*t*(t-3/4)*(t-1), so the interior seam is exactly t=3/4.
    // phi_B(t)=3t/(1+2t) carries t=1/2 to t_A=3/4 exactly.
    let pa = p::split_candidates(&a, chart, &[0.75], &[[1, 0], [0, 0]], 2).unwrap();
    let pb = p::split_candidates(&b, chart, &[0.5], &[[1, 0], [0, 0]], 2).unwrap();
    let receipt =
        p::verify_projective_partition_homotopy(&a, &b, &pa, &pb, [3., 1.], false, 1_000_000)
            .unwrap()
            .unwrap();
    let limits = p::ApproximateChartLimits {
        isotopy: p::IsotopyLimits {
            tolerance_uv: 0.01,
            max_deviation_cells: 40000,
            max_jet_cells: 256,
            max_pairs: 20000,
        },
        max_edges: 128,
        max_images: 256,
        max_cells: 30000,
        max_checks: 10000,
        tolerance_uv: 1e-8,
    };
    let regions = regularize(
        &[a, b],
        &[pa, pb],
        &[receipt],
        chart,
        [0.1, 0.6],
        limits,
        1e-7,
        1_000_000,
    )
    .unwrap();
    regions.reverify_source().unwrap();
    assert_eq!(regions.assembly().transfer.winding, Some(2));
    assert_eq!(regions.candidate_regions().components().len(), 2);
    let square = Curve::from_polyline(vec![
        vec![0., 0., 0.],
        vec![1., 0., 0.],
        vec![1., 1., 0.],
        vec![0., 1., 0.],
        vec![0., 0., 0.],
    ])
    .unwrap();
    let tube = nurbs_core::surface::extrude(&square, [0., 0., 2.]).unwrap();
    let bound = bind_periodic_regions_to_surface(
        &tube,
        &regions,
        [true, false],
        p::SurfaceCorrespondenceLimits {
            tolerance_uv: 0.01,
            tolerance_world: 1e-3,
            max_rectangles: 256,
            max_deviation_cells: 100000,
        },
    )
    .unwrap();
    let faces = brep_core::profile_certificate::assemble_periodic_surface_faces(
        &bound,
        brep_core::profile_certificate::PeriodicFaceLimits::default(),
    )
    .unwrap();
    faces.reverify().unwrap();
    assert_eq!(faces.model().faces.len(), 2);
    assert_eq!(faces.seam_edges().len(), 1);
    assert!(faces.agreement().complete);
    assert_eq!(faces.source().surface(), &tube);
    assert!(!faces.global_spatial_embedding_certified());
}

#[test]
fn nonlinear_multispan_originals_reach_owned_regions_and_actual_plane_face() {
    let a = Curve {
        degree: 1,
        knots: vec![0., 0., 0.5, 0.75, 1., 1.],
        control_points: vec![
            vec![0.5, 0.5],
            vec![0.75, 0.5],
            vec![0.5, 0.75],
            vec![0.5, 0.5],
        ],
        weights: vec![1.; 4],
        periodic: false,
    };
    let mut b = a.clone();
    b.knots = vec![0., 0., 0.25, 0.5, 1., 1.];
    b.weights = vec![1., 1.5, 2., 3.];
    let chart = [[0., 2.]; 2];
    let pa = p::split_candidates(&a, chart, &[0.5, 0.75], &[[0, 0]; 3], 3).unwrap();
    let pb = p::split_candidates(&b, chart, &[0.25, 0.5], &[[0, 0]; 3], 3).unwrap();
    let proof =
        p::verify_projective_partition_homotopy(&a, &b, &pa, &pb, [3., 1.], false, 1_000_000)
            .unwrap()
            .unwrap();
    let limits = p::ApproximateChartLimits {
        isotopy: p::IsotopyLimits {
            tolerance_uv: 0.01,
            max_deviation_cells: 40000,
            max_jet_cells: 256,
            max_pairs: 20000,
        },
        max_edges: 128,
        max_images: 256,
        max_cells: 30000,
        max_checks: 10000,
        tolerance_uv: 1e-8,
    };
    let regions = regularize(
        &[a, b],
        &[pa, pb],
        &[proof],
        chart,
        [0.55, 0.55],
        limits,
        1e-7,
        1_000_000,
    )
    .unwrap();
    regions.reverify_source().unwrap();
    assert_eq!(regions.assembly().transfer.winding, Some(2));
    assert_eq!(regions.candidate_regions().components().len(), 1);
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 2., 2.],
        knots_v: vec![0., 0., 2., 2.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 2., 0.]],
            vec![vec![2., 0., 0.], vec![2., 2., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let bound = bind_periodic_regions_to_surface(
        &surface,
        &regions,
        [false, false],
        p::SurfaceCorrespondenceLimits {
            tolerance_uv: 0.01,
            tolerance_world: 1e-3,
            max_rectangles: 256,
            max_deviation_cells: 100000,
        },
    )
    .unwrap();
    let faces = brep_core::profile_certificate::assemble_periodic_surface_faces(
        &bound,
        brep_core::profile_certificate::PeriodicFaceLimits::default(),
    )
    .unwrap();
    faces.reverify().unwrap();
    assert_eq!(faces.model().faces.len(), 1);
    assert_eq!(faces.model().edges.len(), 3);
    assert_eq!(faces.model().vertices.len(), 3);
    assert!(faces.agreement().complete);
    assert_eq!(faces.source().surface(), &surface);
}
