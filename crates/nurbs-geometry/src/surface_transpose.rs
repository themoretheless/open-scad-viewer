//! Exchange of a surface's U and V parameters.
use crate::{Result, surface::Surface};

/// New S(u,v) = old S(v,u). Transposition reverses normal orientation.
pub fn transpose(source: &Surface) -> Result<Surface> {
    source.validate()?;
    let mut result = Surface {
        degree_u: source.degree_v,
        degree_v: source.degree_u,
        knots_u: source.knots_v.clone(),
        knots_v: source.knots_u.clone(),
        periodic_u: source.periodic_v,
        periodic_v: source.periodic_u,
        control_points: vec![],
        weights: vec![],
    };
    for v in 0..source.control_points[0].len() {
        result
            .control_points
            .push(source.control_points.iter().map(|r| r[v].clone()).collect());
        result
            .weights
            .push(source.weights.iter().map(|r| r[v]).collect());
    }
    result.validate()?;
    Ok(result)
}
