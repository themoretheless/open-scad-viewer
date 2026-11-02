use brep_core::feature_ops::edge_partition::{self, Limits};
#[path = "common/full_periodic_frustum.rs"]
mod fixture;
#[test]
fn a_full_periodic_frustum_keeps_actual_rims_and_closed_material() {
    let s = fixture::source(0.5, false);
    let saved = s.clone();
    let p = edge_partition::split(&s, 2, &[0.375], "frustum-seam", Limits::default()).unwrap();
    assert!(p.reverify().unwrap().trim_regions_valid);
    assert_eq!(p.model().1.faces, s.1.faces);
    assert!((p.evidence().numerical_volume_mm3 - 7. * std::f64::consts::PI / 6.).abs() < 1e-5);
    assert_eq!(s, saved);
}
#[test]
fn a_full_periodic_cone_has_an_owned_apex_and_admitted_actual_seam_refinement() {
    let s = fixture::source(0., false);
    let saved = s.clone();
    let p = edge_partition::split(&s, 2, &[0.375], "cone-seam", Limits::default()).unwrap();
    assert!(p.reverify().unwrap().trim_regions_valid);
    assert!(p.model().edges[1].degenerate);
    assert_eq!(p.model().vertices[1], s.vertices[1]);
    assert!((p.evidence().numerical_volume_mm3 - 2. * std::f64::consts::PI / 3.).abs() < 1e-5);
    assert_eq!(s, saved);
}
#[test]
fn the_opposite_parameter_shore_can_own_the_same_physical_cone_apex() {
    let s = fixture::source(0., true);
    let p =
        edge_partition::split(&s, 2, &[0.25, 0.75], "inverted-cone", Limits::default()).unwrap();
    assert!(p.reverify().unwrap().trim_regions_valid);
    assert!((p.evidence().numerical_volume_mm3 - 2. * std::f64::consts::PI / 3.).abs() < 1e-5);
}
#[test]
fn a_rational_seam_clock_keeps_the_original_cone_pole_identity() {
    let mut s = fixture::source(0., false);
    for row in &mut s.faces[0].surface.weights {
        row[1] *= 2.;
    }
    s.edges[2].curve.weights = vec![1., 2.];
    s.validate().unwrap();
    let p = edge_partition::split(&s, 2, &[0.375], "weighted-cone", Limits::default()).unwrap();
    assert!(p.reverify().unwrap().boundary_complete);
    assert_eq!(p.model().1.vertices[1], s.1.vertices[1]);
}
#[test]
fn an_undeclared_pole_or_exhausted_quotient_work_cannot_gain_regular_chart_authority() {
    let s = fixture::source(0., false);
    let mut bad = s.clone();
    bad.edges[1].degenerate = false;
    assert!(edge_partition::split(&bad, 2, &[0.375], "not-declared", Limits::default()).is_err());
    assert!(
        edge_partition::split(
            &s,
            2,
            &[0.375],
            "small",
            Limits {
                max_cells: 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
}
#[test]
fn oblique_cones_and_frusta_keep_lifted_actual_faces_and_pole_ownership() {
    for scale in [0., 0.5] {
        let mut s = fixture::source(scale, false);
        let transform = |p: &mut [f64]| {
            let [x, y, z] = [p[0], p[1], p[2]];
            p.copy_from_slice(&[x + z + 5., y + z - 3., x + 2. * y + 4. * z + 2.]);
        };
        for v in &mut s.vertices {
            transform(&mut v.point);
        }
        for e in &mut s.edges {
            for p in &mut e.curve.control_points {
                transform(p);
            }
        }
        for f in &mut s.faces {
            for row in &mut f.surface.control_points {
                for p in row {
                    transform(p);
                }
            }
        }
        for c in &mut s.loops[0].coedges {
            for p in &mut c.pcurve.control_points {
                p[0] -= 8.;
            }
        }
        s.validate().unwrap();
        let saved = s.clone();
        let p = edge_partition::split(&s, 2, &[0.375], "oblique-taper", Limits::default()).unwrap();
        assert!(p.reverify().unwrap().trim_regions_valid);
        assert_eq!(p.model().1.faces, s.1.faces);
        assert!(
            (p.evidence().numerical_volume_mm3
                - 2. * std::f64::consts::PI * (1. + scale + scale * scale) / 3.)
                .abs()
                < 1e-5
        );
        assert_eq!(s, saved);
    }
}
