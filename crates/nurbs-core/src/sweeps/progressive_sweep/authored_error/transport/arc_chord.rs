//! Original arc-station path chord error, including inverse-length residuals.
use super::*;
pub(super) fn upper(
    path: &Curve,
    traversal: [f64; 2],
    station: [f64; 2],
    length: [f64; 2],
    residual: f64,
    geometric_c1: bool,
    max: usize,
) -> Result<(usize, Option<f64>)> {
    let source = vector_certificate::certify_traversal(path, traversal, max, false)?;
    if source.status != Status::Certified || !(source.single_span || geometric_c1) {
        return Ok((source.cells, None));
    }
    let mut spent = source.cells;
    let h = I::point(station[1]).sub(I::point(station[0]))?;
    // A proposal relative to the existing first-order chord bound. All
    // accepted leaves still carry complete outward original derivatives.
    let target = I::point(length[1]).mul(h)?.div(I::point(16.))?.hi;
    let mut pending = vec![(traversal, Some(source))];
    let mut error = 0_f64;
    while let Some((range, cached)) = pending.pop() {
        if spent == max {
            return Ok((spent, None));
        }
        spent += 1; // original derivative/refinement cell, including failure
        let source = if let Some(source) = cached {
            source
        } else {
            let source = vector_certificate::certify_traversal(path, range, max - spent, false)?;
            spent += source.cells;
            source
        };
        if source.status != Status::Certified {
            return Ok((spent, None));
        }
        let bound = bound(&source, station, length, residual).ok();
        if let Some(bound) =
            bound.filter(|b| *b <= target || range[1] - range[0] <= 2_f64.powi(-16))
        {
            error = error.max(bound);
            continue;
        }
        let mid = crate::distance_bounds::midpoint(range[0], range[1]);
        if !(range[0] < mid && mid < range[1]) {
            return Ok((spent, None));
        }
        pending.extend([([range[0], mid], None), ([mid, range[1]], None)]);
    }
    Ok((spent, Some(error)))
}
fn bound(
    source: &vector_certificate::Report,
    station: [f64; 2],
    length: [f64; 2],
    residual: f64,
) -> Result<f64> {
    let decode = |b: [[f64; 2]; 3]| -> Result<[I; 3]> {
        Ok([
            I::new(b[0][0], b[0][1])?,
            I::new(b[1][0], b[1][1])?,
            I::new(b[2][0], b[2][1])?,
        ])
    };
    let v = decode(source.first.unwrap())?;
    let a = decode(source.second.unwrap())?;
    let speed = crate::numerics::interval_vec3::norm(v)?;
    check(
        speed.lo > 0.,
        "Arc chord needs a positive whole-source speed",
    )?;
    let tangent = crate::numerics::interval_vec3::div(v, speed)?;
    let omega = crate::numerics::interval_vec3::div(
        crate::numerics::interval_vec3::cross(v, a)?,
        speed.mul(speed)?,
    )?;
    let l = I::new(length[0], length[1])?;
    // C_s=L T, T_u=omega_u cross T. This formula is invariant under
    // original knot-domain scaling and encloses EVERY source point between
    // the independently certified inverse station brackets.
    let second = crate::numerics::interval_vec3::scale(
        crate::numerics::interval_vec3::cross(omega, tangent)?,
        l.mul(l)?.div(speed)?,
    )?;
    let h = I::point(station[1]).sub(I::point(station[0]))?;
    // The source transport owns exact positional/forward-tangent joins,
    // or this query is a single original span. Thus the piecewise second
    // derivative remainder is valid; bounded one-sided jets alone are not.
    Ok(crate::numerics::interval_vec3::norm(second)?
        .mul(h)?
        .mul(h)?
        .div(I::point(8.))?
        .add(I::point(residual))?
        .hi)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_arc_chord_preserves_domain_units_and_inverse_residual() {
        let path = Curve {
            degree: 2,
            knots: vec![-3., -3., -3., 5., 5., 5.],
            control_points: vec![vec![0.; 3], vec![0., 0., 0.5], vec![0., 0., 2.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let (used, bound) =
            upper(&path, [0.1, 0.3], [0.1, 0.3], [2., 2.], 0.001, false, 1000).unwrap();
        assert!(used > 0);
        let bound = bound.unwrap();
        assert!(bound >= 0.001 && bound < 0.001000000001);
        let (spent, b) = upper(
            &path,
            [0.1, 0.3],
            [0.1, 0.3],
            [2., 2.],
            0.001,
            false,
            used - 1,
        )
        .unwrap();
        assert!(b.is_none() && spent <= used - 1);
        let (spent, b) = upper(&path, [0.1, 0.3], [0.1, 0.3], [2., 2.], 0.001, false, 0).unwrap();
        assert!(b.is_none());
        assert_eq!(spent, 0);
    }
    #[test]
    fn a_corner_with_bounded_one_sided_second_jets_has_no_quadratic_remainder_credit() {
        let path = Curve {
            degree: 1,
            knots: vec![0., 0., 0.5, 1., 1.],
            control_points: vec![vec![0.; 3], vec![1., 0., 0.], vec![1., 1., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        assert!(
            upper(&path, [0.49, 0.51], [0.49, 0.51], [2., 2.], 0., false, 1000)
                .unwrap()
                .1
                .is_none()
        );
    }
}
