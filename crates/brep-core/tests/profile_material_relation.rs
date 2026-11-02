use brep_core::profile_certificate::material_relation::{self as m, Limits};
use nurbs_core::curve::Curve;
fn rect(a: [f64; 2], b: [f64; 2]) -> Vec<Curve> {
    let p = [a, [b[0], a[1]], b, [a[0], b[1]]];
    (0..4)
        .map(|i| Curve::from_polyline(vec![p[i].to_vec(), p[(i + 1) % 4].to_vec()]).unwrap())
        .collect()
}
#[test]
fn exact_material_relation_checks_equality_and_actual_removed_regions() {
    let source = vec![rect([0., 0.], [4., 4.])];
    let equal = m::inspect(&source, &source, 1e-12, Limits::default()).unwrap();
    assert!(equal.result_is_subset() && equal.source_is_subset());
    let result = vec![rect([0., 0.], [3., 4.])];
    let r = m::inspect(&source, &result, 1e-12, Limits::default()).unwrap();
    assert!(r.result_is_subset());
    assert!(!r.removed().is_empty());
    assert!(!r.source_is_subset());
    assert!(r.edge_count() > 0);
    r.reverify().unwrap();
    r.require_no_addition().unwrap();
}
#[test]
fn exact_material_relation_detects_filled_hole_despite_identical_outer_boundary() {
    let outer = rect([0., 0.], [4., 4.]);
    let mut hole = rect([1., 1.], [3., 3.]);
    hole.reverse();
    hole = hole.into_iter().map(|c| c.reverse().unwrap()).collect();
    let source = vec![outer.clone(), hole];
    let result = vec![outer];
    let r = m::inspect(&source, &result, 1e-12, Limits::default()).unwrap();
    assert!(!r.result_is_subset());
    assert!(!r.added().is_empty());
    assert!(
        r.added()
            .iter()
            .all(|s| s.source_winding == 0 && s.result_winding != 0)
    );
    assert!(r.require_no_addition().is_err());
}
#[test]
fn unresolved_material_census_never_authorizes_subset() {
    let source = vec![rect([0., 0.], [4., 4.])];
    let limits = Limits {
        ports: brep_core::profile_certificate::ContactPortOrderLimits {
            max_ports: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(m::inspect(&source, &source, 1e-12, limits).is_err());
}

#[test]
fn filled_material_equality_is_not_equality_of_winding_multiplicity() {
    let outer = rect([0., 0.], [4., 4.]);
    let r = m::inspect(
        &[outer.clone()],
        &[outer.clone(), outer],
        1e-12,
        Limits::default(),
    )
    .unwrap();
    assert!(r.result_is_subset() && r.source_is_subset());
    assert!(r.added().is_empty() && r.removed().is_empty());
}

#[test]
fn crossing_profiles_have_both_added_and_removed_material() {
    let source = vec![rect([0., 0.], [3., 2.])];
    let result = vec![rect([1., -1.], [2., 3.])];
    let relation = m::inspect(&source, &result, 1e-12, Limits::default()).unwrap();
    assert!(!relation.result_is_subset());
    assert!(!relation.source_is_subset());
    assert!(!relation.added().is_empty());
    assert!(!relation.removed().is_empty());
    relation.reverify().unwrap();
    assert!(relation.require_no_addition().is_err());
}

#[test]
fn disconnected_added_island_cannot_hide_behind_retained_source_component() {
    let retained = rect([0., 0.], [2., 2.]);
    let island = rect([4., 0.], [5., 1.]);
    let relation = m::inspect(
        &[retained.clone()],
        &[retained, island],
        1e-12,
        Limits::default(),
    )
    .unwrap();
    assert!(relation.source_is_subset());
    assert!(!relation.result_is_subset());
    assert!(!relation.added().is_empty());
    assert!(relation.removed().is_empty());
    relation.reverify().unwrap();
    assert!(relation.require_no_addition().is_err());
}

#[test]
fn thin_removed_band_uses_geometry_scale_without_more_radius_halvings() {
    let gap=2f64.powi(-45);
    let source=vec![rect([0.,0.],[1.,1.])];
    let result=vec![rect([gap,gap],[1.-gap,1.-gap])];
    let relation=m::inspect(&source,&result,1e-12,Limits::default()).unwrap();
    assert!(relation.result_is_subset());
    assert!(!relation.removed().is_empty());
    assert!(!relation.source_is_subset());
    relation.reverify().unwrap();
    relation.require_no_addition().unwrap();
}
