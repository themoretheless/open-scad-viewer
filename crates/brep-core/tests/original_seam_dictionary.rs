use brep_core::profile_certificate as p;
use nurbs_core::curve::Curve;
use std::sync::Arc;
fn line(a: [f64; 2], b: [f64; 2]) -> Curve {
    Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap()
}
fn wire(nonlinear: bool) -> Vec<Curve> {
    let right = if nonlinear {
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            weights: vec![1.; 3],
            control_points: vec![vec![1., 0.], vec![1., 0.], vec![1., 1.]],
            periodic: false,
        }
    } else {
        line([1., 0.], [1., 1.])
    };
    vec![
        line([0., 0.], [1., 0.]),
        right,
        line([1., 1.], [0., 1.]),
        line([0., 1.], [0., 0.]),
    ]
}
fn setup(
    wire: Vec<Curve>,
    wrapped: [bool; 2],
) -> (
    Arc<p::OwnedOriginalFilledRegions>,
    Arc<p::OwnedOriginalChartVertices>,
) {
    let regions = Arc::new(
        p::assemble_original_filled_regions(
            &[wire],
            &[1],
            1e-12,
            Default::default(),
            Default::default(),
        )
        .unwrap(),
    );
    let chart = Arc::new(
        p::own_original_chart_vertices(
            regions.owner().cycles().graph().clone(),
            [[0., 1.], [0., 1.]],
            wrapped,
            Default::default(),
        )
        .unwrap(),
    );
    (regions, chart)
}
#[test]
fn complete_cylinder_dictionary_pairs_nonlinear_clocks_with_opposite_material_uses() {
    let (r, c) = setup(wire(true), [true, false]);
    let d = p::own_original_seam_dictionary(r, c, Default::default()).unwrap();
    d.reverify_source().unwrap();
    assert_eq!(d.census().len(), 8);
    assert_eq!(d.uses().len(), 2);
    assert_eq!(d.pairs().len(), 1);
    assert_eq!(
        d.census()
            .iter()
            .filter(|c| c.3 == p::OriginalShoreMembership::Entire)
            .count(),
        2
    );
    assert_eq!(
        d.census()
            .iter()
            .filter(|c| c.3 == p::OriginalShoreMembership::Absent)
            .count(),
        6
    );
}
#[test]
fn complete_torus_dictionary_retains_two_separate_shore_pairs_at_one_corner() {
    let (r, c) = setup(wire(false), [true, true]);
    let d = p::own_original_seam_dictionary(r, c, Default::default()).unwrap();
    d.reverify_source().unwrap();
    assert_eq!(d.chart().groups().len(), 1);
    assert_eq!(d.census().len(), 16);
    assert_eq!(d.uses().len(), 4);
    assert_eq!(d.pairs().len(), 2);
}
#[test]
fn unmatched_and_partial_original_shore_intervals_never_become_complete_dictionary() {
    let trapezoid = vec![
        line([0., 0.], [1., 0.25]),
        line([1., 0.25], [1., 0.75]),
        line([1., 0.75], [0., 1.]),
        line([0., 1.], [0., 0.]),
    ];
    let (r, c) = setup(trapezoid, [true, false]);
    assert!(p::own_original_seam_dictionary(r, c, Default::default()).is_err());
    let (r, c) = setup(wire(false), [true, false]);
    let (_, foreign) = setup(wire(false), [true, false]);
    assert!(p::own_original_seam_dictionary(r, foreign, Default::default()).is_err());
    let class = p::classify_original_shore_carrier(c, 0, 0, 0, 192).unwrap();
    assert_eq!(class.membership(), p::OriginalShoreMembership::Absent);
    class.reverify_source().unwrap();
}
