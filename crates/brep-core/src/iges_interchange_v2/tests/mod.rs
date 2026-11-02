use super::*;
use crate::nurbs_step_solid::freeform_cuboid_solid;

fn append(target: &mut Model, source: Model) {
    let Model(source, _) = source;
    let (vo, eo, lo, fo, so) = (
        target.vertices.len(),
        target.edges.len(),
        target.loops.len(),
        target.faces.len(),
        target.shells.len(),
    );
    target.0.vertices.extend(source.vertices);
    target.0.edges.extend(source.edges.into_iter().map(|mut e| {
        e.vertices = [e.vertices[0] + vo, e.vertices[1] + vo];
        e
    }));
    target.0.loops.extend(source.loops.into_iter().map(|mut l| {
        for c in &mut l.coedges {
            c.edge += eo
        }
        l
    }));
    target.0.faces.extend(source.faces.into_iter().map(|mut f| {
        f.outer += lo;
        for h in &mut f.holes {
            *h += lo
        }
        f
    }));
    target
        .0
        .shells
        .extend(source.shells.into_iter().map(|mut s| {
            for f in &mut s.faces {
                f.face += fo
            }
            s
        }));
    target
        .0
        .bodies
        .extend(source.bodies.into_iter().map(|mut b| {
            b.outer_shell += so;
            for s in &mut b.inner_shells {
                *s += so
            }
            b
        }));
}

fn base_text() -> String {
    let model = freeform_cuboid_solid([0., 0., 0.], [2., 3., 4.]).unwrap();
    export_iges_v2(&model).unwrap().0
}

fn composite_base_text() -> String {
    let model = crate::cuboid([0., 0., 0.], [2., 3., 4.]).unwrap();
    export_iges_v2(&model).unwrap().0
}

fn without_identity(mut text: String) -> String {
    let (entities, _, _) = parse(&text).unwrap();
    for (&de, entity) in &entities {
        if entity.directory.ty == 406 && entity.directory.form == 15 {
            text = set_status(&text, de, "01000000");
        }
    }
    text
}

fn trimmed_outer_fixture() -> (String, usize, usize) {
    let text = composite_base_text();
    let (entities, _, _) = parse(&text).unwrap();
    let face = directory_entity(&text, 510, 0);
    let surface = u(&entities[&face].values, 1, "face surface").unwrap();
    let loop_de = u(&entities[&face].values, 4, "face loop").unwrap();
    let n = u(&entities[&loop_de].values, 1, "loop count").unwrap();
    let mut boundary = vec![
        "143".into(),
        "0".into(),
        "2".into(),
        surface.to_string(),
        n.to_string(),
    ];
    for i in 0..n {
        let at = 2 + 7 * i;
        let edge_list = u(&entities[&loop_de].values, at + 1, "loop edge list").unwrap();
        let edge_index = u(&entities[&loop_de].values, at + 2, "loop edge").unwrap();
        let orientation = u(&entities[&loop_de].values, at + 3, "loop orientation").unwrap();
        let pcurve = u(&entities[&loop_de].values, at + 6, "loop pcurve").unwrap();
        let curve = u(
            &entities[&edge_list].values,
            2 + 5 * (edge_index - 1),
            "edge curve",
        )
        .unwrap();
        boundary.extend([
            curve.to_string(),
            pcurve.to_string(),
            if orientation == 1 { "1" } else { "2" }.into(),
        ]);
    }
    let (text, outer) = append_entity(&text, 143, 0, "00010000", |_| boundary);
    let (text, trimmed) = append_entity(&text, 144, 0, "00000000", |_| {
        vec![
            "144".into(),
            surface.to_string(),
            "1".into(),
            "0".into(),
            outer.to_string(),
        ]
    });
    let (entities, _, _) = parse(&text).unwrap();
    let (shell, shell_at) = entities
        .iter()
        .find_map(|(&de, entity)| {
            if entity.directory.ty != 514 {
                return None;
            }
            (0..u(&entity.values, 1, "shell face count").unwrap())
                .map(|i| 2 + 2 * i)
                .find(|&at| u(&entity.values, at, "shell face").unwrap() == face)
                .map(|at| (de, at))
        })
        .unwrap();
    let text = rewire_reference(&text, shell, shell_at, trimmed);
    let text = set_status(&text, face, "01000000");
    let text = set_status(&text, loop_de, "01000000");
    (without_identity(text), trimmed, outer)
}

fn trimmed_hole_fixture() -> (String, usize) {
    let (text, trimmed, _) = trimmed_outer_fixture();
    let (entities, _, _) = parse(&text).unwrap();
    let surface = u(&entities[&trimmed].values, 1, "trimmed surface").unwrap();
    let geometry = surface128(&entities, surface, 1.).unwrap();
    let uv = [[0.25, 0.25], [0.75, 0.25], [0.75, 0.75], [0.25, 0.75]];
    let mut text = text;
    let mut values = vec![
        "143".into(),
        "0".into(),
        "2".into(),
        surface.to_string(),
        "4".into(),
    ];
    for i in 0..4 {
        let a = uv[i];
        let b = uv[(i + 1) % 4];
        let xyz_a = geometry.evaluate(a[0], a[1]).unwrap().point;
        let xyz_b = geometry.evaluate(b[0], b[1]).unwrap().point;
        let (next, curve) = append_line(&text, xyz_a.to_vec(), xyz_b.to_vec(), false);
        let (next, pcurve) = append_line(&next, a.to_vec(), b.to_vec(), false);
        text = next;
        values.extend([curve.to_string(), pcurve.to_string(), "1".into()]);
    }
    let (text, inner) = append_entity(&text, 143, 0, "00010000", |_| values);
    let (entities, _, _) = parse(&text).unwrap();
    let mut trimmed_values = entities[&trimmed].values.clone();
    trimmed_values[3] = "1".into();
    trimmed_values.push(inner.to_string());
    let text = replace_parameters(&text, trimmed, trimmed_values);
    let solid = directory_entity(&text, 186, 0);
    let text = set_status(&text, solid, "01000000");
    // A single face with a hole is deliberately an open shell, not a closed solid.
    let shell = entities
        .values()
        .find(|entity| entity.directory.ty == 514)
        .map(|entity| entity.directory.form)
        .unwrap();
    assert_eq!(shell, 0);
    let shell_de = entities
        .iter()
        .find_map(|(&de, entity)| (entity.directory.ty == 514).then_some(de))
        .unwrap();
    let mut lines = text.lines().map(str::to_string).collect::<Vec<_>>();
    let directory = lines
        .iter_mut()
        .find(|line| {
            line.as_bytes()[72] == b'D'
                && line[73..80].trim().parse::<usize>().unwrap() == shell_de + 1
        })
        .unwrap();
    directory.replace_range(32..40, &format!("{:>8}", 1));
    let solid_de = entities
        .iter()
        .find_map(|(&de, entity)| (entity.directory.ty == 186).then_some(de))
        .unwrap();
    let text = set_status(&(lines.join("\n") + "\n"), solid_de, "01000000");
    (text, trimmed)
}

fn loop_uv_area(model: &Model, loop_index: usize) -> f64 {
    let points = model.loops[loop_index]
        .coedges
        .iter()
        .flat_map(|coedge| coedge.pcurve.control_points.iter().cloned())
        .collect::<Vec<_>>();
    let mut area = points
        .windows(2)
        .map(|pair| pair[0][0] * pair[1][1] - pair[1][0] * pair[0][1])
        .sum::<f64>();
    area += points.last().unwrap()[0] * points.first().unwrap()[1]
        - points.first().unwrap()[0] * points.last().unwrap()[1];
    area
}

fn directory_entity(text: &str, ty: usize, ordinal: usize) -> usize {
    text.lines()
        .filter(|line| {
            line.as_bytes()[72] == b'D'
                && line[..8].trim().parse::<usize>().unwrap() == ty
                && line[73..80].trim().parse::<usize>().unwrap() % 2 == 1
        })
        .nth(ordinal)
        .map(|line| line[73..80].trim().parse().unwrap())
        .unwrap()
}

fn set_status(text: &str, de: usize, status: &str) -> String {
    let mut lines = text.lines().map(str::to_string).collect::<Vec<_>>();
    let line = lines
        .iter_mut()
        .find(|line| {
            line.as_bytes()[72] == b'D' && line[73..80].trim().parse::<usize>().unwrap() == de
        })
        .unwrap();
    line.replace_range(64..72, status);
    lines.join("\n") + "\n"
}

fn append_entity(
    text: &str,
    ty: usize,
    form: usize,
    status: &str,
    values: impl FnOnce(usize) -> Vec<String>,
) -> (String, usize) {
    let mut lines = text.lines().map(str::to_string).collect::<Vec<_>>();
    let mut terminate = lines.pop().unwrap();
    let old_d = lines
        .iter()
        .filter(|line| line.as_bytes()[72] == b'D')
        .count()
        / 2;
    let old_p = lines
        .iter()
        .filter(|line| line.as_bytes()[72] == b'P')
        .count();
    let de = 2 * old_d + 1;
    let p_start = old_p + 1;
    let a = format!(
        "{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{status}",
        ty, p_start, 0, 0, 0, 0, 0, 0
    );
    let b = format!(
        "{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:<8}{:>8}",
        ty, 0, 0, 0, form, 0, 0, "", 0
    );
    let d_first = lines
        .iter()
        .position(|line| line.as_bytes()[72] == b'P')
        .unwrap();
    lines.splice(
        d_first..d_first,
        [
            pad_record(&a, 'D', de, 72).unwrap(),
            pad_record(&b, 'D', de + 1, 72).unwrap(),
        ],
    );
    let parameter = format!("{};", values(de).join(","));
    let mut p_count = 0;
    for chunk in parameter.as_bytes().chunks(64) {
        let sequence = old_p + p_count + 1;
        let payload = std::str::from_utf8(chunk).unwrap();
        lines.push(format!("{payload:<64}{de:>8}P{sequence:>7}",));
        p_count += 1;
    }
    terminate.replace_range(16..24, &format!("D{:>7}", old_d + 1));
    terminate.replace_range(24..32, &format!("P{:>7}", old_p + p_count));
    lines.push(terminate);
    (lines.join("\n") + "\n", de)
}

fn replace_parameters(text: &str, de: usize, values: Vec<String>) -> String {
    let mut lines = text.lines().map(str::to_string).collect::<Vec<_>>();
    let rows = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| {
            line.as_bytes()[72] == b'P' && line[64..72].trim().parse::<usize>().unwrap() == de
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let parameter = format!("{};", values.join(","));
    let chunks = parameter.as_bytes().chunks(64).collect::<Vec<_>>();
    assert_eq!(chunks.len(), rows.len());
    for (row, chunk) in rows.into_iter().zip(chunks) {
        let payload = std::str::from_utf8(chunk).unwrap();
        lines[row].replace_range(..64, &format!("{payload:<64}"));
    }
    lines.join("\n") + "\n"
}

fn refresh_identity_for(text: &str, target: usize) -> String {
    let (entities, _, _) = parse(text).unwrap();
    let mut updates = Vec::new();
    for (&de, entity) in &entities {
        if entity.directory.ty != 406 || entity.directory.form != 15 || entity.values.len() != 4 {
            continue;
        }
        if u(&entity.values, 3, "identity target").unwrap() != target {
            continue;
        }
        let payload = entity.values[2].strip_prefix("OSCAD_TOPO/IGES2|").unwrap();
        let parts = payload.split('|').collect::<Vec<_>>();
        let mut values = entity.values.clone();
        values[2] = hollerith(&format!(
            "OSCAD_TOPO/IGES2|{}|{}|{}|{}",
            parts[0],
            parts[1],
            topology_digest(&entities[&target]),
            parts[3]
        ));
        updates.push((de, values));
    }
    updates
        .into_iter()
        .fold(text.to_string(), |text, (de, values)| {
            replace_parameters(&text, de, values)
        })
}

fn rewire_reference(text: &str, owner: usize, at: usize, target: usize) -> String {
    let (entities, _, _) = parse(text).unwrap();
    let mut values = entities[&owner].values.clone();
    values[at] = target.to_string();
    refresh_identity_for(&replace_parameters(text, owner, values), owner)
}

fn append_line(text: &str, a: Vec<f64>, b: Vec<f64>, bspline: bool) -> (String, usize) {
    if bspline {
        let curve = Curve::from_polyline(vec![a, b]).unwrap();
        let dim = curve.control_points[0].len();
        append_entity(text, 126, 0, "00000000", |_| curve_values(&curve, dim))
    } else {
        let mut values = vec!["110".into()];
        for point in [a, b] {
            values.push(number(point[0]));
            values.push(number(point[1]));
            values.push(number(if point.len() == 3 { point[2] } else { 0. }));
        }
        append_entity(text, 110, 0, "00000000", |_| values)
    }
}

fn append_composite(text: &str, members: &[usize]) -> (String, usize) {
    let members = members.to_vec();
    append_entity(text, 102, 0, "00000000", |_| {
        let mut values = vec!["102".into(), members.len().to_string()];
        values.extend(members.into_iter().map(|de| de.to_string()));
        values
    })
}

fn first_edge_curve(text: &str) -> (usize, Curve) {
    let (entities, _, _) = parse(text).unwrap();
    let edge_list = directory_entity(text, 504, 0);
    let curve = u(&entities[&edge_list].values, 2, "edge curve").unwrap();
    (edge_list, curve126(&entities, curve, 3, 1.).unwrap())
}

fn split_edge_fixture(
    base: &str,
    count: usize,
    gap: f64,
    reversed: &[bool],
) -> (String, usize, Vec<usize>) {
    let (edge_list, curve) = first_edge_curve(base);
    let (entities, _, _) = parse(base).unwrap();
    let loop_de = directory_entity(base, 508, 0);
    let pcurve_de = u(&entities[&loop_de].values, 8, "loop pcurve").unwrap();
    let pcurve = curve126(&entities, pcurve_de, 2, 1.).unwrap();
    let domain = curve.domain();
    let pdomain = pcurve.domain();
    let start = curve.evaluate(domain[0]).unwrap().point;
    let end = curve.evaluate(domain[1]).unwrap().point;
    let length = start
        .iter()
        .zip(&end)
        .map(|(a, b)| (b - a) * (b - a))
        .sum::<f64>()
        .sqrt();
    let tangent = start
        .iter()
        .zip(&end)
        .map(|(a, b)| (b - a) / length)
        .collect::<Vec<_>>();
    let point = |fraction: f64| {
        curve
            .evaluate(domain[0] + fraction * (domain[1] - domain[0]))
            .unwrap()
            .point
    };
    let ppoint = |fraction: f64| {
        pcurve
            .evaluate(pdomain[0] + fraction * (pdomain[1] - pdomain[0]))
            .unwrap()
            .point
    };
    let mut text = base.to_string();
    let mut members = Vec::new();
    for i in 0..count {
        let mut a = point(i as f64 / count as f64);
        let mut b = point((i + 1) as f64 / count as f64);
        if i > 0 {
            for axis in 0..3 {
                a[axis] += tangent[axis] * gap * 0.5;
            }
        }
        if i + 1 < count {
            for axis in 0..3 {
                b[axis] -= tangent[axis] * gap * 0.5;
            }
        }
        if reversed.get(i).copied().unwrap_or(false) {
            std::mem::swap(&mut a, &mut b);
        }
        let (next, de) = append_line(&text, a, b, i % 2 == 1);
        text = next;
        members.push(de);
    }
    let (text, composite) = append_composite(&text, &members);
    let mut text = rewire_reference(&text, edge_list, 2, composite);
    let mut pmembers = Vec::new();
    for i in 0..count {
        let mut a = ppoint(i as f64 / count as f64);
        let mut b = ppoint((i + 1) as f64 / count as f64);
        if reversed.get(i).copied().unwrap_or(false) {
            std::mem::swap(&mut a, &mut b);
        }
        let (next, de) = append_line(&text, a, b, i % 2 == 1);
        text = next;
        pmembers.push(de);
    }
    let (text, pcomposite) = append_composite(&text, &pmembers);
    (
        rewire_reference(&text, loop_de, 8, pcomposite),
        composite,
        members,
    )
}

fn rewrite_global_value(text: &str, search: &str, replacement: &str) -> String {
    let lines = text.lines().collect::<Vec<_>>();
    let first = lines
        .iter()
        .position(|line| line.as_bytes()[72] == b'G')
        .unwrap();
    let last = lines
        .iter()
        .rposition(|line| line.as_bytes()[72] == b'G')
        .unwrap();
    let global = lines[first..=last]
        .iter()
        .map(|line| &line[..72])
        .collect::<String>();
    let updated = global.replace(search, replacement);
    assert_ne!(updated, global);
    let mut output = lines[..first]
        .iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>();
    let mut global_lines = updated
        .as_bytes()
        .chunks(72)
        .enumerate()
        .map(|(i, chunk)| pad_record(std::str::from_utf8(chunk).unwrap(), 'G', i + 1, 72).unwrap())
        .collect::<Vec<_>>();
    let global_count = global_lines.len();
    output.append(&mut global_lines);
    output.extend(lines[last + 1..].iter().map(|line| line.to_string()));
    let terminate = output.last_mut().unwrap();
    terminate.replace_range(8..16, &format!("G{:>7}", global_count));
    output.join("\n") + "\n"
}

fn rewrite_global(text: &str, replacement: &str) -> String {
    rewrite_global_value(text, ",2,2HMM,", replacement)
}

fn rewrite_resolution(text: &str, resolution: f64) -> String {
    let max_coordinate = number(1e-7);
    rewrite_global_value(
        text,
        &format!(",0.001,{max_coordinate},"),
        &format!(",{},{max_coordinate},", number(resolution)),
    )
}

fn scaled_geometry(mut model: Model, factor: f64) -> Model {
    for vertex in &mut model.vertices {
        for coordinate in &mut vertex.point {
            *coordinate *= factor;
        }
    }
    for edge in &mut model.edges {
        for point in &mut edge.curve.control_points {
            for coordinate in point {
                *coordinate *= factor;
            }
        }
    }
    for face in &mut model.faces {
        for point in face.surface.control_points.iter_mut().flatten() {
            for coordinate in point {
                *coordinate *= factor;
            }
        }
    }
    model.validate().unwrap();
    model
}

fn fixture_with_unit(size_mm: f64, unit: IgesLengthUnit) -> String {
    let model = freeform_cuboid_solid([0., 0., 0.], [size_mm; 3]).unwrap();
    let model = scaled_geometry(model, 1. / unit.scale_to_mm());
    let (text, _, _) = export_iges_v2(&model).unwrap();
    rewrite_global(&text, &format!(",{},2HIN,", unit.flag()))
}

fn assert_geometry_eq(left: &Model, right: &Model) {
    for (a, b) in left.vertices.iter().zip(&right.vertices) {
        for axis in 0..3 {
            assert!((a.point[axis] - b.point[axis]).abs() < 1e-9);
        }
    }
    for (a, b) in left.edges.iter().zip(&right.edges) {
        for (pa, pb) in a.curve.control_points.iter().zip(&b.curve.control_points) {
            for axis in 0..3 {
                assert!((pa[axis] - pb[axis]).abs() < 1e-9);
            }
        }
    }
    for (a, b) in left.faces.iter().zip(&right.faces) {
        for (pa, pb) in a
            .surface
            .control_points
            .iter()
            .flatten()
            .zip(b.surface.control_points.iter().flatten())
        {
            for axis in 0..3 {
                assert!((pa[axis] - pb[axis]).abs() < 1e-9);
            }
        }
    }
}

#[test]
fn direct_roundtrip_is_80_column_and_preserves_graph_identity() {
    let model = freeform_cuboid_solid([0., 0., 0.], [2., 3., 4.]).unwrap();
    let (text, _, report) = export_iges_v2(&model).unwrap();
    assert!(text.lines().all(|line| line.len() == 80));
    assert!(text.contains("     126") && text.contains("     128") && text.contains("     186"));
    let (back, cert, identity) = import_iges_v2(&text).unwrap();
    assert_eq!(cert.capability, IGES_INTERCHANGE_V2_CAPABILITY);
    assert!(identity.identity.preserved);
    assert_eq!(
        (
            back.vertices.len(),
            back.edges.len(),
            back.faces.len(),
            back.bodies.len()
        ),
        (8, 12, 6, 1)
    );
    assert_eq!(back.1.faces, model.1.faces);
    assert_eq!(report.topology_count, 34);
    assert_eq!(report.units.declared_unit, IgesLengthUnit::Millimeter);
    assert_eq!(report.units.applied_unit, IgesLengthUnit::Millimeter);
    assert_eq!(report.units.declared_flag, 2);
    assert_eq!(report.units.declared_scale_to_mm, 1.);
    assert_eq!(report.units.applied_scale_to_mm, 1.);
    assert!(!report.units.overridden);
    assert!(report.units.warning.is_none());
    assert_eq!(identity.units, report.units);
    assert_eq!(report.discarded, IgesV2DiscardReport::default());
    assert_eq!(identity.discarded, IgesV2DiscardReport::default());
}

#[test]
fn metadata_free_reports_loss_and_mutation_refuses() {
    let model = freeform_cuboid_solid([0., 0., 0.], [1., 1., 1.]).unwrap();
    let (text, _, _) = export_iges_v2(&model).unwrap();
    let mut lines = text.lines().map(str::to_string).collect::<Vec<_>>();
    for line in &mut lines {
        if line.as_bytes()[72] == b'D' && line[..8].trim() == "406" {
            line.replace_range(0..8, "     314");
        }
    }
    let stripped = lines.join("\n") + "\n";
    assert!(import_iges_v2(&stripped).is_err());
    let mutated = text.replacen("0.00000000000000000e0", "1.00000000000000000e-1", 1);
    assert!(import_iges_v2(&mutated).is_err());
}

#[test]
fn malformed_records_units_unknown_and_resources_refuse() {
    assert!(parse("not iges").is_err());
    let model = freeform_cuboid_solid([0., 0., 0.], [1., 1., 1.]).unwrap();
    let (text, _, _) = export_iges_v2(&model).unwrap();
    assert!(import_iges_v2(&text.replacen("     128", "     999", 1)).is_err());
    assert!(parse(&" ".repeat(MAX_BYTES + 1)).is_err());
}

#[test]
fn inch_mm_and_meter_flags_produce_identical_mm_geometry() {
    let (expected, _, _) =
        import_iges_v2(&fixture_with_unit(10., IgesLengthUnit::Millimeter)).unwrap();
    for unit in [IgesLengthUnit::Inch, IgesLengthUnit::Meter] {
        let (actual, _, report) = import_iges_v2(&fixture_with_unit(10., unit)).unwrap();
        assert_geometry_eq(&expected, &actual);
        assert_eq!(report.units.declared_unit, unit);
        assert_eq!(report.units.applied_unit, unit);
        assert_eq!(report.units.declared_flag, unit.flag());
        assert_eq!(report.units.applied_scale_to_mm, unit.scale_to_mm());
        assert!(!report.units.overridden);
    }
}

#[test]
fn misdeclared_inch_warns_but_default_and_override_are_explicit() {
    let model = freeform_cuboid_solid([0., 0., 0.], [1000.; 3]).unwrap();
    let (mm_text, _, _) = export_iges_v2(&model).unwrap();
    let text = rewrite_global(&mm_text, ",1,2HIN,");
    let (default, _, report) = import_iges_v2(&text).unwrap();
    assert_eq!(
        default
            .vertices
            .iter()
            .map(|v| v.point[0])
            .fold(0., f64::max),
        25400.
    );
    assert_eq!(report.units.declared_unit, IgesLengthUnit::Inch);
    assert_eq!(report.units.applied_unit, IgesLengthUnit::Inch);
    let warning = report.units.warning.as_deref().unwrap();
    for evidence in ["25.4", "scale 1 mm/unit", "25400", "1000"] {
        assert!(warning.contains(evidence), "missing {evidence}: {warning}");
    }

    let (corrected, _, override_report) = import_iges_v2_with_options(
        &text,
        IgesV2ImportOptions {
            unit_override: Some(IgesLengthUnit::Millimeter),
        },
    )
    .unwrap();
    assert_geometry_eq(&model, &corrected);
    assert_eq!(corrected.1.vertices, model.1.vertices);
    assert_eq!(corrected.1.edges, model.1.edges);
    assert_eq!(override_report.units.declared_unit, IgesLengthUnit::Inch);
    assert_eq!(
        override_report.units.applied_unit,
        IgesLengthUnit::Millimeter
    );
    assert_eq!(override_report.units.declared_scale_to_mm, 25.4);
    assert_eq!(override_report.units.applied_scale_to_mm, 1.);
    assert!(override_report.units.overridden);
}

#[test]
fn equally_reasonable_or_unreasonable_alternatives_do_not_warn() {
    for size_mm in [10., 1e-5] {
        let model = freeform_cuboid_solid([0., 0., 0.], [size_mm; 3]).unwrap();
        let (mm_text, _, _) = export_iges_v2(&model).unwrap();
        let (_, _, report) = import_iges_v2(&rewrite_global(&mm_text, ",1,2HIN,")).unwrap();
        assert!(report.units.warning.is_none());
    }
}

#[test]
fn microinch_flag_uses_iges_scale() {
    let (model, _, report) =
        import_iges_v2(&fixture_with_unit(1., IgesLengthUnit::Microinch)).unwrap();
    assert_eq!(IgesLengthUnit::Microinch.scale_to_mm(), 25.4e-6);
    assert_eq!(report.units.applied_scale_to_mm, 25.4e-6);
    assert_eq!(
        model.vertices.iter().map(|v| v.point[0]).fold(0., f64::max),
        1.
    );
}

#[test]
fn malformed_missing_and_unknown_unit_flags_are_typed_refusals() {
    let model = freeform_cuboid_solid([0., 0., 0.], [1.; 3]).unwrap();
    let (text, _, _) = export_iges_v2(&model).unwrap();
    for replacement in [",NaN,2HMM,", ",,2HMM,", ",3,2HMM,"] {
        let error = import_iges_v2(&rewrite_global(&text, replacement)).unwrap_err();
        assert_eq!(error.code, "BREP_IGES_V2_REFUSED");
    }
}

#[test]
fn blanked_malformed_curve_is_discarded_before_geometry() {
    let base = base_text();
    let source_count = parse(&base).unwrap().0.len();
    let (text, _) = append_entity(&base, 126, 0, "01010000", |_| vec!["126".into()]);
    let (_, _, report) = import_iges_v2(&text).unwrap();
    assert_eq!(report.entity_count, source_count + 1);
    assert_eq!(report.discarded.blanked, 1);
    assert_eq!(report.discarded.dependent_without_owner, 0);
}

#[test]
fn retained_face_rejects_blanked_surface_with_both_des() {
    let base = base_text();
    let surface = directory_entity(&base, 128, 0);
    let face = directory_entity(&base, 510, 0);
    let error = import_iges_v2(&set_status(&base, surface, "01000000")).unwrap_err();
    assert_eq!(error.code, "BREP_IGES_V2_REFUSED");
    assert!(error.message.contains(&format!("DE {face}")));
    assert!(error.message.contains(&format!("DE {surface}")));
    assert!(error.message.contains("discarded"));
}

#[test]
fn physical_logical_and_both_orphans_are_counted() {
    let (text, _) = append_entity(&base_text(), 126, 0, "00010000", |_| vec!["126".into()]);
    let (text, _) = append_entity(&text, 126, 0, "00020000", |_| vec!["126".into()]);
    let (text, _) = append_entity(&text, 126, 0, "00030000", |_| vec!["126".into()]);
    let (_, _, report) = import_iges_v2(&text).unwrap();
    assert_eq!(report.discarded.blanked, 0);
    assert_eq!(report.discarded.dependent_without_owner, 3);
}

#[test]
fn dependents_with_live_physical_and_logical_owners_survive() {
    let base = base_text();
    let physical = directory_entity(&base, 126, 0);
    let both = directory_entity(&base, 126, 1);
    let text = set_status(&base, physical, "00010000");
    let text = set_status(&text, both, "00030000");
    let (text, logical) = append_entity(&text, 126, 0, "00020000", |_| vec!["126".into()]);
    let (text, _) = append_entity(&text, 402, 15, "00000000", |_| {
        vec![
            "402".into(),
            "2".into(),
            both.to_string(),
            logical.to_string(),
        ]
    });
    let (_, _, report) = import_iges_v2(&text).unwrap();
    assert_eq!(report.discarded.dependent_without_owner, 0);
}

#[test]
fn discarded_group_owner_cascades_to_a_fixpoint() {
    let (text, _) = append_entity(&base_text(), 402, 15, "01000000", |de| {
        vec!["402".into(), "1".into(), (de + 2).to_string()]
    });
    let (text, _) = append_entity(&text, 402, 15, "00020000", |de| {
        vec!["402".into(), "1".into(), (de + 2).to_string()]
    });
    let (text, _) = append_entity(&text, 402, 15, "00020000", |_| {
        vec!["402".into(), "0".into()]
    });
    let (_, _, report) = import_iges_v2(&text).unwrap();
    assert_eq!(report.discarded.blanked, 1);
    assert_eq!(report.discarded.dependent_without_owner, 2);
}

#[test]
fn retained_group_rejects_missing_member_with_source_and_role() {
    let (text, group) = append_entity(&base_text(), 402, 15, "00000000", |_| {
        vec!["402".into(), "1".into(), "999999".into()]
    });
    let error = import_iges_v2(&text).unwrap_err();
    assert_eq!(error.code, "BREP_IGES_V2_REFUSED");
    assert!(error.message.contains(&format!("DE {group}")));
    assert!(error.message.contains("DE 999999"));
    assert!(error.message.contains("missing"));
    assert!(error.message.contains("associativity member"));
}

#[test]
fn malformed_directory_status_is_a_typed_refusal() {
    let base = base_text();
    let de = directory_entity(&base, 128, 0);
    for status in ["0A000000", "02000000", "00040000", "00000700", "00000003"] {
        let error = import_iges_v2(&set_status(&base, de, status)).unwrap_err();
        assert_eq!(error.code, "BREP_IGES_V2_REFUSED");
        assert!(error.message.contains(&format!("DE {de}")));
    }
}

#[test]
fn self_and_multi_node_402_cycles_are_bounded_typed_refusals() {
    let (self_text, self_de) = append_entity(&base_text(), 402, 15, "00000000", |de| {
        vec!["402".into(), "1".into(), de.to_string()]
    });
    let error = import_iges_v2(&self_text).unwrap_err();
    assert_eq!(error.code, "BREP_IGES_V2_REFUSED");
    assert!(error.message.contains("cycle"));
    assert!(error.message.contains(&format!("DE {self_de}")));

    let (text, a) = append_entity(&base_text(), 402, 15, "00000000", |de| {
        vec!["402".into(), "1".into(), (de + 2).to_string()]
    });
    let (text, b) = append_entity(&text, 402, 15, "00000000", |_| {
        vec!["402".into(), "1".into(), a.to_string()]
    });
    let error = import_iges_v2(&text).unwrap_err();
    assert_eq!(error.code, "BREP_IGES_V2_REFUSED");
    assert!(error.message.contains("cycle"));
    assert!(
        error.message.contains(&format!("DE {b}")) || error.message.contains(&format!("DE {a}"))
    );
}

#[test]
fn back_pointer_402_forms_are_explicitly_unsupported() {
    for form in [1, 14] {
        let (text, de) = append_entity(&base_text(), 402, form, "00000000", |_| {
            vec!["402".into(), "0".into()]
        });
        let error = import_iges_v2(&text).unwrap_err();
        assert_eq!(error.code, "BREP_IGES_V2_REFUSED");
        assert!(error.message.contains(&format!("DE {de}")));
        assert!(error.message.contains("back-pointer"));
    }
}

#[test]
fn global_parameter_19_is_the_exported_minimum_resolution() {
    let text = base_text();
    let global = text
        .lines()
        .filter(|line| line.as_bytes()[72] == b'G')
        .map(|line| &line[..72])
        .collect::<String>();
    let values = tokenize(&global, ',', ';').unwrap();
    assert_eq!(values[14], "MM");
    assert_eq!(values[18], "0.001");
    assert_eq!(parse_global(&global).unwrap().3, 0.001);
}

#[test]
fn trimmed_144_imports_143_outer_and_reuses_existing_edges() {
    let (text, trimmed, outer) = trimmed_outer_fixture();
    let (model, _, _) = import_iges_v2(&text).unwrap();
    model.validate().unwrap();
    assert_eq!(model.faces.len(), 6);
    assert_eq!(model.edges.len(), 12);
    let face = model.faces.last().unwrap();
    assert!(face.holes.is_empty());
    assert_eq!(model.loops[face.outer].coedges.len(), 4);
    assert!(loop_uv_area(&model, face.outer) > 0.);
    let (entities, _, _) = parse(&text).unwrap();
    assert_eq!(entities[&trimmed].directory.ty, 144);
    assert_eq!(entities[&outer].directory.ty, 143);
}

#[test]
fn trimmed_reversed_segment_order_is_oriented_without_reordering() {
    let (text, _, outer) = trimmed_outer_fixture();
    let (entities, _, _) = parse(&text).unwrap();
    let old = &entities[&outer].values;
    let n = u(old, 4, "boundary count").unwrap();
    let mut values = old[..5].to_vec();
    for i in (0..n).rev() {
        values.extend_from_slice(&old[5 + 3 * i..8 + 3 * i]);
    }
    let text = replace_parameters(&text, outer, values);
    let (model, _, _) = import_iges_v2(&text).unwrap();
    model.validate().unwrap();
    assert!(loop_uv_area(&model, model.faces.last().unwrap().outer) > 0.);
}

#[test]
fn trimmed_144_classifies_explicit_inner_loop_and_normalizes_direction() {
    let (text, trimmed) = trimmed_hole_fixture();
    let (model, _, _) = import_iges_v2(&text).unwrap();
    let face = model.faces.last().unwrap();
    assert_eq!(face.holes.len(), 1);
    assert!(loop_uv_area(&model, face.outer) > 0.);
    assert!(loop_uv_area(&model, face.holes[0]) < 0.);
    let (entities, _, _) = parse(&text).unwrap();
    assert_eq!(u(&entities[&trimmed].values, 3, "inner count").unwrap(), 1);
}

#[test]
fn trimmed_3d_only_and_natural_outer_are_typed_refusals_with_ids() {
    let (text, trimmed, outer) = trimmed_outer_fixture();
    let (entities, _, _) = parse(&text).unwrap();
    let mut boundary = entities[&outer].values.clone();
    boundary[1] = "1".into();
    let error = import_iges_v2(&replace_parameters(&text, outer, boundary)).unwrap_err();
    assert_eq!(error.code, "BREP_IGES_V2_TRIMMED_UNSUPPORTED");
    assert!(error.message.contains(&format!("DE {outer}")));

    let mut trimmed_values = entities[&trimmed].values.clone();
    trimmed_values[2] = "0".into();
    trimmed_values[4] = "0".into();
    let error = import_iges_v2(&replace_parameters(&text, trimmed, trimmed_values)).unwrap_err();
    assert_eq!(error.code, "BREP_IGES_V2_TRIMMED_UNSUPPORTED");
    assert!(error.message.contains(&format!("DE {trimmed}")));
}

#[test]
fn trimmed_open_boundary_reports_id_gap_and_tolerance() {
    let (text, _, outer) = trimmed_outer_fixture();
    let (entities, _, _) = parse(&text).unwrap();
    let curve_de = u(&entities[&outer].values, 5, "first boundary curve").unwrap();
    let curve = curve126(&entities, curve_de, 3, 1.).unwrap();
    let domain = curve.domain();
    let a = curve.evaluate(domain[0]).unwrap().point;
    let mut b = curve.evaluate(domain[1]).unwrap().point;
    b[1] += 0.01;
    let (text, replacement) = append_line(&text, a, b, false);
    let text = rewire_reference(&text, outer, 5, replacement);
    let error = import_iges_v2(&text).unwrap_err();
    assert_eq!(error.code, "BREP_IGES_V2_TRIMMED_GAP");
    assert!(error.message.contains(&format!("DE {outer}")));
    assert!(error.message.contains("gap_mm="));
    assert!(error.message.contains("tolerance_mm="));
}

#[test]
fn trimmed_joint_within_tolerance_snaps_xyz_and_uv_to_midpoint() {
    let entities = BTreeMap::new();
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 2., 2.],
        knots_v: vec![0., 0., 2., 2.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 2., 0.]],
            vec![vec![2., 0., 0.], vec![2., 2., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let mut builder = Builder {
        e: &entities,
        scale: 1.,
        merge_tolerance_mm: 0.001,
        vertices: vec![
            Vertex {
                point: [0., 0., 0.],
            },
            Vertex {
                point: [1., 0., 0.],
            },
            Vertex {
                point: [1.0005, 0., 0.],
            },
            Vertex {
                point: [1., 1., 0.],
            },
        ],
        edges: vec![
            Edge {
                degenerate: false,
                vertices: [0, 1],
                curve: crate::line(vec![0., 0., 0.], vec![1., 0., 0.]),
            },
            Edge {
                degenerate: false,
                vertices: [2, 3],
                curve: crate::line(vec![1.0005, 0., 0.], vec![1., 1., 0.]),
            },
        ],
        loops: vec![],
        faces: vec![],
        shells: vec![],
        bodies: vec![],
        vertex_de: vec![41; 4],
        edge_de: vec![41; 2],
        loop_de: vec![],
        face_de: vec![],
        shell_de: vec![],
        body_de: vec![],
    };
    let mut coedges = vec![
        Coedge::new(0, false, crate::line(vec![0., 0.], vec![1., 0.])),
        Coedge::new(1, false, crate::line(vec![1.0005, 0.], vec![1., 1.])),
    ];
    builder
        .close_trimmed_joint(41, 0, 1, &mut coedges, &BTreeSet::from([0, 1]), &surface)
        .unwrap();
    assert!((builder.edges[0].curve.control_points[1][0] - 1.00025).abs() < 1e-12);
    assert!((builder.edges[1].curve.control_points[0][0] - 1.00025).abs() < 1e-12);
    assert!((coedges[0].pcurve.control_points[1][0] - 1.00025).abs() < 1e-12);
    assert!((coedges[1].pcurve.control_points[0][0] - 1.00025).abs() < 1e-12);
    assert_eq!(builder.vertices[1].point, builder.vertices[2].point);
}

#[test]
fn composite_mixed_lines_and_linear_bsplines_join_in_an_existing_wire() {
    let (text, _, _) = split_edge_fixture(&composite_base_text(), 3, 0., &[true, true, false]);
    let (model, _, _) = import_iges_v2(&text).unwrap();
    model.validate().unwrap();
    assert_eq!(model.edges.len(), 12);
    assert_eq!(model.edges[0].curve.degree, 1);
    assert_eq!(model.edges[0].curve.control_points.len(), 4);
}

#[test]
fn composite_gap_zero_below_and_equal_tolerance_snaps_shared_endpoint() {
    let tolerance = 0.0009765625;
    for gap in [0., tolerance * 0.5, tolerance] {
        let base = rewrite_resolution(&composite_base_text(), tolerance);
        let (text, _, _) = split_edge_fixture(&base, 2, gap, &[false, false]);
        let (model, _, _) = import_iges_v2(&text).unwrap();
        model.validate().unwrap();
        let (_, original) = first_edge_curve(&base);
        let domain = original.domain();
        let a = original.evaluate(domain[0]).unwrap().point;
        let b = original.evaluate(domain[1]).unwrap().point;
        let points = &model.edges[0].curve.control_points;
        assert_eq!(points.len(), 3);
        for axis in 0..3 {
            assert!((points[1][axis] - (a[axis] + b[axis]) * 0.5).abs() < 1e-12);
        }
    }
}

#[test]
fn composite_gap_defect_has_des_and_measurements() {
    let (text, composite, members) =
        split_edge_fixture(&composite_base_text(), 2, 0.01, &[false, false]);
    let error = import_iges_v2(&text).unwrap_err();
    assert_eq!(error.code, "BREP_IGES_V2_COMPOSITE_GAP");
    assert!(error.message.contains(&format!("DE {composite}")));
    assert!(error.message.contains(&format!("DE {}", members[0])));
    assert!(error.message.contains(&format!("DE {}", members[1])));
    assert!(error.message.contains("gap_mm="));
    assert!(error.message.contains("tolerance_mm="));
}

#[test]
fn composite_reordering_and_orientation_ambiguity_are_refused() {
    let (text, a) = append_line(&base_text(), vec![0., 0., 0.], vec![1., 0., 0.], false);
    let (text, b) = append_line(&text, vec![2., 0., 0.], vec![0., 0., 0.], false);
    let (text, c) = append_line(&text, vec![1., 0., 0.], vec![2., 0., 0.], false);
    let (text, composite) = append_composite(&text, &[a, b, c]);
    let (edge_list, _) = first_edge_curve(&text);
    let error = import_iges_v2(&rewire_reference(&text, edge_list, 2, composite)).unwrap_err();
    assert_eq!(error.code, "BREP_IGES_V2_COMPOSITE_AMBIGUOUS");

    let (text, a) = append_line(&base_text(), vec![0., 0., 0.], vec![1., 0., 0.], false);
    let (text, b) = append_line(&text, vec![1., 0., 0.], vec![1., 0., 0.], false);
    let (text, composite) = append_composite(&text, &[a, b]);
    let (edge_list, _) = first_edge_curve(&text);
    let error = import_iges_v2(&rewire_reference(&text, edge_list, 2, composite)).unwrap_err();
    assert_eq!(error.code, "BREP_IGES_V2_COMPOSITE_AMBIGUOUS");
}

#[test]
fn composite_dangling_and_wrong_type_are_typed_defects() {
    let (text, composite) = append_composite(&base_text(), &[999_999]);
    let (edge_list, _) = first_edge_curve(&text);
    let error = import_iges_v2(&rewire_reference(&text, edge_list, 2, composite)).unwrap_err();
    assert_eq!(error.code, "BREP_IGES_V2_COMPOSITE_DANGLING");

    let base = base_text();
    let surface = directory_entity(&base, 128, 0);
    let (text, composite) = append_composite(&base, &[surface]);
    let (edge_list, _) = first_edge_curve(&text);
    let error = import_iges_v2(&rewire_reference(&text, edge_list, 2, composite)).unwrap_err();
    assert_eq!(error.code, "BREP_IGES_V2_COMPOSITE_DANGLING");
}

#[test]
fn composite_self_and_multi_node_cycles_are_typed_defects() {
    let (text, composite) = append_entity(&base_text(), 102, 0, "00000000", |de| {
        vec!["102".into(), "1".into(), de.to_string()]
    });
    let (edge_list, _) = first_edge_curve(&text);
    let error = import_iges_v2(&rewire_reference(&text, edge_list, 2, composite)).unwrap_err();
    assert_eq!(error.code, "BREP_IGES_V2_COMPOSITE_CYCLE");

    let (text, a) = append_entity(&base_text(), 102, 0, "00000000", |de| {
        vec!["102".into(), "1".into(), (de + 2).to_string()]
    });
    let (text, _) = append_entity(&text, 102, 0, "00000000", |_| {
        vec!["102".into(), "1".into(), a.to_string()]
    });
    let (edge_list, _) = first_edge_curve(&text);
    let error = import_iges_v2(&rewire_reference(&text, edge_list, 2, a)).unwrap_err();
    assert_eq!(error.code, "BREP_IGES_V2_COMPOSITE_CYCLE");
}

#[test]
fn composite_empty_and_depth_budget_are_typed_defects() {
    let (text, composite) = append_composite(&base_text(), &[]);
    let (edge_list, _) = first_edge_curve(&text);
    let error = import_iges_v2(&rewire_reference(&text, edge_list, 2, composite)).unwrap_err();
    assert_eq!(error.code, "BREP_IGES_V2_COMPOSITE_DANGLING");

    let (mut text, mut child) =
        append_line(&base_text(), vec![0., 0., 0.], vec![1., 0., 0.], false);
    for _ in 0..=MAX_COMPOSITE_DEPTH {
        let (next, parent) = append_composite(&text, &[child]);
        text = next;
        child = parent;
    }
    let (edge_list, _) = first_edge_curve(&text);
    let error = import_iges_v2(&rewire_reference(&text, edge_list, 2, child)).unwrap_err();
    assert_eq!(error.code, "BREP_IGES_V2_COMPOSITE_BUDGET");
}

#[test]
fn composite_tolerance_uses_applied_unit_scale() {
    let (mm_text, _, _) = split_edge_fixture(&composite_base_text(), 2, 0.002, &[false, false]);
    assert_eq!(
        import_iges_v2(&mm_text).unwrap_err().code,
        "BREP_IGES_V2_COMPOSITE_GAP"
    );
    let inch_base = rewrite_global(&composite_base_text(), ",1,2HIN,");
    let (inch_text, _, _) = split_edge_fixture(&inch_base, 2, 0.0001, &[false, false]);
    import_iges_v2(&inch_text).unwrap();
}

#[test]
fn composite_2d_pcurve_and_3d_edge_pass_face_correspondence() {
    let base = composite_base_text();
    let (text, _, _) = split_edge_fixture(&base, 2, 0., &[false, false]);
    let (entities, _, _) = parse(&base).unwrap();
    let loop_de = directory_entity(&text, 508, 0);
    let pcurve_de = u(&entities[&loop_de].values, 8, "loop pcurve").unwrap();
    let pcurve = curve126(&entities, pcurve_de, 2, 1.).unwrap();
    let domain = pcurve.domain();
    let a = pcurve.evaluate(domain[0]).unwrap().point;
    let b = pcurve.evaluate(domain[1]).unwrap().point;
    let mid = pcurve
        .evaluate((domain[0] + domain[1]) * 0.5)
        .unwrap()
        .point;
    let (text, first) = append_line(&text, a, mid.clone(), false);
    let (text, second) = append_line(&text, mid, b, true);
    let (text, composite) = append_composite(&text, &[first, second]);
    let text = rewire_reference(&text, loop_de, 8, composite);
    let (model, _, _) = import_iges_v2(&text).unwrap();
    model.validate().unwrap();
    assert_eq!(model.loops[0].coedges[0].pcurve.control_points.len(), 3);
}

#[test]
fn composite_arc_and_general_rational_join_are_explicitly_unsupported() {
    let (text, arc) = append_entity(&base_text(), 100, 0, "00000000", |_| vec!["100".into()]);
    let (text, composite) = append_composite(&text, &[arc]);
    let (edge_list, _) = first_edge_curve(&text);
    let error = import_iges_v2(&rewire_reference(&text, edge_list, 2, composite)).unwrap_err();
    assert_eq!(error.code, "BREP_IGES_V2_COMPOSITE_UNSUPPORTED");

    let curve = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.], vec![2., 0., 0.]])
        .unwrap()
        .elevate(2)
        .unwrap();
    let (text, spline) = append_entity(&base_text(), 126, 0, "00000000", |_| {
        curve_values(&curve, 3)
    });
    let (text, composite) = append_composite(&text, &[spline]);
    let (edge_list, _) = first_edge_curve(&text);
    let error = import_iges_v2(&rewire_reference(&text, edge_list, 2, composite)).unwrap_err();
    assert_eq!(error.code, "BREP_IGES_V2_COMPOSITE_UNSUPPORTED");
}

#[test]
fn multiple_bodies_and_cavities_roundtrip_without_topology_reconstruction() {
    let mut model = Model::empty(1e-7).unwrap();
    append(
        &mut model,
        freeform_cuboid_solid([0., 0., 0.], [10., 10., 10.]).unwrap(),
    );
    append(
        &mut model,
        freeform_cuboid_solid([2., 2., 2.], [4., 4., 4.]).unwrap(),
    );
    append(
        &mut model,
        freeform_cuboid_solid([20., 0., 0.], [21., 1., 1.]).unwrap(),
    );
    model.0.bodies = vec![
        Body {
            outer_shell: 0,
            inner_shells: vec![1],
        },
        Body {
            outer_shell: 2,
            inner_shells: vec![],
        },
    ];
    model.rebuild_topology_ids();
    model.validate().unwrap();
    let (text, _, _) = export_iges_v2(&model).unwrap();
    let (back, _, report) = import_iges_v2(&text).unwrap();
    assert_eq!(back.bodies.len(), 2);
    assert_eq!(back.bodies[0].inner_shells.len(), 1);
    assert_eq!(
        (
            back.vertices.len(),
            back.edges.len(),
            back.faces.len(),
            back.shells.len()
        ),
        (24, 36, 18, 3)
    );
    assert!(report.identity.preserved);
}
