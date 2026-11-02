//! Fresh root-identity links between a boundary collar and interior paths.
use super::{Boundary, Limits as BoundaryLimits, Proof, certify as endpoint, domains};
use crate::{
    Result, check,
    surface::Surface,
    surface_contact::Witness,
    surface_offset::{
        PairDomain,
        coverage::{self, PathRequest, graph},
    },
};
pub use graph::Limits;
pub mod bridge;
pub struct Connection {
    pub path: usize,
    pub segment: usize,
    pub witness: Witness,
}
pub struct Report {
    pub endpoint: Proof,
    pub coverage: coverage::Report,
    pub connections: Vec<Connection>,
    /// Entire requested paths whose relation remains unresolved.
    pub unresolved_paths: Vec<usize>,
    /// No shared root with this collar. NOT disjoint global components: an
    /// unsearched middle arc may still connect the endpoint and distant path.
    pub disjoint_from_collar: Vec<usize>,
    pub connection_steps: usize,
    pub section_queries: usize,
}
fn collar(p: &Proof) -> PairDomain {
    let mut d = p.tube;
    let b = p.boundary;
    let natural = domains(&p.sources[b.source]);
    let r = natural[b.axis];
    let w = (r[1] - r[0]) * p.limits.inward_fraction;
    d[b.source][b.axis] = if b.high {
        [r[1] - w, r[1]]
    } else {
        [r[0], r[0] + w]
    };
    d
}
/// Reprove the original endpoint, its regular inward branch, and every requested
/// path. Link ONLY with a fresh common-root witness inside BOTH uniqueness tubes.
/// Complete path continuity then connects that witness to the whole path, while
/// the inward branch connects it to the literal endpoint. No world proximity,
/// endpoint samples or preexisting caller-positive reports are authority.
///
/// This proves connected subsets of the ORIGINAL four-parameter contact set.
/// Missing arcs, full-root coverage, world embedding, trims and material roles
/// remain independent; even disjoint collar/path sets may connect elsewhere.
#[allow(clippy::too_many_arguments)]
pub fn certify(
    surfaces: [&Surface; 2],
    distances: [f64; 2],
    boundary: Boundary,
    tube: PairDomain,
    boundary_limits: BoundaryLimits,
    requests: &[PathRequest],
    limits: Limits,
) -> Result<Report> {
    validate_limits(limits)?;
    let proof = endpoint(surfaces, distances, boundary, tube, boundary_limits)?;
    let coverage = coverage::certify(surfaces, distances, requests, limits.coverage)?;
    qualify(surfaces, distances, proof, coverage, requests, limits)
}
pub(super) fn validate_limits(limits: Limits) -> Result<()> {
    check(
        (1..=100000).contains(&limits.connection_steps)
            && (1..=1000000).contains(&limits.section_queries)
            && (1..=100000).contains(&limits.spans_per_query),
        "Choose bounded endpoint/path relation work",
    )?;
    check(
        (1..=100000).contains(&limits.coverage.boxes)
            && (1..=100000).contains(&limits.coverage.spans_per_bound)
            && (1..=1000000).contains(&limits.coverage.path_cells),
        "Choose bounded path replay work",
    )
}
fn qualify(
    surfaces: [&Surface; 2],
    distances: [f64; 2],
    proof: Proof,
    coverage: coverage::Report,
    requests: &[PathRequest],
    limits: Limits,
) -> Result<Report> {
    let mut work = graph::Work {
        steps: 0,
        sections: 0,
    };
    let mut connections = vec![];
    let mut unresolved_paths = vec![];
    let mut disjoint_from_collar = vec![];
    for (index, (request, path)) in requests.iter().zip(&coverage.paths).enumerate() {
        if !proof.regular_inward_branch_proven() || !path.continuous_path_proven {
            unresolved_paths.push(index);
            continue;
        }
        let mut unresolved = false;
        let mut joined = false;
        for (segment, cell) in path.cells.iter().enumerate() {
            if work.steps == limits.connection_steps {
                unresolved = true;
                break;
            }
            work.steps += 1;
            let Some(overlap) = graph::overlap(collar(&proof), graph::tube(request.axis, cell))
            else {
                continue;
            };
            match graph::common(surfaces, distances, overlap, limits, &mut work)? {
                graph::Relation::Common(witness) => {
                    connections.push(Connection {
                        path: index,
                        segment,
                        witness,
                    });
                    joined = true;
                    break;
                }
                graph::Relation::Disjoint => {}
                graph::Relation::Unresolved => {
                    unresolved = true;
                }
            }
        }
        if !joined {
            if unresolved {
                unresolved_paths.push(index);
            } else {
                disjoint_from_collar.push(index);
            }
        }
    }
    Ok(Report {
        endpoint: proof,
        coverage,
        connections,
        unresolved_paths,
        disjoint_from_collar,
        connection_steps: work.steps,
        section_queries: work.sections,
    })
}
