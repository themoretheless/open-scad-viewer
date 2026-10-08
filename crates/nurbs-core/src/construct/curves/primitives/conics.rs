//! Exact rational conic constructions: circles, ellipses, parabolas, hyperbolas,
//! rho-form and osculating conics. Angles are in degrees.
use crate::{Result, check, curve::Curve};
use std::f64::consts::PI;

/// Circle in the plane through center, oriented by normal. Domain [0,1].
pub fn circle(center: [f64; 3], normal: [f64; 3], radius: f64) -> Result<Curve> {
    circle_arc(center, normal, radius, 0., 360.)
}

/// Four counterclockwise rational quadratic arcs in the XY plane.
/// Signed radii retain their authored phase; each arc has domain [0, 1].
pub fn circle_quadrants(radius: f64) -> Result<[Curve; 4]> {
    if radius == 0. {
        return Ok(std::array::from_fn(|_| Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0.; 3]; 3],
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            periodic: false,
        }));
    }
    let arcs = (0..4)
        .map(|i| {
            ellipse_arc(
                [0.; 3],
                [radius, 0., 0.],
                [0., radius, 0.],
                i as f64 * 90.,
                90.,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(arcs.try_into().expect("four circle quadrants"))
}

/// Rational circle arc. The start direction is the least-aligned Cartesian
/// axis projected into the plane. Positive sweep follows the normal.
pub fn circle_arc(
    center: [f64; 3],
    normal: [f64; 3],
    radius: f64,
    start: f64,
    sweep: f64,
) -> Result<Curve> {
    check(
        radius.is_finite() && radius > 0.,
        "Circle radius must be finite and positive",
    )?;
    check(
        normal.iter().all(|x| x.is_finite()),
        "Circle normal must be finite",
    )?;
    let scale = normal.iter().fold(0_f64, |a, x| a.max(x.abs()));
    check(scale > 0., "Circle normal must be nonzero")?;
    let n = normal.map(|x| x / scale);
    let length = n.iter().map(|x| x * x).sum::<f64>().sqrt();
    let n = n.map(|x| x / length);
    let mut reference = 0;
    for i in 1..3 {
        if n[i].abs() < n[reference].abs() {
            reference = i
        }
    }
    let u = std::array::from_fn::<_, 3, _>(|i| {
        if i == reference {
            1. - n[i] * n[reference]
        } else {
            -n[i] * n[reference]
        }
    });
    let length = u.iter().map(|x| x * x).sum::<f64>().sqrt();
    let u = u.map(|x| x / length);
    let v = [
        n[1] * u[2] - n[2] * u[1],
        n[2] * u[0] - n[0] * u[2],
        n[0] * u[1] - n[1] * u[0],
    ];
    ellipse_arc(
        center,
        u.map(|x| x * radius),
        v.map(|x| x * radius),
        start,
        sweep,
    )
}

/// Affine ellipse arc: center + axis_u*cos(theta) + axis_v*sin(theta).
/// Axes are radius vectors and need not be orthogonal. Domain is [0, 1].
/// Full turns have coincident endpoints but use a clamped, non-periodic basis.
pub fn ellipse_arc(
    center: [f64; 3],
    axis_u: [f64; 3],
    axis_v: [f64; 3],
    start: f64,
    sweep: f64,
) -> Result<Curve> {
    check(
        center
            .iter()
            .chain(&axis_u)
            .chain(&axis_v)
            .chain([start, sweep].iter())
            .all(|x| x.is_finite()),
        "Ellipse data must be finite",
    )?;
    check(
        sweep != 0. && sweep.abs() <= 360.,
        "Ellipse sweep must be nonzero and within +/-360 degrees",
    )?;
    let scale_u = axis_u.iter().fold(0_f64, |a, x| a.max(x.abs()));
    let scale_v = axis_v.iter().fold(0_f64, |a, x| a.max(x.abs()));
    check(scale_u > 0. && scale_v > 0., "Ellipse axes must be nonzero")?;
    let u = axis_u.map(|x| x / scale_u);
    let v = axis_v.map(|x| x / scale_v);
    let cross = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    check(
        cross.iter().any(|x| x.abs() > 1e-14),
        "Ellipse axes must be independent and numerically well-conditioned",
    )?;
    let arcs = (sweep.abs() / 90.).ceil() as usize;
    let delta = sweep * PI / 180. / arcs as f64;
    let initial = start.rem_euclid(360.) * PI / 180.;
    // Exact authored quarter turns use shared signed axis coefficients.
    // Independently rounded trigonometric values at pi/2 and its multiples
    // introduce represented tangent/curvature mismatches between quadrants.
    let quarter_start=start.rem_euclid(360.)/90.;
    let canonical_quarters=quarter_start.fract()==0. && sweep.abs()/arcs as f64==90.;
    let quarter_coefficients=[[1.,0.],[1.,1.],[0.,1.],[-1.,1.],[-1.,0.],[-1.,-1.],[0.,-1.],[1.,-1.]];
    let mut points: Vec<Vec<f64>> = Vec::with_capacity(2 * arcs + 1);
    let mut weights = Vec::with_capacity(2 * arcs + 1);
    for i in 0..=2 * arcs {
        let w = if i % 2 == 0 { 1. } else if canonical_quarters {std::f64::consts::FRAC_1_SQRT_2} else { (delta / 2.).cos() };
        let theta = initial + i as f64 * delta / 2.;
        let point = if i == 2 * arcs && sweep.abs() == 360. {
            points[0].clone()
        } else if canonical_quarters {
            let index=(2*quarter_start as i32+if sweep>0. {i as i32}else{-(i as i32)}).rem_euclid(8) as usize;
            let [u,v]=quarter_coefficients[index];
            (0..3).map(|a|center[a]+axis_u[a]*u+axis_v[a]*v).collect()
        } else {
            (0..3)
                .map(|a| center[a] + (axis_u[a] * theta.cos() + axis_v[a] * theta.sin()) / w)
                .collect()
        };
        points.push(point);
        weights.push(w);
    }
    let mut knots = vec![0.; 3];
    for i in 1..arcs {
        knots.extend([i as f64 / arcs as f64; 2]);
    }
    knots.extend([1.; 3]);
    let result = Curve {
        degree: 2,
        knots,
        control_points: points,
        weights,
        periodic: false,
    };
    result.validate()?;
    Ok(result)
}

/// Polynomial parabola center + axis_u*t + axis_v*t^2, t in [a,b].
/// Curve parameter s in [0,1] maps affinely to t. Axes follow ellipse validation.
pub fn parabola(
    center: [f64; 3],
    axis_u: [f64; 3],
    axis_v: [f64; 3],
    a: f64,
    b: f64,
) -> Result<Curve> {
    ellipse_arc(center, axis_u, axis_v, 0., 90.)?;
    check(
        a.is_finite() && b.is_finite() && a < b,
        "Parabola interval must be finite and increasing",
    )?;
    let xy = [[a, a * a], [(a + b) / 2., a * b], [b, b * b]];
    let result = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: xy
            .iter()
            .map(|q| {
                (0..3)
                    .map(|i| center[i] + axis_u[i] * q[0] + axis_v[i] * q[1])
                    .collect()
            })
            .collect(),
        weights: vec![1.; 3],
        periodic: false,
    };
    result.validate()?;
    Ok(result)
}

/// One hyperbola branch center + axis_u*cosh(t) + axis_v*sinh(t).
/// Rational quadratic parameter is not affine in t. Finite intervals only.
pub fn hyperbola(
    center: [f64; 3],
    axis_u: [f64; 3],
    axis_v: [f64; 3],
    a: f64,
    b: f64,
) -> Result<Curve> {
    ellipse_arc(center, axis_u, axis_v, 0., 90.)?;
    check(
        a.is_finite() && b.is_finite() && a < b,
        "Hyperbola interval must be finite and increasing",
    )?;
    let mid = a / 2. + b / 2.;
    let w = (b / 2. - a / 2.).cosh();
    let xy = [
        [a.cosh(), a.sinh()],
        [mid.cosh() / w, mid.sinh() / w],
        [b.cosh(), b.sinh()],
    ];
    let result = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: xy
            .iter()
            .map(|q| {
                (0..3)
                    .map(|i| center[i] + axis_u[i] * q[0] + axis_v[i] * q[1])
                    .collect()
            })
            .collect(),
        weights: vec![1., w, 1.],
        periodic: false,
    };
    result.validate()?;
    Ok(result)
}

/// Conic in rho form: rational quadratic Bézier from endpoints + end tangents
/// + shoulder parameter rho (0 < rho < 1; rho < 1/2 ellipse, = 1/2 parabola,
/// > 1/2 hyperbola). The middle control point is the intersection of the two
/// tangent lines; weights are [1, rho/(1-rho), 1], so the shoulder point —
/// the curve at u = 1/2 — is (1-rho)*midpoint(chord) + rho*intersection.
/// Tangents are used as unoriented directions; they must not be parallel.
pub fn conic_rho(
    start: [f64; 3],
    end: [f64; 3],
    start_tangent: [f64; 3],
    end_tangent: [f64; 3],
    rho: f64,
) -> Result<Curve> {
    check(
        start
            .iter()
            .chain(&end)
            .chain(&start_tangent)
            .chain(&end_tangent)
            .all(|x| x.is_finite()),
        "Conic data must be finite",
    )?;
    check(
        rho.is_finite() && rho > 0. && rho < 1.,
        "Conic rho must lie in the open interval (0, 1)",
    )?;
    check(start != end, "Conic endpoints must differ")?;
    let scale_s = start_tangent.iter().fold(0_f64, |a, x| a.max(x.abs()));
    let scale_e = end_tangent.iter().fold(0_f64, |a, x| a.max(x.abs()));
    check(
        scale_s > 0. && scale_e > 0.,
        "Conic tangents must be nonzero",
    )?;
    let ts = start_tangent.map(|x| x / scale_s);
    let te = end_tangent.map(|x| x / scale_e);
    let cross = [
        ts[1] * te[2] - ts[2] * te[1],
        ts[2] * te[0] - ts[0] * te[2],
        ts[0] * te[1] - ts[1] * te[0],
    ];
    check(
        cross.iter().any(|x| x.abs() > 1e-14),
        "Conic tangents must not be parallel and must be numerically well-conditioned",
    )?;
    // Intersect start + s*ts with end + t*te on the dominant cross-product plane.
    let drop = (0..3)
        .max_by(|&a, &b| cross[a].abs().total_cmp(&cross[b].abs()))
        .expect("three axes");
    let [i, j] = {
        let mut pair: Vec<usize> = (0..3).filter(|&k| k != drop).collect();
        pair.sort_unstable();
        [pair[0], pair[1]]
    };
    let d = [end[i] - start[i], end[j] - start[j]];
    let det = ts[j] * te[i] - ts[i] * te[j];
    check(det.is_finite() && det != 0., "Conic tangent lines do not intersect")?;
    let s = (te[i] * d[1] - d[0] * te[j]) / det;
    let middle = std::array::from_fn::<_, 3, _>(|a| start[a] + s * ts[a]);
    check(
        middle.iter().all(|x| x.is_finite()),
        "Conic tangent intersection must be finite",
    )?;
    let w = rho / (1. - rho);
    let result = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![start.to_vec(), middle.to_vec(), end.to_vec()],
        weights: vec![1., w, 1.],
        periodic: false,
    };
    result.validate()?;
    Ok(result)
}

/// Osculating conic at a point: matches the point, the tangent direction, and
/// the signed curvature. Construction: the osculating circular arc of radius
/// 1/|curvature|, centered at point + normal/curvature, sweeping 60 degrees
/// from the query point along the tangent; expressed in rho form with
/// w = cos(30°) and rho = w/(1+w) ≈ 0.464. The curvature sign selects which
/// side of the tangent the center lies on; the in-plane normal is the
/// least-aligned Cartesian axis projected perpendicular to the tangent (the
/// circle_arc convention). Zero curvature has no osculating conic of this
/// family and is rejected.
pub fn conic_osculating(point: [f64; 3], tangent: [f64; 3], curvature: f64) -> Result<Curve> {
    check(
        point.iter().chain(&tangent).all(|x| x.is_finite()),
        "Osculating conic data must be finite",
    )?;
    check(
        curvature.is_finite() && curvature != 0.,
        "Osculating curvature must be finite and nonzero",
    )?;
    let scale = tangent.iter().fold(0_f64, |a, x| a.max(x.abs()));
    check(scale > 0., "Osculating tangent must be nonzero")?;
    let t = tangent.map(|x| x / scale);
    let length = t.iter().map(|x| x * x).sum::<f64>().sqrt();
    let t = t.map(|x| x / length);
    let mut reference = 0;
    for i in 1..3 {
        if t[i].abs() < t[reference].abs() {
            reference = i;
        }
    }
    let raw = std::array::from_fn::<_, 3, _>(|i| {
        if i == reference {
            1. - t[i] * t[reference]
        } else {
            -t[i] * t[reference]
        }
    });
    let length = raw.iter().map(|x| x * x).sum::<f64>().sqrt();
    let n = raw.map(|x| x / length * curvature.signum());
    let r = 1. / curvature.abs();
    let center = std::array::from_fn::<_, 3, _>(|a| point[a] + r * n[a]);
    // Arc of sweep theta0 from the point: position center + r*(-cos* n + sin* t).
    let theta0 = PI / 3.;
    let (sin, cos) = theta0.sin_cos();
    let end = std::array::from_fn(|a| center[a] + r * (-cos * n[a] + sin * t[a]));
    let end_tangent = std::array::from_fn(|a| sin * n[a] + cos * t[a]);
    let w = (theta0 / 2.).cos();
    conic_rho(point, end, t, end_tangent, w / (1. + w))
}
