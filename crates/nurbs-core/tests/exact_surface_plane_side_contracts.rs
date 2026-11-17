use num_rational::BigRational as Q;
use nurbs_core::{
    editing::exact_surface_derivative::{
        PlaneGraphPosition, Refusal, SurfacePlaneGraphStatus, SurfacePlaneSide,
        audit_surface_plane_graphs, audit_surface_plane_side, refine_surface_plane_side,
    },
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

#[test]
fn proves_a_mixed_control_net_is_strictly_separated_after_subdivision() {
    let folded = surface(2, vec![0., 0., 0., 1., 1., 1.], &[1., -0.125, 1.]);
    let audit = audit_surface_plane_side(&folded, [0., 0., 1.], 0., WORK)
        .unwrap()
        .cells
        .unwrap();
    assert_eq!(audit[0].side, SurfacePlaneSide::Unresolved);
    let refined = refine_surface_plane_side(&folded, [0., 0., 1.], 0., 8, WORK)
        .unwrap()
        .regions
        .unwrap();
    assert_eq!(refined.len(), 2);
    assert_eq!(refined[0].cell.domain, [[0., 0.5], [0., 1.]]);
    assert_eq!(refined[1].cell.domain, [[0.5, 1.], [0., 1.]]);
    assert!(
        refined
            .iter()
            .all(|region| region.cell.side == SurfacePlaneSide::Positive)
    );
    assert_eq!(&refined[0].cell.corners[2..], &refined[1].cell.corners[..2]);
}

#[test]
fn localizes_a_folded_crossing_and_keeps_the_whole_original_domain_covered() {
    let folded = surface(2, vec![0., 0., 0., 1., 1., 1.], &[1., -3., 1.]);
    let report = refine_surface_plane_side(&folded, [0., 0., 1.], 0., 8, WORK).unwrap();
    let regions = report.regions.unwrap();
    let area: f64 = regions
        .iter()
        .map(|region| {
            region
                .cell
                .domain
                .iter()
                .map(|[a, b]| b - a)
                .product::<f64>()
        })
        .sum();
    assert_eq!(area, 1.);
    assert!(
        regions
            .iter()
            .any(|region| region.cell.side == SurfacePlaneSide::Positive)
    );
    assert!(
        regions
            .iter()
            .any(|region| region.cell.side == SurfacePlaneSide::Negative)
    );
    assert!(
        regions
            .iter()
            .any(|region| region.cell.side == SurfacePlaneSide::Straddles)
    );
    let limited =
        refine_surface_plane_side(&folded, [0., 0., 1.], 0., 8, report.work_used - 1).unwrap();
    assert_eq!(limited.regions, Err(Refusal::ResourceLimit));
}

#[test]
fn certifies_unique_intersection_graphs_in_the_interior_and_on_an_edge() {
    use nurbs_core::editing::exact_projective::UvAxis;
    let crossing = surface(1, vec![0., 0., 1., 1.], &[-1., 1.]);
    let report = audit_surface_plane_graphs(&crossing, [0., 0., 1.], 0., WORK).unwrap();
    let cells = report.cells.unwrap();
    let SurfacePlaneGraphStatus::Graphs(graphs) = &cells[0].status else {
        panic!("{:?}", cells[0].status);
    };
    let graph = graphs[0].as_ref().unwrap();
    assert_eq!(graph.solve_axis, UvAxis::U);
    assert_eq!(graph.position, PlaneGraphPosition::Interior);
    assert_eq!(graph.derivative, [q(3), q(5)]);
    assert!(graphs[1].is_none());

    let edge = surface(1, vec![-2., -2., 4., 4.], &[0., 1.]);
    let report = audit_surface_plane_graphs(&edge, [0., 0., -1.], 0., WORK).unwrap();
    let cells = report.cells.unwrap();
    let SurfacePlaneGraphStatus::Graphs(graphs) = &cells[0].status else {
        panic!("{:?}", cells[0].status);
    };
    assert_eq!(
        graphs[0].as_ref().unwrap().position,
        PlaneGraphPosition::LowerEdge
    );
    assert_eq!(graphs[0].as_ref().unwrap().derivative, [q(-3), q(-2)]);
}

#[test]
fn handles_a_diagonal_across_closed_edges_and_refuses_to_certify_a_fold() {
    let mut diagonal = surface(1, vec![0., 0., 1., 1.], &[-1., 0.]);
    diagonal.weights = vec![vec![1.; 2]; 2];
    diagonal.control_points[0][1][2] = 0.;
    diagonal.control_points[1][1][2] = 1.;
    let report = audit_surface_plane_graphs(&diagonal, [0., 0., 1.], 0., WORK).unwrap();
    let cells = report.cells.unwrap();
    let SurfacePlaneGraphStatus::Graphs(graphs) = &cells[0].status else {
        panic!("{:?}", cells[0].status);
    };
    assert!(graphs.iter().all(|graph| {
        graph
            .as_ref()
            .is_some_and(|g| g.position == PlaneGraphPosition::Closed)
    }));
    let limited =
        audit_surface_plane_graphs(&diagonal, [0., 0., 1.], 0., report.work_used - 1).unwrap();
    assert_eq!(limited.cells, Err(Refusal::ResourceLimit));

    let folded = surface(2, vec![0., 0., 0., 1., 1., 1.], &[1., -3., 1.]);
    let report = audit_surface_plane_graphs(&folded, [0., 0., 1.], 0., WORK).unwrap();
    assert_eq!(
        report.cells.unwrap()[0].status,
        SurfacePlaneGraphStatus::Unresolved
    );
}
