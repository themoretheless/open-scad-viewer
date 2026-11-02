//! Certified connectivity of the original four-parameter offset-root set.
use super::{Limits as CoverageLimits, PathRequest, Report as Coverage, certify as cover};
use crate::{
    Result, check, surface::Surface, surface_contact::Witness, surface_offset::PairDomain,
};
use std::collections::BTreeMap;
mod relations;
pub(crate) use relations::{Relation, Work, common, overlap};

#[derive(Clone, Copy)]
pub struct Limits {
    pub coverage: CoverageLimits,
    /// Shared segment-pair tests AND subdivision-box visits, across all pairs.
    pub connection_steps: usize,
    pub section_queries: usize,
    pub spans_per_query: usize,
}
pub struct Connection {
    pub paths: [usize; 2],
    pub segments: [usize; 2],
    /// Fresh common-root evidence inside both original uniqueness tubes.
    pub witness: Witness,
}
pub struct Report {
    pub coverage: Coverage,
    pub connections: Vec<Connection>,
    /// Proved connected subsets. Distinct groups may still merge if unresolved.
    pub connected_groups: Vec<Vec<usize>>,
    /// Full path-pair relationships not resolved by the bounded original proof.
    pub unresolved_pairs: Vec<[usize; 2]>,
    pub connection_steps: usize,
    pub section_queries: usize,
    /// Connected-component count of the PARAMETER root set, only if complete.
    /// This is not a count of world curves on overlapping/self-mapped sources.
    pub component_count: Option<usize>,
}
fn root(parent: &[usize], mut i: usize) -> usize {
    while parent[i] != i {
        i = parent[i];
    }
    i
}
pub(crate) fn tube(axis: usize, c: &crate::offset_contact_path::Cell) -> PairDomain {
    let mut first = [[0.; 2]; 2];
    first[axis] = c.drive;
    first[1 - axis] = c.first_other;
    [first, c.second]
}
/// Freshly prove root coverage and path connectivity, without caller-positive
/// reports. An edge requires a certified root in BOTH segment tubes. No distance
/// welding, sampled endpoint coincidence or section-only exclusion is accepted.
/// Disjointness requires complete tube-product exclusion (or disjoint parameter
/// boxes); all budget/precision ambiguity stays associated with the whole pair.
///
/// Each admitted path is a compact connected graph over a closed driving
/// interval. Finite groups joined by common-root witnesses are connected. Only
/// full root coverage AND disjointness of all remaining groups justify a count
/// of connected components of the original four-parameter root set. World
/// embedding, material trims, branch valence and BRep authority are separate.
pub fn certify(
    surfaces: [&Surface; 2],
    distances: [f64; 2],
    requests: &[PathRequest],
    limits: Limits,
) -> Result<Report> {
    check(
        (1..=100000).contains(&limits.connection_steps)
            && (1..=1000000).contains(&limits.section_queries)
            && (1..=100000).contains(&limits.spans_per_query),
        "Choose bounded connectivity proof work",
    )?;
    let coverage = cover(surfaces, distances, requests, limits.coverage)?;
    let nodes: Vec<_> = coverage
        .paths
        .iter()
        .enumerate()
        .filter_map(|(i, p)| p.continuous_path_proven.then_some(i))
        .collect();
    let mut parent: Vec<_> = (0..requests.len()).collect();
    let mut work = Work {
        steps: 0,
        sections: 0,
    };
    let mut connections = vec![];
    let mut unresolved_pairs = vec![];
    for (n, &first) in nodes.iter().enumerate() {
        for &second in &nodes[n + 1..] {
            if root(&parent, first) == root(&parent, second) {
                continue;
            }
            let mut unresolved = false;
            let mut joined = false;
            'segments: for (a, ca) in coverage.paths[first].cells.iter().enumerate() {
                for (b, cb) in coverage.paths[second].cells.iter().enumerate() {
                    if work.steps == limits.connection_steps {
                        unresolved = true;
                        break 'segments;
                    }
                    work.steps += 1;
                    let Some(intersection) = overlap(
                        tube(requests[first].axis, ca),
                        tube(requests[second].axis, cb),
                    ) else {
                        continue;
                    };
                    match common(surfaces, distances, intersection, limits, &mut work)? {
                        Relation::Common(w) => {
                            let r = root(&parent, second);
                            parent[r] = root(&parent, first);
                            connections.push(Connection {
                                paths: [first, second],
                                segments: [a, b],
                                witness: w,
                            });
                            joined = true;
                            break 'segments;
                        }
                        Relation::Disjoint => {}
                        Relation::Unresolved => {
                            unresolved = true;
                        }
                    }
                }
            }
            if !joined && unresolved {
                unresolved_pairs.push([first, second]);
            }
        }
    }
    unresolved_pairs.retain(|p| root(&parent, p[0]) != root(&parent, p[1]));
    let mut groups = BTreeMap::<usize, Vec<usize>>::new();
    for &i in &nodes {
        groups.entry(root(&parent, i)).or_default().push(i);
    }
    let connected_groups: Vec<_> = groups.into_values().collect();
    let component_count = (coverage.root_coverage_proven && unresolved_pairs.is_empty())
        .then_some(connected_groups.len());
    Ok(Report {
        coverage,
        connections,
        connected_groups,
        unresolved_pairs,
        connection_steps: work.steps,
        section_queries: work.sections,
        component_count,
    })
}
