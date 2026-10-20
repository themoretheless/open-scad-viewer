use brep_core::{radius_transition_body as body, watertight};
use nurbs_core::radius_transition::Options;
#[test]
fn unequal_radius_transition_closes_with_two_explicit_original_g2_seams() {
    let out = body::build(Options::default()).unwrap();
    println!(
        "body faces={} edges={} vertices={} cells={} exact={} coef={}",
        out.model.faces.len(),
        out.model.edges.len(),
        out.model.vertices.len(),
        out.cells,
        out.exact_work,
        out.transition.coefficient_work
    );
    assert_eq!(out.model.faces.len(), 11);
    assert_eq!(out.model.edges.len(), 21);
    assert_eq!(out.model.vertices.len(), 12);
    assert!(
        out.cells <= 100000
            && out.exact_work <= 1000000
            && out.transition.coefficient_work <= 2000000
    );
    assert!(out.trims.all_valid && out.transition.admitted);
    for edge in out.smooth_edges {
        assert_eq!(
            out.model
                .loops
                .iter()
                .flat_map(|l| &l.coedges)
                .filter(|c| c.edge == edge)
                .count(),
            2
        );
    }
    let saved = value_codec::to_string(&out.model).unwrap();
    let restored: brep_core::Model = value_codec::from_str(&saved).unwrap();
    assert_eq!(restored, out.model);
    assert_eq!(
        watertight::inspect(&restored, 100000).unwrap().status,
        watertight::Closure::WithinTolerance
    );
    assert!(
        body::build(Options {
            max_cells: out.cells - 1,
            ..Default::default()
        })
        .is_err()
    );
}

#[test]
fn reversed_radii_and_a_rotated_frame_preserve_closed_source_ownership() {
    let angle = 0.37_f64;
    let out = body::build(Options {
        radii_mm: [0.3, 0.1],
        origin_mm: [2., -3., 1.],
        axis: [angle.sin(), 0., angle.cos()],
        radial: [angle.cos(), 0., -angle.sin()],
        ..Default::default()
    })
    .unwrap();
    assert!(out.transition.admitted && out.trims.all_valid);
    assert_eq!(
        out.closure.closure.status,
        watertight::Closure::WithinTolerance
    );
    assert_eq!(out.model.bodies.len(), 1);
    assert!(out.model.persistent_naming_complete());
    assert!(out.cells <= 100000 && out.exact_work <= 1000000);
}
