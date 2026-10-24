//! One common contraction for every ORIGINAL interpolated carrier.
//! Separate endpoint certificates cannot establish the intermediate sections.
use super::*;

pub(super) fn certify_pair(
    caps: [&Surface; 2],
    out: &mut IdealVolumeReport,
    max: usize,
) -> Result<bool> {
    for cap in caps {
        cap.validate()?;
        if !surface_c0(cap) {
            return Ok(false);
        }
    }
    // Tighten ORIGINAL whole-family derivative bounds. Every failed complete
    // cover and every partial query stays charged in the same owner ledger.
    for n in [1, 2, 4, 8, 16] {
        let mut jacobian: Option<[[I; 2]; 3]> = None;
        for cap in caps {
            for u in 0..n {
                for v in 0..n {
                    let uv = [
                        [u as f64 / n as f64, (u + 1) as f64 / n as f64],
                        [v as f64 / n as f64, (v + 1) as f64 / n as f64],
                    ];
                    let stage = max.min(out.cells.saturating_add(100000));
                    // cap_jets owns each cap's knot domain and returns normalized UV
                    // derivatives, including every original one-sided knot limit.
                    let Some([_, du, dv]) = cap_jets(cap, uv, &mut out.cells, stage)? else {
                        return Ok(false);
                    };
                    let next = std::array::from_fn(|k| [du[k], dv[k]]);
                    if let Some(j) = &mut jacobian {
                        for k in 0..3 {
                            for a in 0..2 {
                                j[k][a] = I::new(
                                    j[k][a].lo.min(next[k][a].lo),
                                    j[k][a].hi.max(next[k][a].hi),
                                )?;
                            }
                        }
                    } else {
                        jacobian = Some(next);
                    }
                }
            }
        }
        let Some(j) = jacobian else {
            return Ok(false);
        };
        // D((1-s)C0+sC1) is a convex blend of the normalized endpoint
        // Jacobians. The SAME projection and inverse must work on its full
        // hull; unrelated successful endpoint metrics give no credit.
        for axes in [[0, 1], [0, 2], [1, 2]] {
            if out.cells == max {
                return Ok(false);
            }
            out.cells += 1;
            if crate::surface_injectivity::contraction([j[axes[0]], j[axes[1]]])?
                .is_some_and(|q| q < 1.)
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
