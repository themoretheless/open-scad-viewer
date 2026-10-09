//! Airfoil profile constructors: NACA 4/5-digit parametric profiles, CST
//! (class function / shape function) parameterization with a Tikhonov-
//! regularized inverse, profile fitting with leading-edge radius and
//! trailing-edge closure constraints, and curvature-quality analysis.
//!
//! Conventions: chord along +X with the leading edge at x=0 and the
//! trailing edge at x=chord, thickness along Y, Z=0. Profile loops are
//! ordered trailing edge -> upper surface -> leading edge -> lower
//! surface -> trailing edge. Sampling in x is clustered at the nose with
//! spacing proportional to sqrt(x/c) (x_i = (i/N)^2).
use crate::curve::basis;
use crate::foundation::guards::{
    Budget, assert_finite_slice_debug, require_finite_f64, require_finite_slice,
};
use crate::{Result, check, curve::Curve, numeric_err};
use math_core::next_up;

/// Maximum bisection + Newton refinement steps for the internal 5-digit
/// camber-line solver.
const SOLVER_BUDGET: usize = 96;

/// NACA 4-digit half-thickness distribution (chord fraction coordinates).
/// `closed_te` selects the -0.1036 x^4 coefficient (zero trailing-edge
/// thickness); `!closed_te` selects the original -0.1015 coefficient.
pub fn naca4_half_thickness(x: f64, thickness: f64, closed_te: bool) -> f64 {
    let a4 = if closed_te { -0.1036 } else { -0.1015 };
    5. * thickness
        * (0.2969 * x.sqrt() - 0.1260 * x - 0.3516 * x * x + 0.2843 * x * x * x
            + a4 * x * x * x * x)
}

/// NACA 4-digit mean camber line: returns (y_c, dy_c/dx), piecewise
/// quadratic in x with the kink at `position`.
pub fn naca4_camber(x: f64, camber: f64, position: f64) -> (f64, f64) {
    if camber == 0. || position <= 0. || position >= 1. {
        return (0., 0.);
    }
    if x < position {
        (
            camber / (position * position) * (2. * position * x - x * x),
            2. * camber / (position * position) * (position - x),
        )
    } else {
        let q = 1. - position;
        (
            camber / (q * q) * ((1. - 2. * position) + 2. * position * x - x * x),
            2. * camber / (q * q) * (position - x),
        )
    }
}

/// NACA 5-digit mean line parameters solved from the design position of
/// maximum camber. The mean line is a cubic for x < m and a straight line
/// for x >= m; m is the root of 3 p^2 - 6 m p + m^2 (3 - m) = 0 in (p, 1)
/// (maximum of the cubic branch at x = p) and k1 scales the line to the
/// requested maximum camber at x = p.
#[derive(Clone, Copy, Debug)]
pub struct Naca5Camber {
    /// Cubic/linear transition station (chord fraction).
    pub transition: f64,
    /// Cubic scale factor k1.
    pub k1: f64,
    /// Solver iterations actually used.
    pub iterations: usize,
}

/// Solve the 5-digit mean-line constants for a design max-camber position
/// `position` (chord fraction, 0.05..0.3) and max camber `camber`.
/// Bracketing bisection followed by Newton polish under SOLVER_BUDGET.
pub fn naca5_camber(position: f64, camber: f64) -> Result<Naca5Camber> {
    require_finite_f64(position, "naca5_position")?;
    require_finite_f64(camber, "naca5_camber")?;
    check(
        (0.05..=0.3).contains(&position),
        "NACA 5-digit camber position must be a chord fraction in [0.05, 0.3]",
    )?;
    check(
        camber > 0. && camber <= 0.06,
        "NACA 5-digit camber must be in (0, 0.06]",
    )?;
    let p = position;
    let f = |m: f64| 3. * p * p - 6. * m * p + m * m * (3. - m);
    let df = |m: f64| -6. * p + 6. * m - 3. * m * m;
    // f(p) = -p^3 < 0 and f(1) = 3p^2 - 6p + 2 > 0 for p in [0.05, 0.3];
    // f' = -3((m-1)^2 + 2p - 1) > 0 on (p, 1), so the root is unique.
    let mut solver_budget =
        Budget::with_iterations(2 * SOLVER_BUDGET)?.guard("naca5-camber-k1-solver");
    let mut lo = p;
    let mut hi = 1.;
    let mut iterations = 0_usize;
    let mut m = 0.5 * (lo + hi);
    for step in 0..SOLVER_BUDGET {
        solver_budget.tick()?;
        iterations = step + 1;
        m = 0.5 * (lo + hi);
        if f(m) > 0. {
            hi = m;
        } else {
            lo = m;
        }
        if hi - lo < 1e-14 {
            break;
        }
    }
    // Newton polish from the bisection midpoint.
    for step in 0..SOLVER_BUDGET {
        solver_budget.tick()?;
        let d = df(m);
        if d.abs() < 1e-300 {
            break;
        }
        let next = m - f(m) / d;
        if !next.is_finite() || next <= p || next >= 1. {
            break;
        }
        let delta = (next - m).abs();
        m = next;
        iterations += step + 1;
        if delta < 1e-15 {
            break;
        }
    }
    crate::numeric(
        (f(m)).abs() < 1e-8,
        "NACA 5-digit camber solver did not converge within budget",
    )?;
    let shape = p * p * p - 3. * m * p * p + m * m * (3. - m) * p;
    crate::numeric(
        shape.is_finite() && shape.abs() > 1e-12,
        "NACA 5-digit camber shape factor is degenerate",
    )?;
    let k1 = 6. * camber / shape;
    Ok(Naca5Camber {
        transition: m,
        k1,
        iterations,
    })
}

/// NACA 5-digit mean camber line given solved constants: (y_c, dy_c/dx).
pub fn naca5_camber_line(x: f64, constants: &Naca5Camber) -> (f64, f64) {
    let m = constants.transition;
    let k1 = constants.k1;
    if x < m {
        (
            k1 / 6. * (x * x * x - 3. * m * x * x + m * m * (3. - m) * x),
            k1 / 6. * (3. * x * x - 6. * m * x + m * m * (3. - m)),
        )
    } else {
        (k1 * m * m * m / 6. * (1. - x), -k1 * m * m * m / 6.)
    }
}

/// Analytic profile sample source shared by the NACA constructors.
enum MeanLine {
    FourDigit { camber: f64, position: f64 },
}

impl MeanLine {
    fn at(&self, x: f64) -> (f64, f64) {
        match self {
            MeanLine::FourDigit { camber, position } => naca4_camber(x, *camber, *position),
        }
    }
}

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

fn mean_line_4(camber: f64, position: f64) -> Result<MeanLine> {
    check(
        camber.is_finite() && (0. ..=0.095).contains(&camber),
        "NACA camber fraction must be in [0, 0.095]",
    )?;
    check(
        camber == 0. || (position.is_finite() && (0.1..=0.9).contains(&position)),
        "NACA camber position must be in [0.1, 0.9] for cambered profiles",
    )?;
    Ok(MeanLine::FourDigit { camber, position })
}

/// NACA 4-digit profile (MPXX `code`, e.g. 12 for 0012, 2412) as a single
/// B-spline curve fitted through the clustered analytic loop. `degree` and
/// `controls` steer the fit; knot placement follows the data parameters so
/// knots cluster at the nose automatically.
pub fn naca4(
    code: u32,
    chord: f64,
    closed_te: bool,
    samples: usize,
    degree: usize,
    controls: usize,
) -> Result<Curve> {
    check(
        (1..=9999).contains(&code) && code / 1000 <= 9,
        "NACA 4-digit code must be MPXX (e.g. 0012 passed as 12)",
    )?;
    let camber = (code / 1000) as f64 / 100.;
    let position = ((code / 100) % 10) as f64 / 10.;
    let thickness = (code % 100) as f64 / 100.;
    check(
        thickness >= 0.01,
        "NACA thickness digits must encode at least 1 percent",
    )?;
    let mean = mean_line_4(camber, position)?;
    let closure = |x: f64| mean.at(x);
    let points = naca_points(&closure, thickness, chord, closed_te, samples)?;
    fit_open_bspline(&points, degree, controls, 0., None)
}

/// NACA 5-digit-style profile from the design max-camber position and
/// camber (the k1 constant is solved internally, see `naca5_camber`).
pub fn naca5(
    position: f64,
    camber: f64,
    thickness: f64,
    chord: f64,
    closed_te: bool,
    samples: usize,
    degree: usize,
    controls: usize,
) -> Result<Curve> {
    let constants = naca5_camber(position, camber)?;
    let closure = |x: f64| naca5_camber_line(x, &constants);
    let points = naca_points(&closure, thickness, chord, closed_te, samples)?;
    fit_open_bspline(&points, degree, controls, 0., None)
}

mod cst;
pub use cst::{CstFitResult, CstProfile, cst_fit, cst_points, cst_shape};

/// Solve (A^T A + lambda I) X = A^T B by Gaussian elimination with partial
/// pivoting. `design` rows are samples, columns are unknowns; `targets`
/// share the row count. Returns one coefficient row per unknown.
fn lsq_tikhonov(design: &[Vec<f64>], targets: &[Vec<f64>], lambda: f64) -> Result<Vec<Vec<f64>>> {
    let columns = design[0].len();
    let width = targets[0].len();
    let mut matrix = vec![vec![0.; columns]; columns];
    let mut rhs = vec![vec![0.; width]; columns];
    for (row, target) in design.iter().zip(targets) {
        for i in 0..columns {
            for j in 0..=i {
                matrix[i][j] += row[i] * row[j];
            }
            for (k, r) in rhs[i].iter_mut().enumerate() {
                *r += row[i] * target[k];
            }
        }
    }
    for i in 0..columns {
        for j in 0..i {
            matrix[j][i] = matrix[i][j];
        }
        matrix[i][i] += lambda;
    }
    // Gaussian elimination with partial pivoting.
    for column in 0..columns {
        let mut pivot = column;
        for row in column + 1..columns {
            if matrix[row][column].abs() > matrix[pivot][column].abs() {
                pivot = row;
            }
        }
        if matrix[pivot][column].abs() <= 1e-14 * (1. + matrix[column][column].abs()) {
            return Err(numeric_err("Least-squares system is rank deficient"));
        }
        matrix.swap(column, pivot);
        rhs.swap(column, pivot);
        let divisor = matrix[column][column];
        for r in column + 1..columns {
            let factor = matrix[r][column] / divisor;
            if factor == 0. {
                continue;
            }
            for c in column..columns {
                matrix[r][c] -= factor * matrix[column][c];
            }
            let source = rhs[column].clone();
            for (value, pivot_value) in rhs[r].iter_mut().zip(source) {
                *value -= factor * pivot_value;
            }
        }
    }
    let mut result = vec![vec![0.; width]; columns];
    for column in (0..columns).rev() {
        for k in 0..width {
            let mut sum = rhs[column][k];
            for c in column + 1..columns {
                sum -= matrix[column][c] * result[c][k];
            }
            result[column][k] = sum / matrix[column][column];
        }
    }
    Ok(result)
}

/// Chordal parameters of an ordered point loop in [0, 1].
fn chord_parameters(points: &[[f64; 3]]) -> Vec<f64> {
    let mut parameters = vec![0.];
    for pair in points.windows(2) {
        let step = (0..3)
            .map(|k| pair[1][k] - pair[0][k])
            .map(|d| d * d)
            .sum::<f64>()
            .sqrt();
        parameters.push(parameters.last().unwrap() + step);
    }
    let total = *parameters.last().unwrap();
    if total == 0. {
        (0..points.len())
            .map(|i| i as f64 / (points.len() - 1) as f64)
            .collect()
    } else {
        parameters.into_iter().map(|v| v / total).collect()
    }
}

/// Clamped knot vector from data parameters by parameter averaging; knots
/// cluster where the data cluster (the nose for sqrt-clustered sampling).
fn average_knots(parameters: &[f64], degree: usize, controls: usize) -> Result<Vec<f64>> {
    let interior = controls - degree - 1;
    let mut knots = std::iter::repeat_n(0., degree + 1).collect::<Vec<_>>();
    let data = parameters.len();
    let d = (data - 1) as f64 / (controls - degree) as f64;
    for j in 1..=interior {
        let position = j as f64 * d;
        let i = (position.floor() as usize).clamp(1, data - 2);
        let alpha = (position - i as f64).clamp(0., 1.);
        let knot = (1. - alpha) * parameters[i - 1] + alpha * parameters[i];
        check(
            knot > *knots.last().unwrap() && knot < 1.,
            "Airfoil fit knot averaging collapsed distinct knots",
        )?;
        knots.push(knot);
    }
    knots.extend(std::iter::repeat_n(1., degree + 1));
    Ok(knots)
}

/// Least-squares B-spline fit of an ordered 3D point loop with an optional
/// second-difference smoothing penalty `smooth` (weight on
/// c_{i+1} - 2 c_i + c_{i-1}) and optional trailing-edge closure
/// (last control point pinned to the first after the solve).
fn fit_open_bspline(
    points: &[[f64; 3]],
    degree: usize,
    controls: usize,
    smooth: f64,
    close_te: Option<bool>,
) -> Result<Curve> {
    check(
        (1..=25).contains(&degree),
        "Airfoil fit degree must be 1..=25",
    )?;
    check(
        controls >= degree + 1 && controls + 2 <= points.len() && controls <= 64,
        "Airfoil fit control count must be degree+1..=64 and below the site count",
    )?;
    check(
        points.len() <= 4096,
        "Airfoil fit data must have at most 4096 sites",
    )?;
    require_finite_slice(
        &points.iter().flatten().copied().collect::<Vec<_>>(),
        "airfoil_fit_data",
    )?;
    check(
        smooth.is_finite() && smooth >= 0.,
        "Smoothing weight must be non-negative and finite",
    )?;
    let parameters = chord_parameters(points);
    let knots = average_knots(&parameters, degree, controls)?;
    let mut design = Vec::with_capacity(points.len() + controls);
    let mut target = Vec::with_capacity(points.len() + controls);
    for (&u, point) in parameters.iter().zip(points) {
        design.push(basis(degree, &knots, controls, u, false)?.basis);
        target.push(point.to_vec());
    }
    if smooth > 0. {
        for i in 1..controls - 1 {
            let mut row = vec![0.; controls];
            row[i - 1] = smooth;
            row[i] = -2. * smooth;
            row[i + 1] = smooth;
            design.push(row);
            target.push(vec![0.; 3]);
        }
    }
    let solved = lsq_tikhonov(&design, &target, 1e-14)?;
    let mut control_points: Vec<Vec<f64>> = solved[..controls].to_vec();
    if close_te == Some(true) {
        let first = control_points[0].clone();
        *control_points.last_mut().unwrap() = first;
    }
    // Result boundary: never hand back a non-finite control net.
    for point in &control_points {
        assert_finite_slice_debug(point, "airfoil_fit_control_points")?;
    }
    let curve = Curve {
        degree,
        knots,
        control_points,
        weights: vec![1.; controls],
        periodic: false,
    };
    curve.validate()?;
    Ok(curve)
}

/// Planar curvature of a curve evaluation: |d1 x d2| / |d1|^3.
fn curvature_at(curve: &Curve, u: f64) -> Result<f64> {
    let evaluation = curve.evaluate(u)?;
    let (Some(d1), Some(d2)) = (evaluation.d1, evaluation.d2) else {
        return Err(numeric_err("Curvature needs first and second derivatives"));
    };
    check(d1.len() == 3 && d2.len() == 3, "Curvature needs a 3D curve")?;
    let cross = [
        d1[1] * d2[2] - d1[2] * d2[1],
        d1[2] * d2[0] - d1[0] * d2[2],
        d1[0] * d2[1] - d1[1] * d2[0],
    ];
    let speed2 = d1[0] * d1[0] + d1[1] * d1[1] + d1[2] * d1[2];
    crate::numeric(speed2 > 1e-24, "Curvature at a stationary point is undefined")?;
    let numerator = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
    Ok(numerator / (speed2 * speed2.sqrt()))
}

/// Leading-edge radius of a profile curve: 1/kappa at the point of minimal
/// x (the nose), located by golden-section refinement of |x(u)| minimum.
fn nose_radius(curve: &Curve) -> Result<f64> {
    let [a, b] = curve.domain();
    let x_of = |u: f64| -> Result<f64> { Ok(curve.evaluate(u)?.point[0]) };
    // Coarse scan then golden-section refinement around the best sample.
    let mut best_u = a;
    let mut best_x = f64::INFINITY;
    for i in 0..=256 {
        let u = a + (b - a) * i as f64 / 256.;
        let x = x_of(u)?;
        if x < best_x {
            best_x = x;
            best_u = u;
        }
    }
    let span = (b - a) / 256.;
    let (mut lo, mut hi) = ((best_u - span).max(a), (best_u + span).min(b));
    let inv_phi = 0.6180339887498949;
    let (mut c, mut d) = (hi - inv_phi * (hi - lo), lo + inv_phi * (hi - lo));
    let (mut fc, mut fd) = (x_of(c)?, x_of(d)?);
    let mut golden_budget = Budget::with_iterations(64)?.guard("nose-radius-golden-section");
    for _ in 0..64 {
        golden_budget.tick()?;
        if fc < fd {
            hi = d;
            d = c;
            fd = fc;
            c = hi - inv_phi * (hi - lo);
            fc = x_of(c)?;
        } else {
            lo = c;
            c = d;
            fc = fd;
            d = lo + inv_phi * (hi - lo);
            fd = x_of(d)?;
        }
    }
    let u_nose = 0.5 * (lo + hi);
    let kappa = curvature_at(curve, u_nose)?;
    crate::numeric(kappa > 1e-12, "Profile nose curvature is degenerate")?;
    Ok(1. / kappa)
}

/// Options for constraint-aware profile fitting.
#[derive(Clone, Copy, Debug)]
pub struct AirfoilFitOptions {
    pub degree: usize,
    pub controls: usize,
    /// Lower bound on the leading-edge radius of curvature (absolute units).
    pub min_nose_radius: f64,
    /// Pin the trailing edge closed (last control point = first).
    pub close_te: bool,
    /// Budget of smoothing escalation rounds (<= 32).
    pub max_iterations: usize,
}

/// Profile fitting report.
#[derive(Clone, Debug)]
pub struct AirfoilFitReport {
    pub curve: Curve,
    /// Achieved leading-edge radius of curvature.
    pub nose_radius: f64,
    /// Smoothing escalation rounds actually used.
    pub iterations: usize,
    /// Max distance from data sites to the fitted curve at data parameters.
    pub max_residual: f64,
}

/// Fit a B-spline to a profile point cloud with a leading-edge radius
/// constraint and optional trailing-edge closure. The radius constraint is
/// enforced by escalating a second-difference smoothing penalty
/// (geometric x8) until the nose radius clears the bound or the iteration
/// budget is exhausted; failure to clear the bound is an error.
pub fn fit_airfoil(points: &[[f64; 3]], options: &AirfoilFitOptions) -> Result<AirfoilFitReport> {
    check(
        options.min_nose_radius.is_finite() && options.min_nose_radius >= 0.,
        "Minimum nose radius must be non-negative and finite",
    )?;
    check(
        options.max_iterations <= 32,
        "Fit smoothing budget is at most 32 rounds",
    )?;
    let mut smooth = 1e-10;
    let mut best: Option<(Curve, f64)> = None;
    let mut iterations = 0_usize;
    let mut escalation = Budget::with_iterations(options.max_iterations + 1)?
        .guard("airfoil-smoothing-escalation");
    for round in 0..=options.max_iterations {
        escalation.tick()?;
        iterations = round;
        let curve = fit_open_bspline(points, options.degree, options.controls, smooth, None)?;
        let radius = nose_radius(&curve)?;
        best = Some((curve, radius));
        if radius >= options.min_nose_radius {
            break;
        }
        smooth *= 8.;
    }
    let (mut curve, radius) = best.unwrap();
    crate::numeric(
        radius >= options.min_nose_radius,
        "Nose radius constraint is unreachable within the smoothing budget",
    )?;
    if options.close_te {
        let first = curve.control_points[0].clone();
        *curve.control_points.last_mut().unwrap() = first;
        curve.validate()?;
    }
    let parameters = chord_parameters(points);
    let max_residual = parameters
        .iter()
        .zip(points)
        .map(|(&u, p)| {
            let q = curve.evaluate(u)?.point;
            Ok((0..3)
                .map(|k| q[k] - p[k])
                .map(|d| d * d)
                .sum::<f64>()
                .sqrt())
        })
        .collect::<Result<Vec<f64>>>()?
        .into_iter()
        .fold(0., f64::max);
    for point in &curve.control_points {
        assert_finite_slice_debug(point, "airfoil_fit_result_control_points")?;
    }
    Ok(AirfoilFitReport {
        curve,
        nose_radius: radius,
        iterations,
        max_residual: next_up(max_residual),
    })
}

/// Curvature-quality report for a profile curve.
#[derive(Clone, Debug)]
pub struct CurvatureQuality {
    pub samples: usize,
    /// Total variation of curvature along the arc, sum |kappa_{i+1} - kappa_i|
    /// (discrete integral of |d kappa / ds|).
    pub curvature_variation: f64,
    /// Sign changes of the discrete second difference of kappa (waviness).
    pub second_difference_sign_changes: usize,
    pub max_curvature: f64,
    pub min_radius: f64,
    pub arc_length: f64,
}

/// Curvature-quality analysis: samples kappa along the curve and reports
/// the total curvature variation plus the number of sign changes of the
/// discrete curvature second difference (fairness / waviness indicator).
pub fn analyze_curvature(curve: &Curve, samples: usize) -> Result<CurvatureQuality> {
    curve.validate()?;
    check(
        (32..=4096).contains(&samples),
        "Curvature analysis needs 32..4096 samples",
    )?;
    let [a, b] = curve.domain();
    let mut kappa = Vec::with_capacity(samples);
    let mut previous: Option<[f64; 3]> = None;
    let mut arc_length = 0.;
    for i in 0..samples {
        let u = a + (b - a) * i as f64 / (samples - 1) as f64;
        let point = curve.evaluate(u)?.point;
        if let Some(p) = previous {
            arc_length += (0..3)
                .map(|k| point[k] - p[k])
                .map(|d| d * d)
                .sum::<f64>()
                .sqrt();
        }
        previous = Some([point[0], point[1], point[2]]);
        kappa.push(curvature_at(curve, u)?);
    }
    let variation = kappa
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .sum::<f64>();
    let mut sign_changes = 0_usize;
    let mut last_sign = 0_i32;
    for w in kappa.windows(3) {
        let second = w[2] - 2. * w[1] + w[0];
        let scale = w.iter().fold(0_f64, |m, v| m.max(v.abs())).max(1e-300);
        // Deadband: ignore sub-per-mille wiggles of the local curvature
        // level; a genuine waviness reversal swings well past it.
        let sign = if second > 1e-3 * scale {
            1
        } else if second < -1e-3 * scale {
            -1
        } else {
            0
        };
        if sign != 0 {
            if last_sign != 0 && sign != last_sign {
                sign_changes += 1;
            }
            last_sign = sign;
        }
    }
    let max_curvature = kappa.iter().copied().fold(0., f64::max);
    let min_radius = if max_curvature > 1e-300 {
        1. / max_curvature
    } else {
        f64::INFINITY
    };
    Ok(CurvatureQuality {
        samples,
        curvature_variation: variation,
        second_difference_sign_changes: sign_changes,
        max_curvature,
        min_radius,
        arc_length,
    })
}

#[cfg(test)]
#[path = "tests/airfoil.rs"]
mod tests;
