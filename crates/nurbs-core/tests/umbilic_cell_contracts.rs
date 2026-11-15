use nurbs_core::{
    editing::exact_surface_derivative::{Refusal, UmbilicStatus, audit_umbilics},
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn joined_planes() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 0.5, 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [0., 1., 2.]
            .map(|x| vec![vec![x, 0., 0.], vec![x, 1., 0.]])
            .to_vec(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}

#[test]
fn classifies_adjacent_cells_with_their_own_uv_traces() {
    let mut surface = joined_planes();
    surface.control_points[2][1][2] = 1.;
    let report = audit_umbilics(&surface, WORK).unwrap();
    let cells = report
        .cells
        .unwrap_or_else(|| panic!("{:?}", report.refusal));
    assert_eq!(cells.len(), 2);
    assert_eq!(cells[0].domain, [[0., 0.5], [0., 1.]]);
    assert_eq!(cells[1].domain, [[0.5, 1.], [0., 1.]]);
    assert_eq!(cells[0].status, UmbilicStatus::EverywhereUmbilic);
    assert_eq!(cells[1].status, UmbilicStatus::EverywhereSeparated);
}

#[test]
fn zero_boundary_gap_prevents_a_strict_whole_cell_proof() {
    // S(u,v) = (3u,v,u³) has zero curvature at u=0 and a nonzero
    // principal curvature for every u>0, with exactly representable poles.
    let mut surface = joined_planes();
    surface.degree_u = 3;
    surface.knots_u = vec![0., 0., 0., 0., 1., 1., 1., 1.];
    surface.control_points = [(0., 0.), (1., 0.), (2., 0.), (3., 1.)]
        .map(|(x, z)| vec![vec![x, 0., z], vec![x, 1., z]])
        .to_vec();
    surface.weights = vec![vec![1.; 2]; 4];
    let cells = audit_umbilics(&surface, WORK).unwrap().cells.unwrap();
    assert_eq!(cells[0].status, UmbilicStatus::ContainsNonUmbilicPoint);
}

#[test]
fn late_regularity_refusal_discards_earlier_cells() {
    let mut surface = joined_planes();
    surface.control_points[2] = surface.control_points[1].clone();
    let audit = audit_umbilics(&surface, WORK).unwrap();
    assert_eq!(audit.refusal, Some(Refusal::UnprovedDenominator));
    assert!(audit.cells.is_none());
}

#[test]
fn work_refusal_is_atomic_and_limits_are_validated() {
    let surface = joined_planes();
    assert!(audit_umbilics(&surface, 0).is_err());
    let audit = audit_umbilics(&surface, 1).unwrap();
    assert_eq!(audit.refusal, Some(Refusal::ResourceLimit));
    assert!(audit.cells.is_none());
}
