use super::*;

/// True when the point is strictly inside the sphere; ambiguous → refusal.
pub(crate) fn sphere_inside(sphere: &CanonicalSphere, point: [f64; 3], band: f64) -> Result<bool> {
    let d = dist(point, sphere.center) - sphere.radius;
    if d < -band {
        Ok(true)
    } else if d > band {
        Ok(false)
    } else {
        Err(unsupported(
            "Sphere Boolean: region classification falls within the error band of the sphere",
        ))
    }
}

/// Parameter of the point on a single-span arc `curve` (domain [0, 1])
/// nearest to `point`, by ternary search on the squared distance, which is
/// unimodal along an arc of at most a half turn. Refuses when the point is
/// not on the curve within `limit`.
pub(super) fn curve_parameter_of(curve: &Curve, point: [f64; 3], limit: f64) -> Result<f64> {
    let d2 = |t: f64| -> Result<f64> {
        let p = point3(&curve.evaluate(t)?.point);
        Ok(dot(sub(p, point), sub(p, point)))
    };
    let (mut lo, mut hi) = (0., 1.);
    for _ in 0..200 {
        let m1 = lo + (hi - lo) / 3.;
        let m2 = hi - (hi - lo) / 3.;
        if d2(m1)? <= d2(m2)? {
            hi = m2;
        } else {
            lo = m1;
        }
        if hi - lo < 1e-16 {
            break;
        }
    }
    let t = (lo + hi) / 2.;
    // Endpoint snap: the ends are the exact vertices.
    let t = if t < 1e-9 {
        0.
    } else if t > 1. - 1e-9 {
        1.
    } else {
        t
    };
    if d2(t)?.sqrt() > limit {
        return Err(unsupported(
            "Sphere Boolean: station point is not on its sphere edge",
        ));
    }
    Ok(t)
}

/// Exact pcurve of a single-span rational quadratic arc on the sphere in
/// patch `patch`: degree-2 Bernstein conversion of the inverted 3D arc. The
/// caller snaps the ends to its chart vertices and verifies by sampling.
pub(crate) fn sphere_arc_pcurve(
    sphere: &CanonicalSphere,
    patch: usize,
    curve: &Curve,
) -> Result<Curve> {
    let (a, b, pole) = sphere.frame(patch);
    let center = sphere.center;
    let radius = sphere.radius;
    let sample = |t: f64| -> (f64, f64, f64) {
        let b0 = (1. - t) * (1. - t);
        let b1 = 2. * t * (1. - t);
        let b2 = t * t;
        let mut h = [0.; 3];
        let mut hw = 0.;
        for (i, basis) in [b0, b1, b2].into_iter().enumerate() {
            let w = basis * curve.weights[i];
            for axis in 0..3 {
                h[axis] += w * curve.control_points[i][axis];
            }
            hw += w;
        }
        let d = std::array::from_fn(|axis| h[axis] - center[axis] * hw);
        (dot(d, a), dot(d, b), radius * hw + dot(d, pole))
    };
    let f0 = sample(0.);
    let f1 = sample(0.5);
    let f2 = sample(1.);
    let bern = |f: (f64, f64, f64), i: usize| -> f64 {
        match i {
            0 => f.0,
            2 => f.2,
            _ => 2. * f.1 - 0.5 * f.0 - 0.5 * f.2,
        }
    };
    let mut control_points = Vec::with_capacity(3);
    let mut weights = Vec::with_capacity(3);
    for i in 0..3 {
        let den = bern((f0.2, f1.2, f2.2), i);
        if !den.is_finite() || den <= 0. {
            return Err(unsupported(
                "Sphere Boolean: arc leaves the patch hemisphere",
            ));
        }
        control_points.push(vec![
            bern((f0.0, f1.0, f2.0), i) / den,
            bern((f0.1, f1.1, f2.1), i) / den,
        ]);
        weights.push(den);
    }
    Ok(Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points,
        weights,
        periodic: false,
    })
}

/// Sample `pcurve` through `surface` against the 3D `curve` at matching
/// parameters; both are single-span over [0, 1].
pub(crate) fn verify_pcurve(
    pcurve: &Curve,
    curve: &Curve,
    surface: &Surface,
    limit: f64,
) -> Result<()> {
    for i in 0..=8 {
        let t = i as f64 / 8.;
        let uv = pcurve.evaluate(t)?.point;
        let through = point3(&surface.evaluate(uv[0], uv[1])?.point);
        let along = point3(&curve.evaluate(t)?.point);
        if dist(through, along) > limit {
            return Err(unsupported(
                "Sphere Boolean: arc pcurve does not track the 3D arc on its surface",
            ));
        }
    }
    Ok(())
}
