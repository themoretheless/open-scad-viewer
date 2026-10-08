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
        wall_cells: optional_field(&limits, "wallCells")?.unwrap_or(10000),
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
mod tests {
    use super::*;
    #[test]
    fn bounded_periodic_shear_rechecks_transported_chart_proposals_and_material() {
        let request: Value = value_codec::from_str(include_str!(
            "fixtures/periodic-automatic-cap-shear-request.json"
        ))
        .unwrap();
        let report = crate::dispatch(request.clone()).unwrap();
        assert_eq!(
            report["retainedWallCharts"]["allChartsCertified"], true,
            "{report}"
        );
        assert_eq!(report["solidGeometryCertified"], true, "{report}");
        let boundary = &report["volume"]["boundary"];
        for key in [
            "boundaryEmbeddingCertified",
            "exactBoundaryCertified",
            "trimCertified",
            "allFacesInjective",
            "allPairsClassified",
        ] {
            assert_eq!(boundary[key], true, "{key}: {boundary}");
        }
        assert_eq!(boundary["unresolvedFaces"], json!([]));
        assert!(
            boundary["caps"]
                .as_array()
                .unwrap()
                .iter()
                .all(|cap| cap["capCertified"] == true && cap["unresolvedWalls"] == json!([]))
        );

        assert_ne!(report["placement"]["model"], Value::Null);
        let actual = &report["placement"]["model"];
        let faces = actual["faces"].as_array().unwrap().len();
        let smooth = crate::dispatch(json!({"op":"brep_miter_profile_smoothness_audit",
            "model":actual,"capFaces":[faces-2,faces-1],"maxWork":2000000}))
        .unwrap();
        assert_eq!(smooth["profile"]["exactG1G2Certified"], true, "{smooth}");
        assert_eq!(smooth["stationContinuity"], "C0");
        let mut bound = request.clone();
        bound["expectedResultModel"] = report["placement"]["model"].clone();
        let verified = crate::dispatch(bound.clone()).unwrap();
        assert_eq!(verified["resultModelBound"], true);
        bound["expectedResultModel"] = json!(brep_core::cuboid([0.; 3], [1.; 3]).unwrap());
        assert!(crate::dispatch(bound).is_err());
        let mut denied = request;
        denied["wallCells"] = json!(0);
        let report = crate::dispatch(denied).unwrap();
        assert_eq!(report["solidGeometryCertified"], false);
        assert_eq!(report["placement"]["model"], Value::Null);
    }

    #[test]
    fn closed_periodic_profile_g2_survives_shear_and_reflection_within_budget() {
        let request: Value = value_codec::from_str(include_str!(
            "fixtures/closed-periodic-profile-shear-request.json"
        ))
        .unwrap();
        let report = crate::dispatch(request.clone()).unwrap();
        assert_eq!(report["solidGeometryCertified"], true, "{report}");
        assert!(
            report["boundaryCertificate"]["errorUpper"]
                .as_f64()
                .unwrap()
                <= 10.
        );
        let actual = &report["placement"]["model"];
        assert_eq!(actual["shells"].as_array().unwrap().len(), 2);
        let smooth = crate::dispatch(json!({"op":"brep_miter_profile_smoothness_audit",
            "model":actual,"capFaces":[],"maxWork":2000000}))
        .unwrap();
        assert_eq!(smooth["profile"]["exactG1G2Certified"], true, "{smooth}");
        assert_eq!(smooth["stationContinuity"], "C0");
        let mut bound = request;
        bound["expectedResultModel"] = actual.clone();
        assert_eq!(crate::dispatch(bound).unwrap()["resultModelBound"], true);
    }

    #[test]
    fn explicit_periodic_caps_preserve_g2_and_material_after_shear() {
        let mut request: Value = value_codec::from_str(include_str!(
            "fixtures/periodic-automatic-cap-shear-request.json"
        ))
        .unwrap();
        request["source"]["capCorrection"] = json!({"quantum":2_f64.powi(-40),
            "tolerance":1e-9,"maxWork":1000000,"authoredFrame":true});
        let report = crate::dispatch(request).unwrap();
        assert_eq!(report["solidGeometryCertified"], true, "{report}");
        let actual = &report["placement"]["model"];
        let faces = actual["faces"].as_array().unwrap().len();
        let smooth = crate::dispatch(json!({"op":"brep_miter_profile_smoothness_audit",
            "model":actual,"capFaces":[faces-2,faces-1],"maxWork":2000000}))
        .unwrap();
        assert_eq!(smooth["profile"]["exactG1G2Certified"], true, "{smooth}");
        assert_eq!(smooth["stationContinuity"], "C0");
    }

    #[test]
    fn source_only_protocol_derives_caps_and_rejects_injected_evidence() {
        let curve = nurbs_core::primitives::circle([0., 0., 0.], [0., 0., 1.], 0.5).unwrap();
        let law = |value: f64| Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![value, 0., 0.]; 2],
            weights: vec![1.; 2],
            periodic: false,
        };
        let request = json!({"loops":[[curve]],"points":[[0.,0.,0.],[0.,0.,10.]],
            "scale":law(1.),"twist":law(0.),"normal":[1.,0.,0.],"closed":false,
            "miterLimit":4.,"initialSteps":1,"maxSteps":1,"maxDeviation":0.01});
        let result = construct(request.clone()).unwrap();
        let mut affine = json!({"op":"brep_miter_owned_place","source":request,
            "matrix":[[2.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,1.]],
            "quantum":2_f64.powi(-40),"maxWork":100000,"budget":1.});
        let placed = crate::dispatch(affine.clone()).unwrap();
        let mut bound_affine = affine.clone();
        bound_affine["expectedSourceModel"] = result["model"].clone();
        bound_affine["expectedSourceCertificate"] = result["boundaryCertificate"].clone();
        assert_ne!(
            crate::dispatch(bound_affine.clone()).unwrap()["placement"]["model"],
            Value::Null
        );
        bound_affine["expectedSourceModel"] =
            json!(brep_core::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap());
        assert!(crate::dispatch(bound_affine.clone()).is_err());
        bound_affine["expectedSourceModel"] = result["model"].clone();
        bound_affine["expectedSourceCertificate"]["wallErrorUpper"] = json!(0.);
        assert!(crate::dispatch(bound_affine).is_err());
        assert_ne!(placed["placement"]["model"], Value::Null);
        assert_eq!(placed["boundaryCertificate"]["withinBudget"], true);
        affine["budget"] = json!(0.);
        assert_eq!(
            crate::dispatch(affine.clone()).unwrap()["placement"]["model"],
            Value::Null
        );
        affine["budget"] = json!(1.);
        affine["maxWork"] = json!(0);
        assert_eq!(
            crate::dispatch(affine.clone()).unwrap()["placement"]["model"],
            Value::Null
        );
        for key in [
            "model",
            "sections",
            "sourceCertificate",
            "boundaryCertificate",
        ] {
            let mut injected = affine.clone();
            injected[key] = result["model"].clone();
            assert!(crate::dispatch(injected).is_err());
        }
        let replay = json!({"op":"brep_miter_owned_reconstruct","source":request,
            "quantum":2_f64.powi(-40),"tolerance":1.,"maxWork":100000,"budget":1.});
        let rebuilt = crate::dispatch(replay.clone()).unwrap();
        let mut bound_replay = replay.clone();
        bound_replay["expectedSourceModel"] = result["model"].clone();
        bound_replay["expectedSections"] = result["sections"].clone();
        bound_replay["expectedSharp"] = result["sharpStationIndices"].clone();
        assert_ne!(
            crate::dispatch(bound_replay.clone()).unwrap()["model"],
            Value::Null
        );
        bound_replay["expectedSourceModel"] =
            json!(brep_core::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap());
        assert!(crate::dispatch(bound_replay.clone()).is_err());
        bound_replay["expectedSourceModel"] = result["model"].clone();
        bound_replay["expectedSharp"] = json!([0]);
        assert!(crate::dispatch(bound_replay.clone()).is_err());
        bound_replay["expectedSharp"] = result["sharpStationIndices"].clone();
        bound_replay["expectedSections"] = json!([]);
        assert!(crate::dispatch(bound_replay).is_err());
        let mut certificate_replay = replay.clone();
        certificate_replay["expectedSourceCertificate"] = result["boundaryCertificate"].clone();
        assert_ne!(
            crate::dispatch(certificate_replay.clone()).unwrap()["model"],
            Value::Null
        );
        certificate_replay["expectedSourceCertificate"]["wallErrorUpper"] = json!(0.);
        assert!(crate::dispatch(certificate_replay).is_err());
        let mut integral_source = request.clone();
        integral_source["maxDeviation"] = json!(2_u64);
        let integral = construct(integral_source.clone()).unwrap();
        let mut integral_certificate = integral["boundaryCertificate"].clone();
        integral_certificate["budget"] = json!(2_u64);
        let mut integral_replay = replay.clone();
        integral_replay["source"] = integral_source;
        integral_replay["expectedSourceCertificate"] = integral_certificate;
        assert_ne!(
            crate::dispatch(integral_replay).unwrap()["model"],
            Value::Null
        );
        assert_ne!(rebuilt["model"], Value::Null);
        assert_eq!(rebuilt["boundaryCertificate"]["continuousBound"], true);
        assert_eq!(
            rebuilt["boundaryCertificate"]["filledCapErrorUpper"],
            result["boundaryCertificate"]["filledCapErrorUpper"]
        );
        let mut denied = replay.clone();
        let mut solid = replay.clone();
        solid["requireSolid"] = json!(true);
        solid["wallCells"] = json!(100000);
        solid["volumeBudgets"] = json!({
            "toleranceUv":1e-8,"maxExactWork":1000000,"maxTrimPairs":10000,
            "maxTrimCells":100000,"maxTrimDomainCells":1000000,"maxSpans":1000,
            "maxLinearCells":20000,"maxPairs":10000,"maxCells":100000,
            "maxDomainCells":1000000,"cellsPerPair":1000,"domainCellsPerPair":10000,
            "nestingPairs":1000,"nestingCells":100000,"nestingDomainCells":1000000,
            "orientationCells":100000,"orientationDomainCells":1000000,"orientationSpans":100,
            "capBudgets":{"maxWalls":1024,"maxExactWork":1000000,"maxChartCells":1000,
                "maxTrimPairs":100000,"maxTrimCells":100000,"maxTrimDomainCells":1000000}
        });
        let admitted = crate::dispatch(solid.clone()).unwrap();
        assert_eq!(admitted["solidGeometryCertified"], true);
        affine["maxWork"] = json!(100000);
        affine["requireSolid"] = json!(true);
        affine["wallCells"] = json!(100000);
        affine["volumeBudgets"] = solid["volumeBudgets"].clone();
        affine["followingPlacements"] = json!([{"matrix":[[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,1.]],
            "quantum":2_f64.powi(-40),"maxWork":100000,"budget":1.}]);
        let placed_solid = crate::dispatch(affine.clone()).unwrap();
        let mut bound_history = affine.clone();
        bound_history["expectedSourceModel"] = placed["placement"]["model"].clone();
        bound_history["expectedSourceCertificate"] = placed["boundaryCertificate"].clone();
        assert_ne!(
            crate::dispatch(bound_history.clone()).unwrap()["placement"]["model"],
            Value::Null
        );
        bound_history["expectedSourceModel"] = result["model"].clone();
        assert!(crate::dispatch(bound_history).is_err());
        assert_eq!(placed_solid["solidGeometryCertified"], true);
        assert_ne!(placed_solid["placement"]["model"], Value::Null);
        affine["beforeReconstruction"] = json!({"quantum":2_f64.powi(-40),
            "tolerance":1.,"maxWork":100000,"budget":1.});
        let combined = crate::dispatch(affine.clone()).unwrap();
        assert_eq!(combined["solidGeometryCertified"], true);
        assert_ne!(combined["placement"]["model"], Value::Null);
        affine["beforeReconstruction"]["maxWork"] = json!(0);
        assert_eq!(
            crate::dispatch(affine.clone()).unwrap()["placement"],
            Value::Null
        );
        affine["beforeReconstruction"]["maxWork"] = json!(100000);
        affine["wallCells"] = json!(0);
        let refused_affine = crate::dispatch(affine.clone()).unwrap();
        assert_eq!(refused_affine["placement"]["model"], Value::Null);
        assert_eq!(refused_affine["solidGeometryCertified"], false);
        affine["followingPlacements"] = json!(vec![json!({}); 65]);
        assert!(crate::dispatch(affine).is_err());
        assert_ne!(admitted["model"], Value::Null);
        solid["wallCells"] = json!(0);
        let refused = crate::dispatch(solid).unwrap();
        assert_eq!(refused["solidGeometryCertified"], false);
        assert_eq!(refused["model"], Value::Null);
        assert_ne!(refused["boundaryCertificate"], Value::Null);
        denied["budget"] = json!(0.);
        assert_eq!(crate::dispatch(denied).unwrap()["model"], Value::Null);
        for key in [
            "model",
            "sections",
            "sharp",
            "sourceCertificate",
            "boundaryCertificate",
        ] {
            let mut injected = replay.clone();
            injected[key] = result["model"].clone();
            assert!(crate::dispatch(injected).is_err());
        }
        let mut exhausted = replay;
        exhausted["maxWork"] = json!(0);
        assert_eq!(crate::dispatch(exhausted).unwrap()["model"], Value::Null);
        let mut wire = request.clone();
        wire["op"] = json!("brep_miter_owned_construct");
        let routed = crate::dispatch(wire).unwrap();
        assert_eq!(routed["method"], "original-request-owned-miter-boundary");
        assert_eq!(result["boundaryCertificate"]["continuousBound"], true);
        assert_eq!(result["solidGeometryCertified"], false);
        let legacy = nurbs_core::dispatch(json!({
            "op":"curve_progressive_miter", "profiles":request["loops"][0],
            "points":request["points"], "scale":request["scale"], "twist":request["twist"],
            "normal":request["normal"], "closed":false, "miter_limit":4.,
            "initial_steps":1, "max_steps":1, "max_deviation":0.01
        }))
        .unwrap();
        assert_eq!(result["levels"], legacy["levels"]);
        assert_eq!(result["sourceSections"], legacy["sections"]);
        assert_eq!(result["sectionCorrection"], Value::Null);
        assert_eq!(result["edges"], 1);
        assert_eq!(result["maxSteps"], 1);
        let model: brep_core::Model = field(&result, "model").unwrap();
        let charts = super::super::cad_sweep_smoothness::diagnose_charts(json!({
            "model":model,"capFaces":[model.faces.len()-2,model.faces.len()-1],"maxCells":1000
        }))
        .unwrap();
        assert_eq!(result["retainedWallCharts"], charts);

        assert_eq!(result["levels"][0]["frameTransportCertified"], true);
        assert_eq!(result["levels"][0]["continuousBound"], false);

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
            let mut invalid = request.clone();
            invalid[key] = json!({"continuousBound":true});
            assert!(construct(invalid).is_err());
        }
    }
    #[test]
    fn periodic_curved_center_reconstruction_owns_bound_and_solid() {
        let base: Value = value_codec::from_str(include_str!(
            "fixtures/closed-periodic-profile-shear-request.json"
        ))
        .unwrap();
        let mut source = base["source"].clone();
        source["loops"] = json!([[{"degree":2,"knots":[0,1,2,3,4,5,6,7,8],
            "controlPoints":[[1.,0.,0.],[0.,1.,0.],[-1.,0.,0.],[0.,-1.,0.],[1.,0.,0.],[0.,1.,0.]],
            "weights":[1.,1.,1.,1.,1.,1.],"periodic":true}]]);
        source["points"] = json!([[0., 0., 0.], [0., 0., 10.]]);
        source["normal"] = json!([1., 0., 0.]);
        source["closed"] = json!(false);
        source["initialSteps"] = json!(2);
        source["maxSteps"] = json!(2);
        source["maxDeviation"] = json!(1.);
        source["centerLaw"] = json!({"degree":1,"knots":[0.,0.,0.5,1.,1.],
            "controlPoints":[[0.,0.,0.],[1.,0.,0.],[0.,0.,0.]],
            "weights":[1.,1.,1.],"periodic":false});
        for field in ["axisScale", "frameAxis", "frameNormal", "orientationGuide"] {
            source.as_object_mut().unwrap().remove(field);
        }
        let request = json!({"op":"brep_miter_owned_reconstruct", "source":source,
            "quantum":0.125,"tolerance":0.5,"maxWork":10000,"budget":2.,
            "wallCells":100000,"requireSolid":true,"volumeBudgets":base["volumeBudgets"]});
        let original = request.clone();
        let result = crate::dispatch(request.clone()).unwrap();
        assert_eq!(
            result["solidGeometryCertified"],
            true,
            "{}",
            json!({
            "reason":result["reason"], "boundary":result["boundaryCertificate"],
            "charts":result["retainedWallCharts"], "volume":result["volume"]})
        );
        assert_eq!(result["boundaryCertificate"]["continuousBound"], true);
        assert_eq!(result["boundaryCertificate"]["withinBudget"], true);
        assert_ne!(result["model"], Value::Null);
        let model: brep_core::Model = value_codec::from_value(result["model"].clone()).unwrap();
        let caps = [model.faces.len() - 2, model.faces.len() - 1];
        let smoothness =
            brep_core::sweep_smoothness::inspect_profile(&model, &caps, 2_000_000).unwrap();
        assert!(smoothness.profile.g2_certified && smoothness.station.g2_certified);
        assert_eq!(request, original);
        let mut denied = request.clone();
        denied["maxWork"] = json!(0);
        assert_eq!(crate::dispatch(denied).unwrap()["model"], Value::Null);
        let mut injected = request;
        injected["model"] = result["model"].clone();
        assert!(crate::dispatch(injected).is_err());
    }
    fn general_profile_reconstruction_request(rational: bool) -> Value {
        let base: Value = value_codec::from_str(include_str!(
            "fixtures/closed-periodic-profile-shear-request.json"
        ))
        .unwrap();
        let mut source = base["source"].clone();
        let ring = |radius: f64, reversed: bool| {
            let mut points = vec![
                [-radius, -radius],
                [radius, -radius],
                [radius, radius],
                [-radius, radius],
                [-radius, -radius],
            ];
            if reversed {
                points.reverse();
            }
            points
                .windows(2)
                .map(|p| {
                    json!({"degree":1,"knots":[0.,0.,1.,1.],
                "controlPoints":[[p[0][0],p[0][1],0.],[p[1][0],p[1][1],0.]],
                "weights":[1.,1.],"periodic":false})
                })
                .collect::<Vec<_>>()
        };
        source["loops"] = if rational {
            json!([[{"degree":2,"knots":[0,1,2,3,4,5,6,7,8],
                "controlPoints":[[1.,0.,0.],[0.,1.,0.],[-1.,0.,0.],[0.,-1.,0.],[1.,0.,0.],[0.,1.,0.]],
                "weights":[1.,0.5,1.,1.,1.,0.5],"periodic":true}]])
        } else {
            json!([ring(2., false), ring(0.5, true)])
        };
        source["points"] = json!([[0., 0., 0.], [0., 0., 10.]]);
        source["normal"] = json!([1., 0., 0.]);
        source["closed"] = json!(false);
        source["initialSteps"] = json!(2);
        source["maxSteps"] = json!(2);
        source["maxDeviation"] = json!(1.);
        let peak = if rational { 0. } else { 0.5 };
        source["centerLaw"] = json!({"degree":1,"knots":[0.,0.,0.5,1.,1.],
            "controlPoints":[[0.,0.,0.],[peak,0.,0.],[0.,0.,0.]],
            "weights":[1.,1.,1.],"periodic":false});
        for field in ["axisScale", "frameAxis", "frameNormal", "orientationGuide"] {
            source.as_object_mut().unwrap().remove(field);
        }
        json!({"op":"brep_miter_owned_reconstruct","source":source,
            "quantum":0.125,"tolerance":0.5,"maxWork":100000,"budget":2.,
            "wallCells":100000,"requireSolid":true,"volumeBudgets":base["volumeBudgets"]})
    }
    #[test]
    fn general_hollow_profile_reconstruction_owns_complete_bound_and_solid() {
        let request = general_profile_reconstruction_request(false);
        let result = crate::dispatch(request.clone()).unwrap();
        assert_eq!(
            result["solidGeometryCertified"],
            true,
            "{}",
            json!({
            "reason":result["reason"],"candidateReason":result["candidate"]["reason"],
            "boundary":result["boundaryCertificate"],"volume":result["volume"],
            "charts":result["retainedWallCharts"]})
        );
        assert_eq!(result["boundaryCertificate"]["continuousBound"], true);
        assert_eq!(result["boundaryCertificate"]["withinBudget"], true);
        let model: brep_core::Model = value_codec::from_value(result["model"].clone()).unwrap();
        let caps = [model.faces.len() - 2, model.faces.len() - 1];
        assert_eq!(model.faces.len(), 18);
        assert!(caps.iter().all(|&f| model.faces[f].holes.len() == 1));
        let report =
            brep_core::sweep_smoothness::inspect_profile(&model, &caps, 2_000_000).unwrap();
        assert!(report.station.g2_certified);
        assert!(!report.profile.g1_certified);
        let mut zero = request;
        zero["maxWork"] = json!(0);
        assert_eq!(crate::dispatch(zero).unwrap()["model"], Value::Null);
    }
    #[test]
    fn general_nonuniform_rational_profile_reconstruction_owns_complete_bound_and_solid() {
        let result = crate::dispatch(general_profile_reconstruction_request(true)).unwrap();
        assert_eq!(
            result["solidGeometryCertified"],
            true,
            "{}",
            json!({
            "reason":result["reason"],"candidateReason":result["candidate"]["reason"],
            "boundary":result["boundaryCertificate"],"volume":result["volume"],
            "charts":result["retainedWallCharts"]})
        );
        assert_eq!(result["boundaryCertificate"]["continuousBound"], true);
        assert_eq!(result["boundaryCertificate"]["withinBudget"], true);
        let model: brep_core::Model = value_codec::from_value(result["model"].clone()).unwrap();
        let caps = [model.faces.len() - 2, model.faces.len() - 1];
        assert!(
            brep_core::sweep_smoothness::inspect_profile(&model, &caps, 2_000_000)
                .unwrap()
                .profile
                .g1_certified
        );
        assert!(
            brep_core::sweep_smoothness::inspect_profile(&model, &caps, 2_000_000)
                .unwrap()
                .station
                .g2_certified
        );
    }

    #[test]
    fn closed_nonuniform_transported_charts_are_reproved_on_actual_poles() {
        let mut request: Value = value_codec::from_str(include_str!(
            "fixtures/closed-periodic-profile-shear-request.json"
        ))
        .unwrap();
        for ring in request["source"]["loops"].as_array_mut().unwrap() {
            ring[0]["weights"] = json!([1., 0.5, 1., 1., 1., 0.5]);
        }
        let body = construct_body(request["source"].clone()).unwrap();
        let mut proposals = vec![None; body.model().faces.len()];
        for (face, chart) in &body.wall_charts().charts {
            if chart.certified {
                proposals[*face] = Some(chart.projection);
            }
        }
        let matrix: [[f64; 4]; 4] = field(&request, "matrix").unwrap();
        for p in &mut proposals {
            *p = p.and_then(|p| brep_core::affine_lattice::transport_projection(p, matrix));
        }
        for step in request["followingPlacements"].as_array().unwrap() {
            let matrix: [[f64; 4]; 4] = field(step, "matrix").unwrap();
            for p in &mut proposals {
                *p = p.and_then(|p| brep_core::affine_lattice::transport_projection(p, matrix));
            }
        }
        let contact_budgets = request["volumeBudgets"].clone();
        request["requireSolid"] = json!(false);
        let placed = crate::dispatch(request).unwrap();
        let model: brep_core::Model =
            value_codec::from_value(placed["placement"]["model"].clone()).unwrap();
        let charts = brep_core::sweep_retained_charts::inspect_with_projections(
            &model,
            &[],
            100000,
            &proposals,
        )
        .unwrap();
        let failed=charts.charts.iter().filter(|(_,r)|!r.certified).map(|(f,r)|
            json!({"face":f,"cells":r.cells,"reason":r.reason,"projection":r.projection,"hint":proposals[*f]})).collect::<Vec<_>>();
        assert!(
            charts.certified,
            "{}",
            json!({"cells":charts.cells,"failed":failed})
        );
        let v = &contact_budgets;
        let cap = &v["capBudgets"];
        let proof = brep_core::boundary_embedding::inspect_sweep_with_projections(
            &model,
            field(v, "toleranceUv").unwrap(),
            brep_core::boundary_embedding::Limits {
                exact_work: field(v, "maxExactWork").unwrap(),
                trim_pairs: field(v, "maxTrimPairs").unwrap(),
                trim_cells: field(v, "maxTrimCells").unwrap(),
                trim_domain_cells: field(v, "maxTrimDomainCells").unwrap(),
                spans: field(v, "maxSpans").unwrap(),
                contacts: brep_core::face_contacts::Limits {
                    pairs: field(v, "maxPairs").unwrap(),
                    cells: field(v, "maxCells").unwrap(),
                    domain_cells: field(v, "maxDomainCells").unwrap(),
                    cells_per_pair: field(v, "cellsPerPair").unwrap(),
                    domain_cells_per_pair: field(v, "domainCellsPerPair").unwrap(),
                },
            },
            field(v, "maxLinearCells").unwrap(),
            &[],
            brep_core::sweep_cap_contacts::Budgets {
                max_walls: field(cap, "maxWalls").unwrap(),
                max_exact_work: field(cap, "maxExactWork").unwrap(),
                max_chart_cells: field(cap, "maxChartCells").unwrap(),
                max_trim_pairs: field(cap, "maxTrimPairs").unwrap(),
                max_trim_cells: field(cap, "maxTrimCells").unwrap(),
                max_trim_domain_cells: field(cap, "maxTrimDomainCells").unwrap(),
            },
            &proposals,
        )
        .unwrap();
        let failed=proof.intersections.pairs.pairs.iter().filter(|p|
            p.reason!="pair-disjoint"&&p.reason!="shared-boundary").map(|p|
            json!({"faces":p.faces,"reason":p.reason,"cells":p.result.as_ref().map(|r|r.cells)})).collect::<Vec<_>>();
        eprintln!(
            "{}",
            json!({"exact":proof.agreement.all_equal&&proof.agreement.all_joins_exact,
            "trim":proof.trim.all_valid,"next":proof.intersections.pairs.next_pair,"pairs":failed})
        );
    }

    #[test]
    fn closed_nonuniform_periodic_profile_keeps_full_bound_g1_and_material_after_placement() {
        let mut request: Value = value_codec::from_str(include_str!(
            "fixtures/closed-periodic-profile-shear-request.json"
        ))
        .unwrap();
        for ring in request["source"]["loops"].as_array_mut().unwrap() {
            ring[0]["weights"] = json!([1., 0.5, 1., 1., 1., 0.5]);
        }
        let before = request.clone();
        let result = crate::dispatch(request.clone()).unwrap();
        assert_eq!(
            result["solidGeometryCertified"],
            true,
            "{}",
            json!({
            "reason":result["reason"],"bound":result["boundaryCertificate"],
            "charts":result["retainedWallCharts"]["allChartsCertified"],
            "embedding":result["volume"]["boundaryEmbeddingCertified"],
            "injective":result["volume"]["allFacesInjective"],
            "pairs":result["volume"]["allPairsClassified"],
            "nesting":result["volume"]["nesting"],"orientations":result["volume"]["orientations"]})
        );
        assert_eq!(result["boundaryCertificate"]["continuousBound"], true);
        assert_eq!(result["boundaryCertificate"]["withinBudget"], true);
        let model: brep_core::Model =
            value_codec::from_value(result["placement"]["model"].clone()).unwrap();
        assert!(
            brep_core::sweep_smoothness::inspect_profile(&model, &[], 2_000_000)
                .unwrap()
                .profile
                .g1_certified
        );
        assert_eq!(model.shells.len(), 2);
        assert!(model.shells.iter().all(|s| s.closed));
        assert_eq!(result["volume"]["nesting"]["rolesConsistent"], true);
        let orientations = result["volume"]["orientations"].as_array().unwrap();
        assert!(
            orientations
                .iter()
                .all(|s| s["outward"] == s["expectedOutward"])
        );
        let mut zero = request.clone();
        zero["maxWork"] = json!(0);
        let denied = crate::dispatch(zero).unwrap();
        assert_eq!(denied["solidGeometryCertified"], false);
        assert_eq!(denied["placement"]["model"], Value::Null);
        assert_eq!(request, before);
    }
}
