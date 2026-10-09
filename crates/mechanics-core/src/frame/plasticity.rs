use super::*;

/// One plastic hinge in formation order.
#[derive(Clone, Debug, PartialEq)]
pub struct PlasticHinge {
    pub member: usize,
    /// True when the hinge formed at node A (x = 0), false at node B (x = L).
    pub at_node_a: bool,
    /// Cumulative load factor at which the hinge formed.
    pub load_factor: f64,
}

/// How a collapse analysis terminated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollapseStatus {
    /// The last hinge insertion turned the structure into a mechanism.
    Mechanism,
    /// The hinge budget ran out while the structure still carried load.
    HingeLimit,
    /// Every yieldable section is released or carries no moment increment:
    /// the remaining structure carries the reference load elastically.
    ElasticUnlimited,
}

/// Incremental plastic collapse under proportional loading.
#[derive(Clone, Debug)]
pub struct FrameCollapseResponse {
    /// Hinges in formation order; simultaneous hinges share a load factor.
    pub hinges: Vec<PlasticHinge>,
    pub status: CollapseStatus,
    /// Collapse load factor (status `Mechanism`), else None.
    pub collapse_load_factor: Option<f64>,
}

/// Step-by-step plastic hinge analysis: the reference load set scales by a
/// single factor λ. Each step solves the current structure elastically, finds
/// the member end whose accumulated bending moment first reaches its plastic
/// moment `plastic_moments_nmm` (None = elastic member), and inserts a bending
/// hinge there (releases about local y and z; torsion stays). The moment at a
/// hinge stays at Mp, so moments accumulate across steps. The loop ends when
/// the structure becomes a mechanism (collapse), when the hinge budget runs
/// out, or when no yieldable section sees any further moment. Hinges form at
/// member ends only; under distributed loads a span interior can govern, so
/// refine the mesh where a mid-span hinge is expected. Supports must be rigid
/// or bidirectional springs — unilateral contacts are refused. Small
/// displacements, elastic-perfectly-plastic sections; not a certified
/// ultimate-limit-state calculation.
pub fn collapse(
    nodes_mm: &[[f64; 3]],
    members: &[Member],
    restrained: &[[bool; 6]],
    supports: &[Support],
    reference: &LoadSet,
    plastic_moments_nmm: &[Option<f64>],
    max_hinges: usize,
) -> Result<FrameCollapseResponse> {
    if plastic_moments_nmm.len() != members.len() {
        return Err(invalid(
            "Each member requires a plastic moment entry (null for elastic)",
        ));
    }
    if plastic_moments_nmm.iter().all(Option::is_none) {
        return Err(invalid("At least one member requires a plastic moment"));
    }
    if plastic_moments_nmm
        .iter()
        .flatten()
        .any(|m| !m.is_finite() || *m <= 0.)
    {
        return Err(invalid("Plastic moments must be finite and positive"));
    }
    if max_hinges == 0 || max_hinges > MAX_PLASTIC_HINGES {
        return Err(invalid(
            "Collapse analysis admits between 1 and 256 hinge insertions",
        ));
    }
    validate_supports(supports, nodes_mm.len(), restrained)?;
    if supports.iter().any(|s| s.is_unilateral()) {
        return Err(invalid(
            "Collapse analysis admits rigid restraints and bidirectional springs only",
        ));
    }
    let mut work = members.to_vec();
    // Accumulated end moments per member end: [node A, node B], local y/z.
    let mut cum_my = vec![[0f64; 2]; members.len()];
    let mut cum_mz = vec![[0f64; 2]; members.len()];
    let mut hinged = vec![[false; 2]; members.len()];
    let mut lambda = 0f64;
    let mut hinges = Vec::new();
    loop {
        let model = Model {
            nodes_mm: nodes_mm.to_vec(),
            members: work.clone(),
            restrained: restrained.to_vec(),
            supports: supports.to_vec(),
            forces_n: reference.forces_n.clone(),
            moments_nmm: reference.moments_nmm.clone(),
            loads: reference.loads.clone(),
        };
        let response = match solve(&model) {
            Ok(r) => r,
            Err(err) => {
                if hinges.is_empty() {
                    // Unstable or unsound before any yielding: the caller's error.
                    return Err(err);
                }
                return Ok(FrameCollapseResponse {
                    hinges,
                    status: CollapseStatus::Mechanism,
                    collapse_load_factor: Some(lambda),
                });
            }
        };
        let end_station = |m: usize, end: usize| -> Result<&Station> {
            let stations = &response.members[m].stations;
            let s = if end == 0 {
                stations.first()
            } else {
                stations.last()
            };
            s.ok_or_else(numeric)
        };
        // Next yielding end: smallest λ step to bring |M_cum + Δλ·M| to Mp.
        let mut step_best = f64::INFINITY;
        let mut sites: Vec<(usize, usize)> = Vec::new();
        for (m, mp) in plastic_moments_nmm.iter().enumerate() {
            let Some(&mp) = mp.as_ref() else { continue };
            for end in 0..2 {
                if hinged[m][end] {
                    continue;
                }
                let s = end_station(m, end)?;
                let inc = s.moment_y_nmm.hypot(s.moment_z_nmm);
                if !(inc > 0.) {
                    continue;
                }
                let residual = mp - cum_my[m][end].hypot(cum_mz[m][end]);
                if !(residual > 0.) {
                    continue;
                }
                let step = residual / inc;
                if step < step_best * (1. - 1e-6) {
                    step_best = step;
                    sites.clear();
                    sites.push((m, end));
                } else if step <= step_best * (1. + 1e-6) {
                    sites.push((m, end));
                }
            }
        }
        if sites.is_empty() || !step_best.is_finite() {
            if hinges.is_empty() {
                return Err(invalid(
                    "The reference load produces no bending at yieldable sections",
                ));
            }
            return Ok(FrameCollapseResponse {
                hinges,
                status: CollapseStatus::ElasticUnlimited,
                collapse_load_factor: None,
            });
        }
        lambda += step_best;
        for (m, mp) in plastic_moments_nmm.iter().enumerate() {
            if mp.is_none() {
                continue;
            }
            for end in 0..2 {
                let s = end_station(m, end)?;
                cum_my[m][end] += step_best * s.moment_y_nmm;
                cum_mz[m][end] += step_best * s.moment_z_nmm;
            }
        }
        if hinges.len() + sites.len() > max_hinges {
            return Ok(FrameCollapseResponse {
                hinges,
                status: CollapseStatus::HingeLimit,
                collapse_load_factor: None,
            });
        }
        for (m, end) in sites {
            if end == 0 {
                work[m].release_a[1] = true;
                work[m].release_a[2] = true;
            } else {
                work[m].release_b[1] = true;
                work[m].release_b[2] = true;
            }
            hinged[m][end] = true;
            hinges.push(PlasticHinge {
                member: m,
                at_node_a: end == 0,
                load_factor: lambda,
            });
        }
    }
}
