use super::*;

pub fn compose_step_v8_occurrences(models: &[Model]) -> Result<Model> {
    let aggregate = compose_step_occurrences(models, STEP_INTERCHANGE_V8_CAPABILITY)?;
    certify_step_v8_topology(&aggregate)?;
    Ok(aggregate)
}
pub fn compose_step_v9_occurrences(models: &[Model]) -> Result<Model> {
    let aggregate = compose_step_occurrences(models, STEP_INTERCHANGE_V9_CAPABILITY)?;
    certify_step_v8_topology(&aggregate)?;
    Ok(aggregate)
}

#[derive(Clone)]
pub(super) struct RepresentationOccurrence {
    pub(super) representation: usize,
    pub(super) bodies: Vec<usize>,
    pub(super) transform: [[f64; 4]; 4],
    pub(super) linked: BTreeSet<usize>,
    pub(super) path: Vec<usize>,
}

pub(super) fn relationship_components(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
) -> Result<Option<(usize, usize, usize)>> {
    let parts = components(entities, id)?;
    let Some(base) = component(&parts, "REPRESENTATION_RELATIONSHIP") else {
        return Ok(None);
    };
    let Some(with_transform) = component(&parts, "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION")
    else {
        return Ok(None);
    };
    if base.len() != 4 || with_transform.len() != 1 {
        return Err(refuse(
            "Malformed REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION complex",
        ));
    }
    Ok(Some((
        one_ref(&base[2], "relationship source representation")?,
        one_ref(&base[3], "relationship target representation")?,
        one_ref(&with_transform[0], "relationship transformation operator")?,
    )))
}

pub(super) fn representation_occurrences(
    entities: &BTreeMap<usize, Entity>,
    selected: &[usize],
    allow_affine: bool,
    general_affine: bool,
    allow_open_shells: bool,
) -> Result<Vec<RepresentationOccurrence>> {
    // CDSR identifies product occurrences. Its relationship runs child -> parent;
    // traverse parent -> child while retaining the child-to-parent placement.
    let mut products = BTreeMap::new();
    let mut assembly = BTreeMap::new();
    if general_affine {
        for entity in entities.values() {
            if let Value::Call(name, args) = &entity.value
                && name == "SHAPE_DEFINITION_REPRESENTATION"
                && args.len() == 2
            {
                let (kind, shape) = call(entities, one_ref(&args[0], "product shape")?)?;
                if kind == "PRODUCT_DEFINITION_SHAPE" && shape.len() == 3 {
                    products.insert(
                        one_ref(&args[1], "product representation")?,
                        one_ref(&shape[2], "product definition")?,
                    );
                }
            }
        }
        for entity in entities.values() {
            if let Value::Call(name, args) = &entity.value
                && name == "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION"
            {
                if args.len() != 2 {
                    return Err(refuse("Malformed product occurrence"));
                }
                let relation = one_ref(&args[0], "occurrence relationship")?;
                let (child, parent, _) = relationship_components(entities, relation)?
                    .ok_or_else(|| refuse("Occurrence requires a placement relationship"))?;
                let (kind, shape) = call(entities, one_ref(&args[1], "occurrence shape")?)?;
                if kind != "PRODUCT_DEFINITION_SHAPE" || shape.len() != 3 {
                    return Err(refuse("Invalid occurrence product shape"));
                }
                let (kind, usage) = call(entities, one_ref(&shape[2], "occurrence usage")?)?;
                if kind != "NEXT_ASSEMBLY_USAGE_OCCURRENCE"
                    || usage.len() != 6
                    || products.get(&parent) != Some(&one_ref(&usage[3], "parent product")?)
                    || products.get(&child) != Some(&one_ref(&usage[4], "child product")?)
                {
                    return Err(refuse(
                        "Occurrence product endpoints disagree with placement",
                    ));
                }
                if assembly.insert(relation, (parent, child)).is_some() {
                    return Err(refuse("Duplicate product occurrence relationship"));
                }
            }
        }
    }
    let mut outgoing: BTreeMap<usize, Vec<(usize, usize, usize, bool)>> = BTreeMap::new();
    let mut component_representations = BTreeSet::new();
    for id in entities.keys() {
        if let Some((from, to, operator)) = relationship_components(entities, *id)? {
            if let Some(&(parent, child)) = assembly.get(id) {
                outgoing
                    .entry(parent)
                    .or_default()
                    .push((*id, child, operator, true));
                component_representations.insert(child);
            } else {
                outgoing
                    .entry(from)
                    .or_default()
                    .push((*id, to, operator, false));
            }
        }
    }
    for edges in outgoing.values_mut() {
        edges.sort_by_key(|edge| edge.0)
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "recursive STEP graph walk keeps traversal state explicit"
    )]
    pub(super) fn walk(
        rep: usize,
        matrix: [[f64; 4]; 4],
        entities: &BTreeMap<usize, Entity>,
        outgoing: &BTreeMap<usize, Vec<(usize, usize, usize, bool)>>,
        active: &mut BTreeSet<usize>,
        depth: usize,
        allow_affine: bool,
        general_affine: bool,
        allow_open_shells: bool,
        path: &[usize],
        out: &mut Vec<RepresentationOccurrence>,
    ) -> Result<()> {
        if depth > 32 {
            return Err(refuse(
                "Representation relationship placement depth exceeds 32",
            ));
        }
        if !active.insert(rep) {
            return Err(refuse(
                "Cycle in selected representation-relationship placement graph",
            ));
        }
        let (ty, args) = call(entities, rep)?;
        if ty != "ADVANCED_BREP_SHAPE_REPRESENTATION"
            && !(general_affine && ty == "SHAPE_REPRESENTATION")
            && !(allow_open_shells && ty == "MANIFOLD_SURFACE_SHAPE_REPRESENTATION")
        {
            return Err(refuse(
                "Placement graph reaches an unsupported representation",
            ));
        }
        if args.len() != 3 {
            return Err(refuse("Shape representation argument count mismatch"));
        }
        if ty == "SHAPE_REPRESENTATION" {
            if !outgoing.contains_key(&rep) {
                return Err(refuse("Empty product assembly container"));
            }
            for item in list(&args[1], "assembly placement items")? {
                if call(entities, one_ref(item, "assembly item")?)?.0 != "AXIS2_PLACEMENT_3D" {
                    return Err(refuse("Unsupported geometry in product assembly container"));
                }
            }
        }
        let linked = reachable(entities, &[rep])?;
        let scale = length_scale(entities, &linked, false)?;
        let bodies = list(&args[1], "representation items")?
            .iter()
            .filter_map(|value| match value {
                Value::Ref(id) => call(entities, *id).ok().and_then(|(name, _)| {
                    (matches!(name, "MANIFOLD_SOLID_BREP" | "BREP_WITH_VOIDS")
                        || allow_open_shells && name == "SHELL_BASED_SURFACE_MODEL")
                        .then_some(*id)
                }),
                _ => None,
            })
            .collect::<Vec<_>>();
        if !bodies.is_empty() {
            if out.len() >= MAX_OCCURRENCES {
                return Err(refuse("Assembly occurrence count exceeds 256"));
            }
            out.push(RepresentationOccurrence {
                representation: rep,
                bodies,
                transform: matrix,
                linked,
                path: path.to_vec(),
            })
        }
        if let Some(edges) = outgoing.get(&rep) {
            for (relationship, child, operator, product_occurrence) in edges {
                let affine_complex = allow_affine
                    && component(
                        &components(entities, *operator)?,
                        "CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM",
                    )
                    .is_some();
                let edge_matrix = if affine_complex {
                    cartesian_operator_affine(entities, *operator, scale, general_affine)?
                } else {
                    let (operator_type, operator_args) = call(entities, *operator)?;
                    match operator_type {
                        "CARTESIAN_TRANSFORMATION_OPERATOR_3D" => {
                            cartesian_operator_3d(entities, operator_args, scale)?
                        }
                        "ITEM_DEFINED_TRANSFORMATION" => {
                            if operator_args.len() != 4 {
                                return Err(refuse(
                                    "ITEM_DEFINED_TRANSFORMATION argument count mismatch",
                                ));
                            }
                            let source_scale = if *product_occurrence {
                                length_scale(entities, &reachable(entities, &[*child])?, false)?
                            } else {
                                scale
                            };
                            let from = rigid_frame(
                                entities,
                                one_ref(&operator_args[2], "transformation source")?,
                                source_scale,
                            )?;
                            let to = rigid_frame(
                                entities,
                                one_ref(&operator_args[3], "transformation target")?,
                                scale,
                            )?;
                            mul(to, inverse_rigid(from))
                        }
                        _ => {
                            return Err(refuse(
                                "Representation relationship uses a non-rigid or unsupported transformation operator",
                            ));
                        }
                    }
                };
                let mut child_path = path.to_vec();
                child_path.extend([*relationship, *child]);
                walk(
                    *child,
                    mul(matrix, edge_matrix),
                    entities,
                    outgoing,
                    active,
                    depth + 1,
                    allow_affine,
                    general_affine,
                    allow_open_shells,
                    &child_path,
                    out,
                )?;
            }
        }
        active.remove(&rep);
        Ok(())
    }
    let identity = [
        [1., 0., 0., 0.],
        [0., 1., 0., 0.],
        [0., 0., 1., 0.],
        [0., 0., 0., 1.],
    ];
    let roots: Vec<_> = selected
        .iter()
        .filter(|rep| !component_representations.contains(rep))
        .copied()
        .collect();
    let mut covered = BTreeSet::new();
    let mut pending = roots.clone();
    while let Some(rep) = pending.pop() {
        if covered.insert(rep) {
            if let Some(edges) = outgoing.get(&rep) {
                pending.extend(edges.iter().map(|edge| edge.1));
            }
        }
    }
    if selected.iter().any(|rep| !covered.contains(rep)) {
        return Err(refuse("Cyclic or unreachable product assembly"));
    }
    let mut out = Vec::new();
    for rep in &roots {
        walk(
            *rep,
            identity,
            entities,
            &outgoing,
            &mut BTreeSet::new(),
            0,
            allow_affine,
            general_affine,
            allow_open_shells,
            &[*rep],
            &mut out,
        )?
    }
    let mut paths = BTreeSet::new();
    for occurrence in &out {
        let key = (
            occurrence.representation,
            occurrence.bodies.clone(),
            occurrence
                .transform
                .iter()
                .flatten()
                .map(|v| v.to_bits())
                .collect::<Vec<_>>(),
        );
        if !paths.insert(key) {
            return Err(refuse(
                "Ambiguous duplicate selected representation occurrence",
            ));
        }
    }
    Ok(out)
}

pub(super) fn digest_value(
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
pub(super) fn hash(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
pub(super) fn entity_digest(entities: &BTreeMap<usize, Entity>, id: usize) -> Result<String> {
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

pub(super) fn restore_identity(
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

pub(super) const UNREACHABLE_ALLOWLIST: &[&str] = &[
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
