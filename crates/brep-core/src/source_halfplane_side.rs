//! Whole-original monotonicity proves the side of an implicit line root.
use crate::source_boundary_fragment::{Endpoint, Fragment, Role};
use nurbs_core::{curve::Curve, Result};
pub struct Report {
    pub proven: bool,
    pub driver_cells: usize,
}
pub fn prove(
    fragment: &Fragment,
    contact: &Curve,
    winding: i32,
    kept: bool,
    max_cells: usize,
) -> Result<Report> {
    let mut out = Report {
        proven: false,
        driver_cells: 0,
    };
    contact.validate()?;
    if contact.control_points[0].len() != 2 || (winding != 1 && winding != -1) || max_cells == 0 {
        return Ok(out);
    }
    let d = contact.domain();
    if contact.degree != 1
        || contact.control_points.len() != 2
        || !contact.knots[..2].iter().all(|&t| t == d[0])
        || !contact.knots[2..].iter().all(|&t| t == d[1])
    {
        return Ok(out);
    }
    let root = fragment
        .endpoints()
        .iter()
        .enumerate()
        .filter_map(|(i, e)| match e {
            Endpoint::Crossing {
                point,
                role: Role::Boundary,
            } if point.boundary() == fragment.curve() && point.contact() == contact => Some(i),
            _ => None,
        })
        .collect::<Vec<_>>();
    if root.len() != 1 || !matches!(fragment.endpoints()[1 - root[0]], Endpoint::Parameter(_)) {
        return Ok(out);
    }
    let a = &contact.control_points[0];
    let b = &contact.control_points[1];
    let Some(axis) = (0..2).find(|&k| a[k] == b[k] && a[1 - k] != b[1 - k]) else {
        return Ok(out);
    };
    // Signed cross-product coefficient of this coordinate; only authored
    // comparisons are used, so no subtractive underflow can erase its sign.
    let coefficient_positive = if axis == 1 {
        (b[0] > a[0]) == (winding > 0)
    } else {
        (b[1] < a[1]) == (winding > 0)
    };
    let driver = nurbs_core::curve_axis_driver::certify(fragment.curve(), axis, max_cells)?;
    out.driver_cells = driver.visited;
    if !driver.monotonic_proven {
        return Ok(out);
    }
    let travel_increases = driver.increasing != fragment.reversed();
    let coordinate_above_root = travel_increases != (root[0] == 1);
    let positive_side = coordinate_above_root == coefficient_positive;
    out.proven = positive_side == kept;
    Ok(out)
}
