use super::*;
fn digest_value(
    value: &Value,
    entities: &BTreeMap<usize, Entity>,
    active: &mut BTreeSet<usize>,
    depth: usize,
) -> Result<String> {
    if depth > MAX_GRAPH_DEPTH {
        return Err(refuse("Canonical graph digest depth exceeds 64"));
    }
    let text = match value {
        Value::String(_) => "''".into(), // entity names are non-geometric metadata
        Value::Number(n) => format!("{n:.17e}"),
        Value::Integer(n) => n.to_string(),
        Value::Enum(v) => format!(".{v}."),
        Value::Omitted => "$".into(),
        Value::Derived => "*".into(),
        Value::List(v) => format!(
            "({})",
            v.iter()
                .map(|x| digest_value(x, entities, active, depth + 1))
                .collect::<Result<Vec<_>>>()?
                .join(",")
        ),
        Value::Call(n, v) => {
            let args = v
                .iter()
                .enumerate()
                .map(|(i, x)| {
                    if i == 0 && matches!(x, Value::String(_)) {
                        Ok("''".into())
                    } else {
                        digest_value(x, entities, active, depth + 1)
                    }
                })
                .collect::<Result<Vec<_>>>()?;
            format!("{n}({})", args.join(","))
        }
        Value::Ref(id) => {
            if !active.insert(*id) {
                return Err(refuse("Cycle while computing canonical graph digest"));
            }
            let e = entities
                .get(id)
                .ok_or_else(|| refuse("Missing digest dependency"))?;
            let result = digest_value(&e.value, entities, active, depth + 1)?;
            active.remove(id);
            format!("#{result}")
        }
    };
    Ok(text)
}
pub(crate) fn hash(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
pub(crate) fn entity_digest(entities: &BTreeMap<usize, Entity>, id: usize) -> Result<String> {
    let entity = entities
        .get(&id)
        .ok_or_else(|| refuse("Missing identity entity"))?;
    Ok(hash(&digest_value(
        &entity.value,
        entities,
        &mut BTreeSet::from([id]),
        0,
    )?))
}

pub(crate) fn restore_identity(
    entities: &BTreeMap<usize, Entity>,
    map: &[(TopoKind, usize, usize)],
    model: &mut Model,
) -> Result<crate::step_interchange::StepIdentityReport> {
    let mut metadata = Vec::new();
    for &(kind, index, entity_id) in map {
        let (_, args) = call(entities, entity_id)?;
        let Some(n) = name(args) else {
            continue;
        };
        let Some(payload) = n.strip_prefix("OSCAD_TOPO/3|") else {
            continue;
        };
        let fields: Vec<_> = payload.split('|').collect();
        if fields.len() != 3 {
            return Err(refuse("Malformed STEP /3 identity metadata"));
        }
        let mk =
            TopoKind::parse(fields[0]).ok_or_else(|| refuse("Unknown STEP /3 identity kind"))?;
        let id = TopoId::parse(fields[1]).map_err(refuse)?;
        if mk != kind || id.kind() != kind || fields[2] != entity_digest(entities, entity_id)? {
            return Err(refuse("Transplanted STEP /3 identity metadata"));
        }
        metadata.push((kind, index, id));
    }
    let count = map.len();
    if metadata.is_empty() {
        model.rebuild_topology_ids();
        return Ok(crate::step_interchange::StepIdentityReport {
            preserved: false,
            source: "external-step",
            preserved_count: 0,
            created_count: count,
            lost_count: count,
        });
    }
    if metadata.len() != count {
        return Err(refuse("Partial STEP /3 identity metadata"));
    }
    let mut seen = BTreeSet::new();
    let mut remap = BTreeMap::new();
    for (kind, index, id) in metadata {
        if !seen.insert(id) {
            return Err(refuse("Duplicate STEP /3 identity metadata"));
        }
        let old = match kind {
            TopoKind::Vertex => model.1.vertices[index],
            TopoKind::Edge => model.1.edges[index],
            TopoKind::Loop => model.1.loops[index],
            TopoKind::Face => model.1.faces[index],
            TopoKind::Shell => model.1.shells[index],
            TopoKind::Body => model.1.bodies[index],
            TopoKind::ControlPoint => {
                return Err(refuse(
                    "Control-point identity is not STEP topology identity",
                ));
            }
        };
        remap.insert(old, id);
        match kind {
            TopoKind::Vertex => model.1.vertices[index] = id,
            TopoKind::Edge => model.1.edges[index] = id,
            TopoKind::Loop => model.1.loops[index] = id,
            TopoKind::Face => model.1.faces[index] = id,
            TopoKind::Shell => model.1.shells[index] = id,
            TopoKind::Body => model.1.bodies[index] = id,
            TopoKind::ControlPoint => {
                return Err(refuse(
                    "Control-point identity is not STEP topology identity",
                ));
            }
        }
    }
    model.1.change_set.nodes = model
        .1
        .change_set
        .nodes
        .iter()
        .map(|(id, kind)| (remap.get(id).copied().unwrap_or(*id), *kind))
        .collect();
    for change in &mut model.1.change_set.changes {
        for id in change.parents.iter_mut().chain(&mut change.children) {
            if let Some(replacement) = remap.get(id) {
                *id = *replacement;
            }
        }
    }
    Ok(crate::step_interchange::StepIdentityReport {
        preserved: true,
        source: "internal-metadata",
        preserved_count: count,
        created_count: 0,
        lost_count: 0,
    })
}

pub(crate) const UNREACHABLE_ALLOWLIST: &[&str] = &[
    "APPLICATION_CONTEXT",
    "PRODUCT",
    "PRODUCT_DEFINITION",
    "PRODUCT_DEFINITION_FORMATION",
    "PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE",
    "PRODUCT_RELATED_PRODUCT_CATEGORY",
    "PRODUCT_CONTEXT",
    "PRODUCT_DEFINITION_CONTEXT",
    "PRODUCT_DEFINITION_SHAPE",
    "SHAPE_DEFINITION_REPRESENTATION",
    "ADVANCED_BREP_SHAPE_REPRESENTATION",
    "MANIFOLD_SURFACE_SHAPE_REPRESENTATION",
    "SHELL_BASED_SURFACE_MODEL",
    "GEOMETRIC_REPRESENTATION_CONTEXT",
    "GLOBAL_UNIT_ASSIGNED_CONTEXT",
    "SI_UNIT",
    "PRESENTATION_LAYER_ASSIGNMENT",
    "STYLED_ITEM",
    "COLOUR_RGB",
    "SURFACE_STYLE_USAGE",
    "SURFACE_SIDE_STYLE",
    "SURFACE_STYLE_FILL_AREA",
    "FILL_AREA_STYLE",
    "FILL_AREA_STYLE_COLOUR",
    "DRAUGHTING_PRE_DEFINED_COLOUR",
    "PROPERTY_DEFINITION",
    "PROPERTY_DEFINITION_REPRESENTATION",
    "DESCRIPTIVE_REPRESENTATION_ITEM",
    "MEASURE_REPRESENTATION_ITEM",
    "MATERIAL_DESIGNATION",
    "APPLIED_NAME_ASSIGNMENT",
    "APPLIED_DESCRIPTION_TEXT_ASSIGNMENT",
    "PERSON",
    "ORGANIZATION",
    "PERSON_AND_ORGANIZATION",
    "PERSON_AND_ORGANIZATION_ROLE",
    "APPLIED_PERSON_AND_ORGANIZATION_ASSIGNMENT",
    "DATE_AND_TIME",
    "CALENDAR_DATE",
    "LOCAL_TIME",
    "COORDINATED_UNIVERSAL_TIME_OFFSET",
    "DATE_TIME_ROLE",
    "APPLIED_DATE_AND_TIME_ASSIGNMENT",
    "APPROVAL",
    "APPROVAL_STATUS",
    "APPROVAL_ROLE",
    "APPLIED_APPROVAL_ASSIGNMENT",
    "SECURITY_CLASSIFICATION",
    "SECURITY_CLASSIFICATION_LEVEL",
    "APPLIED_SECURITY_CLASSIFICATION_ASSIGNMENT",
    "EXTERNAL_CLASS_LIBRARY",
    "ITEM_DEFINED_TRANSFORMATION",
    "AXIS2_PLACEMENT_3D",
    "CARTESIAN_POINT",
    "DIRECTION",
    "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION",
    "CARTESIAN_TRANSFORMATION_OPERATOR_3D",
];
