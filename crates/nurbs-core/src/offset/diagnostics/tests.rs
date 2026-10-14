use super::*;
fn chain(points: &[[f64; 2]]) -> Vec<Segment> {
    points
        .windows(2)
        .enumerate()
        .map(|(i, p)| Segment {
            domain: [i as f64, (i + 1) as f64],
            points: [p[0], p[1]],
            error_upper_mm: 0.,
        })
        .collect()
}
#[test]
fn crossing_and_adjacent_vertices_have_distinct_results() {
    let bow = chain(&[[0., 0.], [2., 2.], [0., 2.], [2., 0.]]);
    let report = inspect_chain(&bow, false, 100).unwrap();
    assert!(report.complete);
    assert_eq!(report.crossings, vec![[0, 2]]);
    let square = chain(&[[0., 0.], [2., 0.], [2., 2.], [0., 2.], [0., 0.]]);
    let report = inspect_chain(&square, true, 100).unwrap();
    assert!(report.complete && report.crossings.is_empty() && report.contacts.is_empty());
}
#[test]
#[cfg(feature = "codec")]
fn overlap_resource_and_degeneracy_do_not_prove_simple() {
    let overlap = chain(&[[0., 0.], [2., 0.], [1., 0.]]);
    let report = inspect_chain(&overlap, false, 100).unwrap();
    assert!(report.complete && report.contacts == vec![[0, 1]] && report.uncertain.is_empty());
    assert_eq!(report.to_value()["simple"], false);
    assert_eq!(report.to_value()["originalOffsetTopologyCertified"], false);
    let square = chain(&[[0., 0.], [2., 0.], [2., 2.], [0., 2.], [0., 0.]]);
    let report = inspect_chain(&square, true, 1).unwrap();
    assert!(!report.complete && report.checks == 1 && report.total_pairs == 6);
    let degenerate = chain(&[[0., 0.], [0., 0.], [1., 0.]]);
    let report = inspect_chain(&degenerate, false, 100).unwrap();
    assert_eq!(report.degenerate, vec![0]);
    assert_eq!(report.to_value()["simple"], false);
}
