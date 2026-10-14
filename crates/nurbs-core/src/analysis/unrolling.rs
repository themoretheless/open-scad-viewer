//! Developability check and isometric unrolling of ruled/developable surfaces.
//!
//! `check_developable_report` samples Gaussian curvature |K| on a grid;
//! `unroll_ruled_report` flattens a sampled grid by exact edge lengths: each
//! quad cell is triangulated and placed in the plane by circle-circle
//! intersection, preserving generator lengths (u direction) and true 3D
//! distances between adjacent generators (v strips). This is an isometry only
//! when the surface is developable; the unroller refuses otherwise. Per-cell
//! area distortion (3D/2D − 1) is reported honestly.
use crate::{Result, check, numeric, surface::Surface};
use math_core::{next_up, norm};

#[derive(Clone, Debug)]
pub struct DevelopabilityReport {
    /// Max |K| over the sample grid, outward-rounded.
    pub max_gaussian_curvature: f64,
    /// |K| ≤ tolerance at every regular sample; false also when any sample is
    /// singular (developability cannot be confirmed there).
    pub is_developable: bool,
    pub samples: usize,
}

#[derive(Clone, Debug)]
pub struct UnrollReport {
    /// Flattened boundary: u=0 column, then v=vmax row, then u=umax column
    /// reversed — one loop (open along the v=vmin generator).
    pub profile: Vec<[f64; 2]>,
    /// Flattened interior grid points, row-major over (u, v).
    pub uv_grid: Vec<[f64; 2]>,
    /// Per-cell area ratio 3D/2D − 1 (triangulated).
    pub distortion: Vec<f64>,
    pub max_abs_distortion: f64,
}

fn domains(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}

/// Grid resolution from a cell budget: (n+1)² samples, n² cells.
fn resolution(max_cells: usize) -> Result<usize> {
    check(
        (4..=1_000_000).contains(&max_cells),
        "Developability budget must be 4..1000000 cells.",
    )?;
    let mut n = (max_cells as f64).sqrt() as usize;
    while (n + 1) * (n + 1) > max_cells {
        n -= 1;
    }
    Ok(n.max(2))
}

/// Samples |K| on a uniform grid; outward-rounded max.
pub fn check_developable_report(
    s: &Surface,
    tolerance: f64,
    max_cells: usize,
) -> Result<DevelopabilityReport> {
    s.validate()?;
    check(
        tolerance.is_finite() && tolerance >= 0.,
        "Developability tolerance must be finite and nonnegative.",
    )?;
    let n = resolution(max_cells)?;
    let d = domains(s);
    let mut max_k = 0_f64;
    let mut developable = true;
    let mut samples = 0;
    for i in 0..=n {
        for j in 0..=n {
            let u = d[0][0] + (d[0][1] - d[0][0]) * i as f64 / n as f64;
            let v = d[1][0] + (d[1][1] - d[1][0]) * j as f64 / n as f64;
            samples += 1;
            let e = s.evaluate_validated(u, v)?;
            let Some((k, _)) = e.curvatures() else {
                // Singular or C1 sample: developability is not confirmed.
                developable = false;
                continue;
            };
            max_k = max_k.max(k.abs());
            if k.abs() > tolerance {
                developable = false;
            }
        }
    }
    Ok(DevelopabilityReport {
        max_gaussian_curvature: next_up(max_k),
        is_developable: developable,
        samples,
    })
}

/// 3D distance between evaluated points.
fn distance3(a: [f64; 3], b: [f64; 3]) -> f64 {
    norm([a[0] - b[0], a[1] - b[1], a[2] - b[2]])
}

/// Circle-circle intersection: points at distance ra from a and rb from b.
/// With a reference point, returns the candidate farther from it — the
/// non-folding choice that keeps the strip oriented consistently; without
/// one, returns the candidate left of the directed edge a → b. The base
/// distance is clamped when the circles only nearly intersect; the mismatch
/// then surfaces honestly in the per-cell distortion.
fn place(a: [f64; 2], b: [f64; 2], ra: f64, rb: f64, reference: Option<[f64; 2]>) -> Result<[f64; 2]> {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let d = (dx * dx + dy * dy).sqrt();
    numeric(d > 0., "Unroll degenerate flattened edge.")?;
    let d = d.clamp((ra - rb).abs() * (1. + 1e-12), (ra + rb) * (1. - 1e-12));
    let along = (ra * ra - rb * rb + d * d) / (2. * d);
    let height = (ra * ra - along * along).max(0.).sqrt();
    let (ux, uy) = (dx / d, dy / d);
    let left = [a[0] + along * ux - height * uy, a[1] + along * uy + height * ux];
    let right = [a[0] + along * ux + height * uy, a[1] + along * uy - height * ux];
    let Some(r) = reference else {
        return Ok(left);
    };
    let dist = |p: [f64; 2]| (p[0] - r[0]).powi(2) + (p[1] - r[1]).powi(2);
    Ok(if dist(left) >= dist(right) { left } else { right })
}

/// Unrolls a ruled/developable surface by preserving generator lengths
/// (u direction) and true distances between adjacent generators (v strips
/// triangulated with exact edge lengths). Valid only when
/// `check_developable_report` passes; refuses otherwise.
pub fn unroll_ruled_report(s: &Surface, tolerance: f64, max_cells: usize) -> Result<UnrollReport> {
    let developable = check_developable_report(s, tolerance, max_cells)?;
    check(
        developable.is_developable,
        "Unroll refuses: surface is not developable within tolerance.",
    )?;
    let n = resolution(max_cells)?;
    let d = domains(s);
    let mut points = Vec::with_capacity((n + 1) * (n + 1));
    for i in 0..=n {
        for j in 0..=n {
            let u = d[0][0] + (d[0][1] - d[0][0]) * i as f64 / n as f64;
            let v = d[1][0] + (d[1][1] - d[1][0]) * j as f64 / n as f64;
            points.push(s.evaluate_validated(u, v)?.point);
        }
    }
    let at = |i: usize, j: usize| points[i * (n + 1) + j];
    let mut flat = vec![[0.; 2]; (n + 1) * (n + 1)];
    // First generator (v = vmin) along the x-axis, cumulative true lengths.
    let mut x = 0.;
    for i in 1..=n {
        x += distance3(at(i, 0), at(i - 1, 0));
        flat[i * (n + 1)] = [x, 0.];
    }
    // Each subsequent generator: first point from the diagonal triangle, then
    // each strip point from its two flattened neighbors with exact lengths.
    for j in 1..=n {
        // Reference for the non-folding choice: the previous point on this
        // generator (or the left rule for the very first strip point).
        let reference = (j >= 2).then(|| flat[j - 2]);
        flat[j] = place(
            flat[j - 1],
            flat[(n + 1) + (j - 1)],
            distance3(at(0, j), at(0, j - 1)),
            distance3(at(0, j), at(1, j - 1)),
            reference,
        )?;
        for i in 1..=n {
            flat[i * (n + 1) + j] = place(
                flat[i * (n + 1) + (j - 1)],
                flat[(i - 1) * (n + 1) + j],
                distance3(at(i, j), at(i, j - 1)),
                distance3(at(i, j), at(i - 1, j)),
                Some(flat[(i - 1) * (n + 1) + (j - 1)]),
            )?;
        }
    }
    // Per-cell area ratio 3D/2D − 1 over the two diagonal triangles.
    let triangle3 = |a: [f64; 3], b: [f64; 3], c: [f64; 3]| {
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        norm([
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ]) / 2.
    };
    let triangle2 = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| {
        ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() / 2.
    };
    let mut distortion = Vec::with_capacity(n * n);
    let mut max_abs = 0_f64;
    for i in 0..n {
        for j in 0..n {
            let a3 = triangle3(at(i, j), at(i + 1, j), at(i, j + 1))
                + triangle3(at(i + 1, j), at(i + 1, j + 1), at(i, j + 1));
            let f = |i: usize, j: usize| flat[i * (n + 1) + j];
            let a2 = triangle2(f(i, j), f(i + 1, j), f(i, j + 1))
                + triangle2(f(i + 1, j), f(i + 1, j + 1), f(i, j + 1));
            let ratio = if a2 > 0. { a3 / a2 - 1. } else { f64::INFINITY };
            distortion.push(ratio);
            max_abs = max_abs.max(ratio.abs());
        }
    }
    numeric(
        max_abs.is_finite(),
        "Unroll produced a degenerate flattened cell.",
    )?;
    // Boundary loop: u=0 column, v=vmax row, u=umax column reversed.
    let mut profile = Vec::with_capacity(3 * (n + 1));
    for j in 0..=n {
        profile.push(flat[j]);
    }
    for i in 1..=n {
        profile.push(flat[i * (n + 1) + n]);
    }
    for j in (0..n).rev() {
        profile.push(flat[n * (n + 1) + j]);
    }
    Ok(UnrollReport {
        profile,
        uv_grid: flat,
        distortion,
        max_abs_distortion: next_up(max_abs),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::Curve;
    use crate::surface::revolve;

    fn cylinder() -> Surface {
        let profile = Curve::from_polyline(vec![vec![1., 0., 0.], vec![1., 0., 1.]]).unwrap();
        revolve(&profile, [0.; 3], [0., 0., 1.], 360.).unwrap()
    }
    fn frustum() -> Surface {
        // Truncated cone: radius 1 at z=0 to radius 0.5 at z=1.
        let profile = Curve::from_polyline(vec![vec![1., 0., 0.], vec![0.5, 0., 1.]]).unwrap();
        revolve(&profile, [0.; 3], [0., 0., 1.], 360.).unwrap()
    }
    fn sphere() -> Surface {
        let profile = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 2., 2., 2.],
            control_points: vec![
                vec![0., 0., -1.],
                vec![1., 0., -1.],
                vec![1., 0., 0.],
                vec![1., 0., 1.],
                vec![0., 0., 1.],
            ],
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            periodic: false,
        };
        revolve(&profile, [0.; 3], [0., 0., 1.], 360.).unwrap()
    }

    #[test]
    fn cylinder_is_developable_and_unrolls_to_rectangle() {
        let s = cylinder();
        let check = check_developable_report(&s, 1e-9, 10000).unwrap();
        assert!(check.is_developable, "K max {}", check.max_gaussian_curvature);
        assert!(check.max_gaussian_curvature < 1e-9);
        let report = unroll_ruled_report(&s, 1e-9, 10000).unwrap();
        assert!(report.max_abs_distortion < 1e-8, "distortion {}", report.max_abs_distortion);
        // Rectangle: generators (u, height) along x, strips (v) straight up.
        let n = (report.uv_grid.len() as f64).sqrt() as usize - 1;
        let height = report.uv_grid[n * (n + 1)][0];
        assert!((height - 1.).abs() < 1e-12, "height {height}");
        // Flattened width of the v strip = n chords ≈ circumference 2π.
        let width = report.uv_grid[n][1] - report.uv_grid[0][1];
        let circumference = 2. * std::f64::consts::PI;
        assert!((width - circumference).abs() < 0.01, "width {width}");
        // Straight strip: x stays 0 along the first generator strip.
        for j in 0..=n {
            assert!(report.uv_grid[j][0].abs() < 1e-9, "{:?}", report.uv_grid[j]);
        }
    }

    #[test]
    fn frustum_unrolls_to_annular_sector_with_arc_length() {
        let s = frustum();
        let check = check_developable_report(&s, 1e-9, 10000).unwrap();
        assert!(check.is_developable, "K max {}", check.max_gaussian_curvature);
        let report = unroll_ruled_report(&s, 1e-9, 10000).unwrap();
        assert!(report.max_abs_distortion < 1e-8, "distortion {}", report.max_abs_distortion);
        // Outer boundary arc length (u=0 row of the profile loop) ≈ 2π·R.
        let n = (report.uv_grid.len() as f64).sqrt() as usize - 1;
        let outer: f64 = (1..=n)
            .map(|j| {
                let a = report.profile[j - 1];
                let b = report.profile[j];
                ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt()
            })
            .sum();
        assert!((outer - 2. * std::f64::consts::PI).abs() < 0.01, "arc {outer}");
        // Inner radius 0.5: arc length ≈ π (u=umax column, reversed).
        let base = 2 * n + 1;
        let inner: f64 = (1..n)
            .map(|j| {
                let a = report.profile[base + j - 1];
                let b = report.profile[base + j];
                ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt()
            })
            .sum();
        let expected_inner = std::f64::consts::PI * (n - 1) as f64 / n as f64;
        assert!((inner - expected_inner).abs() < 0.005, "inner arc {inner}");
    }

    #[test]
    fn sphere_is_not_developable_and_unroll_refuses() {
        let s = sphere();
        let check = check_developable_report(&s, 1e-6, 10000).unwrap();
        assert!(!check.is_developable);
        assert!(check.max_gaussian_curvature > 0.9, "K {}", check.max_gaussian_curvature);
        assert!(unroll_ruled_report(&s, 1e-6, 10000).is_err());
    }
}
