use super::*;

pub(super) struct AssemblyGraph<'a> {
    entities: &'a BTreeMap<usize, Entity>,
    outgoing: &'a BTreeMap<usize, Vec<(usize, usize, usize, bool)>>,
}

#[derive(Clone, Copy)]
pub(super) struct WalkPolicy {
    allow_affine: bool,
    general_affine: bool,
    allow_open_shells: bool,
}

#[derive(Clone, Copy)]
struct WalkNode<'p> {
    rep: usize,
    matrix: [[f64; 4]; 4],
    depth: usize,
    path: &'p [usize],
}

impl Clone for AssemblyGraph<'_> {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for AssemblyGraph<'_> {}

pub(super) fn representation_occurrences(
    entities: &BTreeMap<usize, Entity>,
    selected: &[usize],
    allow_affine: bool,
    general_affine: bool,
    allow_open_shells: bool,
) -> Result<Vec<RepresentationOccurrence>> {
    let assembly = if general_affine {
        collect_product_assembly(entities)?
    } else {
        BTreeMap::new()
    };
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
        edges.sort_by_key(|edge| edge.0);
    }
    for rep in selected {
        validate_identity_cycles(
            *rep,
            &outgoing,
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
            0,
        )?;
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
        if covered.insert(rep)
            && let Some(edges) = outgoing.get(&rep)
        {
            pending.extend(edges.iter().map(|edge| edge.1));
        }
    }
    if selected.iter().any(|rep| !covered.contains(rep)) {
        return Err(refuse("Cyclic or unreachable product assembly"));
    }
    let graph = AssemblyGraph {
        entities,
        outgoing: &outgoing,
    };
    let policy = WalkPolicy {
        allow_affine,
        general_affine,
        allow_open_shells,
    };
    let mut out = Vec::new();
    for rep in &roots {
        walk(
            graph,
            policy,
            WalkNode {
                rep: *rep,
                matrix: identity,
                depth: 0,
                path: &[*rep],
            },
            &mut BTreeSet::new(),
            &mut out,
        )?;
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

fn collect_product_assembly(
    entities: &BTreeMap<usize, Entity>,
) -> Result<BTreeMap<usize, (usize, usize)>> {
    // CDSR identifies product occurrences. Its relationship runs child -> parent;
    // traverse parent -> child while retaining the child-to-parent placement.
    let mut products = BTreeMap::new();
    let mut assembly = BTreeMap::new();
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
    Ok(assembly)
}

fn validate_shape_representation_container(
    entities: &BTreeMap<usize, Entity>,
    outgoing: &BTreeMap<usize, Vec<(usize, usize, usize, bool)>>,
    rep: usize,
    args: &[Value],
) -> Result<()> {
    let items = list(&args[1], "assembly placement items")?;
    let has_direct_body = items.iter().any(|item| {
        one_ref(item, "assembly item").is_ok_and(|item| {
            call(entities, item)
                .is_ok_and(|(ty, _)| matches!(ty, "MANIFOLD_SOLID_BREP" | "BREP_WITH_VOIDS"))
        })
    });
    if !outgoing.contains_key(&rep) && !has_direct_body {
        return Err(refuse("Empty product assembly container"));
    }
    for item in items {
        if !matches!(
            call(entities, one_ref(item, "assembly item")?)?.0,
            "AXIS2_PLACEMENT_3D"
                | "MANIFOLD_SOLID_BREP"
                | "BREP_WITH_VOIDS"
                | "MAPPED_ITEM"
                | "GEOMETRIC_CURVE_SET"
        ) {
            return Err(refuse("Unsupported geometry in product assembly container"));
        }
    }
    Ok(())
}

fn resolve_edge_transform(
    entities: &BTreeMap<usize, Entity>,
    policy: WalkPolicy,
    matrix: [[f64; 4]; 4],
    scale: f64,
    edge: (usize, usize, bool),
) -> Result<[[f64; 4]; 4]> {
    let (child, operator, product_occurrence) = edge;
    if operator == 0 {
        return Ok(matrix);
    }
    let affine_complex = policy.allow_affine
        && component(
            &components(entities, operator)?,
            "CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM",
        )
        .is_some();
    if affine_complex {
        return cartesian_operator_affine(entities, operator, scale, policy.general_affine);
    }
    let (operator_type, operator_args) = call(entities, operator)?;
    match operator_type {
        "CARTESIAN_TRANSFORMATION_OPERATOR_3D" => {
            cartesian_operator_3d(entities, operator_args, scale)
        }
        "ITEM_DEFINED_TRANSFORMATION" => {
            if operator_args.len() != 4 {
                return Err(refuse(
                    "ITEM_DEFINED_TRANSFORMATION argument count mismatch",
                ));
            }
            let source_scale = if product_occurrence {
                length_scale(entities, &reachable(entities, &[child])?, false)?
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
            Ok(mul(to, inverse_rigid(from)))
        }
        _ => Err(refuse(
            "Representation relationship uses a non-rigid or unsupported transformation operator",
        )),
    }
}

fn walk(
    graph: AssemblyGraph<'_>,
    policy: WalkPolicy,
    node: WalkNode<'_>,
    active: &mut BTreeSet<usize>,
    out: &mut Vec<RepresentationOccurrence>,
) -> Result<()> {
    let AssemblyGraph { entities, outgoing } = graph;
    let WalkPolicy {
        general_affine,
        allow_open_shells,
        ..
    } = policy;
    let WalkNode {
        rep,
        matrix,
        depth,
        path,
    } = node;
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
        validate_shape_representation_container(entities, outgoing, rep, args)?;
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
        });
    }
    if let Some(edges) = outgoing.get(&rep) {
        let exact_identity_edges = edges
            .iter()
            .filter(|(_, child, operator, _)| {
                *operator == 0
                    && call(entities, *child)
                        .is_ok_and(|(ty, _)| ty == "ADVANCED_BREP_SHAPE_REPRESENTATION")
            })
            .count();
        if exact_identity_edges > 1 {
            return Err(refuse(
                "Ambiguous multiple exact B-rep representation roots",
            ));
        }
        for &(relationship, child, operator, product_occurrence) in edges {
            // Plain relationships can also connect PMI, wireframe and other
            // non-exact views. They remain retained metadata, not B-rep occurrences.
            if operator == 0 && call(entities, child)?.0 != "ADVANCED_BREP_SHAPE_REPRESENTATION" {
                continue;
            }
            let edge_matrix = resolve_edge_transform(
                entities,
                policy,
                matrix,
                scale,
                (child, operator, product_occurrence),
            )?;
            let mut child_path = path.to_vec();
            child_path.extend([relationship, child]);
            walk(
                graph,
                policy,
                WalkNode {
                    rep: child,
                    matrix: mul(matrix, edge_matrix),
                    depth: depth + 1,
                    path: &child_path,
                },
                active,
                out,
            )?;
        }
    }
    active.remove(&rep);
    Ok(())
}

fn validate_identity_cycles(
    rep: usize,
    outgoing: &BTreeMap<usize, Vec<(usize, usize, usize, bool)>>,
    active: &mut BTreeSet<usize>,
    done: &mut BTreeSet<usize>,
    depth: usize,
) -> Result<()> {
    if depth > 32 {
        return Err(refuse(
            "Identity representation relationship depth exceeds 32",
        ));
    }
    if done.contains(&rep) {
        return Ok(());
    }
    if !active.insert(rep) {
        return Err(refuse(
            "Cycle in identity representation relationship graph",
        ));
    }
    if let Some(edges) = outgoing.get(&rep) {
        for (_, child, _, _) in edges.iter().filter(|edge| edge.2 == 0) {
            validate_identity_cycles(*child, outgoing, active, done, depth + 1)?;
        }
    }
    active.remove(&rep);
    done.insert(rep);
    Ok(())
}
