use super::*;
/// Compose already validated document occurrences into one authoritative
/// bounded model and assign occurrence-local topology identities.
fn compose_step_occurrences(models: &[Model], capability: &str) -> Result<Model> {
    if models.is_empty() {
        return Err(refuse(
            "STEP /7 composition requires at least one occurrence",
        ));
    }
    if models.len() > MAX_OCCURRENCES {
        return Err(refuse("STEP /7 composed occurrence count exceeds 256"));
    }
    let mut aggregate = Model::empty(1e-7)?;
    let mut ignored = Vec::new();
    for model in models {
        model.validate()?;
        append_direct_model(&mut aggregate, model.clone(), &[], &mut ignored);
    }
    aggregate.rebuild_topology_ids();
    let occurrence_ids = |kind: TopoKind, ids: &[TopoId]| {
        ids.iter()
            .enumerate()
            .map(|(index, id)| {
                TopoId::derive(
                    kind,
                    capability,
                    &format!("occurrence-entity-{index}"),
                    kind.as_str(),
                    id.to_string().as_bytes(),
                )
            })
            .collect::<Vec<_>>()
    };
    aggregate.1.vertices = occurrence_ids(TopoKind::Vertex, &aggregate.1.vertices);
    aggregate.1.edges = occurrence_ids(TopoKind::Edge, &aggregate.1.edges);
    aggregate.1.loops = occurrence_ids(TopoKind::Loop, &aggregate.1.loops);
    aggregate.1.faces = occurrence_ids(TopoKind::Face, &aggregate.1.faces);
    aggregate.1.shells = occurrence_ids(TopoKind::Shell, &aggregate.1.shells);
    aggregate.1.bodies = occurrence_ids(TopoKind::Body, &aggregate.1.bodies);
    aggregate.1.lineage.clear();
    aggregate.1.change_set = brep_topology::ChangeSet::default();
    for (kind, ids) in [
        (TopoKind::Vertex, &aggregate.1.vertices),
        (TopoKind::Edge, &aggregate.1.edges),
        (TopoKind::Loop, &aggregate.1.loops),
        (TopoKind::Face, &aggregate.1.faces),
        (TopoKind::Shell, &aggregate.1.shells),
        (TopoKind::Body, &aggregate.1.bodies),
    ] {
        for id in ids {
            aggregate.1.change_set.nodes.insert(*id, kind);
        }
    }
    aggregate.validate()?;
    Ok(aggregate)
}
pub fn compose_step_v7_occurrences(models: &[Model]) -> Result<Model> {
    compose_step_occurrences(models, STEP_INTERCHANGE_V7_CAPABILITY)
}
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
pub(crate) struct RepresentationOccurrence {
    pub(crate) representation: usize,
    pub(crate) bodies: Vec<usize>,
    pub(crate) transform: [[f64; 4]; 4],
    pub(crate) linked: BTreeSet<usize>,
    pub(crate) path: Vec<usize>,
}

pub(crate) fn relationship_components(
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

pub(crate) fn representation_occurrences(
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
    fn walk(
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
