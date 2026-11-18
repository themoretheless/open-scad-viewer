//! Exact periodic source ownership, before permitting a wrapped pair chart.
use super::*;
fn endpoint_identity(c: &Curve, spent: &mut usize, max: usize) -> bool {
    let charge = 2 * (c.degree + 1) + 6;
    if charge > max - *spent {
        return false;
    }
    *spent += charge;
    let [a, b] = c.domain();
    // This direct proof deliberately accepts only clamped original endpoints.
    // Different positive endpoint weights still have exactly the stored
    // Cartesian point. Near closure, tolerance and rounded evaluation do not
    // supply an identity of ideal-source endpoints.
    !c.periodic
        && c.knots[..=c.degree].iter().all(|&u| u == a)
        && c.knots[c.control_points.len()..].iter().all(|&u| u == b)
        && c.control_points.first() == c.control_points.last()
}
pub(super) fn uses_periodic_path(s: &Sweep<'_>, closed: bool) -> bool {
    closed
        && s.path.periodic
        && (s.frame_laws.is_some()
            || s.orientation_guide.is_some()
            || s.options.orientation == Orientation::Frenet
            || (s.options.orientation == Orientation::RotationMinimizing
                && s.spatial_rmf_error_limits.is_some()))
}

pub(super) fn premises(
    s: &Sweep<'_>,
    caps: [&Surface; 2],
    identity: Option<&cap_identity::Identity<'_>>,
    out: &mut IdealVolumeReport,
    max_cells: usize,
    max_work: u64,
) -> Result<bool> {
    if !identity.is_some_and(|p| p.matches(caps)) {
        out.reason = Some("ideal-volume-closed-common-carrier-required");
        return Ok(false);
    }
    let laws = [(s.path, uses_periodic_path(s, true)), (s.scale, false), (s.twist, false)]
        .into_iter()
        .chain(s.orientation_guide.map(|g| (g, g.periodic)))
        .chain(s.affine_laws.into_iter().flat_map(|(a, c)| [(a, false), (c, false)]));
    for (law, active_basis) in laws
    {
        // Periodic path and guide storage require an identity of the
        // original active endpoint basis. Frame and pose laws still
        // pass through their respective full continuity proofs below.
        let equal = if active_basis {
            let seam = super::super::endpoint_jets::certify_position(
                law, true, max_work - out.exact_work,
            )?;
            out.exact_work += seam.exact_work;
            seam.certified
        } else {
            endpoint_identity(law, &mut out.cells, max_cells)
        };
        if !equal {
            out.reason = Some("ideal-volume-exact-source-seam-unproved");
            return Ok(false);
        }
    }
    let (proven, cells, exact_work, reason) = if s.frame_laws.is_some() {
        let r = s.certify_closed_authored_frame_smoothness(
            1,
            max_cells - out.cells,
            max_work - out.exact_work,
        )?;
        (
            r.closed_source_frame_smoothness_certified,
            r.cells,
            r.exact_work,
            r.reason,
        )
    } else if s.orientation_guide.is_some() {
        let r = s.certify_closed_guided_frame_smoothness(
            1,
            max_cells - out.cells,
            max_work - out.exact_work,
        )?;
        (
            r.closed_source_frame_smoothness_certified,
            r.cells,
            r.exact_work,
            r.reason,
        )
    } else {
        // Explicit spatial Bishop uses its OWN corrected derivative owner and
        // periodic source theorem. Frenet owns its nonzero curvature cover.
        // Corrected-Frenet/implicit fallback modes
        // still require their original source derivative owners.
        if !matches!(s.options.orientation,Orientation::FixedNormal|Orientation::Frenet)
            && !(s.options.orientation == Orientation::RotationMinimizing
                && s.spatial_rmf_error_limits.is_some())
        {
            out.reason = Some("ideal-volume-closed-frame-jets-unproved");
            return Ok(false);
        }
        let r = s.certify_closed_path_frame_smoothness(
            1,
            max_cells - out.cells,
            max_work - out.exact_work,
        )?;
        (
            r.closed_source_frame_smoothness_certified,
            r.cells,
            r.exact_work,
            r.reason,
        )
    };
    out.cells += cells;
    out.exact_work += exact_work;
    out.reason = reason;
    Ok(proven)
}
