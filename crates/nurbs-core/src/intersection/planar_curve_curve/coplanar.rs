//! Exact affine-plane identity on the original Cartesian controls.
use super::*;
use num_rational::BigRational as Q;
use num_traits::{FromPrimitive, Signed, Zero};
fn cross(a: &[Q; 3], b: &[Q; 3]) -> [Q; 3] {
    [
        &a[1] * &b[2] - &a[2] * &b[1],
        &a[2] * &b[0] - &a[0] * &b[2],
        &a[0] * &b[1] - &a[1] * &b[0],
    ]
}
pub(super) fn axes(a: &Curve, b: &Curve) -> Option<[usize; 2]> {
    let points: Vec<[Q; 3]> = a
        .control_points
        .iter()
        .chain(&b.control_points)
        .map(|p| std::array::from_fn(|i| Q::from_f64(p[i]).unwrap()))
        .collect();
    let directions: Vec<[Q; 3]> = points
        .iter()
        .map(|p| std::array::from_fn(|i| &p[i] - &points[0][i]))
        .collect();
    let Some(direction) = directions.iter().find(|v| v.iter().any(|x| !x.is_zero())) else {
        return Some([0, 1]);
    };
    let normal = directions
        .iter()
        .map(|v| cross(direction, v))
        .find(|v| v.iter().any(|x| !x.is_zero()));
    let Some(normal) = normal else {
        // A shared line is still injectively represented by a projection containing
        // one of its nonconstant coordinates. Transverse roots remain rank-deficient.
        let keep = (0..3).max_by_key(|&i| direction[i].abs()).unwrap();
        return Some([keep, (keep + 1) % 3]);
    };
    if !directions
        .iter()
        .all(|v| (0..3).map(|i| &normal[i] * &v[i]).sum::<Q>().is_zero())
    {
        return None;
    }
    // Dropping a coordinate with nonzero normal makes the projection injective
    // on this exact common plane. Two projected coordinates then determine all 3.
    let drop = (0..3).max_by_key(|&i| normal[i].abs()).unwrap();
    let free: Vec<_> = (0..3).filter(|&i| i != drop).collect();
    Some([free[0], free[1]])
}
