//! Complete actual ruled-wall decomposition correspondence with native budgets.
//! Filled caps, original transport error and global embedding are separate.
use crate::{Model, Result};
use nurbs_core::{
    curve::Curve,
    curve_decomposition_certificate,
    retained_wall_domain_certificate::{self, Coedge},
};
pub struct Report {
    pub certified: bool,
    pub wall_error_upper: Option<f64>,
    pub inspected_faces: usize,
    pub products: usize,
    pub reason: Option<&'static str>,
}
pub fn inspect(
    model: &Model,
    sections: &[Vec<Vec<Curve>>],
    closed: bool,
    max_products: usize,
    max_faces: usize,
    max_exact_work: u64,
) -> Result<Report> {
    let mut out = Report {
        certified: false,
        wall_error_upper: None,
        inspected_faces: 0,
        products: 0,
        reason: Some("work-limit"),
    };
    if max_exact_work == 0
        || max_exact_work > 1000000
        || max_faces == 0
        || max_faces > 1024
        || max_products > 1000000
        || !(2..=1025).contains(&sections.len())
        || model.faces.len() > max_faces + if closed { 0 } else { 2 }
    {
        return Ok(out);
    }
    out.reason = Some("retained-body-face-coverage-unproved");
    if !crate::sweep_retained_walls::covers(model) {
        return Ok(out);
    }
    out.reason = Some("profile-partition-mismatch");
    let first = &sections[0];
    if first.is_empty()
        || first.len() > 1024
        || first.iter().map(Vec::len).sum::<usize>() == 0
        || sections.iter().any(|r| {
            r.len() != first.len()
                || r.iter()
                    .zip(first)
                    .any(|(a, b)| a.len() != b.len() || a.len() > 1024)
        })
    {
        return Ok(out);
    }
    let mut pairs = Vec::new();
    let mut exact_work = 0;
    for stations in sections.windows(2) {
        for (a, b) in stations[0]
            .iter()
            .flatten()
            .zip(stations[1].iter().flatten())
        {
            out.reason = Some("source-profile-correspondence-unproved");
            if a.validate().is_err()
                || b.validate().is_err()
                || a.periodic != b.periodic
                || a.degree != b.degree
                || a.control_points.len() != b.control_points.len()
                || a.knots != b.knots
                || a.weights != b.weights
            {
                return Ok(out);
            }
            for span in a.degree..a.control_points.len() {
                if a.knots[span] >= a.knots[span + 1] {
                    continue;
                }
                out.reason = Some("work-limit");
                if out.inspected_faces == max_faces {
                    return Ok(out);
                }
                out.reason = Some("retained-wall-domain-unproved");
                let Some(face) = model.faces.get(out.inspected_faces) else {
                    return Ok(out);
                };
                let Some(wire) = model.loops.get(face.outer) else {
                    return Ok(out);
                };
                if wire.coedges.len() != 4 || exact_work == max_exact_work {
                    return Ok(out);
                }
                let mut coedges = Vec::new();
                for usage in &wire.coedges {
                    let Some(edge) = model.edges.get(usage.edge) else {
                        return Ok(out);
                    };
                    coedges.push(Coedge {
                        world: &edge.curve,
                        uv: &usage.pcurve,
                        reversed: usage.reversed,
                    });
                }
                let Ok(domain) = retained_wall_domain_certificate::inspect(
                    &face.surface,
                    &coedges,
                    !face.holes.is_empty(),
                    max_exact_work - exact_work,
                ) else {
                    return Ok(out);
                };
                if domain.work > max_exact_work - exact_work {
                    return Ok(out);
                }
                exact_work += domain.work;
                if !domain.domain_certified {
                    return Ok(out);
                }
                out.inspected_faces += 1;
                out.reason = Some("retained-ruled-face-unproved");
                let s = &face.surface;
                let p = a.degree;
                if s.validate().is_err()
                    || s.periodic_u
                    || s.periodic_v
                    || s.degree_u != p
                    || s.degree_v != 1
                    || s.control_points.len() != p + 1
                    || s.weights.len() != p + 1
                    || s.knots_u.len() != 2 * (p + 1)
                    || s.knots_u
                        .iter()
                        .enumerate()
                        .any(|(i, &k)| k != if i <= p { 0. } else { 1. })
                    || s.knots_v != [0., 0., 1., 1.]
                    || s.control_points
                        .iter()
                        .any(|r| r.len() != 2 || r.iter().any(|v| v.len() != 3))
                    || s.weights.iter().any(|r| r.len() != 2 || r[0] != r[1])
                {
                    return Ok(out);
                }
                for (endpoint, curve) in [a, b].into_iter().enumerate() {
                    pairs.push((
                        curve,
                        span,
                        Curve {
                            degree: p,
                            knots: s.knots_u.clone(),
                            control_points: s
                                .control_points
                                .iter()
                                .map(|r| r[endpoint].clone())
                                .collect(),
                            weights: s.weights.iter().map(|r| r[endpoint]).collect(),
                            periodic: false,
                        },
                    ));
                }
            }
        }
    }
    out.reason = Some("wall-coverage-unproved");
    if out.inspected_faces == 0
        || Some(out.inspected_faces) != model.faces.len().checked_sub(if closed { 0 } else { 2 })
    {
        return Ok(out);
    }
    out.reason = Some("work-limit");
    if pairs.is_empty() || pairs.len() > 2048 {
        return Ok(out);
    }
    let mut maximum = 0f64;
    for (curve, span, retained) in pairs {
        let Ok(r) = curve_decomposition_certificate::inspect(
            curve,
            span,
            &retained,
            max_products - out.products,
        ) else {
            out.reason = Some("native-span-unproved");
            out.products = 0;
            return Ok(out);
        };
        if r.products > max_products - out.products {
            return Ok(out);
        }
        out.products += r.products;
        let Some(upper) = r.error_upper else {
            out.reason = r.reason;
            return Ok(out);
        };
        maximum = maximum.max(upper);
    }
    out.certified = true;
    out.wall_error_upper = Some(maximum);
    out.reason = None;
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_low_multiplicity_spans_bind_every_actual_wall_and_refuse_partial_proofs() {
        let ring = |z| Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.25, 0.5, 0.75, 1., 1., 1.],
            control_points: vec![
                vec![1., 0., z],
                vec![1., 1., z],
                vec![-1., 1., z],
                vec![-1., -1., z],
                vec![1., -1., z],
                vec![1., 0., z],
            ],
            weights: vec![1.; 6],
            periodic: false,
        };
        let sections = vec![vec![vec![ring(0.)]], vec![vec![ring(10.)]]];
        let model = crate::rational_section_loft(&sections).unwrap();
        let r = inspect(&model, &sections, false, 100000, 1024, 1000000).unwrap();
        assert!(r.certified && r.inspected_faces == 4 && r.wall_error_upper.unwrap() < 1e-10);
        let mut shifted = model.clone();
        let walls = shifted.faces.len() - 2;
        for face in &mut shifted.faces[..walls] {
            for row in &mut face.surface.control_points {
                for pole in row {
                    pole[2] += 0.125;
                }
            }
        }
        for edge in &mut shifted.edges {
            for pole in &mut edge.curve.control_points {
                pole[2] += 0.125;
            }
        }
        let offset = inspect(&shifted, &sections, false, 100000, 1024, 1000000).unwrap();
        assert!(offset.certified && offset.wall_error_upper.unwrap() >= 0.125);
        let mut partition = sections.clone();
        partition[1].push(Vec::new());
        assert_eq!(
            inspect(&model, &partition, false, 100000, 1024, 1000000)
                .unwrap()
                .reason,
            Some("profile-partition-mismatch")
        );
        let short = inspect(&model, &sections, false, r.products - 1, 1024, 1000000).unwrap();
        assert!(
            !short.certified
                && short.wall_error_upper.is_none()
                && short.reason == Some("work-limit")
        );
        assert!(
            !inspect(&model, &sections, false, 100000, 1024, 1)
                .unwrap()
                .certified
        );
        let mut changed = model.clone();
        let wire = changed.faces[0].outer;
        let edge = changed.loops[wire].coedges[0].edge;
        changed.edges[edge].curve.control_points[0][2] += 0.125;
        assert_eq!(
            inspect(&changed, &sections, false, 100000, 1024, 1000000)
                .unwrap()
                .reason,
            Some("retained-wall-domain-unproved")
        );
        changed = model.clone();
        changed.shells[0].faces.pop();
        assert_eq!(
            inspect(&changed, &sections, false, 100000, 1024, 1000000)
                .unwrap()
                .reason,
            Some("retained-body-face-coverage-unproved")
        );
    }
}

/// Endpoint curve error plus independently owned retained planar cap regions.
/// Original contour regularity and the full sweep error are separate premises.
pub struct CapReport {
    pub certified: bool,
    pub cap_error_upper: Option<[f64; 2]>,
    pub products: usize,
    pub regions: Option<crate::sweep_retained_caps::Report>,
    pub reason: Option<&'static str>,
}
pub fn inspect_caps(
    model: &Model,
    endpoints: &[Vec<Vec<Curve>>; 2],
    budgets: crate::sweep_cap_contacts::Budgets,
    max_products: usize,
    max_edges: usize,
) -> Result<CapReport> {
    let mut out = CapReport {
        certified: false,
        cap_error_upper: None,
        products: 0,
        regions: None,
        reason: Some("work-limit"),
    };
    if max_products > 1000000 || max_edges == 0 || max_edges > 1024 {
        return Ok(out);
    }
    let mut prepared = [Vec::new(), Vec::new()];
    let mut bounds = [0f64; 2];
    let mut edges = 0;
    for (endpoint, rings) in endpoints.iter().enumerate() {
        if rings.len() > 1024 {
            return Ok(out);
        }
        let mut pairs = Vec::new();
        for ring in rings {
            if ring.len() > 1024 {
                return Ok(out);
            }
            let mut curves = Vec::new();
            for original in ring {
                if original.validate().is_err() {
                    out.reason = Some("native-span-unproved");
                    return Ok(out);
                }
                let parts = if let Some(copied) =
                    nurbs_core::retained_wall_coefficients::segmented_bezier_controls(
                        original, 1000000,
                    ) {
                    copied
                } else {
                    let Ok(decomposed) = original.decompose() else {
                        out.reason = Some("native-span-unproved");
                        return Ok(out);
                    };
                    decomposed
                        .into_iter()
                        .map(|p| {
                            let mut c = p.definition().clone();
                            c.knots = std::iter::repeat_n(0., original.degree + 1)
                                .chain(std::iter::repeat_n(1., original.degree + 1))
                                .collect();
                            c
                        })
                        .collect()
                };
                let spans = (original.degree..original.control_points.len())
                    .filter(|&i| original.knots[i] < original.knots[i + 1])
                    .collect::<Vec<_>>();
                if parts.is_empty() || parts.len() != spans.len() {
                    out.reason = Some("source-span-layout-unproved");
                    return Ok(out);
                }
                if parts.len() > max_edges - edges {
                    return Ok(out);
                }
                edges += parts.len();
                for (span, part) in spans.into_iter().zip(&parts) {
                    pairs.push((original, span, part.clone()));
                }
                curves.extend(parts);
            }
            prepared[endpoint].push(curves);
        }
        if pairs.is_empty() || pairs.len() > 2048 {
            return Ok(out);
        }
        for (original, span, part) in pairs {
            let Ok(r) = curve_decomposition_certificate::inspect(
                original,
                span,
                &part,
                max_products - out.products,
            ) else {
                out.reason = Some("native-span-unproved");
                return Ok(out);
            };
            if r.products > max_products - out.products {
                return Ok(out);
            }
            out.products += r.products;
            let Some(upper) = r.error_upper else {
                out.reason = r.reason.or(Some("decomposition-unproved"));
                return Ok(out);
            };
            bounds[endpoint] = bounds[endpoint].max(upper);
        }
    }
    let regions = crate::sweep_retained_caps::inspect(model, &prepared, budgets, max_edges)
        .unwrap_or(crate::sweep_retained_caps::Report {
            exact: false,
            exact_work: 0,
            inspected_edges: 0,
            reason: "cap-contour-mismatch",
        });
    let exact = regions.exact;
    out.reason = Some(regions.reason);
    out.regions = Some(regions);
    if !exact {
        return Ok(out);
    }
    out.certified = true;
    out.cap_error_upper = Some(bounds);
    out.reason = None;
    Ok(out)
}

#[cfg(test)]
mod cap_tests {
    use super::*;
    #[test]
    fn two_endpoints_share_curve_products_and_require_actual_regions() {
        let sections = [0., 10.]
            .iter()
            .map(|&z| {
                vec![vec![
                    nurbs_core::primitives::circle([0., 0., z], [0., 0., 1.], 0.5).unwrap(),
                ]]
            })
            .collect::<Vec<_>>();
        let model = crate::rational_section_loft(&sections).unwrap();
        let endpoints = [sections[0].clone(), sections[1].clone()];
        let budgets = crate::sweep_cap_contacts::Budgets {
            max_walls: 1024,
            max_exact_work: 1000000,
            max_chart_cells: 100000,
            max_trim_pairs: 100000,
            max_trim_cells: 100000,
            max_trim_domain_cells: 100000,
        };
        let r = inspect_caps(&model, &endpoints, budgets, 100000, 1024).unwrap();
        assert!(r.certified && r.regions.as_ref().unwrap().exact);
        assert!(r.cap_error_upper.unwrap().iter().all(|&v| v < 1e-10));
        let ring = |z| Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.25, 0.5, 0.75, 1., 1., 1.],
            control_points: vec![
                vec![1., 0., z],
                vec![1., 1., z],
                vec![-1., 1., z],
                vec![-1., -1., z],
                vec![1., -1., z],
                vec![1., 0., z],
            ],
            weights: vec![1.; 6],
            periodic: false,
        };
        let low_sections = vec![vec![vec![ring(0.)]], vec![vec![ring(10.)]]];
        let low_model = crate::rational_section_loft(&low_sections).unwrap();
        let low = inspect_caps(
            &low_model,
            &[low_sections[0].clone(), low_sections[1].clone()],
            budgets,
            100000,
            1024,
        )
        .unwrap();
        assert!(low.certified && low.cap_error_upper.unwrap().iter().all(|&v| v < 1e-10));
        let hollow_sections = [0., 10.]
            .iter()
            .map(|&z| {
                vec![
                    vec![nurbs_core::primitives::circle([0., 0., z], [0., 0., 1.], 0.5).unwrap()],
                    vec![
                        nurbs_core::primitives::circle([0., 0., z], [0., 0., 1.], 0.2)
                            .unwrap()
                            .reverse()
                            .unwrap(),
                    ],
                ]
            })
            .collect::<Vec<_>>();
        let hollow_model = crate::rational_section_loft(&hollow_sections).unwrap();
        let hollow_endpoints = [hollow_sections[0].clone(), hollow_sections[1].clone()];
        let hollow = inspect_caps(&hollow_model, &hollow_endpoints, budgets, 100000, 1024).unwrap();
        assert!(hollow.certified && hollow.regions.as_ref().unwrap().exact);
        let mut omitted_hole = hollow_model.clone();
        let cap = omitted_hole.faces.len() - 2;
        omitted_hole.faces[cap].holes.clear();
        assert!(
            !inspect_caps(&omitted_hole, &hollow_endpoints, budgets, 100000, 1024)
                .unwrap()
                .certified
        );
        let short = inspect_caps(&model, &endpoints, budgets, r.products - 1, 1024).unwrap();
        assert!(
            !short.certified && short.cap_error_upper.is_none() && short.products <= r.products - 1
        );
        let mut damaged = model.clone();
        let wire = damaged.faces[damaged.faces.len() - 2].outer;
        let edge = damaged.loops[wire].coedges[0].edge;
        damaged.edges[edge].curve.control_points[0][0] += 0.125;
        let r = inspect_caps(&damaged, &endpoints, budgets, 100000, 1024).unwrap();
        assert!(!r.certified && r.cap_error_upper.is_none());
        assert!(
            !inspect_caps(&model, &endpoints, budgets, 100000, 1)
                .unwrap()
                .certified
        );
    }
}
