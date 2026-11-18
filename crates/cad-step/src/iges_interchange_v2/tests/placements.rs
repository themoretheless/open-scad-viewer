use super::*;

fn attach(text: &str, de: usize, matrix: usize) -> String {
    let mut lines: Vec<_> = text.lines().map(str::to_owned).collect();
    let entry = lines
        .iter_mut()
        .find(|line| line.as_bytes()[72] == b'D' && line[73..].trim().parse::<usize>() == Ok(de))
        .unwrap();
    entry.replace_range(48..56, &format!("{matrix:8}"));
    lines.join("\n") + "\n"
}

fn matrix(text: &str, values: [f64; 12]) -> (String, usize) {
    append_entity(text, 124, 0, "00000000", |_| {
        std::iter::once(124.)
            .chain(values)
            .map(|v| v.to_string())
            .collect()
    })
}

#[test]
fn placed_arc_survives_a_composite_rim_and_preserves_the_cylinder() {
    let base = export_iges_v2(&brep_core::cylinder(3., 5.).unwrap())
        .unwrap()
        .0;
    let (entities, _, _) = parse(&base).unwrap();
    let edges = directory_entity(&base, 504, 0);
    let fields = &entities[&edges].values;
    let (at, a, b) = (2..fields.len())
        .step_by(5)
        .find_map(|at| {
            let c = curve126(&entities, u(fields, at, "rim").ok()?, 3, 1.).ok()?;
            let (a, b) = CurveDispatcher::endpoints(&c).ok()?;
            (c.degree == 2 && a[0] * b[1] - a[1] * b[0] > 8.).then_some((at, a, b))
        })
        .unwrap();
    // World (x,y,z) = (-v+4, u-7, w+9).
    let (text, arc) = append_entity(&base, 100, 0, "00000000", |_| {
        [
            100.,
            a[2] - 9.,
            7.,
            4.,
            a[1] + 7.,
            4. - a[0],
            b[1] + 7.,
            4. - b[0],
        ]
        .map(|v| v.to_string())
        .to_vec()
    });
    let (text, tr) = matrix(&text, [0., -1., 0., 4., 1., 0., 0., -7., 0., 0., 1., 9.]);
    let (text, composite) = append_composite(&attach(&text, arc, tr), &[arc]);
    let text = rewire_reference(&text, edges, at, composite);
    let restored = import_iges_v2(&text).unwrap().0;
    assert_eq!(restored.validate().unwrap().boundary_edge_count, 0);
    let curve = &restored.edges[(at - 2) / 5].curve;
    let (start, end) = CurveDispatcher::endpoints(curve).unwrap();
    for (p, expected) in [(start, &a), (end, &b)] {
        assert!(p.iter().zip(expected).all(|(x, y)| (x - y).abs() < 1e-12));
    }
    if let Some(dir) = std::env::var_os("CAD_IGES_PLACEMENT_FIXTURE") {
        let dir = std::path::Path::new(&dir);
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("placed.igs"), text).unwrap();
        std::fs::write(
            dir.join("restored.step"),
            crate::export_step_v9(&restored).unwrap().0,
        )
        .unwrap();
    }
}

#[test]
fn placed_line_and_spline_use_scaled_offsets_and_reject_nonplanar_uv() {
    for spline in [true, false] {
        let (text, line) = append_line(&base_text(), vec![1., 2., 3.], vec![2., 2., 3.], spline);
        let (text, tr) = matrix(&text, [1., 0., 0., 4., 0., 0., -1., 5., 0., 1., 0., 6.]);
        let (entities, _, _) = parse(&attach(&text, line, tr)).unwrap();
        let dispatch = |dim, scale| {
            CurveDispatcher {
                entities: &entities,
                dim,
                scale,
                tolerance_mm: 1e-7,
                work: 0,
            }
            .dispatch(line)
        };
        let world = dispatch(3, 25.4).unwrap();
        assert!(
            world.control_points[0]
                .iter()
                .zip([127., 50.8, 203.2])
                .all(|(a, b)| (a - b).abs() < 1e-12)
        );
        assert!(dispatch(2, 1.).is_err());
    }
}

#[test]
fn refuses_dangling_reflected_scaled_and_chained_placements() {
    let base = composite_base_text();
    let curve = directory_entity(&base, 126, 0);
    assert!(import_iges_v2(&attach(&base, curve, 999999)).is_err());
    let (text, tr) = matrix(&base, [-1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0.]);
    assert!(import_iges_v2(&attach(&text, curve, tr)).is_err());
    let (text, tr) = matrix(&base, [2., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0.]);
    assert!(import_iges_v2(&attach(&text, curve, tr)).is_err());
    assert!(import_iges_v2(&attach(&attach(&text, curve, tr), tr, tr)).is_err());
}

#[test]
fn placed_rational_surface_keeps_weights_charts_and_edge_incidence() {
    let cylinder = brep_core::cylinder(3., 5.).unwrap();
    let base = export_iges_v2(&cylinder).unwrap().0;
    let original = import_iges_v2(&base).unwrap().0;
    let (entities, _, _) = parse(&base).unwrap();
    let (de, mut local) = entities
        .iter()
        .filter(|(_, e)| e.directory.ty == 128)
        .find_map(|(&de, _)| {
            let s = surface128(&entities, de, 1.).unwrap();
            s.weights
                .iter()
                .flatten()
                .any(|w| *w != 1.)
                .then_some((de, s))
        })
        .unwrap();
    // Inverse of the cyclic axis rotation and placement below.
    for p in local.control_points.iter_mut().flatten() {
        *p = vec![p[2] - 2., p[0] - 4., p[1] + 7.];
    }
    let changed = replace_parameters(&base, de, surface_values(&local));
    let (changed, tr) = matrix(&changed, [0., 1., 0., 4., 0., 0., 1., -7., 1., 0., 0., 2.]);
    let text = attach(&changed, de, tr);
    let restored = import_iges_v2(&text).unwrap().0;
    assert_eq!(restored.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(restored.faces.len(), original.faces.len());
    assert_eq!(restored.loops, original.loops);
    for (actual, expected) in restored.faces.iter().zip(&original.faces) {
        assert_eq!(actual.surface.weights, expected.surface.weights);
        assert_eq!(actual.surface.knots_u, expected.surface.knots_u);
        assert_eq!(actual.surface.knots_v, expected.surface.knots_v);
        for (a, b) in actual
            .surface
            .control_points
            .iter()
            .flatten()
            .flatten()
            .zip(expected.surface.control_points.iter().flatten().flatten())
        {
            assert!((a - b).abs() < 1e-12);
        }
    }
    if let Some(dir) = std::env::var_os("CAD_IGES_SURFACE_PLACEMENT_FIXTURE") {
        let dir = std::path::Path::new(&dir);
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("placed.igs"), text).unwrap();
        std::fs::write(
            dir.join("restored.step"),
            crate::export_step_v9(&restored).unwrap().0,
        )
        .unwrap();
    }
}

#[test]
fn legacy_writer_surface_order_remains_readable() {
    let text = include_str!("../../../tests/fixtures/iges/legacy-spinner-rim.igs");
    let restored = import_iges_v2(text).unwrap().0;
    let cylinder = brep_core::cylinder(3., 5.).unwrap();
    assert_eq!(restored.faces.len(), cylinder.faces.len());
    assert_eq!(restored.validate().unwrap().boundary_edge_count, 0);
    for (actual, expected) in restored.faces.iter().zip(&cylinder.faces) {
        assert_eq!(actual.surface, expected.surface);
    }
}
