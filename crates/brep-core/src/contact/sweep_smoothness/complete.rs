//! Complete owned boundary joins, including cap/wall and closed seam uses.
//! No caller-supplied cap list or seam list can hide a sharp or unclassified join.
use super::*;
#[derive(Clone, Copy)]
pub struct Limits {
    pub order: usize,
    pub exact_work: u64,
    pub agreement_work: u64,
    /// Shared cells for face rank and exact interior knot joins.
    pub regularity_cells: usize,
}
pub struct Report {
    pub extraction_complete: bool,
    /// Edges indexed by the standalone strip proof collection.
    pub edge_ids: Vec<usize>,
    /// Physical same-face seam edges proved by the original periodic atlas.
    pub periodic_edge_ids: Vec<usize>,
    pub unresolved_edges: Vec<usize>,
    pub edge_correspondence: Option<crate::boundary_agreement::ExactReport>,
    pub vertex_links_complete: bool,
    pub regularity: crate::boundary_embedding::regularity::Report,
    pub proof: ProjectiveSeamCollectionReport,
    pub face_interiors: InteriorReport,
    pub g1_certified: bool,
    pub g2_certified: bool,
    pub exact_work: u64,
    pub parameter_work: u64,
}
mod graph_jets;
pub(crate) mod interiors;
mod periodic_uses;
pub(super) use periodic_uses::selected_wrap;
pub use interiors::InteriorReport;
mod reversal;
use reversal::reverse_along;
// Every oriented face corner connects two (edge,end) nodes in the link
// of its authored vertex. One connected degree-two graph is mandatory;
// merely counting two uses per edge would admit pinched vertex neighborhoods.
pub(crate) fn vertex_links(model: &Model) -> bool {
    use std::collections::{BTreeMap, BTreeSet};
    type Node = (usize, usize);
    let mut links = vec![BTreeMap::<Node, Vec<Node>>::new(); model.vertices.len()];
    for f in &model.faces {
        for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            let coedges = &model.loops[wire].coedges;
            for i in 0..coedges.len() {
                let a = &coedges[i];
                let b = &coedges[(i + 1) % coedges.len()];
                let ae = usize::from(!a.reversed);
                let bs = usize::from(b.reversed);
                let vertex = model.edges[a.edge].vertices[ae];
                if vertex != model.edges[b.edge].vertices[bs] {
                    return false;
                }
                let x = (a.edge, ae);
                let y = (b.edge, bs);
                links[vertex].entry(x).or_default().push(y);
                links[vertex].entry(y).or_default().push(x);
            }
        }
    }
    links.into_iter().all(|graph| {
        if graph.is_empty() || graph.values().any(|n| n.len() != 2) {
            return false;
        }
        let mut visited = BTreeSet::new();
        let mut pending = vec![*graph.first_key_value().unwrap().0];
        while let Some(n) = pending.pop() {
            if visited.insert(n) {
                pending.extend(graph[&n].iter().copied());
            }
        }
        visited.len() == graph.len()
    })
}
pub fn inspect(model: &Model, limits: Limits) -> Result<Report> {
    model.validate_boundary_diagnostic_inputs()?;
    require(
        matches!(limits.order, 1 | 2)
            && limits.exact_work <= continuity::MAX_PROJECTIVE_SEAM_COLLECTION_WORK
            && limits.agreement_work <= 1000000
            && limits.regularity_cells <= 100000,
        "Invalid complete boundary smoothness budget",
    )?;
    let mut uses = vec![Vec::new(); model.edges.len()];
    let mut parameter_work = 0;
    for (face, f) in model.faces.iter().enumerate() {
        for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            for c in &model.loops[wire].coedges {
                let (side, used) = periodic_uses::full_side(
                    model, &f.surface, c, limits.exact_work - parameter_work,
                );
                parameter_work += used;
                uses[c.edge].push((face, side.map(|s| s.0), side.map(|s| s.1), c.reversed));
            }
        }
    }
    let mut patches = Vec::new();
    let mut seams = Vec::new();
    let mut edge_ids = Vec::new();
    let mut unresolved_edges = Vec::new();
    let mut periodic_candidates = Vec::new();
    for (edge, owned) in uses.iter().enumerate() {
        let [(a, Some(ab), Some(ad), ar), (b, Some(bb), Some(bd), br)] = owned.as_slice() else {
            unresolved_edges.push(edge);
            continue;
        };
        if a == b && periodic_uses::opposite_sides(&model.faces[*a].surface, ab, bb) {
            if ad == bd && ar != br {
                periodic_candidates.push(edge);
            } else {
                unresolved_edges.push(edge);
            }
            continue;
        }
        let left = model.faces[*a].surface.clone();
        let mut right = model.faces[*b].surface.clone();
        if ad != bd {
            let (reversed, work) =
                reverse_along(&mut right, bb, limits.exact_work - parameter_work)?;
            parameter_work += work;
            if !reversed {
                unresolved_edges.push(edge);
                continue;
            }
        }
        let start = patches.len();
        patches.push(left);
        patches.push(right);
        seams.push(ProjectiveSeam {
            patches: [start, start + 1],
            boundaries: [(*ab).into(), (*bb).into()],
            order: limits.order,
            normal_scale: 1.,
        });
        edge_ids.push(edge);
    }
    // Correspondence is a distinct shared ledger; neither a seam identity nor
    // matching endpoint samples may stand in for the authored world edge.
    let correspondence = if limits.agreement_work > 0 {
        Some(crate::boundary_agreement::verify_exact(
            model,
            limits.agreement_work,
        )?)
    } else {
        None
    };
    let vertex_links_complete = vertex_links(model);
    let regularity =
        crate::boundary_embedding::regularity::inspect(model, limits.regularity_cells)?;
    let world_owned = vertex_links_complete
        && correspondence
            .as_ref()
            .is_some_and(|p| p.all_equal && p.all_joins_exact);
    let proof = graph_jets::qualify(&patches, &seams, limits.exact_work - parameter_work)?;
    let seam_work = parameter_work + proof.exact_work;
    let face_interiors = interiors::inspect(
        model,
        limits.order,
        limits.regularity_cells - regularity.cells,
        limits.exact_work - seam_work,
    )?;
    let periodic_edge_ids = if world_owned && face_interiors.certified {
        periodic_candidates
    } else {
        unresolved_edges.extend(periodic_candidates);
        unresolved_edges.sort_unstable();
        Vec::new()
    };
    let complete = world_owned
        && !(edge_ids.is_empty() && periodic_edge_ids.is_empty())
        && unresolved_edges.is_empty();
    let ordinary_proven = edge_ids.is_empty() || proof.certified;
    let g1_certified =
        complete && regularity.all_faces_regular && ordinary_proven && face_interiors.certified;
    let g2_certified = g1_certified && limits.order == 2;
    let work = seam_work + face_interiors.exact_work;
    Ok(Report {
        extraction_complete: complete,
        edge_ids,
        periodic_edge_ids,
        unresolved_edges,
        edge_correspondence: correspondence,
        vertex_links_complete,
        regularity,
        proof,
        face_interiors,
        g1_certified,
        g2_certified,
        exact_work: work,
        parameter_work,
    })
}
#[cfg(test)]
mod reversal_tests;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn caps_cannot_be_omitted_to_upgrade_a_sharp_closed_body() {
        let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let before = model.clone();
        let r = inspect(
            &model,
            Limits {
                order: 1,
                exact_work: 2000000,
                agreement_work: 1000000,
                regularity_cells: 100000,
            },
        )
        .unwrap();
        assert!(r.extraction_complete);
        assert_eq!(r.edge_ids.len(), model.edges.len());
        assert!(!r.g1_certified && !r.g2_certified);
        assert!(r.exact_work <= 2000000);
        let zero = inspect(
            &model,
            Limits {
                order: 1,
                exact_work: 0,
                agreement_work: 0,
                regularity_cells: 0,
            },
        )
        .unwrap();
        assert!(!zero.extraction_complete && !zero.g1_certified && !zero.g2_certified);
        assert_eq!(zero.exact_work, 0);
        assert_eq!(model, before);
    }
    #[test]
    fn all_closed_torus_joins_are_inspected_and_source_edge_mutations_refuse() {
        let mut model = crate::torus(4., 1.).unwrap();
        // Author a binary-exact coefficient fixture. The public torus producer
        // uses trigonometric proposals; this fixture does not grant it credit.
        for f in &mut model.faces {
            for p in f.surface.control_points.iter_mut().flatten() {
                for x in p {
                    assert!((*x - x.round()).abs() < 1e-12);
                    *x = x.round();
                }
            }
        }
        for e in &mut model.edges {
            for p in &mut e.curve.control_points {
                for x in p {
                    assert!((*x - x.round()).abs() < 1e-12);
                    *x = x.round();
                }
            }
        }
        for v in &mut model.vertices {
            for x in &mut v.point {
                *x = x.round();
            }
        }
        for f in &mut model.faces {
            for w in f.surface.weights.iter_mut().flatten() {
                *w = if *w == 1. {
                    1.
                } else if *w > 0.6 {
                    0.5
                } else {
                    0.25
                };
            }
        }
        for e in &mut model.edges {
            for w in &mut e.curve.weights {
                if *w != 1. {
                    *w = 0.5;
                }
            }
        }
        let before = model.clone();
        let r = inspect(
            &model,
            Limits {
                order: 1,
                exact_work: 2000000,
                agreement_work: 1000000,
                regularity_cells: 100000,
            },
        )
        .unwrap();
        assert_eq!(
            r.edge_ids.len() + r.unresolved_edges.len(),
            model.edges.len()
        );
        assert!(
            r.g1_certified,
            "complete={} unresolved={:?} G2={:?} G1={:?}",
            r.extraction_complete,
            r.unresolved_edges,
            r.proof.unresolved_seams,
            r.proof.seams.iter().take(2).collect::<Vec<_>>()
        );
        let g2 = inspect(
            &model,
            Limits {
                order: 2,
                exact_work: 2000000,
                agreement_work: 1000000,
                regularity_cells: 100000,
            },
        )
        .unwrap();
        assert!(
            g2.g2_certified,
            "G2 work={} unresolved={:?} first={:?}",
            g2.exact_work,
            g2.proof.unresolved_seams,
            g2.proof.seams.first()
        );
        assert!(g2.g1_certified && g2.vertex_links_complete && g2.regularity.all_faces_regular);
        let short = inspect(
            &model,
            Limits {
                order: 2,
                exact_work: g2.exact_work - 1,
                agreement_work: 1000000,
                regularity_cells: 100000,
            },
        )
        .unwrap();
        assert!(!short.g2_certified);
        let mut trimmed = model.clone();
        let outer = trimmed.faces[0].outer;
        let edge = trimmed.loops[outer].coedges[0].edge;
        let world = trimmed.edges[edge].curve.domain();
        let c = &mut trimmed.0.loops[outer].coedges[0];
        let a = &c.pcurve.control_points[0];
        let b = &c.pcurve.control_points[1];
        c.pcurve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 4., 4., 4.],
            control_points: [-1., 0., 3.].map(|t| {
                a.iter().zip(b).map(|(a, b)| a + t * (b - a)).collect()
            }).to_vec(),
            weights: vec![1., 2., 3.],
            periodic: false,
        };
        *c = c.clone().with_trim(brep_topology::CoedgeTrim {
            curve_parameter: world,
            pcurve_parameter: [1., 2.],
            periodic_lift: [[0; 2]; 2],
        }).unwrap();
        trimmed.validate().unwrap();
        let limits = Limits {
            order: 2,
            exact_work: 2_000_000,
            agreement_work: 1_000_000,
            regularity_cells: 100_000,
        };
        let active = inspect(&trimmed, limits).unwrap();
        assert!(active.g2_certified && active.parameter_work > g2.parameter_work);
        assert!(!inspect(&trimmed, Limits { exact_work: active.exact_work - 1, ..limits })
            .unwrap().g2_certified);
        let classified = classified::inspect(
            &trimmed, &vec![classified::Class::G2; trimmed.edges.len()],
            classified::Limits {
                incidence_steps: 10_000,
                exact_work: 2_000_000,
                agreement_work: 1_000_000,
                regularity_cells: 100_000,
            },
        ).unwrap();
        assert!(classified.certified, "{}", classified.reason);
        let mut bad = model.clone();
        bad.edges[0].curve.control_points[1][0] += 0.125;
        if let Ok(bad) = inspect(
            &bad,
            Limits {
                order: 1,
                exact_work: 2000000,
                agreement_work: 1000000,
                regularity_cells: 100000,
            },
        ) {
            assert!(!bad.g1_certified && !bad.g2_certified);
        }
        assert_eq!(model, before);
    }
}

pub mod classified;
