use brep_core::profile_certificate as p;
use nurbs_core::{
    curve::Curve,
    surface::{self, Surface},
};
use std::sync::Arc;
fn rectangle(lo: [f64; 2], hi: [f64; 2]) -> Vec<Curve> {
    let v = [lo, [hi[0], lo[1]], hi, [lo[0], hi[1]], lo];
    v.windows(2)
        .map(|p| Curve::from_polyline(vec![p[0].to_vec(), p[1].to_vec()]).unwrap())
        .collect()
}
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
fn tube() -> Surface {
    let c = Curve::from_polyline(vec![
        vec![0., 0., 0.],
        vec![1., 0., 0.],
        vec![1., 1., 0.],
        vec![0., 1., 0.],
        vec![0., 0., 0.],
    ])
    .unwrap();
    surface::extrude(&c, [0., 0., 1.]).unwrap()
}
fn seams(
    surface: &Surface,
    loops: &[Vec<Curve>],
    weights: &[i32],
    wrapped: [bool; 2],
) -> Arc<p::OwnedOriginalSurfaceSeams> {
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
    let regions = Arc::new(
        p::assemble_original_filled_regions(
            loops,
            weights,
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
    Arc::new(p::bind_original_seams_to_surface(dictionary, surface).unwrap())
}
#[test]
fn cylinder_quotient_has_two_boundary_loops_and_interval_vertex_links() {
    let s = tube();
    let t = p::own_original_periodic_topology(seams(
        &s,
        &[rectangle([0., 0.], [4., 1.])],
        &[1],
        [true, false],
    ))
    .unwrap();
    t.reverify_source().unwrap();
    assert_eq!(t.edges().len(), 3);
    assert_eq!(t.vertex_links().len(), 2);
    assert!(t.vertex_links().iter().all(|l| l.boundary));
    assert_eq!(t.boundaries().len(), 2);
    assert_eq!(t.euler_characteristics(), [0]);
    assert_eq!(t.components(), [vec![0]]);
}
#[test]
fn torus_quotient_keeps_distinct_loop_edge_darts_at_one_vertex() {
    let s = nurbs_core::primitives::torus([0.; 3], 3., 1., 1.).unwrap();
    let hi = [
        s.knots_u[s.control_points.len()],
        s.knots_v[s.control_points[0].len()],
    ];
    let lo = [s.knots_u[s.degree_u], s.knots_v[s.degree_v]];
    let t = p::own_original_periodic_topology(seams(&s, &[rectangle(lo, hi)], &[1], [true, true]))
        .unwrap();
    t.reverify_source().unwrap();
    assert_eq!(t.edges().len(), 2);
    assert!(
        t.edges()
            .iter()
            .all(|e| e.vertices[0] == e.vertices[1] && e.halfedges.len() == 2)
    );
    assert_eq!(t.vertex_links().len(), 1);
    assert_eq!(t.vertex_links()[0].darts.len(), 4);
    assert!(!t.vertex_links()[0].boundary);
    assert!(t.boundaries().is_empty());
    assert_eq!(t.euler_characteristics(), [0]);
}
#[test]
fn annular_face_holes_contribute_to_actual_euler_characteristic() {
    let s = plane();
    let loops = [
        rectangle([0., 0.], [1., 1.]),
        rectangle([0.25, 0.25], [0.75, 0.75]),
    ];
    let t = p::own_original_periodic_topology(seams(&s, &loops, &[1, -1], [false, false])).unwrap();
    t.reverify_source().unwrap();
    assert_eq!(t.edges().len(), 8);
    assert_eq!(t.boundaries().len(), 2);
    assert_eq!(t.euler_characteristics(), [0]);
}
#[test]
fn point_touching_material_patches_refuse_a_nonmanifold_vertex_link() {
    let s = plane();
    let loops = [
        rectangle([0., 0.], [0.5, 0.5]),
        rectangle([0.5, 0.5], [1., 1.]),
    ];
    let owner = seams(&s, &loops, &[1, 1], [false, false]);
    assert!(p::own_original_periodic_topology(owner).is_err());
}
#[test]
fn separated_chart_patches_join_into_one_material_component_through_the_seam() {
    let s = tube();
    let loops = [
        rectangle([0., 0.25], [1., 0.75]),
        rectangle([3., 0.25], [4., 0.75]),
    ];
    let t = p::own_original_periodic_topology(seams(&s, &loops, &[1, 1], [true, false])).unwrap();
    t.reverify_source().unwrap();
    assert_eq!(t.components(), [vec![0, 1]]);
    assert_eq!(t.boundaries().len(), 1);
    assert_eq!(t.euler_characteristics(), [1]);
    assert_eq!(t.edges().len(), 7);
}
