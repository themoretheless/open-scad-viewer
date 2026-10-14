use nurbs_core::{
    chord_arrangement::{self, Arrangement},
    chord_fill_selection::{boundaries, boundary_curves},
    chord_winding::FillRule,
    curve_offset::Segment,
};
fn graph(points: &[[f64; 2]]) -> Arrangement {
    let segments: Vec<_> = points
        .windows(2)
        .enumerate()
        .map(|(i, p)| Segment {
            domain: [i as f64, (i + 1) as f64],
            points: [p[0], p[1]],
            error_upper_mm: 0.,
        })
        .collect();
    chord_arrangement::split(&segments, true, 1e-6, 100000).unwrap()
}
fn measures(g: &Arrangement, c: &[usize]) -> (f64, f64) {
    let mut area = 0.;
    let mut perimeter = 0.;
    for i in 0..c.len() {
        let a = g.vertices[c[i]].point;
        let b = g.vertices[c[(i + 1) % c.len()]].point;
        area += a[0] * b[1] - a[1] * b[0];
        perimeter += (a[0] - b[0]).hypot(a[1] - b[1]);
    }
    (area * 0.5, perimeter)
}
#[test]
fn shared_edge_is_removed_from_merged_union_boundary() {
    let g = graph(&[
        [0., 0.],
        [4., 0.],
        [4., 4.],
        [0., 4.],
        [0., 0.],
        [0., -2.],
        [2., -2.],
        [2., 0.],
        [0., 0.],
    ]);
    for rule in [FillRule::NonZero, FillRule::EvenOdd] {
        let loops = boundaries(&g, rule, 10000).unwrap();
        assert_eq!(loops.len(), 1);
        let (area, perimeter) = measures(&g, &loops[0]);
        assert_eq!(area, 20.);
        assert_eq!(perimeter, 20.);
        let curves = boundary_curves(&g, rule, 10000).unwrap();
        assert_eq!(curves.len(), 1);
        assert_eq!(
            curves[0][0].control_points.first(),
            curves[0].last().unwrap().control_points.last()
        );
    }
}
#[test]
fn crossing_chain_becomes_two_filled_lobes() {
    let g = graph(&[[0., 0.], [4., 4.], [0., 4.], [4., 0.], [0., 0.]]);
    for rule in [FillRule::NonZero, FillRule::EvenOdd] {
        let loops = boundaries(&g, rule, 10000).unwrap();
        assert_eq!(loops.len(), 2);
        assert!(loops.iter().all(|c| c.len() == 3));
        assert_eq!(loops.iter().map(|c| measures(&g, c).0).sum::<f64>(), 8.);
    }
}
#[test]
fn doubled_loop_distinguishes_nonzero_and_evenodd() {
    let g = graph(&[
        [0., 0.],
        [4., 0.],
        [4., 4.],
        [0., 4.],
        [0., 0.],
        [4., 0.],
        [4., 4.],
        [0., 4.],
        [0., 0.],
    ]);
    assert_eq!(boundaries(&g, FillRule::NonZero, 10000).unwrap().len(), 1);
    assert!(boundaries(&g, FillRule::EvenOdd, 10000).unwrap().is_empty());
    assert!(boundaries(&g, FillRule::NonZero, 0).is_err());
}
