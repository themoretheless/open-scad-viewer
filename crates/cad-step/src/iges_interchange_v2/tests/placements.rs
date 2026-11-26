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
    // First translate by (-7,-4,9), then rotate about Z in the parent frame.
    let (chain, local) = matrix(&text, [1., 0., 0., -7., 0., 1., 0., -4., 0., 0., 1., 9.]);
    let (chain, parent) = matrix(&chain, [0., -1., 0., 0., 1., 0., 0., 0., 0., 0., 1., 0.]);
    let chain = attach(&attach(&chain, arc, local), local, parent);
    let nested = import_iges_v2(&chain).unwrap().0;
    assert_eq!(nested.validate().unwrap().boundary_edge_count, 0);
    assert!(import_iges_v2(&attach(&chain, parent, local)).is_err());
    let restored = import_iges_v2(&text).unwrap().0;
    assert_eq!(restored.validate().unwrap().boundary_edge_count, 0);
    let curve = &restored.edges[(at - 2) / 5].curve;
    assert_eq!(nested.edges[(at - 2) / 5].curve.control_points, curve.control_points);
    // Place the arc in the composite frame, then rotate the composite in the rim frame.
    let grouped = attach(&attach(&chain, local, 0), composite, parent);
    let (grouped, outer) = append_composite(&grouped, &[composite]);
    let grouped_text = rewire_reference(&grouped, edges, at, outer);
    let grouped = import_iges_v2(&grouped_text).unwrap().0;
    assert_eq!(grouped.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(grouped.edges[(at - 2) / 5].curve.control_points, curve.control_points);
    let (start, end) = CurveDispatcher::endpoints(curve).unwrap();
    for (p, expected) in [(start, &a), (end, &b)] {
        assert!(p.iter().zip(expected).all(|(x, y)| (x - y).abs() < 1e-12));
    }
    if let Some(dir) = std::env::var_os("CAD_IGES_PLACEMENT_FIXTURE") {
        let dir = std::path::Path::new(&dir);
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("placed.igs"), text).unwrap();
        std::fs::write(dir.join("nested.igs"), chain).unwrap();
        std::fs::write(dir.join("grouped.igs"), grouped_text).unwrap();
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
fn refuses_dangling_reflected_scaled_and_cyclic_placements() {
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
fn placement_depth_and_translation_sum_are_bounded() {
    let mut text = base_text();
    let mut previous = 0;
    for depth in 1..=33 {
        let (next, tr) = matrix(&text, [1., 0., 0., 1., 0., 1., 0., 0., 0., 0., 1., 0.]);
        text = attach(&next, tr, previous);
        previous = tr;
        let (entities, _, _) = parse(&text).unwrap();
        let result = placement::read(&entities, tr);
        if depth <= 32 {
            assert_eq!(result.unwrap()[0][3], depth as f64);
        } else {
            assert!(result.is_err());
        }
    }
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
    let changed = replace_parameters(&base, de, surface_values(&local).unwrap());
    let (changed, tr) = matrix(&changed, [0., 1., 0., 4., 0., 0., 1., -7., 1., 0., 0., 2.]);
    let (chain, rotation) = matrix(&changed, [0., 1., 0., 0., 0., 0., 1., 0., 1., 0., 0., 0.]);
    let (chain, shift) = matrix(&chain, [1., 0., 0., 4., 0., 1., 0., -7., 0., 0., 1., 2.]);
    let chain = attach(&attach(&chain, de, rotation), rotation, shift);
    let nested = import_iges_v2(&chain).unwrap().0;
    assert_eq!(nested.faces, original.faces);
    assert_eq!(nested.validate().unwrap().boundary_edge_count, 0);
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
        std::fs::write(dir.join("nested.igs"), chain).unwrap();
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

#[test]
fn solid_directory_counts_match_parameter_records() {
    let text = export_iges_v2(&brep_core::cylinder(3., 5.).unwrap()).unwrap().0;
    let directory: Vec<_> = text.lines().filter(|line| &line[72..73] == "D").collect();
    let parameters: Vec<_> = text.lines().filter(|line| &line[72..73] == "P").collect();
    let mut used = 0;
    for pair in directory.chunks_exact(2) {
        let count = pair[1][24..32].trim().parse::<usize>().unwrap();
        assert!(count > 0);
        assert_eq!(pair[0][8..16].trim().parse::<usize>().unwrap(), used + 1);
        for row in &parameters[used..used + count] {
            assert_eq!(row[64..72].trim(), pair[0][73..].trim());
        }
        used += count;
    }
    assert_eq!(used, parameters.len());
    if let Some(path) = std::env::var_os("CAD_IGES_SOLID_FIXTURE") {
        std::fs::write(path, text).unwrap();
    }
}

#[test]
fn rotated_composite_profile_maps_local_z_to_uv_before_projection() {
    let (text, a) = append_line(&base_text(), vec![1., 0., 2.], vec![2., 0., 2.], false);
    let (text, b) = append_line(&text, vec![2., 0., 2.], vec![2., 0., 4.], false);
    let (text, composite) = append_composite(&text, &[a, b]);
    let (text, turn) = matrix(&text, [1., 0., 0., 0., 0., 0., 1., 0., 0., -1., 0., 0.]);
    let read = |source: &str| {
        let (entities, _, _) = parse(source).unwrap();
        CurveDispatcher { entities: &entities, dim: 2, scale: 1., tolerance_mm: 1e-7, work: 0 }
            .dispatch(composite)
    };
    let placed = read(&attach(&text, composite, turn)).unwrap();
    assert_eq!(placed.control_points, [[1., 2.], [2., 2.], [2., 4.]]);
    let (text, shift) = matrix(&text, [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 1.]);
    assert!(read(&attach(&text, composite, shift)).is_err());
}

#[test]
fn standard_curve_flags_preserve_open_polynomials_and_rational_periodicity() {
    let line = Curve::from_polyline(vec![vec![0., 0., 0.], vec![3., 0., 0.]]).unwrap();
    let triangle = Curve {
        degree: 1,
        knots: vec![-1., 0., 1., 2., 3., 4.],
        control_points: vec![vec![0., 0., 0.], vec![2., 0., 0.], vec![0., 2., 0.], vec![0., 0., 0.]],
        weights: vec![1., 0.5, 1., 1.],
        periodic: true,
    };
    let mut text = base_text();
    for (curve, flags) in [(line, ["0", "1", "0"]), (triangle, ["1", "0", "1"])] {
        let values = curve_values(&curve, 3).unwrap();
        assert_eq!(values[4..7], flags);
        let (next, de) = append_entity(&text, 126, 0, "00000000", |_| values);
        text = next;
        let (entities, _, _) = parse(&text).unwrap();
        assert_eq!(curve126(&entities, de, 3, 1.).unwrap(), curve);
        assert_eq!(import_iges_curve(&text, de).unwrap(), curve);
        let previous = text.replace("IGES2.3", "IGES2.1");
        let mut legacy = entities[&de].values.clone();
        legacy[5] = usize::from(curve.periodic).to_string();
        legacy[6] = "0".into();
        let previous = replace_parameters(&previous, de, legacy);
        let (entities, _, _) = parse(&previous).unwrap();
        assert_eq!(curve126(&entities, de, 3, 1.).unwrap(), curve);
    }
    if let Some(path) = std::env::var_os("CAD_IGES_CURVE_FLAGS_FIXTURE") {
        std::fs::write(path, text).unwrap();
    }
}

#[test]
fn ruled_surface_keeps_its_periodic_axis_in_standard_and_legacy_iges() {
    let mut text = base_text();
    for periodic_u in [true, false] {
        let (nu, nv) = if periodic_u { (4, 2) } else { (2, 4) };
        let knots = |periodic| if periodic { vec![-1., 0., 1., 2., 3., 4.] } else { vec![0., 0., 1., 1.] };
        let mut points = Vec::new();
        let mut weights = Vec::new();
        for i in 0..nu {
            let mut row = Vec::new();
            let mut w = Vec::new();
            for j in 0..nv {
                let (around, height) = if periodic_u { (i, j) } else { (j, i) };
                let [x, y] = [[0.,0.], [3.,0.], [0.,2.], [0.,0.]][around];
                row.push(vec![x, y, height as f64]);
                w.push(if periodic_u && around == 1 { 0.5 } else { 1. });
            }
            points.push(row);
            weights.push(w);
        }
        let surface = Surface {
            degree_u: 1, degree_v: 1, knots_u: knots(periodic_u), knots_v: knots(!periodic_u),
            periodic_u, periodic_v: !periodic_u, control_points: points, weights,
        };
        let mut clamped = surface.clone();
        let clamped_knots = vec![0., 0., 1., 2., 3., 3.];
        if periodic_u {
            clamped.periodic_u = false;
            clamped.knots_u = clamped_knots;
        } else {
            clamped.periodic_v = false;
            clamped.knots_v = clamped_knots;
        }
        let flags = surface_values(&clamped).unwrap();
        assert_eq!(flags[5..7], if periodic_u { ["1", "0"] } else { ["0", "1"] });
        assert_eq!(flags[8..10], ["0", "0"]);
        let values = surface_values(&surface).unwrap();
        let expected = if periodic_u { ["1","0","0","1","0"] } else { ["0","1","1","0","1"] };
        assert_eq!(values[5..10], expected);
        let (next, de) = append_entity(&text, 128, 0, "00000000", |_| values.clone());
        text = next;
        let (entities, _, _) = parse(&text).unwrap();
        assert_eq!(surface128(&entities, de, 1.).unwrap(), surface);
        assert_eq!(import_iges_surface(&text, de).unwrap(), surface);
        let mut legacy = values;
        legacy[5..10].clone_from_slice(&["0", "0", expected[3], expected[4], "0"].map(str::to_owned));
        let old = replace_parameters(&text.replace("IGES2.3", "IGES2.2"), de, legacy);
        let (entities, _, _) = parse(&old).unwrap();
        assert_eq!(surface128(&entities, de, 1.).unwrap(), surface);
    }
    if let Some(path) = std::env::var_os("CAD_IGES_SURFACE_FLAGS_FIXTURE") {
        std::fs::write(path, text).unwrap();
    }
}

#[test]
fn iges_edge_uses_only_the_declared_piece_of_a_longer_curve() {
    let text = composite_base_text();
    let original = import_iges_v2(&text).unwrap().0;
    let edge_list = directory_entity(&text, 504, 0);
    let edge = &original.edges[0].curve;
    let (a, b) = CurveDispatcher::endpoints(edge).unwrap();
    let extend = |t: f64| a.iter().zip(&b).map(|(x, y)| x + t * (y - x)).collect();
    let mut basis = Curve::from_polyline(vec![extend(-1.), extend(2.)]).unwrap();
    let [lo, hi] = edge.domain();
    basis.knots = vec![lo - (hi-lo), lo - (hi-lo), hi + (hi-lo), hi + (hi-lo)];
    let mut values = curve_values(&basis, 3).unwrap();
    let at = values.len() - 5;
    values[at] = number(lo);
    values[at + 1] = number(hi);
    let (text, de) = append_entity(&text, 126, 0, "00000000", |_| values.clone());
    let text = rewire_reference(&text, edge_list, 2, de);
    let restored = import_iges_v2(&text).unwrap().0;
    assert_eq!(restored.validate().unwrap().boundary_edge_count, 0);
    let trimmed = &restored.edges[0].curve;
    assert_eq!(trimmed.domain(), edge.domain());
    for t in [lo, (lo + hi) / 2., hi] {
        assert!(CurveDispatcher::distance(&trimmed.evaluate(t).unwrap().point,
            &edge.evaluate(t).unwrap().point).unwrap() < 1e-12);
    }
    values[at] = number(basis.domain()[0] - 1.);
    let invalid = replace_parameters(&text, de, values);
    assert!(import_iges_v2(&invalid).is_err());
}

#[test]
fn declared_iges_subinterval_preserves_weighted_multispan_geometry() {
    let basis = Curve {
        degree: 2,
        control_points: vec![vec![0.,0.,1.], vec![1.,3.,1.], vec![2.,1.,2.], vec![4.,0.,3.]],
        knots: vec![0.,0.,0.,1.,2.,2.,2.],
        weights: vec![1.,0.5,1.5,1.],
        periodic: false,
    };
    let mut values = curve_values(&basis, 3).unwrap();
    let at = values.len() - 5;
    values[at] = number(0.25);
    values[at + 1] = number(1.5);
    let (text, de) = append_entity(&base_text(), 126, 0, "00000000", |_| values);
    let (entities, _, _) = parse(&text).unwrap();
    let segment = curve126(&entities, de, 3, 1.).unwrap();
    assert_eq!(segment.domain(), [0.25, 1.5]);
    assert_eq!(import_iges_curve(&text, de).unwrap(), segment);
    for t in [0.25, 0.5, 1., 1.25, 1.5] {
        assert!(CurveDispatcher::distance(&basis.evaluate(t).unwrap().point,
            &segment.evaluate(t).unwrap().point).unwrap() < 1e-12);
    }
    if let Some(dir) = std::env::var_os("CAD_IGES_CURVE_INTERVAL_FIXTURE") {
        let dir = std::path::Path::new(&dir);
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("source.igs"), text).unwrap();
        let (exported, _) = append_entity(&base_text(), 126, 0, "00000000", |_| curve_values(&segment, 3).unwrap());
        std::fs::write(dir.join("trimmed.igs"), exported).unwrap();
    }
}

#[test]
fn declared_surface_rectangle_preserves_geometry_and_uncut_periodicity() {
    let base = Surface {
        degree_u: 1, degree_v: 1,
        knots_u: vec![-1., 0., 1., 2., 3., 4.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [[0., 0.], [2., 0.], [0., 3.], [0., 0.]].map(|[x,y]| {
            vec![vec![x,y,0.], vec![x,y,4.]]
        }).to_vec(),
        weights: vec![vec![1.,1.],vec![0.5,0.5],vec![1.,1.],vec![1.,1.]],
        periodic_u: true, periodic_v: false,
    };
    for range in [[0.,3.,0.25,0.75], [0.5,2.25,0.25,0.75], [-1.,2.,0.,1.]] {
        let mut values = surface_values(&base).unwrap();
        let at = values.len() - 4;
        values[at..].clone_from_slice(&range.map(number));
        let (text, de) = append_entity(&base_text(), 128, 0, "00000000", |_| values);
        let (entities, _, _) = parse(&text).unwrap();
        let patch = surface128(&entities, de, 1.);
        if range[0] < 0. {
            assert!(patch.is_err());
            continue;
        }
        let patch = patch.unwrap();
        assert_eq!(import_iges_surface(&text, de).unwrap(), patch);
        assert_eq!(patch.periodic_u, range[0] == 0.);
        assert_eq!(patch.knots_u[patch.degree_u], range[0]);
        assert_eq!(patch.knots_u[patch.control_points.len()], range[1]);
        assert_eq!(patch.knots_v[patch.degree_v], range[2]);
        assert_eq!(patch.knots_v[patch.control_points[0].len()], range[3]);
        for u in [range[0], 1., 1.5, 2., range[1]] {
            for v in [range[2], 0.5, range[3]] {
                assert!(CurveDispatcher::distance(&patch.evaluate(u,v).unwrap().point,
                    &base.evaluate(u,v).unwrap().point).unwrap() < 1e-12);
            }
        }
        if let Some(dir) = std::env::var_os("CAD_IGES_PATCH_RECTANGLE_FIXTURE") {
            let dir = std::path::Path::new(&dir);
            std::fs::create_dir_all(dir).unwrap();
            let file = if patch.periodic_u { "band" } else { "patch" };
            std::fs::write(dir.join(format!("{file}-source.igs")), text).unwrap();
            let (out, _) = append_entity(&base_text(), 128, 0, "00000000", |_| surface_values(&patch).unwrap());
            std::fs::write(dir.join(format!("{file}-trimmed.igs")), out).unwrap();
        }
    }
}

#[test]
fn restricted_support_surface_retains_the_closed_cuboid_face() {
    let text = composite_base_text();
    let (entities, _, _) = parse(&text).unwrap();
    let face = directory_entity(&text, 510, 0);
    let de = u(&entities[&face].values, 1, "face surface").unwrap();
    let original = surface128(&entities, de, 1.).unwrap();
    let mut extended = original.clone();
    assert_eq!((extended.degree_u, extended.degree_v), (1, 1));
    for row in 0..2 {
        for column in 0..2 {
            let a = original.control_points[row][column].clone();
            let b = &original.control_points[1-row][column];
            extended.control_points[row][column] = a.iter().zip(b).map(|(x,y)| 2.*x-y).collect();
        }
    }
    let (lo, hi) = (original.knots_u[1], original.knots_u[2]);
    extended.knots_u = vec![2.*lo-hi, 2.*lo-hi, 2.*hi-lo, 2.*hi-lo];
    let mut values = surface_values(&extended).unwrap();
    let at = values.len()-4;
    values[at] = number(lo);
    values[at+1] = number(hi);
    let (text, support) = append_entity(&text, 128, 0, "00000000", |_| values);
    let text = rewire_reference(&text, face, 1, support);
    let model = import_iges_v2(&text).unwrap().0;
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(model.faces.len(), 6);
    let (entities, _, _) = parse(&text).unwrap();
    let patch = surface128(&entities, support, 1.).unwrap();
    for (a,b) in patch.control_points.iter().flatten().zip(original.control_points.iter().flatten()) {
        assert!(CurveDispatcher::distance(a,b).unwrap() < 1e-12);
    }
}

#[test]
fn spatial_trim_recovers_tilted_chart_clock_and_rejects_off_plane_trace() {
    let chart = Surface {
        degree_u: 1, degree_v: 1, knots_u: vec![-2.,-2.,2.,2.], knots_v: vec![3.,3.,5.,5.],
        control_points: vec![vec![vec![1.,2.,3.], vec![1.,6.,7.]], vec![vec![1.,2.,11.], vec![1.,6.,15.]]],
        weights: vec![vec![1.;2];2], periodic_u: false, periodic_v: false,
    };
    let uv = Curve { degree: 2, knots: vec![4.,4.,4.,8.,8.,8.],
        control_points: vec![vec![-1.,3.5], vec![0.,4.5], vec![1.,4.]],
        weights: vec![1.,0.5,1.], periodic: false };
    let mut world = uv.clone();
    for p in &mut world.control_points {
        *p = chart.evaluate(p[0], p[1]).unwrap().point.to_vec();
    }
    assert_eq!(affine_boundary::recover(1, &chart, &world, &mut 0).unwrap(), uv);
    world.control_points[1][0] += 1e-10;
    assert!(affine_boundary::recover(1, &chart, &world, &mut 0).is_err());
    world.control_points[1][0] = 1.;
    world.control_points[0][1] = -10.;
    assert!(affine_boundary::recover(1, &chart, &world, &mut 0).is_err());
}
