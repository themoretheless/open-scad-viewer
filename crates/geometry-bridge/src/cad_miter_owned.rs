//! Original source request transport. No registration of caller-authored bounds.
use super::{Result, Value, field, optional_field};
use brep_core::sweep_miter_owned::{self, Limits, Request};
use nurbs_core::{
    curve::Curve,
    miter_section_correction::{Budget, CapCorrection},
    progressive_miter,
};
use value_codec::json;
use std::rc::Rc;
mod source_body_cache;
fn budget(v: &Value) -> Result<Budget> {
    Ok(Budget {
        quantum: field(v, "quantum")?,
        tolerance: field(v, "tolerance")?,
        max_work: optional_field(v, "maxWork")?,
    })
}
// Certificate numeric fields have f64 semantics. JSON round-trips may encode
// an integral f64 as an integer; compare values, not codec number tags.
fn same_certificate(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(_), Value::Number(_)) => a.as_f64() == b.as_f64(),
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same_certificate(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b.iter())
                    .all(|((ka, va), (kb, vb))| ka == kb && same_certificate(va, vb))
        }
        _ => a == b,
    }
}
struct OriginalRequest {
    direct: bool,
    cap_endpoints: Option<[bool; 2]>,
    loops: Vec<Vec<Curve>>,
    points: Vec<[f64; 3]>,
    scale: Curve,
    twist: Curve,
    axes: Curve,
    center: Curve,
    use_affine: bool,
    axis: Option<Curve>,
    normal: Option<Curve>,
    guide: Option<Curve>,
    circle: Option<Budget>,
    caps: Option<CapCorrection>,
    options: progressive_miter::Options,
    limits: Limits,
}
// Parse once for both native representations; the large source decoder is not
// duplicated for each generic result type in the WASM bridge.
fn parse_original(v: Value, allow_unbudgeted_direct: bool) -> Result<OriginalRequest> {
    let direct = optional_field::<bool>(&v, "direct")?.unwrap_or(false);
    let cap_endpoints: Option<[bool; 2]> = optional_field(&v, "capEndpoints")?;
    if !direct && cap_endpoints.is_some() {
        return Err(nurbs_core::Error::new(
            "NURBS_INVALID_INPUT",
            "Endpoint selection is a direct-miter source option",
        ));
    }
    for key in [
        "model",
        "sections",
        "boundaryCertificate",
        "sourceCertificate",
        "sourceSections",
        "sectionCorrection",
        "retainedWallCharts",
        "levels",
        "sharpStationIndices",
        "edges",
        "capFaceGroups",
        "capFaces",
        "filledCapErrorUpper",
        "retainedAnnularFilling",
        "retainedSmoothness",
    ] {
        if v.as_object().is_some_and(|fields| fields.contains_key(key)) {
            return Err(nurbs_core::Error::new(
                "NURBS_INVALID_INPUT",
                "Owned miter construction accepts original source only",
            ));
        }
    }
    let loops: Vec<Vec<Curve>> = field(&v, "loops")?;
    let points: Vec<[f64; 3]> = field(&v, "points")?;
    if direct
        && [
            "scale",
            "twist",
            "axisScale",
            "centerLaw",
            "frameAxis",
            "frameNormal",
            "orientationGuide",
            "initialSteps",
            "maxSteps",
        ]
        .iter()
        .any(|key| {
            v.as_object()
                .is_some_and(|fields| fields.contains_key(*key))
        })
    {
        return Err(nurbs_core::Error::new(
            "NURBS_INVALID_INPUT",
            "Direct owned miter accepts direct source and explicit circle/cap correction only",
        ));
    }
    let scale: Curve = if direct {
        nurbs_core::progressive_sweep::constant_vector_law([1., 0., 0.])?
    } else {
        field(&v, "scale")?
    };
    let twist: Curve = if direct {
        nurbs_core::progressive_sweep::constant_vector_law([0.; 3])?
    } else {
        field(&v, "twist")?
    };
    let axes: Option<Curve> = optional_field(&v, "axisScale")?;
    let center: Option<Curve> = optional_field(&v, "centerLaw")?;
    let axis: Option<Curve> = optional_field(&v, "frameAxis")?;
    let normal: Option<Curve> = optional_field(&v, "frameNormal")?;
    if axis.is_some() != normal.is_some() {
        return Err(nurbs_core::Error::new(
            "NURBS_INVALID_INPUT",
            "Authored miter requires both frame_axis and frame_normal",
        ));
    }
    let use_affine = axes.is_some() || center.is_some();
    let axes = axes.unwrap_or(nurbs_core::progressive_sweep::constant_vector_law([1.; 3])?);
    let center = center.unwrap_or(nurbs_core::progressive_sweep::constant_vector_law([0.; 3])?);
    let guide: Option<Curve> = optional_field(&v, "orientationGuide")?;
    let circle = v.get("circleCorrection").filter(|x| !x.is_null());
    let cap = v.get("capCorrection").filter(|x| !x.is_null());
    let default_limits = json!({
        "maxProducts":100000,"maxFaces":1024,"exactWork":1000000,
        "domainTolerance":0.001,"domainPairs":1000,"domainCells":10000,"projectionCells":10000,
        "capRegions":{"maxWalls":1024,"maxExactWork":1000000,"maxChartCells":1000,
            "maxTrimPairs":100000,"maxTrimCells":100000,"maxTrimDomainCells":1000000}});
    let limits = v
        .get("limits")
        .filter(|x| !x.is_null())
        .unwrap_or(&default_limits);
    let cap_regions = &limits["capRegions"];
    let authored_budget: Option<f64> = optional_field(&v, "maxDeviation")?;
    let unbudgeted = allow_unbudgeted_direct && direct && authored_budget.is_none();
    if unbudgeted && circle.is_some() {
        return Err(nurbs_core::Error::new(
            "NURBS_INVALID_INPUT",
            "Circle correction requires an authored maxDeviation",
        ));
    }
    let options = progressive_miter::Options {
        normal: field(&v, "normal")?,
        closed: field(&v, "closed")?,
        miter_limit: field(&v, "miterLimit")?,
        initial_steps: if direct {
            1
        } else {
            field(&v, "initialSteps")?
        },
        max_steps: if direct { 1 } else { field(&v, "maxSteps")? },
        // Direct neutral-law construction does not refine or automatically
        // correct geometry. This finite ceiling only reuses its acceptance
        // machinery; it is not an author tolerance or a transported premise.
        max_deviation: if unbudgeted {
            f64::MAX
        } else {
            field(&v, "maxDeviation")?
        },
    };
    let circle = circle.map(budget).transpose()?;
    let caps = cap
        .map(|c| {
            Ok::<_, nurbs_core::Error>(CapCorrection {
                budget: budget(c)?,
                authored_frame: optional_field(c, "authoredFrame")?.unwrap_or(false),
            })
        })
        .transpose()?;
    let max_faces = field(limits, "maxFaces")?;
    let exact_work = field(limits, "exactWork")?;
    let limits = Limits {
        max_products: field(limits, "maxProducts")?,
        max_faces,
        correspondence_faces: optional_field(limits, "correspondenceMaxFaces")?
            .unwrap_or(max_faces),
        cap_max_edges: optional_field(limits, "capMaxEdges")?
            .unwrap_or(max_faces),
        wall_cells: optional_field(limits, "wallCells")?.unwrap_or(10000),
        exact_work,
        domain_exact_work: optional_field(limits, "domainExactWork")?
            .unwrap_or(exact_work),
        projection_exact_work: optional_field(limits, "projectionExactWork")?
            .unwrap_or(exact_work),
        domain_tolerance: field(limits, "domainTolerance")?,
        domain_pairs: field(limits, "domainPairs")?,
        domain_cells: field(limits, "domainCells")?,
        projection_cells: field(limits, "projectionCells")?,
        cap_regions: brep_core::sweep_cap_contacts::Budgets {
            max_walls: field(cap_regions, "maxWalls")?,
            max_exact_work: field(cap_regions, "maxExactWork")?,
            max_chart_cells: field(cap_regions, "maxChartCells")?,
            max_trim_pairs: field(cap_regions, "maxTrimPairs")?,
            max_trim_cells: field(cap_regions, "maxTrimCells")?,
            max_trim_domain_cells: field(cap_regions, "maxTrimDomainCells")?,
        },
    };
    Ok(OriginalRequest {
        direct,
        cap_endpoints,
        loops,
        points,
        scale,
        twist,
        axes,
        center,
        use_affine,
        axis,
        normal,
        guide,
        circle,
        caps,
        options,
        limits,
    })
}
fn construct_body_with<T>(
    v: Value,
    allow_unbudgeted_direct: bool,
    finish: impl FnOnce(&Request<'_>, &Limits, bool, Option<[bool; 2]>) -> Result<T>,
) -> Result<T> {
    let original = parse_original(v, allow_unbudgeted_direct)?;
    finish_original_with(original, finish)
}
fn finish_original_with<T>(
    original: OriginalRequest,
    finish: impl FnOnce(&Request<'_>, &Limits, bool, Option<[bool; 2]>) -> Result<T>,
) -> Result<T> {
    let request = Request {
        loops: &original.loops,
        points: &original.points,
        scale: &original.scale,
        twist: &original.twist,
        options: original.options,
        affine: original
            .use_affine
            .then_some((&original.axes, &original.center)),
        frames: original.axis.as_ref().zip(original.normal.as_ref()),
        guide: original.guide.as_ref(),
        circle: original.circle,
        caps: original.caps.as_ref().map(|c| CapCorrection {
            budget: c.budget,
            authored_frame: c.authored_frame,
        }),
    };
    finish(
        &request,
        &original.limits,
        original.direct,
        original.cap_endpoints,
    )
}
fn construct_body(v: Value) -> Result<Rc<sweep_miter_owned::BoundaryBody>> {
    // Validate every request, including forbidden transferred evidence, before
    // looking up private constructor-owned geometry. All source limits belong
    // to the exact key. Only the transport operation label is irrelevant.
    let original = parse_original(v.clone(), false)?;
    let key = source_body_cache::key(v);
    if let Some(body) = key.as_deref().and_then(source_body_cache::get) {
        return Ok(body);
    }
    let body = Rc::new(finish_original_with(original, |request, limits, direct, cap_endpoints| {
        if direct {
            sweep_miter_owned::construct_direct(
                &sweep_miter_owned::DirectRequest {
                    loops: request.loops,
                    points: request.points,
                    normal: request.options.normal,
                    closed: request.options.closed,
                    miter_limit: request.options.miter_limit,
                    max_deviation: request.options.max_deviation,
                    circle: request.circle,
                    caps: request.caps.as_ref().map(|c| CapCorrection {
                        budget: c.budget,
                        authored_frame: c.authored_frame,
                    }),
                    cap_endpoints,
                },
                limits,
            )
        } else {
            sweep_miter_owned::construct(request, limits)
        }
    })?);
    if let Some(key) = key {
        source_body_cache::insert(key, body.clone());
    }
    Ok(body)
}
pub fn construct(v: Value) -> Result<Value> {
    let direct = optional_field::<bool>(&v, "direct")?.unwrap_or(false);
    let cap_endpoints: Option<[bool; 2]> = optional_field(&v, "capEndpoints")?;
    let body = construct_body(v)?;
    body_value(&body, body.model(), body.boundary(), direct, cap_endpoints)
}
fn body_value(
    body: &sweep_miter_owned::BoundaryBody,
    model: &brep_core::Model,
    bound: &nurbs_core::sweeps::filled_cap_error::BoundaryCertificate,
    direct: bool,
    cap_endpoints: Option<[bool; 2]>,
) -> Result<Value> {
    let correction = body.correction().map(|r| {
        json!({"wallDisplacementUpper":r.wall_displacement_upper,
        "exactPlanarSections":r.exact_planar_sections,"work":r.work,"reason":r.reason})
    });
    Ok(
        json!({"method":"original-request-owned-miter-boundary","retentionMode":if direct {"direct"} else {"progressive"},"capEndpoints":cap_endpoints,"model":model,"sections":body.sections(),
        "sourceSections":body.source_sections(),"sectionCorrection":correction,
        "retainedWallCharts":super::cad_sweep_smoothness::charts_value(body.wall_charts()),
        "edges":body.edges(),"maxSteps":body.max_steps(),
        "sharpStationIndices":body.sharp_stations(),"closed":body.closed(),"budget":body.budget(),
        "boundaryCertificate":{"method":"retained-sweep-boundary-union","scope":"boundary-set-hausdorff",
            "continuousBound":bound.continuous_bound,"withinBudget":bound.within_budget,
            "errorUpper":bound.error_upper,"budget":body.budget(),"closed":body.closed(),
            "wallErrorUpper":bound.wall_error_upper,"filledCapErrorUpper":bound.filled_cap_error_upper,"reason":bound.reason},
        "levels":body.levels(),
        "solidGeometryCertified":false,"globalEmbeddingCertified":false}),
    )
}

/// Annular mode accepts the same original inputs, and recomputes both source
/// fillings and all retained rectangular domains in this receiving realm.
pub fn construct_annular(v: Value) -> Result<Value> {
    let direct = optional_field::<bool>(&v, "direct")?.unwrap_or(false);
    let cap_endpoints = optional_field(&v, "capEndpoints")?;
    let smoothness_work: Option<u64> = optional_field(&v, "smoothnessMaxWork")?;
    let unbudgeted = direct && optional_field::<f64>(&v, "maxDeviation")?.is_none();
    let automatic = optional_field::<bool>(&v, "automaticCaps")?.unwrap_or(unbudgeted);
    if automatic && !direct {
        return Err(nurbs_core::Error::new(
            "NURBS_INVALID_INPUT",
            "Automatic cap selection is a direct-miter source option",
        ));
    }
    let body = construct_body_with(v, true, |request, limits, direct, cap_endpoints| {
        if direct {
            let construct = if automatic {
                sweep_miter_owned::annular::construct_direct_auto
            } else {
                sweep_miter_owned::annular::construct_direct
            };
            construct(
                &sweep_miter_owned::DirectRequest {
                    loops: request.loops,
                    points: request.points,
                    normal: request.options.normal,
                    closed: request.options.closed,
                    miter_limit: request.options.miter_limit,
                    max_deviation: request.options.max_deviation,
                    circle: request.circle,
                    caps: request.caps.as_ref().map(|c| CapCorrection {
                        budget: c.budget,
                        authored_frame: c.authored_frame,
                    }),
                    cap_endpoints,
                },
                limits,
            )
        } else {
            sweep_miter_owned::annular::construct(request, limits)
        }
    })?;
    let mut value = body_value(
        body.source(),
        body.model(),
        body.boundary(),
        direct,
        cap_endpoints,
    )?;
    if unbudgeted {
        let previous = body.boundary();
        let numerical = nurbs_core::sweeps::filled_cap_error::compose_unbudgeted_boundary(
            previous.wall_error_upper,
            previous.filled_cap_error_upper,
            body.source().closed(),
        );
        value["budget"] = Value::Null;
        value["levels"] = json!([]);
        value["boundaryCertificate"]["budget"] = Value::Null;
        value["boundaryCertificate"]["withinBudget"] = Value::Null;
        value["boundaryCertificate"]["continuousBound"] = json!(numerical.continuous_bound);
        value["boundaryCertificate"]["errorUpper"] = json!(numerical.error_upper);
        value["boundaryCertificate"]["reason"] = json!(numerical.reason);
    }
    let caps = body.cap_faces();
    value["capFaceGroups"] = json!(body.cap_face_groups());
    value["capFaces"] = json!(&caps);
    value["planarCapFaces"] = json!(if body.filled_caps().is_some() {
        vec![]
    } else {
        caps
    });
    value["capRepresentation"] = json!(if body.filled_caps().is_some() {
        "radial-annular"
    } else {
        "legacy"
    });
    value["retainedAnnularFilling"]=body.filled_caps().map_or(Value::Null,|proof|json!({
        "method":"authored-affine-endpoint-annular-filling","sourceFillingCovered":proof.source_filling_covered,
        "filledCapErrorUpper":proof.filled_cap_error_upper,"withinBudget":proof.within_budget,
        "exactWork":body.annular_exact_work(),"fillingExactWork":proof.exact_work,
        "capConstructionExactWork":body.cap_construction_work(),"products":proof.products,"endpointCells":proof.endpoint_cells,
            "projectiveReparameterizedPatches":proof.projective_reparameterized_patches,
        "sourceUvFacesInjective":proof.source_uv_faces.as_ref().is_some_and(|r|r.all_faces_injective),
        "sourceUvPairsClassified":proof.source_uv_pairs.as_ref().is_some_and(|r|r.all_pairs_classified),
        "reason":proof.reason,"solidGeometryCertified":false
    }));
    if unbudgeted && !value["retainedAnnularFilling"].is_null() {
        value["retainedAnnularFilling"]["withinBudget"] = Value::Null;
    }
    value["retainedSmoothness"] = if let Some(max_work) = smoothness_work {
        let audit = body.inspect_smoothness(max_work)?;
        json!({"walls":super::cad_sweep_smoothness::profile_value(&audit.walls),
            "maxWork":max_work,"totalExactWork":audit.exact_work,
            "auditOrder":"caps-first","capInternalSeams":audit.cap_seams.iter().map(|r|json!({
                "method":"retained-annular-cap-internal-jets","edgeIds":r.edge_ids,
                "extractionComplete":r.extraction_complete,"unclassifiedFaces":r.unclassified_faces,
                "unpairedEdges":r.unpaired_edges,"g1Certified":r.g1_certified,"g2Certified":r.g2_certified,
                "exactWork":r.exact_work,"maxWork":r.max_work,
            })).collect::<Vec<_>>(),"capWallContinuity":"C0","fullBoundarySmoothnessCertified":false})
    } else {
        Value::Null
    };
    Ok(value)
}

/// Replay original inputs in this realm rather than accepting a transferred
/// numerical proof or relying on a resident handle from another worker.
pub fn reconstruct(v: Value) -> Result<Value> {
    for key in [
        "model",
        "sections",
        "sharp",
        "sourceCertificate",
        "boundaryCertificate",
    ] {
        if v.as_object().is_some_and(|fields| fields.contains_key(key)) {
            return Err(nurbs_core::Error::new(
                "NURBS_INVALID_INPUT",
                "Owned station reconstruction accepts original source only",
            ));
        }
    }
    let body = construct_body(field(&v, "source")?)?;
    let target_budget: Option<f64> = optional_field(&v, "budget")?;
    if let Some(expected) = optional_field::<Value>(&v, "expectedSourceCertificate")? {
        let bound = body.boundary();
        let actual = json!({"method":"retained-sweep-boundary-union","scope":"boundary-set-hausdorff",
            "continuousBound":bound.continuous_bound,"withinBudget":bound.within_budget,
            "errorUpper":bound.error_upper,"budget":body.budget(),"closed":body.closed(),
            "wallErrorUpper":bound.wall_error_upper,"filledCapErrorUpper":bound.filled_cap_error_upper,"reason":bound.reason});
        if !same_certificate(&expected, &actual) {
            return Err(nurbs_core::Error::new(
                "NURBS_INVALID_INPUT",
                "Reconstruction certificate differs from original replay",
            ));
        }
    }
    if let Some(expected) = optional_field::<brep_core::Model>(&v, "expectedSourceModel")? {
        if &expected != body.model() {
            return Err(nurbs_core::Error::new(
                "NURBS_INVALID_INPUT",
                "Reconstruction source differs from original replay",
            ));
        }
    }
    if let Some(expected) = optional_field::<Vec<Vec<Vec<Curve>>>>(&v, "expectedSections")? {
        if expected != body.sections() {
            return Err(nurbs_core::Error::new(
                "NURBS_INVALID_INPUT",
                "Reconstruction sections do not reproduce original replay",
            ));
        }
    }
    if let Some(expected) = optional_field::<Vec<usize>>(&v, "expectedSharp")? {
        if expected != body.sharp_stations() {
            return Err(nurbs_core::Error::new(
                "NURBS_INVALID_INPUT",
                "Reconstruction sharp stations differ from original replay",
            ));
        }
    }
    let report = body.reconstruct_stations(
        field(&v, "quantum")?,
        field(&v, "tolerance")?,
        field(&v, "maxWork")?,
        target_budget,
    )?;
    let reconstruction = report.reconstruction.as_ref();
    let require_solid = optional_field::<bool>(&v, "requireSolid")?.unwrap_or(false);
    let mut charts = None;
    let mut volume = None;
    let mut solid_certified = false;
    if require_solid {
        if let Some(model) = reconstruction.and_then(|r| r.model.as_ref()) {
            let caps = if body.closed() {
                vec![]
            } else {
                vec![model.faces.len() - 2, model.faces.len() - 1]
            };
            let wall =
                brep_core::sweep_retained_charts::inspect(model, &caps, field(&v, "wallCells")?)?;
            let regular = wall.certified;
            charts = Some(super::cad_sweep_smoothness::charts_value(&wall));
            let mut volume_request: Value = field(&v, "volumeBudgets")?;
            if !volume_request.is_object() {
                return Err(nurbs_core::Error::new(
                    "NURBS_INVALID_INPUT",
                    "Volume budgets must be an object",
                ));
            }
            // Derived fields overwrite any caller values before the native audit.
            volume_request["model"] = json!(model);
            volume_request["capFaces"] = json!(caps);
            let proof = super::cad_face_contacts::diagnose_sweep_volume(volume_request)?;
            solid_certified = regular && field::<bool>(&proof, "solidGeometryCertified")?;
            volume = Some(proof);
        }
    }
    let boundary = report.boundary.as_ref().map(|b| {
        json!({
            "method":"retained-sweep-boundary-union","scope":"boundary-set-hausdorff",
            "continuousBound":b.continuous_bound,"withinBudget":b.within_budget,
            "errorUpper":b.error_upper,"budget":target_budget,
            "closed":body.closed(),"wallErrorUpper":b.wall_error_upper,
            "filledCapErrorUpper":b.filled_cap_error_upper,"reason":b.reason
        })
    });
    let model = reconstruction
        .and_then(|r| r.model.as_ref())
        .filter(|_| !require_solid || solid_certified);
    Ok(json!({"model":model,
        "candidate":reconstruction.map(|r|json!({"sides":r.candidate.sides,
            "wallDisplacementUpper":r.candidate.wall_displacement_upper,
            "work":r.candidate.work,"reason":r.candidate.reason})),
        "boundaryCertificate":boundary,"sharpStationIndices":body.sharp_stations(),
        "reason":if require_solid && !solid_certified && reconstruction.and_then(|r|r.model.as_ref()).is_some() {
            "reconstructed-solid-geometry-unproved"
        } else { report.reason },
        "retainedWallCharts":charts,"volume":volume,
        "solidGeometryCertified":solid_certified,"globalEmbeddingCertified":false}))
}

/// Placement derives every numerical premise from a freshly owned original
/// construction, never from a supplied model or source certificate.
fn bind_affine_source(
    v: &Value,
    model: &brep_core::Model,
    bound: &nurbs_core::sweeps::filled_cap_error::BoundaryCertificate,
    budget: Option<f64>,
    closed: bool,
) -> Result<()> {
    if let Some(expected) = optional_field::<brep_core::Model>(v, "expectedSourceModel")? {
        if expected != *model {
            return Err(nurbs_core::Error::new(
                "NURBS_INVALID_INPUT",
                "Affine placement requires unchanged constructor-owned replay model",
            ));
        }
    }
    if let Some(expected) = optional_field::<Value>(v, "expectedSourceCertificate")? {
        let actual = json!({"method":"retained-sweep-boundary-union","scope":"boundary-set-hausdorff",
            "continuousBound":bound.continuous_bound,"withinBudget":bound.within_budget,
            "errorUpper":bound.error_upper,"budget":budget,"closed":closed,
            "wallErrorUpper":bound.wall_error_upper,"filledCapErrorUpper":bound.filled_cap_error_upper,"reason":bound.reason});
        if !same_certificate(&expected, &actual) {
            return Err(nurbs_core::Error::new(
                "NURBS_INVALID_INPUT",
                "Affine placement requires unchanged constructor-owned replay certificate",
            ));
        }
    }
    Ok(())
}
pub fn place(v: Value) -> Result<Value> {
    for key in [
        "model",
        "sections",
        "sourceCertificate",
        "boundaryCertificate",
    ] {
        if v.as_object().is_some_and(|fields| fields.contains_key(key)) {
            return Err(nurbs_core::Error::new(
                "NURBS_INVALID_INPUT",
                "Owned affine placement accepts original source only",
            ));
        }
    }
    let body = construct_body(field(&v, "source")?)?;
    let history: Vec<Value> = optional_field(&v, "followingPlacements")?.unwrap_or_default();
    if history.len() > 64 {
        return Err(nurbs_core::Error::new(
            "NURBS_INVALID_INPUT",
            "Too many affine replay steps",
        ));
    }
    let bind_first = history.is_empty();
    let budget = optional_field(&v, "budget")?;
    let before: Option<Value> = optional_field(&v, "beforeReconstruction")?;
    let mut report = if let Some(before) = before {
        let source_budget = optional_field(&before, "budget")?;
        let reconstructed = body.reconstruct_stations(
            field(&before, "quantum")?,
            field(&before, "tolerance")?,
            field(&before, "maxWork")?,
            source_budget,
        )?;
        match (
            reconstructed
                .reconstruction
                .as_ref()
                .and_then(|r| r.model.as_ref()),
            reconstructed.boundary.as_ref(),
        ) {
            (Some(model), Some(boundary)) => {
                if bind_first {
                    bind_affine_source(&v, model, boundary, source_budget, body.closed())?;
                }
                brep_core::sweep_affine_boundary::place(
                    model,
                    &brep_core::sweep_affine_boundary::Premises {
                        wall: boundary.wall_error_upper,
                        caps: boundary.filled_cap_error_upper,
                        closed: body.closed(),
                        source_budget,
                    },
                    field(&v, "matrix")?,
                    field(&v, "quantum")?,
                    field(&v, "maxWork")?,
                    budget,
                )?
            }
            _ => brep_core::sweep_affine_boundary::Report {
                placement: None,
                boundary: None,
                reason: reconstructed.reason,
            },
        }
    } else {
        if bind_first {
            bind_affine_source(
                &v,
                body.model(),
                body.boundary(),
                Some(body.budget()),
                body.closed(),
            )?;
        }
        body.place(
            field(&v, "matrix")?,
            field(&v, "quantum")?,
            field(&v, "maxWork")?,
            budget,
        )?
    };
    if history.len() > 64 {
        return Err(nurbs_core::Error::new(
            "NURBS_INVALID_INPUT",
            "Too many affine replay steps",
        ));
    }
    let mut final_budget = budget;
    for (index, step) in history.iter().enumerate() {
        let Some(model) = report.placement.as_ref().and_then(|p| p.model.as_ref()) else {
            break;
        };
        let Some(boundary) = report.boundary.as_ref() else {
            break;
        };
        if index + 1 == history.len() {
            bind_affine_source(&v, model, boundary, final_budget, body.closed())?;
        }
        let premises = brep_core::sweep_affine_boundary::Premises {
            wall: boundary.wall_error_upper,
            caps: boundary.filled_cap_error_upper,
            closed: body.closed(),
            source_budget: final_budget,
        };
        final_budget = optional_field(&step, "budget")?;
        report = brep_core::sweep_affine_boundary::place(
            model,
            &premises,
            field(&step, "matrix")?,
            field(&step, "quantum")?,
            field(&step, "maxWork")?,
            final_budget,
        )?;
    }
    let mut result_model_bound = false;
    if let Some(expected) = optional_field::<brep_core::Model>(&v, "expectedResultModel")? {
        if let Some(actual) = report.placement.as_ref().and_then(|p| p.model.as_ref()) {
            if expected != *actual {
                return Err(nurbs_core::Error::new(
                    "NURBS_INVALID_INPUT",
                    "Owned affine final model differs from original replay",
                ));
            }
            result_model_bound = true;
        }
    }
    let mut proposals = vec![None; body.model().faces.len()];
    for (face, chart) in &body.wall_charts().charts {
        if chart.certified {
            proposals[*face] = Some(chart.projection);
        }
    }
    let first_matrix = field::<[[f64; 4]; 4]>(&v, "matrix")?;
    for proposal in &mut proposals {
        *proposal =
            proposal.and_then(|p| brep_core::affine_lattice::transport_projection(p, first_matrix));
    }
    for step in &history {
        let matrix = field::<[[f64; 4]; 4]>(step, "matrix")?;
        for proposal in &mut proposals {
            *proposal =
                proposal.and_then(|p| brep_core::affine_lattice::transport_projection(p, matrix));
        }
    }
    let mut charts = None;
    let mut volume = None;
    let require_solid = optional_field::<bool>(&v, "requireSolid")?.unwrap_or(false);
    let mut certified = false;
    if require_solid {
        if let Some(model) = report.placement.as_ref().and_then(|p| p.model.as_ref()) {
            let caps = if body.closed() {
                vec![]
            } else {
                vec![model.faces.len() - 2, model.faces.len() - 1]
            };
            let wall = brep_core::sweep_retained_charts::inspect_with_projections(
                model,
                &caps,
                field(&v, "wallCells")?,
                &proposals,
            )?;
            let mut request: Value = field(&v, "volumeBudgets")?;
            if !request.is_object() {
                return Err(nurbs_core::Error::new(
                    "NURBS_INVALID_INPUT",
                    "Volume budgets must be an object",
                ));
            }
            request["model"] = json!(model);
            request["capFaces"] = json!(caps);
            let proof = super::cad_face_contacts::diagnose_sweep_volume_with_projections(
                request, &proposals,
            )?;
            certified = wall.certified && field::<bool>(&proof, "solidGeometryCertified")?;
            charts = Some(super::cad_sweep_smoothness::charts_value(&wall));
            volume = Some(proof);
            if !certified {
                report.placement.as_mut().unwrap().model = None;
                report.reason = "affine-solid-geometry-unproved";
            }
        }
    }
    let mut result =
        super::cad_miter_layout::affine_report_value(report, final_budget, body.closed());
    result["retainedWallCharts"] = json!(charts);
    result["volume"] = json!(volume);
    result["solidGeometryCertified"] = json!(certified);
    result["resultModelBound"] = json!(result_model_bound);
    Ok(result)
}

#[cfg(test)]
#[path = "cad_miter_owned/tests/tests_fixtures.rs"]
mod tests;
