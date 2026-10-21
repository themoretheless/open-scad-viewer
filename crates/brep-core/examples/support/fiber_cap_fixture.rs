//! Analytic curved cap on the actual shared-root, four-face body network.
#[path = "fiber_cut_fixture.rs"]
mod sheets;
#[path = "curved_trim_fixture.rs"]
mod stock;
use brep_core::{
    Model,
    face_cut::{FiberCapRequest, FiberFaceRequest},
};
use nurbs_core::{ss_intersection::FiberAxis, surface::Surface};
pub fn build() -> (Model, Model, Vec<FiberCapRequest>) {
    let square = [1., -2., 4.];
    let cutter = sheets::sheet(Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        vec![
                            -1. + 1.5 * i as f64,
                            -1. + 1.5 * j as f64,
                            -0.2 + 0.125 * square[i] + 0.02 * square[j],
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    });
    let requests = [2, 3, 4, 5]
        .map(|face| FiberCapRequest {
            section: FiberFaceRequest {
                face,
                parameter_box: [[0., 1.], [0.1, 0.95], [0.25, 0.75], [0.25, 0.75]],
                axis: FiberAxis::FirstU,
            },
            remove_uv: [0.5, 0.95],
        })
        .into_iter()
        .collect();
    (stock::build(), cutter, requests)
}
