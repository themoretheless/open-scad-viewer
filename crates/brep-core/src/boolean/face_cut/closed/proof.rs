use super::*;
use nurbs_core::{surface::Surface, trim_region_audit};
#[derive(Default)]
pub(super) struct Work {
    pub cells: usize,
    pub pairs: usize,
}
pub(super) fn audit(
    loops: &[Vec<Curve>],
    uv: f64,
    w: &mut Work,
    l: ClosedCutLimits,
) -> Result<Option<bool>> {
    if l.cells - w.cells < 2 || w.pairs == l.pairs {
        return Err(unproven("Shared closed-cut trim budget exhausted"));
    }
    let half = ((l.cells - w.cells) / 2).min(100000);
    let r = trim_region_audit::inspect(loops, uv, (l.pairs - w.pairs).min(100000), half, half)?;
    w.cells += r.cells + r.domain_cells;
    w.pairs += r.pairs;
    if r.valid.is_none() {
        return Err(unproven(&format!(
            "Closed-cut trim proof unresolved: {} {:?}",
            r.reason, r.problem_loops
        )));
    }
    Ok(if r.valid == Some(true) && r.winding[0] != Some(1) {
        Some(false)
    } else {
        r.valid
    })
}
pub(super) fn lift(
    c: &Curve,
    pc: &Curve,
    s: &Surface,
    reverse: bool,
    tol: f64,
    w: &mut Work,
    l: ClosedCutLimits,
) -> Result<()> {
    if w.cells == l.cells {
        return Err(unproven("Shared closed-cut lift budget exhausted"));
    }
    let r =
        curve_surface_agreement::verify(c, pc, s, reverse, tol, (l.cells - w.cells).min(100000))?;
    w.cells += r.cells;
    if r.status != Status::WithinTolerance {
        return Err(unproven(
            "Actual closed-cut boundary lift is unresolved or mismatched",
        ));
    }
    Ok(())
}
