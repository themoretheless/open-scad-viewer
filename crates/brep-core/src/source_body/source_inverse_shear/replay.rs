use super::*;

/// Original region recipes are replayed after exact inverse boundary composition.
/// Supported recipe steps: original, splitRoot and splitParameter.
pub struct RegionReport {
    pub region: Option<crate::source_contour_proposal::SourceRegion>,
    pub exact_work: u64,
    pub root_checks: usize,
    pub mapping_cells: usize,
    pub reason: &'static str,
}
pub fn transport_region(
    certificate: &Certificate,
    original: &crate::source_contour_proposal::SourceRegion,
    limits: &crate::source_region_restore::Limits,
    max_exact_work: u64,
) -> Result<RegionReport> {
    if !(1..=64).contains(&limits.steps)
        || !(1..=100_000_000).contains(&max_exact_work)
        || !(1..=100000).contains(&limits.mapping_cells)
    {
        return Err(Error::new(
            "BREP_SOURCE_INVERSE_SHEAR",
            "Bound inverse region replay work",
        ));
    }
    let mut out = RegionReport {
        region: None,
        exact_work: 0,
        root_checks: 0,
        mapping_cells: 0,
        reason: "source-inverse-shear-region-recipe-unproven",
    };
    let Some(value) = rewrite_region(
        certificate,
        original.definition(),
        limits.steps,
        limits.mapping_cells,
        max_exact_work,
        &mut out,
    )?
    else {
        return Ok(out);
    };
    out.region = Some(crate::source_region_restore::restore(value, limits)?);
    out.reason = "source-inverse-shear-region-qualified";
    Ok(out)
}
fn rewrite_region(
    certificate: &Certificate,
    mut value: value_codec::Value,
    remaining: usize,
    max_mapping: usize,
    max_work: u64,
    out: &mut RegionReport,
) -> Result<Option<value_codec::Value>> {
    use value_codec::{Deserialize, Serialize};
    if remaining == 0 {
        return Ok(None);
    }
    match value["kind"].as_str() {
        Some("splitRoot") | Some("splitParameter") => {
            let Some(parent) = rewrite_region(
                certificate,
                value["parent"].clone(),
                remaining - 1,
                max_mapping,
                max_work,
                out,
            )?
            else {
                return Ok(None);
            };
            value["parent"] = parent;
            if value["kind"].as_str() == Some("splitRoot") {
                let source =
                    crate::source_contact_point::restore(value["point"].clone(), max_mapping)?;
                let Some(point) = source.point else {
                    return Ok(None);
                };
                if point.surface() != certificate.source() || out.mapping_cells == max_mapping {
                    return Ok(None);
                }
                let inverse = crate::source_contact_point::qualify(
                    certificate.inverse(),
                    point.boundary(),
                    point.contact(),
                    point.selector(),
                    max_mapping - out.mapping_cells,
                )?;
                out.mapping_cells += inverse.mapping_cells;
                out.root_checks += 1;
                let Some(point) = inverse.point else {
                    return Ok(None);
                };
                value["point"] = point.definition();
            }
        }
        Some("original") => {
            let source = Surface::from_value(value["surface"].clone()).map_err(|_| {
                Error::new(
                    "BREP_SOURCE_INVERSE_SHEAR",
                    "Invalid original region surface",
                )
            })?;
            if &source != certificate.source() {
                return Ok(None);
            }
            let wires = value["wires"]
                .as_array_mut()
                .ok_or_else(|| Error::new("BREP_SOURCE_INVERSE_SHEAR", "Missing original wires"))?;
            for wire in wires {
                for boundary in wire.as_array_mut().ok_or_else(|| {
                    Error::new("BREP_SOURCE_INVERSE_SHEAR", "Invalid original wire")
                })? {
                    let pcurve = nurbs_core::curve::Curve::from_value(boundary["pcurve"].clone())
                        .map_err(|_| {
                        Error::new("BREP_SOURCE_INVERSE_SHEAR", "Invalid original pcurve")
                    })?;
                    let mut world = pcurve.clone();
                    let mut points = Vec::new();
                    for p in &pcurve.control_points {
                        points.push(certificate.inverse().evaluate(p[0], p[1])?.point.to_vec());
                    }
                    world.control_points = points;
                    if out.exact_work == max_work {
                        return Ok(None);
                    }
                    let Some(proof) = nurbs_core::curve_surface_agreement::verify_exact_algebraic(
                        &world,
                        &pcurve,
                        certificate.inverse(),
                        false,
                        (max_work - out.exact_work).min(cad_predicates::MAX_WORK),
                    )?
                    else {
                        return Ok(None);
                    };
                    out.exact_work += proof.work_used;
                    if proof.outcome != cad_predicates::BezierIdentity::Equal {
                        return Ok(None);
                    }
                    boundary["curve"] = world.to_value();
                }
            }
            value["surface"] = certificate.inverse().to_value();
        }
        _ => return Ok(None),
    }
    Ok(Some(value))
}
