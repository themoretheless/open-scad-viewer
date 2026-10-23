//! Exact retained-family and whole-domain premises. Embedding remains separate.
use crate::{Model, Result};
use nurbs_core::{curve::Curve, retained_wall_coefficients as coefficients};
#[derive(Clone, Debug)]
pub struct Report {
    pub exact: bool,
    pub inspected_faces: usize,
    pub work: u64,
    pub reason: &'static str,
}
fn coverage(model: &Model) -> bool {
    if model.bodies.len() != 1 || model.faces.is_empty() || model.faces.len() > 1026 {
        return false;
    }
    let body = &model.bodies[0];
    let shells = std::iter::once(body.outer_shell)
        .chain(body.inner_shells.iter().copied())
        .collect::<Vec<_>>();
    if shells.len() != model.shells.len()
        || shells
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != shells.len()
    {
        return false;
    }
    let mut seen = vec![false; model.faces.len()];
    for shell in shells {
        let Some(shell) = model.shells.get(shell) else {
            return false;
        };
        if !shell.closed || shell.faces.is_empty() {
            return false;
        }
        for usage in &shell.faces {
            let Some(visited) = seen.get_mut(usage.face) else {
                return false;
            };
            if *visited {
                return false;
            }
            *visited = true;
        }
    }
    seen.into_iter().all(|x| x)
}
pub fn domain(model: &Model, face: usize, max_work: u64) -> Result<(bool, u64)> {
    let Some(face) = model.faces.get(face) else {
        return Ok((false, 0));
    };
    let Some(wire) = model.loops.get(face.outer) else {
        return Ok((false, 0));
    };
    let mut uses = Vec::new();
    for coedge in &wire.coedges {
        let Some(edge) = model.edges.get(coedge.edge) else {
            return Ok((false, 0));
        };
        uses.push((&edge.curve, &coedge.pcurve, coedge.reversed));
    }
    Ok(
        nurbs_core::retained_wall_domain::covers(&face.surface, &face.holes, &uses, max_work)
            .unwrap_or((false, max_work)),
    )
}
pub fn inspect(
    model: &Model,
    sections: &[Vec<Vec<Curve>>],
    closed: bool,
    max_faces: usize,
    max_work: u64,
) -> Result<Report> {
    let mut out = Report {
        exact: false,
        inspected_faces: 0,
        work: 0,
        reason: "work-limit",
    };
    if !(1..=1024).contains(&max_faces)
        || !(1..=1_000_000).contains(&max_work)
        || model.faces.len() > max_faces + if closed { 0 } else { 2 }
    {
        return Ok(out);
    }
    out.reason = "unsupported-section-decomposition";
    if !(2..=1025).contains(&sections.len()) {
        return Ok(out);
    }
    out.reason = "retained-body-face-coverage-unproved";
    if !coverage(model) {
        return Ok(out);
    }
    out.reason = "unsupported-section-decomposition";
    for section in sections {
        if section.is_empty() || section.iter().flatten().next().is_none() {
            return Ok(out);
        }
        for curve in section.iter().flatten() {
            if coefficients::segmented_bezier_controls(curve, 1_000_000).is_none() {
                return Ok(out);
            }
        }
    }
    out.reason = "retained-wall-mismatch";
    let surfaces = model
        .faces
        .iter()
        .map(|f| f.surface.clone())
        .collect::<Vec<_>>();
    if !coefficients::family_matches(&surfaces, sections, closed, max_faces) {
        return Ok(out);
    }
    out.reason = "retained-wall-domain-unproved";
    for face in 0..model.faces.len() - if closed { 0 } else { 2 } {
        if out.work >= max_work {
            return Ok(out);
        }
        let (certified, work) = domain(model, face, max_work - out.work)?;
        out.work += work;
        if !certified {
            return Ok(out);
        }
        out.inspected_faces += 1;
    }
    out.exact = true;
    out.reason = "exact-retained-coefficients";
    Ok(out)
}

#[derive(Clone, Debug)]
pub struct DecompositionReport {
    pub error_upper: Option<f64>,
    pub inspected_faces: usize,
    pub products: usize,
    pub reason: Option<&'static str>,
}
/// Certify all actual ruled faces against original spans under shared budgets.
pub fn decomposition(
    model: &Model,
    sections: &[Vec<Vec<Curve>>],
    closed: bool,
    max_products: usize,
    max_faces: usize,
    max_work: u64,
) -> Result<DecompositionReport> {
    let mut out = DecompositionReport {
        error_upper: None,
        inspected_faces: 0,
        products: 0,
        reason: Some("work-limit"),
    };
    if !(1..=1024).contains(&max_faces)
        || !(1..=1_000_000).contains(&max_work)
        || max_products > 1_000_000
        || !(2..=1025).contains(&sections.len())
        || model.faces.len() > max_faces + if closed { 0 } else { 2 }
    {
        return Ok(out);
    }
    out.reason = Some("retained-body-face-coverage-unproved");
    if !coverage(model) {
        return Ok(out);
    }
    out.reason = Some("profile-partition-mismatch");
    if sections[0].iter().flatten().next().is_none()
        || sections.iter().any(|section| {
            section.len() != sections[0].len()
                || section
                    .iter()
                    .zip(&sections[0])
                    .any(|(a, b)| a.len() != b.len())
        })
    {
        return Ok(out);
    }
    let rows = sections
        .iter()
        .map(|s| s.iter().flatten().collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let mut owned = Vec::new();
    let mut work = 0;
    for layer in rows.windows(2) {
        for (&a, &b) in layer[0].iter().zip(&layer[1]) {
            if a.validate().is_err() || b.validate().is_err() {
                out.reason = Some("native-span-unproved");
                return Ok(out);
            }
            out.reason = Some("source-profile-correspondence-unproved");
            if a.periodic != b.periodic
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
                let (accepted, used) = domain(model, out.inspected_faces, max_work - work)?;
                work += used;
                if !accepted {
                    return Ok(out);
                }
                let s = &model.faces[out.inspected_faces].surface;
                out.inspected_faces += 1;
                let p = a.degree;
                out.reason = Some("retained-ruled-face-unproved");
                if s.periodic_u
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
                    || s.control_points.iter().any(|row| {
                        row.len() != 2
                            || row.iter().any(|point| {
                                point.len() != 3 || point.iter().any(|x| !x.is_finite())
                            })
                    })
                    || s.weights.iter().any(|row| {
                        row.len() != 2 || !row[0].is_finite() || row[0] <= 0. || row[0] != row[1]
                    })
                {
                    return Ok(out);
                }
                for (endpoint, original) in [a, b].into_iter().enumerate() {
                    owned.push((
                        original,
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
        || out.inspected_faces != model.faces.len().saturating_sub(if closed { 0 } else { 2 })
    {
        return Ok(out);
    }
    let pairs = owned
        .iter()
        .map(|(a, span, b)| (*a, *span, b))
        .collect::<Vec<_>>();
    let batch = nurbs_core::curve_decomposition_certificate::inspect_batch(&pairs, max_products)?;
    out.error_upper = batch.error_upper;
    out.products = batch.products;
    out.reason = batch.reason;
    Ok(out)
}

#[derive(Clone, Debug)]
pub struct CapDecompositionReport {
    pub error_upper: Option<[f64; 2]>,
    pub products: usize,
    pub regions: Option<crate::sweep_cap_contacts::RetainedCapsReport>,
    pub reason: Option<&'static str>,
}
/// Preserve the outer/hole partition while certifying both endpoint decompositions.
pub fn cap_decomposition(
    model: &Model,
    endpoints: &[Vec<Vec<Curve>>],
    budgets: crate::sweep_cap_contacts::Budgets,
    max_products: usize,
    max_edges: usize,
) -> Result<CapDecompositionReport> {
    let mut out = CapDecompositionReport {
        error_upper: None,
        products: 0,
        regions: None,
        reason: Some("work-limit"),
    };
    if max_products > 1_000_000 || !(1..=1024).contains(&max_edges) || endpoints.len() != 2 {
        return Ok(out);
    }
    let mut prepared = Vec::new();
    let mut bounds = [0.; 2];
    let mut edges = 0;
    for (endpoint_index, endpoint) in endpoints.iter().enumerate() {
        let mut rings = Vec::new();
        let mut owned = Vec::new();
        for ring in endpoint {
            let mut curves = Vec::new();
            for original in ring {
                if original.validate().is_err() {
                    out.reason = Some("native-span-unproved");
                    return Ok(out);
                }
                let pieces = match coefficients::segmented_bezier_controls(original, 1_000_000) {
                    Some(parts) => parts,
                    None => match original.decompose() {
                        Ok(parts) => parts.iter().map(|part| part.definition().clone()).collect(),
                        Err(_) => {
                            out.reason = Some("native-span-unproved");
                            return Ok(out);
                        }
                    },
                };
                let spans = (original.degree..original.control_points.len())
                    .filter(|&s| original.knots[s] < original.knots[s + 1])
                    .collect::<Vec<_>>();
                out.reason = Some("source-span-layout-unproved");
                if pieces.is_empty() || pieces.len() != spans.len() {
                    return Ok(out);
                }
                edges += pieces.len();
                out.reason = Some("work-limit");
                if edges > max_edges {
                    return Ok(out);
                }
                for (span, mut retained) in spans.into_iter().zip(pieces) {
                    retained.knots = std::iter::repeat_n(0., original.degree + 1)
                        .chain(std::iter::repeat_n(1., original.degree + 1))
                        .collect();
                    curves.push(retained.clone());
                    owned.push((original, span, retained));
                }
            }
            rings.push(curves);
        }
        let pairs = owned
            .iter()
            .map(|(original, span, retained)| (*original, *span, retained))
            .collect::<Vec<_>>();
        let audit = nurbs_core::curve_decomposition_certificate::inspect_batch(
            &pairs,
            max_products - out.products,
        )?;
        out.products += audit.products;
        let Some(upper) = audit.error_upper else {
            out.reason = Some(audit.reason.unwrap_or("decomposition-unproved"));
            return Ok(out);
        };
        bounds[endpoint_index] = upper;
        prepared.push(rings);
    }
    let regions = match crate::sweep_cap_contacts::inspect_retained_caps(
        model, &prepared, budgets, max_edges,
    ) {
        Ok(r) => r,
        Err(_) => {
            out.reason = Some("native-span-unproved");
            return Ok(out);
        }
    };
    let accepted = regions.exact;
    out.reason = if accepted { None } else { Some(regions.reason) };
    out.regions = Some(regions);
    if accepted {
        out.error_upper = Some(bounds);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ring(z: f64) -> Curve {
        Curve {
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
        }
    }
    #[test]
    fn decomposition_is_atomic_and_preserves_originals() {
        let sections = vec![vec![vec![ring(0.)]], vec![vec![ring(10.)]]];
        let model = crate::rational_section_loft(&sections).unwrap();
        let before = model.clone();
        let originals = sections.clone();
        let wall = decomposition(&model, &sections, false, 100000, 1024, 1000000).unwrap();
        assert!(wall.error_upper.unwrap() < 1e-10, "{wall:?}");
        assert_eq!(wall.inspected_faces, 4);
        assert!(
            decomposition(&model, &sections, false, wall.products - 1, 1024, 1000000)
                .unwrap()
                .error_upper
                .is_none()
        );
        assert!(
            decomposition(&model, &sections, false, 100000, 1024, 1)
                .unwrap()
                .error_upper
                .is_none()
        );
        let budgets = crate::sweep_cap_contacts::Budgets {
            max_walls: 1024,
            max_exact_work: 1000000,
            max_chart_cells: 10000,
            max_trim_pairs: 10000,
            max_trim_cells: 100000,
            max_trim_domain_cells: 1000000,
        };
        let caps = cap_decomposition(&model, &sections, budgets, 100000, 1024).unwrap();
        assert!(
            caps.error_upper.unwrap().iter().all(|&x| x < 1e-10),
            "{caps:?}"
        );
        assert!(
            cap_decomposition(&model, &sections, budgets, caps.products - 1, 1024)
                .unwrap()
                .error_upper
                .is_none()
        );
        assert!(
            cap_decomposition(
                &model,
                &sections,
                crate::sweep_cap_contacts::Budgets {
                    max_exact_work: 1,
                    ..budgets
                },
                100000,
                1024
            )
            .unwrap()
            .error_upper
            .is_none()
        );
        let mut orphan = model.clone();
        orphan.shells[0].faces.remove(0);
        assert_eq!(
            decomposition(&orphan, &sections, false, 100000, 1024, 1000000)
                .unwrap()
                .reason,
            Some("retained-body-face-coverage-unproved")
        );
        let mut broken = model.clone();
        let cap = broken.faces.last().unwrap().outer;
        let edge = broken.loops[cap].coedges[0].edge;
        broken.edges[edge].curve.control_points[0][0] += 0.125;
        assert!(
            cap_decomposition(&broken, &sections, budgets, 100000, 1024)
                .unwrap()
                .error_upper
                .is_none()
        );
        assert_eq!(model.0, before.0);
        assert_eq!(sections, originals);
    }
}
