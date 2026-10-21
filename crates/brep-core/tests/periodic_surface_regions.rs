use brep_core::profile_certificate::{
    CoherentPeriodicWindingRegions, bind_periodic_regions_to_surface,
    regularize_coherent_periodic_winding,
};
use nurbs_core::{
    curve::Curve,
    periodic_trim::{self as p, SurfaceCorrespondenceLimits},
    surface::{self, Surface},
};
fn tube() -> Surface {
    let c = Curve::from_polyline(vec![
        vec![0., 0., 0.],
        vec![1., 0., 0.],
        vec![1., 1., 0.],
        vec![0., 1., 0.],
        vec![0., 0., 0.],
    ])
    .unwrap();
    surface::extrude(&c, [0., 0., 2.]).unwrap()
}
fn regions() -> CoherentPeriodicWindingRegions {
    let c = Curve::from_polyline(vec![
        vec![3.5, 0.25],
        vec![4.5, 0.25],
        vec![4.5, 0.75],
        vec![3.5, 0.75],
        vec![3.5, 0.25],
    ])
    .unwrap();
    let split =
        p::split_approximated(&c, [[0., 4.], [0., 1.]], 1e-8, 1e-4, 16, 32, 10000, 100000).unwrap();
    let part = split
        .partition
        .clone()
        .unwrap_or_else(|| panic!("split status {:?}", split.status));
    let l = p::ApproximateChartLimits {
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
    regularize_coherent_periodic_winding(
        &[c.clone(), c],
        &[part.clone(), part],
        [[0., 4.], [0., 1.]],
        [3.75, 0.5],
        l,
        1e-7,
    )
    .unwrap()
}
fn limits() -> SurfaceCorrespondenceLimits {
    SurfaceCorrespondenceLimits {
        tolerance_uv: 0.01,
        tolerance_world: 1e-3,
        max_rectangles: 256,
        max_deviation_cells: 100000,
    }
}
fn map([u, v]: [f64; 2]) -> [f64; 3] {
    let u = u.rem_euclid(4.);
    let (x, y) = if u <= 1. {
        (u, 0.)
    } else if u <= 2. {
        (1., u - 1.)
    } else if u <= 3. {
        (3. - u, 1.)
    } else {
        (0., 4. - u)
    };
    [x, y, 2. * v]
}
#[test]
fn actual_closed_support_and_world_error_bind_original_coherent_regions() {
    let mut s = tube();
    let r = regions();
    let bound = bind_periodic_regions_to_surface(&s, &r, [true, false], limits()).unwrap();
    bound.reverify_source().unwrap();
    assert_eq!(bound.regions().candidate_regions().components().len(), 2);
    assert_eq!(
        bound.correspondence().seams[0],
        Some(p::SurfaceSeamProof::ClampedBoundaryIdentity)
    );
    assert!(bound.correspondence().error_upper <= limits().tolerance_world);
    for (source, partition) in r.source().iter().zip(r.partitions()) {
        for piece in &partition.pieces {
            for alpha in [0., 0.25, 0.5, 0.75, 1.] {
                let t = piece.source_domain[0] * (1. - alpha) + piece.source_domain[1] * alpha;
                let uv = source.evaluate_de_boor(t).unwrap();
                let a = map([uv[0], uv[1]]);
                let d = piece.curve.domain();
                let uv = piece
                    .curve
                    .evaluate_de_boor(d[0] * (1. - alpha) + d[1] * alpha)
                    .unwrap();
                let b = map([uv[0] + piece.translation[0], uv[1] + piece.translation[1]]);
                let error = a
                    .iter()
                    .zip(b)
                    .map(|(x, y)| (x - y) * (x - y))
                    .sum::<f64>()
                    .sqrt();
                assert!(error <= bound.correspondence().error_upper + 1e-12);
            }
        }
    }
    s.control_points[0][0][0] = 100.;
    assert_ne!(bound.surface(), &s);
    bound.reverify_source().unwrap();
}
#[test]
fn flags_wrong_axes_and_missing_world_work_never_replace_actual_support() {
    let r = regions();
    let s = tube();
    assert!(bind_periodic_regions_to_surface(&s, &r, [false, false], limits()).is_err());
    let mut broken = s.clone();
    let n = broken.control_points.len();
    broken.control_points[n - 1][0][0] += 0.125;
    assert!(bind_periodic_regions_to_surface(&broken, &r, [true, false], limits()).is_err());
    broken.periodic_u = true;
    assert!(bind_periodic_regions_to_surface(&broken, &r, [true, false], limits()).is_err());
    let mut low = limits();
    low.max_rectangles = 0;
    assert!(bind_periodic_regions_to_surface(&s, &r, [true, false], low).is_err());
}

#[test]
fn actual_surface_weight_gauge_is_admitted_without_changing_saved_surface() {
    let r = regions();
    let mut s = tube();
    let n = s.weights.len();
    for w in &mut s.weights[n - 1] {
        *w *= 1.125;
    }
    let original = s.clone();
    let bound = bind_periodic_regions_to_surface(&s, &r, [true, false], limits()).unwrap();
    bound.reverify_source().unwrap();
    assert_eq!(bound.surface(), &original);
    s.weights[n - 1][1] = s.weights[n - 1][1].next_up();
    assert!(bind_periodic_regions_to_surface(&s, &r, [true, false], limits()).is_err());
}

#[test]
fn retained_original_surface_regions_become_owned_faces_with_one_shared_seam() {
    use brep_core::profile_certificate::{PeriodicFaceLimits, assemble_periodic_surface_faces};
    let r = regions();
    let s = tube();
    let bound = bind_periodic_regions_to_surface(&s, &r, [true, false], limits()).unwrap();
    let owner = assemble_periodic_surface_faces(&bound, PeriodicFaceLimits::default()).unwrap();
    owner.reverify().unwrap();
    owner.verify_model(owner.model()).unwrap();
    let m = owner.model();
    assert_eq!(m.faces.len(), 2);
    // Three original partition curves retain their interior polyline knots;
    // the fourth edge is the one sewn artificial connector.
    assert_eq!(m.edges.len(), 4);
    assert_eq!(m.vertices.len(), 3);
    assert_eq!(m.shells.len(), 1);
    assert!(!m.shells[0].closed);
    assert!(m.bodies.is_empty());
    assert_eq!(owner.seam_edges().len(), 1);
    for expected in [[0., 0.5, 0.5], [0., 0., 0.5], [0., 0., 1.5]] {
        assert!(m.vertices.iter().any(|v| {
            v.point
                .iter()
                .zip(expected)
                .all(|(a, b)| (a - b).abs() < 1e-7)
        }));
    }
    assert!(owner.agreement().complete);
    assert_eq!(owner.injectivity().len(), 2);
    assert!(owner.injectivity().iter().all(|q| q.proven));
    assert!(!owner.global_spatial_embedding_certified());
    assert!(owner.boundary_error_upper() >= bound.correspondence().error_upper);
    for f in &m.faces {
        assert_eq!(f.surface, s);
    }
    let seam = owner.seam_edges()[0];
    let uses = m
        .loops
        .iter()
        .flat_map(|l| &l.coedges)
        .filter(|c| c.edge == seam)
        .collect::<Vec<_>>();
    assert_eq!(uses.len(), 2);
    assert_ne!(uses[0].reversed, uses[1].reversed);
    for alpha in [0., 0.25, 0.5, 0.75, 1.] {
        let c = &m.edges[seam].curve;
        let d = c.domain();
        let q = c.evaluate_de_boor(d[0] + (d[1] - d[0]) * alpha).unwrap();
        assert!(q[0].abs() < 1e-6 && q[1].abs() < 1e-6);
        assert!((0.5 - 1e-6..=1.5 + 1e-6).contains(&q[2]));
    }
    let mut changed = m.clone();
    changed.edges[0].curve.control_points[1][0] += 0.001;
    assert!(owner.verify_model(&changed).is_err());
    let mut changed = m.clone();
    changed.loops[0].coedges[0].pcurve.control_points[0][0] += 1e-6;
    assert!(owner.verify_model(&changed).is_err());
    eprintln!(
        "owned periodic faces: 2 original supports, 4 edges, 3 vertices, 1 sewn seam, full edge/support agreement"
    );
}
#[test]
fn supporting_correspondence_alone_cannot_admit_a_collapsed_face_or_partial_work() {
    use brep_core::profile_certificate::{PeriodicFaceLimits, assemble_periodic_surface_faces};
    let r = regions();
    let mut s = tube();
    for p in s.control_points.iter_mut().flatten() {
        p[2] = 0.;
    }
    let bound = bind_periodic_regions_to_surface(&s, &r, [true, false], limits()).unwrap();
    assert!(assemble_periodic_surface_faces(&bound, PeriodicFaceLimits::default()).is_err());
    let bound = bind_periodic_regions_to_surface(&tube(), &r, [true, false], limits()).unwrap();
    for l in [
        PeriodicFaceLimits {
            lift_cells: 1,
            ..Default::default()
        },
        PeriodicFaceLimits {
            agreement_cells: 1,
            ..Default::default()
        },
        PeriodicFaceLimits {
            injectivity_spans: 1,
            ..Default::default()
        },
        PeriodicFaceLimits {
            tolerance_world: f64::NAN,
            ..Default::default()
        },
        PeriodicFaceLimits {
            injectivity_depth: 7,
            ..Default::default()
        },
    ] {
        assert!(assemble_periodic_surface_faces(&bound, l).is_err());
    }
}

#[test]
fn rational_closed_support_produces_curved_owned_world_edges_and_a_shared_seam() {
    use brep_core::profile_certificate::{PeriodicFaceLimits, assemble_periodic_surface_faces};
    let w = std::f64::consts::FRAC_1_SQRT_2;
    let points = vec![
        vec![1., 0., 0.],
        vec![1., 1., 0.],
        vec![0., 1., 0.],
        vec![-1., 1., 0.],
        vec![-1., 0., 0.],
        vec![-1., -1., 0.],
        vec![0., -1., 0.],
        vec![1., -1., 0.],
        vec![1., 0., 0.],
    ];
    let circle = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.],
        control_points: points,
        weights: vec![1., w, 1., w, 1., w, 1., w, 1.],
        periodic: false,
    };
    let s = surface::extrude(&circle, [0., 0., 2.]).unwrap();
    let r = regions();
    let db = nurbs_core::surface_parameter_bounds::inspect(&s, 256).unwrap();
    let diagnostic = p::verify_surface_correspondence(
        &s,
        r.source(),
        r.partitions(),
        [[0., 4.], [0., 1.]],
        [true, false],
        limits(),
    )
    .unwrap();
    eprintln!(
        "rational source derivative bounds {:?}, admission {:?}",
        db.derivative_norm_upper_bounds,
        diagnostic
            .admissions
            .iter()
            .map(|a| (&a.status, a.error_upper, a.work))
            .collect::<Vec<_>>()
    );
    let bound = bind_periodic_regions_to_surface(&s, &r, [true, false], limits()).unwrap();
    let owner = assemble_periodic_surface_faces(&bound, PeriodicFaceLimits::default()).unwrap();
    owner.reverify().unwrap();
    assert_eq!(owner.model().faces.len(), 2);
    assert_eq!(owner.seam_edges().len(), 1);
    for e in &owner.model().edges {
        let d = e.curve.domain();
        for i in 0..=32 {
            let q = e
                .curve
                .evaluate_de_boor(d[0] + (d[1] - d[0]) * i as f64 / 32.)
                .unwrap();
            assert!((q[0] * q[0] + q[1] * q[1] - 1.).abs() < 1e-6);
        }
    }
    assert!(owner.agreement().complete);
}

#[test]
fn two_wrapped_axes_own_one_four_face_corner_fan_on_the_original_torus() {
    use brep_core::profile_certificate::{PeriodicFaceLimits, assemble_periodic_surface_faces};
    let s = nurbs_core::primitives::torus([0.; 3], 3., 1., 1.).unwrap();
    let chart = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    assert_eq!(chart, [[0., 1.], [0., 4.]]);
    let c = Curve::from_polyline(vec![
        vec![-0.05, -0.05],
        vec![0.05, -0.05],
        vec![0.05, 0.05],
        vec![-0.05, 0.05],
        vec![-0.05, -0.05],
    ])
    .unwrap();
    let split = p::split_approximated(&c, chart, 1e-8, 1e-5, 32, 64, 10000, 100000).unwrap();
    let part = split.partition.unwrap();
    let l = p::ApproximateChartLimits {
        isotopy: p::IsotopyLimits {
            tolerance_uv: 0.001,
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
    let r = regularize_coherent_periodic_winding(
        &[c.clone(), c],
        &[part.clone(), part],
        chart,
        [0.02, 0.02],
        l,
        1e-7,
    )
    .unwrap();
    let bound = bind_periodic_regions_to_surface(&s, &r, [true, true], limits()).unwrap();
    assert!(
        assemble_periodic_surface_faces(
            &bound,
            PeriodicFaceLimits {
                injectivity_depth: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
    let owner = assemble_periodic_surface_faces(&bound, PeriodicFaceLimits::default()).unwrap();
    owner.reverify().unwrap();
    let m = owner.model();
    assert_eq!(m.faces.len(), 4);
    assert_eq!(owner.seam_edges().len(), 4);
    assert_eq!(m.edges.len(), 9);
    assert_eq!(m.vertices.len(), 6);
    assert_eq!(m.shells.len(), 1);
    assert!(!m.shells[0].closed);
    assert!(m.bodies.is_empty());
    assert!(owner.agreement().complete);
    let center = m
        .vertices
        .iter()
        .position(|v| {
            (v.point[0] - 4.).abs() < 1e-7 && v.point[1].abs() < 1e-7 && v.point[2].abs() < 1e-7
        })
        .unwrap();
    assert_eq!(
        m.edges
            .iter()
            .filter(|e| e.vertices.contains(&center))
            .count(),
        4
    );
    for e in &m.edges {
        let d = e.curve.domain();
        for i in 0..=16 {
            let q = e
                .curve
                .evaluate_de_boor(d[0] + (d[1] - d[0]) * i as f64 / 16.)
                .unwrap();
            assert!(((q[0].hypot(q[1]) - 3.).powi(2) + q[2] * q[2] - 1.).abs() < 1e-6);
        }
    }
    eprintln!(
        "two-axis owned torus: 4 faces, 9 edges, 6 vertices, 4 sewn seam spokes, cyclic corner fan"
    );
}
