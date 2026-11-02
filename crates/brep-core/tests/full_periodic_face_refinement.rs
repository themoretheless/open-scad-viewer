use brep_core::feature_ops::edge_partition::{self, Limits};
#[path = "common/full_periodic_cylinder.rs"]
mod fixture;
#[test]
fn a_full_lap_cylinder_face_admits_actual_rim_and_shared_seam_refinement() {
    let s = fixture::source();
    let saved = s.clone();
    let rim = edge_partition::split(&s, 0, &[0.375, 2.25], "full-rim", Limits::default()).unwrap();
    assert!(rim.reverify().unwrap().trim_regions_valid);
    assert_eq!(rim.model().1.faces, s.1.faces);
    assert!((rim.evidence().numerical_volume_mm3 - 2. * std::f64::consts::PI).abs() < 1e-5);
    let seam = edge_partition::split(&s, 2, &[0.375], "full-seam", Limits::default()).unwrap();
    assert!(seam.reverify().unwrap().boundary_complete);
    assert_eq!(seam.model().loops[0].coedges.len(), 6);
    assert_eq!(s, saved);
}
#[test]
fn whole_lap_lifted_uv_clocks_retain_actual_original_chart_authority() {
    let mut s = fixture::source();
    for c in &mut s.loops[0].coedges {
        for p in &mut c.pcurve.control_points {
            p[0] += 8.;
        }
    }
    s.validate().unwrap();
    let saved = s.clone();
    let p = edge_partition::split(&s, 2, &[0.25, 0.75], "lifted-full", Limits::default()).unwrap();
    assert!(p.reverify().unwrap().trim_regions_valid);
    assert_eq!(s, saved);
}
#[test]
fn wrong_source_seams_multiple_laps_and_exhausted_quotient_work_refuse() {
    let s = fixture::source();
    let saved = s.clone();
    assert!(
        edge_partition::split(
            &s,
            2,
            &[0.375],
            "small",
            Limits {
                max_pairs: 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    let mut bad = s.clone();
    bad.faces[0].surface.knots_u[0] = (-1f64).next_up();
    assert!(edge_partition::split(&bad, 2, &[0.375], "bad-seam", Limits::default()).is_err());
    let mut bad = s.clone();
    for c in &mut bad.loops[0].coedges {
        for p in &mut c.pcurve.control_points {
            p[0] *= 2.;
        }
    }
    assert!(edge_partition::split(&bad, 2, &[0.375], "double", Limits::default()).is_err());
    assert_eq!(s, saved);
}
#[test]
fn transposed_full_lap_face_keeps_both_actual_uses_and_oriented_material() {
    let mut s = fixture::source();
    let old = s.faces[0].surface.clone();
    let n = old.control_points.len();
    s.faces[0].surface = nurbs_core::surface::Surface {
        degree_u: old.degree_v,
        degree_v: old.degree_u,
        knots_u: old.knots_v,
        knots_v: old.knots_u,
        control_points: (0..2)
            .map(|j| (0..n).map(|i| old.control_points[i][j].clone()).collect())
            .collect(),
        weights: (0..2)
            .map(|j| (0..n).map(|i| old.weights[i][j]).collect())
            .collect(),
        periodic_u: false,
        periodic_v: true,
    };
    s.loops[0].coedges.reverse();
    for c in &mut s.loops[0].coedges {
        c.pcurve = c.pcurve.reverse().unwrap();
        for p in &mut c.pcurve.control_points {
            p.swap(0, 1);
        }
        c.reversed = !c.reversed;
    }
    s.shells[0].faces[0].reversed = true;
    s.validate().unwrap();
    let saved = s.clone();
    let p = edge_partition::split(&s, 2, &[0.375], "full-v-period", Limits::default()).unwrap();
    assert!(p.reverify().unwrap().trim_regions_valid);
    assert!((p.evidence().numerical_volume_mm3 - 2. * std::f64::consts::PI).abs() < 1e-5);
    assert_eq!(s, saved);
}
