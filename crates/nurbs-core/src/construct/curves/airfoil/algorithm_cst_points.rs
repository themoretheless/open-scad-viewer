//! airfoil algorithm: cst_points.
use super::*;
/// Ordered point loop of a CST profile (upper then lower, nose clustered).
pub fn cst_points(profile: &CstProfile, chord: f64, count: usize) -> Result<Vec<[f64; 3]>> {
    check(
        profile.n1.is_finite() && profile.n2.is_finite() && profile.n1 > 0. && profile.n2 > 0.,
        "CST class exponents must be positive and finite",
    )?;
    check(
        (2..=12).contains(&profile.upper.len()) && profile.upper.len() == profile.lower.len(),
        "CST coefficient orders must match in 2..=12 per surface",
    )?;
    require_finite_slice(&profile.upper, "cst_upper")?;
    require_finite_slice(&profile.lower, "cst_lower")?;
    check(
        profile
            .upper
            .iter()
            .chain(&profile.lower)
            .all(|x| x.abs() <= 1.),
        "CST coefficients must satisfy |a| <= 1",
    )?;
    check(
        (8..=2048).contains(&count) && chord.is_finite() && chord > 0.,
        "CST sampling needs 8..2048 stations and a positive chord",
    )?;
    let mut points = Vec::with_capacity(2 * count + 1);
    let station = |i: usize| {
        let s = i as f64 / count as f64;
        s * s
    };
    for i in (0..=count).rev() {
        let x = station(i);
        points.push([chord * x, chord * cst_shape(x, profile.n1, profile.n2, &profile.upper), 0.]);
    }
    for i in 1..=count {
        let x = station(i);
        points.push([chord * x, chord * cst_shape(x, profile.n1, profile.n2, &profile.lower), 0.]);
    }
    Ok(points)
}
