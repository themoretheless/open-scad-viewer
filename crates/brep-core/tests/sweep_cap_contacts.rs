use brep_core::{
    rational_section_loft,
    sweep_cap_contacts::{self, Budgets},
};
use nurbs_core::primitives::circle;
fn budgets() -> Budgets {
    Budgets {
        max_walls: 1024,
        max_exact_work: 1000000,
        max_chart_cells: 1000,
        max_trim_pairs: 100000,
        max_trim_cells: 100000,
        max_trim_domain_cells: 1000000,
    }
}
#[test]
fn actual_hollow_loft_contacts_are_certified_only_with_all_prerequisites() {
    let loops = |z| {
        vec![
            vec![circle([0., 0., z], [0., 0., 1.], 0.5).unwrap()],
            vec![
                circle([0., 0., z], [0., 0., 1.], 0.2)
                    .unwrap()
                    .reverse()
                    .unwrap(),
            ],
        ]
    };
    let model = rational_section_loft(&[loops(0.), loops(5.), loops(10.)]).unwrap();
    let before = model.clone();
    let caps = [16, 17];
    for cap in caps {
        let report = sweep_cap_contacts::inspect(&model, cap, &caps, budgets()).unwrap();
        assert!(
            report.cap_certified && report.all_cap_wall_contacts_certified,
            "{report:?}"
        );
        assert_eq!(report.separated_walls.len(), 8);
        assert_eq!(report.allowed_boundaries.len(), 8);
        assert!(report.unresolved_walls.is_empty());
        assert!(report.exact_work < 1000000);
        let mut b = budgets();
        b.max_walls = 2;
        let partial = sweep_cap_contacts::inspect(&model, cap, &caps, b).unwrap();
        assert!(partial.cap_certified);
        assert!(!partial.all_cap_wall_contacts_certified);
        assert_eq!(partial.unresolved_walls, (2..16).collect::<Vec<_>>());
        assert_eq!(partial.reason, Some("cap-wall-budget-exhausted"));
        b = budgets();
        b.max_exact_work = 0;
        let zero = sweep_cap_contacts::inspect(&model, cap, &caps, b).unwrap();
        assert!(!zero.cap_certified && !zero.all_cap_wall_contacts_certified);
        assert_eq!(zero.unresolved_walls.len(), 16);
    }
    let mut changed = model.clone();
    let wire = changed.faces[16].outer;
    changed.loops[wire].coedges[0].pcurve.control_points[1][0] =
        changed.loops[wire].coedges[0].pcurve.control_points[1][0].next_down();
    changed.validate().unwrap();
    let wrong = sweep_cap_contacts::inspect(&changed, 16, &caps, budgets()).unwrap();
    assert!(!wrong.cap_certified && !wrong.all_cap_wall_contacts_certified);
    assert!(sweep_cap_contacts::inspect(&model, 16, &[16, 16], budgets()).is_err());
    let mut b = budgets();
    b.max_chart_cells = 0;
    assert!(
        !sweep_cap_contacts::inspect(&model, 16, &caps, b)
            .unwrap()
            .cap_certified
    );
    assert_eq!(model, before);
}
