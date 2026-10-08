//! Principal curvature bounds of the original rational surface at a point.
use crate::{
    Result, check,
    curve_differential::Side,
    distance_bounds::Interval as I,
    numerics::interval_vec3::{cross, dot, norm},
    surface::Surface,
    surface_measure::jets,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Available,
    ContinuityNotProven,
    RegularityNotProven,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectionStatus {
    NotComputed,
    EigenvaluesNotSeparated,
    VectorNotSeparated,
    Available,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub status: Status,
    pub direction_status: DirectionStatus,
    /// Signed ascending curvatures for the orientation S_u cross S_v.
    pub principal: Option<[[f64; 2]; 2]>,
    pub gaussian: Option<[f64; 2]>,
    pub mean: Option<[f64; 2]>,
    /// Unit vector boxes, one for each eigenvalue. Signs are arbitrary.
    /// None at umbilics or when eigenvalue/vector separation is unproven.
    pub directions: Option<[[[f64; 2]; 3]; 2]>,
}
fn bounds(x: I) -> [f64; 2] {
    [x.lo, x.hi]
}
/// Explicit sides allow C0/C1 knots; Automatic requires a C2 basis in both
/// interior directions. Periodic endpoints are queried on the natural-domain
/// side, without claiming continuity across their storage seam.
pub fn at(surface: &Surface, parameter: [f64; 2], sides: [Side; 2]) -> Result<Report> {
    surface.validate()?;
    let mut spans = [0; 2];
    let mut domain = [[0.; 2]; 2];
    let mut corner = [0; 2];
    let mut smooth = true;
    for axis in 0..2 {
        let (p, k, n) = if axis == 0 {
            (
                surface.degree_u,
                &surface.knots_u,
                surface.control_points.len(),
            )
        } else {
            (
                surface.degree_v,
                &surface.knots_v,
                surface.control_points[0].len(),
            )
        };
        let t = parameter[axis];
        let a = k[p];
        let b = k[n];
        check(
            t.is_finite() && a <= t && t <= b,
            "Surface differential query lies outside its domain",
        )?;
        check(
            !(t == a && sides[axis] == Side::Left) && !(t == b && sides[axis] == Side::Right),
            "Requested side lies outside the natural domain",
        )?;
        if sides[axis] == Side::Automatic && a < t && t < b {
            let m = k.iter().filter(|&&x| x == t).count();
            if m > 0 && (p as i32 - m as i32) < 2 {
                smooth = false;
            }
        }
        let left = sides[axis] == Side::Left || t == b;
        let i = (p..n)
            .find(|&i| {
                k[i] < k[i + 1]
                    && if left {
                        k[i] < t && t <= k[i + 1]
                    } else {
                        k[i] <= t && t < k[i + 1]
                    }
            })
            .ok_or_else(|| crate::input("No span owns the requested differential side"))?;
        spans[axis] = i;
        if t - k[i] >= k[i + 1] - t {
            domain[axis] = [k[i], t];
            corner[axis] = 1;
        } else {
            domain[axis] = [t, k[i + 1]];
            corner[axis] = 0;
        }
    }
    let mut out = Report {
        status: Status::ContinuityNotProven,
        direction_status: DirectionStatus::NotComputed,
        principal: None,
        gaussian: None,
        mean: None,
        directions: None,
    };
    if !smooth {
        return Ok(out);
    }
    out.status = Status::RegularityNotProven;
    let j = jets::calculate(surface, spans, domain, Some(corner))?;
    let u = j[1][0];
    let v = j[0][1];
    let normal = cross(u, v)?;
    let area = norm(normal)?;
    if area.lo <= 0. {
        return Ok(out);
    }
    let mut unit = normal;
    for x in &mut unit {
        *x = x.div(area)?.intersect(-1., 1.)?;
    }
    let e1 = dot(u, u)?;
    let f1 = dot(u, v)?;
    let g1 = dot(v, v)?;
    let e2 = dot(unit, j[2][0])?;
    let f2 = dot(unit, j[1][1])?;
    let g2 = dot(unit, j[0][2])?;
    let determinant = area.mul(area)?;
    let gaussian = e2.mul(g2)?.sub(f2.mul(f2)?)?.div(determinant)?;
    let mean = e2
        .mul(g1)?
        .add(g2.mul(e1)?)?
        .sub(f2.mul(f1)?.mul(I::point(2.))?)?
        .div(determinant.mul(I::point(2.))?)?;
    // The discriminant of a real self-adjoint shape operator is nonnegative.
    let d = mean.mul(mean)?.sub(gaussian)?;
    crate::numeric(
        d.hi >= 0.,
        "Curvature discriminant enclosure is inconsistent",
    )?;
    let root = I::new(
        d.lo.max(0.).sqrt().next_down().max(0.),
        d.hi.max(0.).sqrt().next_up(),
    )?;
    let values = [mean.sub(root)?, mean.add(root)?];
    out.principal = Some(values.map(bounds));
    out.gaussian = Some(bounds(gaussian));
    out.mean = Some(bounds(mean));
    out.status = Status::Available;
    out.direction_status = DirectionStatus::EigenvaluesNotSeparated;
    if values[0].hi >= values[1].lo {
        return Ok(out);
    }
    let mut directions = [[[0.; 2]; 3]; 2];
    out.direction_status = DirectionStatus::VectorNotSeparated;
    for (index, k) in values.into_iter().enumerate() {
        let candidates = [
            [f2.sub(k.mul(f1)?)?, k.mul(e1)?.sub(e2)?],
            [k.mul(g1)?.sub(g2)?, f2.sub(k.mul(f1)?)?],
        ];
        let mut found = None;
        for [a, b] in candidates {
            let mut vector = [I::point(0.); 3];
            for q in 0..3 {
                vector[q] = u[q].mul(a)?.add(v[q].mul(b)?)?;
            }
            let length = norm(vector)?;
            if length.lo > 0. {
                for q in 0..3 {
                    vector[q] = vector[q].div(length)?.intersect(-1., 1.)?;
                }
                found = Some(vector.map(bounds));
                break;
            }
        }
        let Some(vector) = found else {
            return Ok(out);
        };
        directions[index] = vector;
    }
    out.directions = Some(directions);
    out.direction_status = DirectionStatus::Available;
    Ok(out)
}
