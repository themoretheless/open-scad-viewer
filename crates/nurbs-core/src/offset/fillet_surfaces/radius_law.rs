use super::*;

/// Radius law along the normalized spine parameter `t ∈ [0, 1]`.
#[derive(Clone, Debug)]
pub enum RadiusLaw {
    /// Constant rolling-ball radius.
    Constant(f64),
    /// Linear interpolation between the start and end radii.
    Linear { start: f64, end: f64 },
    /// Natural cubic B-spline interpolation through `(t, r)` control points
    /// (`t` strictly increasing, at least two points; clamped outside).
    Spline(Vec<(f64, f64)>),
}

impl RadiusLaw {
    pub(super) fn validate(&self) -> Result<()> {
        match self {
            RadiusLaw::Constant(r) => {
                require_finite_f64(*r, "radius")?;
                check(*r > 0., "Fillet radius must be positive")?;
            }
            RadiusLaw::Linear { start, end } => {
                require_finite_f64(*start, "radius_start")?;
                require_finite_f64(*end, "radius_end")?;
                check(
                    *start > 0. && *end > 0.,
                    "Fillet linear law radii must be positive",
                )?;
            }
            RadiusLaw::Spline(points) => {
                check(
                    points.len() >= 2 && points.len() <= 64,
                    "Fillet spline law needs 2..=64 control points",
                )?;
                for (i, &(t, r)) in points.iter().enumerate() {
                    require_finite_at(t, "law_parameter", i)?;
                    require_finite_at(r, "law_radius", i)?;
                }
                for w in points.windows(2) {
                    check(
                        w[1].0 > w[0].0,
                        "Fillet spline law parameters must be strictly increasing",
                    )?;
                }
                check(
                    points.iter().all(|&(_, r)| r > 0.),
                    "Fillet spline law radii must be positive",
                )?;
            }
        }
        Ok(())
    }

    /// Evaluate the law at `t` (clamped to the control range for splines).
    pub fn evaluate(&self, t: f64) -> f64 {
        match self {
            RadiusLaw::Constant(r) => *r,
            RadiusLaw::Linear { start, end } => start + (end - start) * t.clamp(0., 1.),
            RadiusLaw::Spline(points) => natural_cubic(points, t),
        }
    }

    /// `[min, max, mean]` over a budgeted uniform sample.
    pub(super) fn sampled_range(&self) -> [f64; 3] {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        let mut sum = 0.;
        for i in 0..=LAW_SAMPLES {
            let r = self.evaluate(i as f64 / LAW_SAMPLES as f64);
            lo = lo.min(r);
            hi = hi.max(r);
            sum += r;
        }
        [lo, hi, sum / (LAW_SAMPLES + 1) as f64]
    }
}

/// Natural cubic spline scalar interpolation (Thomas solve, budgeted by the
/// 64-point validation cap).
pub(super) fn natural_cubic(points: &[(f64, f64)], t: f64) -> f64 {
    let n = points.len();
    let clamped = t.clamp(points[0].0, points[n - 1].0);
    // Second derivatives m[i]; natural boundary m[0] = m[n-1] = 0. Thomas
    // forward sweep over the interior rows of the tridiagonal system.
    let mut m = vec![0.; n];
    let interior = n.saturating_sub(2);
    let mut cp = vec![0.; interior];
    let mut dp = vec![0.; interior];
    for row in 0..interior {
        let i = row + 1;
        let h0 = points[i].0 - points[i - 1].0;
        let h1 = points[i + 1].0 - points[i].0;
        let rhs = 6. * ((points[i + 1].1 - points[i].1) / h1 - (points[i].1 - points[i - 1].1) / h0);
        let denom = if row == 0 {
            2. * (h0 + h1)
        } else {
            2. * (h0 + h1) - h0 * cp[row - 1]
        };
        cp[row] = h1 / denom;
        dp[row] = (rhs - if row == 0 { 0. } else { h0 * dp[row - 1] }) / denom;
    }
    for row in (0..interior).rev() {
        m[row + 1] = dp[row] - cp[row] * m[row + 2];
    }
    let i = (1..n).find(|&i| clamped <= points[i].0).unwrap_or(n - 1);
    let h = points[i].0 - points[i - 1].0;
    let a = (points[i].0 - clamped) / h;
    let b = 1. - a;
    a * points[i - 1].1 + b * points[i].1 + ((a.powi(3) - a) * m[i - 1] + (b.powi(3) - b) * m[i]) * h * h / 6.
}
