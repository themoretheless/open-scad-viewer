//! Numerical bridge proposals admitted only by BOTH fresh endpoint and path links.
use super::{Boundary, BoundaryLimits, PairDomain, Proof, collar, endpoint, validate_limits};
use crate::{
    Result, check, offset_contact_path as path, offset_contact_pcurve as uv,
    surface::Surface,
    surface_offset::coverage::{self, PathRequest, graph},
};
#[derive(Clone, Copy)]
pub struct Limits {
    pub initial_paths: coverage::Limits,
    pub proposal: path::Limits,
    /// Independent allowances for each of the two final requalification calls.
    pub path_links: graph::Limits,
    pub endpoint_links: super::Limits,
    pub max_attempts: usize,
    pub anchor_refinements: usize,
}
impl Default for Limits {
    fn default() -> Self {
        let initial_paths = coverage::Limits {
            boxes: 256,
            spans_per_bound: 128,
            path_cells: 4096,
        };
        let links = graph::Limits {
            coverage: initial_paths,
            connection_steps: 10000,
            section_queries: 10000,
            spans_per_query: 128,
        };
        Self {
            initial_paths,
            proposal: path::Limits {
                cells: 256,
                spans: 128,
                iterations: 16,
                numerical_tolerance_mm: 1e-9,
                padding_fraction: 0.01,
            },
            path_links: links,
            endpoint_links: links,
            max_attempts: 8,
            anchor_refinements: 8,
        }
    }
}
pub struct Attempt {
    target: usize,
    request: Option<PathRequest>,
    reason: String,
    proposal: Option<path::Report>,
    path_relation: Option<graph::Report>,
    endpoint_relation: Option<super::Report>,
}
impl Attempt {
    pub fn target(&self) -> usize {
        self.target
    }
    pub fn request(&self) -> Option<&PathRequest> {
        self.request.as_ref()
    }
    pub fn reason(&self) -> &str {
        &self.reason
    }
    pub fn proposal(&self) -> Option<&path::Report> {
        self.proposal.as_ref()
    }
    pub fn path_relation(&self) -> Option<&graph::Report> {
        self.path_relation.as_ref()
    }
    pub fn endpoint_relation(&self) -> Option<&super::Report> {
        self.endpoint_relation.as_ref()
    }
    pub fn connected(&self) -> bool {
        self.proposal
            .as_ref()
            .is_some_and(|p| p.continuous_path_proven)
            && self
                .path_relation
                .as_ref()
                .is_some_and(|p| !p.connections.is_empty())
            && self
                .endpoint_relation
                .as_ref()
                .is_some_and(|p| !p.connections.is_empty())
    }
}
pub struct Report {
    endpoint: Proof,
    requests: Vec<PathRequest>,
    limits: Limits,
    pub initial_paths: coverage::Report,
    pub attempts: Vec<Attempt>,
    pub anchor_queries: usize,
    pub reason: &'static str,
}
impl Report {
    pub fn endpoint(&self) -> &Proof {
        &self.endpoint
    }
    pub fn reverify(&self) -> Result<()> {
        let p = &self.endpoint;
        let r = connect(
            p.sources.each_ref(),
            p.distances,
            p.boundary,
            p.tube,
            p.limits,
            &self.requests,
            self.limits,
        )?;
        let signature = |a: &Attempt| a.request.as_ref().map(|r| (r.axis, r.drive, r.seeds));
        check(
            r.reason == self.reason
                && r.attempts.len() == self.attempts.len()
                && r.attempts.iter().zip(&self.attempts).all(|(a, b)| {
                    a.target == b.target
                        && a.reason == b.reason
                        && a.connected() == b.connected()
                        && signature(a) == signature(b)
                }),
            "Endpoint bridge differs from fresh original reconstruction",
        )
    }
}
/// Propose a path from an independently certified inward-collar anchor to the
/// interior of each fresh input path. No caller supplies a bridge or anchor.
/// Every bridge must pass full adaptive path/join proof, a fresh shared-root
/// graph link to its target AND a fresh shared-root link to the endpoint collar.
/// Merely reaching a numerical endpoint or the correct world position is not
/// enough. Failure preserves diagnostics and never supplies missing coverage.
///
/// Initial replay has its own budget. EACH bounded attempt has independent
/// proposal, path-link and endpoint-link allowances. This is not one global
/// allowance silently reused by several proof calls. No material/embedding or
/// complete original-root-set claim follows from these connected subsets.
#[allow(clippy::too_many_arguments)]
pub fn connect(
    surfaces: [&Surface; 2],
    distances: [f64; 2],
    boundary: Boundary,
    tube: PairDomain,
    boundary_limits: BoundaryLimits,
    requests: &[PathRequest],
    limits: Limits,
) -> Result<Report> {
    validate_limits(limits.path_links)?;
    validate_limits(limits.endpoint_links)?;
    let p = limits.proposal;
    check(
        (1..=32).contains(&limits.max_attempts)
            && limits.anchor_refinements <= 16
            && (1..=100000).contains(&p.cells)
            && (1..=100000).contains(&p.spans)
            && (1..=32).contains(&p.iterations)
            && p.numerical_tolerance_mm.is_finite()
            && p.numerical_tolerance_mm > 0.
            && p.padding_fraction.is_finite()
            && p.padding_fraction > 0.
            && p.padding_fraction <= 0.25
            && limits.path_links.coverage.path_cells >= 2,
        "Choose bounded bridge attempts, proposals and anchor work",
    )?;
    let proof = endpoint(surfaces, distances, boundary, tube, boundary_limits)?;
    let initial_paths = coverage::certify(surfaces, distances, requests, limits.initial_paths)?;
    let mut out = Report {
        endpoint: proof,
        requests: requests.to_vec(),
        limits,
        initial_paths,
        attempts: vec![],
        anchor_queries: 0,
        reason: "endpoint-inward-branch-unresolved",
    };
    if !out.endpoint.regular_inward_branch_proven() {
        return Ok(out);
    }
    let c = collar(&out.endpoint);
    let owner = boundary.source;
    let other = 1 - owner;
    let drive = c[owner][boundary.axis];
    let fixed = drive[0] * 0.5 + drive[1] * 0.5;
    let (anchor, queries) = uv::section(
        [surfaces[owner], surfaces[other]],
        [distances[owner], distances[other]],
        boundary.axis,
        fixed,
        c[owner][1 - boundary.axis],
        c[other],
        limits.proposal.spans,
        limits.anchor_refinements,
    )?;
    out.anchor_queries = queries;
    let Some(w) = anchor else {
        out.reason = "inward-anchor-unresolved";
        return Ok(out);
    };
    let mut anchor = [w.first_uv, w.second_uv].map(|s| s.map(|r| r[0] * 0.5 + r[1] * 0.5));
    if owner == 1 {
        anchor.swap(0, 1);
    }
    for (target, (request, path)) in requests.iter().zip(&out.initial_paths.paths).enumerate() {
        if out.attempts.len() == limits.max_attempts {
            break;
        }
        let mut a = Attempt {
            target,
            request: None,
            reason: "target-path-unresolved".into(),
            proposal: None,
            path_relation: None,
            endpoint_relation: None,
        };
        if !path.continuous_path_proven {
            out.attempts.push(a);
            continue;
        }
        let axis = request.axis;
        let first = request.drive[0] * 0.5 + request.drive[1] * 0.5;
        let last = anchor[0][axis];
        if first == last {
            a.reason = "bridge-drive-collapsed".into();
            out.attempts.push(a);
            continue;
        }
        let mut seed = request.seeds[0];
        for s in 0..2 {
            for k in 0..2 {
                seed[s][k] = request.seeds[0][s][k] * 0.5 + request.seeds[1][s][k] * 0.5;
            }
        }
        seed[0][axis] = first;
        let (drive, seeds) = if first < last {
            ([first, last], [seed, anchor])
        } else {
            ([last, first], [anchor, seed])
        };
        let proposed = PathRequest {
            axis,
            drive,
            seeds,
            limits: limits.proposal,
        };
        a.request = Some(proposed);
        match path::certify(surfaces, distances, axis, drive, seeds, limits.proposal) {
            Err(e) => {
                a.reason = e.to_string();
                out.attempts.push(a);
                continue;
            }
            Ok(p) => {
                let valid = p.continuous_path_proven;
                a.proposal = Some(p);
                if !valid {
                    a.reason = "bridge-path-unresolved".into();
                    out.attempts.push(a);
                    continue;
                }
            }
        }
        let paths = graph::certify(
            surfaces,
            distances,
            &[*request, proposed],
            limits.path_links,
        )?;
        let target_connected = !paths.connections.is_empty();
        a.path_relation = Some(paths);
        if !target_connected {
            a.reason = "target-shared-root-unresolved".into();
            out.attempts.push(a);
            continue;
        }
        let ends = super::certify(
            surfaces,
            distances,
            boundary,
            tube,
            boundary_limits,
            &[proposed],
            limits.endpoint_links,
        )?;
        let connected = !ends.connections.is_empty();
        a.endpoint_relation = Some(ends);
        a.reason = if connected {
            "endpoint-and-target-shared-roots-proven"
        } else {
            "endpoint-shared-root-unresolved"
        }
        .into();
        out.attempts.push(a);
    }
    out.reason = "attempts-complete";
    Ok(out)
}
