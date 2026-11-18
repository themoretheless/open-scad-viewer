//! Independent multi-view pose verification.
//!
//! A pose is checked only against evidence it did not produce: for every track seen
//! by the camera under test, a point is triangulated from a pair of *other* cameras
//! (with a minimum triangulation angle and positive depth in both), then projected
//! into the tested camera. The pair agreeing with most other partners is used, which
//! tolerates a minority of bad partner poses. The residual distribution and cheirality failures
//! decide whether the pose is consistent; too little evidence is reported as such,
//! never as success.
use crate::bundle::{Observation, Pose, project_calibrated};
use crate::calibration::Calibration;
use crate::math::{V3, dot, norm, smallest, sub};

impl Calibration {
    /// Measured pixel -> ideal normalized ray (inverse of [`Calibration::project_ray`]).
    pub fn unproject_pixel(&self, pixel: [f64; 2]) -> Option<[f64; 2]> {
        let target = [
            (pixel[0] - self.cx) / self.fx,
            (pixel[1] - self.cy) / self.fy,
        ];
        let mut ray = target;
        for _ in 0..50 {
            let [x, y] = ray;
            let r2 = x * x + y * y;
            let radial = 1. + r2 * (self.k1 + r2 * (self.k2 + r2 * self.k3));
            let dx = 2. * self.p1 * x * y + self.p2 * (r2 + 2. * x * x);
            let dy = self.p1 * (r2 + 2. * y * y) + 2. * self.p2 * x * y;
            ray = [(target[0] - dx) / radial, (target[1] - dy) / radial];
            if !ray.iter().all(|v| v.is_finite()) {
                return None;
            }
        }
        let back = self.project_ray(ray)?;
        ((back[0] - pixel[0]).hypot(back[1] - pixel[1]) < 1e-6 * self.fx.max(1.)).then_some(ray)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PoseVerificationOptions {
    /// Minimum angle between the two partner rays at the triangulated point.
    pub min_triangulation_degrees: f64,
    /// Partner pairs examined per track (bounded work).
    pub max_pairs_per_track: usize,
    /// Tracks with fewer checks leave the camera `Insufficient`.
    pub min_checks: usize,
    /// Median reprojection residual above which a pose is inconsistent (pixels).
    pub max_median_residual: f64,
    /// Fraction of checks with the point behind the tested camera that fails the pose.
    pub max_cheirality_fraction: f64,
}

impl Default for PoseVerificationOptions {
    fn default() -> Self {
        Self {
            min_triangulation_degrees: 2.,
            max_pairs_per_track: 12,
            min_checks: 10,
            max_median_residual: 2.,
            max_cheirality_fraction: 0.1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoseStatus {
    Consistent,
    Inconsistent,
    /// Not enough independent evidence; neither accepted nor rejected.
    Insufficient,
    /// No pose to verify.
    Missing,
}

#[derive(Clone, Debug)]
pub struct CameraVerification {
    pub camera: usize,
    pub status: PoseStatus,
    pub reason: &'static str,
    /// Tracks with at least one valid partner triangulation.
    pub checks: usize,
    pub median_residual: f64,
    pub p90_residual: f64,
    pub cheirality_failures: usize,
    pub median_triangulation_degrees: f64,
}

#[derive(Clone, Debug)]
pub struct PoseVerificationReport {
    pub cameras: Vec<CameraVerification>,
}

impl PoseVerificationReport {
    pub fn inconsistent(&self) -> Vec<usize> {
        self.cameras
            .iter()
            .filter(|c| c.status == PoseStatus::Inconsistent)
            .map(|c| c.camera)
            .collect()
    }
}

/// (camera, measured pixel, ideal normalized ray).
type TrackView = (usize, [f64; 2], [f64; 2]);

/// Linear two-view triangulation of normalized rays.
fn triangulate_pair(a: (&Pose, [f64; 2]), b: (&Pose, [f64; 2])) -> Option<V3> {
    let rows = [a, b].into_iter().flat_map(|(pose, [x, y])| {
        let r = pose.rotation;
        let t = pose.translation;
        let p = |i: usize| [r[i][0], r[i][1], r[i][2], t[i]];
        let (p0, p1, p2) = (p(0), p(1), p(2));
        [
            std::array::from_fn::<f64, 4, _>(|k| x * p2[k] - p0[k]),
            std::array::from_fn::<f64, 4, _>(|k| y * p2[k] - p1[k]),
        ]
    });
    let h = smallest::<4>(rows);
    if h[3].abs() < 1e-12 {
        return None;
    }
    let point = [h[0] / h[3], h[1] / h[3], h[2] / h[3]];
    (point.iter().all(|v| v.is_finite())
        && a.0.camera_point(point)[2] > 0.
        && b.0.camera_point(point)[2] > 0.)
        .then_some(point)
}

fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    sorted[((sorted.len() - 1) as f64 * q).round() as usize]
}

/// Verify every pose against triangulations from the other cameras.
/// `camera_intrinsics[c]` selects the calibration of camera `c`.
pub fn verify_poses(
    intrinsics: &[Calibration],
    camera_intrinsics: &[usize],
    poses: &[Option<Pose>],
    observations: &[Observation],
    options: &PoseVerificationOptions,
) -> crate::Result<PoseVerificationReport> {
    if camera_intrinsics.len() != poses.len()
        || camera_intrinsics.iter().any(|&b| b >= intrinsics.len())
        || observations.iter().any(|o| o.camera >= poses.len())
        || options.max_pairs_per_track == 0
        || !(options.min_triangulation_degrees >= 0.)
    {
        return Err(crate::error("Pose verification inputs do not match"));
    }
    let points = observations.iter().map(|o| o.point + 1).max().unwrap_or(0);
    // Track -> (camera, pixel, normalized ray) for posed cameras only.
    let mut tracks: Vec<Vec<TrackView>> = vec![Vec::new(); points];
    for o in observations {
        if poses[o.camera].is_some()
            && let Some(ray) = intrinsics[camera_intrinsics[o.camera]].unproject_pixel(o.xy)
        {
            tracks[o.point].push((o.camera, o.xy, ray));
        }
    }
    let min_cos = options.min_triangulation_degrees.to_radians().cos();
    let cameras = (0..poses.len())
        .map(|camera| {
            let mut residuals = Vec::new();
            let mut angles = Vec::new();
            let mut cheirality_failures = 0;
            let Some(pose) = &poses[camera] else {
                return CameraVerification {
                    camera,
                    status: PoseStatus::Missing,
                    reason: "No pose",
                    checks: 0,
                    median_residual: f64::NAN,
                    p90_residual: f64::NAN,
                    cheirality_failures: 0,
                    median_triangulation_degrees: f64::NAN,
                };
            };
            let calibration = &intrinsics[camera_intrinsics[camera]];
            for track in &tracks {
                let Some(&(_, pixel, _)) = track.iter().find(|t| t.0 == camera) else {
                    continue;
                };
                let partners: Vec<_> = track.iter().filter(|t| t.0 != camera).collect();
                // Best-supported partner pair: support counts partners that agree with
                // the triangulated point, so one bad partner pose cannot dominate.
                let mut best: Option<(usize, f64, V3, f64)> = None;
                let mut pairs = 0;
                'pairs: for i in 0..partners.len() {
                    for j in i + 1..partners.len() {
                        if pairs >= options.max_pairs_per_track {
                            break 'pairs;
                        }
                        let (a, b) = (partners[i], partners[j]);
                        let (pa, pb) = (poses[a.0].as_ref().unwrap(), poses[b.0].as_ref().unwrap());
                        let Some(point) = triangulate_pair((pa, a.2), (pb, b.2)) else {
                            continue;
                        };
                        let (ra, rb) = (sub(point, pa.center()), sub(point, pb.center()));
                        let cos = dot(ra, rb) / (norm(ra) * norm(rb));
                        if !(cos <= min_cos) {
                            continue;
                        }
                        pairs += 1;
                        let mut support = 0;
                        let mut spread = 0.;
                        for p in &partners {
                            let partner = poses[p.0].as_ref().unwrap();
                            let error = project_calibrated(
                                &intrinsics[camera_intrinsics[p.0]],
                                partner,
                                point,
                            )
                            .map_or(f64::INFINITY, |q| (q[0] - p.1[0]).hypot(q[1] - p.1[1]));
                            if error <= options.max_median_residual {
                                support += 1;
                                spread += error;
                            }
                        }
                        let angle = cos.clamp(-1., 1.).acos().to_degrees();
                        if best
                            .is_none_or(|(s, e, _, _)| support > s || (support == s && spread < e))
                        {
                            best = Some((support, spread, point, angle));
                        }
                    }
                }
                let Some((support, _, point, angle)) = best else {
                    continue;
                };
                if support < 2 {
                    continue;
                }
                if pose.camera_point(point)[2] <= 0. {
                    cheirality_failures += 1;
                    continue;
                }
                residuals.push(
                    project_calibrated(calibration, pose, point)
                        .map_or(f64::INFINITY, |q| (q[0] - pixel[0]).hypot(q[1] - pixel[1])),
                );
                angles.push(angle);
            }
            let checks = residuals.len() + cheirality_failures;
            residuals.sort_by(f64::total_cmp);
            let median_residual = quantile(&residuals, 0.5);
            let p90_residual = quantile(&residuals, 0.9);
            let median_triangulation_degrees = median(&mut angles);
            let (status, reason) = if checks < options.min_checks {
                (
                    PoseStatus::Insufficient,
                    "Too few independently triangulated tracks",
                )
            } else if cheirality_failures as f64 > options.max_cheirality_fraction * checks as f64 {
                (
                    PoseStatus::Inconsistent,
                    "Points triangulated by other views lie behind the camera",
                )
            } else if !(median_residual <= options.max_median_residual) {
                (
                    PoseStatus::Inconsistent,
                    "Median reprojection residual exceeds the limit",
                )
            } else {
                (PoseStatus::Consistent, "Consistent with the other views")
            };
            CameraVerification {
                camera,
                status,
                reason,
                checks,
                median_residual,
                p90_residual,
                cheirality_failures,
                median_triangulation_degrees,
            }
        })
        .collect();
    Ok(PoseVerificationReport { cameras })
}
