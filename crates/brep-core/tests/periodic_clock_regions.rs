use brep_core::profile_certificate::regularize_clock_mapped_periodic_winding;
use nurbs_core::{
    curve::Curve,
    periodic_trim::{self as p, Piece, Report},
};
#[test]
fn actual_nonlinear_clock_aliases_reach_owned_original_nonzero_regions() {
    let a = Curve {
        degree: 3,
        knots: [vec![0.; 4], vec![1.; 4]].concat(),
        control_points: vec![vec![0., 0.], vec![5., 0.], vec![0., 5.], vec![0., 0.]],
        weights: vec![1.; 4],
        periodic: false,
    };
    let b = Curve {
        degree: 6,
        knots: [vec![0.; 7], vec![1.; 7]].concat(),
        control_points: vec![
            vec![0., 0.],
            vec![0., 0.],
            vec![1., 0.],
            vec![3., 0.],
            vec![4., 1.],
            vec![0., 5.],
            vec![0., 0.],
        ],
        weights: vec![1.; 7],
        periodic: false,
    };
    let clock = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![0., 0.], vec![1., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let parts = |c: &Curve, u| Report {
        pieces: c
            .split(u)
            .unwrap()
            .into_iter()
            .map(|curve| Piece {
                source_domain: curve.domain(),
                curve,
                translation: [0.; 2],
                tile: [0; 2],
            })
            .collect(),
        candidate_hulls_contained: false,
    };
    let partitions = vec![parts(&a, 0.25), parts(&b, 0.5)];
    let sources = vec![a, b];
    let maps = clock.split(0.5).unwrap();
    let specs = vec![
        p::ClockAliasSpec {
            edges: [0, 2],
            clock: maps[0].clone(),
        },
        p::ClockAliasSpec {
            edges: [1, 3],
            clock: maps[1].clone(),
        },
    ];
    let limits = p::ChartAssemblyLimits {
        max_segments: 64,
        max_links: 32,
        max_edges: 32,
        max_images: 256,
        max_cells: 30000,
        tolerance_uv: 1e-8,
    };
    let owned = regularize_clock_mapped_periodic_winding(
        &sources,
        &partitions,
        &[[0, 0]; 2],
        [[0., 8.]; 2],
        [1., 1.],
        limits,
        &specs,
        1e-7,
        1_000_000,
    )
    .unwrap();
    owned.reverify_source().unwrap();
    assert_eq!(owned.regions().components().len(), 1);
    assert_eq!(owned.regions().coefficients(), [2]);
    assert_eq!(owned.source().graph().unwrap().edges.len(), 2);
    let surface = nurbs_core::surface::Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 8., 8.],
        knots_v: vec![0., 0., 8., 8.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 8., 0.]],
            vec![vec![8., 0., 0.], vec![8., 8., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let surface_limits = p::SurfaceCorrespondenceLimits {
        tolerance_uv: 1e-6,
        tolerance_world: 1e-4,
        max_rectangles: 256,
        max_deviation_cells: 100000,
    };
    let bound = brep_core::profile_certificate::bind_clock_mapped_regions_to_surface(
        &surface,
        &owned,
        [false; 2],
        surface_limits,
    )
    .unwrap();
    bound.reverify_source().unwrap();
    assert!(bound.correspondence().unwrap().within_world_tolerance);
    assert_eq!(bound.surface(), &surface);
    assert_eq!(bound.regions().regions().coefficients(), [2]);
    // The same planar patch has no U seam, so requesting one must refuse.
    assert!(
        brep_core::profile_certificate::bind_clock_mapped_regions_to_surface(
            &surface,
            &owned,
            [true, false],
            surface_limits
        )
        .is_err()
    );
    let mut wrong_domain = surface.clone();
    wrong_domain.knots_u = vec![0., 0., 7., 7.];
    assert!(
        brep_core::profile_certificate::bind_clock_mapped_regions_to_surface(
            &wrong_domain,
            &owned,
            [false; 2],
            surface_limits
        )
        .is_err()
    );
    let face_limits = brep_core::profile_certificate::PeriodicFaceLimits::default();
    let faces =
        brep_core::profile_certificate::assemble_clock_mapped_surface_faces(&bound, face_limits)
            .unwrap();
    faces.reverify().unwrap();
    faces.verify_model(faces.model()).unwrap();
    assert_eq!(faces.model().faces.len(), 1);
    assert_eq!(faces.model().edges.len(), 2);
    assert_eq!(faces.model().vertices.len(), 2);
    assert_eq!(faces.model().shells.len(), 1);
    assert!(!faces.model().shells[0].closed);
    assert!(faces.agreement().complete);
    assert!(faces.injectivity().iter().all(|p| p.proven));
    assert!(faces.boundary_error_upper() <= 1e-4);
    assert!(faces.seam_edges().is_empty());
    assert!(!faces.global_spatial_embedding_certified());
    // A bilinear saddle keeps the exact XY chart projection while its world
    // boundary is genuinely curved. Whole lift/agreement proofs are mandatory.
    let mut warped = surface.clone();
    warped.control_points[1][1][2] = 2.;
    let curved_support = brep_core::profile_certificate::bind_clock_mapped_regions_to_surface(
        &warped,
        &owned,
        [false; 2],
        surface_limits,
    )
    .unwrap();
    let curved_faces = brep_core::profile_certificate::assemble_clock_mapped_surface_faces(
        &curved_support,
        face_limits,
    )
    .unwrap();
    curved_faces.reverify().unwrap();
    assert!(curved_faces.agreement().complete);
    assert!(
        curved_faces
            .model()
            .edges
            .iter()
            .any(|e| e.curve.control_points.iter().any(|p| p[2] > 0.))
    );
    let mut saved = faces.model().clone();
    saved.vertices[0].point[0] = saved.vertices[0].point[0].next_up();
    assert!(faces.verify_model(&saved).is_err());
    let mut exhausted = face_limits;
    exhausted.agreement_cells = 1;
    assert!(
        brep_core::profile_certificate::assemble_clock_mapped_surface_faces(&bound, exhausted)
            .is_err()
    );
    let mut singular = surface.clone();
    singular.control_points[1] = singular.control_points[0].clone();
    let singular_bound = brep_core::profile_certificate::bind_clock_mapped_regions_to_surface(
        &singular,
        &owned,
        [false; 2],
        surface_limits,
    )
    .unwrap();
    assert!(
        brep_core::profile_certificate::assemble_clock_mapped_surface_faces(
            &singular_bound,
            face_limits
        )
        .is_err()
    );
    let mut damaged = partitions;
    damaged[1].pieces[0].curve.control_points[2][0] =
        damaged[1].pieces[0].curve.control_points[2][0].next_up();
    assert!(
        regularize_clock_mapped_periodic_winding(
            &sources,
            &damaged,
            &[[0, 0]; 2],
            [[0., 8.]; 2],
            [1., 1.],
            limits,
            &specs,
            1e-7,
            1_000_000
        )
        .is_err()
    );
}

#[test]
fn clock_proof_kind_sews_only_original_opposite_seam_connectors() {
    use brep_core::profile_certificate as pc;
    let curve = Curve::from_polyline(vec![
        vec![3.5, 0.25],
        vec![4.5, 0.25],
        vec![4.5, 0.75],
        vec![3.5, 0.75],
        vec![3.5, 0.25],
    ])
    .unwrap();
    let chart = [[0., 4.], [0., 1.]];
    let partition =
        p::split_candidates(&curve, chart, &[0.5, 2.5], &[[0, 0], [1, 0], [0, 0]], 16).unwrap();
    let regions = pc::regularize_clock_mapped_periodic_winding(
        &[curve.clone(), curve],
        &[partition.clone(), partition],
        &[[0; 2]; 2],
        chart,
        [3.75, 0.5],
        p::ChartAssemblyLimits {
            max_segments: 64,
            max_links: 32,
            max_edges: 64,
            max_images: 256,
            max_cells: 30000,
            tolerance_uv: 1e-8,
        },
        &[],
        1e-7,
        1_000_000,
    )
    .unwrap();
    assert_eq!(regions.regions().coefficients(), [2, 2]);
    let profile = Curve::from_polyline(vec![
        vec![0., 0., 0.],
        vec![1., 0., 0.],
        vec![1., 1., 0.],
        vec![0., 1., 0.],
        vec![0., 0., 0.],
    ])
    .unwrap();
    let surface = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let support = pc::bind_clock_mapped_regions_to_surface(
        &surface,
        &regions,
        [true, false],
        p::SurfaceCorrespondenceLimits {
            tolerance_uv: 1e-6,
            tolerance_world: 1e-4,
            max_rectangles: 256,
            max_deviation_cells: 100000,
        },
    )
    .unwrap();
    let faces =
        pc::assemble_clock_mapped_surface_faces(&support, pc::PeriodicFaceLimits::default())
            .unwrap();
    faces.reverify().unwrap();
    assert_eq!(faces.model().faces.len(), 2);
    assert_eq!(faces.seam_edges().len(), 1);
    assert_eq!(faces.model().shells.len(), 1);
    assert!(!faces.model().shells[0].closed);
    let seam = faces.seam_edges()[0];
    let uses = faces
        .model()
        .loops
        .iter()
        .flat_map(|w| &w.coedges)
        .filter(|c| c.edge == seam)
        .collect::<Vec<_>>();
    assert_eq!(uses.len(), 2);
    assert_ne!(uses[0].reversed, uses[1].reversed);
    assert!(faces.agreement().complete);
    assert!(!faces.global_spatial_embedding_certified());
}
