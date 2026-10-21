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
fn coverage(surface: &Surface, wrapped: [bool; 2]) -> Arc<p::OwnedOriginalTrimChartCoverage> {
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
    Arc::new(p::own_original_trim_chart_coverage(topology, Default::default()).unwrap())
}
#[test]
fn regular_plane_owns_actual_source_normal_proof_and_budget() {
    let c = coverage(&plane(), [false, false]);
    let owner = p::own_original_regular_trim_surface(c.clone(), 1).unwrap();
    owner.reverify_source().unwrap();
    assert_eq!(owner.record().proven_cells, 1);
    assert!(owner.record().interior_basis_c1);
    assert!(p::own_original_regular_trim_surface(c, 0).is_err());
}
#[test]
fn complete_original_knot_grid_and_one_sided_crease_normals_are_preserved() {
    let curve = Curve::from_polyline(vec![
        vec![0., 0., 0.],
        vec![1., 0., 0.],
        vec![1., 1., 0.],
        vec![0., 1., 0.],
        vec![0., 0., 0.],
    ])
    .unwrap();
    let surface = surface::extrude(&curve, [0., 0., 1.]).unwrap();
    let c = coverage(&surface, [true, false]);
    assert!(p::own_original_regular_trim_surface(c.clone(), 1).is_err());
    let owner = p::own_original_regular_trim_surface(c, 64).unwrap();
    owner.reverify_source().unwrap();
    assert_eq!(owner.record().proven_cells, 4);
    assert!(!owner.record().interior_basis_c1);
}
#[test]
fn folded_and_collapsed_world_surfaces_cannot_inherit_valid_uv_topology() {
    let mut folded = plane();
    folded.degree_u = 2;
    folded.knots_u = vec![0., 0., 0., 1., 1., 1.];
    folded.control_points = vec![
        vec![vec![0., 0., 0.], vec![0., 1., 0.]],
        vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        vec![vec![0., 0., 0.], vec![0., 1., 0.]],
    ];
    folded.weights = vec![vec![1.; 2]; 3];
    assert!(p::own_original_regular_trim_surface(coverage(&folded, [false, false]), 63).is_err());
    let mut collapsed = plane();
    for row in &mut collapsed.control_points {
        for point in row {
            point[0] = 0.;
        }
    }
    assert!(
        p::own_original_regular_trim_surface(coverage(&collapsed, [false, false]), 31).is_err()
    );
}
#[test]
fn actual_rational_torus_normals_are_proved_with_periodic_quotient_ownership() {
    let surface = nurbs_core::primitives::torus([0.; 3], 3., 1., 1.).unwrap();
    let owner =
        p::own_original_regular_trim_surface(coverage(&surface, [true, true]), 4096).unwrap();
    owner.reverify_source().unwrap();
    assert!(owner.record().proven_cells >= 16);
}
