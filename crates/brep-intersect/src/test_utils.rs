//! Shared placement helpers for the intersection test modules: exact rigid
//! affine transforms applied to a retained `Model` through
//! `brep_core::transform::affine`.
use brep_core::Model;
use nurbs_core::curve::Curve;

pub(crate) fn translated(model: &Model, offset: [f64; 3]) -> Model {
    brep_core::transform::affine(
        model,
        [
            [1., 0., 0., offset[0]],
            [0., 1., 0., offset[1]],
            [0., 0., 1., offset[2]],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap()
}
pub(crate) fn rotated_translated(model: &Model, angle: f64, offset: [f64; 3]) -> Model {
    let (sin, cos) = angle.sin_cos();
    brep_core::transform::affine(
        model,
        [
            [1., 0., 0., offset[0]],
            [0., cos, -sin, offset[1]],
            [0., sin, cos, offset[2]],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap()
}

/// Degree-1 segment between two points (3D or UV), as the planar test patch needs it.
pub(crate) fn line(a: Vec<f64>, b: Vec<f64>) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a, b],
        weights: vec![1., 1.],
        periodic: false,
    }
}
