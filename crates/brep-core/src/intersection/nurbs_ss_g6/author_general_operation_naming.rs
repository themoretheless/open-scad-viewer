use super::*;

pub(super) fn author_general_operation_naming(
    result: &mut Model,
    source: &Model,
    cutter: &Model,
    source_face: usize,
    operation: &str,
    bounds: &[[f64; 4]],
    axis: ExactIsoAxis,
) -> Result<GeneralNurbsBooleanNaming> {
    let root_signature = bounds
        .iter()
        .flat_map(|bound| bound.iter())
        .map(|value| format!("{:016x}", value.to_bits()))
        .collect::<Vec<_>>()
        .join(":");
    let source_occurrence = source.1.bodies[0].to_string();
    derive_operation_local_ids(result, &source_occurrence, operation, &root_signature);

    let mut change_set = ChangeSet::default();
    for (kind, ids) in source
        .identity_groups()
        .into_iter()
        .chain(cutter.identity_groups())
    {
        change_set.nodes.extend(ids.iter().map(|id| (*id, kind)));
    }
    for (kind, ids) in result.identity_groups() {
        change_set.nodes.extend(ids.iter().map(|id| (*id, kind)));
    }
    let split_children = retained_split_children(result, bounds, axis)?;
    change_set.changes.push(TopologyChange {
        kind: ChangeKind::Split,
        topo_kind: TopoKind::Face,
        parents: vec![source.1.faces[source_face]],
        children: split_children.clone(),
        provenance: ChangeProvenance {
            operation: GENERAL_NURBS_BOOLEAN_AUTHORITY.into(),
            operand: Some(source_occurrence.clone()),
            occurrence: root_signature.clone(),
        },
        role: format!("{operation}-split-retained-generated-boundaries"),
        anchor: Some("global-uv-cells".into()),
    });
    let generated = push_generated_changes(
        &mut change_set,
        result,
        &split_children,
        &source_occurrence,
        operation,
    );
    let deleted = push_deleted_cutter_changes(&mut change_set, cutter, operation);
    change_set.validate()?;
    result.1.change_set = change_set;
    result.validate()?;
    Ok(GeneralNurbsBooleanNaming {
        split: 1,
        retained: split_children.len(),
        deleted,
        generated,
        operation_stable: true,
    })
}

/// Faces of each clipped graph cell (top face plus side faces).
const CELL_FACE_COUNT: usize = 6;

/// Replaces every result identity with an operation-local derived id.
fn derive_operation_local_ids(
    result: &mut Model,
    source_occurrence: &str,
    operation: &str,
    root_signature: &str,
) {
    let derive_group = |kind: TopoKind, count: usize| {
        (0..count)
            .map(|index| {
                TopoId::derive(
                    kind,
                    GENERAL_NURBS_BOOLEAN_AUTHORITY,
                    &format!("{source_occurrence}:{operation}:{root_signature}:{index}"),
                    kind.as_str(),
                    b"operation-local-topology",
                )
            })
            .collect::<Vec<_>>()
    };
    result.1.vertices = derive_group(TopoKind::Vertex, result.vertices.len());
    result.1.edges = derive_group(TopoKind::Edge, result.edges.len());
    result.1.loops = derive_group(TopoKind::Loop, result.loops.len());
    result.1.faces = derive_group(TopoKind::Face, result.faces.len());
    result.1.shells = derive_group(TopoKind::Shell, result.shells.len());
    result.1.bodies = derive_group(TopoKind::Body, result.bodies.len());
    result.1.lineage.clear();
}

/// Sorted retained top faces and strict-interior seam faces of every cell.
fn retained_split_children(
    result: &Model,
    bounds: &[[f64; 4]],
    axis: ExactIsoAxis,
) -> Result<Vec<TopoId>> {
    let face_id = |face: usize| {
        result
            .1
            .faces
            .get(face)
            .copied()
            .ok_or_else(|| refuse("General NURBS naming partition cell is missing"))
    };
    let mut split_children = Vec::new();
    for (component, bound) in bounds.iter().enumerate() {
        let base = component * CELL_FACE_COUNT;
        split_children.push(face_id(base + 1)?);
        let candidates = match axis {
            ExactIsoAxis::U => [(bound[0], base + 5), (bound[1], base + 3)],
            ExactIsoAxis::V => [(bound[2], base + 2), (bound[3], base + 4)],
        };
        for (parameter, face) in candidates {
            if parameter > 0. && parameter < 1. {
                split_children.push(face_id(face)?);
            }
        }
    }
    split_children.sort();
    split_children.dedup();
    Ok(split_children)
}

/// Records every non-split result identity as generated; returns the count.
fn push_generated_changes(
    change_set: &mut ChangeSet,
    result: &Model,
    split_children: &[TopoId],
    source_occurrence: &str,
    operation: &str,
) -> usize {
    let split_set = split_children.iter().copied().collect::<BTreeSet<_>>();
    let mut generated = 0;
    for (kind, ids) in result.identity_groups() {
        for id in ids {
            if kind == TopoKind::Face && split_set.contains(id) {
                continue;
            }
            generated += 1;
            change_set.changes.push(TopologyChange {
                kind: ChangeKind::Generated,
                topo_kind: kind,
                parents: vec![],
                children: vec![*id],
                provenance: ChangeProvenance {
                    operation: GENERAL_NURBS_BOOLEAN_AUTHORITY.into(),
                    operand: Some(source_occurrence.to_owned()),
                    occurrence: id.to_string(),
                },
                role: format!("{operation}-generated-{}", kind.as_str()),
                anchor: None,
            });
        }
    }
    generated
}

/// Records every cutter identity as deleted; returns the count.
fn push_deleted_cutter_changes(
    change_set: &mut ChangeSet,
    cutter: &Model,
    operation: &str,
) -> usize {
    let mut deleted = 0;
    for (kind, ids) in cutter.identity_groups() {
        for id in ids {
            deleted += 1;
            change_set.changes.push(TopologyChange {
                kind: ChangeKind::Deleted,
                topo_kind: kind,
                parents: vec![*id],
                children: vec![],
                provenance: ChangeProvenance {
                    operation: GENERAL_NURBS_BOOLEAN_AUTHORITY.into(),
                    operand: cutter.1.bodies.first().map(ToString::to_string),
                    occurrence: id.to_string(),
                },
                role: format!("{operation}-deleted-cutter-{}", kind.as_str()),
                anchor: None,
            });
        }
    }
    deleted
}

pub(super) fn graph_frame_box(
    origin: [f64; 3],
    u: [f64; 3],
    v: [f64; 3],
    normal: [f64; 3],
    bounds: [[f64; 2]; 3],
    tolerance: f64,
) -> Result<Model> {
    let local = crate::cuboid(
        [bounds[0][0], bounds[1][0], bounds[2][0]],
        [bounds[0][1], bounds[1][1], bounds[2][1]],
    )?;
    let matrix = [
        [u[0], v[0], normal[0], origin[0]],
        [u[1], v[1], normal[1], origin[1]],
        [u[2], v[2], normal[2], origin[2]],
        [0., 0., 0., 1.],
    ];
    let mut model = crate::transform::affine(&local, matrix)?;
    model.tolerance_mm = tolerance;
    model.validate()?;
    Ok(model)
}
