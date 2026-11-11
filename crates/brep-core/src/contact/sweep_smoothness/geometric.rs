//! Owned-edge, whole-interval geometric G2 inspection. Exact strip identities
//! remain a separate claim; tolerance acceptance never upgrades their flags.
use crate::{Error, Model, Result};
use nurbs_core::{
    surface_g2, surface_g2_pcurves,
    surface_join::{Boundary, Decision},
};
#[derive(Clone, Copy)]
pub struct Tolerances {
    pub position: f64,
    pub normal: f64,
    pub curvature: f64,
}
pub struct Seam {
    pub edge: usize,
    pub faces: Vec<usize>,
    pub inspection: Option<surface_g2::Report>,
    pub positions: Option<surface_g2_pcurves::Positions>,
}
pub struct Corner {
    pub vertex: usize,
    pub edges: Vec<usize>,
    pub faces: Vec<usize>,
    pub omitted_edges: Vec<usize>,
    pub closed_link: bool,
    pub all_incident_seams_within_tolerance: bool,
}
pub struct Report {
    pub complete: bool,
    pub within_tolerance: bool,
    pub cells: usize,
    pub exact_work: u64,
    pub seams: Vec<Seam>,
    pub corners: Vec<Corner>,
}
/// The edge list is a scope declaration, never a supplied proof. Every result
/// comes from the actual incident faces and their complete original boundaries.
pub fn inspect_edges(
    model: &Model,
    edges: &[usize],
    t: Tolerances,
    max_cells: usize,
) -> Result<Report> {
    inspect_edges_with_exact(model, edges, t, max_cells, 1000000)
}
/// Shared exact-work limit for callers combining assembly and seam admission.
pub fn inspect_edges_with_exact(
    model: &Model,
    edges: &[usize],
    t: Tolerances,
    max_cells: usize,
    max_exact: u64,
) -> Result<Report> {
    inspect_edges_impl(model, edges, t, max_cells, max_exact, None).map(|p| p.report)
}
pub struct GraphReport {
    pub report: Report,
    pub coefficient_work: usize,
}
/// Fresh saved-coedge graph proofs share one explicit coefficient ledger.
/// Unsupported graph charts retain the generic original-edge path.
pub fn inspect_edges_with_graph_work(
    model: &Model,
    edges: &[usize],
    t: Tolerances,
    max_cells: usize,
    max_exact: u64,
    max_coefficients: usize,
) -> Result<GraphReport> {
    if (max_coefficients as u64) > 50_000_000_000 {
        return Err(Error::new(
            "BREP_G2_LIMITS",
            "Invalid graph coefficient budget",
        ));
    }
    inspect_edges_impl(
        model,
        edges,
        t,
        max_cells,
        max_exact,
        Some(max_coefficients),
    )
}
type EdgeUseRef<'a> = (usize, Option<Boundary>, Option<bool>, &'a crate::Coedge);

fn collect_edge_uses(model: &Model) -> Vec<Vec<EdgeUseRef<'_>>> {
    let mut uses = vec![Vec::new(); model.edges.len()];
    for (face, f) in model.faces.iter().enumerate() {
        for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            for c in &model.loops[wire].coedges {
                let label = super::boundary(&f.surface, &c.pcurve, super::Axis::Profile)
                    .or_else(|| super::boundary(&f.surface, &c.pcurve, super::Axis::Station));
                let side = label.map(|label| match label {
                    "uMin" => Boundary::UMin,
                    "uMax" => Boundary::UMax,
                    "vMin" => Boundary::VMin,
                    "vMax" => Boundary::VMax,
                    _ => unreachable!(),
                });
                let increasing = side.map(|side| {
                    let free = usize::from(matches!(side, Boundary::UMin | Boundary::UMax));
                    (c.pcurve.control_points.last().unwrap()[free]
                        > c.pcurve.control_points[0][free])
                        != c.reversed
                });
                uses[c.edge].push((face, side, increasing, c));
            }
        }
    }
    uses
}

fn coedge_clock_reversed(model: &Model, edge: usize, c: &crate::Coedge) -> Result<bool> {
    let Some(trim) = c.trim() else {
        return Ok(c.reversed);
    };
    let cd = model.edges[edge].curve.domain();
    let pd = c.pcurve.domain();
    let backward = |r: [f64; 2], d: [f64; 2]| {
        if r == d {
            Some(false)
        } else if r == [d[1], d[0]] {
            Some(true)
        } else {
            None
        }
    };
    if trim.periodic_lift != [[0; 2]; 2] {
        return Err(Error::new(
            "BREP_G2_SCOPE",
            &format!("edge={edge}: periodic coedge lifts need a trim-aware G2 proof"),
        ));
    }
    match (
        backward(trim.curve_parameter, cd),
        backward(trim.pcurve_parameter, pd),
    ) {
        (Some(cback), Some(pback)) => Ok(c.reversed ^ cback ^ pback),
        _ => Err(Error::new(
            "BREP_G2_SCOPE",
            &format!("edge={edge}: partial authored coedge clocks need a trim-aware G2 proof"),
        )),
    }
}

fn is_c2_surface(s: &nurbs_core::surface::Surface) -> bool {
    for (p, k, n) in [
        (s.degree_u, &s.knots_u, s.control_points.len()),
        (s.degree_v, &s.knots_v, s.control_points[0].len()),
    ] {
        let mut i = p + 1;
        while i < n {
            let mut end = i + 1;
            while end < n && k[end] == k[i] {
                end += 1;
            }
            if end - i > p.saturating_sub(2) {
                return false;
            }
            i = end;
        }
    }
    true
}

fn supports_graph_seam_proof(
    surfaces: [&nurbs_core::surface::Surface; 2],
    pcurves: [&nurbs_core::curve::Curve; 2],
) -> bool {
    surfaces.iter().all(|s| {
        is_c2_surface(s)
            && !s.periodic_u
            && !s.periodic_v
            && s.degree_u <= 12
            && s.degree_v <= 12
            && s.control_points.len() * s.control_points[0].len() <= 4096
    }) && pcurves.iter().all(|p| {
        !p.periodic
            && p.degree <= 16
            && p.control_points.len() <= 4096
            && p.knots[..=p.degree].iter().all(|&k| k == p.knots[p.degree])
            && p.knots[p.control_points.len()..]
                .iter()
                .all(|&k| k == p.knots[p.control_points.len()])
    })
}

struct SeamInspectBudgets {
    tolerances: Tolerances,
    max_cells: usize,
    max_exact: u64,
    graph_work: Option<usize>,
}

fn inspect_manifold_seam(
    model: &Model,
    edge: usize,
    uses: [(usize, &crate::Coedge); 2],
    budgets: SeamInspectBudgets,
    coefficient_work: &mut usize,
    out: &mut Report,
) -> Result<(surface_g2::Report, surface_g2_pcurves::Positions)> {
    let [(a, ca), (b, cb)] = uses;
    let SeamInspectBudgets {
        tolerances: t,
        max_cells,
        max_exact,
        graph_work,
    } = budgets;
    let surfaces = [&model.faces[a].surface, &model.faces[b].surface];
    let pcurves = [&ca.pcurve, &cb.pcurve];
    let reversed = [
        coedge_clock_reversed(model, edge, ca)?,
        coedge_clock_reversed(model, edge, cb)?,
    ];
    let mut graph = None;
    let remaining = graph_work.unwrap_or(0).saturating_sub(*coefficient_work);
    let polynomial =
        |s: &nurbs_core::surface::Surface| s.weights.iter().flatten().all(|&w| w == 1.);
    if remaining > 0
        && max_cells - out.cells >= 2
        && supports_graph_seam_proof(surfaces, pcurves)
        && t.position > 0.
        && t.normal > 0.
        && t.curvature > 0.
        && (polynomial(surfaces[1]) || polynomial(surfaces[0]))
    {
        let swapped = !polynomial(surfaces[1]);
        let order = if swapped { [1, 0] } else { [0, 1] };
        let proof = surface_g2_pcurves::inspect_owned_graph_oriented(
            &model.edges[edge].curve,
            order.map(|i| pcurves[i]),
            order.map(|i| surfaces[i]),
            order.map(|i| reversed[i]),
            [t.position, t.normal, t.curvature],
            (max_cells - out.cells) / 2,
            (max_exact - out.exact_work) / 2,
            remaining,
        )
        .map_err(|e| Error::new("BREP_G2_GRAPH", &format!("edge={edge} faces=[{a},{b}]: {e}")))?;
        if proof.coefficient_work > remaining
            || (proof.owned.report.geometric.decision == Decision::WithinTolerance
                && proof.owned.positions.is_none())
        {
            return Err(Error::new(
                "BREP_G2_GRAPH",
                "Invalid saved-edge graph ownership or work receipt",
            ));
        }
        *coefficient_work += proof.coefficient_work;
        let mut report = proof.owned.report;
        if swapped {
            report.positions.sides.swap(0, 1);
        }
        graph = Some(report);
    }
    let p = if let Some(mut p) = graph {
        if p.geometric.decision == Decision::Unresolved && out.cells + p.geometric.cells < max_cells
        {
            let mut fallback = surface_g2_pcurves::inspect(
                surface_g2_pcurves::PcurveSeam { curve: &model.edges[edge].curve, pcurves, surfaces, reversed },
                surface_g2_pcurves::G2PcurveTolerance { position: t.position, normal: t.normal, curvature: t.curvature },
                max_cells - out.cells - p.geometric.cells,
                max_exact - out.exact_work - p.positions.exact_work,
            )?;
            fallback.geometric.cells += p.geometric.cells;
            fallback.positions.cells += p.positions.cells;
            fallback.positions.exact_work += p.positions.exact_work;
            p = fallback;
        }
        p
    } else {
        surface_g2_pcurves::inspect(
            surface_g2_pcurves::PcurveSeam { curve: &model.edges[edge].curve, pcurves, surfaces, reversed },
            surface_g2_pcurves::G2PcurveTolerance { position: t.position, normal: t.normal, curvature: t.curvature },
            max_cells - out.cells,
            max_exact - out.exact_work,
        )?
    };
    out.exact_work += p.positions.exact_work;
    Ok((p.geometric, p.positions))
}

fn collect_vertex_corners(
    model: &Model,
    selected: &[bool],
    uses: &[Vec<EdgeUseRef<'_>>],
    seams: &[Seam],
) -> Vec<Corner> {
    let mut corners = Vec::new();
    let mut incident = vec![Vec::new(); model.vertices.len()];
    for (edge, e) in model.edges.iter().enumerate() {
        for &vertex in &e.vertices {
            if !incident[vertex].contains(&edge) {
                incident[vertex].push(edge);
            }
        }
    }
    for (vertex, star) in incident.into_iter().enumerate() {
        if star.len() < 3 || !star.iter().any(|&e| selected[e]) {
            continue;
        }
        let omitted_edges = star
            .iter()
            .filter(|&&e| !selected[e])
            .copied()
            .collect::<Vec<_>>();
        let mut link = std::collections::BTreeMap::<usize, Vec<usize>>::new();
        let mut closed = true;
        for &edge in &star {
            if let [(a, _, _, _), (b, _, _, _)] = uses[edge].as_slice() {
                if a == b {
                    closed = false;
                    continue;
                }
                link.entry(*a).or_default().push(*b);
                link.entry(*b).or_default().push(*a);
            } else {
                closed = false;
            }
        }
        closed &= link.len() >= 3 && link.values().all(|n| n.len() == 2);
        let mut visited = std::collections::BTreeSet::new();
        let mut pending = link.keys().next().copied().into_iter().collect::<Vec<_>>();
        while let Some(face) = pending.pop() {
            if visited.insert(face) {
                pending.extend(link[&face].iter().copied());
            }
        }
        closed &= visited.len() == link.len();
        let all_incident_seams_within_tolerance = closed
            && omitted_edges.is_empty()
            && star.iter().all(|e| {
                seams.iter().any(|s| {
                    s.edge == *e
                        && s.inspection
                            .as_ref()
                            .is_some_and(|r| r.decision == Decision::WithinTolerance)
                })
            });
        corners.push(Corner {
            vertex,
            edges: star,
            faces: link.keys().copied().collect(),
            omitted_edges,
            closed_link: closed,
            all_incident_seams_within_tolerance,
        });
    }
    corners
}

fn inspect_edges_impl(
    model: &Model,
    edges: &[usize],
    t: Tolerances,
    max_cells: usize,
    max_exact: u64,
    graph_work: Option<usize>,
) -> Result<GraphReport> {
    let mut coefficient_work = 0usize;
    model.validate_boundary_diagnostic_inputs()?;
    if max_cells > 100_000
        || max_exact > 1000000
        || edges.len() > crate::MAX_ENTITIES
        || !t.position.is_finite()
        || t.position < 0.
        || !t.normal.is_finite()
        || !(0. ..=2.).contains(&t.normal)
        || !t.curvature.is_finite()
        || t.curvature < 0.
    {
        return Err(Error::new(
            "BREP_G2_LIMITS",
            "Invalid geometric seam tolerances or shared cell budget",
        ));
    }
    let mut selected = vec![false; model.edges.len()];
    for &edge in edges {
        if edge >= selected.len() || selected[edge] {
            return Err(Error::new(
                "BREP_G2_SCOPE",
                "Invalid or duplicate owned edge",
            ));
        }
        selected[edge] = true;
    }
    let uses = collect_edge_uses(model);
    let mut out = Report {
        complete: !edges.is_empty(),
        within_tolerance: !edges.is_empty(),
        cells: 0,
        exact_work: 0,
        seams: Vec::new(),
        corners: Vec::new(),
    };
    for &edge in edges {
        let owned = &uses[edge];
        let mut positions = None;
        let inspection = if model.edges[edge].degenerate {
            None
        } else if let [(a, _, _, ca), (b, _, _, cb)] = owned.as_slice() {
            if out.cells < max_cells {
                let (geom, pos) = inspect_manifold_seam(
                    model,
                    edge,
                    [(*a, ca), (*b, cb)],
                    SeamInspectBudgets {
                        tolerances: t,
                        max_cells,
                        max_exact,
                        graph_work,
                    },
                    &mut coefficient_work,
                    &mut out,
                )?;
                positions = Some(pos);
                Some(geom)
            } else {
                None
            }
        } else {
            None
        };
        if let Some(proof) = &inspection {
            out.cells += proof.cells;
            out.complete &= proof.decision != Decision::Unresolved;
            out.within_tolerance &= proof.decision == Decision::WithinTolerance;
        } else {
            out.complete = false;
            out.within_tolerance = false;
        }
        out.seams.push(Seam {
            edge,
            faces: owned.iter().map(|u| u.0).collect(),
            inspection,
            positions,
        });
    }
    out.corners = collect_vertex_corners(model, &selected, &uses, &out.seams);
    Ok(GraphReport {
        report: out,
        coefficient_work,
    })
}

#[cfg(test)]
mod corner_tests {
    use super::*;
    #[test]
    fn explicit_exact_ledger_never_exceeds_the_callers_remaining_work() {
        let model = crate::cuboid([0.; 3], [10.; 3]).unwrap();
        let before = model.clone();
        let t = Tolerances {
            position: 1e-7,
            normal: 1e-5,
            curvature: 1e-4,
        };
        for budget in [0, 1, 128] {
            let report = inspect_edges_with_exact(&model, &[0, 1, 2], t, 256, budget).unwrap();
            assert!(report.exact_work <= budget && report.cells <= 256);
            assert!(!report.within_tolerance);
        }
        assert!(inspect_edges_with_exact(&model, &[0], t, 256, 1000001).is_err());
        assert_eq!(model, before);
    }
    #[test]
    fn graph_inspection_keeps_the_explicit_shared_ledgers() {
        let model = crate::cuboid([0.; 3], [10.; 3]).unwrap();
        let before = model.clone();
        let t = Tolerances {
            position: 1e-7,
            normal: 1e-5,
            curvature: 1e-4,
        };
        let legacy = inspect_edges_with_exact(&model, &[0], t, 256, 100000).unwrap();
        let zero = inspect_edges_with_graph_work(&model, &[0], t, 256, 100000, 0).unwrap();
        assert_eq!(zero.coefficient_work, 0);
        assert_eq!(zero.report.cells, legacy.cells);
        assert_eq!(zero.report.exact_work, legacy.exact_work);
        assert_eq!(zero.report.within_tolerance, legacy.within_tolerance);
        for budget in [1, 10000, 100000] {
            if let Ok(proof) = inspect_edges_with_graph_work(&model, &[0], t, 256, 100000, budget) {
                assert!(
                    proof.coefficient_work <= budget
                        && proof.report.cells <= 256
                        && proof.report.exact_work <= 100000
                );
                assert!(
                    !proof.report.within_tolerance,
                    "sharp cube edge must not pass G2"
                );
            }
        }
        assert_eq!(model, before);
    }
    #[test]
    fn incomplete_star_and_zero_work_never_certify_a_corner() {
        let model = crate::cuboid([0.; 3], [10.; 3]).unwrap();
        let before = model.clone();
        let edge = 0;
        let vertex = model.edges[edge].vertices[0];
        let t = Tolerances {
            position: 1e-7,
            normal: 1e-7,
            curvature: 1e-5,
        };
        let partial = inspect_edges(&model, &[edge], t, 0).unwrap();
        let c = partial.corners.iter().find(|c| c.vertex == vertex).unwrap();
        assert!(c.closed_link);
        assert_eq!(c.omitted_edges.len(), 2);
        assert!(!c.all_incident_seams_within_tolerance);
        let complete = inspect_edges(&model, &c.edges, t, 0).unwrap();
        let c = complete
            .corners
            .iter()
            .find(|c| c.vertex == vertex)
            .unwrap();
        assert!(c.omitted_edges.is_empty());
        assert!(!c.all_incident_seams_within_tolerance);
        assert_eq!(model, before);
    }
}
