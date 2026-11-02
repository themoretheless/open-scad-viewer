//! Automatic local overlap proposals. A failed search never proves separation,
//! and a cycle of successful transitions never proves physical branch topology.
use super::*;
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug)]
pub struct FiberTransitionDiscoveryLimits {
    pub max_rectangles: usize,
    pub max_proposals: usize,
    pub max_depth: usize,
}
impl Default for FiberTransitionDiscoveryLimits {
    fn default() -> Self {
        Self {
            max_rectangles: 100000,
            max_proposals: 128,
            max_depth: 12,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct TransitionAttempt {
    pub domain: Box4,
    pub bridge_domain: Box4,
    pub reason: String,
    pub rectangles: usize,
}
#[derive(Clone, Debug)]
pub struct DiscoveredFiberTransition {
    proof: QualifiedFiberTransition,
    limits: FiberTransitionDiscoveryLimits,
    attempts: Vec<TransitionAttempt>,
    rectangles: usize,
}
impl DiscoveredFiberTransition {
    pub fn proof(&self) -> &QualifiedFiberTransition {
        &self.proof
    }
    pub fn attempts(&self) -> &[TransitionAttempt] {
        &self.attempts
    }
    pub fn rectangles(&self) -> usize {
        self.rectangles
    }
    /// Repeats failed proposals as well as the accepted original-source proof.
    pub fn reverify(&self) -> Result<()> {
        let fresh =
            discover_fiber_transition(self.proof.first(), self.proof.second(), self.limits)?;
        let other = fresh
            .into_transition()
            .ok_or_else(|| crate::input("Automatic original chart overlap remains unproved"))?;
        check(
            self.proof.matches(&other.proof)
                && self.attempts == other.attempts
                && self.rectangles == other.rectangles,
            "Automatic transition source, proposal history or shared work changed",
        )
    }
}
pub struct FiberTransitionDiscoveryResult {
    transition: Option<DiscoveredFiberTransition>,
    reason: &'static str,
    rectangles: usize,
    attempts: Vec<TransitionAttempt>,
}
impl FiberTransitionDiscoveryResult {
    pub fn transition(&self) -> Option<&DiscoveredFiberTransition> {
        self.transition.as_ref()
    }
    pub fn into_transition(self) -> Option<DiscoveredFiberTransition> {
        self.transition
    }
    pub fn reason(&self) -> &'static str {
        self.reason
    }
    pub fn rectangles(&self) -> usize {
        self.rectangles
    }
    pub fn attempts(&self) -> &[TransitionAttempt] {
        &self.attempts
    }
}
/// Infer a local overlap box from two sealed original root charts. No caller
/// bridge box or endpoint image is accepted. Breadth-first free-coordinate
/// subdivision only proposes boxes; the unchanged original transition verifier
/// supplies existence, uniqueness and local orientation. Failed proofs share
/// the same rectangle allowance. This is neither a complete overlap census nor
/// a negative root/separation certificate when no proposal succeeds.
pub fn discover_fiber_transition(
    first: &QualifiedFiber,
    second: &QualifiedFiber,
    limits: FiberTransitionDiscoveryLimits,
) -> Result<FiberTransitionDiscoveryResult> {
    check(
        (1..=100000).contains(&limits.max_rectangles)
            && (1..=1024).contains(&limits.max_proposals)
            && limits.max_depth <= 24,
        "Need bounded automatic transition work",
    )?;
    check(
        first.source().first() == second.source().first()
            && first.source().second() == second.source().second(),
        "Automatic transition needs the same ordered original surfaces",
    )?;
    let a = first.domain();
    let b = second.domain();
    let common: Box4 = std::array::from_fn(|k| [a[k][0].max(b[k][0]), a[k][1].min(b[k][1])]);
    let mut out = FiberTransitionDiscoveryResult {
        transition: None,
        reason: "no-open-chart-box",
        rectangles: 0,
        attempts: vec![],
    };
    if common.iter().any(|d| d[0] >= d[1]) {
        return Ok(out);
    }
    let free = first.free_axis().index();
    let mut pending = VecDeque::from([(common, 0)]);
    while let Some((domain, depth)) = pending.pop_front() {
        if out.attempts.len() == limits.max_proposals {
            out.reason = "proposal-budget-unproven";
            return Ok(out);
        }
        let remaining = limits.max_rectangles - out.rectangles;
        // Reserving one tiny equal share for every possible future proposal
        // can starve ALL bridge proofs before their two source replays and
        // endpoint queries complete. Offer a viable local reservation while
        // retaining the same total allowance and charging every failure.
        let allowance = (remaining / (limits.max_proposals - out.attempts.len()))
            .max(128)
            .min(remaining);
        if allowance == 0 {
            out.reason = "rectangle-budget-unproven";
            return Ok(out);
        }
        // A common chart box can be too dependent even when a smaller original
        // root enclosure overlaps. Contract UNKNOWN coordinates using fresh
        // source images; the free interval stays unchanged. The resulting box
        // is only a proposal until the full transition verifier accepts it.
        let proposal_allowance = (allowance / 3).max(1);
        let (bridge_domain, proposal_work) =
            match first
                .source()
                .certify_contracted(domain, first.free_axis(), proposal_allowance)
            {
                Ok(r) => (r.domain(), r.rectangles()),
                Err(_) => (domain, proposal_allowance),
            };
        let proof_allowance = allowance - proposal_work;
        let (proof, reason, used) = if proof_allowance == 0 {
            (None, "bridge-reservation-unproven".into(), 0)
        } else {
            match qualify_fiber_transition(first, second, bridge_domain, proof_allowance) {
                Ok(r) => {
                    let reason = r.reason().to_owned();
                    let used = r.rectangles();
                    (r.into_transition(), reason, used)
                }
                // Native errors cannot report partial work. Charge the entire
                // reservation rather than granting hidden retries for free.
                Err(e) => (None, format!("original-proof-error: {e}"), proof_allowance),
            }
        };
        let used = used + proposal_work;
        out.rectangles += used;
        out.attempts.push(TransitionAttempt {
            domain,
            bridge_domain,
            reason,
            rectangles: used,
        });
        if let Some(proof) = proof {
            out.reason = "qualified-discovered-original-transition";
            out.transition = Some(DiscoveredFiberTransition {
                proof,
                limits,
                attempts: out.attempts.clone(),
                rectangles: out.rectangles,
            });
            return Ok(out);
        }
        if depth == limits.max_depth {
            continue;
        }
        // After the whole common box fails, try the center of the ORIGINAL
        // second chart's root image in the first free coordinate. Anisotropic
        // chart boxes can be much wider than their shared root segment, so
        // blind breadth-first peeling wastes the fixed pair allowance. The
        // image only seeds a proposal; both sources and the complete bridge
        // are independently reverified before any connection is accepted.
        if out.attempts.len() == 1 && pending.len() < limits.max_proposals - out.attempts.len() {
            let target = first.free_axis().index();
            let image = if second.free_axis().index() == target {
                Some(second.domain()[target])
            } else {
                let index = (0..4)
                    .filter(|&k| k != second.free_axis().index())
                    .position(|k| k == target)
                    .unwrap();
                second.report().root_image.map(|b| b[index])
            };
            if let Some(image) = image {
                let low = common[target][0].max(image[0]);
                let high = common[target][1].min(image[1]);
                let a = 0.75 * low + 0.25 * high;
                let b = 0.25 * low + 0.75 * high;
                if low <= a && a < b && b <= high {
                    let mut seed = common;
                    seed[target] = [a, b];
                    pending.push_front((seed, 1));
                }
            }
        }
        let mid = 0.5 * domain[free][0] + 0.5 * domain[free][1];
        if !(domain[free][0] < mid && mid < domain[free][1]) {
            continue;
        }
        let mut left = domain;
        let mut right = domain;
        left[free][1] = mid;
        right[free][0] = mid;
        // The queue itself is bounded, independently of depth or f64 spacing.
        if pending.len() + 2 <= limits.max_proposals - out.attempts.len() {
            pending.push_back((left, depth + 1));
            pending.push_back((right, depth + 1));
        }
    }
    out.reason = "local-overlap-search-unproven";
    Ok(out)
}
