//! Standard nonreflex five-digit profiles from NACA Report 537, Table I.
use super::*;

// Nominal published constants at design lift coefficient 0.3 (L=2).
// These are NOT maximum-camber values. Custom max-camber designs remain
// available separately through naca5(position, camber, ...).
fn standard_mean(code: u32) -> Result<(Naca5Camber, f64)> {
    check((10000..=99999).contains(&code),
        "NACA 5-digit code must be LPQXX in 10000..=99999")?;
    let l_digit = code / 10000;
    let p_digit = (code / 1000) % 10;
    let q_digit = (code / 100) % 10;
    check(q_digit == 0 && (1..=5).contains(&p_digit),
        "Standard nonreflex NACA mean line requires Q=0 and P in 1..=5")?;
    let thickness = (code % 100) as f64 / 100.;
    check((0.01..=0.4).contains(&thickness),
        "NACA 5-digit thickness must be in [0.01, 0.40]")?;
    let (transition, baseline_k1) = match p_digit {
        1 => (0.0580, 361.4),
        2 => (0.1260, 51.64),
        3 => (0.2025, 15.957),
        4 => (0.2900, 6.643),
        5 => (0.3910, 3.230),
        _ => unreachable!(),
    };
    Ok((Naca5Camber { transition, k1: baseline_k1 * l_digit as f64 / 2., iterations: 0 }, thickness))
}

/// Nominal analytic upper/lower coordinates of a standard nonreflex LP0XX
/// profile. Published mean-line constants are scaled by L/2; the design
/// lift digit is not interpreted as maximum camber. No aerodynamic proof
/// or continuous error certificate is returned.
pub fn naca5_section_at(
    code: u32,
    x: f64,
    chord: f64,
    closed_te: bool,
) -> Result<([f64; 3], [f64; 3])> {
    require_finite_f64(x, "naca5_x")?;
    require_finite_f64(chord, "naca5_chord")?;
    check((0. ..=1.).contains(&x), "NACA station x must be in [0, 1]")?;
    check(chord > 0. && chord <= 1e6, "Airfoil chord must be in (0, 1e6]")?;
    let (mean, thickness) = standard_mean(code)?;
    let (yc, slope) = naca5_camber_line(x, &mean);
    let yt = naca4_half_thickness(x, thickness, closed_te);
    let denominator = 1_f64.hypot(slope);
    let sin = slope / denominator;
    let cos = 1. / denominator;
    Ok((
        [chord * (x - yt * sin), chord * (yc + yt * cos), 0.],
        [chord * (x + yt * sin), chord * (yc - yt * cos), 0.],
    ))
}

/// Standard nonreflex NACA LP0XX profile fitted to sampled analytic sites.
/// P is 1..=5 and L is 1..=9. Uses nominal NACA Report 537 coefficients.
/// Degree/control selection governs a numerical least-squares fit; there
/// is no continuous fitting bound. Reflex mean lines are explicitly refused.
pub fn naca5_airfoil(
    code: u32,
    chord: f64,
    closed_te: bool,
    samples: usize,
    degree: usize,
    controls: usize,
) -> Result<Curve> {
    let (constants, thickness) = standard_mean(code)?;
    let mean = MeanLine::FiveDigit(constants);
    let points = naca_points(&|x| mean.at(x), thickness, chord, closed_te, samples)?;
    fit_open_bspline(&points, degree, controls, 0., Some(closed_te))
}
