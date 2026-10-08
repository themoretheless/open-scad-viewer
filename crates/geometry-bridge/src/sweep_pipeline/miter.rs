use super::*;
pub fn progressive_miter(v: Value) -> Result<Value> {
    progressive_miter_from(v, None)
}
pub(super) fn progressive_miter_from(v: Value, accepted: Option<Value>) -> Result<Value> {
    let loops: Vec<Vec<Value>> = field(&v, "loops")?;
    if loops.is_empty() || loops.len() > 16 || loops.iter().any(Vec::is_empty) {
        return Err(input("Miter body needs 1..16 nonempty loops"));
    }
    let profiles = json!(loops.iter().flatten().cloned().collect::<Vec<_>>());
    let sizes: Vec<_> = loops.iter().map(Vec::len).collect();
    let o = &v["options"];
    let closed = yes(o, "closed");
    let points: Vec<[f64; 3]> = field(&v, "points")?;
    let edges = points
        .len()
        .checked_sub(usize::from(!closed))
        .ok_or_else(|| input("Invalid miter sites"))?;
    let mut spans = 0;
    for profile in profiles.as_array().unwrap() {
        spans += nurbs("curve_decompose", json!({"curve":profile}))?
            .as_array()
            .ok_or_else(|| input("Invalid native decomposition"))?
            .len();
    }
    if edges < 1 || spans < 1 || spans > 64 {
        return Err(input("Progressive miter body exceeds its site/span budget"));
    }
    let mut request = miter_request(&v, &profiles)?;
    let maximum = field::<usize>(&request, "max_steps")?
        .min(1024 / edges)
        .min((1024 - if closed { 0 } else { 2 }) / (edges * spans));
    if maximum < field::<usize>(&request, "initial_steps")? {
        return Err(input("Progressive miter initial steps exceed face budget"));
    }
    request["max_steps"] = json!(maximum);
    let approximation = match accepted {
        Some(level) => level,
        None => nurbs("curve_progressive_miter", request.clone())?,
    };
    let report = &approximation["report"];
    if report["frameTransportCertified"] == json!(false) {
        return Err(input(
            "Progressive miter frame transport could not be proved; review the path, normal and miter limit",
        ));
    }
    if report["certifiedErrorUpper"].is_null() {
        return Err(input(format!(
            "Progressive miter wall error bound could not be proved: {}",
            report["errorCertificateReason"]
        )));
    }
    if report["profileRegularityCertified"] == json!(false) {
        return Err(input(
            "Progressive miter profile tangent regularity could not be proved",
        ));
    }
    if report["wallRegularityCertified"] == json!(false) {
        return Err(input(
            "Progressive miter retained wall Jacobian regularity could not be proved",
        ));
    }
    if !yes(report, "accepted") || approximation["sections"].is_null() {
        return Err(input(
            "Progressive miter refinement/phase budget not met within the body face budget",
        ));
    }
    let mut correction =
        correct(json!({"sections":approximation["sections"],"points":points,"options":o}))?;
    let retained = if correction.is_null() {
        approximation["sections"].clone()
    } else {
        correction["sections"].clone()
    };
    if let Some(fields) = correction.as_object_mut() {
        fields.remove("sections");
    }
    let wall_defaults = json!({"clearance":0.,"distanceTolerance":0.001,"maxInjectivityCells":1000,"maxPairs":1000,"maxPairCells":1000});
    let wall_request = merge(
        merge(
            request.clone(),
            &json!({"sections":retained,"loopSizes":sizes}),
        ),
        &value(o, "wallAuditBudgets", wall_defaults.clone()),
    );
    let wall_audit = nurbs("curve_progressive_miter_wall_audit", wall_request)?;
    let sections = partition(&retained, &sizes)?;
    let model = call(
        if closed {
            "brep_nurbs_periodic_section_loft"
        } else {
            "brep_nurbs_rational_section_loft"
        },
        json!({"sections":sections}),
    )?;
    let mut cap_domains = Value::Null;
    if !closed {
        let rows = sections.as_array().unwrap();
        let budgets = value(
            o,
            "contourAuditBudgets",
            json!({"tolerance":0.001,"maxPairs":1000,"maxCells":1000}),
        );
        let mut reports = Vec::new();
        for row in [&rows[0], rows.last().unwrap()] {
            let mut req = merge(json!({"loops":row}), &budgets);
            req["maxExactWork"] = json!(1000000);
            reports.push(nurbs("sweep_contour_audit", req)?);
        }
        cap_domains = json!(reports);
    }
    let correspondence = call(
        "brep_sweep_retained_correspondence_audit",
        json!({"model":model,"sections":sections,"closed":closed,"maxFaces":1024,"maxExactWork":1000000}),
    )?;
    let decomposition = if yes(&correspondence, "exact") {
        Value::Null
    } else {
        call(
            "brep_sweep_retained_decomposition_audit",
            json!({"model":model,"sections":sections,"closed":closed,"maxProducts":value(&o["retainedDecompositionBudgets"],"maxProducts",json!(100000)),"maxFaces":value(&o["retainedDecompositionBudgets"],"maxFaces",json!(1024)),"maxExactWork":1000000}),
        )?
    };
    let decomp_error = if yes(&correspondence, "exact") {
        json!(0)
    } else {
        decomposition["wallErrorUpper"].clone()
    };
    if !correction.is_null() && decomp_error.is_null() {
        return Err(input(
            "Corrected progressive miter retained wall correspondence unproved",
        ));
    }
    let wall_error = add(
        add(
            report["certifiedErrorUpper"].clone(),
            value(&correction, "wallDisplacementUpper", json!(0)),
        )?,
        decomp_error,
    )?;
    let budget: f64 = field(o, "maxDeviation")?;
    if !correction.is_null() && (wall_error.is_null() || wall_error.as_f64().unwrap() > budget) {
        return Err(input(
            "Corrected progressive miter retained wall error exceeds max_deviation or is unproved",
        ));
    }
    let caps = cap_faces(&model, closed)?;
    let volume_budgets = value(o, "volumeBudgets", volume_budgets());
    let cap_budgets = &volume_budgets["capBudgets"];
    let rows = sections.as_array().unwrap();
    let endpoints = json!([rows[0], rows.last().unwrap()]);
    let retained_caps = if closed {
        Value::Null
    } else {
        call(
            "brep_sweep_retained_caps_audit",
            merge(
                json!({"model":model,"endpoints":endpoints,"maxEdges":1024}),
                cap_budgets,
            ),
        )?
    };
    let cap_decomposition = if closed || yes(&retained_caps, "exact") {
        Value::Null
    } else {
        call(
            "brep_sweep_retained_cap_decomposition_audit",
            merge(
                json!({"model":model,"endpoints":endpoints,"maxProducts":value(&o["retainedDecompositionBudgets"],"maxProducts",json!(100000)),"maxEdges":1024}),
                cap_budgets,
            ),
        )?
    };
    if !correction.is_null()
        && !closed
        && !yes(&retained_caps, "exact")
        && !yes(&cap_decomposition, "certified")
    {
        return Err(input(
            "Corrected progressive miter filled retained cap regions unproved",
        ));
    }
    let ideal_domains = if closed {
        Value::Null
    } else {
        nurbs(
            "curve_progressive_miter_cap_domains",
            merge(
                merge(request.clone(), &json!({"loopSizes":sizes})),
                &value(
                    o,
                    "capDomainBudgets",
                    json!({"tolerance":0.001,"maxPairs":1000,"maxCells":10000,"maxExactWork":1000000}),
                ),
            ),
        )?
    };
    let mut projection = Value::Null;
    let mut parallelism = Value::Null;
    if !closed {
        let faces = model["faces"].as_array().unwrap();
        let req = merge(
            request.clone(),
            &json!({"caps":[faces[faces.len()-2]["surface"],faces[faces.len()-1]["surface"]],"maxCells":value(&o["capProjectionBudgets"],"maxCells",json!(10000)),"maxExactWork":value(&o["capProjectionBudgets"],"maxExactWork",json!(1000000))}),
        );
        projection = nurbs("curve_progressive_miter_cap_projection", req.clone())?;
        parallelism = nurbs("curve_progressive_miter_cap_parallelism", req)?;
    }
    let cap_error = if closed {
        Value::Null
    } else {
        nurbs("sweep_filled_cap_error_upper",json!({"parallelPlanesCertified":parallelism["parallel"],"idealCapDomainsCertified":yes(&ideal_domains,"idealCapDomainsCertified"),"retainedCapRegionsExact":yes(&retained_caps,"exact")||yes(&cap_decomposition,"certified"),"decompositionErrorUpper":if yes(&retained_caps,"exact"){json!([0,0])}else{cap_decomposition["capErrorUpper"].clone()},"projectionNormalDots":projection["normalDots"],"endpointContourErrorUpper":report["endpointContourErrorUpper"],"correctionDisplacementUpper":value(&correction,"wallDisplacementUpper",json!(0))}))?["capErrorUpper"].clone()
    };
    let certificate = nurbs(
        "sweep_boundary_certificate",
        json!({"wall":wall_error,"caps":cap_error,"closed":closed,"budget":budget}),
    )?;
    if certificate["withinBudget"] == json!(false) {
        return Err(input(
            "Progressive miter complete boundary error exceeds max_deviation",
        ));
    }
    let charts = call(
        "brep_sweep_retained_wall_charts_audit",
        json!({"model":model,"capFaces":caps,"maxCells":value(o,"retainedWallMaxInjectivityCells",json!(1000))}),
    )?;
    if !correction.is_null() && !yes(&charts, "allChartsCertified") {
        return Err(input(
            "Corrected progressive miter retained wall regularity unproved",
        ));
    }
    let cap_pairs = if closed {
        Value::Null
    } else {
        call(
            "brep_sweep_cap_pairs_audit",
            json!({"model":model,"capFaces":caps,"budgets":value(o,"capPairAuditBudgets",wall_defaults)}),
        )?
    };
    let contacts = if closed {
        Value::Null
    } else {
        call(
            "brep_sweep_cap_evidence_audit",
            json!({"model":model,"capFaces":caps,"maxWalls":value(o,"capWallMaxWalls",json!(1024)),"boundaryOptions":{"tolerance":1e-9,"maxProducts":100000,"maxCells":1000,"maxWork":1000000}}),
        )?
    };
    let mut embedding_budgets = volume_budgets.clone();
    for (k, val) in [
        ("maxTrimPairs", 1000),
        ("maxTrimCells", 10000),
        ("maxTrimDomainCells", 100000),
        ("maxLinearCells", 1000),
        ("maxPairs", 1000),
        ("maxCells", 10000),
        ("maxDomainCells", 100000),
        ("cellsPerPair", 16),
        ("domainCellsPerPair", 1000),
    ] {
        embedding_budgets[k] = json!(val);
    }
    let embedding = if closed {
        Value::Null
    } else {
        call(
            "brep_sweep_embedding_audit",
            merge(
                json!({"model":model,"capFaces":caps}),
                &value(o, "embeddingBudgets", embedding_budgets),
            ),
        )?
    };
    let volume = call(
        "brep_sweep_volume_audit",
        merge(json!({"model":model,"capFaces":caps}), &volume_budgets),
    )?;
    let smoothness = call(
        "brep_miter_profile_smoothness_audit",
        json!({"model":model,"capFaces":caps,"maxWork":2000000}),
    )?;
    let authoring = json!({"authoredFramesApplied":report["authoredFramesApplied"],"orientationGuideApplied":report["orientationGuideApplied"],"affineLawsApplied":report["affineLawsApplied"]});
    let steps: usize = field(report, "steps")?;
    if rows.len() != edges * steps + 1 {
        return Err(input(
            "Miter station ownership requires complete uniform span coverage",
        ));
    }
    let sharp: Vec<_> = (0..if closed { edges } else { edges - 1 })
        .map(|i| (i + usize::from(!closed)) * steps)
        .collect();
    let mut body = json!({"model":model,"approximation":approximation,"profileSmoothness":smoothness,"boundaryCertificate":certificate,"retainedCapDecomposition":cap_decomposition,"retainedDecomposition":decomposition,"capParallelism":parallelism,"boundaryErrorWithinBudget":certificate["withinBudget"],"boundaryErrorUpper":certificate["errorUpper"],"filledCapErrorUpper":cap_error,"idealCapDomains":ideal_domains,"capProjection":projection,"retainedWallErrorUpper":wall_error,"wallAudit":wall_audit,"retainedCorrespondence":correspondence,"retainedCaps":retained_caps,"retainedWallCharts":charts,"capDomains":cap_domains,"capContacts":contacts,"capPairs":cap_pairs,"embedding":embedding,"volume":volume,"globalEmbeddingCertified":false});
    if !correction.is_null() {
        body["sectionCorrection"] = correction;
    }
    own(body, Some(sections), Some(json!(sharp)), authoring)
}
pub fn miter(v: Value) -> Result<Value> {
    let loops: Vec<Vec<Value>> = field(&v, "loops")?;
    if loops.is_empty() || loops.len() > 16 || loops.iter().any(Vec::is_empty) {
        return Err(input("Miter body needs 1..16 nonempty loops"));
    }
    let profiles = json!(loops.iter().flatten().cloned().collect::<Vec<_>>());
    let sizes: Vec<_> = loops.iter().map(Vec::len).collect();
    let closed = yes(&v, "closed");
    let sections = nurbs(
        "curve_miter_sections",
        json!({"profiles":profiles,"points":v["points"],"normal":v["normal"],"miter_limit":value(&v,"miterLimit",json!(4)),"closed":closed}),
    )?;
    let mut correction = if v["capCorrection"].is_null() {
        Value::Null
    } else {
        cap_correction(
            &sections,
            &field::<Vec<[f64; 3]>>(&v, "points")?,
            closed,
            &v["capCorrection"],
        )?
    };
    let retained = if correction.is_null() {
        sections
    } else {
        correction["sections"].clone()
    };
    if let Some(fields) = correction.as_object_mut() {
        fields.remove("sections");
    }
    let nested = partition(&retained, &sizes)?;
    let model = call(
        if closed {
            "brep_nurbs_periodic_section_loft"
        } else {
            "brep_nurbs_rational_section_loft"
        },
        json!({"sections":nested}),
    )?;
    let correspondence = if correction.is_null() {
        Value::Null
    } else {
        call(
            "brep_sweep_retained_correspondence_audit",
            json!({"model":model,"sections":nested,"closed":closed,"maxFaces":1024,"maxExactWork":1000000}),
        )?
    };
    if !correction.is_null() && !yes(&correspondence, "exact") {
        return Err(input(
            "Corrected miter retained wall correspondence unproved",
        ));
    }
    let caps = cap_faces(&model, closed)?;
    let volume = call(
        "brep_sweep_volume_audit",
        merge(json!({"model":model,"capFaces":caps}), &volume_budgets()),
    )?;
    let regularity = nurbs(
        "sweep_profile_regularity_audit",
        json!({"profiles":profiles,"maxCells":10000}),
    )?;
    let charts = call(
        "brep_sweep_retained_wall_charts_audit",
        json!({"model":model,"capFaces":caps,"maxCells":20000}),
    )?;
    let smoothness = call(
        "brep_miter_profile_smoothness_audit",
        json!({"model":model,"capFaces":caps,"maxWork":2000000}),
    )?;
    let mut report = json!({"profileSmoothness":smoothness,"method":"polyline-miter-sections","sections":retained.as_array().unwrap().len(),"closedPath":closed,"globalEmbeddingCertified":false,"roundingCertified":false,"continuousBound":false,"profileRegularityCertified":regularity["spanwiseRegular"],"profileRegularity":regularity,"wallRegularityCertified":charts["allChartsCertified"],"retainedWallCharts":charts,"volume":volume});
    if !correction.is_null() {
        report["sectionCorrection"] = correction;
        report["retainedCorrespondence"] = correspondence;
    }
    Ok(json!({"model":model,"report":report}))
}
pub(super) fn audit_placed(model: &Value, certificate: &Value) -> Result<(Value, Value, Value)> {
    let caps = cap_faces(model, yes(certificate, "closed"))?;
    let smoothness = call(
        "brep_miter_profile_smoothness_audit",
        json!({"model":model,"capFaces":caps,"maxWork":2000000}),
    )?;
    let charts = call(
        "brep_sweep_retained_wall_charts_audit",
        json!({"model":model,"capFaces":caps,"maxCells":100000}),
    )?;
    let volume = call(
        "brep_sweep_volume_audit",
        merge(json!({"model":model,"capFaces":caps}), &volume_budgets()),
    )?;
    if !yes(&charts, "allChartsCertified") || !yes(&volume, "solidGeometryCertified") {
        return Err(input("Reconstructed Solid geometry unproved"));
    }
    Ok((smoothness, charts, volume))
}
pub(super) fn complete(certificate: &Value, message: &str) -> Result<()> {
    if !yes(certificate, "continuousBound") || !yes(certificate, "withinBudget") {
        return Err(input(message));
    }
    Ok(())
}
pub(super) fn boundary_fields(body: Value, certificate: &Value) -> Value {
    merge(
        body,
        &json!({"boundaryCertificate":certificate,"continuousBound":certificate["continuousBound"],"boundaryErrorUpper":certificate["errorUpper"],"boundaryErrorWithinBudget":certificate["withinBudget"],"budget":certificate["budget"],"filledCapErrorUpper":certificate["filledCapErrorUpper"],"retainedWallErrorUpper":certificate["wallErrorUpper"],"globalEmbeddingCertified":false}),
    )
}
pub fn transform(v: Value) -> Result<Value> {
    let source = owner(&v)?;
    let o = &v["options"];
    complete(
        &source.certificate,
        "Affine source complete boundary bound unproved",
    )?;
    let placement = call(
        "brep_nurbs_affine_lattice",
        json!({"model":source.model,"matrix":v["matrix"],"quantum":o["quantum"],"maxWork":o["maxWork"]}),
    )?;
    if placement["model"].is_null()
        || placement["operatorNormUpper"].is_null()
        || placement["arithmeticErrorUpper"].as_f64() != Some(0.)
    {
        return Err(input(format!(
            "Exact affine placement unproved: {}",
            placement["reason"]
        )));
    }
    let scale = |upper: &Value| -> Result<Value> {
        if upper.is_null() {
            Ok(Value::Null)
        } else {
            Ok(nurbs(
                "sweep_error_upper_compose",
                json!({"kind":"multiply","a":placement["operatorNormUpper"],"b":upper}),
            )?["errorUpper"]
                .clone())
        }
    };
    let caps = if let Some(caps) = source.certificate["filledCapErrorUpper"].as_array() {
        let values = caps.iter().map(scale).collect::<Result<Vec<_>>>()?;
        if values.iter().any(Value::is_null) {
            Value::Null
        } else {
            json!(values)
        }
    } else {
        Value::Null
    };
    let certificate = nurbs(
        "sweep_boundary_certificate",
        json!({"wall":scale(&source.certificate["wallErrorUpper"])?,"caps":caps,"closed":source.certificate["closed"],"budget":o["maxDeviation"]}),
    )?;
    complete(
        &certificate,
        "Affine complete boundary error exceeds max_deviation or is unproved",
    )?;
    let model = &placement["model"];
    let (smoothness, charts, volume) = audit_placed(model, &certificate)?;
    let body = boundary_fields(
        json!({"model":model,"placement":placement,"profileSmoothness":smoothness,"retainedWallCharts":charts,"volume":volume,"wallRegularityCertified":charts["allChartsCertified"],"profileRegularityCertified":charts["allChartsCertified"]}),
        &certificate,
    );
    own(body, None, None, Value::Null)
}
pub fn smooth(mut v: Value, reconstruct: bool) -> Result<Value> {
    let source = owner(&v)?;
    let o = v["options"].clone();
    if reconstruct {
        v["sections"] = source.sections.clone().ok_or_else(|| {
            input("Station reconstruction requires constructor-owned retained sections")
        })?;
        v["sharp"] = source.sharp.clone().ok_or_else(|| {
            input("Station reconstruction requires constructor-owned retained sections")
        })?;
    }
    complete(
        &source.certificate,
        "Station smoothing source complete boundary bound unproved",
    )?;
    if source.certificate["wallErrorUpper"].is_null() {
        return Err(input(
            "Station smoothing source complete boundary bound unproved",
        ));
    }
    let closed = yes(&source.certificate, "closed");
    let identity = call(
        "brep_nurbs_section_loft_source_audit",
        json!({"model":source.model,"sections":v["sections"],"closed":closed}),
    )?;
    if !yes(&identity, "geometryAndTopologyIdentical") {
        return Err(input(
            "Station smoothing sections do not reproduce the certified source body",
        ));
    }
    let candidate = call(
        "brep_nurbs_smooth_station_walls",
        json!({"sections":v["sections"],"sharp":v["sharp"],"closed":closed,"quantum":o["quantum"],"tolerance":o["wallTolerance"],"maxWork":o["maxWork"]}),
    )?;
    if candidate["sides"].is_null() || candidate["wallDisplacementUpper"].is_null() {
        return Err(input(format!(
            "Station smoothing candidate unproved: {}",
            candidate["reason"]
        )));
    }
    let certificate = nurbs(
        "sweep_boundary_certificate",
        json!({"wall":add(source.certificate["wallErrorUpper"].clone(),candidate["wallDisplacementUpper"].clone())?,"caps":source.certificate["filledCapErrorUpper"],"closed":closed,"budget":o["maxDeviation"]}),
    )?;
    complete(
        &certificate,
        "Smoothed complete boundary error exceeds max_deviation or is unproved",
    )?;
    let model = call(
        "brep_nurbs_section_loft_surfaces",
        json!({"sections":v["sections"],"sides":candidate["sides"],"closed":closed}),
    )?;
    let caps = cap_faces(&model, closed)?;
    for face in caps.as_array().unwrap() {
        let i = face.as_u64().unwrap() as usize;
        if !equivalent(&model["faces"][i], &source.model["faces"][i]) {
            return Err(input("Station smoothing changed a filled cap"));
        }
        let f = &model["faces"][i];
        let wires = std::iter::once(&f["outer"]).chain(f["holes"].as_array().unwrap());
        for wire in wires {
            let wire = wire.as_u64().unwrap() as usize;
            if !equivalent(&model["loops"][wire], &source.model["loops"][wire]) {
                return Err(input("Station smoothing changed cap trims"));
            }
            for coedge in model["loops"][wire]["coedges"].as_array().unwrap() {
                let edge = coedge["edge"].as_u64().unwrap() as usize;
                if !equivalent(&model["edges"][edge], &source.model["edges"][edge]) {
                    return Err(input("Station smoothing changed a cap boundary edge"));
                }
                for vertex in model["edges"][edge]["vertices"].as_array().unwrap() {
                    let vertex = vertex.as_u64().unwrap() as usize;
                    if !equivalent(
                        &model["vertices"][vertex],
                        &source.model["vertices"][vertex],
                    ) {
                        return Err(input("Station smoothing changed a cap boundary vertex"));
                    }
                }
            }
        }
    }
    let (smoothness, charts, volume) = audit_placed(&model, &certificate)?;
    let body = boundary_fields(
        merge(
            json!({"method":"bounded-miter-station-reconstruction","model":model,"candidate":candidate,"sharpStationIndices":v["sharp"],"profileSmoothness":smoothness,"retainedWallCharts":charts,"volume":volume,"wallRegularityCertified":charts["allChartsCertified"],"profileRegularityCertified":charts["allChartsCertified"]}),
            &source.authoring,
        ),
        &certificate,
    );
    own(body, None, None, Value::Null)
}
