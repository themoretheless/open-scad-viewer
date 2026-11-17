use num_rational::BigRational as Q;
use nurbs_core::{
    editing::exact_surface_derivative::{Refusal, SurfacePlaneSide, audit_surface_plane_side},
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn q(n: i64) -> Q {
    Q::from_integer(n.into())
}

fn surface(degree: usize, knots: Vec<f64>, heights: &[f64]) -> Surface {
    Surface {
        degree_u: degree,
        degree_v: 1,
        knots_u: knots,
        knots_v: vec![0., 0., 1., 1.],
        control_points: heights
            .iter()
            .enumerate()
            .map(|(i, &z)| vec![vec![i as f64, 0., z], vec![i as f64, 1., z]])
            .collect(),
        weights: (0..heights.len())
            .map(|i| vec![i as f64 + 1., i as f64 + 2.])
            .collect(),
        periodic_u: false,
        periodic_v: false,
    }
}

#[test]
fn certifies_closed_span_sides_at_a_rational_knot_seam() {
    let surface = surface(1, vec![0., 0., 1., 2., 2.], &[1., 0., -1.]);
    let report = audit_surface_plane_side(&surface, [0., 0., 1.], 0., WORK).unwrap();
    let cells = report.cells.unwrap();
    assert_eq!(cells.len(), 2);
    assert_eq!(cells[0].domain, [[0., 1.], [0., 1.]]);
    assert_eq!(cells[0].side, SurfacePlaneSide::Nonnegative);
    assert_eq!(cells[0].residual, [q(0), q(1)]);
    assert_eq!(cells[1].domain, [[1., 2.], [0., 1.]]);
    assert_eq!(cells[1].side, SurfacePlaneSide::Nonpositive);
    assert_eq!(cells[1].residual, [q(-1), q(0)]);
    assert_eq!(&cells[0].corners[2..], &cells[1].corners[..2]);
}

#[test]
fn distinguishes_corner_crossings_from_unresolved_mixed_coefficients() {
    let crossing = surface(1, vec![0., 0., 1., 1.], &[-1., 1.]);
    let crossing = audit_surface_plane_side(&crossing, [0., 0., 1.], 0., WORK)
        .unwrap()
        .cells
        .unwrap();
    assert_eq!(crossing[0].side, SurfacePlaneSide::Straddles);
    let folded = surface(2, vec![0., 0., 0., 1., 1., 1.], &[1., -3., 1.]);
    let folded = audit_surface_plane_side(&folded, [0., 0., 1.], 0., WORK)
        .unwrap()
        .cells
        .unwrap();
    assert_eq!(folded[0].side, SurfacePlaneSide::Unresolved);
    assert_eq!(folded[0].corners, std::array::from_fn(|_| q(1)));
    assert_eq!(folded[0].residual, [q(-3), q(1)]);
}

#[test]
fn proves_coplanarity_for_unclamped_spans_and_shares_the_arithmetic_budget() {
    let flat = surface(2, vec![-2., -1., 0., 1., 2., 3., 4.], &[3.; 4]);
    let report = audit_surface_plane_side(&flat, [0., 0., 2.], 6., WORK).unwrap();
    let cells = report.cells.unwrap();
    assert_eq!(cells.len(), 2);
    assert!(
        cells
            .iter()
            .all(|cell| cell.side == SurfacePlaneSide::Coplanar && cell.residual == [q(0), q(0)])
    );
    let limited = audit_surface_plane_side(&flat, [0., 0., 2.], 6., report.work_used - 1).unwrap();
    assert_eq!(limited.cells, Err(Refusal::ResourceLimit));
    assert!(limited.work_used < report.work_used);
}
