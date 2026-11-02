//! Independently authored rational Bezier degree elevation and common refinement.
use crate::{Result, check, curve::Curve, surface::Surface};

#[derive(Clone, Debug, PartialEq)]
pub struct Compatibility {
    pub source_domains: Vec<[f64; 2]>,
    pub source_degrees: Vec<usize>,
    pub degree: usize,
    pub spans: usize,
    /// Every output span uses a common Bezier basis. This representation has
    /// C0 knot multiplicity; actual geometric smoothness may be higher.
    pub representation_continuity: &'static str,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CompatibleLoft {
    pub surface: Surface,
    pub profiles: Vec<Curve>,
    pub report: Compatibility,
}

fn normalize(c: &Curve) -> Result<Curve> {
    let [a, b] = c.domain();
    Ok(Curve::new(
        c.degree(),
        c.knots().iter().map(|u| (u - a) / (b - a)).collect(),
        c.controls().to_vec(),
        c.weights().to_vec(),
    )?)
}
fn elevate(c: &Curve, degree: usize) -> Result<Curve> {
    check(
        c.controls().len() == c.degree() + 1,
        "Bezier segment expected after split",
    )?;
    let mut points = c
        .controls()
        .iter()
        .zip(c.weights())
        .map(|(&p, &w)| [p[0] * w, p[1] * w, p[2] * w, w])
        .collect::<Vec<_>>();
    let mut p = c.degree();
    while p < degree {
        let mut next = vec![[0.; 4]; p + 2];
        next[0] = points[0];
        next[p + 1] = points[p];
        for i in 1..=p {
            let alpha = i as f64 / (p + 1) as f64;
            next[i] =
                std::array::from_fn(|d| alpha * points[i - 1][d] + (1. - alpha) * points[i][d]);
        }
        points = next;
        p += 1;
    }
    let [a, b] = c.domain();
    let mut knots = vec![a; degree + 1];
    knots.extend(vec![b; degree + 1]);
    Ok(Curve::new(
        degree,
        knots,
        points
            .iter()
            .map(|p| [p[0] / p[3], p[1] / p[3], p[2] / p[3]])
            .collect(),
        points.iter().map(|p| p[3]).collect(),
    )?)
}
/// Align normalized parameter partitions and degree without sampled fitting.
/// Discontinuous source curves are refused; orientation/correspondence is caller-owned.
pub fn compatible_profiles(profiles: &[Curve]) -> Result<(Vec<Curve>, Compatibility)> {
    check(
        (2..=128).contains(&profiles.len()),
        "loft requires 2..128 profiles",
    )?;
    let degree = profiles.iter().map(Curve::degree).max().unwrap();
    let normalized = profiles.iter().map(normalize).collect::<Result<Vec<_>>>()?;
    let mut breaks = normalized
        .iter()
        .flat_map(|c| c.knots().iter().copied())
        .filter(|t| *t > 0. && *t < 1.)
        .collect::<Vec<_>>();
    breaks.sort_by(f64::total_cmp);
    breaks.dedup();
    check(breaks.len() <= 512, "loft partition budget exceeded")?;
    for c in &normalized {
        for &t in &breaks {
            check(
                c.knots().iter().filter(|u| **u == t).count() <= c.degree(),
                "discontinuous loft profile",
            )?;
        }
    }
    let mut result = Vec::new();
    for c in normalized {
        let mut remainder = c;
        let mut segments = Vec::new();
        for &t in &breaks {
            let [left, right] = remainder.split(t)?;
            segments.push(elevate(&left, degree)?);
            remainder = right;
        }
        segments.push(elevate(&remainder, degree)?);
        let mut controls = Vec::new();
        let mut weights = Vec::new();
        let mut knots = vec![0.; degree + 1];
        for (i, s) in segments.iter().enumerate() {
            let first = usize::from(i > 0);
            controls.extend_from_slice(&s.controls()[first..]);
            weights.extend_from_slice(&s.weights()[first..]);
            if i < breaks.len() {
                knots.extend(vec![breaks[i]; degree]);
            }
        }
        knots.extend(vec![1.; degree + 1]);
        result.push(Curve::new(degree, knots, controls, weights)?);
    }
    Ok((
        result,
        Compatibility {
            source_domains: profiles.iter().map(Curve::domain).collect(),
            source_degrees: profiles.iter().map(Curve::degree).collect(),
            degree,
            spans: breaks.len() + 1,
            representation_continuity: "C0 basis; no loss of authored locus except binary64 roundoff",
        },
    ))
}
pub fn loft(profiles: &[Curve]) -> Result<CompatibleLoft> {
    let (profiles, report) = compatible_profiles(profiles)?;
    let surface = Surface::loft(&profiles)?;
    Ok(CompatibleLoft {
        surface,
        profiles,
        report,
    })
}
