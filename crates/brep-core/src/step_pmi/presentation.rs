//! Deterministic vector feature-control/datum frames and canonical JSON for AP242 PMI.

use super::*;
use std::collections::BTreeMap;

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

pub(super) fn validate_point(point: &[f64; 3], entity: Option<usize>) -> PmiResult<()> {
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

pub(super) fn validate_line(line: &PmiPolyline) -> PmiResult<()> {
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
pub(super) enum Json {
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

    pub(super) fn stringify(&self) -> String {
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
