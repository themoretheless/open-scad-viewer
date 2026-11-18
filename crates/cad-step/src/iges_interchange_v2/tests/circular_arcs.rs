use super::*;

#[test]
fn type_100_arc_imports_directly_and_in_a_composite_cylinder_rim() {
    let base = export_iges_v2(&brep_core::cylinder(3., 5.).unwrap())
        .unwrap()
        .0;
    let (entities, _, _) = parse(&base).unwrap();
    let edge_list = directory_entity(&base, 504, 0);
    let values = &entities[&edge_list].values;
    let (at, start, end) = (2..values.len())
        .step_by(5)
        .find_map(|at| {
            let de = u(values, at, "edge curve").unwrap();
            let c = curve126(&entities, de, 3, 1.).unwrap();
            let (start, end) = CurveDispatcher::endpoints(&c).unwrap();
            (c.degree == 2 && start[0] * end[1] - start[1] * end[0] > 8.)
                .then_some((at, start, end))
        })
        .unwrap();
    let (text, arc) = append_entity(&base, 100, 0, "00000000", |_| {
        [100., start[2], 0., 0., start[0], start[1], end[0], end[1]]
            .into_iter()
            .map(|v| v.to_string())
            .collect()
    });
    let (arc_entities, _, _) = parse(&text).unwrap();
    let scaled = circular_arc::read(&arc_entities[&arc], 3, 25.4, 1e-8).unwrap();
    let p = scaled.evaluate(0.5).unwrap().point;
    assert!((p[0].hypot(p[1]) - 76.2).abs() < 1e-10);
    let (text, composite) = append_composite(&text, &[arc]);
    for reference in [arc, composite] {
        let text = rewire_reference(&text, edge_list, at, reference);
        let model = import_iges_v2(&text).unwrap().0;
        assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
        let c = &model.edges[(at - 2) / 5].curve;
        assert_eq!(c.degree, 2);
        let [lo, hi] = c.domain();
        for i in 0..=24 {
            let p = c.evaluate(lo + (hi - lo) * i as f64 / 24.).unwrap().point;
            assert!((p[0].hypot(p[1]) - 3.).abs() < 1e-12);
            assert!((p[2] - start[2]).abs() < 1e-12);
            assert!(start[0] * p[1] - start[1] * p[0] >= -1e-12);
        }
        if reference == composite
            && let Some(dir) = std::env::var_os("CAD_IGES_ARC_FIXTURE")
        {
            let dir = std::path::Path::new(&dir);
            std::fs::create_dir_all(dir).unwrap();
            std::fs::write(dir.join("rim.igs"), text).unwrap();
            std::fs::write(
                dir.join("restored.step"),
                crate::export_step_v9(&model).unwrap().0,
            )
            .unwrap();
        }
    }
}

#[test]
fn type_100_distinguishes_full_circles_major_arcs_and_invalid_radii() {
    let check = |end: [f64; 2]| {
        let (text, arc) = append_entity(&base_text(), 100, 0, "00000000", |_| {
            [100., 0., 0., 0., 1., 0., end[0], end[1]]
                .into_iter()
                .map(|v| v.to_string())
                .collect()
        });
        let (entities, _, _) = parse(&text).unwrap();
        circular_arc::read(&entities[&arc], 2, 1., 1e-9)
    };
    let circle = check([1., 0.]).unwrap();
    assert_eq!(circle.control_points.len(), 9);
    assert_eq!(circle.control_points.first(), circle.control_points.last());
    let major = check([0., -1.]).unwrap();
    assert_eq!(major.control_points.len(), 7);
    assert!(major.evaluate(0.5).unwrap().point[0] < 0.);
    assert!(check([0., 2.]).is_err());
    assert!(check([0., 0.]).is_err());
}
