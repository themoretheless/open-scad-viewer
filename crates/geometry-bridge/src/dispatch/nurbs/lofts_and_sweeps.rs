//! Section lofts, capped lofts, ruled lofts, and progressive profile bodies.
use crate::*;

pub(super) const OPS: &[&str] = &[
    "brep_nurbs_section_loft_source_audit",
    "brep_nurbs_capped_loft_with_caps",
    "brep_nurbs_natural_section_loft",
    "brep_nurbs_section_loft_surfaces",
    "brep_nurbs_smooth_polygon_station_body",
    "brep_nurbs_smooth_station_walls",
    "brep_nurbs_capped_loft_surfaces",
    "brep_nurbs_periodic_section_loft",
    "brep_nurbs_rational_section_loft",
    "brep_nurbs_progressive_profile_body",
    "brep_nurbs_ruled_loft",
];

pub(super) fn handle(op: &str, v: Value) -> Result<Value> {
    match op {
        "brep_nurbs_section_loft_source_audit" => {
            let exact = brep_core::analytic::section_loft_source_matches(
                &field(&v, "model")?,
                &field::<Vec<Vec<Vec<Curve>>>>(&v, "sections")?,
                field(&v, "closed")?,
            )?;
            Ok(json!({
                "geometryAndTopologyIdentical": exact,
                "globalEmbeddingCertified": false
            }))
        }
        "brep_nurbs_capped_loft_with_caps" => handle_capped_loft_with_caps(&v),
        "brep_nurbs_natural_section_loft" => encode(brep_core::natural_section_loft(
            &field::<Vec<Vec<Vec<Curve>>>>(&v, "sections")?,
            &field::<Vec<f64>>(&v, "parameters")?,
        )?),
        "brep_nurbs_smooth_polygon_station_body"
        | "brep_nurbs_section_loft_surfaces"
        | "brep_nurbs_rational_section_loft" => crate::polygon_station_body::dispatch(op, &v),
        "brep_nurbs_smooth_station_walls" => {
            let report = brep_core::analytic::smooth_station_walls(
                &field::<Vec<Vec<Vec<Curve>>>>(&v, "sections")?,
                &field::<Vec<usize>>(&v, "sharp")?,
                field(&v, "closed")?,
                field(&v, "quantum")?,
                field(&v, "tolerance")?,
                field(&v, "maxWork")?,
            )?;
            encode(json!({
                "sides": report.sides,
                "wallDisplacementUpper": report.wall_displacement_upper,
                "work": report.work,
                "reason": report.reason
            }))
        }
        "brep_nurbs_capped_loft_surfaces" => encode(brep_core::capped_loft_surfaces(
            &field::<Vec<Vec<Curve>>>(&v, "start")?,
            &field::<Vec<Vec<Curve>>>(&v, "end")?,
            &field::<Vec<Vec<Surface>>>(&v, "sides")?,
        )?),
        "brep_nurbs_periodic_section_loft" => encode(brep_core::periodic_section_loft(
            &field::<Vec<Vec<Vec<Curve>>>>(&v, "sections")?,
        )?),
        "brep_nurbs_progressive_profile_body" => handle_progressive_profile_body(&v),
        "brep_nurbs_ruled_loft" => encode(brep_core::ruled_loft(
            &field::<Vec<Vec<[f64; 3]>>>(&v, "sections")?,
        )?),
        _ => unreachable!("lofts_and_sweeps admits only its OPS"),
    }
}

fn handle_capped_loft_with_caps(v: &Value) -> Result<Value> {
    let definitions: Vec<Value> = field(v, "caps")?;
    if definitions.len() != 2 {
        return Err(input("Loft requires two authored caps"));
    }
    let caps = definitions
        .iter()
        .map(|c| {
            Ok(brep_core::LoftCap {
                surface: field(c, "surface")?,
                trims: field(c, "trims")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let l: Value = field(v, "embeddingLimits")?;
    let limits = brep_core::boundary_embedding::Limits {
        exact_work: field(&l, "exactWork")?,
        trim_pairs: field(&l, "trimPairs")?,
        trim_cells: field(&l, "trimCells")?,
        trim_domain_cells: field(&l, "trimDomainCells")?,
        spans: field(&l, "spans")?,
        contacts: brep_core::face_contacts::Limits {
            pairs: field(&l, "facePairs")?,
            cells: field(&l, "faceCells")?,
            domain_cells: field(&l, "faceDomainCells")?,
            cells_per_pair: field(&l, "faceCellsPerPair")?,
            domain_cells_per_pair: field(&l, "faceDomainCellsPerPair")?,
        },
    };
    encode(brep_core::capped_loft_with_caps_checked(
        &field::<Vec<Vec<Curve>>>(v, "start")?,
        &field::<Vec<Vec<Curve>>>(v, "end")?,
        &field::<Vec<Vec<Surface>>>(v, "sides")?,
        [&caps[0], &caps[1]],
        field(v, "toleranceUv")?,
        limits,
    )?)
}

fn handle_progressive_profile_body(v: &Value) -> Result<Value> {
    use nurbs_core::progressive_sweep::{Options, Orientation, Spacing};
    let orientation_raw = field::<String>(v, "orientation")?;
    let orientation = match orientation_raw.as_str() {
        "rmf" => Orientation::RotationMinimizing,
        "fixed" | "authored" => Orientation::Fixed,
        "fixed_normal" => Orientation::FixedNormal,
        "frenet" => Orientation::Frenet,
        "corrected_frenet" => Orientation::CorrectedFrenet,
        _ => {
            return Err(Error::new(
                "BREP_RATIONAL_SWEEP_REFUSED",
                "Unknown orientation",
            ));
        }
    };
    let spacing = match field::<String>(v, "spacing")?.as_str() {
        "parameter" => Spacing::Parameter,
        "arc_length" => Spacing::ArcLength {
            tolerance: field(v, "length_tolerance")?,
            max_cells: field(v, "length_max_cells")?,
        },
        _ => return Err(Error::new("BREP_RATIONAL_SWEEP_REFUSED", "Unknown spacing")),
    };
    let authored = orientation_raw == "authored";
    let options = Options {
        normal: field(v, "normal")?,
        orientation: if authored {
            Orientation::Fixed
        } else {
            orientation
        },
        spacing,
        initial_sections: field(v, "initial_sections")?,
        max_sections: field(v, "max_sections")?,
        max_deviation: field(v, "max_deviation")?,
    };
    let loops = field::<Vec<Vec<Curve>>>(v, "loops")?;
    let path = field::<Curve>(v, "path")?;
    let scale = field::<Curve>(v, "scale")?;
    let twist = field::<Curve>(v, "twist")?;
    let axes = optional_field::<Curve>(v, "axis_scale")?;
    let center = optional_field::<Curve>(v, "center_law")?;
    let guide = optional_field::<Curve>(v, "orientation_guide")?;
    let contact = optional_field::<f64>(v, "contact_parameter")?;
    let contact_profile = optional_field::<usize>(v, "contact_profile")?;
    if (contact.is_some() && guide.is_none()) || (contact_profile.is_some() && contact.is_none()) {
        return Err(Error::new(
            "BREP_RATIONAL_SWEEP_REFUSED",
            "Contact anchor requires orientation guide and parameter",
        ));
    }
    if authored && guide.is_some() {
        return Err(Error::new(
            "BREP_RATIONAL_SWEEP_REFUSED",
            "Orientation guide cannot be combined with authored frames",
        ));
    }
    let frame_axis = if authored {
        Some(field::<Curve>(v, "frame_axis")?)
    } else {
        None
    };
    let frame_normal = if authored {
        Some(field::<Curve>(v, "frame_normal")?)
    } else {
        None
    };
    let default_axes = nurbs_core::progressive_sweep::constant_vector_law([1.; 3])?;
    let default_center = nurbs_core::progressive_sweep::constant_vector_law([0.; 3])?;
    let affine = if authored || guide.is_some() || axes.is_some() || center.is_some() {
        Some((
            axes.as_ref().unwrap_or(&default_axes),
            center.as_ref().unwrap_or(&default_center),
        ))
    } else {
        None
    };
    let correction = parse_cap_correction(v)?;
    let rmf_policy = parse_rmf_policy(v, &orientation_raw)?;
    let result = brep_core::analytic::progressive_profile_body_with_rmf_policy(
        &loops,
        &path,
        &scale,
        &twist,
        affine,
        frame_axis.as_ref().zip(frame_normal.as_ref()),
        guide
            .as_ref()
            .map(|g| (g, contact.map(|p| (contact_profile.unwrap_or(0), p)))),
        options,
        correction,
        rmf_policy,
    )?;
    encode_progressive_profile_body(result, rmf_policy)
}

fn encode_progressive_profile_body(
    result: brep_core::analytic::ProgressiveBodyEvidence,
    rmf_policy: Option<(usize, usize, usize)>,
) -> Result<Value> {
    let retained_caps = result.retained_caps.as_ref().map(|r| {
        json!({
            "exact": r.exact,
            "capErrorUpper": if r.exact { Some(0.) } else { None },
            "exactWork": r.exact_work,
            "inspectedEdges": r.inspected_edges,
            "reason": r.reason,
            "scope": "constructor-owned-retained-endpoint-regions",
            "continuousBound": false,
            "globalEmbeddingCertified": false
        })
    });
    let cap_projection = result.cap_projection.as_ref().map(|r| {
        json!({
            "idealCapDomainsCertified": r.original.domains.ideal_endpoint_domains_certified,
            "normalDots": r.normal_dots,
            "reversesOrientation": r.reverses_orientation,
            "cells": r.cells,
            "exactWork": r.exact_work,
            "reason": r.reason,
            "scope": "constructor-owned-endpoint-plane-projection"
        })
    });
    let mut output = encode(json!({
        "model": result.model,
        "approximation": result.approximation,
        "retainedCaps": retained_caps,
        "capProjection": cap_projection,
        "filledCapErrorUpper": result.filled_cap_error_upper,
        "capCorrectionErrorUpper": result.cap_correction_error_upper,
        "bodyDecompositionErrorUpper": result.body_decomposition_error_upper,
        "bodyDecompositionProducts": result.body_decomposition_products,
        "retainedWalls": {
            "certified": result.retained_walls.certified,
            "faceCoverageCertified": result.retained_walls.face_coverage_certified,
            "coefficientFamilyCertified": result.retained_walls.coefficient_family_certified,
            "inspectedFaces": result.retained_walls.inspected_faces,
            "exactWork": result.retained_walls.exact_work,
            "reason": result.retained_walls.reason
        },
        "boundaryErrorUpper": result.boundary_error_upper,
        "boundaryContinuousBound": result.boundary_error_upper.is_some(),
        "boundaryErrorWithinBudget": result.boundary_error_within_budget,
        "boundaryErrorScope": "constructor-owned-retained-wall-and-cap-union",
        "globalEmbeddingCertified": false
    }))?;
    if let Some((_, cells, products)) = rmf_policy {
        output["approximation"]["report"]["errorCertificateMaxCells"] = json!(cells);
        output["approximation"]["report"]["decompositionMaxProducts"] = json!(products);
        if let Some(levels) = output["approximation"]["levels"].as_array_mut() {
            for level in levels {
                level["errorCertificateMaxCells"] = json!(cells);
                level["decompositionMaxProducts"] = json!(products);
            }
        }
    }
    Ok(output)
}

fn parse_cap_correction(v: &Value) -> Result<Option<brep_core::analytic::EndpointCapCorrection>> {
    let tolerance = optional_field::<f64>(v, "cap_correction_tolerance")?;
    let quantum = optional_field::<f64>(v, "cap_correction_quantum")?;
    let max_work = optional_field::<u64>(v, "cap_correction_max_work")?;
    if tolerance.is_none() && (quantum.is_some() || max_work.is_some()) {
        return Err(Error::new(
            "BREP_RATIONAL_SWEEP_REFUSED",
            "Cap correction requires a displacement tolerance",
        ));
    }
    Ok(tolerance.map(|tolerance| brep_core::analytic::EndpointCapCorrection {
        tolerance,
        quantum: quantum.unwrap_or(2_f64.powi(-40)),
        max_work: max_work.unwrap_or(1_000_000),
    }))
}

fn parse_rmf_policy(v: &Value, orientation: &str) -> Result<Option<(usize, usize, usize)>> {
    let rmf_steps = optional_field::<usize>(v, "rmf_transport_steps")?;
    let rmf_cells = optional_field::<usize>(v, "error_max_cells")?;
    let rmf_products = optional_field::<usize>(v, "error_max_products")?;
    match (rmf_steps, rmf_cells, rmf_products) {
        (None, None, None) => Ok(None),
        (Some(steps), Some(cells), Some(products)) if orientation == "rmf" => {
            Ok(Some((steps, cells, products)))
        }
        _ => Err(Error::new(
            "BREP_RATIONAL_SWEEP_REFUSED",
            "Spatial RMF policy requires unguided RMF and three proof budgets",
        )),
    }
}
