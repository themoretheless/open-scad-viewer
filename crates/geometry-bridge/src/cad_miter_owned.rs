//! Original source request transport. No registration of caller-authored bounds.
use super::{Result, Value, field, optional_field};
use brep_core::sweep_miter_owned::{self, Limits, Request};
use nurbs_core::{
    curve::Curve,
    miter_section_correction::{Budget, CapCorrection},
    progressive_miter,
};
use value_codec::json;
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
                    .all(|(k, a)| b.get(k).is_some_and(|b| same_certificate(a, b)))
        }
        _ => a == b,
    }
}
fn construct_body(v: Value) -> Result<sweep_miter_owned::BoundaryBody> {
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
    let scale: Curve = field(&v, "scale")?;
    let twist: Curve = field(&v, "twist")?;
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
    let circle: Option<Value> = optional_field(&v, "circleCorrection")?;
    let cap: Option<Value> = optional_field(&v, "capCorrection")?;
    let limits: Value = optional_field(&v, "limits")?.unwrap_or(json!({
        "maxProducts":100000,"maxFaces":1024,"exactWork":1000000,
        "domainTolerance":0.001,"domainPairs":1000,"domainCells":10000,"projectionCells":10000,
        "capRegions":{"maxWalls":1024,"maxExactWork":1000000,"maxChartCells":1000,
            "maxTrimPairs":100000,"maxTrimCells":100000,"maxTrimDomainCells":1000000}}));
    let cap_regions: Value = field(&limits, "capRegions")?;
    let request = Request {
        loops: &loops,
        points: &points,
        scale: &scale,
        twist: &twist,
        options: progressive_miter::Options {
            normal: field(&v, "normal")?,
            closed: field(&v, "closed")?,
            miter_limit: field(&v, "miterLimit")?,
            initial_steps: field(&v, "initialSteps")?,
            max_steps: field(&v, "maxSteps")?,
            max_deviation: field(&v, "maxDeviation")?,
        },
        affine: use_affine.then_some((&axes, &center)),
        frames: axis.as_ref().zip(normal.as_ref()),
        guide: guide.as_ref(),
        circle: circle.as_ref().map(budget).transpose()?,
        caps: cap
            .as_ref()
            .map(|c| {
                Ok::<_, nurbs_core::Error>(CapCorrection {
                    budget: budget(c)?,
                    authored_frame: optional_field(c, "authoredFrame")?.unwrap_or(false),
                })
            })
            .transpose()?,
    };
    let limits = Limits {
        max_products: field(&limits, "maxProducts")?,
        max_faces: field(&limits, "maxFaces")?,
        correspondence_faces: optional_field(&limits, "correspondenceMaxFaces")?
            .unwrap_or(field(&limits, "maxFaces")?),
        cap_max_edges: optional_field(&limits, "capMaxEdges")?
            .unwrap_or(field(&limits, "maxFaces")?),
        wall_cells: optional_field(&limits, "wallCells")?.unwrap_or(sweep_miter_owned::DEFAULT_WALL_CELLS),
        exact_work: field(&limits, "exactWork")?,
        domain_exact_work: optional_field(&limits, "domainExactWork")?
            .unwrap_or(field(&limits, "exactWork")?),
        projection_exact_work: optional_field(&limits, "projectionExactWork")?
            .unwrap_or(field(&limits, "exactWork")?),
        domain_tolerance: field(&limits, "domainTolerance")?,
        domain_pairs: field(&limits, "domainPairs")?,
        domain_cells: field(&limits, "domainCells")?,
        projection_cells: field(&limits, "projectionCells")?,
        cap_regions: brep_core::sweep_cap_contacts::Budgets {
            max_walls: field(&cap_regions, "maxWalls")?,
            max_exact_work: field(&cap_regions, "maxExactWork")?,
            max_chart_cells: field(&cap_regions, "maxChartCells")?,
            max_trim_pairs: field(&cap_regions, "maxTrimPairs")?,
            max_trim_cells: field(&cap_regions, "maxTrimCells")?,
            max_trim_domain_cells: field(&cap_regions, "maxTrimDomainCells")?,
        },
    };
    sweep_miter_owned::construct(&request, &limits)
}
pub fn construct(v: Value) -> Result<Value> {
    let body = construct_body(v)?;
    let bound = body.boundary();
    let correction = body.correction().map(|r| {
        json!({"wallDisplacementUpper":r.wall_displacement_upper,
        "exactPlanarSections":r.exact_planar_sections,"work":r.work,"reason":r.reason})
    });
    Ok(
        json!({"method":"original-request-owned-miter-boundary","model":body.model(),"sections":body.sections(),
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
            volume = Some(super::cad_face_contacts::compact_volume_report(proof));
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
            volume = Some(super::cad_face_contacts::compact_volume_report(proof));
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
#[path = "tests/cad_miter_owned.rs"]
mod tests;
