//! Typed profile rotation and placement in an authored sketch basis.
use crate::{Mesh, Result, check};
use planar_geometry::sampled_corner;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
}
#[derive(Clone, Copy, Debug)]
pub struct RevolveOptions {
    pub axis: Axis,
    pub offset: f64,
    pub angle: f64,
    pub segments: usize,
}
/// Basis scale is retained; the depth direction is `u × v`.
#[derive(Clone, Copy, Debug)]
pub struct SketchPlane {
    pub origin: [f64; 3],
    pub u: [f64; 3],
    pub v: [f64; 3],
}
impl Default for SketchPlane {
    fn default() -> Self {
        Self {
            origin: [0.; 3],
            u: [1., 0., 0.],
            v: [0., 1., 0.],
        }
    }
}
impl SketchPlane {
    pub fn place(&self, points: &[[f64; 3]]) -> Result<Vec<[f64; 3]>> {
        check(
            points.len() <= 300000,
            "Sketch placement point budget exceeded.",
        )?;
        let normal = crate::cross(self.u, self.v);
        check(
            self.origin
                .into_iter()
                .chain(self.u)
                .chain(self.v)
                .chain(normal)
                .all(f64::is_finite),
            "Sketch geometry exceeds finite numeric range.",
        )?;
        points
            .iter()
            .map(|p| {
                check(
                    p.iter().all(|x| x.is_finite()),
                    "Sketch geometry exceeds finite numeric range.",
                )?;
                let point = std::array::from_fn(|k| {
                    self.origin[k] + p[0] * self.u[k] + p[1] * self.v[k] + p[2] * normal[k]
                });
                check(
                    point.iter().all(|x| x.is_finite()),
                    "Sketch geometry exceeds finite numeric range.",
                )?;
                Ok(point)
            })
            .collect()
    }
}
pub fn revolve(points: &[[f64; 2]], options: RevolveOptions, plane: SketchPlane) -> Result<Mesh> {
    sampled_corner::validate(points)?;
    let RevolveOptions {
        axis,
        offset,
        angle,
        segments,
    } = options;
    if !offset.is_finite()
        || offset.abs() > 1e6
        || !angle.is_finite()
        || angle.abs() < 0.1
        || angle.abs() > 360.
        || !(8..=128).contains(&segments)
    {
        return Err(crate::error(
            "Use a nonzero angle up to 360° and 8–128 segments.",
        ));
    }
    let radial: Vec<f64> = points
        .iter()
        .map(|p| p[usize::from(axis == Axis::X)] - offset)
        .collect();
    let min = radial.iter().copied().fold(f64::INFINITY, f64::min);
    let max = radial.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if min < -1e-8 && max > 1e-8 {
        return Err(crate::error(
            "The profile crosses the rotation axis. Move the axis outside the contour.",
        ));
    }
    let side = if max > 1e-8 { 1. } else { -1. };
    let profile: Vec<_> = points
        .iter()
        .zip(radial)
        .map(|(p, r)| [(side * r).max(0.), p[usize::from(axis == Axis::Y)]])
        .collect();
    let mut mesh = crate::solid::modeling::revolve(&profile, angle, segments, true)?.mesh;
    let local: Vec<[f64; 3]> = mesh
        .positions
        .as_chunks::<3>()
        .0
        .iter()
        .map(|[r, t, h]| {
            if axis == Axis::Y {
                [offset + side * r, *h, -side * t]
            } else {
                [*h, offset + side * r, side * t]
            }
        })
        .collect();
    mesh.positions = plane.place(&local)?.into_iter().flatten().collect();
    Ok(mesh)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn placement_retains_basis_scale_and_depth() {
        let plane = SketchPlane {
            origin: [3., 4., 5.],
            u: [2., 0., 0.],
            v: [0., 3., 0.],
        };
        assert_eq!(plane.place(&[[1., 2., 4.]]).unwrap(), vec![[5., 10., 29.]]);
        assert!(plane.place(&[[f64::NAN, 0., 0.]]).is_err());
    }
    #[test]
    fn revolve_both_axes_sides_and_reject_crossing() {
        for axis in [Axis::X, Axis::Y] {
            for side in [-1., 1.] {
                let points = [
                    [side, side],
                    [2. * side, side],
                    [2. * side, 2. * side],
                    [side, 2. * side],
                ];
                let options = RevolveOptions {
                    axis,
                    offset: 0.,
                    angle: 360.,
                    segments: 16,
                };
                let mesh = revolve(&points, options, SketchPlane::default()).unwrap();
                mesh.validate().unwrap();
                assert!(!mesh.indices.is_empty());
                let shifted = revolve(
                    &points,
                    options,
                    SketchPlane {
                        origin: [0., 0., 10.],
                        ..Default::default()
                    },
                )
                .unwrap();
                assert_eq!(mesh.indices, shifted.indices);
                for (a, b) in mesh
                    .positions
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .zip(shifted.positions.as_chunks::<3>().0.iter())
                {
                    assert_eq!(a[0], b[0]);
                    assert_eq!(a[1], b[1]);
                    assert!((a[2] + 10. - b[2]).abs() < 1e-12);
                }
            }
        }
        assert!(
            revolve(
                &[[-1., 0.], [1., 0.], [1., 2.], [-1., 2.]],
                RevolveOptions {
                    axis: Axis::Y,
                    offset: 0.,
                    angle: 360.,
                    segments: 16
                },
                SketchPlane::default()
            )
            .is_err()
        );
    }
}
