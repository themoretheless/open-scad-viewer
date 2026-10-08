//! Correlated original G(t)-P(t) values in a literally shared positive basis.
//! Rounding the subtraction is enclosed; no poles or source identity are fitted.
use super::*;

pub(super) fn same_parameter_offset_values(
    path: &Curve,
    guide: &Curve,
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<Option<vector_certificate::ValuesReport>> {
    path.validate()?;
    guide.validate()?;
    check(max_cells <= 100000, "Invalid paired source value budget")?;
    if path.degree != guide.degree
        || path.knots != guide.knots
        || path.weights != guide.weights
        || path.periodic != guide.periodic
        || path.control_points.len() != guide.control_points.len()
        || [path, guide]
            .iter()
            .any(|s| s.control_points.iter().any(|p| p.len() != 3))
    {
        return Ok(None);
    }
    // Three global axis residual cells: positivity of the common basis extends
    // each exhaustive pole residual maximum to the entire curve domain. The
    // subsequent restricted curve image charges its original knot-span cells.
    let charge = 3;
    let mut out = vector_certificate::ValuesReport {
        cells: 0,
        value: None,
    };
    if charge > max_cells {
        return Ok(Some(out));
    }
    let mut offset = path.clone();
    let mut rounding = [0f64; 3];
    for ((stored, p), g) in offset
        .control_points
        .iter_mut()
        .zip(&path.control_points)
        .zip(&guide.control_points)
    {
        for k in 0..3 {
            let delta = g[k] - p[k];
            // The generated scalar certificate has its original coordinate
            // input ceiling. Unsupported differences use the caller's old
            // independent boxes rather than changing that contract.
            if delta.abs() > 1e9 {
                return Ok(None);
            }
            let residual = I::point(g[k]).sub(I::point(p[k]))?.sub(I::point(delta))?;
            rounding[k] = rounding[k].max(residual.lo.abs()).max(residual.hi.abs());
            stored[k] = delta;
        }
    }
    out.cells = charge;
    let image = vector_certificate::certify_values_traversal(
        &offset,
        traversal,
        max_cells - charge,
        false,
    )?;
    out.cells += image.cells;
    if let Some(values) = image.value {
        // Positive common rational weights form a convex combination of the
        // exact original pole subtraction residuals on the entire traversal.
        let mut covered = [[0.; 2]; 3];
        for k in 0..3 {
            let value =
                I::new(values[k][0], values[k][1])?.add(I::new(-rounding[k], rounding[k])?)?;
            covered[k] = [value.lo, value.hi];
        }
        out.value = Some(covered);
    }
    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn common_circle_offset_owns_original_correlation_and_full_rounding_budget() {
        let path = crate::primitives::circle([0.; 3], [0., 0., 1.], 4.).unwrap();
        let guide = crate::primitives::circle([0.; 3], [0., 0., 1.], 4.25).unwrap();
        let saved = guide.control_points.clone();
        let range = [0., 0.0625];
        let proof = same_parameter_offset_values(&path, &guide, range, 1000)
            .unwrap()
            .unwrap();
        let bounds = proof.value.unwrap();
        assert!(bounds[0][0] > 0.2 && bounds[0][1] < 0.26);
        for t in [0., 0.015625, 0.03125, 0.046875, 0.0625] {
            let p = path.evaluate(t).unwrap().point;
            let g = guide.evaluate(t).unwrap().point;
            for k in 0..3 {
                let delta = g[k] - p[k];
                assert!(bounds[k][0] <= delta && delta <= bounds[k][1]);
            }
        }
        assert!(
            same_parameter_offset_values(&path, &guide, range, proof.cells - 1)
                .unwrap()
                .unwrap()
                .value
                .is_none()
        );
        assert!(
            same_parameter_offset_values(&path, &guide, range, 0)
                .unwrap()
                .unwrap()
                .value
                .is_none()
        );
        assert_eq!(guide.control_points, saved);
        let mut changed = guide.clone();
        changed.weights[1] = changed.weights[1].next_up();
        assert!(
            same_parameter_offset_values(&path, &changed, range, 1000)
                .unwrap()
                .is_none()
        );
    }
}
