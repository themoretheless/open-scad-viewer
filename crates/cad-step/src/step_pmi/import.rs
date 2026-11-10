//! AP242 semantic GD&T and its deliberately separate graphic presentation.

use super::*;
use brep_core::TopoId;
use crate::step_part21::{Entity, Value, call, components, list, number, one_ref, parse, refs};
use std::collections::{BTreeMap, BTreeSet};

const MAX_PMI_DEPTH: usize = 64;

fn semantic_error(kind: PmiErrorKind, entity: usize, message: impl Into<String>) -> PmiError {
    PmiError::new(kind, Some(entity), message)
}

fn parse_error(error: nurbs_core::Error) -> PmiError {
    PmiError::new(PmiErrorKind::MalformedPart21, None, error.message)
}

fn text(value: &Value, entity: usize, what: &str) -> PmiResult<String> {
    match value {
        Value::String(s) => Ok(s.clone()),
        Value::Omitted | Value::Derived => Ok(String::new()),
        _ => Err(semantic_error(
            PmiErrorKind::MalformedSemantic,
            entity,
            format!("{what} must be a string or $"),
        )),
    }
}

fn direct_refs(value: &Value) -> BTreeSet<usize> {
    let mut out = Vec::new();
    refs(value, &mut out);
    out.into_iter().collect()
}

fn is_shape_aspect_type(name: &str) -> bool {
    matches!(
        name,
        "SHAPE_ASPECT"
            | "DATUM"
            | "DATUM_FEATURE"
            | "COMPOSITE_SHAPE_ASPECT"
            | "COMPOSITE_GROUP_SHAPE_ASPECT"
            | "ALL_AROUND_SHAPE_ASPECT"
            | "BETWEEN_SHAPE_ASPECT"
    )
}

fn face_candidates(
    entities: &BTreeMap<usize, Entity>,
    aspect: usize,
    face_entities: &BTreeMap<usize, TopoId>,
) -> BTreeSet<TopoId> {
    let mut out = BTreeSet::new();
    for (&id, entity) in entities {
        let refs = direct_refs(&entity.value);
        if !refs.contains(&aspect) {
            continue;
        }
        for face in refs {
            if let Some(topo) = face_entities.get(&face) {
                out.insert(*topo);
            }
        }
        // A representation usage may point to a representation rather than
        // directly to its ADVANCED_FACE items.
        for reference in direct_refs(&entity.value) {
            if let Some(target) = entities.get(&reference) {
                for nested in direct_refs(&target.value) {
                    if let Some(topo) = face_entities.get(&nested) {
                        out.insert(*topo);
                    }
                }
            }
        }
        if id == aspect {
            continue;
        }
    }
    out
}

fn measure(entities: &BTreeMap<usize, Entity>, value: &Value, owner: usize) -> PmiResult<f64> {
    match value {
        Value::Number(_) | Value::Integer(_) => number(value, "tolerance magnitude")
            .map_err(|e| semantic_error(PmiErrorKind::MalformedSemantic, owner, e.message)),
        Value::Ref(id) => {
            let (ty, args) = call(entities, *id)
                .map_err(|e| semantic_error(PmiErrorKind::DanglingDatum, owner, e.message))?;
            match ty {
                "LENGTH_MEASURE_WITH_UNIT"
                | "PLANE_ANGLE_MEASURE_WITH_UNIT"
                | "MEASURE_WITH_UNIT"
                    if !args.is_empty() =>
                {
                    measure(entities, &args[0], owner)
                }
                "MEASURE_REPRESENTATION_ITEM" if args.len() >= 2 => {
                    measure(entities, &args[1], owner)
                }
                "LENGTH_MEASURE" | "PLANE_ANGLE_MEASURE" | "MEASURE_VALUE" if args.len() == 1 => {
                    measure(entities, &args[0], owner)
                }
                _ => Err(semantic_error(
                    PmiErrorKind::MalformedSemantic,
                    owner,
                    format!("#{id} ({ty}) is not a supported tolerance measure"),
                )),
            }
        }
        Value::Call(name, args)
            if matches!(
                name.as_str(),
                "LENGTH_MEASURE" | "PLANE_ANGLE_MEASURE" | "MEASURE_VALUE"
            ) && args.len() == 1 =>
        {
            measure(entities, &args[0], owner)
        }
        _ => Err(semantic_error(
            PmiErrorKind::MalformedSemantic,
            owner,
            "tolerance magnitude must be numeric or a supported measure",
        )),
    }
}

fn datum_label(entities: &BTreeMap<usize, Entity>, id: usize) -> PmiResult<String> {
    let parts = components(entities, id)
        .map_err(|e| semantic_error(PmiErrorKind::DanglingDatum, id, e.message))?;
    for (ty, args) in parts {
        if ty == "DATUM" {
            if args.len() >= 5 {
                return text(&args[4], id, "DATUM identification");
            }
            return Err(semantic_error(
                PmiErrorKind::MalformedSemantic,
                id,
                "DATUM requires identification at argument 5",
            ));
        }
    }
    let (_, args) = call(entities, id)
        .map_err(|e| semantic_error(PmiErrorKind::DanglingDatum, id, e.message))?;
    text(args.first().unwrap_or(&Value::Omitted), id, "datum name")
}

fn resolve_datum(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
    index: &BTreeMap<usize, TopoId>,
    owner: usize,
) -> PmiResult<DatumTarget> {
    let parts = components(entities, id)
        .map_err(|e| semantic_error(PmiErrorKind::DanglingDatum, owner, e.message))?;
    if !parts
        .iter()
        .any(|(ty, _)| matches!(*ty, "DATUM" | "DATUM_FEATURE"))
    {
        return Err(semantic_error(
            PmiErrorKind::DanglingDatum,
            owner,
            format!("datum reference #{id} is not DATUM or DATUM_FEATURE"),
        ));
    }
    let face_id = index.get(&id).copied().ok_or_else(|| {
        semantic_error(
            PmiErrorKind::DanglingFace,
            owner,
            format!("datum #{id} has no unique persistent face binding"),
        )
    })?;
    Ok(DatumTarget {
        entity: id,
        label: datum_label(entities, id)?,
        feature_entity: id,
        face_id,
    })
}

fn datum_precedence(index: usize) -> DatumPrecedence {
    match index {
        0 => DatumPrecedence::Primary,
        1 => DatumPrecedence::Secondary,
        2 => DatumPrecedence::Tertiary,
        n => DatumPrecedence::Subsequent((n - 2) as u32),
    }
}

fn datum_reference(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
    precedence: DatumPrecedence,
    index: &BTreeMap<usize, TopoId>,
) -> PmiResult<DatumReference> {
    let parts = components(entities, id)
        .map_err(|e| semantic_error(PmiErrorKind::DanglingDatum, id, e.message))?;
    let mut target_ids = Vec::new();
    let mut modifiers = Vec::new();
    let mut own_precedence = precedence;
    for (ty, args) in parts {
        match ty {
            "DATUM_REFERENCE_COMPARTMENT" | "DATUM_REFERENCE_ELEMENT" => {
                if args.len() >= 5 {
                    match &args[4] {
                        Value::Ref(target) => target_ids.push(*target),
                        Value::List(items) => {
                            for item in items {
                                target_ids.push(one_ref(item, "composite datum target").map_err(
                                    |e| semantic_error(PmiErrorKind::DanglingDatum, id, e.message),
                                )?);
                            }
                        }
                        Value::Omitted | Value::Derived => {}
                        _ => {
                            return Err(semantic_error(
                                PmiErrorKind::MalformedSemantic,
                                id,
                                "invalid datum compartment target",
                            ));
                        }
                    }
                }
                if let Some(value) = args.get(5) {
                    modifiers.extend(PmiModifier::parse(value, id)?);
                }
            }
            "DATUM_REFERENCE" => {
                if let Some(Value::Enum(value)) = args.first() {
                    own_precedence = match value.as_str() {
                        "PRIMARY" => DatumPrecedence::Primary,
                        "SECONDARY" => DatumPrecedence::Secondary,
                        "TERTIARY" => DatumPrecedence::Tertiary,
                        _ => DatumPrecedence::Unspecified,
                    };
                }
                if let Some(value) = args.get(1) {
                    target_ids.push(
                        one_ref(value, "datum reference target").map_err(|e| {
                            semantic_error(PmiErrorKind::DanglingDatum, id, e.message)
                        })?,
                    );
                }
            }
            _ => {}
        }
    }
    target_ids.sort_unstable();
    target_ids.dedup();
    if target_ids.is_empty() {
        return Err(semantic_error(
            PmiErrorKind::DanglingDatum,
            id,
            "datum reference has no target",
        ));
    }
    let datums = target_ids
        .into_iter()
        .map(|target| resolve_datum(entities, target, index, id))
        .collect::<PmiResult<Vec<_>>>()?;
    Ok(DatumReference {
        precedence: own_precedence,
        datums,
        modifiers,
    })
}

fn tolerance_datums(
    entities: &BTreeMap<usize, Entity>,
    value: &Value,
    index: &BTreeMap<usize, TopoId>,
    owner: usize,
) -> PmiResult<Vec<DatumReference>> {
    let values: Vec<&Value> = match value {
        Value::Omitted | Value::Derived => return Ok(vec![]),
        Value::List(items) => items.iter().collect(),
        other => vec![other],
    };
    let mut out = Vec::new();
    for value in values {
        let id = one_ref(value, "tolerance datum system")
            .map_err(|e| semantic_error(PmiErrorKind::DanglingDatum, owner, e.message))?;
        let parts = components(entities, id)
            .map_err(|e| semantic_error(PmiErrorKind::DanglingDatum, owner, e.message))?;
        if let Some((_, args)) = parts.iter().find(|(ty, _)| *ty == "DATUM_SYSTEM") {
            let members = args.get(4).ok_or_else(|| {
                semantic_error(
                    PmiErrorKind::MalformedSemantic,
                    id,
                    "DATUM_SYSTEM is missing members",
                )
            })?;
            for (position, member) in list(members, "DATUM_SYSTEM members")
                .map_err(|e| semantic_error(PmiErrorKind::MalformedSemantic, id, e.message))?
                .iter()
                .enumerate()
            {
                let member = one_ref(member, "DATUM_SYSTEM member")
                    .map_err(|e| semantic_error(PmiErrorKind::DanglingDatum, id, e.message))?;
                out.push(datum_reference(
                    entities,
                    member,
                    datum_precedence(position),
                    index,
                )?);
            }
        } else {
            out.push(datum_reference(
                entities,
                id,
                datum_precedence(out.len()),
                index,
            )?);
        }
    }
    Ok(out)
}

fn presentation_roots(entities: &BTreeMap<usize, Entity>) -> BTreeSet<usize> {
    entities
        .iter()
        .filter_map(|(&id, entity)| {
            let mut names = Vec::new();
            match &entity.value {
                Value::Call(name, _) => names.push(name.as_str()),
                Value::List(parts) => names.extend(parts.iter().filter_map(|part| match part {
                    Value::Call(name, _) => Some(name.as_str()),
                    _ => None,
                })),
                _ => {}
            }
            names
                .into_iter()
                .any(|name| {
                    name.contains("ANNOTATION")
                        || name == "DRAUGHTING_CALLOUT"
                        || name == "TESSELLATED_PRESENTATION_ITEM"
                })
                .then_some(id)
        })
        .collect()
}

fn imported_presentation(
    entities: &BTreeMap<usize, Entity>,
) -> PmiResult<(Vec<PmiPolyline>, BTreeSet<usize>)> {
    let roots = presentation_roots(entities);
    let mut out = Vec::new();
    for &root in &roots {
        let root_semantic = entities.get(&root).and_then(|entity| {
            direct_refs(&entity.value).into_iter().find(|reference| {
                components(entities, *reference).ok().is_some_and(|parts| {
                    parts.iter().any(|(ty, _)| {
                        ty.ends_with("_TOLERANCE") || matches!(*ty, "DATUM" | "DATUM_FEATURE")
                    })
                })
            })
        });
        let mut queue = vec![(root, 0usize)];
        let mut seen = BTreeSet::new();
        while let Some((id, depth)) = queue.pop() {
            if depth > MAX_PMI_DEPTH {
                return Err(semantic_error(
                    PmiErrorKind::MalformedPart21,
                    id,
                    "PMI presentation graph exceeds 64 levels",
                ));
            }
            if !seen.insert(id) {
                continue;
            }
            let entity = entities.get(&id).ok_or_else(|| {
                semantic_error(
                    PmiErrorKind::MalformedPart21,
                    root,
                    format!("presentation reference #{id} is missing"),
                )
            })?;
            let all_refs = direct_refs(&entity.value);
            if matches!(&entity.value, Value::Call(name, _) if name == "POLYLINE") {
                let args = match &entity.value {
                    Value::Call(_, args) => args,
                    _ => unreachable!(),
                };
                let points = args.get(1).ok_or_else(|| {
                    semantic_error(
                        PmiErrorKind::MalformedPart21,
                        id,
                        "POLYLINE is missing points",
                    )
                })?;
                let mut line_points = Vec::new();
                for point in list(points, "POLYLINE points")
                    .map_err(|e| semantic_error(PmiErrorKind::MalformedPart21, id, e.message))?
                {
                    let point_id = one_ref(point, "POLYLINE point").map_err(|e| {
                        semantic_error(PmiErrorKind::MalformedPart21, id, e.message)
                    })?;
                    let (ty, args) = call(entities, point_id).map_err(|e| {
                        semantic_error(PmiErrorKind::MalformedPart21, id, e.message)
                    })?;
                    if ty != "CARTESIAN_POINT" || args.len() != 2 {
                        return Err(semantic_error(
                            PmiErrorKind::MalformedPart21,
                            point_id,
                            "POLYLINE point is not CARTESIAN_POINT",
                        ));
                    }
                    let coordinates =
                        list(&args[1], "CARTESIAN_POINT coordinates").map_err(|e| {
                            semantic_error(PmiErrorKind::MalformedPart21, point_id, e.message)
                        })?;
                    if coordinates.len() != 3 {
                        return Err(semantic_error(
                            PmiErrorKind::MalformedPart21,
                            point_id,
                            "PMI presentation requires 3D points",
                        ));
                    }
                    let mut xyz = [0.0; 3];
                    for (axis, coordinate) in coordinates.iter().enumerate() {
                        xyz[axis] = number(coordinate, "presentation coordinate").map_err(|e| {
                            semantic_error(PmiErrorKind::MalformedPart21, point_id, e.message)
                        })?;
                    }
                    line_points.push(xyz);
                }
                let semantic_entity = root_semantic.or_else(|| {
                    all_refs.iter().copied().find(|reference| {
                        components(entities, *reference).ok().is_some_and(|parts| {
                            parts.iter().any(|(ty, _)| {
                                ty.ends_with("_TOLERANCE")
                                    || matches!(*ty, "DATUM" | "DATUM_FEATURE")
                            })
                        })
                    })
                });
                out.push(PmiPolyline {
                    source_entity: id,
                    semantic_entity,
                    role: PmiPolylineRole::Imported,
                    points: line_points,
                });
            }
            queue.extend(all_refs.into_iter().map(|reference| (reference, depth + 1)));
        }
    }
    out.sort_by_key(|line| line.source_entity);
    Ok((out, roots))
}

/// Imports semantic PMI independently of geometry construction.
///
/// `face_entities` must be the geometry importer's exact STEP entity map.
/// Missing or ambiguous bindings needed by a tolerance/datum are hard errors;
/// unrelated product-level shape aspects are allowed to remain unbound.
pub fn import_step_pmi(
    source: &str,
    face_entities: &BTreeMap<usize, TopoId>,
) -> PmiResult<StepPmiModel> {
    let entities = parse(source).map_err(parse_error)?;
    import_parsed_pmi(&entities, face_entities)
}

fn import_parsed_pmi(
    entities: &BTreeMap<usize, Entity>,
    face_entities: &BTreeMap<usize, TopoId>,
) -> PmiResult<StepPmiModel> {
    let mut model = StepPmiModel::default();
    let mut aspect_ids = BTreeSet::new();
    for (&id, _) in entities {
        let parts = components(entities, id)
            .map_err(|e| semantic_error(PmiErrorKind::MalformedPart21, id, e.message))?;
        if parts.iter().any(|(ty, _)| is_shape_aspect_type(ty)) {
            aspect_ids.insert(id);
        }
    }

    for (&id, _) in entities {
        let parts = components(entities, id)
            .map_err(|e| semantic_error(PmiErrorKind::MalformedPart21, id, e.message))?;
        if let Some((_, args)) = parts
            .iter()
            .find(|(ty, _)| *ty == "SHAPE_ASPECT_RELATIONSHIP")
        {
            if args.len() != 4 {
                return Err(semantic_error(
                    PmiErrorKind::MalformedSemantic,
                    id,
                    "SHAPE_ASPECT_RELATIONSHIP requires four arguments",
                ));
            }
            let relating = one_ref(&args[2], "relating shape_aspect")
                .map_err(|e| semantic_error(PmiErrorKind::MalformedSemantic, id, e.message))?;
            let related = one_ref(&args[3], "related shape_aspect")
                .map_err(|e| semantic_error(PmiErrorKind::MalformedSemantic, id, e.message))?;
            if !aspect_ids.contains(&relating) || !aspect_ids.contains(&related) {
                return Err(semantic_error(
                    PmiErrorKind::MalformedSemantic,
                    id,
                    "shape_aspect relationship endpoint is not a shape_aspect",
                ));
            }
            model
                .shape_aspect_relationships
                .push(ShapeAspectRelationship {
                    entity: id,
                    name: text(&args[0], id, "relationship name")?,
                    description: text(&args[1], id, "relationship description")?,
                    relating,
                    related,
                });
        }
    }

    for &aspect in &aspect_ids {
        let parts = components(entities, aspect)
            .map_err(|e| semantic_error(PmiErrorKind::MalformedPart21, aspect, e.message))?;
        let (_, args) = parts
            .iter()
            .find(|(ty, _)| is_shape_aspect_type(ty))
            .unwrap();
        let mut related_aspects = Vec::new();
        let mut candidates = face_candidates(entities, aspect, face_entities);
        let mut visited = BTreeSet::from([aspect]);
        let mut queue = vec![aspect];
        while let Some(current) = queue.pop() {
            for relation in model
                .shape_aspect_relationships
                .iter()
                .filter(|r| r.relating == current || r.related == current)
            {
                let other = if relation.relating == current {
                    relation.related
                } else {
                    relation.relating
                };
                if visited.insert(other) {
                    related_aspects.push(other);
                    candidates.extend(face_candidates(entities, other, face_entities));
                    queue.push(other);
                }
            }
        }
        related_aspects.sort_unstable();
        let candidate_face_ids = candidates.into_iter().collect::<Vec<_>>();
        let face_id = (candidate_face_ids.len() == 1).then(|| candidate_face_ids[0]);
        if let Some(face) = face_id {
            model.shape_aspect_to_face.insert(aspect, face);
        } else if candidate_face_ids.len() > 1 {
            model.diagnostics.push(PmiDiagnostic {
                kind: PmiDiagnosticKind::AmbiguousFaceBinding,
                entity: Some(aspect),
                message:
                    "shape_aspect identifies multiple faces; one-face PMI index is not guessed"
                        .into(),
            });
        }
        model.shape_aspects.push(ShapeAspect {
            entity: aspect,
            name: args
                .first()
                .map_or(Ok(String::new()), |v| text(v, aspect, "shape_aspect name"))?,
            description: args.get(1).map_or(Ok(String::new()), |v| {
                text(v, aspect, "shape_aspect description")
            })?,
            face_id,
            candidate_face_ids,
            related_aspects,
        });
    }

    for &datum in &aspect_ids {
        let parts = components(entities, datum)
            .map_err(|e| semantic_error(PmiErrorKind::MalformedPart21, datum, e.message))?;
        if !parts
            .iter()
            .any(|(ty, _)| matches!(*ty, "DATUM" | "DATUM_FEATURE"))
        {
            continue;
        }
        let face_id = model
            .shape_aspect_to_face
            .get(&datum)
            .copied()
            .ok_or_else(|| {
                semantic_error(
                    PmiErrorKind::DanglingFace,
                    datum,
                    "datum has no unique persistent face binding",
                )
            })?;
        let datum_entity = parts.iter().any(|(ty, _)| *ty == "DATUM").then_some(datum);
        model.datum_features.push(DatumFeature {
            entity: datum,
            datum_entity,
            label: datum_label(entities, datum)?,
            shape_aspect: datum,
            face_id,
        });
    }

    for (&id, _) in entities {
        let parts = components(entities, id)
            .map_err(|e| semantic_error(PmiErrorKind::MalformedPart21, id, e.message))?;
        let tolerance_part = parts
            .iter()
            .find(|(ty, _)| GeometricToleranceKind::from_step(ty).is_some());
        let Some((kind_name, _)) = tolerance_part else {
            if parts.iter().any(|(ty, _)| *ty == "GEOMETRIC_TOLERANCE") {
                return Err(semantic_error(
                    PmiErrorKind::UnsupportedEntity,
                    id,
                    "generic or unsupported geometric_tolerance subtype",
                ));
            }
            continue;
        };
        let kind = GeometricToleranceKind::from_step(kind_name).unwrap();
        let (_, args) = parts
            .iter()
            .find(|(ty, _)| *ty == "GEOMETRIC_TOLERANCE")
            .or_else(|| parts.iter().find(|(ty, _)| *ty == *kind_name))
            .ok_or_else(|| {
                semantic_error(
                    PmiErrorKind::MalformedSemantic,
                    id,
                    "geometric tolerance subtype lacks GEOMETRIC_TOLERANCE component",
                )
            })?;
        if args.len() != 4 {
            return Err(semantic_error(
                PmiErrorKind::MalformedSemantic,
                id,
                "GEOMETRIC_TOLERANCE requires four arguments",
            ));
        }
        let shape_aspect = one_ref(&args[3], "toleranced shape_aspect")
            .map_err(|e| semantic_error(PmiErrorKind::MalformedSemantic, id, e.message))?;
        let face_id = model.shape_aspect_to_face.get(&shape_aspect).copied().ok_or_else(|| semantic_error(PmiErrorKind::DanglingFace, id, format!("toleranced shape_aspect #{shape_aspect} has no unique persistent face binding")))?;
        let magnitude = measure(entities, &args[2], id)?;
        if !magnitude.is_finite() || magnitude < 0.0 {
            return Err(semantic_error(
                PmiErrorKind::MalformedSemantic,
                id,
                "tolerance magnitude must be finite and nonnegative",
            ));
        }
        let modifiers = parts
            .iter()
            .find(|(ty, _)| *ty == "GEOMETRIC_TOLERANCE_WITH_MODIFIERS")
            .map_or(Ok(Vec::new()), |(_, args)| {
                args.first()
                    .map_or(Ok(Vec::new()), |v| PmiModifier::parse(v, id))
            })?;
        let datum_references = parts
            .iter()
            .find(|(ty, _)| *ty == "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE")
            .map_or(Ok(Vec::new()), |(_, args)| {
                args.first().map_or(Ok(Vec::new()), |v| {
                    tolerance_datums(entities, v, &model.shape_aspect_to_face, id)
                })
            })?;
        model.tolerances.push(GeometricTolerance {
            entity: id,
            name: text(&args[0], id, "tolerance name")?,
            description: text(&args[1], id, "tolerance description")?,
            kind,
            magnitude,
            shape_aspect,
            face_id,
            modifiers,
            datum_references,
        });
    }

    let (polylines, presentation_roots) = imported_presentation(entities)?;
    model.presentation_polylines = polylines;
    if model.tolerances.is_empty()
        && model.datum_features.is_empty()
        && (!model.presentation_polylines.is_empty() || !presentation_roots.is_empty())
    {
        model.diagnostics.push(PmiDiagnostic {
            kind: PmiDiagnosticKind::PresentationOnly,
            entity: None,
            message: "PMI is a picture only: no machine-readable semantic GD&T was imported".into(),
        });
    }
    for line in &model.presentation_polylines {
        if line.semantic_entity.is_none() && !model.tolerances.is_empty() {
            model.diagnostics.push(PmiDiagnostic {
                kind: PmiDiagnosticKind::PresentationWithoutSemanticLink,
                entity: Some(line.source_entity),
                message: "presentation polyline has no explicit semantic PMI link".into(),
            });
        }
    }
    let linked = model
        .presentation_polylines
        .iter()
        .filter_map(|line| line.semantic_entity)
        .collect::<BTreeSet<_>>();
    for semantic in model
        .tolerances
        .iter()
        .map(|t| t.entity)
        .chain(model.datum_features.iter().map(|d| d.entity))
    {
        if !linked.contains(&semantic) {
            model.diagnostics.push(PmiDiagnostic {
                kind: PmiDiagnosticKind::SemanticWithoutPresentation,
                entity: Some(semantic),
                message: "semantic PMI has no explicitly linked presentation polyline".into(),
            });
        }
    }
    for root in presentation_roots {
        let is_tessellated = components(entities, root)
            .ok()
            .is_some_and(|parts| parts.iter().any(|(ty, _)| ty.contains("TESSELLATED")));
        if is_tessellated {
            model.diagnostics.push(PmiDiagnostic { kind: PmiDiagnosticKind::UnsupportedPresentation, entity: Some(root), message: "tessellated PMI presentation is retained only as a diagnostic; no semantic value is inferred".into() });
        }
    }
    model
        .diagnostics
        .sort_by(|a, b| (&a.kind, a.entity, &a.message).cmp(&(&b.kind, b.entity, &b.message)));
    model.validate()?;
    Ok(model)
}
