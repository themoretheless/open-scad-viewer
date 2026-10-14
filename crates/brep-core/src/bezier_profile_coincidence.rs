//! Common source cut ownership for admitted overlap carriers.
use super::{Curve, Result, refused};
pub(super) fn synchronize(
    curves: &[Curve],
    aliases: &[usize],
    cuts: &mut [Vec<f64>],
    tolerance: f64,
    work: &mut usize,
) -> Result<()> {
    let mut points = Vec::new();
    for (i, c) in curves.iter().enumerate() {
        for t in &cuts[i] {
            points.push((aliases[i], c.evaluate(*t)?.point));
        }
        if points.len() > 8192 {
            return Err(refused("Shared Bézier cut budget exceeded"));
        }
    }
    for (i, c) in curves.iter().enumerate() {
        for (alias, p) in &points {
            if *alias != aliases[i] {
                continue;
            }
            *work += 1;
            if *work > 100000 {
                return Err(refused("Shared Bézier cut projection budget exceeded"));
            }
            for axis in 0..2 {
                if !nurbs_core::intersection::planar_scalar::monotone(c, axis) {
                    continue;
                }
                if let Some(t) = nurbs_core::intersection::planar_scalar::invert(c, axis, p[axis])?
                {
                    let q = c.evaluate(t)?.point;
                    if (p[0] - q[0]).hypot(p[1] - q[1]) <= tolerance {
                        cuts[i].push(t);
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_cuts_use_another_coordinate_when_the_first_inverse_is_flat() {
        let a = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![1., 1.], vec![-1., 2.], vec![3., 3.]],
            weights: vec![1.; 4],
            periodic: false,
        };
        let mut b = a.clone();
        b.knots = vec![2., 2., 2., 2., 5., 5., 5., 5.];
        let mut cuts = vec![vec![0., 1. / 3., 1.], vec![2., 5.]];
        synchronize(&[a, b], &[0, 0], &mut cuts, 1e-7, &mut 0).unwrap();
        assert!(cuts[1].iter().any(|t| (t - 3.).abs() < 1e-12));
    }
}
