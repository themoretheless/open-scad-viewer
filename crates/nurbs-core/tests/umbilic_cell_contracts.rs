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

fn cubic_chart() -> Surface {
    let mut surface = joined_planes();
    surface.degree_u = 3;
    surface.knots_u = vec![2., 2., 2., 2., 6., 6., 6., 6.];
    surface.control_points = [(0., 0.), (1., 0.), (2., 0.), (3., 1.)]
        .map(|(x, z)| vec![vec![x, 0., z], vec![x, 1., z]])
        .to_vec();
    surface.weights = vec![vec![1.; 2]; 4];
    surface
}

#[test]
fn refinement_proves_separation_away_from_a_zero_boundary() {
    use nurbs_core::editing::exact_surface_derivative::refine_umbilics;
    let surface = cubic_chart();
    let base = refine_umbilics(&surface, 0, WORK).unwrap().regions.unwrap();
    assert_eq!(base[0].status, UmbilicStatus::ContainsNonUmbilicPoint);
    let report = refine_umbilics(&surface, 3, WORK).unwrap();
    let regions = report
        .regions
        .unwrap_or_else(|| panic!("{:?}", report.refusal));
    assert_eq!(regions.len(), 5);
    let mut area = 0.;
    for region in regions {
        assert_eq!(region.original_domain, [[2., 6.], [0., 1.]]);
        let [[a, b], [c, d]] = region.unit_domain;
        area += (b - a) * (d - c);
        let expected = if a == 0. {
            UmbilicStatus::ContainsNonUmbilicPoint
        } else {
            UmbilicStatus::EverywhereSeparated
        };
        assert_eq!(region.status, expected);
    }
    assert_eq!(area, 1.);
}

#[test]
fn refinement_retains_original_umbilic_and_separated_cells() {
    use nurbs_core::editing::exact_surface_derivative::refine_umbilics;
    let mut surface = joined_planes();
    surface.control_points[2][1][2] = 1.;
    let cells = refine_umbilics(&surface, 8, WORK).unwrap().regions.unwrap();
    assert_eq!(cells.len(), 2);
    assert_eq!(cells[0].status, UmbilicStatus::EverywhereUmbilic);
    assert_eq!(cells[1].status, UmbilicStatus::EverywhereSeparated);
    assert!(cells.iter().all(|cell| cell.unit_domain == [[0., 1.]; 2]));
}

#[test]
fn reached_ledger_limit_discards_completed_regions() {
    use nurbs_core::editing::exact_surface_derivative::refine_umbilics;
    let surface = cubic_chart();
    let full = refine_umbilics(&surface, 3, WORK).unwrap();
    assert!(full.regions.is_some());
    let short = refine_umbilics(&surface, 3, full.work_used - 1).unwrap();
    assert_eq!(short.refusal, Some(Refusal::ResourceLimit));
    assert!(short.regions.is_none());
    assert!(refine_umbilics(&surface, 9, WORK).is_err());
}
