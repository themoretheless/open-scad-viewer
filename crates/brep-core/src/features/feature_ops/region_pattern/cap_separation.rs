//! Distinct proposed charts gain authority only after their actual trimmed cap
//! images prove strict separation. An unresolved carrier proposal grants none.
use super::closure::Cap;
use super::*;
use nurbs_core::trim_domain::TrimDomain;
pub(super) fn verify(
    caps: &[Cap],
    tolerance: f64,
    limits: Limits,
    work: &mut ProofWork,
) -> Result<()> {
    if caps.len() < 2 || caps.iter().all(|c| c.chart == caps[0].chart) {
        return Ok(());
    }
    let mut bounds = Vec::new();
    for cap in caps {
        cap.surface.validate()?;
        let count = cap.surface.control_points.iter().flatten().count();
        work.charge_cells(count, limits)?;
        let mut b = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
        for p in cap.surface.control_points.iter().flatten() {
            for k in 0..3 {
                b[k][0] = b[k][0].min(p[k]);
                b[k][1] = b[k][1].max(p[k]);
            }
        }
        // Positive rational basis preserves this ORIGINAL pole hull. Min/max
        // use existing real coefficient values; no rounded difference is needed.
        bounds.push(b);
    }
    let domains = caps
        .iter()
        .map(|c| {
            TrimDomain::new(
                &c.rings
                    .iter()
                    .map(|r| r.iter().map(|e| e.pcurve.clone()).collect())
                    .collect::<Vec<_>>(),
                1e-8,
            )
        })
        .collect::<nurbs_core::Result<Vec<_>>>()?;
    for i in 0..caps.len() {
        for j in i + 1..caps.len() {
            if caps[i].chart != caps[j].chart {
                let remaining = work.remaining(limits)?;
                work.pairs += 1;
                if (0..3)
                    .any(|k| bounds[i][k][1] < bounds[j][k][0] || bounds[j][k][1] < bounds[i][k][0])
                {
                    continue;
                }
                let proof = nurbs_core::trimmed_surface_distance::distance(
                    &caps[i].surface,
                    &domains[i],
                    &caps[j].surface,
                    &domains[j],
                    tolerance,
                    remaining.max_cells.min(4096),
                    remaining.max_domain_cells,
                )?;
                work.cells += proof.cells;
                work.domain_cells += proof.domain_cells;
                // A conservative positive lower bound proves separation even when the
                // requested nearest-distance accuracy has not converged.
                if !proof.lower_bound_mm.is_finite() || proof.lower_bound_mm <= 0. {
                    return Err(error(
                        "Distinct closure charts have no proven trimmed-cap separation",
                    ));
                }
            }
        }
    }
    Ok(())
}
