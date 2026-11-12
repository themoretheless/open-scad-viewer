//! All eight natural chart faces searched without caller root seeds.
use super::{Boundary, Limits, Proof, certify, domains, validate};
use crate::{
    Result, check, offset_contact_predictor as predictor,
    surface::Surface,
    surface_contact::Verdict,
    surface_offset::{self, Candidates},
};
use std::collections::VecDeque;
#[derive(Clone, Copy)]
pub struct DiscoveryLimits {
    /// Explicit per-face allowance; total candidate visits <= 8 times this.
    pub boxes_per_boundary: usize,
    /// Independent final full-face replay budget, per natural boundary.
    pub qualification_boxes_per_boundary: usize,
    pub parameter_tolerance: f64,
    /// Shared across all faces, including every smaller-padding retry.
    pub predictor_queries: usize,
    pub max_endpoints: usize,
    pub iterations: usize,
    pub numerical_tolerance_mm: f64,
    pub padding_fraction: f64,
    pub proof: Limits,
}
impl Default for DiscoveryLimits {
    fn default() -> Self {
        Self {
            boxes_per_boundary: 4096,
            qualification_boxes_per_boundary: 20000,
            parameter_tolerance: 0.125,
            predictor_queries: 128,
            max_endpoints: 32,
            iterations: 16,
            numerical_tolerance_mm: 1e-9,
            padding_fraction: 0.1,
            proof: Limits::default(),
        }
    }
}
pub struct FaceSearch {
    pub boundary: Boundary,
    pub candidates: Candidates,
    pub coverage: super::FaceCoverage,
}
impl FaceSearch {
    /// Complete enclosure separation of this face only, not a sampled absence.
    pub fn absence_proven(&self) -> bool {
        self.coverage.absence_proven
    }
}
pub struct DiscoveryReport {
    pub faces: Vec<FaceSearch>,
    pub endpoints: Vec<Proof>,
    pub predictor_queries: usize,
    /// Each qualification uses one section and at most one inward band proof.
    pub qualifications: usize,
    pub numerical_failures: Vec<String>,
}
impl DiscoveryReport {
    /// Boundary root-set coverage only. No component count or interior branch
    /// completeness follows, and duplicate endpoint witnesses remain possible.
    pub fn boundary_coverage_proven(&self) -> bool {
        self.faces.len() == 8 && self.faces.iter().all(|f| f.coverage.root_coverage_proven)
    }
}
/// Search all natural boundaries. Numerical candidates propose original tubes;
/// only fresh section/inward proofs admit endpoints. Candidates and pending
/// remain explicit even after successful discovery. A separate full-face walk
/// excludes every box or assigns it wholly to a fresh unique endpoint tube.
/// Endpoints may overlap: full boundary root coverage is not a component count,
/// interior branch completeness or a globally trimmed material/solid receipt.
pub fn discover(
    surfaces: [&Surface; 2],
    distances: [f64; 2],
    limits: DiscoveryLimits,
) -> Result<DiscoveryReport> {
    validate(surfaces, distances, limits.proof)?;
    check(
        (1..=100000).contains(&limits.boxes_per_boundary)
            && (1..=100000).contains(&limits.qualification_boxes_per_boundary)
            && limits.parameter_tolerance.is_finite()
            && limits.parameter_tolerance > 0.
            && limits.parameter_tolerance <= 1.
            && (1..=4096).contains(&limits.predictor_queries)
            && (1..=256).contains(&limits.max_endpoints)
            && (1..=32).contains(&limits.iterations)
            && limits.numerical_tolerance_mm.is_finite()
            && limits.numerical_tolerance_mm > 0.
            && limits.padding_fraction.is_finite()
            && limits.padding_fraction > 0.
            && limits.padding_fraction <= 0.25,
        "Choose bounded boundary discovery and positive proposal tolerances",
    )?;
    let natural = surfaces.map(domains);
    let mut out = DiscoveryReport {
        faces: vec![],
        endpoints: vec![],
        predictor_queries: 0,
        qualifications: 0,
        numerical_failures: vec![],
    };
    // First collect every face, so an early successful root/budget exhaustion
    // cannot make later unsearched natural boundaries disappear from the report.
    for source in 0..2 {
        for axis in 0..2 {
            for high in [false, true] {
                let boundary = Boundary { source, axis, high };
                let mut d = natural;
                d[source][axis] = [natural[source][axis][usize::from(high)]; 2];
                let candidates = surface_offset::intersection_candidates(
                    surfaces,
                    d,
                    distances,
                    limits.parameter_tolerance,
                    limits.boxes_per_boundary,
                    limits.proof.spans,
                )?;
                out.faces.push(FaceSearch {
                    boundary,
                    candidates,
                    coverage: super::FaceCoverage::empty(),
                });
            }
        }
    }
    let mut seeds = VecDeque::new();
    // Round-robin faces prevents one face's many possible boxes from hiding
    // every other boundary under a small global proposal budget.
    let max = out
        .faces
        .iter()
        .map(|f| f.candidates.boxes.len() + f.candidates.pending.len())
        .max()
        .unwrap_or(0);
    for n in 0..max {
        for f in &out.faces {
            if let Some(cell) = f
                .candidates
                .boxes
                .iter()
                .chain(&f.candidates.pending)
                .nth(n)
            {
                seeds.push_back((f.boundary, *cell));
            }
        }
    }
    'search: while let Some((b, cell)) = seeds.pop_front() {
        if out.endpoints.len() == limits.max_endpoints {
            break;
        }
        let owner = b.source;
        let other = 1 - owner;
        let ordered = [surfaces[owner], surfaces[other]];
        let d = [distances[owner], distances[other]];
        let seed = [cell[owner], cell[other]].map(|s| s.map(|r| r[0] * 0.5 + r[1] * 0.5));
        let mut padding = limits.padding_fraction;
        for _ in 0..=8 {
            if out.predictor_queries == limits.predictor_queries {
                break 'search;
            }
            out.predictor_queries += 1;
            let root = match predictor::propose(
                crate::offset::contact_pcurve::OffsetFaces {
                    surfaces: ordered,
                    distances: d,
                },
                seed,
                b.axis,
                limits.numerical_tolerance_mm,
                limits.iterations,
                padding,
                limits.proof.spans,
            ) {
                Ok(r) => r,
                Err(e) => {
                    out.numerical_failures.push(e.to_string());
                    break;
                }
            };
            if root.residual_mm > limits.numerical_tolerance_mm {
                break;
            }
            if matches!(root.certificate, Some(Verdict::Witness(_))) {
                let mut tube = natural;
                let uv = [root.first_uv, root.second_uv];
                for (s, original) in [owner, other].into_iter().enumerate() {
                    for k in 0..2 {
                        if original == owner && k == b.axis {
                            tube[original][k] = [natural[original][k][usize::from(b.high)]; 2];
                            continue;
                        }
                        let range = natural[original][k];
                        let pad = (range[1] - range[0]) * padding;
                        tube[original][k] = [
                            (uv[s][k] - pad).max(range[0]),
                            (uv[s][k] + pad).min(range[1]),
                        ];
                    }
                }
                out.qualifications += 1;
                let proof = certify(surfaces, distances, b, tube, limits.proof)?;
                if proof.witness().is_some() {
                    out.endpoints.push(proof);
                }
                break;
            }
            if !matches!(root.certificate, Some(Verdict::Unresolved)) {
                break;
            }
            padding *= 0.5;
        }
    }
    for face in &mut out.faces {
        face.coverage = super::qualification::certify(
            surfaces,
            distances,
            face.boundary,
            &out.endpoints,
            limits.qualification_boxes_per_boundary,
            limits.proof.spans,
        )?;
    }
    Ok(out)
}
