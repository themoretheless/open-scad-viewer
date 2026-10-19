//! Strict symmetric-Jacobian proof on an original rational cell.
//! Unlike a P-matrix minor, this proves the same fixed strong metric needed to
//! glue C0 charts. Polynomial products retain shared weight dependence.
use super::*;
use crate::surface_injectivity::{
    PolarPoly, polar_poly_add, polar_poly_derivative, polar_poly_mul,
};
pub(super) fn certify(
    s: &Surface,
    span: [usize; 2],
    domain: [[f64; 2]; 2],
    projection: [[i8; 3]; 2],
    metric: [f64; 2],
    rates: [[f64; 2]; 2],
) -> Result<bool> {
    // Bound polynomial arithmetic inside one already charged enclosure cell.
    // Other degrees keep the existing interval-jet/subdivision proof.
    if s.degree_u > 2 || s.degree_v > 2 {
        return Ok(false);
    }
    let net = crate::curve_surface_composition::surface_net_on(s, span, domain)?;
    if net.iter().flatten().any(|h| h[3].lo <= 0.) {
        return Ok(false);
    }
    let origin = &s.control_points[span[0] - s.degree_u][span[1] - s.degree_v];
    let mut coordinates: [PolarPoly; 3] = std::array::from_fn(|_| Vec::new());
    for row in net {
        let mut projected: [Vec<I>; 3] = std::array::from_fn(|_| Vec::new());
        for h in row {
            for r in 0..2 {
                let mut value = I::point(0.);
                for k in 0..3 {
                    if projection[r][k] != 0 {
                        value = value.add(
                            h[k].sub(I::point(origin[k]).mul(h[3])?)?
                                .mul(I::point(projection[r][k] as f64))?,
                        )?;
                    }
                }
                projected[r].push(value.mul(I::point(metric[r]))?);
            }
            projected[2].push(h[3]);
        }
        for (target, row) in coordinates.iter_mut().zip(projected) {
            target.push(row);
        }
    }
    let w = &coordinates[2];
    let numerator = |row: usize, axis: usize| -> Result<PolarPoly> {
        let x = &coordinates[row];
        let mut out = polar_poly_add(
            &polar_poly_mul(&polar_poly_derivative(x, axis, domain)?, w)?,
            &polar_poly_mul(x, &polar_poly_derivative(w, axis, domain)?)?,
            -1.,
        )?;
        if rates[axis] != [1., 1.] {
            let rate = I::new(rates[axis][0], rates[axis][1])?;
            for c in out.iter_mut().flatten() {
                *c = c.mul(rate)?;
            }
        }
        Ok(out)
    };
    // Elevate all four derivative numerators to one bidegree by multiplying
    // by the constant polynomial 1, represented in the missing direction.
    let common = |p: PolarPoly, axis: usize| {
        polar_poly_mul(
            &p,
            &if axis == 0 {
                vec![vec![I::point(1.)]; 2]
            } else {
                vec![vec![I::point(1.); 2]]
            },
        )
    };
    let uu = common(numerator(0, 0)?, 0)?;
    let uv = common(numerator(0, 1)?, 1)?;
    let vu = common(numerator(1, 0)?, 0)?;
    let vv = common(numerator(1, 1)?, 1)?;
    let positive = |p: &PolarPoly| p.iter().flatten().all(|c| c.lo > 0.);
    if !positive(&uu) || !positive(&vv) {
        return Ok(false);
    }
    let off = polar_poly_add(&uv, &vu, 1.)?;
    let determinant = polar_poly_add(
        &polar_poly_mul(&uu, &vv)?,
        &polar_poly_mul(&off, &off)?,
        -0.25,
    )?;
    // J=A/W², W>0. Both diagonal minors and det(sym A) are positive
    // everywhere, including the complete closed cell boundary.
    Ok(positive(&determinant))
}
