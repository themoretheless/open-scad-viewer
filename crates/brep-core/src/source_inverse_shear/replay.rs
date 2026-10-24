use super::*;

pub fn transport_fragment(
    certificate: &Certificate,
    original: &crate::source_boundary_fragment::Fragment,
    max_mapping_cells: usize,
) -> Result<FragmentReport> {
    use crate::source_boundary_fragment::{Endpoint, Fragment};
    if original.surface() != certificate.source() || !(1..=100000).contains(&max_mapping_cells) {
        return Err(Error::new(
            "BREP_SOURCE_INVERSE_SHEAR",
            "Choose matching original chart and bounded root mapping",
        ));
    }
    let mut out = FragmentReport {
        fragment: None,
        mapping_cells: 0,
        root_checks: 0,
        reason: "source-inverse-shear-root-unproven",
    };
    let mut endpoints = Vec::new();
    for endpoint in original.endpoints() {
        let endpoint = match endpoint {
            Endpoint::Parameter(t) => Endpoint::Parameter(*t),
            Endpoint::Crossing { point, role } => {
                if out.mapping_cells == max_mapping_cells {
                    return Ok(out);
                }
                let report = crate::source_contact_point::qualify(
                    certificate.inverse(),
                    point.boundary(),
                    point.contact(),
                    point.selector(),
                    max_mapping_cells - out.mapping_cells,
                )?;
                out.mapping_cells += report.mapping_cells;
                out.root_checks += 1;
                let Some(replayed) = report.point else {
                    return Ok(out);
                };
                Endpoint::Crossing {
                    point: replayed,
                    role: *role,
                }
            }
        };
        endpoints.push(endpoint);
    }
    let mut endpoints = endpoints.into_iter();
    out.fragment = Some(Fragment::new(
        certificate.inverse(),
        original.curve(),
        endpoints.next().unwrap(),
        endpoints.next().unwrap(),
    )?);
    out.reason = "source-inverse-shear-fragment-qualified";
    Ok(out)
}
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
pub struct ShellReport {
    pub shell: Option<crate::source_shell_incidence::Shell>,
    pub exact_work: u64,
    pub root_checks: usize,
    pub mapping_cells: usize,
    pub reason: &'static str,
}
/// Recompute every inverse chart/region and every exact shared edge.
/// This is inverse shell construction; original embedded Body admission is separate.
pub fn transport_shell(
    original: &crate::source_shell_incidence::Shell,
    axes: [usize; 2],
    coefficient: f64,
    limits: &crate::source_region_restore::Limits,
    max_work: u64,
) -> Result<ShellReport> {
    use crate::source_shell_incidence::Pair;
    let mut out = ShellReport {
        shell: None,
        exact_work: 0,
        root_checks: 0,
        mapping_cells: 0,
        reason: "source-inverse-shear-shell-unproven",
    };
    if !(1..=100_000_000).contains(&max_work) || original.regions().is_none() {
        return Err(Error::new(
            "BREP_SOURCE_INVERSE_SHEAR",
            "Choose original regions and bounded inverse shell work",
        ));
    }
    if !original.poles().is_empty() {
        return Ok(out);
    }
    let mut regions = Vec::new();
    for region in original.regions().unwrap() {
        if out.exact_work == max_work {
            return Ok(out);
        }
        let chart = qualify(
            region.loops()[0][0].surface(),
            axes,
            coefficient,
            max_work - out.exact_work,
        )?;
        out.exact_work += chart.exact_work;
        let Some(chart) = chart.certificate else {
            out.reason = chart.reason;
            return Ok(out);
        };
        if out.exact_work == max_work {
            return Ok(out);
        }
        let region = transport_region(&chart, region, limits, max_work - out.exact_work)?;
        out.exact_work += region.exact_work;
        out.root_checks += region.root_checks;
        out.mapping_cells += region.mapping_cells;
        let Some(region) = region.region else {
            out.reason = region.reason;
            return Ok(out);
        };
        regions.push(region);
    }
    let mut pairs = Vec::new();
    for (index, addresses) in original.uses().iter().enumerate() {
        let address = addresses[0];
        let fragment = &regions[address.face].loops()[address.wire][address.edge];
        let mut world = fragment.curve().clone();
        let surface = fragment.surface();
        let mut points = Vec::new();
        for p in &world.control_points {
            points.push(surface.evaluate(p[0], p[1])?.point.to_vec());
        }
        world.control_points = points;
        let reversed = original.edges()[index].reversed();
        // Edge::reversed describes the directed fragment; canonical composition
        // instead uses the underlying forward UV traversal.
        let world_reversed = std::array::from_fn(|side| {
            let address = addresses[side];
            let original_fragment =
                &original.faces()[address.face][address.wire].edges()[address.edge];
            reversed[side] ^ original_fragment.reversed()
        });
        if world_reversed[0] {
            let d = world.domain();
            world.control_points.reverse();
            world.weights.reverse();
            world.knots = world.knots.iter().rev().map(|k| d[0] + d[1] - k).collect();
        }
        pairs.push(Pair {
            uses: *addresses,
            world,
            world_reversed,
            cutters: [None, None],
        });
    }
    if out.exact_work == max_work {
        return Ok(out);
    }
    let shell = crate::source_shell_incidence::assemble_regions(
        &regions,
        &pairs,
        max_work - out.exact_work,
    )?;
    out.exact_work += shell.work_used;
    out.reason = shell.reason;
    out.shell = shell.shell;
    Ok(out)
}
