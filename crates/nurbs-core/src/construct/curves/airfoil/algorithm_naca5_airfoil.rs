//! airfoil algorithm: naca5_airfoil.
use super::*;
/// Standard NACA 5-digit profile builder from a 5-digit integer code `LPQXX`
/// (e.g. `23012`: design lift digit `L = 2` -> $C_l = 0.3$, max-camber position
/// digits `PQ = 30` -> $p = 0.15$ when $Q = 0$, and thickness `XX = 12` -> $t = 0.12$).
pub fn naca5_airfoil(
    code: u32,
    chord: f64,
    closed_te: bool,
    samples: usize,
    degree: usize,
    controls: usize,
) -> Result<Curve> {
    check(
        (10000..=99999).contains(&code),
        "NACA 5-digit code must be LPQXX in 10000..=99999 (e.g. 23012)",
    )?;
    let l_digit = code / 10000;
    let pq = (code / 100) % 100;
    let q_digit = pq % 10;
    let p_digit = pq / 10;
    let xx = code % 100;
    check(
        q_digit == 0 && (1..=6).contains(&p_digit),
        "NACA 5-digit standard mean line requires Q=0 and P in 1..=6 (e.g. 23012)",
    )?;
    let position = p_digit as f64 * 0.05;
    // Standard NACA 230XX scales maximum camber by L / 2 relative to the L=2 (0.02 camber) baseline.
    let camber = l_digit as f64 * 0.01;
    let thickness = xx as f64 / 100.;
    check(
        (0.01..=0.4).contains(&thickness),
        "NACA 5-digit thickness must be in [0.01, 0.40]",
    )?;
    let mean = MeanLine::FiveDigit(naca5_camber(position, camber)?);
    let points = naca_points(&|x| mean.at(x), thickness, chord, closed_te, samples)?;
    fit_open_bspline(&points, degree, controls, 0., Some(closed_te))
}
