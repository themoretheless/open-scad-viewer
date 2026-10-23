//! Fresh inverse chart construction for a globally bijective quadratic shear.
//! Exact equations and exact candidate coordinates are both required.
//! This certificate alone does not authorize face contacts, shells or Bodies.
use cad_predicates::{
    AuthoredScalar, Limits, ParameterIdentity, PredicateContext, SourceArena, ToleranceContext,
};
use nurbs_core::{Error, Result, surface::Surface};
pub struct Certificate {
    source: Surface,
    inverse: Surface,
    axes: [usize; 2],
    coefficient: f64,
}
impl Certificate {
    pub fn source(&self) -> &Surface {
        &self.source
    }
    pub fn inverse(&self) -> &Surface {
        &self.inverse
    }
    pub fn axes(&self) -> [usize; 2] {
        self.axes
    }
    pub fn coefficient(&self) -> f64 {
        self.coefficient
    }
}
pub struct Report {
    pub certificate: Option<Certificate>,
    pub exact_work: u64,
    pub reason: &'static str,
}
pub fn qualify(
    source: &Surface,
    axes: [usize; 2],
    coefficient: f64,
    max_work: u64,
) -> Result<Report> {
    source.validate()?;
    if axes.iter().any(|&a| a > 2)
        || axes[0] == axes[1]
        || !coefficient.is_finite()
        || !(1..=100_000_000).contains(&max_work)
    {
        return Err(Error::new(
            "BREP_SOURCE_INVERSE_SHEAR",
            "Choose finite shear and bounded exact work",
        ));
    }
    let mut out = Report {
        certificate: None,
        exact_work: 0,
        reason: "source-inverse-shear-layout-unproven",
    };
    if source.degree_u != 2
        || source.degree_v != 2
        || source.periodic_u
        || source.periodic_v
        || source.control_points.len() != 3
        || source.control_points[0].len() != 3
        || source.control_points[0][0].len() != 3
    {
        return Ok(out);
    }
    for knots in [&source.knots_u, &source.knots_v] {
        if knots.len() != 6
            || !knots[..3].iter().all(|&k| k == knots[0])
            || !knots[3..].iter().all(|&k| k == knots[3])
        {
            return Ok(out);
        }
    }
    let weight = source.weights[0][0];
    if weight <= 0. || source.weights.iter().flatten().any(|&w| w != weight) {
        return Ok(out);
    }
    let corners = [(0, 0), (0, 2), (2, 0), (2, 2)];
    let inverse = corners.map(|(i, j)| {
        let mut p = source.control_points[i][j].clone();
        p[axes[1]] -= coefficient * p[axes[0]] * p[axes[0]];
        p
    });
    if inverse.iter().flatten().any(|v| !v.is_finite()) {
        return Ok(out);
    }
    let mut values = source
        .control_points
        .iter()
        .flatten()
        .flatten()
        .copied()
        .collect::<Vec<_>>();
    values.push(coefficient);
    values.extend(inverse.iter().flatten().copied());
    let arena = SourceArena::authored(
        "source-inverse-shear",
        1,
        values
            .iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|_| {
        Error::new(
            "BREP_SOURCE_INVERSE_SHEAR",
            "Invalid original source leaves",
        )
    })?;
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: max_work.min(cad_predicates::MAX_WORK),
            ..Limits::default()
        },
        None,
    );
    let refs = std::array::from_fn(|i| {
        std::array::from_fn(|j| std::array::from_fn(|k| arena.leaf(9 * i + 3 * j + k).unwrap()))
    });
    let identity = cad_predicates::quadratic_shear_chart_identity(
        &mut ctx,
        refs,
        arena.leaf(27).unwrap(),
        axes[0],
        axes[1],
    )
    .map_err(|_| {
        Error::new(
            "BREP_SOURCE_INVERSE_SHEAR",
            "Invalid chart identity request",
        )
    })?;
    out.exact_work = ctx.work_used();
    out.reason = "source-inverse-shear-equations-unproven";
    if identity.outcome != ParameterIdentity::Equal {
        return Ok(out);
    }
    let original =
        corners.map(|(i, j)| std::array::from_fn(|k| arena.leaf(9 * i + 3 * j + k).unwrap()));
    let proposed =
        std::array::from_fn(|i| std::array::from_fn(|k| arena.leaf(28 + 3 * i + k).unwrap()));
    let coordinates = cad_predicates::quadratic_shear_inverse_corners(
        &mut ctx,
        original,
        proposed,
        arena.leaf(27).unwrap(),
        axes[0],
        axes[1],
    )
    .map_err(|_| {
        Error::new(
            "BREP_SOURCE_INVERSE_SHEAR",
            "Invalid inverse coordinate request",
        )
    })?;
    out.exact_work = ctx.work_used();
    out.reason = "source-inverse-shear-coordinates-unproven";
    if coordinates.outcome != ParameterIdentity::Equal {
        return Ok(out);
    }
    let inverse = Surface {
        degree_u: 1,
        degree_v: 1,
        periodic_u: false,
        periodic_v: false,
        knots_u: vec![
            source.knots_u[0],
            source.knots_u[0],
            source.knots_u[3],
            source.knots_u[3],
        ],
        knots_v: vec![
            source.knots_v[0],
            source.knots_v[0],
            source.knots_v[3],
            source.knots_v[3],
        ],
        control_points: vec![
            vec![inverse[0].clone(), inverse[1].clone()],
            vec![inverse[2].clone(), inverse[3].clone()],
        ],
        weights: vec![vec![1.; 2]; 2],
    };
    inverse.validate()?;
    out.certificate = Some(Certificate {
        source: source.clone(),
        inverse,
        axes,
        coefficient,
    });
    out.reason = "source-inverse-shear-chart-qualified";
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inverse_chart_requires_exact_original_equations_and_coordinates() {
        let source = Surface {
            degree_u: 2,
            degree_v: 2,
            periodic_u: false,
            periodic_v: false,
            knots_u: vec![2., 2., 2., 4., 4., 4.],
            knots_v: vec![-1., -1., -1., 1., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| vec![i as f64 / 2., j as f64 / 2., if i == 2 { 0.25 } else { 0. }])
                        .collect()
                })
                .collect(),
            weights: vec![vec![2.; 3]; 3],
        };
        let report = qualify(&source, [0, 2], 0.25, 100000).unwrap();
        let certificate = report.certificate.expect(report.reason);
        assert_eq!(certificate.source(), &source);
        assert_eq!(certificate.axes(), [0, 2]);
        assert_eq!(certificate.coefficient(), 0.25);
        assert_eq!(
            certificate.inverse().control_points,
            vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]]
            ]
        );
        assert_eq!(certificate.inverse().knots_u, vec![2., 2., 4., 4.]);
        let mut damaged = source.clone();
        damaged.control_points[1][1][2] += 1e-12;
        assert!(
            qualify(&damaged, [0, 2], 0.25, 100000)
                .unwrap()
                .certificate
                .is_none()
        );
        let mut weighted = source.clone();
        weighted.weights[1][1] = 3.;
        assert!(
            qualify(&weighted, [0, 2], 0.25, 100000)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(
            qualify(&source, [0, 2], 0.25, 1)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(
            qualify(&source, [0, 2], 0.5, 100000)
                .unwrap()
                .certificate
                .is_none()
        );
    }
}
/// Rebind unchanged UV definitions to the exact accepted inverse chart.
/// Each root selector is replayed; cached world boxes are never transported.
pub struct FragmentReport {
    pub fragment: Option<crate::source_boundary_fragment::Fragment>,
    pub mapping_cells: usize,
    pub root_checks: usize,
    pub reason: &'static str,
}
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
