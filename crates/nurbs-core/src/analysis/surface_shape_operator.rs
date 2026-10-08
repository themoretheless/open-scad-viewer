//! World-coordinate curvature tensor for future geometric G2 seam inspection.
use crate::{
    Result,
    curve_differential::Side,
    distance_bounds::{Interval as I, box_distance},
    numerics::interval_vec3::dot,
    surface::Surface,
    surface_differential::{self, Status},
    surface_measure::jets,
};
#[derive(Clone, Debug)]
pub struct Report {
    pub status: Status,
    pub normal: Option<[[f64; 2]; 3]>,
    /// A G^-1 II G^-1 A^T, extended by zero on the normal direction.
    /// Signed curvature uses orientation S_u cross S_v.
    pub tensor: Option<[[[f64; 2]; 3]; 3]>,
}
pub(crate) struct Shape {
    pub normal: [I; 3],
    pub tensor: [[I; 3]; 3],
}
pub(crate) fn from_jets(j: &jets::Jets) -> Result<Option<Shape>> {
    let u = j[1][0];
    let v = j[0][1];
    let mut normal = [I::point(0.); 3];
    for k in 0..3 {
        let a = (k + 1) % 3;
        let b = (k + 2) % 3;
        normal[k] = u[a].mul(v[b])?.sub(u[b].mul(v[a])?)?;
    }
    let (lo, hi) = box_distance(&normal, &[I::point(0.); 3])?;
    if lo <= 0. {
        return Ok(None);
    }
    let area = I::new(lo, hi)?;
    let determinant = area.mul(area)?;
    for x in &mut normal {
        *x = x.div(area)?.intersect(-1., 1.)?;
    }
    let e = dot(u, u)?;
    let f = dot(u, v)?;
    let g = dot(v, v)?;
    let minus_f = I::new(-f.hi, -f.lo)?;
    let inverse = [
        [g.div(determinant)?, minus_f.div(determinant)?],
        [minus_f.div(determinant)?, e.div(determinant)?],
    ];
    let second = [
        [dot(normal, j[2][0])?, dot(normal, j[1][1])?],
        [dot(normal, j[1][1])?, dot(normal, j[0][2])?],
    ];
    let mut coefficients = [[I::point(0.); 2]; 2];
    for a in 0..2 {
        for b in 0..2 {
            for r in 0..2 {
                for s in 0..2 {
                    coefficients[a][b] = coefficients[a][b]
                        .add(inverse[a][r].mul(second[r][s])?.mul(inverse[s][b])?)?;
                }
            }
        }
    }
    let vectors = [u, v];
    let mut tensor = [[I::point(0.); 3]; 3];
    for a in 0..3 {
        for b in 0..3 {
            for r in 0..2 {
                for s in 0..2 {
                    tensor[a][b] = tensor[a][b]
                        .add(vectors[r][a].mul(coefficients[r][s])?.mul(vectors[s][b])?)?;
                }
            }
        }
    }
    Ok(Some(Shape { normal, tensor }))
}
/// Local query only: this is not a whole-seam G2 certificate. Automatic needs
/// a C2 basis at interior knots; explicit sides resolve one-sided jet queries.
pub fn at(surface: &Surface, parameter: [f64; 2], sides: [Side; 2]) -> Result<Report> {
    let status = surface_differential::at(surface, parameter, sides)?.status;
    let mut report = Report {
        status,
        normal: None,
        tensor: None,
    };
    if status != Status::Available {
        return Ok(report);
    }
    let mut spans = [0; 2];
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
        let left = sides[axis] == Side::Left || t == k[n];
        spans[axis] = (p..n)
            .find(|&i| {
                k[i] < k[i + 1]
                    && if left {
                        k[i] < t && t <= k[i + 1]
                    } else {
                        k[i] <= t && t < k[i + 1]
                    }
            })
            .ok_or_else(|| crate::input("No span owns the curvature tensor query"))?;
    }
    let j = jets::calculate_partial_stable(surface, spans, parameter.map(|t| [t, t]), [None; 2])?;
    let Some(shape) = from_jets(&j)? else {
        report.status = Status::RegularityNotProven;
        return Ok(report);
    };
    report.normal = Some(shape.normal.map(|x| [x.lo, x.hi]));
    report.tensor = Some(shape.tensor.map(|row| row.map(|x| [x.lo, x.hi])));
    Ok(report)
}

/// Continuous tensor/normal boxes on a normalized boundary subinterval.
/// This does not compare two surfaces or certify G2. An interior free-axis
/// knot in the query requires a C2 basis; storage seam closure is not inferred.
pub fn boundary_bounds(
    surface: &Surface,
    edge: crate::surface_join::Boundary,
    range: [f64; 2],
    reverse: bool,
) -> Result<Report> {
    surface.validate()?;
    crate::check(
        range.iter().all(|x| x.is_finite())
            && 0. <= range[0]
            && range[0] <= range[1]
            && range[1] <= 1.,
        "Tensor boundary range must lie in [0,1]",
    )?;
    use crate::surface_join::Boundary;
    let (p, k, n) = match edge {
        Boundary::UMin | Boundary::UMax => (
            surface.degree_v,
            &surface.knots_v,
            surface.control_points[0].len(),
        ),
        Boundary::VMin | Boundary::VMax => (
            surface.degree_u,
            &surface.knots_u,
            surface.control_points.len(),
        ),
    };
    let mut t = I::new(range[0], range[1])?;
    if reverse {
        t = I::point(1.).sub(t)?.intersect(0., 1.)?;
    }
    let query = I::point(k[p])
        .add(I::point(k[n]).sub(I::point(k[p]))?.mul(t)?)?
        .intersect(k[p], k[n])?;
    for &node in k {
        if k[p] < node
            && node < k[n]
            && query.lo <= node
            && node <= query.hi
            && (p as i32 - k.iter().filter(|&&x| x == node).count() as i32) < 2
        {
            return Ok(Report {
                status: Status::ContinuityNotProven,
                normal: None,
                tensor: None,
            });
        }
    }
    let mut result: Option<Shape> = None;
    for j in crate::surface_g1::boundary_jets(surface, edge, range, reverse)? {
        let Some(shape) = from_jets(&j)? else {
            return Ok(Report {
                status: Status::RegularityNotProven,
                normal: None,
                tensor: None,
            });
        };
        if let Some(result) = &mut result {
            for a in 0..3 {
                result.normal[a].lo = result.normal[a].lo.min(shape.normal[a].lo);
                result.normal[a].hi = result.normal[a].hi.max(shape.normal[a].hi);
                for b in 0..3 {
                    result.tensor[a][b].lo = result.tensor[a][b].lo.min(shape.tensor[a][b].lo);
                    result.tensor[a][b].hi = result.tensor[a][b].hi.max(shape.tensor[a][b].hi);
                }
            }
        } else {
            result = Some(shape);
        }
    }
    let Some(shape) = result else {
        return Ok(Report {
            status: Status::RegularityNotProven,
            normal: None,
            tensor: None,
        });
    };
    Ok(Report {
        status: Status::Available,
        normal: Some(shape.normal.map(|x| [x.lo, x.hi])),
        tensor: Some(shape.tensor.map(|r| r.map(|x| [x.lo, x.hi]))),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tangential_speed_shear_and_nonlinear_terms_preserve_world_tensor() {
        // Graph z=x²+2y² at the origin, composed with x=2u+v+3u²,y=3v.
        let mut j = [[[I::point(0.); 3]; 4]; 4];
        j[1][0] = [I::point(2.), I::point(0.), I::point(0.)];
        j[0][1] = [I::point(1.), I::point(3.), I::point(0.)];
        j[2][0] = [I::point(6.), I::point(0.), I::point(8.)];
        j[1][1] = [I::point(0.), I::point(0.), I::point(4.)];
        j[0][2] = [I::point(0.), I::point(0.), I::point(38.)];
        let shape = from_jets(&j).unwrap().unwrap();
        for a in 0..3 {
            for b in 0..3 {
                let value = if a == b { [2., 4., 0.][a] } else { 0. };
                let x = shape.tensor[a][b];
                assert!(x.lo <= value && value <= x.hi);
                assert!(x.hi - x.lo < 1e-8);
            }
        }
    }
}
