use brep_core::feature_ops::edge_partition::{self, Limits};
#[path = "common/full_periodic_cylinder.rs"]
mod fixture;
fn transform(p: &mut [f64]) {
    let [x, y, z] = [p[0], p[1], p[2]];
    p.copy_from_slice(&[x + z + 5., y + z - 3., x + 2. * y + 4. * z + 2.]);
}
fn source() -> brep_core::Model {
    let mut s = fixture::source();
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
    s.validate().unwrap();
    s
}
#[test]
fn full_lap_oblique_faces_keep_actual_seam_incidence_and_material_after_refinement() {
    let s = source();
    let saved = s.clone();
    let r =
        edge_partition::split(&s, 2, &[0.375, 0.75], "oblique-seam", Limits::default()).unwrap();
    assert!(r.reverify().unwrap().trim_regions_valid);
    assert_eq!(r.model().1.faces, s.1.faces);
    assert!((r.evidence().numerical_volume_mm3 - 2. * std::f64::consts::PI).abs() < 1e-5);
    assert_eq!(s, saved);
}
#[test]
fn oblique_closed_rim_refinement_and_independent_sheet_lifts_remain_original_bound() {
    let mut s = source();
    for c in &mut s.loops[0].coedges {
        for p in &mut c.pcurve.control_points {
            p[0] -= 8.;
        }
    }
    let saved = s.clone();
    let r = edge_partition::split(&s, 0, &[0.375, 2.25], "oblique-rim", Limits::default()).unwrap();
    assert!(r.reverify().unwrap().boundary_complete);
    assert_eq!(s, saved);
}
#[test]
fn positive_row_gauge_and_actual_rational_seam_clock_preserve_the_closed_body() {
    let mut s = source();
    for row in &mut s.faces[0].surface.weights {
        row[1] *= 2.;
    }
    s.edges[2].curve.weights = vec![1., 2.];
    s.validate().unwrap();
    let saved = s.clone();
    let r =
        edge_partition::split(&s, 2, &[0.375], "weighted-oblique-seam", Limits::default()).unwrap();
    assert!(r.reverify().unwrap().trim_regions_valid);
    assert!((r.evidence().numerical_volume_mm3 - 2. * std::f64::consts::PI).abs() < 1e-5);
    assert_eq!(s, saved);
}
