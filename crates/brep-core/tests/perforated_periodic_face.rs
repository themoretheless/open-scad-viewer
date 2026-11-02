use brep_core::feature_ops::edge_partition::{self, Limits};
#[path = "common/perforated_periodic_cylinder.rs"]
mod fixture;
#[test]
fn an_internal_trim_hole_keeps_real_shared_curved_patch_ownership() {
    let s = fixture::source([0.5, 1.5], [0.25, 0.75]);
    let saved = s.clone();
    let p = edge_partition::split(&s, 3, &[0.375], "hole-boundary", Limits::default()).unwrap();
    assert!(p.reverify().unwrap().trim_regions_valid);
    assert_eq!(p.model().faces[0].holes, s.faces[0].holes);
    assert_eq!(p.model().1.faces, s.1.faces);
    assert_eq!(p.model().loops[3].coedges.len(), 5);
    assert_eq!(p.model().loops[4].coedges.len(), 5);
    assert!((p.evidence().numerical_volume_mm3 - 2. * std::f64::consts::PI).abs() < 1e-5);
    assert_eq!(s, saved);
}
#[test]
fn outer_seam_refinement_does_not_rewrite_the_internal_hole() {
    let s = fixture::source([0.5, 1.5], [0.25, 0.75]);
    let p =
        edge_partition::split(&s, 2, &[0.25, 0.75], "outer-seam-hole", Limits::default()).unwrap();
    assert_eq!(p.model().loops[3], s.loops[3]);
    assert_eq!(p.model().loops[4], s.loops[4]);
    assert!(p.reverify().unwrap().boundary_complete);
}
#[test]
fn authored_partial_hole_clocks_remain_exactly_owned_on_both_faces() {
    let mut s = fixture::source([0.5, 1.5], [0.25, 0.75]);
    for wire in [3, 4] {
        for c in &mut s.loops[wire].coedges {
            if c.edge == 3 {
                let mut g = [0.2, 0.8];
                let mut p = g;
                if c.reversed {
                    g.reverse();
                    p.reverse();
                }
                *c = c
                    .clone()
                    .with_trim(brep_topology::CoedgeTrim {
                        curve_parameter: g,
                        pcurve_parameter: p,
                        periodic_lift: [[0; 2]; 2],
                    })
                    .unwrap();
            }
        }
    }
    let saved = s.clone();
    let p = edge_partition::split(&s, 3, &[0.375], "hole-clocks", Limits::default()).unwrap();
    assert_eq!(p.relations().len(), 2);
    assert!(p.reverify().unwrap().trim_regions_valid);
    assert_eq!(s, saved);
}
#[test]
fn touching_or_wrongly_oriented_holes_and_exhausted_work_cannot_publish() {
    let s = fixture::source([0., 0.5], [0.25, 0.75]);
    let touch = edge_partition::split(&s, 3, &[0.375], "touch", Limits::default())
        .err()
        .unwrap();
    assert!(
        touch.message.contains("trim winding/simplicity"),
        "{touch:?}"
    );
    let mut bad = fixture::source([0.5, 1.5], [0.25, 0.75]);
    bad.loops[3].coedges.reverse();
    for c in &mut bad.loops[3].coedges {
        c.pcurve = c.pcurve.reverse().unwrap();
        c.reversed = !c.reversed;
    }
    // Preserve opposed shell directions: rejection must identify the original
    // UV hole orientation rather than the coarse edge-incidence guard.
    bad.shells[0].faces.last_mut().unwrap().reversed = true;
    let orientation = edge_partition::split(&bad, 3, &[0.375], "wrong-winding", Limits::default())
        .err()
        .unwrap();
    assert_eq!(orientation.code, "BREP_INVALID_TOPOLOGY");
    assert!(
        orientation.message.contains("hole loops CW"),
        "{orientation:?}"
    );
    let s = fixture::source([0.5, 1.5], [0.25, 0.75]);
    let saved = s.clone();
    assert!(
        edge_partition::split(
            &s,
            3,
            &[0.375],
            "budget",
            Limits {
                max_pairs: 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert_eq!(s, saved);
}
#[test]
fn oblique_perforated_faces_with_lifted_hole_and_outer_clocks_remain_qualified() {
    let mut s = fixture::source([0.5, 1.5], [0.25, 0.75]);
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
    for wire in [0, 3, 4] {
        for c in &mut s.loops[wire].coedges {
            for p in &mut c.pcurve.control_points {
                p[0] += 8.;
            }
        }
    }
    s.validate().unwrap();
    let saved = s.clone();
    let p = edge_partition::split(&s, 3, &[0.375], "oblique-hole", Limits::default()).unwrap();
    assert!(p.reverify().unwrap().trim_regions_valid);
    assert!((p.evidence().numerical_volume_mm3 - 2. * std::f64::consts::PI).abs() < 1e-5);
    assert_eq!(s, saved);
}
