use brep_core::profile_certificate as proof;
use nurbs_core::{
    curve::Curve,
    periodic_trim::{self as p, Piece, Report},
};
fn source() -> Curve {
    Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![5., 0.], vec![0., 5.], vec![0., 0.]],
        weights: vec![1.; 4],
        periodic: false,
    }
}
fn mapped() -> Curve {
    Curve {
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
    }
}
fn parts(c: &Curve, cut: f64) -> Report {
    Report {
        pieces: c
            .split(cut)
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
    }
}
fn limits() -> p::ChartAssemblyLimits {
    p::ChartAssemblyLimits {
        max_segments: 64,
        max_links: 32,
        max_edges: 32,
        max_images: 256,
        max_cells: 30000,
        tolerance_uv: 1e-8,
    }
}

#[test]
fn original_multispan_images_cover_irrational_other_curve_knot_preimages() {
    let a = source();
    let b = mapped();
    let mut partitions = vec![parts(&a, 0.25), parts(&b, 0.5)];
    // Independent exact binary insertions: native extraction roundoff cannot
    // grant authority; the original restriction verifier checks every basis.
    partitions[0].pieces[1].curve = partitions[0].pieces[1].curve.insert(0.625, 1).unwrap();
    partitions[1].pieces[0].curve = partitions[1].pieces[0].curve.insert(0.25, 1).unwrap();
    let a = a.insert(0.625, 1).unwrap();
    let b = b.insert(0.25, 1).unwrap();
    assert!(partitions[0].pieces[1].curve.control_points.len() > 4);
    assert!(partitions[1].pieces[0].curve.control_points.len() > 7);
    let sources = vec![a, b];
    let owner = proof::regularize_multispan_periodic_winding(
        &sources,
        &partitions,
        &[[0; 2]; 2],
        [[0., 8.]; 2],
        [1., 1.],
        limits(),
        &[[0, 2], [1, 3]],
        1e-7,
        1_000_000,
    )
    .unwrap();
    owner.reverify_source().unwrap();
    assert_eq!(owner.graph().edges.len(), 2);
    assert!(
        owner
            .graph()
            .edges
            .iter()
            .all(|e| e.coefficient == 2 && e.contributions.len() == 2)
    );
    assert_eq!(owner.regions().components().len(), 1);
    assert_eq!(owner.regions().coefficients(), [2]);
    assert_eq!(owner.aliases().len(), 2);
    assert!(
        owner
            .aliases()
            .iter()
            .all(|a| a.certificate.version == 1 && !a.certificate.steps.is_empty())
    );
    // The second source's exact t² image has no literal cut at sqrt(5/8).
    assert!(!sources[1].knots.contains(&0.625_f64.sqrt()));
    let surface = nurbs_core::surface::Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 8., 8.],
        knots_v: vec![0., 0., 8., 8.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 8., 0.]],
            vec![vec![8., 0., 0.], vec![8., 8., 2.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let support = proof::bind_multispan_regions_to_surface(
        &surface,
        &owner,
        [false; 2],
        p::SurfaceCorrespondenceLimits {
            tolerance_uv: 1e-6,
            tolerance_world: 1e-4,
            max_rectangles: 256,
            max_deviation_cells: 100000,
        },
    )
    .unwrap();
    let faces = proof::assemble_multispan_periodic_surface_faces(
        &support,
        proof::PeriodicFaceLimits::default(),
    )
    .unwrap();
    faces.reverify().unwrap();
    faces.verify_model(faces.model()).unwrap();
    assert_eq!(faces.model().faces.len(), 1);
    assert_eq!(faces.model().edges.len(), 2);
    assert!(faces.agreement().complete);
    assert!(faces.injectivity().iter().all(|r| r.proven));
    assert!(
        faces
            .model()
            .edges
            .iter()
            .any(|e| e.curve.control_points.iter().any(|p| p[2] > 0.))
    );
    assert!(!faces.global_spatial_embedding_certified());
    let mut saved = faces.model().clone();
    saved.vertices[0].point[0] = saved.vertices[0].point[0].next_up();
    assert!(faces.verify_model(&saved).is_err());
    let mut damaged = partitions;
    damaged[1].pieces[0].curve.control_points[2][0] =
        damaged[1].pieces[0].curve.control_points[2][0].next_up();
    assert!(
        proof::regularize_multispan_periodic_winding(
            &sources,
            &damaged,
            &[[0; 2]; 2],
            [[0., 8.]; 2],
            [1., 1.],
            limits(),
            &[[0, 2], [1, 3]],
            1e-7,
            1_000_000
        )
        .is_err()
    );
}

#[test]
fn reverse_multispan_traversals_cancel_with_original_source_roles() {
    let a = source();
    let mut b = mapped();
    b.control_points.reverse();
    b.weights.reverse();
    b.knots = b.knots.iter().rev().map(|u| 1. - u).collect();
    let mut partitions = vec![parts(&a, 0.25), parts(&b, 0.5)];
    partitions[0].pieces[1].curve = partitions[0].pieces[1].curve.insert(0.625, 1).unwrap();
    partitions[1].pieces[1].curve = partitions[1].pieces[1].curve.insert(0.75, 1).unwrap();
    let sources = vec![a.insert(0.625, 1).unwrap(), b.insert(0.75, 1).unwrap()];
    let owner = proof::regularize_multispan_periodic_winding(
        &sources,
        &partitions,
        &[[0; 2]; 2],
        [[0., 8.]; 2],
        [1., 1.],
        limits(),
        &[[0, 3], [1, 2]],
        1e-7,
        1_000_000,
    )
    .unwrap();
    owner.reverify_source().unwrap();
    assert!(owner.graph().edges.is_empty());
    assert_eq!(owner.graph().cancellations.len(), 2);
    assert!(
        owner
            .graph()
            .cancellations
            .iter()
            .all(|p| p.len() == 2 && p[0].orientation == -p[1].orientation)
    );
    assert!(owner.regions().components().is_empty());
    assert!(owner.aliases().iter().all(|p| p.orientation == -1));
}
