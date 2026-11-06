//! AP242 semantic GD&T and its deliberately separate graphic presentation.
//!
//! This module is independent of the B-rep interchange routes.  The caller
//! supplies the `ADVANCED_FACE` entity to persistent topology id mapping that
//! its geometry importer produced.  Presentation-only input is retained as
//! lines plus diagnostics; it is never promoted to semantic PMI.

use crate::TopoId;
use crate::step_part21::{Entity, Value, call, components, list, number, one_ref, parse, refs};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const STEP_PMI_CAPABILITY: &str = "step-ap242-semantic-pmi/1";
pub const STEP_PMI_JSON_SCHEMA: &str = "open-scad-viewer.step-pmi.v1";
const MAX_PMI_DEPTH: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PmiErrorKind {
    MalformedPart21,
    MalformedSemantic,
    UnsupportedEntity,
    UnsupportedModifier,
    DanglingFace,
    DanglingDatum,
    InvalidPresentation,
    MissingLayout,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PmiError {
    pub kind: PmiErrorKind,
    pub entity: Option<usize>,
    pub message: String,
}

impl PmiError {
    fn new(kind: PmiErrorKind, entity: Option<usize>, message: impl Into<String>) -> Self {
        Self {
            kind,
            entity,
            message: message.into(),
        }
    }
}

impl fmt::Display for PmiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(id) = self.entity {
            write!(f, "PMI #{}: {}", id, self.message)
        } else {
            f.write_str(&self.message)
        }
    }
}

impl std::error::Error for PmiError {}

pub type PmiResult<T> = std::result::Result<T, PmiError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GeometricToleranceKind {
    Angularity,
    CircularRunout,
    Roundness,
    Coaxiality,
    Concentricity,
    Cylindricity,
    Flatness,
    Parallelism,
    Perpendicularity,
    Position,
    LineProfile,
    SurfaceProfile,
    Straightness,
    Symmetry,
    TotalRunout,
}

impl GeometricToleranceKind {
    pub fn as_step_entity(self) -> &'static str {
        match self {
            Self::Angularity => "ANGULARITY_TOLERANCE",
            Self::CircularRunout => "CIRCULAR_RUNOUT_TOLERANCE",
            Self::Roundness => "ROUNDNESS_TOLERANCE",
            Self::Coaxiality => "COAXIALITY_TOLERANCE",
            Self::Concentricity => "CONCENTRICITY_TOLERANCE",
            Self::Cylindricity => "CYLINDRICITY_TOLERANCE",
            Self::Flatness => "FLATNESS_TOLERANCE",
            Self::Parallelism => "PARALLELISM_TOLERANCE",
            Self::Perpendicularity => "PERPENDICULARITY_TOLERANCE",
            Self::Position => "POSITION_TOLERANCE",
            Self::LineProfile => "LINE_PROFILE_TOLERANCE",
            Self::SurfaceProfile => "SURFACE_PROFILE_TOLERANCE",
            Self::Straightness => "STRAIGHTNESS_TOLERANCE",
            Self::Symmetry => "SYMMETRY_TOLERANCE",
            Self::TotalRunout => "TOTAL_RUNOUT_TOLERANCE",
        }
    }

    fn from_step(entity: &str) -> Option<Self> {
        Some(match entity {
            "ANGULARITY_TOLERANCE" => Self::Angularity,
            "CIRCULAR_RUNOUT_TOLERANCE" => Self::CircularRunout,
            "ROUNDNESS_TOLERANCE" | "CIRCULARITY_TOLERANCE" => Self::Roundness,
            "COAXIALITY_TOLERANCE" => Self::Coaxiality,
            "CONCENTRICITY_TOLERANCE" => Self::Concentricity,
            "CYLINDRICITY_TOLERANCE" => Self::Cylindricity,
            "FLATNESS_TOLERANCE" => Self::Flatness,
            "PARALLELISM_TOLERANCE" => Self::Parallelism,
            "PERPENDICULARITY_TOLERANCE" => Self::Perpendicularity,
            "POSITION_TOLERANCE" => Self::Position,
            "LINE_PROFILE_TOLERANCE" => Self::LineProfile,
            "SURFACE_PROFILE_TOLERANCE" => Self::SurfaceProfile,
            "STRAIGHTNESS_TOLERANCE" => Self::Straightness,
            "SYMMETRY_TOLERANCE" => Self::Symmetry,
            "TOTAL_RUNOUT_TOLERANCE" => Self::TotalRunout,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PmiModifier {
    MaximumMaterialRequirement,
    LeastMaterialRequirement,
    RegardlessOfFeatureSize,
    FreeState,
    ProjectedToleranceZone,
    StatisticalTolerance,
    Basic,
    Translation,
    ContactingFeature,
    DistanceVariable,
    MinorDiameter,
    MajorDiameter,
    PitchDiameter,
    DegreeOfFreedomConstraint(char),
    OtherStandard(String),
}

impl PmiModifier {
    fn parse(value: &Value, entity: usize) -> PmiResult<Vec<Self>> {
        match value {
            Value::Omitted | Value::Derived => Ok(vec![]),
            Value::Enum(name) => Self::from_enum(name, entity).map(|m| vec![m]),
            Value::List(items) => {
                let mut out = Vec::new();
                for item in items {
                    out.extend(Self::parse(item, entity)?);
                }
                Ok(out)
            }
            Value::Call(name, args)
                if name == "SIMPLE_DATUM_REFERENCE_MODIFIER" && args.len() == 1 =>
            {
                Self::parse(&args[0], entity)
            }
            Value::Call(name, _) => Err(PmiError::new(
                PmiErrorKind::UnsupportedModifier,
                Some(entity),
                format!("modifier {name} with a value is not represented losslessly"),
            )),
            _ => Err(PmiError::new(
                PmiErrorKind::MalformedSemantic,
                Some(entity),
                "malformed modifier",
            )),
        }
    }

    fn from_enum(name: &str, entity: usize) -> PmiResult<Self> {
        Ok(match name {
            "MAXIMUM_MATERIAL_REQUIREMENT" | "MAXIMUM_MATERIAL_CONDITION" | "M" => {
                Self::MaximumMaterialRequirement
            }
            "LEAST_MATERIAL_REQUIREMENT" | "LEAST_MATERIAL_CONDITION" | "L" => {
                Self::LeastMaterialRequirement
            }
            "REGARDLESS_OF_FEATURE_SIZE" | "RFS" | "S" => Self::RegardlessOfFeatureSize,
            "FREE_STATE" | "F" => Self::FreeState,
            "PROJECTED_TOLERANCE_ZONE" | "P" => Self::ProjectedToleranceZone,
            "STATISTICAL_TOLERANCE" | "ST" => Self::StatisticalTolerance,
            "BASIC" => Self::Basic,
            "TRANSLATION" => Self::Translation,
            "CONTACTING_FEATURE" => Self::ContactingFeature,
            "DISTANCE_VARIABLE" => Self::DistanceVariable,
            "MINOR_DIAMETER" => Self::MinorDiameter,
            "MAJOR_DIAMETER" => Self::MajorDiameter,
            "PITCH_DIAMETER" => Self::PitchDiameter,
            "DEGREE_OF_FREEDOM_CONSTRAINT_X" => Self::DegreeOfFreedomConstraint('X'),
            "DEGREE_OF_FREEDOM_CONSTRAINT_Y" => Self::DegreeOfFreedomConstraint('Y'),
            "DEGREE_OF_FREEDOM_CONSTRAINT_Z" => Self::DegreeOfFreedomConstraint('Z'),
            "DEGREE_OF_FREEDOM_CONSTRAINT_U" => Self::DegreeOfFreedomConstraint('U'),
            "DEGREE_OF_FREEDOM_CONSTRAINT_V" => Self::DegreeOfFreedomConstraint('V'),
            "DEGREE_OF_FREEDOM_CONSTRAINT_W" => Self::DegreeOfFreedomConstraint('W'),
            other if other.bytes().all(|b| b.is_ascii_uppercase() || b == b'_') => {
                Self::OtherStandard(other.into())
            }
            _ => {
                return Err(PmiError::new(
                    PmiErrorKind::UnsupportedModifier,
                    Some(entity),
                    format!("unknown modifier .{name}."),
                ));
            }
        })
    }

    pub fn symbol(&self) -> String {
        match self {
            Self::MaximumMaterialRequirement => "M".into(),
            Self::LeastMaterialRequirement => "L".into(),
            Self::RegardlessOfFeatureSize => "S".into(),
            Self::FreeState => "F".into(),
            Self::ProjectedToleranceZone => "P".into(),
            Self::StatisticalTolerance => "ST".into(),
            Self::Basic => "BASIC".into(),
            Self::Translation => "T".into(),
            Self::ContactingFeature => "CF".into(),
            Self::DistanceVariable => "DV".into(),
            Self::MinorDiameter => "LD".into(),
            Self::MajorDiameter => "MD".into(),
            Self::PitchDiameter => "PD".into(),
            Self::DegreeOfFreedomConstraint(c) => c.to_string(),
            Self::OtherStandard(name) => name.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DatumPrecedence {
    Primary,
    Secondary,
    Tertiary,
    Subsequent(u32),
    Unspecified,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShapeAspect {
    pub entity: usize,
    pub name: String,
    pub description: String,
    pub face_id: Option<TopoId>,
    pub candidate_face_ids: Vec<TopoId>,
    pub related_aspects: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DatumTarget {
    pub entity: usize,
    pub label: String,
    pub feature_entity: usize,
    pub face_id: TopoId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DatumReference {
    pub precedence: DatumPrecedence,
    /// More than one target is a composite datum at one precedence.
    pub datums: Vec<DatumTarget>,
    pub modifiers: Vec<PmiModifier>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DatumFeature {
    pub entity: usize,
    pub datum_entity: Option<usize>,
    pub label: String,
    pub shape_aspect: usize,
    pub face_id: TopoId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeometricTolerance {
    pub entity: usize,
    pub name: String,
    pub description: String,
    pub kind: GeometricToleranceKind,
    pub magnitude: f64,
    pub shape_aspect: usize,
    pub face_id: TopoId,
    pub modifiers: Vec<PmiModifier>,
    pub datum_references: Vec<DatumReference>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShapeAspectRelationship {
    pub entity: usize,
    pub name: String,
    pub description: String,
    pub relating: usize,
    pub related: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PmiDiagnosticKind {
    PresentationOnly,
    PresentationWithoutSemanticLink,
    SemanticWithoutPresentation,
    UnsupportedPresentation,
    AmbiguousFaceBinding,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PmiDiagnostic {
    pub kind: PmiDiagnosticKind,
    pub entity: Option<usize>,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PmiPolylineRole {
    Imported,
    FeatureControlFrame,
    DatumFrame,
    Leader,
    Text,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PmiPolyline {
    pub source_entity: usize,
    pub semantic_entity: Option<usize>,
    pub role: PmiPolylineRole,
    pub points: Vec<[f64; 3]>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StepPmiModel {
    pub shape_aspects: Vec<ShapeAspect>,
    pub shape_aspect_relationships: Vec<ShapeAspectRelationship>,
    /// The supported one-face index. Multi-face aspects remain explicit in
    /// `shape_aspects` and are rejected when used as a required face binding.
    pub shape_aspect_to_face: BTreeMap<usize, TopoId>,
    pub tolerances: Vec<GeometricTolerance>,
    pub datum_features: Vec<DatumFeature>,
    pub presentation_polylines: Vec<PmiPolyline>,
    pub diagnostics: Vec<PmiDiagnostic>,
}

impl StepPmiModel {
    pub fn is_presentation_only(&self) -> bool {
        self.tolerances.is_empty()
            && self.datum_features.is_empty()
            && (!self.presentation_polylines.is_empty()
                || self.diagnostics.iter().any(|diagnostic| {
                    matches!(
                        diagnostic.kind,
                        PmiDiagnosticKind::PresentationOnly
                            | PmiDiagnosticKind::UnsupportedPresentation
                    )
                }))
    }

    pub fn validate(&self) -> PmiResult<()> {
        for tolerance in &self.tolerances {
            if !tolerance.magnitude.is_finite() || tolerance.magnitude < 0.0 {
                return Err(PmiError::new(
                    PmiErrorKind::MalformedSemantic,
                    Some(tolerance.entity),
                    "tolerance magnitude must be finite and nonnegative",
                ));
            }
            if self.shape_aspect_to_face.get(&tolerance.shape_aspect) != Some(&tolerance.face_id) {
                return Err(PmiError::new(
                    PmiErrorKind::DanglingFace,
                    Some(tolerance.entity),
                    "tolerance face is absent from the shape_aspect index",
                ));
            }
        }
        for datum in &self.datum_features {
            if self.shape_aspect_to_face.get(&datum.shape_aspect) != Some(&datum.face_id) {
                return Err(PmiError::new(
                    PmiErrorKind::DanglingFace,
                    Some(datum.entity),
                    "datum face is absent from the shape_aspect index",
                ));
            }
        }
        for line in &self.presentation_polylines {
            validate_line(line)?;
        }
        Ok(())
    }

    /// Canonical, dependency-free JSON: sorted keys and entities, escaped
    /// strings, and a stable 17-digit scientific representation for reals.
    pub fn to_json(&self) -> PmiResult<String> {
        self.validate()?;
        Ok(Json::from(self).stringify())
    }
}

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
            if matches!(&entity.value, Value::Call(name, args) if name == "POLYLINE") {
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PmiPresentationLayout {
    /// Point on the toleranced/datum geometry for the leader.
    pub anchor: [f64; 3],
    /// Lower-left frame origin in model coordinates, in the XY plane.
    pub frame_origin: [f64; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct PmiPresentation {
    pub polylines: Vec<PmiPolyline>,
    /// Complete deterministic Part 21 presentation document. It deliberately
    /// contains no invented semantic copy; `semantic_entity` remains the link
    /// for the parent STEP writer when it merges the two graphs.
    pub part21: String,
}

fn validate_point(point: &[f64; 3], entity: Option<usize>) -> PmiResult<()> {
    if point.iter().all(|x| x.is_finite()) {
        Ok(())
    } else {
        Err(PmiError::new(
            PmiErrorKind::InvalidPresentation,
            entity,
            "presentation coordinates must be finite",
        ))
    }
}

fn validate_line(line: &PmiPolyline) -> PmiResult<()> {
    if line.points.len() < 2 {
        return Err(PmiError::new(
            PmiErrorKind::InvalidPresentation,
            Some(line.source_entity),
            "presentation polyline requires at least two points",
        ));
    }
    for point in &line.points {
        validate_point(point, Some(line.source_entity))?;
    }
    Ok(())
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn stroke(character: char) -> &'static [[f64; 2]] {
    // A compact deterministic single-stroke font. Unknown characters are a
    // visible box rather than silently disappearing from the control frame.
    match character {
        'A' => &[[0., 0.], [0.5, 1.], [1., 0.], [0.75, 0.4], [0.25, 0.4]],
        'B' => &[
            [0., 0.],
            [0., 1.],
            [0.7, 1.],
            [1., 0.75],
            [0.7, 0.55],
            [0., 0.55],
            [0.75, 0.55],
            [1., 0.25],
            [0.7, 0.],
            [0., 0.],
        ],
        'C' => &[[1., 1.], [0., 1.], [0., 0.], [1., 0.]],
        'D' => &[
            [0., 0.],
            [0., 1.],
            [0.6, 1.],
            [1., 0.6],
            [1., 0.4],
            [0.6, 0.],
            [0., 0.],
        ],
        'E' => &[
            [1., 1.],
            [0., 1.],
            [0., 0.],
            [1., 0.],
            [1., 0.15],
            [0., 0.15],
            [0., 0.5],
            [0.75, 0.5],
        ],
        'F' => &[
            [0., 0.],
            [0., 1.],
            [1., 1.],
            [1., 0.85],
            [0., 0.85],
            [0., 0.5],
            [0.75, 0.5],
        ],
        'G' => &[
            [1., 1.],
            [0., 1.],
            [0., 0.],
            [1., 0.],
            [1., 0.5],
            [0.55, 0.5],
        ],
        'H' => &[[0., 0.], [0., 1.], [0., 0.5], [1., 0.5], [1., 1.], [1., 0.]],
        'I' => &[[0., 1.], [1., 1.], [0.5, 1.], [0.5, 0.], [1., 0.], [0., 0.]],
        'J' => &[[1., 1.], [1., 0.25], [0.75, 0.], [0.25, 0.], [0., 0.25]],
        'K' => &[
            [0., 0.],
            [0., 1.],
            [0., 0.45],
            [1., 1.],
            [0., 0.45],
            [1., 0.],
        ],
        'L' => &[[0., 1.], [0., 0.], [1., 0.]],
        'M' => &[[0., 0.], [0., 1.], [0.5, 0.45], [1., 1.], [1., 0.]],
        'N' => &[[0., 0.], [0., 1.], [1., 0.], [1., 1.]],
        'O' => &[[0., 0.], [0., 1.], [1., 1.], [1., 0.], [0., 0.]],
        'P' => &[
            [0., 0.],
            [0., 1.],
            [0.75, 1.],
            [1., 0.75],
            [0.75, 0.5],
            [0., 0.5],
        ],
        'Q' => &[
            [0., 0.],
            [0., 1.],
            [1., 1.],
            [1., 0.],
            [0., 0.],
            [0.5, 0.4],
            [1.1, -0.1],
        ],
        'R' => &[
            [0., 0.],
            [0., 1.],
            [0.75, 1.],
            [1., 0.75],
            [0.75, 0.5],
            [0., 0.5],
            [1., 0.],
        ],
        'S' => &[[1., 1.], [0., 1.], [0., 0.5], [1., 0.5], [1., 0.], [0., 0.]],
        'T' => &[[0., 1.], [1., 1.], [0.5, 1.], [0.5, 0.]],
        'U' => &[[0., 1.], [0., 0.], [1., 0.], [1., 1.]],
        'V' => &[[0., 1.], [0.5, 0.], [1., 1.]],
        'W' => &[[0., 1.], [0.2, 0.], [0.5, 0.55], [0.8, 0.], [1., 1.]],
        'X' => &[[0., 0.], [1., 1.], [0.5, 0.5], [0., 1.], [1., 0.]],
        'Y' => &[[0., 1.], [0.5, 0.5], [1., 1.], [0.5, 0.5], [0.5, 0.]],
        'Z' => &[[0., 1.], [1., 1.], [0., 0.], [1., 0.]],
        '0' => &[[0., 0.], [0., 1.], [1., 1.], [1., 0.], [0., 0.], [1., 1.]],
        '1' => &[[0., 0.75], [0.5, 1.], [0.5, 0.], [0., 0.], [1., 0.]],
        '2' => &[[0., 1.], [1., 1.], [1., 0.5], [0., 0.], [1., 0.]],
        '3' => &[[0., 1.], [1., 1.], [0.5, 0.5], [1., 0.], [0., 0.]],
        '4' => &[[0., 1.], [0., 0.5], [1., 0.5], [0.75, 1.], [0.75, 0.]],
        '5' => &[[1., 1.], [0., 1.], [0., 0.5], [1., 0.5], [1., 0.], [0., 0.]],
        '6' => &[[1., 1.], [0., 1.], [0., 0.], [1., 0.], [1., 0.5], [0., 0.5]],
        '7' => &[[0., 1.], [1., 1.], [0.25, 0.]],
        '8' => &[
            [0., 0.],
            [0., 1.],
            [1., 1.],
            [1., 0.],
            [0., 0.],
            [0., 0.5],
            [1., 0.5],
            [1., 1.],
        ],
        '9' => &[[1., 0.5], [0., 0.5], [0., 1.], [1., 1.], [1., 0.], [0., 0.]],
        '.' => &[[0.45, 0.], [0.55, 0.], [0.55, 0.1], [0.45, 0.1], [0.45, 0.]],
        '-' => &[[0., 0.5], [1., 0.5]],
        '+' => &[[0., 0.5], [1., 0.5], [0.5, 0.5], [0.5, 1.], [0.5, 0.]],
        '|' => &[[0.5, 0.], [0.5, 1.]],
        '/' => &[[0., 0.], [1., 1.]],
        _ => &[[0., 0.], [0., 1.], [1., 1.], [1., 0.], [0., 0.], [1., 1.]],
    }
}

/// Generates deterministic vector feature-control/datum frames. Layouts are
/// explicit so this function never invents face geometry or a font dependency.
pub fn export_step_pmi_presentation(
    model: &StepPmiModel,
    layouts: &BTreeMap<usize, PmiPresentationLayout>,
    character_height: f64,
) -> PmiResult<PmiPresentation> {
    model.validate()?;
    if !character_height.is_finite() || character_height <= 0.0 {
        return Err(PmiError::new(
            PmiErrorKind::InvalidPresentation,
            None,
            "character_height must be finite and positive",
        ));
    }
    let mut polylines = Vec::new();
    for tolerance in &model.tolerances {
        let layout = layouts.get(&tolerance.entity).ok_or_else(|| {
            PmiError::new(
                PmiErrorKind::MissingLayout,
                Some(tolerance.entity),
                "missing tolerance presentation layout",
            )
        })?;
        validate_point(&layout.anchor, Some(tolerance.entity))?;
        validate_point(&layout.frame_origin, Some(tolerance.entity))?;
        let mut label = format!(
            "{} {}",
            tolerance
                .kind
                .as_step_entity()
                .trim_end_matches("_TOLERANCE")
                .replace('_', " "),
            canonical_real(tolerance.magnitude)
        );
        for modifier in &tolerance.modifiers {
            label.push('|');
            label.push_str(&modifier.symbol());
        }
        for reference in &tolerance.datum_references {
            label.push('|');
            label.push_str(
                &reference
                    .datums
                    .iter()
                    .map(|d| d.label.as_str())
                    .collect::<Vec<_>>()
                    .join("-"),
            );
            for modifier in &reference.modifiers {
                label.push('|');
                label.push_str(&modifier.symbol());
            }
        }
        emit_frame(
            &mut polylines,
            tolerance.entity,
            PmiPolylineRole::FeatureControlFrame,
            layout,
            &label,
            character_height,
        );
    }
    for datum in &model.datum_features {
        let layout = layouts.get(&datum.entity).ok_or_else(|| {
            PmiError::new(
                PmiErrorKind::MissingLayout,
                Some(datum.entity),
                "missing datum presentation layout",
            )
        })?;
        validate_point(&layout.anchor, Some(datum.entity))?;
        validate_point(&layout.frame_origin, Some(datum.entity))?;
        emit_frame(
            &mut polylines,
            datum.entity,
            PmiPolylineRole::DatumFrame,
            layout,
            &datum.label,
            character_height,
        );
    }
    polylines.sort_by_key(|line| (line.semantic_entity, line.role));
    for line in &polylines {
        validate_line(line)?;
    }
    let part21 = presentation_part21(&polylines);
    Ok(PmiPresentation { polylines, part21 })
}

fn emit_frame(
    out: &mut Vec<PmiPolyline>,
    entity: usize,
    frame_role: PmiPolylineRole,
    layout: &PmiPresentationLayout,
    label: &str,
    height: f64,
) {
    let source = out.len() + 1;
    let width = (label.chars().count() as f64 * 0.8 + 0.4) * height;
    let o = layout.frame_origin;
    let corners = [
        o,
        add(o, [width, 0., 0.]),
        add(o, [width, 2. * height, 0.]),
        add(o, [0., 2. * height, 0.]),
        o,
    ];
    out.push(PmiPolyline {
        source_entity: source,
        semantic_entity: Some(entity),
        role: frame_role,
        points: corners.to_vec(),
    });
    out.push(PmiPolyline {
        source_entity: source + 1,
        semantic_entity: Some(entity),
        role: PmiPolylineRole::Leader,
        points: vec![
            layout.anchor,
            add(o, [-0.5 * height, height, 0.]),
            add(o, [0., height, 0.]),
        ],
    });
    let mut x = 0.2 * height;
    let mut source_entity = source + 2;
    for character in label.chars() {
        if character != ' ' {
            let points = stroke(character.to_ascii_uppercase())
                .iter()
                .map(|p| {
                    add(
                        o,
                        [x + p[0] * 0.6 * height, 0.5 * height + p[1] * height, 0.],
                    )
                })
                .collect();
            out.push(PmiPolyline {
                source_entity,
                semantic_entity: Some(entity),
                role: PmiPolylineRole::Text,
                points,
            });
        }
        source_entity += 1;
        x += 0.8 * height;
    }
}

fn step_string(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn canonical_real(value: f64) -> String {
    let value = if value == 0.0 { 0.0 } else { value };
    format!("{value:.17e}").replace('e', "E")
}

fn presentation_part21(lines: &[PmiPolyline]) -> String {
    let mut next = 1usize;
    let mut body = String::new();
    for line in lines {
        let mut point_ids = Vec::new();
        for point in &line.points {
            point_ids.push(next);
            body.push_str(&format!(
                "#{}=CARTESIAN_POINT('',({}, {}, {}));\n",
                next,
                canonical_real(point[0]),
                canonical_real(point[1]),
                canonical_real(point[2])
            ));
            next += 1;
        }
        let polyline = next;
        next += 1;
        body.push_str(&format!(
            "#{}=POLYLINE('',({}));\n",
            polyline,
            point_ids
                .iter()
                .map(|id| format!("#{id}"))
                .collect::<Vec<_>>()
                .join(",")
        ));
        body.push_str(&format!(
            "#{}=ANNOTATION_CURVE_OCCURRENCE({},'',#{});\n",
            next,
            step_string(&format!(
                "PMI semantic #{}",
                line.semantic_entity.unwrap_or(line.source_entity)
            )),
            polyline
        ));
        next += 1;
    }
    format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('DETERMINISTIC PMI PRESENTATION'),'2;1');\nFILE_NAME('pmi-presentation.step','1970-01-01T00:00:00',(''),(''),'open-scad-viewer','open-scad-viewer','');\nFILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF'));\nENDSEC;\nDATA;\n{body}ENDSEC;\nEND-ISO-10303-21;\n"
    )
}

// Minimal canonical JSON tree. BTreeMap fixes key order; arrays retain the
// importer's STEP-id order. No serde or codec feature is required.
#[derive(Clone, Debug)]
enum Json {
    Null,
    Number(String),
    String(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

impl Json {
    fn string(value: impl Into<String>) -> Self {
        Self::String(value.into())
    }
    fn usize(value: usize) -> Self {
        Self::Number(value.to_string())
    }
    fn real(value: f64) -> Self {
        Self::Number(canonical_real(value))
    }
    fn array<T>(values: impl IntoIterator<Item = T>, convert: impl FnMut(T) -> Json) -> Self {
        Self::Array(values.into_iter().map(convert).collect())
    }
    fn object(fields: impl IntoIterator<Item = (&'static str, Json)>) -> Self {
        Self::Object(
            fields
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect(),
        )
    }

    fn stringify(&self) -> String {
        match self {
            Self::Null => "null".into(),
            Self::Number(value) => value.clone(),
            Self::String(value) => {
                let mut out = String::from("\"");
                for c in value.chars() {
                    match c {
                        '\"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        '\n' => out.push_str("\\n"),
                        '\r' => out.push_str("\\r"),
                        '\t' => out.push_str("\\t"),
                        c if c <= '\u{1f}' => out.push_str(&format!("\\u{:04x}", c as u32)),
                        c => out.push(c),
                    }
                }
                out.push('\"');
                out
            }
            Self::Array(values) => format!(
                "[{}]",
                values
                    .iter()
                    .map(Self::stringify)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Self::Object(values) => format!(
                "{{{}}}",
                values
                    .iter()
                    .map(|(key, value)| format!(
                        "{}:{}",
                        Self::string(key).stringify(),
                        value.stringify()
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }
}

fn modifiers_json(modifiers: &[PmiModifier]) -> Json {
    Json::array(modifiers, |m| Json::string(m.symbol()))
}

fn precedence_json(precedence: DatumPrecedence) -> Json {
    Json::string(match precedence {
        DatumPrecedence::Primary => "primary".into(),
        DatumPrecedence::Secondary => "secondary".into(),
        DatumPrecedence::Tertiary => "tertiary".into(),
        DatumPrecedence::Subsequent(n) => format!("subsequent:{n}"),
        DatumPrecedence::Unspecified => "unspecified".into(),
    })
}

impl From<&StepPmiModel> for Json {
    fn from(model: &StepPmiModel) -> Self {
        Self::object([
            ("capability", Self::string(STEP_PMI_CAPABILITY)),
            (
                "datumFeatures",
                Self::array(&model.datum_features, |datum| {
                    Self::object([
                        (
                            "datumEntity",
                            datum.datum_entity.map_or(Self::Null, Self::usize),
                        ),
                        ("entity", Self::usize(datum.entity)),
                        ("faceId", Self::string(datum.face_id.to_string())),
                        ("label", Self::string(&datum.label)),
                        ("shapeAspect", Self::usize(datum.shape_aspect)),
                    ])
                }),
            ),
            (
                "diagnostics",
                Self::array(&model.diagnostics, |diagnostic| {
                    Self::object([
                        ("entity", diagnostic.entity.map_or(Self::Null, Self::usize)),
                        ("kind", Self::string(format!("{:?}", diagnostic.kind))),
                        ("message", Self::string(&diagnostic.message)),
                    ])
                }),
            ),
            (
                "presentationPolylines",
                Self::array(&model.presentation_polylines, |line| {
                    Self::object([
                        (
                            "points",
                            Self::array(&line.points, |point| {
                                Self::array(point, |x| Self::real(*x))
                            }),
                        ),
                        ("role", Self::string(format!("{:?}", line.role))),
                        (
                            "semanticEntity",
                            line.semantic_entity.map_or(Self::Null, Self::usize),
                        ),
                        ("sourceEntity", Self::usize(line.source_entity)),
                    ])
                }),
            ),
            ("schema", Self::string(STEP_PMI_JSON_SCHEMA)),
            (
                "shapeAspectRelationships",
                Self::array(&model.shape_aspect_relationships, |r| {
                    Self::object([
                        ("description", Self::string(&r.description)),
                        ("entity", Self::usize(r.entity)),
                        ("name", Self::string(&r.name)),
                        ("related", Self::usize(r.related)),
                        ("relating", Self::usize(r.relating)),
                    ])
                }),
            ),
            (
                "shapeAspectToFace",
                Self::Object(
                    model
                        .shape_aspect_to_face
                        .iter()
                        .map(|(aspect, face)| (aspect.to_string(), Self::string(face.to_string())))
                        .collect(),
                ),
            ),
            (
                "shapeAspects",
                Self::array(&model.shape_aspects, |aspect| {
                    Self::object([
                        (
                            "candidateFaceIds",
                            Self::array(&aspect.candidate_face_ids, |id| {
                                Self::string(id.to_string())
                            }),
                        ),
                        ("description", Self::string(&aspect.description)),
                        ("entity", Self::usize(aspect.entity)),
                        (
                            "faceId",
                            aspect
                                .face_id
                                .map_or(Self::Null, |id| Self::string(id.to_string())),
                        ),
                        ("name", Self::string(&aspect.name)),
                        (
                            "relatedAspects",
                            Self::array(&aspect.related_aspects, |id| Self::usize(*id)),
                        ),
                    ])
                }),
            ),
            (
                "tolerances",
                Self::array(&model.tolerances, |tolerance| {
                    Self::object([
                        (
                            "datumReferences",
                            Self::array(&tolerance.datum_references, |reference| {
                                Self::object([
                                    (
                                        "datums",
                                        Self::array(&reference.datums, |datum| {
                                            Self::object([
                                                ("entity", Self::usize(datum.entity)),
                                                ("faceId", Self::string(datum.face_id.to_string())),
                                                (
                                                    "featureEntity",
                                                    Self::usize(datum.feature_entity),
                                                ),
                                                ("label", Self::string(&datum.label)),
                                            ])
                                        }),
                                    ),
                                    ("modifiers", modifiers_json(&reference.modifiers)),
                                    ("precedence", precedence_json(reference.precedence)),
                                ])
                            }),
                        ),
                        ("description", Self::string(&tolerance.description)),
                        ("entity", Self::usize(tolerance.entity)),
                        ("faceId", Self::string(tolerance.face_id.to_string())),
                        ("kind", Self::string(tolerance.kind.as_step_entity())),
                        ("magnitude", Self::real(tolerance.magnitude)),
                        ("modifiers", modifiers_json(&tolerance.modifiers)),
                        ("name", Self::string(&tolerance.name)),
                        ("shapeAspect", Self::usize(tolerance.shape_aspect)),
                    ])
                }),
            ),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brep_topology::TopoKind;

    fn face(label: &str) -> TopoId {
        TopoId::derive(
            TopoKind::Face,
            "step-pmi-test",
            label,
            "face",
            label.as_bytes(),
        )
    }

    fn semantic_fixture() -> String {
        r#"ISO-10303-21;
DATA;
#1=PRODUCT_DEFINITION_SHAPE('','',#2);
#2=PRODUCT_DEFINITION('','',#3,#4);
#3=PRODUCT_DEFINITION_FORMATION('','',#5);
#4=PRODUCT_DEFINITION_CONTEXT('part',#6,'design');
#5=PRODUCT('p','p','',(#6));
#6=PRODUCT_CONTEXT('',#7,'mechanical');
#7=APPLICATION_CONTEXT('managed model based 3d engineering');
#10=ADVANCED_FACE('',(),#999,.T.);
#11=ADVANCED_FACE('',(),#999,.T.);
#20=SHAPE_ASPECT('position face','',#1,.T.);
#21=GEOMETRIC_ITEM_SPECIFIC_USAGE('face usage','',#1,(#20),(#10));
#30=DATUM('D1','',#1,.F.,'A');
#31=GEOMETRIC_ITEM_SPECIFIC_USAGE('datum usage','',#1,(#30),(#11));
#40=DATUM_REFERENCE_COMPARTMENT('',$,#1,.F.,#30,(SIMPLE_DATUM_REFERENCE_MODIFIER(.MAXIMUM_MATERIAL_REQUIREMENT.)));
#41=DATUM_SYSTEM('DS1','',#1,.F.,(#40));
#42=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.25),#50);
#50=SI_UNIT(.MILLI.,.METRE.);
#60=(GEOMETRIC_TOLERANCE('position','',#42,#20)
 GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE((#41))
 GEOMETRIC_TOLERANCE_WITH_MODIFIERS(.MAXIMUM_MATERIAL_REQUIREMENT.)
 POSITION_TOLERANCE());
#999=PLANE('',#998);
#998=AXIS2_PLACEMENT_3D('',#997,$,$);
#997=CARTESIAN_POINT('',(0.,0.,0.));
ENDSEC;
END-ISO-10303-21;"#.into()
    }

    #[test]
    fn imports_complex_semantics_modifiers_and_persistent_faces() {
        let model = import_step_pmi(
            &semantic_fixture(),
            &BTreeMap::from([(10, face("10")), (11, face("11"))]),
        )
        .unwrap();
        assert_eq!(model.tolerances.len(), 1);
        let tolerance = &model.tolerances[0];
        assert_eq!(tolerance.kind, GeometricToleranceKind::Position);
        assert_eq!(tolerance.magnitude, 0.25);
        assert_eq!(tolerance.face_id, face("10"));
        assert_eq!(
            tolerance.modifiers,
            vec![PmiModifier::MaximumMaterialRequirement]
        );
        assert_eq!(
            tolerance.datum_references[0].precedence,
            DatumPrecedence::Primary
        );
        assert_eq!(tolerance.datum_references[0].datums[0].label, "A");
        assert_eq!(tolerance.datum_references[0].datums[0].face_id, face("11"));
        assert_eq!(
            tolerance.datum_references[0].modifiers,
            vec![PmiModifier::MaximumMaterialRequirement]
        );
        assert_eq!(model.shape_aspect_to_face.get(&20), Some(&face("10")));
        let json = model.to_json().unwrap();
        assert!(json.contains("\"kind\":\"POSITION_TOLERANCE\""));
        assert!(json.contains("\"magnitude\":2.50000000000000000E-1"));
        assert_eq!(json, model.to_json().unwrap());
    }

    #[test]
    fn follows_shape_aspect_relationship_for_face_binding() {
        let source = r#"#1=SHAPE_ASPECT('composite','',#90,.T.);
#2=SHAPE_ASPECT('member','',#90,.T.);
#3=SHAPE_ASPECT_RELATIONSHIP('','',#1,#2);
#4=GEOMETRIC_ITEM_SPECIFIC_USAGE('','',#90,(#2),(#10));
#5=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.1),#6);
#6=SI_UNIT(.MILLI.,.METRE.);
#7=FLATNESS_TOLERANCE('flat','',#5,#1);
#10=ADVANCED_FACE('',(),#11,.T.);
#11=PLANE('',#12);
#12=AXIS2_PLACEMENT_3D('',#13,$,$);
#13=CARTESIAN_POINT('',(0.,0.,0.));
#90=PRODUCT_DEFINITION_SHAPE('','',#91);
#91=PRODUCT_DEFINITION('','',#92,#93);
#92=PRODUCT_DEFINITION_FORMATION('','',#94);
#93=PRODUCT_DEFINITION_CONTEXT('',#95,'');
#94=PRODUCT('','','',(#95));
#95=PRODUCT_CONTEXT('',#96,'');
#96=APPLICATION_CONTEXT('');"#;
        let model = import_step_pmi(source, &BTreeMap::from([(10, face("10"))])).unwrap();
        assert_eq!(model.tolerances[0].face_id, face("10"));
        assert_eq!(model.shape_aspects[0].related_aspects, vec![2]);
    }

    #[test]
    fn presentation_only_is_explicit_and_polylines_survive() {
        let source = r#"#1=ANNOTATION_CURVE_OCCURRENCE('picture','',#2);
#2=POLYLINE('',(#3,#4));
#3=CARTESIAN_POINT('',(0.,0.,0.));
#4=CARTESIAN_POINT('',(1.,2.,3.));"#;
        let model = import_step_pmi(source, &BTreeMap::new()).unwrap();
        assert!(model.is_presentation_only());
        assert_eq!(model.presentation_polylines[0].points[1], [1., 2., 3.]);
        assert!(
            model
                .diagnostics
                .iter()
                .any(|d| d.kind == PmiDiagnosticKind::PresentationOnly)
        );
        assert!(model.tolerances.is_empty());
    }

    #[test]
    fn tessellated_picture_without_lines_still_has_honest_diagnostic() {
        let model = import_step_pmi(
            "#1=TESSELLATED_ANNOTATION_OCCURRENCE('picture','',#2);\n#2=TESSELLATED_PRESENTATION_ITEM('');",
            &BTreeMap::new(),
        )
        .unwrap();
        assert!(model.is_presentation_only());
        assert!(
            model
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.kind == PmiDiagnosticKind::PresentationOnly })
        );
        assert!(
            model.diagnostics.iter().any(|diagnostic| {
                diagnostic.kind == PmiDiagnosticKind::UnsupportedPresentation
            })
        );
    }

    #[test]
    fn dangling_semantic_face_is_typed_error() {
        let mut source = semantic_fixture();
        source = source.replace("(#20),(#10)", "(#20),(#9999)");
        let error = import_step_pmi(
            &source,
            &BTreeMap::from([(10, face("10")), (11, face("11"))]),
        )
        .unwrap_err();
        assert_eq!(error.kind, PmiErrorKind::DanglingFace);
        assert_eq!(error.entity, Some(60));
    }

    #[test]
    fn presentation_export_is_byte_identical_and_reimports_as_picture() {
        let model = import_step_pmi(
            &semantic_fixture(),
            &BTreeMap::from([(10, face("10")), (11, face("11"))]),
        )
        .unwrap();
        let layouts = BTreeMap::from([
            (
                30,
                PmiPresentationLayout {
                    anchor: [1., 0., 0.],
                    frame_origin: [2., 3., 0.],
                },
            ),
            (
                60,
                PmiPresentationLayout {
                    anchor: [0., 0., 0.],
                    frame_origin: [4., 5., 0.],
                },
            ),
        ]);
        let first = export_step_pmi_presentation(&model, &layouts, 2.).unwrap();
        let second = export_step_pmi_presentation(&model, &layouts, 2.).unwrap();
        assert_eq!(first.part21, second.part21);
        assert_eq!(first.polylines, second.polylines);
        assert!(
            first
                .polylines
                .iter()
                .any(|line| line.role == PmiPolylineRole::FeatureControlFrame)
        );
        assert!(
            first
                .polylines
                .iter()
                .any(|line| line.role == PmiPolylineRole::Text)
        );
        let picture = import_step_pmi(&first.part21, &BTreeMap::new()).unwrap();
        assert!(picture.is_presentation_only());
        assert_eq!(picture.presentation_polylines.len(), first.polylines.len());
        assert_eq!(
            picture.presentation_polylines[0].points,
            first.polylines[0].points
        );
    }

    #[test]
    fn missing_layout_and_bad_modifier_are_not_silent() {
        let model = import_step_pmi(
            &semantic_fixture(),
            &BTreeMap::from([(10, face("10")), (11, face("11"))]),
        )
        .unwrap();
        let error = export_step_pmi_presentation(&model, &BTreeMap::new(), 1.).unwrap_err();
        assert_eq!(error.kind, PmiErrorKind::MissingLayout);
        let source = semantic_fixture().replace(
            ".MAXIMUM_MATERIAL_REQUIREMENT.)\n POSITION_TOLERANCE",
            ".NOT_A_MODIFIER.)\n POSITION_TOLERANCE",
        );
        let error = import_step_pmi(
            &source,
            &BTreeMap::from([(10, face("10")), (11, face("11"))]),
        )
        .unwrap();
        assert_eq!(
            error.tolerances[0].modifiers,
            vec![PmiModifier::OtherStandard("NOT_A_MODIFIER".into())]
        );
        assert!(model.to_json().unwrap().contains("\"M\""));
    }

    #[test]
    fn composite_datum_members_keep_order_and_shared_precedence() {
        let source = semantic_fixture()
            .replace("#30=DATUM('D1','',#1,.F.,'A');", "#30=DATUM('D1','',#1,.F.,'A');\n#32=DATUM('D2','',#1,.F.,'B');\n#33=GEOMETRIC_ITEM_SPECIFIC_USAGE('datum usage','',#1,(#32),(#11));")
            .replace("#40=DATUM_REFERENCE_COMPARTMENT('',$,#1,.F.,#30,", "#40=DATUM_REFERENCE_COMPARTMENT('',$,#1,.F.,(#30,#32),");
        let model = import_step_pmi(
            &source,
            &BTreeMap::from([(10, face("10")), (11, face("11"))]),
        )
        .unwrap();
        let reference = &model.tolerances[0].datum_references[0];
        assert_eq!(
            reference
                .datums
                .iter()
                .map(|d| d.label.as_str())
                .collect::<Vec<_>>(),
            vec!["A", "B"]
        );
        assert_eq!(reference.precedence, DatumPrecedence::Primary);
    }
}
