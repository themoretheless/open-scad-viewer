//! Shared degree-refined Bezier interpolation used by the offset contact fitters.
//!
//! Each fitter keeps its own sampling, budgets and proofs; this module only owns
//! the degree ladder, the Chebyshev-Lobatto sites, the per-coordinate pole
//! reconstruction and the single-span Bezier assembly they have in common.
use crate::{
    Result, bernstein_interpolation as interpolation, check, curve::Curve,
    distance_bounds::Interval as I,
};

/// Doubling degree ladder `1, 2, 4, ...` capped at `max_degree`.
pub(crate) fn degree_ladder(max_degree: usize) -> Vec<usize> {
    let mut degrees = vec![1usize];
    let mut last = 1;
    while last < max_degree {
        last = (2 * last).min(max_degree);
        degrees.push(last);
    }
    degrees
}

/// Chebyshev-Lobatto interpolation sites on `domain` with exact endpoints;
/// `resolution` is the caller's error when the sites stop increasing.
pub(crate) fn chebyshev_sites(
    domain: [f64; 2],
    degree: usize,
    resolution: &str,
) -> Result<Vec<f64>> {
    let sites: Vec<_> = (0..=degree)
        .map(|i| {
            if i == 0 {
                domain[0]
            } else if i == degree {
                domain[1]
            } else {
                let t = (1. - (std::f64::consts::PI * i as f64 / degree as f64).cos()) * 0.5;
                domain[0] * (1. - t) + domain[1] * t
            }
        })
        .collect();
    check(sites.windows(2).all(|r| r[0] < r[1]), resolution)?;
    Ok(sites)
}

/// Midpoint poles interpolating `samples` coordinate by coordinate; each
/// coordinate adds `n * n` to `work`.
pub(crate) fn interpolate_poles<const D: usize>(
    samples: &[[f64; D]],
    basis: &[Vec<I>],
    work: &mut usize,
) -> Result<Vec<Vec<f64>>> {
    let n = samples.len();
    let mut points = vec![vec![0.; D]; n];
    for k in 0..D {
        let nodal: Vec<_> = samples.iter().map(|p| I::point(p[k])).collect();
        let c = interpolation::reconstruct(&nodal, basis)?;
        *work += n * n;
        for (i, c) in c.into_iter().enumerate() {
            points[i][k] = c.lo * 0.5 + c.hi * 0.5;
        }
    }
    Ok(points)
}

/// Non-periodic polynomial single-span Bezier curve over `domain`.
pub(crate) fn bezier(domain: [f64; 2], control_points: Vec<Vec<f64>>) -> Curve {
    let n = control_points.len();
    Curve {
        degree: n - 1,
        knots: std::iter::repeat_n(domain[0], n)
            .chain(std::iter::repeat_n(domain[1], n))
            .collect(),
        control_points,
        weights: vec![1.; n],
        periodic: false,
    }
}
