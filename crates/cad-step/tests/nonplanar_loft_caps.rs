use brep_core::{LoftCap, capped_loft_with_caps};
use nurbs_core::{
    curve::Curve,
    surface::{Axis, Surface},
};

fn cap(z: f64) -> LoftCap {
    let surface = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [(0., 0.), (0.5, 0.25), (1., 0.)]
            .into_iter()
            .map(|(x, h)| vec![vec![x, 0., z + h], vec![x, 1., z + h]])
            .collect(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let p = [[0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]];
    let trims = vec![
        p.windows(2)
            .map(|p| {
                nurbs_core::paths::bezier(p.iter().map(|p| p.to_vec()).collect(), None).unwrap()
            })
            .collect(),
    ];
    LoftCap { surface, trims }
}
fn edges(s: &Surface) -> Vec<Vec<Curve>> {
    vec![vec![
        s.iso(Axis::V, 0.).unwrap(),
        s.iso(Axis::U, 1.).unwrap(),
        s.iso(Axis::V, 1.).unwrap().reverse().unwrap(),
        s.iso(Axis::U, 0.).unwrap().reverse().unwrap(),
    ]]
}
fn fixture() -> (
    LoftCap,
    LoftCap,
    Vec<Vec<Curve>>,
    Vec<Vec<Curve>>,
    Vec<Vec<Surface>>,
) {
    let (a, b) = (cap(0.), cap(1.));
    let (start, end) = (edges(&a.surface), edges(&b.surface));
    let sides = vec![
        start[0]
            .iter()
            .zip(&end[0])
            .map(|(a, b)| nurbs_core::surface::loft(&[a.clone(), b.clone()]).unwrap())
            .collect(),
    ];
    (a, b, start, end, sides)
}
fn limits() -> brep_core::boundary_embedding::Limits {
    brep_core::boundary_embedding::Limits {
        exact_work: 1_000_000,
        trim_pairs: 1000,
        trim_cells: 10000,
        trim_domain_cells: 100000,
        spans: 1000,
        contacts: brep_core::face_contacts::Limits {
            pairs: 100,
            cells: 100000,
            domain_cells: 100000,
            cells_per_pair: 1000,
            domain_cells_per_pair: 1000,
        },
    }
}
#[test]
fn nonplanar_caps_retain_world_boundaries_and_step_carriers() {
    let (a, b, start, end, sides) = fixture();
    assert!(brep_core::capped_loft_surfaces(&start, &end, &sides).is_err());
    let model = capped_loft_with_caps(&start, &end, &sides, [&a, &b]).unwrap();
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(model.faces.len(), 6);
    for (index, z) in [(4, 0.), (5, 1.)] {
        for u in [0., 0.13, 0.5, 0.87, 1.] {
            let p = model.faces[index].surface.evaluate(u, 0.37).unwrap().point;
            assert!((p[0] - u).abs() < 1e-12);
            assert!((p[2] - (z + 0.5 * u * (1. - u))).abs() < 1e-12);
        }
    }
    let (step, _, _) = cad_step::export_step_v5(&model).unwrap();
    assert!(step.contains("B_SPLINE_SURFACE_WITH_KNOTS"));
    let (restored, _, _) = cad_step::import_step_v5(&step).unwrap();
    assert_eq!(restored.validate().unwrap().boundary_edge_count, 0);
    for (index, z) in [(4, 0.), (5, 1.)] {
        let p = restored.faces[index]
            .surface
            .evaluate(0.5, 0.37)
            .unwrap()
            .point;
        assert!((p[2] - (z + 0.125)).abs() < 1e-12);
    }
    let mut broken = a.clone();
    // The endpoints still match; whole-edge agreement must catch its interior.
    broken.surface.control_points[1][0][2] += 0.125;
    assert!(capped_loft_with_caps(&start, &end, &sides, [&broken, &b]).is_err());
}
#[test]
fn whole_loft_embedding_checks_faces_and_all_pairs_and_preserves_budget_refusal() {
    let (a, b, start, end, sides) = fixture();
    let model = capped_loft_with_caps(&start, &end, &sides, [&a, &b]).unwrap();
    let report = brep_core::boundary_embedding::inspect(&model, 1e-9, limits()).unwrap();
    assert!(
        brep_core::capped_loft_with_caps_checked(&start, &end, &sides, [&a, &b], 1e-9, limits())
            .is_ok()
    );
    assert!(
        report.proven,
        "agreement={} trims={} injective={} pairs={}",
        report.agreement.all_equal,
        report.trim.all_valid,
        report.intersections.faces.all_faces_injective,
        report.intersections.pairs.all_pairs_classified
    );
    let mut small = limits();
    small.contacts.pairs = 1;
    assert!(
        !brep_core::boundary_embedding::inspect(&model, 1e-9, small)
            .unwrap()
            .proven
    );
}
#[test]
fn globally_crossing_side_with_retained_seams_cannot_pass_embedding() {
    let (a, b, start, end, mut sides) = fixture();
    // Front wall is an injective graph over X/Z; its interior penetrates the
    // opposite wall although every endpoint and cap seam remains unchanged.
    sides[0][0] = sides[0][0].edit_axis(Axis::V, |c| c.elevate(2)).unwrap();
    sides[0][0].control_points[1][1][1] = 5.;
    let model = capped_loft_with_caps(&start, &end, &sides, [&a, &b]).unwrap();
    let v = (1. - 0.2_f64.sqrt()) / 2.;
    let front = model.faces[0].surface.evaluate(0.5, v).unwrap().point;
    let back = model.faces[2].surface.evaluate(0.5, v).unwrap().point;
    for k in 0..3 {
        assert!((front[k] - back[k]).abs() < 1e-12);
    }
    let report = brep_core::boundary_embedding::inspect(&model, 1e-9, limits()).unwrap();
    assert!(!report.proven);
    assert!(
        brep_core::capped_loft_with_caps_checked(&start, &end, &sides, [&a, &b], 1e-9, limits())
            .is_err()
    );
    assert!(!report.intersections.pairs.all_pairs_classified);
}
