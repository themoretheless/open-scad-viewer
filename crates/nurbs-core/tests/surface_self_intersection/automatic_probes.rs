use super::*;
#[test]
fn automatic_audit_finds_the_off_center_regular_contact() {
    let s = super::iso_probe::quarter_surface();
    let r = surface_self_intersection::inspect(&s, 1000).unwrap();
    assert_eq!(r.status, Status::Present);
    let c = r.certified_crossing.unwrap();
    for p in c.parameter_boxes {
        assert_eq!(p[1], [0.25, 0.25]);
    }
    assert!(
        r.probe_search
            .attempts
            .iter()
            .any(|p| p.axis == 0 && p.parameter == 0.25)
    );
    assert!(r.probe_search.cells <= 1000);
    assert_eq!(
        r.probe_search.cells,
        r.probe_search
            .attempts
            .iter()
            .map(|p| p.cells)
            .sum::<usize>()
    );
}
#[test]
fn exhausted_admission_budget_stays_unresolved() {
    let r = surface_self_intersection::inspect(&super::iso_probe::quarter_surface(), 1).unwrap();
    assert_eq!(r.status, Status::Unresolved);
    assert_eq!(r.probe_search.cells, 1);
    assert!(r.probe_search.exhausted);
    assert_eq!(r.probe_search.attempts.len(), 1);
}
#[test]
fn zero_initial_solver_work_is_not_an_unbounded_probe_loop() {
    let mut s = super::within_span::loop_surface();
    for (i, row) in s.control_points.iter_mut().enumerate() {
        let z = if i == 3 { 0.25 } else { 0. };
        row[0][2] = z;
        row[1][2] = 2. + z;
    }
    let r = surface_self_intersection::inspect(&s, 1000).unwrap();
    assert_eq!(r.status, Status::Unresolved);
    assert!(r.probe_search.attempts.len() <= 30);
    assert!(r.probe_search.cells <= 1000);
}
