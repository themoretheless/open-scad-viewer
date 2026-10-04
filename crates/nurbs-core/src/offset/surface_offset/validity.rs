//! Fold-free offset precondition: sampled principal-curvature radius bounds
//! with outward rounding (split from `surface_offset`, byte for byte).
use super::*;

/// Certified fold-free offset precondition (checklist 405).
///
/// An offset by signed `distance` is guaranteed fold-free (locally) only when
/// `|distance| < min_S r`, where `r` is the radius `1/|k|` of the principal
/// curvature `k` that opposes the offset side: for `distance > 0` (along
/// `S_u x S_v`) only negative principal curvatures limit the offset, for
/// `distance < 0` only positive ones, and for `distance == 0` both sides
/// bound the report. `max_offset` is the sampled minimum of that radius,
/// rounded DOWN so it never overstates the safe range.
pub struct OffsetValidityReport {
    /// Maximum `|distance|` certified fold-free by the sampled estimate
    /// (conservative lower bound of `min_radius`); `INFINITY` when no
    /// curvature opposes the offset side (e.g. a plane or the convex side).
    pub max_offset: f64,
    /// Certified interval `[lo, hi]` bracketing the estimated minimum
    /// principal curvature radius on the limiting side; widened outward.
    pub min_radius: [f64; 2],
    /// Parameter location where the limiting radius was sampled.
    pub limiting_uv: [f64; 2],
    /// True when the requested `|distance|` is strictly below `max_offset`.
    pub requested_ok: bool,
}

/// Principal curvatures from Gaussian K and signed mean H, ordered k1 >= k2.
pub(super) fn principal(gaussian: f64, mean: f64) -> Option<(f64, f64)> {
    let disc = mean * mean - gaussian;
    (disc >= 0.).then(|| {
        let root = disc.sqrt();
        (mean + root, mean - root)
    })
}

/// Largest limiting principal curvature magnitude at (u, v) for an offset to
/// the side of `signum`. With this crate's convention (`L = <n, S_uu>`, so a
/// sphere with outward normal has k = -1/R) the offset `S + d*n` folds when
/// `d * k >= 1`: for `signum > 0` only positive `k` limits, for `signum < 0`
/// only negative `k`, and `signum == 0` reports the largest `|k|` of either
/// sign. None when the jet or its curvature is unreliable (poles).
fn opposing_curvature(surface: &Surface, u: f64, v: f64, signum: f64) -> Result<Option<f64>> {
    let j = jet(surface, u, v)?;
    let Some((k, h)) = j.gaussian.zip(j.mean) else {
        return Ok(None);
    };
    let Some((k1, k2)) = principal(k, h) else {
        return Ok(None);
    };
    let limiting = |k: f64| {
        if signum > 0. {
            k.max(0.)
        } else if signum < 0. {
            (-k).max(0.)
        } else {
            k.abs()
        }
    };
    Ok(Some(limiting(k1).max(limiting(k2))))
}

/// Sampled minimum fold-free radius on the offset side over one grid.
/// Returns (kappa_max, uv) of the worst reliable sample.
fn worst_cell(
    surface: &Surface,
    [u0, u1]: [f64; 2],
    [v0, v1]: [f64; 2],
    nu: usize,
    nv: usize,
    signum: f64,
) -> Result<(f64, Option<[f64; 2]>)> {
    let mut kappa = 0_f64;
    let mut uv = None;
    for i in 0..=nu {
        for j in 0..=nv {
            let u = u0 + (u1 - u0) * i as f64 / nu as f64;
            let v = v0 + (v1 - v0) * j as f64 / nv as f64;
            if let Some(k) = opposing_curvature(surface, u, v, signum)? {
                if k.is_finite() && k > kappa {
                    kappa = k;
                    uv = Some([u, v]);
                }
            }
        }
    }
    Ok((kappa, uv))
}

/// Precondition check: an offset by `|distance|` is guaranteed fold-free
/// (locally) only when `|distance|` stays below the minimum principal
/// curvature radius on the offset side. The minimum is estimated on a dense
/// grid from the shape operator (Gaussian and signed mean curvature), the
/// worst cell is refined once with a 4x4 subgrid, and the radius interval is
/// widened outward. This is a sampled estimate, not an inter-sample proof;
/// `max_offset` is rounded down so it never overstates the safe range.
/// Degenerate-everywhere surfaces report `max_offset == 0` and reject any
/// nonzero request.
pub fn offset_validity(surface: &Surface, distance: f64) -> Result<OffsetValidityReport> {
    surface.validate()?;
    check(
        distance.is_finite(),
        "Surface offset validity distance must be finite",
    )?;
    let ([u0, u1], [v0, v1]) = domain(surface);
    let nu = (2 * surface.control_points.len()).clamp(4, 64);
    let nv = (2 * surface.control_points[0].len()).clamp(4, 64);
    let signum = distance.signum();
    let (mut kappa, mut limiting) = worst_cell(surface, [u0, u1], [v0, v1], nu, nv, signum)?;
    // One refinement pass around the worst cell: the true maximum curvature
    // may sit between coarse grid nodes.
    let kappa_coarse = kappa;
    if let Some([u, v]) = limiting {
        let du = (u1 - u0) / nu as f64;
        let dv = (v1 - v0) / nv as f64;
        let cell_u = [(u - du / 2.).max(u0), (u + du / 2.).min(u1)];
        let cell_v = [(v - dv / 2.).max(v0), (v + dv / 2.).min(v1)];
        let (refined_kappa, refined_uv) = worst_cell(surface, cell_u, cell_v, 4, 4, signum)?;
        if refined_kappa > kappa {
            kappa = refined_kappa;
        }
        if let Some(uv) = refined_uv {
            limiting = Some(uv);
        }
    }
    let Some(limiting_uv) = limiting else {
        // No reliable curvature anywhere (or none opposes this side): either
        // no fold-free statement is possible, or the offset never folds.
        if kappa == 0. {
            // Distinguish "no opposing curvature" from "no reliable jet" by
            // probing for any reliable normal on the grid.
            let mut any_jet = false;
            'probe: for i in 0..=nu.min(8) {
                for j in 0..=nv.min(8) {
                    let u = u0 + (u1 - u0) * i as f64 / nu.min(8) as f64;
                    let v = v0 + (v1 - v0) * j as f64 / nv.min(8) as f64;
                    if jet(surface, u, v)?.normal.is_some() {
                        any_jet = true;
                        break 'probe;
                    }
                }
            }
            if any_jet {
                return Ok(OffsetValidityReport {
                    max_offset: f64::INFINITY,
                    min_radius: [f64::INFINITY, f64::INFINITY],
                    limiting_uv: [(u0 + u1) / 2., (v0 + v1) / 2.],
                    requested_ok: true,
                });
            }
            return Ok(OffsetValidityReport {
                max_offset: 0.,
                min_radius: [0., 0.],
                limiting_uv: [(u0 + u1) / 2., (v0 + v1) / 2.],
                requested_ok: distance == 0.,
            });
        }
        unreachable!("a positive worst curvature always records its location");
    };
    // Outward-rounded radius interval around 1/kappa, widened by a relative
    // margin that covers both the coarse-to-refined change and numerical
    // curvature evaluation error (floor 1e-6): this is a sampled estimate,
    // not an inter-sample proof.
    let radius = 1. / kappa;
    let refinement_change = (kappa - kappa_coarse).abs() / kappa;
    let margin = refinement_change.max(1e-6);
    let radius_lo = next_down(radius * (1. - margin));
    let radius_hi = next_up(radius * (1. + margin));
    let max_offset = radius_lo;
    numeric(max_offset.is_finite() || kappa == 0., "Offset validity radius overflowed")?;
    Ok(OffsetValidityReport {
        max_offset,
        min_radius: [radius_lo, radius_hi],
        limiting_uv,
        requested_ok: distance == 0. || distance.abs() < max_offset,
    })
}

/// UV rectangle of one validity-grid cell around `uv`, for degenerate-region
/// reporting when the requested offset exceeds the fold-free bound.
pub(super) fn validity_cell(surface: &Surface, [u, v]: [f64; 2]) -> [f64; 4] {
    let ([u0, u1], [v0, v1]) = domain(surface);
    let du = (u1 - u0) / (2 * surface.control_points.len()).clamp(4, 64) as f64;
    let dv = (v1 - v0) / (2 * surface.control_points[0].len()).clamp(4, 64) as f64;
    [
        (u - du / 2.).max(u0),
        (u + du / 2.).min(u1),
        (v - dv / 2.).max(v0),
        (v + dv / 2.).min(v1),
    ]
}
