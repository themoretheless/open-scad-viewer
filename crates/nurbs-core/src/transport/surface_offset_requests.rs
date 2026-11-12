//! Curve and surface offset, envelope, contact-band, and qualification transport requests.
use super::*;

pub(super) fn try_dispatch(v: &Value, op: &str) -> Option<Result<Value>> {
    if let Some(res) = try_dispatch_curve_and_surface_bounds(v, op) {
        return Some(res);
    }
    try_dispatch_envelopes_and_contact_bands(v, op)
}

fn try_dispatch_curve_and_surface_bounds(v: &Value, op: &str) -> Option<Result<Value>> {
    let res = match op {
        "curve_chain_diagnostics" => (|| {
            Ok(crate::curve_offset_diagnostics::inspect_curves(
                &field::<Vec<curve::Curve>>(v, "curves")?,
                optional_field::<usize>(v, "maxPairs")?.unwrap_or(1_000_000),
            )?
            .to_value())
        })(),
        "curve_offset_trimmed_bevel" => handle_curve_offset_trimmed_bevel(v),
        "curve_offset_bevel_wire" => handle_curve_offset_bevel_wire(v),
        "curve_offset_bounded" => handle_curve_offset_bounded(v),
        "surface_offset_evaluate" => (|| {
            let e = surface_offset::evaluate(
                &field(v, "surface")?,
                field(v, "parameters")?,
                field(v, "distance")?,
            )?;
            Ok(json!({
                "method": "normal-offset-numerical-jets",
                "point": e.point,
                "du": e.du,
                "dv": e.dv,
                "sourceUnitNormal": e.source_unit_normal,
                "certified": false,
                "topologyAuthority": false
            }))
        })(),
        "surface_offset_bounds" => (|| {
            let r = surface_offset::bounds(
                &field(v, "surface")?,
                field(v, "domain")?,
                field(v, "distance")?,
                field(v, "maxSpans")?,
            )?;
            Ok(json!({
                "method": "interval-source-normal-offset",
                "scope": "incident-span-offset-images",
                "image": r.image,
                "unitNormals": r.unit_normals,
                "normalSpanVisits": r.spans,
                "reason": r.reason,
                "offsetRegularityCertified": false,
                "continuityCertified": false,
                "topologyAuthority": false
            }))
        })(),
        "surface_offset_jacobian_bounds" => (|| {
            let r = surface_offset::jacobian_bounds(
                &field(v, "surface")?,
                field(v, "domain")?,
                field(v, "distance")?,
                field(v, "maxSpans")?,
            )?;
            Ok(json!({
                "method": "interval-source-normal-offset-jets",
                "scope": "incident-span-offset-jets",
                "image": r.image,
                "derivatives": r.derivatives,
                "normalSpanVisits": r.spans,
                "reason": r.reason,
                "offsetRegularityCertified": false,
                "continuityCertified": false,
                "topologyAuthority": false
            }))
        })(),
        "surface_offset_candidates" => (|| {
            let a: Surface = field(v, "a")?;
            let b: Surface = field(v, "b")?;
            let r = surface_offset::intersection_candidates(
                [&a, &b],
                field(v, "domains")?,
                field(v, "distances")?,
                field(v, "parameterTolerance")?,
                field(v, "maxBoxes")?,
                field(v, "maxSpans")?,
            )?;
            Ok(json!({
                "method": "interval-offset-pair-exclusion",
                "scope": "untrimmed-offset-carriers",
                "candidateBoxes": r.boxes,
                "pendingBoxes": r.pending,
                "visitedBoxes": r.visited_boxes,
                "excludedBoxes": r.excluded_boxes,
                "normalSpanVisits": r.normal_span_visits,
                "reason": r.reason,
                "rootExistenceProven": false,
                "wholeCurveComplete": false,
                "trimMembershipProven": false,
                "topologyAuthority": false
            }))
        })(),
        "surface_offset_contact_qualification" => handle_contact_qualification(v),
        _ => return None,
    };
    Some(res)
}

fn try_dispatch_envelopes_and_contact_bands(v: &Value, op: &str) -> Option<Result<Value>> {
    let res = match op {
        "surface_offset_envelope_fit" => (|| {
            let a: Surface = field(v, "a")?;
            let b: Surface = field(v, "b")?;
            let candidate: Option<Surface> = v
                .get("candidate")
                .filter(|x| !x.is_null())
                .map(|_| field(v, "candidate"))
                .transpose()?;
            crate::offset_envelope_fit::deliver(
                candidate.as_ref(),
                crate::offset_envelope_fit::OffsetFaces { surfaces: [&a, &b], distances: field(v, "distances")? },
                crate::offset_envelope_fit::ContactParameters { axis: field(v, "fixedAxis")?, drive: field(v, "fixedInterval")?, free: field(v, "firstOther")?, second: field(v, "secondDomain")?, spans: field(v, "maxSpans")? },
                field(v, "toleranceMm")?,
                field(v, "maxCells")?,
            )
        })(),
        "surface_offset_envelope" => (|| {
            let a: Surface = field(v, "a")?;
            let b: Surface = field(v, "b")?;
            Ok(crate::offset_envelope::certify(
                crate::offset::contact_pcurve::OffsetFaces {
                    surfaces: [&a, &b],
                    distances: field(v, "distances")?,
                },
                field(v, "fixedAxis")?,
                field(v, "fixedInterval")?,
                field(v, "firstOther")?,
                field(v, "secondDomain")?,
                field(v, "maxSpans")?,
                field(v, "maxCells")?,
            )?
            .to_value())
        })(),
        "surface_offset_contact_tangent" => (|| {
            let a: Surface = field(v, "a")?;
            let b: Surface = field(v, "b")?;
            Ok(crate::offset_contact_tangent::certify(
                [&a, &b],
                field(v, "distances")?,
                field(v, "fixedAxis")?,
                field(v, "fixedInterval")?,
                field(v, "firstOther")?,
                field(v, "secondDomain")?,
                field(v, "maxSpans")?,
            )?
            .to_value())
        })(),
        "surface_offset_source_boundary" => handle_offset_source_boundary(v),
        "surface_offset_trimmed_contact_band" => (|| {
            let a: Surface = field(v, "a")?;
            let b: Surface = field(v, "b")?;
            let first_loops: Vec<Vec<crate::curve::Curve>> = field(v, "firstLoops")?;
            let second_loops: Vec<Vec<crate::curve::Curve>> = field(v, "secondLoops")?;
            Ok(crate::trimmed_offset_contact::certify(crate::trimmed_offset_contact::TrimmedFaces { surfaces: [&a, &b], loops: [&first_loops, &second_loops], distances: field(v, "distances")? }, crate::trimmed_offset_contact::ContactParameters { axis: field(v, "fixedAxis")?, drive: field(v, "fixedInterval")?, free: field(v, "firstOther")?, second: field(v, "secondDomain")?, spans: field(v, "maxSpans")? }, field(v, "toleranceUv")?, crate::trimmed_offset_contact::Limits {
                    max_pairs: field(v, "maxPairs")?,
                    max_cells: field(v, "maxCells")?,
                    max_domain_cells: field(v, "maxDomainCells")?,
                })?
            .to_value())
        })(),
        "surface_offset_contact_band" => handle_offset_contact_band(v),
        "surface_offset_contact_section" => handle_offset_contact_section(v),
        _ => return None,
    };
    Some(res)
}

fn handle_curve_offset_trimmed_bevel(v: &Value) -> Result<Value> {
    let wire = curve_offset_wire::bevel_wire(
        &field(v, "curve")?,
        field(v, "distance")?,
        field(v, "toleranceMm")?,
        field(v, "maxCells")?,
    )?;
    crate::check(wire.closed, "Trimmed offset requires a closed source.")?;
    let rule = match field::<String>(v, "fillRule")?.as_str() {
        "nonzero" => crate::chord_winding::FillRule::NonZero,
        "evenodd" => crate::chord_winding::FillRule::EvenOdd,
        _ => {
            crate::check(false, "Choose nonzero or evenodd fill.")?;
            unreachable!()
        }
    };
    let graph = crate::chord_arrangement::split(
        &wire.edges,
        true,
        field(v, "intersectionToleranceMm")?,
        optional_field::<usize>(v, "maxPairs")?.unwrap_or(1_000_000),
    )?;
    let loops = crate::chord_fill_selection::boundary_curves(
        &graph,
        rule,
        optional_field::<usize>(v, "maxWitnessChecks")?.unwrap_or(1_000_000),
    )?;
    let intersection_error = graph
        .vertices
        .iter()
        .map(|p| p.error_upper_mm)
        .fold(0., f64::max);
    Ok(json!({
        "loops": encode(loops)?,
        "report": {
            "accepted": true,
            "method": "represented-bevel-offset-fill/1",
            "fillRule": v["fillRule"].clone(),
            "regionTrimmed": true,
            "topologyScope": "represented-reconstructed-chord-graph",
            "regionTopologyCertified": false,
            "originalOffsetTopologyCertified": false,
            "sourceWireErrorUpperMm": wire.error_upper_mm,
            "intersectionConstructionErrorUpperMm": intersection_error
        }
    }))
}

fn handle_curve_offset_bevel_wire(v: &Value) -> Result<Value> {
    let result = curve_offset_wire::bevel_wire(
        &field(v, "curve")?,
        field(v, "distance")?,
        field(v, "toleranceMm")?,
        field(v, "maxCells")?,
    )?;
    let diagnostics = crate::curve_offset_diagnostics::inspect_chain(
        &result.edges,
        result.closed,
        optional_field::<usize>(v, "maxPairs")?.unwrap_or(1_000_000),
    )?
    .to_value();
    let cells: Vec<_> = result
        .edges
        .iter()
        .zip(&result.roles)
        .map(|(edge, role)| {
            let source = match role {
                curve_offset_wire::Role::Source(domain) => {
                    json!({"kind": "source-offset", "domain": domain})
                }
                curve_offset_wire::Role::Bevel(knot) => {
                    json!({"kind": "bevel", "sourceKnot": knot})
                }
            };
            json!({
                "domain": edge.domain,
                "stationDomain": edge.domain,
                "source": source,
                "errorUpperMm": edge.error_upper_mm
            })
        })
        .collect();
    Ok(json!({
        "curves": encode(result.curves)?,
        "report": {
            "accepted": true,
            "closed": result.closed,
            "wholeWire": true,
            "wholeCurve": false,
            "method": "outward-source-offset-bevel-wire/1",
            "errorUpperMm": result.error_upper_mm,
            "toleranceMm": v["toleranceMm"].clone(),
            "cells": cells,
            "chainDiagnostics": diagnostics,
            "offsetRegularityCertified": false,
            "regionTopologyCertified": false,
            "regionTrimmed": false
        }
    }))
}

fn handle_curve_offset_bounded(v: &Value) -> Result<Value> {
    let result = curve_offset::approximate_curve(
        &field(v, "curve")?,
        field(v, "distance")?,
        field(v, "toleranceMm")?,
        field(v, "maxCells")?,
    )?;
    let cells: Vec<_> = result
        .segments
        .iter()
        .map(|s| json!({"domain": s.domain, "errorUpperMm": s.error_upper_mm}))
        .collect();
    let diagnostics = if result.segments.is_empty() {
        Value::Null
    } else {
        crate::curve_offset_diagnostics::inspect_chain(
            &result.segments,
            result.closed,
            optional_field::<usize>(v, "maxPairs")?.unwrap_or(1_000_000),
        )?
        .to_value()
    };
    Ok(json!({
        "curves": encode(result.curves)?,
        "report": {
            "closed": result.closed,
            "accepted": true,
            "errorUpperMm": result.error_upper_mm,
            "toleranceMm": v["toleranceMm"].clone(),
            "wholeCurve": true,
            "method": "outward-rational-jets-chord-bound/1",
            "cells": cells,
            "chainDiagnostics": diagnostics,
            "offsetRegularityCertified": false,
            "regionTopologyCertified": false
        }
    }))
}

fn handle_contact_qualification(v: &Value) -> Result<Value> {
    let a: Surface = field(v, "a")?;
    let b: Surface = field(v, "b")?;
    let candidate: Surface = field(v, "candidate")?;
    let pcurves: [crate::curve::Curve; 2] = field(v, "sourcePcurves")?;
    let first_loops: Vec<Vec<crate::curve::Curve>> = field(v, "firstLoops")?;
    let second_loops: Vec<Vec<crate::curve::Curve>> = field(v, "secondLoops")?;
    let report = crate::offset_contact_trims::certify_with_tangency(
        crate::offset_contact_trims::ContactPair { candidate: &candidate, pcurves: [&pcurves[0], &pcurves[1]], surfaces: [&a, &b], loops: [&first_loops, &second_loops], distances: field(v, "distances")? },
        crate::offset_contact_trims::ContactParameters {
            axis: field(v, "fixedAxis")?,
            drive: field(v, "fixedInterval")?,
            free: field(v, "firstOther")?,
            second: field(v, "secondDomain")?,
            spans: field(v, "maxSpans")?,
        },
        crate::offset_contact_trims::ContactTolerance { mm: field(v, "toleranceMm")?, uv: field(v, "toleranceUv")? },
        crate::offset_contact_trims::Limits {
            fit_cells: field(v, "maxFitCells")?,
            uv_cells: field(v, "maxUvCells")?,
            agreement_cells: field(v, "maxAgreementCells")?,
            root_refinements: field(v, "rootRefinements")?,
            trim: crate::trimmed_offset_contact::Limits {
                max_pairs: field(v, "maxPairs")?,
                max_cells: field(v, "maxTrimCells")?,
                max_domain_cells: field(v, "maxDomainCells")?,
            },
        },
        field(v, "maxSineSquared")?,
        crate::offset_contact_trims::TangentLimits {
            position_cells: field(v, "maxTangentPositionCells")?,
            normal_cells: field(v, "maxNormalCells")?,
            normal_spans: field(v, "maxNormalSpans")?,
        },
    )?;
    Ok(json!({
        "method": "offset-contact-qualification-delivery",
        "request": v.clone(),
        "candidateSurface": candidate,
        "qualification": report.to_value()
    }))
}

fn handle_offset_source_boundary(v: &Value) -> Result<Value> {
    let a: Surface = field(v, "a")?;
    let b: Surface = field(v, "b")?;
    let first_loops: Vec<Vec<crate::curve::Curve>> = field(v, "firstLoops")?;
    let second_loops: Vec<Vec<crate::curve::Curve>> = field(v, "secondLoops")?;
    let first_world: Vec<Vec<crate::offset_source_boundary::Coedge>> = field(v, "firstCoedges")?;
    let second_world: Vec<Vec<crate::offset_source_boundary::Coedge>> = field(v, "secondCoedges")?;
    Ok(crate::offset_source_boundary::certify(
        crate::offset_source_boundary::SourceFaces { surfaces: [&a, &b], loops: [&first_loops, &second_loops], coedges: [&first_world, &second_world], distances: field(v, "distances")? },
        crate::offset_source_boundary::ContactParameters { axis: field(v, "fixedAxis")?, drive: field(v, "fixedInterval")?, free: field(v, "firstOther")?, second: field(v, "secondDomain")?, spans: field(v, "maxSpans")? },
        crate::offset_source_boundary::ContactTolerance { mm: field(v, "toleranceMm")?, uv: field(v, "toleranceUv")? },
        crate::trimmed_offset_contact::Limits {
            max_pairs: field(v, "maxPairs")?,
            max_cells: field(v, "maxCells")?,
            max_domain_cells: field(v, "maxDomainCells")?,
        },
        field(v, "maxExactWork")?,
        field(v, "maxAgreementCells")?,
    )?
    .to_value())
}

fn handle_offset_contact_band(v: &Value) -> Result<Value> {
    let a: Surface = field(v, "a")?;
    let b: Surface = field(v, "b")?;
    let r = surface_offset::certify_contact_band(
        [&a, &b],
        field(v, "distances")?,
        field(v, "fixedAxis")?,
        field(v, "fixedInterval")?,
        field(v, "firstOther")?,
        field(v, "secondDomain")?,
        field(v, "maxSpans")?,
    )?;
    let (status, witness) = match r {
        surface_offset::ContactBand::Excluded => ("excluded", Value::Null),
        surface_offset::ContactBand::Unresolved => ("unresolved", Value::Null),
        surface_offset::ContactBand::ContinuousBranch(w) => (
            "continuous-branch",
            json!({
                "firstUV": w.first_uv,
                "secondUV": w.second_uv,
                "centerIntervalMm": w.point,
                "contractionUpper": w.contraction_upper
            }),
        ),
    };
    Ok(json!({
        "method": "interval-offset-band-krawczyk",
        "scope": "parameter-band-within-tube",
        "status": status,
        "witness": witness,
        "rootForEveryParameterProven": status == "continuous-branch",
        "uniqueWithinTube": status == "continuous-branch",
        "continuousBranchProven": status == "continuous-branch",
        "wholeCurveComplete": false,
        "trimMembershipProven": false,
        "topologyAuthority": false
    }))
}

fn handle_offset_contact_section(v: &Value) -> Result<Value> {
    let a: Surface = field(v, "a")?;
    let b: Surface = field(v, "b")?;
    let r = surface_offset::certify_contact_section(
        [&a, &b],
        field(v, "distances")?,
        field(v, "fixedAxis")?,
        field(v, "fixed")?,
        field(v, "firstOther")?,
        field(v, "secondDomain")?,
        field(v, "maxSpans")?,
    )?;
    let (status, witness) = match r {
        surface_contact::Verdict::Excluded => ("excluded", Value::Null),
        surface_contact::Verdict::Unresolved => ("unresolved", Value::Null),
        surface_contact::Verdict::Witness(w) => (
            "unique-contact",
            json!({
                "firstUV": w.first_uv,
                "secondUV": w.second_uv,
                "centerIntervalMm": w.point,
                "contractionUpper": w.contraction_upper
            }),
        ),
    };
    Ok(json!({
        "method": "interval-offset-section-krawczyk",
        "scope": "fixed-parameter-offset-section",
        "status": status,
        "witness": witness,
        "rootExistenceProven": status == "unique-contact",
        "uniqueInSection": status == "unique-contact",
        "wholeCurveComplete": false,
        "trimMembershipProven": false,
        "topologyAuthority": false
    }))
}
