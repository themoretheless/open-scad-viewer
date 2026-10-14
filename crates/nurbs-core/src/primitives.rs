//! Compact rational conics and surfaces of revolution. Angles are in degrees.
//! Exact rational constructions in real arithmetic; binary64 rounding remains.
use crate::{Result, check, curve::Curve};
use std::f64::consts::PI;

/// Exact straight segment, with normalized domain [0,1].
pub fn line(start: [f64; 3], end: [f64; 3]) -> Result<Curve> {
    polyline(&[start, end], false)
}

/// Degree-one path with equal parameter intervals per segment, not arc length.
/// Closed paths repeat the first point and retain a clamped nonperiodic basis.
pub fn polyline(points: &[[f64; 3]], closed: bool) -> Result<Curve> {
    check(
        points.len() >= 2 && points.len() <= 256,
        "Polyline requires 2..256 points",
    )?;
    check(
        points.iter().flatten().all(|x| x.is_finite()),
        "Polyline points must be finite",
    )?;
    check(
        points.windows(2).all(|pair| pair[0] != pair[1]),
        "Polyline has a zero-length segment",
    )?;
    let mut points = points.iter().map(|p| p.to_vec()).collect::<Vec<_>>();
    if closed {
        if points.first() != points.last() {
            points.push(points[0].clone())
        }
        check(
            points.len() >= 4,
            "A closed polyline requires at least three vertices",
        )?;
    }
    let mut curve = Curve::from_polyline(points)?;
    let end = curve.domain()[1];
    for knot in &mut curve.knots {
        *knot /= end
    }
    curve.validate()?;
    Ok(curve)
}

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

#[cfg(test)]
mod canonical_quarter_tests {
    use super::*;
    #[test]
    fn full_ellipse_and_signed_quadrants_share_authored_axis_coefficients() {
        let center=[2.,4.,8.];let u=[0.5,0.,0.];let v=[0.,0.25,0.];
        for direction in [1.,-1.] {
            let full=ellipse_arc(center,u,v,0.,direction*360.).unwrap();
            for quadrant in 0..4 {
                let arc=ellipse_arc(center,u,v,direction*quadrant as f64*90.,direction*90.).unwrap();
                assert_eq!(&full.control_points[2*quadrant..=2*quadrant+2],arc.control_points.as_slice());
                assert_eq!(&full.weights[2*quadrant..=2*quadrant+2],arc.weights.as_slice());
            }
            assert_eq!(full.control_points.first(),full.control_points.last());
            assert_eq!(full.weights[1],std::f64::consts::FRAC_1_SQRT_2);
        }
    }
}

use crate::surface::Surface;
#[path="construct/curves/primitives/revolution.rs"] mod revolution;
pub use revolution::{cone,cone_frustum,cylinder,ellipsoid,elliptic_cylinder,hyperboloid_one_sheet,hyperboloid_two_sheet,sphere,torus};
pub fn quadratic_patch(bounds: [f64; 4], coefficients: [f64; 6]) -> Result<Surface> {
    let [x0, x1, y0, y1] = bounds;
    check(
        bounds.iter().chain(&coefficients).all(|v| v.is_finite()) && x0 < x1 && y0 < y1,
        "Quadratic patch needs finite coefficients and increasing XY bounds",
    )?;
    let x = [x0, x0 / 2. + x1 / 2., x1];
    let y = [y0, y0 / 2. + y1 / 2., y1];
    let xx = [x0 * x0, x0 * x1, x1 * x1];
    let yy = [y0 * y0, y0 * y1, y1 * y1];
    let [a, b, c, d, e, f] = coefficients;
    let controls = (0..3)
        .map(|i| {
            (0..3)
                .map(|j| {
                    vec![
                        x[i],
                        y[j],
                        a * xx[i] + b * x[i] * y[j] + c * yy[j] + d * x[i] + e * y[j] + f,
                    ]
                })
                .collect()
        })
        .collect();
    let result = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: controls,
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    result.validate()?;
    Ok(result)
}


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
