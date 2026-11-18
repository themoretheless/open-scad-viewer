//! Chessboard calibration from detected inner-corner sets (Zhang 2000).
//!
//! Corner detection stays with the host: every view supplies all inner corners of a
//! planar board in row-major order. A closed-form pinhole estimate from plane
//! homographies initializes [`optimize_intrinsics`], which then refines focal lengths,
//! principal point, Brown–Conrady distortion and every board pose with the board
//! points held fixed.
use super::Calibration;
use crate::Result;
use crate::bundle::{
    IntrinsicBundleOptions, IntrinsicBundleSetup, IntrinsicBundleState, IntrinsicsMask,
    Observation, Pose, optimize_intrinsics, project_calibrated,
};
use crate::math::{M3, V3, cross, dot, mm, mv, norm, scale, smallest, svd, tr};

/// Planar board: `columns x rows` inner corners spaced `square` apart, z = 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChessboardPattern {
    pub columns: usize,
    pub rows: usize,
    pub square: f64,
}

impl ChessboardPattern {
    /// Board coordinates of the inner corners in row-major order.
    pub fn points(&self) -> Vec<V3> {
        (0..self.rows)
            .flat_map(|r| {
                (0..self.columns).map(move |c| [c as f64 * self.square, r as f64 * self.square, 0.])
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ChessboardOptions {
    pub mask: IntrinsicsMask,
    pub bundle: IntrinsicBundleOptions,
}

impl Default for ChessboardOptions {
    fn default() -> Self {
        Self {
            mask: IntrinsicsMask::STANDARD,
            bundle: IntrinsicBundleOptions {
                huber_delta: None,
                ..IntrinsicBundleOptions::default()
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChessboardCalibration {
    pub calibration: Calibration,
    /// Board -> camera pose of each view.
    pub poses: Vec<Pose>,
    /// Closed-form zero-distortion estimate used as the starting point.
    pub initial: Calibration,
    pub rms: f64,
    pub view_rms: Vec<f64>,
    pub report: crate::bundle::IntrinsicBundleReport,
}

/// Normalized DLT homography mapping board (x, y) to pixels.
fn homography(board: &[V3], corners: &[[f64; 2]]) -> Option<M3> {
    let normalizer = |points: &mut dyn Iterator<Item = [f64; 2]>| {
        let points: Vec<_> = points.collect();
        let n = points.len() as f64;
        let mean = [0, 1].map(|i| points.iter().map(|p| p[i]).sum::<f64>() / n);
        let spread = points
            .iter()
            .map(|p| ((p[0] - mean[0]).powi(2) + (p[1] - mean[1]).powi(2)).sqrt())
            .sum::<f64>()
            / n;
        let s = 2f64.sqrt() / spread.max(1e-12);
        [[s, 0., -s * mean[0]], [0., s, -s * mean[1]], [0., 0., 1.]]
    };
    let tb = normalizer(&mut board.iter().map(|p| [p[0], p[1]]));
    let ti = normalizer(&mut corners.iter().copied());
    let rows = board.iter().zip(corners).flat_map(|(p, q)| {
        let [x, y, _] = mv(tb, [p[0], p[1], 1.]);
        let [u, v, _] = mv(ti, [q[0], q[1], 1.]);
        [
            [-x, -y, -1., 0., 0., 0., u * x, u * y, u],
            [0., 0., 0., -x, -y, -1., v * x, v * y, v],
        ]
    });
    let h = smallest::<9>(rows);
    let h = [[h[0], h[1], h[2]], [h[3], h[4], h[5]], [h[6], h[7], h[8]]];
    let ti_inverse = [
        [1. / ti[0][0], 0., -ti[0][2] / ti[0][0]],
        [0., 1. / ti[1][1], -ti[1][2] / ti[1][1]],
        [0., 0., 1.],
    ];
    let result = mm(mm(ti_inverse, h), tb);
    result
        .iter()
        .flatten()
        .all(|v| v.is_finite())
        .then_some(result)
}

/// Closed-form zero-skew pinhole from at least three homographies.
fn closed_form(homographies: &[M3]) -> Option<[f64; 4]> {
    let column = |h: &M3, i: usize| [h[0][i], h[1][i], h[2][i]];
    let v = |h: &M3, i: usize, j: usize| {
        let (a, b) = (column(h, i), column(h, j));
        [
            a[0] * b[0],
            a[0] * b[1] + a[1] * b[0],
            a[1] * b[1],
            a[2] * b[0] + a[0] * b[2],
            a[2] * b[1] + a[1] * b[2],
            a[2] * b[2],
        ]
    };
    let rows = homographies.iter().flat_map(|h| {
        let (v11, v22) = (v(h, 0, 0), v(h, 1, 1));
        [v(h, 0, 1), std::array::from_fn(|k| v11[k] - v22[k])]
    });
    let mut b = smallest::<6>(rows);
    if b[0] < 0. {
        b = b.map(|x| -x);
    }
    let [b11, b12, b22, b13, b23, b33] = b;
    let denominator = b11 * b22 - b12 * b12;
    let v0 = (b12 * b13 - b11 * b23) / denominator;
    let lambda = b33 - (b13 * b13 + v0 * (b12 * b13 - b11 * b23)) / b11;
    let alpha = (lambda / b11).sqrt();
    let beta = (lambda * b11 / denominator).sqrt();
    let u0 = -b13 * alpha * alpha / lambda;
    let result = [alpha, beta, u0, v0];
    result
        .iter()
        .all(|x| x.is_finite() && *x > 0.)
        .then_some(result)
}

fn pose_from_homography(k: [f64; 4], h: &M3) -> Option<Pose> {
    let [fx, fy, cx, cy] = k;
    let inverse = |c: V3| [(c[0] - cx * c[2]) / fx, (c[1] - cy * c[2]) / fy, c[2]];
    let column = |i: usize| inverse([h[0][i], h[1][i], h[2][i]]);
    let (h1, h2, h3) = (column(0), column(1), column(2));
    let mut lambda = 1. / norm(h1);
    if h3[2] * lambda < 0. {
        lambda = -lambda;
    }
    let r1 = scale(h1, lambda);
    let r2 = scale(h2, lambda);
    let r3 = cross(r1, r2);
    let (u, _, v) = svd(tr([r1, r2, r3]));
    let mut rotation = mm(u, tr(v));
    if crate::math::det(rotation) < 0. {
        rotation = rotation.map(|r| r.map(|x| -x));
    }
    let pose = Pose {
        rotation,
        translation: scale(h3, lambda),
    };
    (pose
        .rotation
        .iter()
        .flatten()
        .chain(&pose.translation)
        .all(|x| x.is_finite())
        && dot(pose.translation, pose.translation) > 0.)
        .then_some(pose)
}

/// Calibrate one camera from chessboard corner sets observed in an image of
/// `width x height` pixels. At least three views with all inner corners are required.
pub fn calibrate_chessboard(
    pattern: &ChessboardPattern,
    views: &[Vec<[f64; 2]>],
    width: usize,
    height: usize,
    options: &ChessboardOptions,
) -> Result<ChessboardCalibration> {
    let board = pattern.points();
    if pattern.columns < 3
        || pattern.rows < 3
        || !(pattern.square > 0. && pattern.square.is_finite())
        || views.len() < 3
        || views
            .iter()
            .any(|v| v.len() != board.len() || !v.iter().flatten().all(|x| x.is_finite()))
    {
        return Err(crate::error(
            "Chessboard calibration needs at least three views with every inner corner",
        ));
    }
    let homographies = views
        .iter()
        .map(|corners| homography(&board, corners))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| crate::error("Degenerate chessboard homography"))?;
    let k = closed_form(&homographies).ok_or_else(|| {
        crate::error("Chessboard views do not constrain the intrinsics; vary the board tilt")
    })?;
    let initial = Calibration {
        width,
        height,
        fx: k[0],
        fy: k[1],
        cx: k[2],
        cy: k[3],
        k1: 0.,
        k2: 0.,
        k3: 0.,
        p1: 0.,
        p2: 0.,
    };
    initial.validate()?;
    let poses = homographies
        .iter()
        .map(|h| pose_from_homography(k, h))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| crate::error("Chessboard pose initialization failed"))?;
    let observations: Vec<_> = views
        .iter()
        .enumerate()
        .flat_map(|(camera, corners)| {
            corners
                .iter()
                .enumerate()
                .map(move |(point, &xy)| Observation { camera, point, xy })
        })
        .collect();
    let mut state = IntrinsicBundleState {
        intrinsics: vec![initial.clone()],
        poses,
        points: board.clone(),
    };
    let setup = IntrinsicBundleSetup {
        masks: vec![options.mask],
        camera_intrinsics: vec![0; views.len()],
        fixed_poses: vec![false; views.len()],
        fixed_points: true,
        scale_gauge: None,
    };
    let report = optimize_intrinsics(&mut state, &setup, &observations, &options.bundle)?;
    let calibration = state.intrinsics.pop().unwrap();
    calibration.validate()?;
    let view_rms = views
        .iter()
        .zip(&state.poses)
        .map(|(corners, pose)| {
            let sum: f64 = corners
                .iter()
                .zip(&board)
                .map(|(q, &p)| {
                    project_calibrated(&calibration, pose, p).map_or(f64::INFINITY, |r| {
                        (r[0] - q[0]).powi(2) + (r[1] - q[1]).powi(2)
                    })
                })
                .sum();
            (sum / corners.len() as f64).sqrt()
        })
        .collect();
    Ok(ChessboardCalibration {
        calibration,
        poses: state.poses,
        initial,
        rms: report.final_rms,
        view_rms,
        report,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::rotation;

    pub(crate) struct Gauss(u64);
    impl Gauss {
        pub fn next(&mut self) -> f64 {
            let mut uniform = || {
                self.0 = self
                    .0
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                ((self.0 >> 11) as f64 + 0.5) / (1u64 << 53) as f64
            };
            let (a, b) = (uniform(), uniform());
            (-2. * a.ln()).sqrt() * (std::f64::consts::TAU * b).cos()
        }
    }

    fn truth() -> Calibration {
        Calibration {
            width: 640,
            height: 480,
            fx: 812.,
            fy: 798.,
            cx: 331.5,
            cy: 244.,
            k1: -0.18,
            k2: 0.07,
            k3: 0.,
            p1: 0.0012,
            p2: -0.0009,
        }
    }

    #[test]
    fn chessboard_calibration_recovers_known_intrinsics() {
        let truth = truth();
        let pattern = ChessboardPattern {
            columns: 9,
            rows: 6,
            square: 0.03,
        };
        let board = pattern.points();
        let center = [0.12, 0.075, 0.];
        let mut noise = Gauss(7);
        let tilts = [
            [0.0, 0.0, 0.0],
            [0.45, 0.0, 0.1],
            [-0.4, 0.1, -0.1],
            [0.0, 0.45, 0.2],
            [0.1, -0.45, 0.0],
            [0.35, 0.35, -0.3],
            [-0.3, -0.35, 0.4],
            [0.2, -0.2, 1.2],
        ];
        let views: Vec<Vec<[f64; 2]>> = tilts
            .iter()
            .enumerate()
            .map(|(i, &w)| {
                let r = rotation(w);
                let distance = 0.42 + 0.03 * i as f64;
                let offset = [
                    0.015 * (i % 3) as f64 - 0.015,
                    0.01 * (i % 2) as f64,
                    distance,
                ];
                // Board centre is placed at `offset` in camera coordinates.
                let t = crate::math::sub(offset, mv(r, center));
                let pose = Pose {
                    rotation: r,
                    translation: t,
                };
                board
                    .iter()
                    .map(|&p| {
                        let q = project_calibrated(&truth, &pose, p).unwrap();
                        assert!(q[0] > 0. && q[0] < 640. && q[1] > 0. && q[1] < 480.);
                        [q[0] + 0.1 * noise.next(), q[1] + 0.1 * noise.next()]
                    })
                    .collect()
            })
            .collect();
        let result =
            calibrate_chessboard(&pattern, &views, 640, 480, &ChessboardOptions::default())
                .unwrap();
        let c = &result.calibration;
        assert!((c.fx - truth.fx).abs() / truth.fx < 0.005, "fx {}", c.fx);
        assert!((c.fy - truth.fy).abs() / truth.fy < 0.005, "fy {}", c.fy);
        assert!((c.cx - truth.cx).abs() < 2., "cx {}", c.cx);
        assert!((c.cy - truth.cy).abs() < 2., "cy {}", c.cy);
        assert!((c.k1 - truth.k1).abs() < 0.02, "k1 {}", c.k1);
        assert!((c.k2 - truth.k2).abs() < 0.08, "k2 {}", c.k2);
        assert!((c.p1 - truth.p1).abs() < 0.001, "p1 {}", c.p1);
        assert!((c.p2 - truth.p2).abs() < 0.001, "p2 {}", c.p2);
        assert!(result.rms < 0.15, "rms {}", result.rms);
        assert!(result.report.initial_rms > 5. * result.rms);
        assert!(result.view_rms.iter().all(|&r| r < 0.2));
    }

    #[test]
    fn chessboard_calibration_rejects_too_few_views() {
        let pattern = ChessboardPattern {
            columns: 4,
            rows: 3,
            square: 1.,
        };
        let view = vec![[0.; 2]; 12];
        assert!(
            calibrate_chessboard(
                &pattern,
                &[view.clone(), view],
                640,
                480,
                &ChessboardOptions::default()
            )
            .is_err()
        );
    }
}
