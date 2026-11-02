//! Automatic local transitions feed the unchanged complete parameter-component
//! verifier. A transition forest is never physical branch/cycle authority.
use super::*;
#[derive(Clone, Copy, Debug)]
pub struct MixedComponentDiscoveryLimits {
    pub max_rectangles: usize,
    pub max_pairs: usize,
    pub max_bridges: usize,
    pub transition_proposals: usize,
    pub transition_depth: usize,
}
impl Default for MixedComponentDiscoveryLimits {
    fn default() -> Self {
        Self {
            max_rectangles: 100000,
            max_pairs: 2016,
            max_bridges: 63,
            transition_proposals: 128,
            transition_depth: 12,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct MixedTransitionSearch {
    pub charts: [usize; 2],
    pub reason: String,
    pub attempts: Vec<TransitionAttempt>,
    pub rectangles: usize,
    pub point_domain: Option<Box4>,
    pub point_rectangles: usize,
}
#[derive(Clone, Debug)]
pub struct DiscoveredMixedFiberComponents {
    core: QualifiedFiberComponents,
    limits: MixedComponentDiscoveryLimits,
    searches: Vec<MixedTransitionSearch>,
    rectangles: usize,
}
impl DiscoveredMixedFiberComponents {
    pub fn proof(&self) -> &QualifiedFiberComponents {
        &self.core
    }
    pub fn searches(&self) -> &[MixedTransitionSearch] {
        &self.searches
    }
    pub fn rectangles(&self) -> usize {
        self.rectangles
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = connect_mixed_fiber_cover_automatic(self.core.coverage(), self.limits)?;
        let other = fresh
            .into_components()
            .ok_or_else(|| crate::input("Automatic mixed component census remains unproved"))?;
        check(
            self.searches == other.searches
                && self.rectangles == other.rectangles
                && self.core.specs == other.core.specs
                && self.core.components == other.core.components
                && self.core.separations == other.core.separations
                && self.core.rectangles == other.core.rectangles
                && self.core.max_rectangles == other.core.max_rectangles
                && self.core.transitions.len() == other.core.transitions.len()
                && self.core.points.len() == other.core.points.len()
                && self
                    .core
                    .points
                    .iter()
                    .zip(&other.core.points)
                    .all(|(a, b)| a.matches(b))
                && self
                    .core
                    .transitions
                    .iter()
                    .zip(&other.core.transitions)
                    .all(|(a, b)| a.matches(b)),
            "Automatic mixed component sources, searches, transitions or separation census changed",
        )
    }
}
pub struct MixedComponentDiscoveryResult {
    components: Option<DiscoveredMixedFiberComponents>,
    searches: Vec<MixedTransitionSearch>,
    rectangles: usize,
    reason: &'static str,
}
impl MixedComponentDiscoveryResult {
    pub fn components(&self) -> Option<&DiscoveredMixedFiberComponents> {
        self.components.as_ref()
    }
    pub fn into_components(self) -> Option<DiscoveredMixedFiberComponents> {
        self.components
    }
    pub fn searches(&self) -> &[MixedTransitionSearch] {
        &self.searches
    }
    pub fn rectangles(&self) -> usize {
        self.rectangles
    }
    pub fn reason(&self) -> &'static str {
        self.reason
    }
}
/// Infer original-source bridge boxes between charts in a complete mixed
/// coverage. Only fresh qualified transitions join labels; failed searches do
/// not separate them. The existing full coverage/ALL cross-component root-box
/// separation verifier must then independently admit the resulting components.
/// Half the shared allowance is reserved for that final proof. Coverage input
/// remains sealed source evidence, never an externally authored positive flag.
pub fn connect_mixed_fiber_cover_automatic(
    coverage: &QualifiedMixedFiberCoverage,
    limits: MixedComponentDiscoveryLimits,
) -> Result<MixedComponentDiscoveryResult> {
    check(
        (1..=100000).contains(&limits.max_rectangles)
            && limits.max_pairs <= 2016
            && limits.max_bridges <= 256
            && (1..=1024).contains(&limits.transition_proposals)
            && limits.transition_depth <= 24
            && coverage.fibers().len() <= 64,
        "Need bounded automatic mixed component work",
    )?;
    let fs = coverage.fibers();
    let boxes = fs
        .iter()
        .map(root_box)
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| crate::input("Missing original chart root enclosure"))?;
    let mut candidates = vec![];
    for i in 0..fs.len() {
        for j in i + 1..fs.len() {
            if !(0..4).any(|k| boxes[i][k][1] < boxes[j][k][0] || boxes[j][k][1] < boxes[i][k][0]) {
                candidates.push([i, j]);
            }
        }
    }
    // Proximity only orders proposals; every accepted connection still needs
    // its original proof. Try nearby root enclosures before distant box contacts.
    let distance = |pair: &[usize; 2]| {
        let total = (0..4)
            .map(|k| {
                let center = |i: usize| 0.5 * boxes[i][k][0] + 0.5 * boxes[i][k][1];
                let d = (center(pair[0]) - center(pair[1]))
                    / (coverage.domain()[k][1] - coverage.domain()[k][0]);
                d * d
            })
            .sum::<f64>();
        if total.is_finite() {
            total
        } else {
            f64::INFINITY
        }
    };
    candidates.sort_by(|a, b| distance(a).total_cmp(&distance(b)).then_with(|| a.cmp(b)));
    let mut labels = (0..fs.len()).collect::<Vec<_>>();
    let mut specs = vec![];
    let mut out = MixedComponentDiscoveryResult {
        components: None,
        searches: vec![],
        rectangles: 0,
        reason: "automatic-transitions-unproven",
    };
    let proposal_cap = limits.max_rectangles / 2;
    loop {
        candidates.retain(|p| labels[p[0]] != labels[p[1]]);
        if candidates.is_empty() {
            break;
        }
        let before = specs.len();
        let mut failed = vec![];
        let mut stopped = false;
        for (index, &[i, j]) in candidates.iter().enumerate() {
            if labels[i] == labels[j] {
                continue;
            }
            if out.searches.len() == limits.max_pairs || specs.len() == limits.max_bridges {
                stopped = true;
                break;
            }
            let allowance = (proposal_cap - out.rectangles) / (candidates.len() - index);
            if allowance == 0 {
                stopped = true;
                break;
            }
            let r = discover_fiber_transition(
                &fs[i],
                &fs[j],
                FiberTransitionDiscoveryLimits {
                    // Component connectivity can use an original singleton
                    // root when an oriented interval is hard or unavailable.
                    // Reserve that fallback BEFORE spending the pair budget.
                    max_rectangles: allowance - allowance / 3,
                    max_proposals: limits.transition_proposals,
                    max_depth: limits.transition_depth,
                },
            )?;
            let mut search = MixedTransitionSearch {
                charts: [i, j],
                reason: r.reason().to_owned(),
                attempts: r.attempts().to_vec(),
                rectangles: r.rectangles(),
                point_domain: None,
                point_rectangles: 0,
            };
            let domain = if let Some(found) = r.into_transition() {
                Some(found.proof().bridge().domain())
            } else if search.rectangles < allowance {
                // Closed atlas leaves may only touch at one original root. A
                // singleton certificate can connect PARAMETER components but is
                // retained separately and never supplies transition orientation.
                let remaining = allowance - search.rectangles;
                let mut work = Work {
                    used: 0,
                    max: remaining,
                };
                let point = super::super::discovered_components::point_connection(
                    fs[i].source(),
                    &fs[i],
                    &fs[j],
                    &mut work,
                );
                let point = match point {
                    Ok(point) => point,
                    Err(_) => {
                        work.used = remaining;
                        None
                    }
                };
                search.point_rectangles = work.used;
                search.rectangles += work.used;
                if let Some(point) = point {
                    search.reason = "qualified-original-point-connection".into();
                    search.point_domain = Some(point.domain());
                    Some(point.domain())
                } else {
                    search.reason.push_str(";point-connection-unproven");
                    None
                }
            } else {
                None
            };
            out.rectangles += search.rectangles;
            out.searches.push(search);
            if let Some(domain) = domain {
                specs.push(FiberBridgeSpec {
                    charts: [i, j],
                    domain,
                });
                let low = labels[i].min(labels[j]);
                let high = labels[i].max(labels[j]);
                for label in &mut labels {
                    if *label == high {
                        *label = low;
                    }
                }
            } else {
                failed.push([i, j]);
            }
        }
        // Later successful merges can leave only a few unresolved links. Reoffer
        // the remaining ORIGINAL allowance to these pairs instead of discarding
        // unused work. Every retry is retained and charged; no limit is raised.
        if stopped || specs.len() == before {
            break;
        }
        candidates = failed;
    }
    let r = connect_mixed_fiber_cover(coverage, &specs, limits.max_rectangles - out.rectangles)?;
    out.rectangles += r.rectangles();
    out.reason = r.reason();
    if let Some(core) = r.into_components() {
        out.reason = "complete-discovered-mixed-parameter-components";
        out.components = Some(DiscoveredMixedFiberComponents {
            core,
            limits,
            searches: out.searches.clone(),
            rectangles: out.rectangles,
        });
    }
    Ok(out)
}
