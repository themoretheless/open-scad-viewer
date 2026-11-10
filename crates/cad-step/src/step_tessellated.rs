//! AP242 triangulated faces/shells as a separate, immutable LOD-0 payload.
//!
//! Exact B-rep entities are indexed and linked, never replaced by triangles.
//! The optional drift check is a directed vertex-to-sample measurement only; it
//! is intentionally not presented as a Hausdorff-distance certificate.

use super::schema::{StepProtocol, StepSchemaReport, inspect_step_schema};
use crate::step_part21::{
    Entity, Value, call, list, name, number, one_ref, parse, reachable, refs, usize_value, values,
};
use nurbs_core::{Error, Result};
use std::collections::{BTreeMap, BTreeSet};

fn refuse(message: impl Into<String>) -> Error {
    Error::new("BREP_STEP_TESSELLATED_REFUSED", message)
}

#[derive(Clone, Debug, PartialEq)]
pub struct StepTessellatedFace {
    pub entity_id: usize,
    pub name: String,
    pub coordinates_entity_id: usize,
    pub positions: Vec<[f64; 3]>,
    /// Zero-based, counterclockwise-as-authored index triples.  Part 21 input
    /// uses one-based indices; conversion is checked at the boundary.
    pub triangles: Vec<[u32; 3]>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StepDriftAssessment {
    NotAssessed {
        reason: StepDriftNotAssessedReason,
    },
    /// Maximum over tessellated vertices of distance to the closest supplied
    /// exact-geometry sample.  This is not a two-sided Hausdorff bound.
    VertexToExactSamples {
        max_distance: f64,
        declared_tolerance: Option<f64>,
        within_declared_tolerance: Option<bool>,
        exact_sample_count: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepDriftNotAssessedReason {
    NoExactSamples,
    EmptyTessellation,
    NonFiniteExactSample,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StepTessellatedLod0 {
    pub shell_entity_id: usize,
    pub name: String,
    pub faces: Vec<StepTessellatedFace>,
    /// Exact MANIFOLD_SOLID_BREP/BREP_WITH_VOIDS entities related through
    /// GEOMETRIC_MODEL_RELATIONSHIP.  Empty means unlinked, not absent B-rep.
    pub exact_brep_entity_ids: Vec<usize>,
    /// File uncertainty, when present.  It is not automatically a certified
    /// tessellation accuracy attribute.
    pub declared_tolerance: Option<f64>,
    pub drift: StepDriftAssessment,
}

impl StepTessellatedLod0 {
    pub fn triangle_count(&self) -> usize {
        self.faces.iter().map(|face| face.triangles.len()).sum()
    }

    pub fn vertex_count(&self) -> usize {
        self.faces.iter().map(|face| face.positions.len()).sum()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepTessellatedDiagnosticCode {
    MissingExactBrepLink,
    AmbiguousExactBrepLink,
    UnresolvedGeometricModelRelationship,
    OrphanTessellatedFace,
    UnsupportedTessellatedEntity,
    DriftNotAssessed,
    DriftExceedsDeclaredTolerance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepTessellatedSeverity {
    Info,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepTessellatedDiagnostic {
    pub code: StepTessellatedDiagnosticCode,
    pub severity: StepTessellatedSeverity,
    pub entity_id: Option<usize>,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StepTessellatedImport {
    pub schema: StepSchemaReport,
    pub lod0_shells: Vec<StepTessellatedLod0>,
    /// All exact solids remain available to the parent importer as authoritative
    /// geometry roots.  This module does not build or weaken that B-rep.
    pub exact_brep_entity_ids: Vec<usize>,
    pub diagnostics: Vec<StepTessellatedDiagnostic>,
}

impl StepTessellatedImport {
    pub fn has_diagnostic(&self, code: StepTessellatedDiagnosticCode) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == code)
    }
}

fn diagnostic(
    code: StepTessellatedDiagnosticCode,
    severity: StepTessellatedSeverity,
    entity_id: Option<usize>,
    message: impl Into<String>,
) -> StepTessellatedDiagnostic {
    StepTessellatedDiagnostic {
        code,
        severity,
        entity_id,
        message: message.into(),
    }
}

fn coordinates(
    entities: &BTreeMap<usize, Entity>,
    face_id: usize,
    coordinates_id: usize,
) -> Result<Vec<[f64; 3]>> {
    let (ty, args) = call(entities, coordinates_id)?;
    if ty != "COORDINATES_LIST" || args.len() != 2 {
        return Err(refuse(format!(
            "Tessellated face #{face_id} requires COORDINATES_LIST(name, points)"
        )));
    }
    let points = list(&args[1], "COORDINATES_LIST points")?;
    points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let point = values(point, "tessellated coordinate")?;
            if point.len() != 3 || point.iter().any(|coordinate| !coordinate.is_finite()) {
                return Err(refuse(format!(
                    "Tessellated face #{face_id} point {} is not a finite 3D point",
                    index + 1
                )));
            }
            Ok([point[0], point[1], point[2]])
        })
        .collect()
}

fn triangulated_face(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
    ty: &str,
    args: &[Value],
) -> Result<StepTessellatedFace> {
    if !matches!(ty, "TRIANGULATED_FACE" | "TESSELLATED_FACE") || args.len() != 3 {
        return Err(refuse(format!(
            "#{id} must be TRIANGULATED_FACE/TESSELLATED_FACE(name, coordinates, triangles)"
        )));
    }
    let coordinates_id = one_ref(&args[1], "tessellated coordinates")?;
    let positions = coordinates(entities, id, coordinates_id)?;
    let triangles = list(&args[2], "tessellated triangles")?;
    let mut converted = Vec::with_capacity(triangles.len());
    for (triangle_index, triangle) in triangles.iter().enumerate() {
        let indices = list(triangle, "triangle indices")?;
        if indices.len() != 3 {
            return Err(refuse(format!(
                "Tessellated face #{id} triangle {} does not have three indices",
                triangle_index + 1
            )));
        }
        let mut output = [0u32; 3];
        for corner in 0..3 {
            let one_based = usize_value(&indices[corner], "triangle index")?;
            if one_based == 0 || one_based > positions.len() {
                return Err(refuse(format!(
                    "Tessellated face #{id} triangle {} has out-of-range index {one_based}",
                    triangle_index + 1
                )));
            }
            output[corner] = u32::try_from(one_based - 1)
                .map_err(|_| refuse("Tessellated index exceeds the LOD-0 u32 representation"))?;
        }
        if output[0] == output[1] || output[1] == output[2] || output[2] == output[0] {
            return Err(refuse(format!(
                "Tessellated face #{id} triangle {} is topologically degenerate",
                triangle_index + 1
            )));
        }
        converted.push(output);
    }
    Ok(StepTessellatedFace {
        entity_id: id,
        name: name(args).unwrap_or("").to_string(),
        coordinates_entity_id: coordinates_id,
        positions,
        triangles: converted,
    })
}

fn file_uncertainty(entities: &BTreeMap<usize, Entity>) -> Result<Option<f64>> {
    let mut result = None;
    for entity in entities.values() {
        let Value::Call(ty, args) = &entity.value else {
            continue;
        };
        if ty != "UNCERTAINTY_MEASURE_WITH_UNIT" {
            continue;
        }
        let Some(value) = args.first() else {
            return Err(refuse("UNCERTAINTY_MEASURE_WITH_UNIT has no value"));
        };
        let value = match value {
            Value::Call(ty, measure) if ty == "LENGTH_MEASURE" && measure.len() == 1 => {
                number(&measure[0], "file uncertainty")?
            }
            _ => number(value, "file uncertainty")?,
        };
        if !value.is_finite() || value < 0.0 {
            return Err(refuse("File uncertainty must be finite and nonnegative"));
        }
        if result.is_some_and(|previous| previous != value) {
            return Err(refuse(
                "Multiple different file uncertainties require context-specific resolution",
            ));
        }
        result = Some(value);
    }
    Ok(result)
}

/// Import supported AP242 triangulated shells without invoking exact B-rep
/// construction.  Schema preflight runs before the Part 21 entity parser.
pub fn import_ap242_tessellated_lod0(text: &str) -> Result<StepTessellatedImport> {
    let schema = inspect_step_schema(text)?;
    if !schema.supports(StepProtocol::Ap242) {
        return Err(refuse(
            "Tessellated LOD-0 import requires an explicitly declared AP242 schema",
        ));
    }
    let entities = parse(text)?;
    let exact_brep_entity_ids: Vec<usize> = entities
        .iter()
        .filter_map(|(id, entity)| match &entity.value {
            Value::Call(ty, _)
                if matches!(ty.as_str(), "MANIFOLD_SOLID_BREP" | "BREP_WITH_VOIDS") =>
            {
                Some(*id)
            }
            _ => None,
        })
        .collect();
    let exact_set: BTreeSet<usize> = exact_brep_entity_ids.iter().copied().collect();
    let declared_tolerance = file_uncertainty(&entities)?;
    let mut faces = BTreeMap::new();
    let mut diagnostics = Vec::new();

    for (id, entity) in &entities {
        let Value::Call(ty, args) = &entity.value else {
            continue;
        };
        match ty.as_str() {
            "TRIANGULATED_FACE" | "TESSELLATED_FACE" => {
                faces.insert(*id, triangulated_face(&entities, *id, ty, args)?);
            }
            "TESSELLATED_CURVE_SET" | "TESSELLATED_POINT_SET" | "TESSELLATED_WIRE" => {
                diagnostics.push(diagnostic(
                    StepTessellatedDiagnosticCode::UnsupportedTessellatedEntity,
                    StepTessellatedSeverity::Warning,
                    Some(*id),
                    format!("{ty} is outside the triangulated face/shell LOD-0 subset"),
                ));
            }
            _ => {}
        }
    }

    let mut shell_faces = BTreeMap::<usize, Vec<usize>>::new();
    let mut shell_names = BTreeMap::new();
    let mut linked_faces = BTreeSet::new();
    for (id, entity) in &entities {
        let Value::Call(ty, args) = &entity.value else {
            continue;
        };
        if ty != "TESSELLATED_SHELL" {
            continue;
        }
        if args.len() != 2 {
            return Err(refuse(format!(
                "TESSELLATED_SHELL #{id} requires (name, faces)"
            )));
        }
        shell_names.insert(*id, name(args).unwrap_or("").to_string());
        for face in list(&args[1], "TESSELLATED_SHELL faces")? {
            let face_id = one_ref(face, "TESSELLATED_SHELL face")?;
            if !faces.contains_key(&face_id) {
                return Err(refuse(format!(
                    "TESSELLATED_SHELL #{id} references unsupported or missing face #{face_id}"
                )));
            }
            linked_faces.insert(face_id);
            shell_faces.entry(*id).or_default().push(face_id);
        }
    }
    for face_id in faces.keys() {
        if !linked_faces.contains(face_id) {
            diagnostics.push(diagnostic(
                StepTessellatedDiagnosticCode::OrphanTessellatedFace,
                StepTessellatedSeverity::Warning,
                Some(*face_id),
                "Triangulated face is not referenced by a TESSELLATED_SHELL",
            ));
        }
    }

    let mut relationship_exact = BTreeMap::<usize, BTreeSet<usize>>::new();
    let shell_set: BTreeSet<usize> = shell_faces.keys().copied().collect();
    let face_set: BTreeSet<usize> = faces.keys().copied().collect();
    for (id, entity) in &entities {
        let Value::Call(ty, _) = &entity.value else {
            continue;
        };
        if ty != "GEOMETRIC_MODEL_RELATIONSHIP" {
            continue;
        }
        let mut entity_refs = Vec::new();
        refs(&entity.value, &mut entity_refs);
        if entity_refs.len() != 2 {
            return Err(refuse(format!(
                "GEOMETRIC_MODEL_RELATIONSHIP #{id} requires two representation references"
            )));
        }
        let closures = [
            reachable(&entities, &[entity_refs[0]])?,
            reachable(&entities, &[entity_refs[1]])?,
        ];
        let tessellated = closures.each_ref().map(|closure| {
            (
                closure
                    .intersection(&shell_set)
                    .copied()
                    .collect::<BTreeSet<_>>(),
                closure
                    .intersection(&face_set)
                    .copied()
                    .collect::<BTreeSet<_>>(),
            )
        });
        let exact = closures.each_ref().map(|closure| {
            closure
                .intersection(&exact_set)
                .copied()
                .collect::<BTreeSet<_>>()
        });
        let forward =
            !(tessellated[0].0.is_empty() && tessellated[0].1.is_empty()) && !exact[1].is_empty();
        let reverse =
            !(tessellated[1].0.is_empty() && tessellated[1].1.is_empty()) && !exact[0].is_empty();
        if forward == reverse {
            diagnostics.push(diagnostic(
                StepTessellatedDiagnosticCode::UnresolvedGeometricModelRelationship,
                StepTessellatedSeverity::Warning,
                Some(*id),
                if forward {
                    "Relationship has ambiguous tessellated/exact orientation"
                } else {
                    "Relationship does not resolve to both tessellated LOD-0 and exact B-rep"
                },
            ));
            continue;
        }
        let (related_shells, related_faces, exact_targets) = if forward {
            (&tessellated[0].0, &tessellated[0].1, &exact[1])
        } else {
            (&tessellated[1].0, &tessellated[1].1, &exact[0])
        };
        for (shell, members) in &shell_faces {
            if related_shells.contains(shell)
                || members.iter().any(|face| related_faces.contains(face))
            {
                relationship_exact
                    .entry(*shell)
                    .or_default()
                    .extend(exact_targets.iter().copied());
            }
        }
    }

    let mut lod0_shells = Vec::new();
    for (shell_id, face_ids) in shell_faces {
        let exact_brep_entity_ids: Vec<usize> = relationship_exact
            .get(&shell_id)
            .map(|ids| ids.iter().copied().collect())
            .unwrap_or_default();
        if exact_brep_entity_ids.is_empty() && !exact_set.is_empty() {
            diagnostics.push(diagnostic(
                StepTessellatedDiagnosticCode::MissingExactBrepLink,
                StepTessellatedSeverity::Warning,
                Some(shell_id),
                "Exact B-rep exists but no resolved GEOMETRIC_MODEL_RELATIONSHIP links this LOD-0",
            ));
        } else if exact_brep_entity_ids.len() > 1 {
            diagnostics.push(diagnostic(
                StepTessellatedDiagnosticCode::AmbiguousExactBrepLink,
                StepTessellatedSeverity::Warning,
                Some(shell_id),
                "LOD-0 resolves to multiple exact B-rep roots; parent must not choose silently",
            ));
        }
        lod0_shells.push(StepTessellatedLod0 {
            shell_entity_id: shell_id,
            name: shell_names.get(&shell_id).cloned().unwrap_or_default(),
            faces: face_ids
                .into_iter()
                .map(|face_id| faces[&face_id].clone())
                .collect(),
            exact_brep_entity_ids,
            declared_tolerance,
            drift: StepDriftAssessment::NotAssessed {
                reason: StepDriftNotAssessedReason::NoExactSamples,
            },
        });
        diagnostics.push(diagnostic(
            StepTessellatedDiagnosticCode::DriftNotAssessed,
            StepTessellatedSeverity::Info,
            Some(shell_id),
            "Exact-surface samples are unavailable; no Hausdorff or drift certificate is claimed",
        ));
    }

    Ok(StepTessellatedImport {
        schema,
        lod0_shells,
        exact_brep_entity_ids,
        diagnostics,
    })
}

/// Compute a directed, reproducible vertex-to-exact-sample drift observation.
/// The caller owns exact-surface sampling and its accuracy.  Empty/non-finite
/// samples produce a typed non-assessment rather than a numeric certification.
pub fn assess_lod0_vertex_drift(
    lod0: &StepTessellatedLod0,
    exact_samples: &[[f64; 3]],
) -> (StepDriftAssessment, Vec<StepTessellatedDiagnostic>) {
    let entity_id = Some(lod0.shell_entity_id);
    if lod0.vertex_count() == 0 {
        return (
            StepDriftAssessment::NotAssessed {
                reason: StepDriftNotAssessedReason::EmptyTessellation,
            },
            vec![diagnostic(
                StepTessellatedDiagnosticCode::DriftNotAssessed,
                StepTessellatedSeverity::Info,
                entity_id,
                "Empty tessellation has no vertex drift observation",
            )],
        );
    }
    if exact_samples.is_empty() {
        return (
            StepDriftAssessment::NotAssessed {
                reason: StepDriftNotAssessedReason::NoExactSamples,
            },
            vec![diagnostic(
                StepTessellatedDiagnosticCode::DriftNotAssessed,
                StepTessellatedSeverity::Info,
                entity_id,
                "No exact samples supplied; Hausdorff distance remains unassessed",
            )],
        );
    }
    if exact_samples
        .iter()
        .flatten()
        .any(|coordinate| !coordinate.is_finite())
    {
        return (
            StepDriftAssessment::NotAssessed {
                reason: StepDriftNotAssessedReason::NonFiniteExactSample,
            },
            vec![diagnostic(
                StepTessellatedDiagnosticCode::DriftNotAssessed,
                StepTessellatedSeverity::Warning,
                entity_id,
                "Non-finite exact sample prevents drift assessment",
            )],
        );
    }

    let mut max_distance: f64 = 0.0;
    for point in lod0.faces.iter().flat_map(|face| &face.positions) {
        let nearest = exact_samples
            .iter()
            .map(|sample| {
                let delta = [
                    point[0] - sample[0],
                    point[1] - sample[1],
                    point[2] - sample[2],
                ];
                delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]
            })
            .fold(f64::INFINITY, f64::min)
            .sqrt();
        max_distance = max_distance.max(nearest);
    }
    let within_declared_tolerance = lod0
        .declared_tolerance
        .map(|tolerance| max_distance <= tolerance);
    let mut diagnostics = Vec::new();
    if within_declared_tolerance == Some(false) {
        diagnostics.push(diagnostic(
            StepTessellatedDiagnosticCode::DriftExceedsDeclaredTolerance,
            StepTessellatedSeverity::Warning,
            entity_id,
            format!(
                "Directed vertex-to-sample distance {max_distance:.17e} exceeds file uncertainty {:.17e}; this is not a Hausdorff certificate",
                lod0.declared_tolerance.unwrap_or(0.0)
            ),
        ));
    }
    (
        StepDriftAssessment::VertexToExactSamples {
            max_distance,
            declared_tolerance: lod0.declared_tolerance,
            within_declared_tolerance,
            exact_sample_count: exact_samples.len(),
        },
        diagnostics,
    )
}

fn real(value: f64) -> String {
    // Canonicalize negative zero as well as using a platform-stable scientific
    // representation, so repeated exports are byte-identical.
    let value = if value == 0.0 { 0.0 } else { value };
    format!("{value:.17E}")
}

#[derive(Default)]
struct ReplicaWriter {
    next_id: usize,
    rows: Vec<String>,
}

impl ReplicaWriter {
    fn emit(&mut self, body: String) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.rows.push(format!("#{id}={body};"));
        id
    }
}

fn quoted(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// Deterministically export a DATA fragment.  Exact B-rep references are
/// external to this fragment and are resolved by the parent when it splices the
/// rows next to the authoritative B-rep graph.
pub fn export_tessellated_replica_fragment(
    lod0: &StepTessellatedLod0,
    first_entity_id: usize,
) -> Result<String> {
    if first_entity_id == 0 {
        return Err(refuse("Part 21 entity numbers start at one"));
    }
    let mut writer = ReplicaWriter {
        next_id: first_entity_id,
        rows: Vec::new(),
    };
    let mut face_ids = Vec::new();
    for face in &lod0.faces {
        let points = face
            .positions
            .iter()
            .map(|point| format!("({},{},{})", real(point[0]), real(point[1]), real(point[2])))
            .collect::<Vec<_>>()
            .join(",");
        let coordinates_id = writer.emit(format!("COORDINATES_LIST({},({points}))", quoted("")));
        let triangles = face
            .triangles
            .iter()
            .map(|triangle| {
                format!(
                    "({},{},{})",
                    triangle[0] as usize + 1,
                    triangle[1] as usize + 1,
                    triangle[2] as usize + 1
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        face_ids.push(writer.emit(format!(
            "TRIANGULATED_FACE({},#{coordinates_id},({triangles}))",
            quoted(&face.name)
        )));
    }
    let face_refs = face_ids
        .iter()
        .map(|id| format!("#{id}"))
        .collect::<Vec<_>>()
        .join(",");
    let shell_id = writer.emit(format!(
        "TESSELLATED_SHELL({},({face_refs}))",
        quoted(&lod0.name)
    ));
    for exact_id in &lod0.exact_brep_entity_ids {
        writer.emit(format!(
            "GEOMETRIC_MODEL_RELATIONSHIP('','',#{shell_id},#{exact_id})"
        ));
    }
    Ok(writer.rows.join("\n") + "\n")
}

/// Export a standalone AP242 mesh-only replica.  Linked LODs are rejected so a
/// standalone file can never contain dangling exact-B-rep relationship refs.
pub fn export_tessellated_replica(lod0: &StepTessellatedLod0) -> Result<String> {
    if !lod0.exact_brep_entity_ids.is_empty() {
        return Err(refuse(
            "Standalone replica cannot preserve exact-B-rep links; export a fragment beside the exact graph",
        ));
    }
    let fragment = export_tessellated_replica_fragment(lod0, 1)?;
    Ok(format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('OpenSCAD Viewer AP242 LOD-0 replica'),'2;1');\nFILE_NAME('lod0-replica.step','1970-01-01T00:00:00',('open-scad-viewer'),(''),'brep-core','','');\nFILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF'));\nENDSEC;\nDATA;\n{fragment}ENDSEC;\nEND-ISO-10303-21;\n"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step_part21::{Tok, lex};

    const HEADER: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('x'),'2;1');\nFILE_NAME('x','1970-01-01T00:00:00',(''),(''),'','','');\nFILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF'));\nENDSEC;\nDATA;\n";
    const FOOTER: &str = "ENDSEC;\nEND-ISO-10303-21;\n";

    fn triangle_document(extra: &str) -> String {
        format!(
            "{HEADER}#1=COORDINATES_LIST('',((0.,0.,0.),(1.,0.,0.),(0.,1.,0.)));\n#2=TRIANGULATED_FACE('lod',#1,((1,2,3)));\n#3=TESSELLATED_SHELL('shell',(#2));\n{extra}{FOOTER}"
        )
    }

    #[test]
    fn imports_face_shell_as_separate_lod0_and_preserves_exact_root() {
        let text = triangle_document(
            "#4=MANIFOLD_SOLID_BREP('exact',#5);\n#5=CLOSED_SHELL('',());\n#6=GEOMETRIC_MODEL_RELATIONSHIP('','',#3,#4);\n",
        );
        let imported = import_ap242_tessellated_lod0(&text).unwrap();
        assert_eq!(imported.exact_brep_entity_ids, vec![4]);
        assert_eq!(imported.lod0_shells.len(), 1);
        let lod = &imported.lod0_shells[0];
        assert_eq!(lod.exact_brep_entity_ids, vec![4]);
        assert_eq!(lod.triangle_count(), 1);
        assert_eq!(lod.faces[0].triangles, vec![[0, 1, 2]]);
        assert!(matches!(
            lod.drift,
            StepDriftAssessment::NotAssessed {
                reason: StepDriftNotAssessedReason::NoExactSamples
            }
        ));

        let reversed = text.replace(
            "GEOMETRIC_MODEL_RELATIONSHIP('','',#3,#4)",
            "GEOMETRIC_MODEL_RELATIONSHIP('','',#4,#3)",
        );
        let reversed = import_ap242_tessellated_lod0(&reversed).unwrap();
        assert_eq!(reversed.lod0_shells[0].exact_brep_entity_ids, vec![4]);
    }

    #[test]
    fn missing_and_ambiguous_links_are_typed() {
        let missing =
            triangle_document("#4=MANIFOLD_SOLID_BREP('exact',#5);\n#5=CLOSED_SHELL('',());\n");
        let imported = import_ap242_tessellated_lod0(&missing).unwrap();
        assert!(imported.has_diagnostic(StepTessellatedDiagnosticCode::MissingExactBrepLink));

        let ambiguous = triangle_document(
            "#4=MANIFOLD_SOLID_BREP('a',#5);\n#5=CLOSED_SHELL('',());\n#6=MANIFOLD_SOLID_BREP('b',#7);\n#7=CLOSED_SHELL('',());\n#8=GEOMETRIC_MODEL_RELATIONSHIP('','',#3,#4);\n#9=GEOMETRIC_MODEL_RELATIONSHIP('','',#3,#6);\n",
        );
        let imported = import_ap242_tessellated_lod0(&ambiguous).unwrap();
        assert!(imported.has_diagnostic(StepTessellatedDiagnosticCode::AmbiguousExactBrepLink));
    }

    #[test]
    fn reads_length_measure_file_uncertainty() {
        let text = triangle_document(
            "#4=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.01),#5,'distance_accuracy_value','');\n#5=SI_UNIT(.MILLI.,.METRE.);\n",
        );
        let imported = import_ap242_tessellated_lod0(&text).unwrap();
        assert_eq!(imported.lod0_shells[0].declared_tolerance, Some(0.01));
    }

    #[test]
    fn drift_is_observation_not_hausdorff_certificate() {
        let imported = import_ap242_tessellated_lod0(&triangle_document("")).unwrap();
        let lod = &imported.lod0_shells[0];
        let (assessment, diagnostics) =
            assess_lod0_vertex_drift(lod, &[[0., 0., 0.], [1., 0., 0.], [0., 2., 0.]]);
        assert!(matches!(
            assessment,
            StepDriftAssessment::VertexToExactSamples {
                max_distance,
                ..
            } if (max_distance - 1.0).abs() < 1e-12
        ));
        assert!(diagnostics.is_empty());
        let (not_assessed, _) = assess_lod0_vertex_drift(lod, &[]);
        assert!(matches!(
            not_assessed,
            StepDriftAssessment::NotAssessed { .. }
        ));
    }

    #[test]
    fn deterministic_replica_roundtrip_and_fragment_external_link() {
        let imported = import_ap242_tessellated_lod0(&triangle_document("")).unwrap();
        let lod = &imported.lod0_shells[0];
        let first = export_tessellated_replica(lod).unwrap();
        let second = export_tessellated_replica(lod).unwrap();
        assert_eq!(first, second);
        let roundtrip = import_ap242_tessellated_lod0(&first).unwrap();
        assert_eq!(roundtrip.lod0_shells[0].faces, lod.faces);
        assert_eq!(
            export_tessellated_replica(&roundtrip.lod0_shells[0]).unwrap(),
            first
        );

        let mut linked = lod.clone();
        linked.exact_brep_entity_ids = vec![900];
        assert!(export_tessellated_replica(&linked).is_err());
        let fragment = export_tessellated_replica_fragment(&linked, 100).unwrap();
        assert!(fragment.contains("GEOMETRIC_MODEL_RELATIONSHIP('','',#102,#900)"));
        assert!(lex(&fragment).unwrap().contains(&Tok::Hash(900)));
    }

    #[test]
    fn invalid_indices_and_non_ap242_are_refused() {
        let bad = triangle_document("").replace("(1,2,3)", "(0,2,3)");
        assert_eq!(
            import_ap242_tessellated_lod0(&bad).unwrap_err().code,
            "BREP_STEP_TESSELLATED_REFUSED"
        );
        let ap203 = triangle_document("").replace(
            "AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF",
            "CONFIG_CONTROL_DESIGN",
        );
        assert!(import_ap242_tessellated_lod0(&ap203).is_err());
    }
}
