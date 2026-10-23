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

#[test]
fn direct_roundtrip_is_80_column_and_preserves_graph_identity() {
    let model = freeform_cuboid_solid([0., 0., 0.], [2., 3., 4.]).unwrap();
    let (text, _, report) = export_iges_v2(&model).unwrap();
    assert!(text.lines().all(|line| line.len() == 80));
    assert!(
        text.contains("     126") && text.contains("     128") && text.contains("     186")
    );
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
