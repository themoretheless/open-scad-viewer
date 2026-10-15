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
pub(super) fn premises(
    s: &Sweep<'_>,
    caps: [&Surface; 2],
    out: &mut IdealVolumeReport,
    max_cells: usize,
    max_work: u64,
) -> Result<bool> {
    if caps[0] != caps[1] {
        out.reason = Some("ideal-volume-closed-common-carrier-required");
        return Ok(false);
    }
    for law in [s.path, s.scale, s.twist]
        .into_iter()
        .chain(s.orientation_guide)
        .chain(s.affine_laws.into_iter().flat_map(|(a, c)| [a, c]))
    {
        if !endpoint_identity(law, &mut out.cells, max_cells) {
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
        // Corrected-Frenet and holonomy-corrected spatial Bishop jets require
        // their own source derivative owners. Do not borrow open raw jets.
        if s.options.orientation != Orientation::FixedNormal {
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
