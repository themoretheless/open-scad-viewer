use super::*;
use crate::source_seam_tangency::{self as seams, Class};
use nurbs_core::curve::Curve;
fn quadratic(points: [[f64; 2]; 3]) -> Curve {
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: points.into_iter().map(|p| p.to_vec()).collect(),
        weights: vec![1.; 3],
        periodic: false,
    }
}
#[test]
fn exact_tangent_planes_do_not_authorize_a_material_cusp() {
    let curves = vec![
        quadratic([[-1., -1.], [-0.5, 0.], [0., 0.]]),
        quadratic([[0., 0.], [-0.5, 0.], [-1., 1.]]),
        Curve::from_polyline(vec![vec![-1., 1.], vec![-1., -1.]]).unwrap(),
    ];
    let model = crate::prism::extrude(&[curves], 0., 3.).unwrap();
    let r = prepare(&model, limits()).unwrap();
    let shell = r.shell.expect(r.reason);
    let index = shell
        .edges()
        .iter()
        .position(|e| {
            e.world()
                .control_points
                .iter()
                .all(|p| p[0] == 0. && p[1] == 0.)
        })
        .unwrap();
    let edge = &shell.edges()[index];
    let before = edge.definition();
    let planes = seams::qualify_tangency_exact(
        edge,
        seams::Limits {
            max_sine_squared: 0.,
            cells: 10000,
            curve_spans: 10000,
            normal_spans: 10000,
        },
        1,
        100000000,
    )
    .unwrap();
    assert!(planes.seam.is_some(), "{}", planes.reason);
    let pair = shell.uses()[index];
    let regions = shell.regions().unwrap();
    let sign = regions[pair[0].face].chart_winding() * regions[pair[1].face].chart_winding();
    // This checks the native signed-anchor premise after exact full-plane
    // admission. It does NOT claim this cusp has a globally admitted Body.
    let r = seams::material_orientation::inspect_anchor(
        edge,
        sign,
        seams::material_orientation::Limits {
            steps: 0,
            curve_spans: 100000,
            normal_spans: 100000,
        },
    )
    .unwrap();
    assert!(!r.positive);
    assert_eq!(r.reason, "source-material-smooth-seam-opposed-normals");
    assert_eq!(edge.definition(), before);
}
#[test]
fn closed_curved_prism_has_exact_g1_with_an_actual_curvature_jump() {
    let curves = vec![
        quadratic([[-1., 1.], [-0.5, 0.], [0., 0.]]),
        quadratic([[0., 0.], [0.5, 0.], [1., 2.]]),
        Curve::from_polyline(vec![vec![1., 2.], vec![1., 4.]]).unwrap(),
        Curve::from_polyline(vec![vec![1., 4.], vec![-1., 4.]]).unwrap(),
        Curve::from_polyline(vec![vec![-1., 4.], vec![-1., 1.]]).unwrap(),
    ];
    let model = crate::prism::extrude(&[curves], 0., 3.).unwrap();
    let r = prepare(&model, limits()).unwrap();
    let shell = r.shell.expect(r.reason);
    let r = crate::source_shell_geometry::qualify(shell, geometry_limits()).unwrap();
    eprintln!(
        "curved G1 prism geometry: {} pair={:?}",
        r.reason, r.next_pair
    );
    let geometry = r.geometry.expect(r.reason);
    let r = crate::source_volume::qualify(
        geometry,
        crate::source_volume::Limits {
            axis: 2,
            origin: 0.,
            absolute_error: 0.02,
            tolerance_uv: 1e-8,
            cells: 100000,
            spans: 100000,
            domain_cells: 1000000,
        },
    )
    .unwrap();
    let body = r.body.expect(r.reason);
    assert!(body.volume()[0] <= 21. && body.volume()[1] >= 21.);
    let before = body.definition().unwrap();
    let edges = body.geometry().shell().edges();
    let index = edges
        .iter()
        .position(|e| {
            e.world()
                .control_points
                .iter()
                .all(|p| p[0] == 0. && p[1] == 0.)
        })
        .expect("C1 bottom-profile join");
    let mut classes = vec![Class::C0; edges.len()];
    classes[index] = Class::TangentPlanes;
    let r = seams::qualify_collection(
        &body,
        &classes,
        seams::Limits {
            max_sine_squared: 1e-8,
            cells: 10000,
            curve_spans: 10000,
            normal_spans: 10000,
        },
    )
    .unwrap();
    let collection = r.certificate.expect(r.reason);
    let axes = vec![1; edges.len()];
    let limits = || seams::ExactSmoothLimits {
        incidence_steps: 1000000,
        exact_work: 100000000,
        cells: 100000,
        curve_spans: 100000,
        source_spans: 100000,
    };
    let r = seams::qualify_exact_smooth_collection(&collection, &axes, limits()).unwrap();
    let proof = r.certificate.expect(r.reason);
    let orient = seams::material_orientation::qualify(
        &proof,
        seams::material_orientation::Limits {
            steps: 100000,
            curve_spans: 100000,
            normal_spans: 100000,
        },
    )
    .unwrap();
    let oriented = orient.certificate.expect(orient.reason);
    assert!(std::ptr::eq(oriented.exact(), &proof));
    assert_eq!(oriented.edges(), &[index]);
    let tight_orientation = seams::material_orientation::Limits {
        steps: orient.steps,
        curve_spans: orient.curve_spans,
        normal_spans: orient.normal_spans,
    };
    assert!(
        seams::material_orientation::qualify(&proof, tight_orientation)
            .unwrap()
            .certificate
            .is_some()
    );
    for dim in 0..3 {
        let mut l = tight_orientation;
        match dim {
            0 => l.steps -= 1,
            1 => l.curve_spans -= 1,
            _ => l.normal_spans -= 1,
        }
        assert!(
            seams::material_orientation::qualify(&proof, l)
                .unwrap()
                .certificate
                .is_none()
        );
    }
    assert!(std::ptr::eq(proof.original(), &collection));
    assert_eq!(proof.tangent_seams().len(), 1);
    assert_eq!(proof.tangent_seams()[0].0, index);
    assert!(proof.curvature_seams().is_empty() && proof.polar_tangent_edges().is_empty());
    assert!(!proof.all_edges_exact_tangent() && !proof.all_edges_exact_curvature());
    assert_eq!(proof.original().classes(), classes);
    let tight = || seams::ExactSmoothLimits {
        incidence_steps: r.steps,
        exact_work: r.exact_work,
        cells: r.cells,
        curve_spans: r.curve_spans,
        source_spans: r.source_spans,
    };
    assert!(
        seams::qualify_exact_smooth_collection(&collection, &axes, tight())
            .unwrap()
            .certificate
            .is_some()
    );
    for dim in 0..5 {
        let mut l = tight();
        match dim {
            0 => l.incidence_steps -= 1,
            1 => l.exact_work -= 1,
            2 => l.cells -= 1,
            3 => l.curve_spans -= 1,
            _ => l.source_spans -= 1,
        }
        if let Ok(short) = seams::qualify_exact_smooth_collection(&collection, &axes, l) {
            assert!(short.certificate.is_none());
        }
    }
    let second = seams::qualify_curvature_exact(
        &edges[index],
        seams::CurvatureLimits {
            normal_distance: 0.,
            curvature_distance: 0.,
            cells: 1000,
            curve_spans: 4000,
            shape_cells: 4000,
        },
        1,
        100000000,
    )
    .unwrap();
    assert!(second.seam.is_none());
    assert_eq!(
        second.reason,
        "exact-original-meridian-graph-jets-different"
    );
    assert!(
        seams::qualify_exact_smooth_collection(&collection, &axes[..axes.len() - 1], limits())
            .is_err()
    );
    let mut invalid = axes.clone();
    *invalid.last_mut().unwrap() = 3;
    assert!(seams::qualify_exact_smooth_collection(&collection, &invalid, limits()).is_err());
    assert_eq!(body.definition().unwrap(), before);
    eprintln!(
        "owned exact G1 actual H jump: steps={} work={} cells={} source_spans={} volume={:?}",
        r.steps,
        r.exact_work,
        r.cells,
        r.source_spans,
        body.volume()
    );
}
