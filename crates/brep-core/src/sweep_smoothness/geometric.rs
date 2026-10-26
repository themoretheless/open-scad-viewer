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
                let surfaces = [&model.faces[*a].surface, &model.faces[*b].surface];
                let pcurves = [&ca.pcurve, &cb.pcurve];
                let reversed = [ca.reversed, cb.reversed];
                let p = surface_g2_pcurves::inspect(
                    &model.edges[edge].curve,
                    pcurves,
                    surfaces,
                    reversed,
                    t.position,
                    t.normal,
                    t.curvature,
                    max_cells - out.cells,
                    max_exact - out.exact_work,
                )?;
                out.exact_work += p.positions.exact_work;
                positions = Some(p.positions);
                Some(p.geometric)
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
                out.seams.iter().any(|s| {
                    s.edge == *e
                        && s.inspection
                            .as_ref()
                            .is_some_and(|r| r.decision == Decision::WithinTolerance)
                })
            });
        out.corners.push(Corner {
            vertex,
            edges: star,
            faces: link.keys().copied().collect(),
            omitted_edges,
            closed_link: closed,
            all_incident_seams_within_tolerance,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod corner_tests {
    use super::*;
    #[test]
    fn explicit_exact_ledger_never_exceeds_the_callers_remaining_work() {
        let model=crate::cuboid([0.;3],[10.;3]).unwrap();let before=model.clone();
        let t=Tolerances{position:1e-7,normal:1e-5,curvature:1e-4};
        for budget in [0,1,128] {
            let report=inspect_edges_with_exact(&model,&[0,1,2],t,256,budget).unwrap();
            assert!(report.exact_work<=budget&&report.cells<=256);
            assert!(!report.within_tolerance);
        }
        assert!(inspect_edges_with_exact(&model,&[0],t,256,1000001).is_err());
        assert_eq!(model,before);
    }
    #[test]
    fn incomplete_star_and_zero_work_never_certify_a_corner() {
        let model=crate::cuboid([0.;3],[10.;3]).unwrap();let before=model.clone();
        let edge=0;let vertex=model.edges[edge].vertices[0];
        let t=Tolerances{position:1e-7,normal:1e-7,curvature:1e-5};
        let partial=inspect_edges(&model,&[edge],t,0).unwrap();
        let c=partial.corners.iter().find(|c|c.vertex==vertex).unwrap();
        assert!(c.closed_link);assert_eq!(c.omitted_edges.len(),2);assert!(!c.all_incident_seams_within_tolerance);
        let complete=inspect_edges(&model,&c.edges,t,0).unwrap();
        let c=complete.corners.iter().find(|c|c.vertex==vertex).unwrap();
        assert!(c.omitted_edges.is_empty());assert!(!c.all_incident_seams_within_tolerance);
        assert_eq!(model,before);
    }
}
