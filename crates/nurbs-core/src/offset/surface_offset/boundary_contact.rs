//! Isolated contact roots on literal natural chart boundaries, with an inward
//! original-domain branch. A boundary is fixed by ownership, not a nearby sample.
use super::{PairDomain, coverage};
use crate::{
    Result, check, offset_contact_tangent as tangent,
    surface::Surface,
    surface_contact::{Verdict, Witness},
};
mod search;
pub use search::{DiscoveryLimits, DiscoveryReport, FaceSearch, discover};
mod qualification;
pub use qualification::{FaceCoverage, OwnedBox};
pub mod attachment;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Boundary {
    pub source: usize,
    pub axis: usize,
    pub high: bool,
}
#[derive(Clone, Copy)]
pub struct Limits {
    pub spans: usize,
    /// Fraction of the selected natural coordinate width, inside its boundary.
    pub inward_fraction: f64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            spans: 128,
            inward_fraction: 0.01,
        }
    }
}
pub struct Proof {
    sources: [Surface; 2],
    distances: [f64; 2],
    boundary: Boundary,
    tube: PairDomain,
    limits: Limits,
    verdict: Verdict,
    inward: Option<tangent::Report>,
}
impl Proof {
    /// Propose and independently prove bridges from this saved original endpoint
    /// to input paths. The saved positive result is never reused as authority.
    pub fn bridge_paths(
        &self,
        requests: &[coverage::PathRequest],
        limits: attachment::bridge::Limits,
    ) -> Result<attachment::bridge::Report> {
        attachment::bridge::connect(
            self.sources.each_ref(),
            self.distances,
            self.boundary,
            self.tube,
            self.limits,
            requests,
            limits,
        )
    }
    pub fn boundary(&self) -> Boundary {
        self.boundary
    }
    pub fn original_tube(&self) -> PairDomain {
        self.tube
    }
    pub fn verdict(&self) -> &Verdict {
        &self.verdict
    }
    /// Original source order even when the second source owns the boundary.
    pub fn witness(&self) -> Option<&Witness> {
        if let Verdict::Witness(w) = &self.verdict {
            Some(w)
        } else {
            None
        }
    }
    /// World tangent is source-order independent. The report's parameter
    /// derivatives/contact UVs use boundary owner first, other source second.
    pub fn inward(&self) -> Option<&tangent::Report> {
        self.inward.as_ref()
    }
    pub fn regular_inward_branch_proven(&self) -> bool {
        self.witness().is_some() && self.inward.as_ref().is_some_and(|r| r.regular)
    }
    pub fn reverify(&self) -> Result<()> {
        let r = certify(
            self.sources.each_ref(),
            self.distances,
            self.boundary,
            self.tube,
            self.limits,
        )?;
        let same = match (&r.verdict, &self.verdict) {
            (Verdict::Excluded, Verdict::Excluded) | (Verdict::Unresolved, Verdict::Unresolved) => {
                true
            }
            (Verdict::Witness(a), Verdict::Witness(b)) => {
                a.first_uv == b.first_uv
                    && a.second_uv == b.second_uv
                    && a.point == b.point
                    && a.contraction_upper == b.contraction_upper
            }
            _ => false,
        };
        check(
            same && r.regular_inward_branch_proven() == self.regular_inward_branch_proven(),
            "Boundary root differs from original replay",
        )
    }
}
pub(super) fn domains(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
pub(super) fn validate(surfaces: [&Surface; 2], distances: [f64; 2], limits: Limits) -> Result<()> {
    check(
        (1..=100000).contains(&limits.spans)
            && limits.inward_fraction.is_finite()
            && limits.inward_fraction > 0.
            && limits.inward_fraction <= 0.25,
        "Choose bounded spans and a positive inward fraction up to one quarter",
    )?;
    // Reuse the original continuity/nonperiodic/finite chart validation; a
    // single preflight box cannot authorize positive coverage or hide bad input.
    coverage::certify(
        surfaces,
        distances,
        &[],
        coverage::Limits {
            boxes: 1,
            spans_per_bound: limits.spans,
            path_cells: 1,
        },
    )?;
    Ok(())
}
/// Certify one isolated root at an EXACT natural chart boundary. All three
/// other coordinates must have positive original-domain tube widths. Swapping
/// source order makes the boundary coordinate the independent parameter; source
/// UVs are restored before publication. No support extension is evaluated.
///
/// A second independent full-band proof on a one-sided inward collar shows
/// regular continuation up to that endpoint. The original section and collar
/// use the SAME uniqueness tube, so they cannot silently select different roots.
/// This does not prove endpoint completeness, edge/corner contacts, material
/// selection, a globally trimmed path, BRep closure or solid embedding.
pub fn certify(
    surfaces: [&Surface; 2],
    distances: [f64; 2],
    boundary: Boundary,
    mut tube: PairDomain,
    limits: Limits,
) -> Result<Proof> {
    validate(surfaces, distances, limits)?;
    check(
        boundary.source < 2 && boundary.axis < 2,
        "Invalid natural boundary owner",
    )?;
    let natural = surfaces.map(domains);
    for s in 0..2 {
        for k in 0..2 {
            if s == boundary.source && k == boundary.axis {
                continue;
            }
            let r = tube[s][k];
            check(
                r.iter().all(|x| x.is_finite())
                    && r[0] < r[1]
                    && r[0] >= natural[s][k][0]
                    && r[1] <= natural[s][k][1],
                "Boundary root tubes must stay on the original charts",
            )?;
        }
    }
    let owner = boundary.source;
    let axis = boundary.axis;
    let coordinate = natural[owner][axis][usize::from(boundary.high)];
    // Caller input must identify the literal boundary, not be silently snapped.
    check(
        tube[owner][axis] == [coordinate; 2],
        "Selected coordinate must equal the literal natural boundary",
    )?;
    tube[owner][axis] = [coordinate; 2];
    let other = 1 - owner;
    let ordered = [surfaces[owner], surfaces[other]];
    let d = [distances[owner], distances[other]];
    let free = tube[owner][1 - axis];
    let second = tube[other];
    let mut verdict =
        super::certify_contact_section(ordered, d, axis, coordinate, free, second, limits.spans)?;
    let inward = if matches!(verdict, Verdict::Witness(_)) {
        let range = natural[owner][axis];
        let width = (range[1] - range[0]) * limits.inward_fraction;
        let drive = if boundary.high {
            [coordinate - width, coordinate]
        } else {
            [coordinate, coordinate + width]
        };
        check(
            drive[0] < drive[1] && drive[0] >= range[0] && drive[1] <= range[1],
            "Inward collar is numerically unrepresentable",
        )?;
        Some(tangent::certify(
            ordered,
            d,
            axis,
            drive,
            free,
            second,
            limits.spans,
        )?)
    } else {
        None
    };
    if owner == 1 {
        if let Verdict::Witness(w) = &mut verdict {
            std::mem::swap(&mut w.first_uv, &mut w.second_uv);
        }
    }
    Ok(Proof {
        sources: surfaces.map(Clone::clone),
        distances,
        boundary,
        tube,
        limits,
        verdict,
        inward,
    })
}
