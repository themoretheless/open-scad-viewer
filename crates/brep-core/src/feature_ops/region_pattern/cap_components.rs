//! Whole rational-loop separation and interval winding establish the nesting
//! forest. Area sign only orients the resulting disjoint material components.
use super::*;
use nurbs_core::{
    interval_eval::{self, Interval},
    trim_domain::{Location, TrimDomain},
};
pub(super) struct Component {
    pub members: Vec<usize>,
    pub reversed: bool,
}
pub(super) fn classify(
    source: &Model,
    rings: &[Vec<crate::Coedge>],
    areas: &[[f64; 2]],
    limits: Limits,
    work: &mut ProofWork,
) -> Result<Vec<Component>> {
    let curves = rings
        .iter()
        .map(|r| r.iter().map(|c| c.pcurve.clone()).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    for ring in &curves {
        let remaining = work.remaining(limits)?;
        let proof = nurbs_core::trim_simplicity::inspect(
            ring,
            1e-8,
            remaining.max_pairs,
            remaining.max_cells,
        )?;
        work.cells += proof.cells;
        work.pairs += proof.pairs.len();
        if !proof.proven_simple {
            return Err(error("Original cap rim simplicity remains unproven"));
        }
    }
    for i in 0..rings.len() {
        for j in i + 1..rings.len() {
            for a in &curves[i] {
                for b in &curves[j] {
                    let remaining = work.remaining(limits)?;
                    let proof = nurbs_core::curve_distance::prove_separation(
                        a,
                        b,
                        1e-8,
                        remaining.max_cells.min(4096),
                    )?;
                    work.pairs += 1;
                    work.cells += proof.cells;
                    if proof.distance_interval_mm[0] <= 0. {
                        return Err(error("Original cap rim separation remains unproven"));
                    }
                }
            }
        }
    }
    let domains = curves
        .iter()
        .map(|r| TrimDomain::new(std::slice::from_ref(r), 1e-8))
        .collect::<nurbs_core::Result<Vec<_>>>()?;
    let mut inside = vec![vec![false; rings.len()]; rings.len()];
    for i in 0..rings.len() {
        let c = &curves[i][0];
        // Enclose the ACTUAL natural-domain endpoint; the first pole need not
        // equal that endpoint on an unclamped original rational curve.
        let endpoint = interval_eval::evaluate_interval(c, Interval::point(c.domain()[0]))?;
        let query = std::array::from_fn(|k| [endpoint[k].lo, endpoint[k].hi]);
        for j in 0..rings.len() {
            if i != j {
                let remaining = work.remaining(limits)?;
                let proof = domains[j].classify(query, remaining.max_domain_cells.min(4096))?;
                work.domain_cells += proof.cells;
                inside[i][j] = match proof.location {
                    Location::Inside if matches!(proof.winding, Some(1) | Some(-1)) => true,
                    Location::Outside => false,
                    _ => return Err(error("Original cap nesting remains unproven")),
                };
            }
        }
    }
    let depth = inside
        .iter()
        .map(|r| r.iter().filter(|v| **v).count())
        .collect::<Vec<_>>();
    let mut parent = vec![None; rings.len()];
    for i in 0..rings.len() {
        let containers = (0..rings.len())
            .filter(|j| inside[i][*j])
            .collect::<Vec<_>>();
        for &a in &containers {
            if inside[a][i] || depth[a] >= depth[i] {
                return Err(error("Original cap nesting is inconsistent"));
            }
            for &b in &containers {
                if a != b && !inside[a][b] && !inside[b][a] {
                    return Err(error(
                        "Original cap containers do not form a nesting forest",
                    ));
                }
            }
        }
        parent[i] = containers.into_iter().max_by_key(|j| depth[*j]);
        if let Some(p) = parent[i] {
            if depth[p] + 1 != depth[i] || (areas[p][1] < 0.) == (areas[i][1] < 0.) {
                return Err(error("Original cap nesting/orientation is incompatible"));
            }
        } else if depth[i] != 0 {
            return Err(error("Original cap outer ownership is unresolved"));
        }
    }
    let key = |i: usize| {
        let mut ids = rings[i]
            .iter()
            .map(|c| source.1.edges[c.edge])
            .collect::<Vec<_>>();
        ids.sort();
        ids
    };
    let mut outers = (0..rings.len())
        .filter(|i| depth[*i] % 2 == 0)
        .collect::<Vec<_>>();
    outers.sort_by_key(|i| key(*i));
    let mut out = Vec::new();
    for outer in outers {
        let mut holes = (0..rings.len())
            .filter(|i| parent[*i] == Some(outer))
            .collect::<Vec<_>>();
        holes.sort_by_key(|i| key(*i));
        let mut members = vec![outer];
        members.extend(holes);
        out.push(Component {
            members,
            reversed: areas[outer][1] < 0.,
        });
    }
    Ok(out)
}
