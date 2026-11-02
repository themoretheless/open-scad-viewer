use brep_core::profile_certificate::original_restriction::{
    self as r,
    wire::{self, jordan, prism},
};
use nurbs_core::curve::Curve;
fn q(n: i64, d: i64) -> [String; 2] {
    [n.to_string(), d.to_string()]
}
fn polygon(p: &[[f64; 2]]) -> wire::Wire {
    let c = Curve::from_polyline(p.iter().map(|p| p.to_vec()).collect()).unwrap();
    wire::sew(
        vec![r::restrict_binary64(&c, c.domain()).unwrap()],
        vec![false],
        q(0, 1),
    )
    .unwrap()
}
#[test]
fn complete_census_keeps_collinear_subdivision_and_disjoint_hole_loops() {
    let outer = polygon(&[[0., 0.], [2., 0.], [4., 0.], [4., 4.], [0., 4.], [0., 0.]]);
    let hole = polygon(&[[1., 1.], [1., 2.], [2., 2.], [2., 1.], [1., 1.]]);
    let shell = prism::extrude_binary64(&[outer, hole], [0., 2.], 1e-7).unwrap();
    let proof = shell
        .qualify_profile_jordan(jordan::Limits::default())
        .unwrap();
    assert_eq!(proof.pairs().len(), 45);
    proof.reverify().unwrap();
    assert!(shell.model().bodies.is_empty());
    assert!(
        shell
            .qualify_profile_jordan(jordan::Limits { max_pairs: 44 })
            .is_err()
    );
}
#[test]
fn closed_incidence_and_exact_boundary_agreement_do_not_admit_a_bowtie() {
    let bowtie = polygon(&[[0., 0.], [2., 2.], [0., 2.], [2., 0.], [0., 0.]]);
    let shell = prism::extrude_binary64(&[bowtie], [0., 2.], 1e-7).unwrap();
    shell.reverify().unwrap();
    assert!(
        shell
            .qualify_profile_jordan(jordan::Limits::default())
            .is_err()
    );
}
#[test]
fn touching_and_overlapping_loops_refuse_jordan_separation() {
    let a = polygon(&[[0., 0.], [2., 0.], [2., 2.], [0., 2.], [0., 0.]]);
    for b in [
        polygon(&[[2., 2.], [3., 2.], [3., 3.], [2., 3.], [2., 2.]]),
        polygon(&[[1., 0.], [3., 0.], [3., 1.], [1., 1.], [1., 0.]]),
    ] {
        assert!(jordan::qualify(&[a.clone(), b], jordan::Limits::default()).is_err());
    }
}
#[test]
fn rational_circle_including_closing_contact_gets_complete_pair_coverage() {
    let mut c =
        nurbs_core::primitives::circle_arc([0., 0., 0.], [0., 0., 1.], 2., 0., 360.).unwrap();
    for p in &mut c.control_points {
        p.truncate(2);
    }
    let wire = wire::sew(
        vec![r::restrict_binary64(&c, c.domain()).unwrap()],
        vec![true],
        q(1, 1_000_000),
    )
    .unwrap();
    let n = wire.pieces()[0].len();
    let proof = jordan::qualify(&[wire], jordan::Limits::default()).unwrap();
    assert_eq!(proof.pairs().len(), n * (n + 1) / 2);
    proof.reverify().unwrap();
}

#[test]
fn interior_tangency_is_not_missed_by_an_endpoint_only_census() {
    let arc = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![-1., 1.], vec![0., -1.], vec![1., 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let line = Curve::from_polyline(vec![vec![1., 1.], vec![-1., 1.]]).unwrap();
    let lens = wire::sew(
        [arc, line]
            .iter()
            .map(|c| r::restrict_binary64(c, c.domain()).unwrap())
            .collect(),
        vec![false, false],
        q(0, 1),
    )
    .unwrap();
    let rectangle = polygon(&[[-0.5, -1.], [0.5, -1.], [0.5, 0.], [-0.5, 0.], [-0.5, -1.]]);
    let error = jordan::qualify(&[lens, rectangle], jordan::Limits::default()).unwrap_err();
    assert!(error.to_string().contains("interior intersection"));
}

#[test]
fn self_intersection_inside_one_regular_cubic_span_refuses() {
    let c = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: vec![
            vec![3., -1.5],
            vec![-1., 3.5],
            vec![-1., -3.5],
            vec![3., 1.5],
        ],
        weights: vec![1.; 4],
        periodic: false,
    };
    let line = Curve::from_polyline(vec![vec![3., 1.5], vec![3., -1.5]]).unwrap();
    let wire = wire::sew(
        [c, line]
            .iter()
            .map(|c| r::restrict_binary64(c, c.domain()).unwrap())
            .collect(),
        vec![false, false],
        q(0, 1),
    )
    .unwrap();
    let error = jordan::qualify(&[wire], jordan::Limits::default()).unwrap_err();
    assert!(error.to_string().contains("interior intersection"));
}
