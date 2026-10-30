//! airfoil algorithm: naca_points.
use super::*;
/// Ordered profile point loop (TE -> upper -> LE -> lower -> TE) for a
/// NACA mean line plus the standard 4-digit thickness distribution.
/// `count` is the number of x stations per surface; x stations cluster
/// at the nose with spacing proportional to sqrt(x/c).
pub fn naca_points(
    mean: &dyn Fn(f64) -> (f64, f64),
    thickness: f64,
    chord: f64,
    closed_te: bool,
    count: usize,
) -> Result<Vec<[f64; 3]>> {
    check(
        (8..=2048).contains(&count),
        "Airfoil sampling needs 8..2048 stations per surface",
    )?;
    check(
        thickness.is_finite() && (0.01..=0.4).contains(&thickness),
        "Airfoil thickness fraction must be in [0.01, 0.4]",
    )?;
    check(
        chord.is_finite() && chord > 0. && chord <= 1e6,
        "Airfoil chord must be finite and in (0, 1e6]",
    )?;
    let mut points = Vec::with_capacity(2 * count + 1);
    let station = |i: usize| {
        let s = i as f64 / count as f64;
        s * s
    };
    let place = |x: f64, upper: bool| -> Result<[f64; 3]> {
        let (yc, dyc) = mean(x);
        let yt = naca4_half_thickness(x, thickness, closed_te);
        check(
            yc.is_finite() && dyc.is_finite() && yt.is_finite() && yt >= -1e-12,
            "Airfoil ordinate is not finite",
        )?;
        let theta = dyc.atan();
        let (sin, cos) = theta.sin_cos();
        let point = if upper {
            [chord * (x - yt * sin), chord * (yc + yt * cos), 0.]
        } else {
            [chord * (x + yt * sin), chord * (yc - yt * cos), 0.]
        };
        check(
            point.iter().all(|v| v.is_finite()),
            "Airfoil sample is not finite",
        )?;
        Ok(point)
    };
    for i in (0..=count).rev() {
        points.push(place(station(i), true)?);
    }
    for i in 1..=count {
        points.push(place(station(i), false)?);
    }
    Ok(points)
}
