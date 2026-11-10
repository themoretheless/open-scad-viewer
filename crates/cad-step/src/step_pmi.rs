//! AP242 semantic GD&T and its deliberately separate graphic presentation.
//!
//! This module is independent of the B-rep interchange routes.  The caller
//! supplies the `ADVANCED_FACE` entity to persistent topology id mapping that
//! its geometry importer produced.  Presentation-only input is retained as
//! lines plus diagnostics; it is never promoted to semantic PMI.

mod import;
mod presentation;
#[cfg(test)]
mod tests;

pub use import::import_step_pmi;
pub use presentation::{PmiPresentation, PmiPresentationLayout, export_step_pmi_presentation};

use brep_core::TopoId;
use crate::step_part21::Value;
use presentation::{Json, validate_line};
use std::collections::BTreeMap;
use std::fmt;

pub const STEP_PMI_CAPABILITY: &str = "step-ap242-semantic-pmi/1";
pub const STEP_PMI_JSON_SCHEMA: &str = "open-scad-viewer.step-pmi.v1";

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

    pub(super) fn from_step(entity: &str) -> Option<Self> {
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
    pub(super) fn parse(value: &Value, entity: usize) -> PmiResult<Vec<Self>> {
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
