use brep_core::profile_certificate as p;
use nurbs_core::{
    curve::Curve,
    surface::{self, Surface},
};
use std::sync::Arc;
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        weights: vec![vec![1.; 2]; 2],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        periodic_u: false,
        periodic_v: false,
    }
}
fn regularity(surface: &Surface, wrapped: [bool; 2]) -> Arc<p::OwnedOriginalRegularTrimSurface> {
    let chart = [
        [
            surface.knots_u[surface.degree_u],
            surface.knots_u[surface.control_points.len()],
        ],
        [
            surface.knots_v[surface.degree_v],
            surface.knots_v[surface.control_points[0].len()],
        ],
    ];
    let lo = [chart[0][0], chart[1][0]];
    let hi = [chart[0][1], chart[1][1]];
    let v = [lo, [hi[0], lo[1]], hi, [lo[0], hi[1]], lo];
    let wire = v
        .windows(2)
        .map(|p| Curve::from_polyline(vec![p[0].to_vec(), p[1].to_vec()]).unwrap())
        .collect();
    let regions = Arc::new(
        p::assemble_original_filled_regions(
            &[wire],
            &[1],
            1e-12,
            Default::default(),
            Default::default(),
        )
        .unwrap(),
    );
    let vertices = Arc::new(
        p::own_original_chart_vertices(
            regions.owner().cycles().graph().clone(),
            chart,
            wrapped,
            Default::default(),
        )
        .unwrap(),
    );
    let dictionary =
        Arc::new(p::own_original_seam_dictionary(regions, vertices, Default::default()).unwrap());
    let seams = Arc::new(p::bind_original_seams_to_surface(dictionary, surface).unwrap());
    let topology = Arc::new(p::own_original_periodic_topology(seams).unwrap());
    let coverage =
        Arc::new(p::own_original_trim_chart_coverage(topology, Default::default()).unwrap());
    Arc::new(p::own_original_regular_trim_surface(coverage, 4096).unwrap())
}
#[test]
fn plane_atlas_owns_one_actual_injective_chart_and_shared_work() {
    let r = regularity(&plane(), [false, false]);
    let a = p::own_original_injective_chart_atlas(r.clone(), Default::default()).unwrap();
    a.reverify_source().unwrap();
    assert_eq!(a.charts().len(), 1);
    assert_eq!(a.created_cells(), 1);
    assert_eq!(a.span_work(), 1);
    assert_eq!(a.charts()[0].domain, [[0., 1.], [0., 1.]]);
    assert!(
        p::own_original_injective_chart_atlas(
            r,
            p::OriginalInjectiveAtlasLimits {
                max_span_work: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn rational_torus_needs_actual_subcharts_and_no_global_projection_is_inferred() {
    let surface = nurbs_core::primitives::torus([0.; 3], 3., 1., 1.).unwrap();
    let r = regularity(&surface, [true, true]);
    let limited = p::OriginalInjectiveAtlasLimits {
        max_cells: 16,
        max_depth: 0,
        ..Default::default()
    };
    assert!(p::own_original_injective_chart_atlas(r.clone(), limited).is_err());
    let a = p::own_original_injective_chart_atlas(r, Default::default()).unwrap();
    a.reverify_source().unwrap();
    assert!(a.charts().len() > 16);
    assert!(a.created_cells() > a.charts().len());
    assert!(a.span_work() > a.charts().iter().map(|c| c.span_work).sum::<usize>());
    assert!(
        a.charts()
            .iter()
            .all(|c| c.contraction_upper < 1. && c.domain.iter().all(|a| a[0] < a[1]))
    );
}
#[test]
fn local_atlas_does_not_claim_global_embedding_for_a_doubly_covered_tube() {
    let curve = Curve::from_polyline(vec![
        vec![0., 0., 0.],
        vec![1., 0., 0.],
        vec![1., 1., 0.],
        vec![0., 1., 0.],
        vec![0., 0., 0.],
        vec![1., 0., 0.],
        vec![1., 1., 0.],
        vec![0., 1., 0.],
        vec![0., 0., 0.],
    ])
    .unwrap();
    let surface = surface::extrude(&curve, [0., 0., 1.]).unwrap();
    assert_eq!(
        surface.evaluate(0.5, 0.5).unwrap().point,
        surface.evaluate(4.5, 0.5).unwrap().point
    );
    let a = p::own_original_injective_chart_atlas(
        regularity(&surface, [true, false]),
        Default::default(),
    )
    .unwrap();
    a.reverify_source().unwrap();
    assert_eq!(a.charts().len(), 8);
}
