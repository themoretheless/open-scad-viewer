//! surface injectivity algorithm: certify_polar_p_matrix.
use super::*;
/// Positive principal minors on one global Jacobian enclosure suffice for
/// univalence of this C1 projected map on the complete closed rectangle.
/// Nonzero determinant alone is insufficient. A polar branch is proven in
/// every original cell by the same strictly positive X enclosure as above.
pub fn certify_polar_p_matrix(
    s: &Surface,
    coordinates: [[f64; 4]; 3],
    subdivisions: usize,
    max_cells: usize,
) -> Result<Option<PolarPMatrix>> {
    s.validate()?;
    let Some(global) = polar_projection_jacobian(s, coordinates, subdivisions, max_cells)? else {
        return Ok(None);
    };
    for (knots, degree, count) in [
        (&s.knots_u, s.degree_u, s.control_points.len()),
        (&s.knots_v, s.degree_v, s.control_points[0].len()),
    ] {
        if knots
            .iter()
            .filter(|k| **k > knots[degree] && **k < knots[count])
            .any(|k| knots.iter().filter(|x| *x == k).count() >= degree)
        {
            return Ok(None);
        }
    }
    let mut oriented = global;
    let mut signs = [1.; 2];
    for row in 0..2 {
        if global[row][row].lo > 0. {
        } else if global[row][row].hi < 0. {
            signs[row] = -1.;
            for axis in 0..2 {
                oriented[row][axis] = I::point(-1.).mul(global[row][axis])?;
            }
        } else {
            return Ok(None);
        }
    }
    if oriented[0][0].lo <= 0. || oriented[1][1].lo <= 0. {
        return Ok(None);
    }
    let determinant = oriented[0][0]
        .mul(oriented[1][1])?
        .sub(oriented[0][1].mul(oriented[1][0])?)?;
    if determinant.lo <= 0. {
        return Ok(None);
    }
    Ok(Some(PolarPMatrix {
        coordinates,
        jacobian: global.map(|r| r.map(|v| [v.lo, v.hi])),
        row_signs: signs,
        diagonal_lower: [oriented[0][0].lo, oriented[1][1].lo],
        determinant_lower: determinant.lo,
    }))
}
