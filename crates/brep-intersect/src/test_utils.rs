//! Shared placement helpers for the intersection test modules: exact rigid
//! affine transforms applied to a retained `Model` through
//! `brep_core::transform::affine`.
use brep_core::Model;

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
pub(crate) use nurbs_core::curve::line;
