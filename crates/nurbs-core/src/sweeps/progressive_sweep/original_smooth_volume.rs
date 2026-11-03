//! Constructor-owned C1/C2 embedding of the ORIGINAL complete filled map.
//! Boundary corners/cap-wall Gk, material trims and retained BRep smoothness
//! remain separate. An authored C0 crease is never upgraded by this factory.
use super::*;
#[derive(Clone, Copy)]
pub struct OriginalSmoothVolumeLimits {
    pub order: usize,
    pub cells: usize,
    pub exact_work: u64,
    pub products: usize,
}
#[derive(Debug)]
pub struct OriginalSmoothVolumeReport {
    pub certified: bool,
    pub order: usize,
    pub volume: IdealVolumeReport,
    pub cap_continuity_certified: bool,
    pub motion_continuity_certified: bool,
    pub closed_boundary: Option<OriginalBoundarySmoothnessReport>,
    pub cells: usize,
    pub exact_work: u64,
    pub products: usize,
    pub reason: Option<&'static str>,
}
/// Opaque proof for this exact immutable sweep and these exact original caps.
/// Report data cannot construct/rebind this object. Its Ck assertion concerns
/// volume charts, not Gk of open cap/wall corners or unowned trim boundaries.
pub struct OriginalSmoothVolume<'s, 'a> {
    sweep: &'s MultiSweep<'a>,
    caps: [&'s Surface; 2],
    loop_sizes: Vec<usize>,
    report: OriginalSmoothVolumeReport,
}
impl<'s, 'a> OriginalSmoothVolume<'s, 'a> {
    pub fn report(&self) -> &OriginalSmoothVolumeReport {
        &self.report
    }
    pub fn sweep(&self) -> &'s MultiSweep<'a> {
        self.sweep
    }
    pub fn loop_sizes(&self) -> &[usize] {
        &self.loop_sizes
    }
    pub fn caps(&self) -> [&'s Surface; 2] {
        self.caps
    }
}
pub enum OriginalSmoothVolumeAttempt<'s, 'a> {
    Certified(Box<OriginalSmoothVolume<'s, 'a>>),
    Refused(Box<OriginalSmoothVolumeReport>),
}
pub(super) mod motion;
fn basis_smooth(s: &Surface, order: usize) -> bool {
    for (p, k, n) in [
        (s.degree_u, &s.knots_u, s.control_points.len()),
        (s.degree_v, &s.knots_v, s.control_points[0].len()),
    ] {
        let mut i = 0;
        while i < k.len() {
            let t = k[i];
            let mut end = i + 1;
            while end < k.len() && k[end] == t {
                end += 1;
            }
            if k[p] < t && t < k[n] && (p < order || end - i > p - order) {
                return false;
            }
            i = end;
        }
    }
    true
}
fn caps_smooth(
    caps: [&Surface; 2],
    out: &mut OriginalSmoothVolumeReport,
    limits: OriginalSmoothVolumeLimits,
) -> bool {
    let mut ledger = IdealVolumeReport {
        embedding_certified: false,
        regularity_certified: false,
        parameter_topology_preserved: false,
        cells: out.cells,
        exact_work: out.exact_work,
        ..out.volume.clone()
    };
    let proven = (|| {
        for (i, s) in caps.into_iter().enumerate() {
            if i == 1 && caps[0] == caps[1] {
                continue;
            }
            let charge = s.knots_u.len() + s.knots_v.len();
            if charge > 100000 || charge > limits.cells - ledger.cells {
                return false;
            }
            ledger.cells += charge;
            if !basis_smooth(s, limits.order)
                && !ideal_volume::cap_identity::certify_continuity(
                    s,
                    limits.order,
                    &mut ledger,
                    limits.cells,
                    limits.exact_work,
                )
            {
                return false;
            }
        }
        true
    })();
    out.cells = ledger.cells;
    out.exact_work = ledger.exact_work;
    proven
}
impl<'a> MultiSweep<'a> {
    /// Fresh global original embedding + complete cap Ck + actual motion Ck.
    /// The open traversal is an interval; closed sources use the exact native
    /// quotient and periodic motion owner. Closed source profile compatibility
    /// is additionally recomputed. A valid material region/profile binding is
    /// still the BRep owner's obligation; sharp open cap-wall corners stay C0.
    /// Every failed/successful source stage consumes ONE aggregate ledger.
    pub fn certify_original_smooth_volume_atlas<'s>(
        &'s self,
        caps: [&'s Surface; 2],
        loop_sizes: &[usize],
        projection: [[f64; 3]; 3],
        limits: OriginalSmoothVolumeLimits,
    ) -> Result<OriginalSmoothVolumeAttempt<'s, 'a>> {
        let out = certify_charts(self, caps, loop_sizes, projection, limits, true)?;
        if !out.certified {
            return Ok(OriginalSmoothVolumeAttempt::Refused(Box::new(out)));
        }
        Ok(OriginalSmoothVolumeAttempt::Certified(Box::new(
            OriginalSmoothVolume {
                sweep: self,
                caps,
                loop_sizes: loop_sizes.to_vec(),
                report: out,
            },
        )))
    }
}
/// Shared fresh chart proof. The classified owner separately checks its exact
/// declared profile classes; the legacy owner retains uniform closed Gk.
pub(super) fn certify_charts(
    sweep: &MultiSweep<'_>,
    caps: [&Surface; 2],
    loop_sizes: &[usize],
    projection: [[f64; 3]; 3],
    limits: OriginalSmoothVolumeLimits,
    strict_profiles: bool,
) -> Result<OriginalSmoothVolumeReport> {
    check(
        matches!(limits.order, 1 | 2)
            && limits.cells <= 1000000
            && limits.exact_work <= 1000000
            && limits.products <= 1000000
            && !loop_sizes.is_empty()
            && loop_sizes.len() <= 64
            && loop_sizes.iter().all(|&n| n > 0 && n <= 64)
            && loop_sizes.iter().sum::<usize>() == sweep.sweeps.len(),
        "Invalid original smooth volume limits or profile partition",
    )?;
    let volume =
        sweep.certify_ideal_volume_atlas(caps, projection, limits.cells, limits.exact_work)?;
    let mut out = OriginalSmoothVolumeReport {
        certified: false,
        order: limits.order,
        cells: volume.cells,
        exact_work: volume.exact_work,
        volume,
        cap_continuity_certified: false,
        motion_continuity_certified: false,
        closed_boundary: None,
        products: 0,
        reason: None,
    };
    if !(out.volume.embedding_certified
        && out.volume.regularity_certified
        && out.volume.parameter_topology_preserved)
    {
        out.reason = out.volume.reason;
        return Ok(out);
    }
    if !caps_smooth(caps, &mut out, limits) {
        out.reason = Some("original-filled-cap-continuity-unproved");
        return Ok(out);
    }
    out.cap_continuity_certified = true;
    if sweep.source_is_closed()? {
        let motion_limits = OriginalBoundarySmoothnessLimits {
            order: limits.order,
            cells: (limits.cells - out.cells).min(100000),
            exact_work: limits.exact_work - out.exact_work,
            products: limits.products - out.products,
        };
        let proof = if strict_profiles {
            sweep.certify_original_closed_boundary_smoothness(loop_sizes, motion_limits)?
        } else {
            sweep.certify_closed_motion(motion_limits)?
        };
        out.cells += proof.cells;
        out.exact_work += proof.exact_work;
        out.products += proof.products;
        let certified = proof.laws_certified
            && proof.frame_certified
            && (!strict_profiles || proof.differential_compatibility_certified);
        out.reason = proof.reason;
        out.closed_boundary = Some(proof);
        if !certified {
            return Ok(out);
        }
    } else if !motion::open(&sweep.sweeps[0], &mut out, limits)? {
        return Ok(out);
    }
    out.motion_continuity_certified = true;
    out.certified = true;
    out.reason = None;
    Ok(out)
}
#[cfg(test)]
mod tests;
