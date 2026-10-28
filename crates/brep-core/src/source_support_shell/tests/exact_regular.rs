use super::*;
use crate::source_seam_tangency::{self as seams, Class, ExactCollectionLimits};
#[test]
fn closed_split_prism_owns_every_exact_regular_g2_carrier_and_aggregate_budget() {
    let points = [
        [0., 0.],
        [5., 0.],
        [10., 0.],
        [10., 10.],
        [10., 20.],
        [5., 20.],
        [0., 20.],
    ];
    let curves = (0..points.len())
        .map(|i| {
            nurbs_core::curve::Curve::from_polyline(vec![
                points[i].to_vec(),
                points[(i + 1) % points.len()].to_vec(),
            ])
            .unwrap()
        })
        .collect();
    let model = crate::prism::extrude(&[curves], 0., 30.).unwrap();
    let r = prepare(&model, limits()).unwrap();
    let shell = r.shell.expect(r.reason);
    let r = crate::source_shell_geometry::qualify(shell, geometry_limits()).unwrap();
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
    assert!(body.volume()[0] <= 6000. && body.volume()[1] >= 6000.);
    let original = body.definition().unwrap();
    let edges = body.geometry().shell().edges();
    let mut axes = vec![2; edges.len()];
    let mut classes = vec![Class::C0; edges.len()];
    for (index, e) in edges.iter().enumerate() {
        let w = e.world();
        let p = &w.control_points;
        if p.iter().all(|p| p[0] == 5. && (p[1] == 0. || p[1] == 20.)) {
            axes[index] = 1;
            classes[index] = Class::Curvature {
                normal_distance: 1e-8,
                curvature_distance: 1e-6,
            };
        } else if p.iter().all(|p| p[0] == 10. && p[1] == 10.) {
            axes[index] = 0;
            classes[index] = Class::Curvature {
                normal_distance: 1e-8,
                curvature_distance: 1e-6,
            };
        }
    }
    assert_eq!(
        classes
            .iter()
            .filter(|c| matches!(c, Class::Curvature { .. }))
            .count(),
        3
    );
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
    assert!(
        collection
            .curvature_seams()
            .iter()
            .all(|(_, s)| !s.is_exact())
    );
    let limits = || ExactCollectionLimits {
        incidence_steps: 100000,
        exact_work: 100000000,
        cells: 100000,
        curve_spans: 100000,
        shape_cells: 100000,
    };
    let r = seams::qualify_exact_curvature_collection(&collection, &axes, limits()).unwrap();
    let exact = r.certificate.expect(r.reason);
    assert!(std::ptr::eq(exact.original(), &collection));
    assert!(!exact.all_edges_exact_curvature());
    assert_eq!(exact.regular_seams().len(), 3);
    assert!(exact.polar_edges().is_empty());
    assert_eq!(exact.original().classes(), classes);
    assert_eq!(r.steps, classes.len());
    for (index, seam) in exact.regular_seams() {
        assert!(matches!(classes[*index], Class::Curvature { .. }));
        assert!(seam.is_exact());
        assert_eq!(seam.normal_upper(), 0.);
        assert_eq!(seam.curvature_upper(), 0.);
        assert_eq!(seam.edge().definition(), edges[*index].definition());
    }
    let tight = || ExactCollectionLimits {
        incidence_steps: r.steps,
        exact_work: r.exact_work,
        cells: r.cells,
        curve_spans: r.curve_spans,
        shape_cells: r.shape_cells,
    };
    assert!(
        seams::qualify_exact_curvature_collection(&collection, &axes, tight())
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
            _ => l.shape_cells -= 1,
        }
        let denied = seams::qualify_exact_curvature_collection(&collection, &axes, l).unwrap();
        assert!(denied.certificate.is_none());
        if dim > 0 {
            assert!(denied.completed_curvature_edges < 3);
        }
    }
    assert!(
        seams::qualify_exact_curvature_collection(&collection, &axes[..axes.len() - 1], limits())
            .is_err()
    );
    let mut invalid = axes.clone();
    *invalid.last_mut().unwrap() = 3;
    assert!(seams::qualify_exact_curvature_collection(&collection, &invalid, limits()).is_err());
    let first = exact.regular_seams()[0].0;
    let mut mixed = classes.clone();
    mixed[first] = Class::TangentPlanes;
    let r_mixed = seams::qualify_collection(
        &body,
        &mixed,
        seams::Limits {
            max_sine_squared: 1e-8,
            cells: 10000,
            curve_spans: 10000,
            normal_spans: 10000,
        },
    )
    .unwrap();
    let mixed_collection = r_mixed.certificate.expect(r_mixed.reason);
    let r_mixed =
        seams::qualify_exact_curvature_collection(&mixed_collection, &axes, limits()).unwrap();
    let mixed_proof = r_mixed.certificate.expect(r_mixed.reason);
    assert_eq!(mixed_proof.regular_seams().len(), 2);
    assert_eq!(
        mixed_proof.original().classes()[first],
        Class::TangentPlanes
    );
    assert!(!mixed_proof.regular_seams().iter().any(|(i, _)| *i == first));
    let smooth_limits = || seams::ExactSmoothLimits {
        incidence_steps: 1000000,
        exact_work: 100000000,
        cells: 100000,
        curve_spans: 100000,
        source_spans: 100000,
    };
    let smooth =
        seams::qualify_exact_smooth_collection(&mixed_collection, &axes, smooth_limits()).unwrap();
    let smooth_proof = smooth.certificate.expect(smooth.reason);
    assert_eq!(smooth_proof.tangent_seams().len(), 1);
    assert_eq!(smooth_proof.curvature_seams().len(), 2);
    assert_eq!(smooth_proof.tangent_seams()[0].0, first);
    assert_eq!(smooth_proof.original().classes(), mixed);
    assert!(
        smooth_proof.polar_tangent_edges().is_empty()
            && smooth_proof.polar_curvature_edges().is_empty()
    );
    assert!(!smooth_proof.all_edges_exact_tangent() && !smooth_proof.all_edges_exact_curvature());
    let tight = || seams::ExactSmoothLimits {
        incidence_steps: smooth.steps,
        exact_work: smooth.exact_work,
        cells: smooth.cells,
        curve_spans: smooth.curve_spans,
        source_spans: smooth.source_spans,
    };
    assert!(
        seams::qualify_exact_smooth_collection(&mixed_collection, &axes, tight())
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
        let denied = seams::qualify_exact_smooth_collection(&mixed_collection, &axes, l).unwrap();
        assert!(denied.certificate.is_none());
    }
    eprintln!(
        "mixed exact G1/G2: g1=1 g2=2 steps={} work={} cells={} source_spans={}",
        smooth.steps, smooth.exact_work, smooth.cells, smooth.source_spans
    );

    assert_eq!(body.definition().unwrap(), original);
    eprintln!(
        "owned exact regular G2: seams={} steps={} work={} cells={} curve_spans={} shape_cells={}",
        exact.regular_seams().len(),
        r.steps,
        r.exact_work,
        r.cells,
        r.curve_spans,
        r.shape_cells
    );
}
