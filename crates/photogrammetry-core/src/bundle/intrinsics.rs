//! Joint refinement of Brown–Conrady intrinsics, poses and points.
//!
//! Complements [`optimize`](super::optimize), which keeps the pinhole intrinsics fixed.
//! Every observation belongs to one camera; each camera references one intrinsics block,
//! so intrinsics may be shared by a series or kept per camera. Individual parameter
//! groups of every block, individual poses and all points can be fixed. Points are
//! eliminated as 3x3 blocks (Schur complement); the reduced intrinsics/pose system is
//! dense and bounded by [`IntrinsicBundleOptions::max_parameters`]. Jacobians use
//! central differences of the exact projection, which keeps the distortion model in
//! one place ([`Calibration::project_ray`]). Inputs change only on success.
use super::{BundleTermination, Observation};
use crate::Result;
use crate::calibration::Calibration;
use crate::camera::Camera;
use crate::math::{M3, V3, add, mm, mv, norm, rotation, scale, sub, tr};

/// World -> camera rigid transform: `x_cam = R x + t`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub rotation: M3,
    pub translation: V3,
}

impl Pose {
    pub fn from_camera(camera: &Camera) -> Self {
        Self {
            rotation: camera.rotation,
            translation: camera.translation,
        }
    }
    pub fn camera_point(&self, point: V3) -> V3 {
        add(mv(self.rotation, point), self.translation)
    }
    pub fn center(&self) -> V3 {
        scale(mv(tr(self.rotation), self.translation), -1.)
    }
    /// Left SE(3) update, identical to the fixed-intrinsics bundle.
    pub(crate) fn perturbed(&self, delta: &[f64]) -> Self {
        let dr = rotation([delta[0], delta[1], delta[2]]);
        Self {
            rotation: mm(dr, self.rotation),
            translation: add(mv(dr, self.translation), [delta[3], delta[4], delta[5]]),
        }
    }
    fn finite(&self) -> bool {
        self.rotation
            .iter()
            .flatten()
            .chain(&self.translation)
            .all(|v| v.is_finite())
    }
}

/// Project a world point through a pose and a Brown–Conrady calibration.
/// Points behind (or on) the camera plane and folded distortion maps return `None`.
pub fn project_calibrated(calibration: &Calibration, pose: &Pose, point: V3) -> Option<[f64; 2]> {
    let [x, y, z] = pose.camera_point(point);
    if !(z > 1e-12) {
        return None;
    }
    calibration.project_ray([x / z, y / z])
}

/// Parameter order used by masks and steps: fx, fy, cx, cy, k1, k2, k3, p1, p2.
pub const INTRINSIC_PARAMETERS: usize = 9;

pub(crate) fn intrinsic(calibration: &Calibration, index: usize) -> f64 {
    [
        calibration.fx,
        calibration.fy,
        calibration.cx,
        calibration.cy,
        calibration.k1,
        calibration.k2,
        calibration.k3,
        calibration.p1,
        calibration.p2,
    ][index]
}

pub(crate) fn set_intrinsic(calibration: &mut Calibration, index: usize, value: f64) {
    *[
        &mut calibration.fx,
        &mut calibration.fy,
        &mut calibration.cx,
        &mut calibration.cy,
        &mut calibration.k1,
        &mut calibration.k2,
        &mut calibration.k3,
        &mut calibration.p1,
        &mut calibration.p2,
    ][index] = value;
}

/// Which intrinsic parameter groups of one block are optimized.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntrinsicsMask {
    pub focal: bool,
    pub principal_point: bool,
    /// k1 and k2.
    pub radial: bool,
    pub k3: bool,
    /// p1 and p2.
    pub tangential: bool,
}

impl IntrinsicsMask {
    pub const FIXED: Self = Self {
        focal: false,
        principal_point: false,
        radial: false,
        k3: false,
        tangential: false,
    };
    /// Focal, principal point, k1/k2 and p1/p2; k3 stays fixed (usually poorly observed).
    pub const STANDARD: Self = Self {
        focal: true,
        principal_point: true,
        radial: true,
        k3: false,
        tangential: true,
    };
    pub const ALL: Self = Self {
        k3: true,
        ..Self::STANDARD
    };
    pub fn free(&self, index: usize) -> bool {
        match index {
            0 | 1 => self.focal,
            2 | 3 => self.principal_point,
            4 | 5 => self.radial,
            6 => self.k3,
            _ => self.tangential,
        }
    }
}

/// Which blocks are optimized and how cameras share intrinsics.
#[derive(Clone, Debug)]
pub struct IntrinsicBundleSetup {
    /// One mask per intrinsics block.
    pub masks: Vec<IntrinsicsMask>,
    /// Intrinsics block of every camera.
    pub camera_intrinsics: Vec<usize>,
    /// Poses kept constant; at least one is required when points are free.
    pub fixed_poses: Vec<bool>,
    /// Keep all points constant (for example a known calibration target).
    pub fixed_points: bool,
    /// `[anchor, scale]`: after each step free poses and points are scaled about the
    /// fixed anchor centre so the anchor-to-scale camera baseline is preserved.
    pub scale_gauge: Option<[usize; 2]>,
}

/// Current estimate. Updated in place only when [`optimize_intrinsics`] succeeds.
#[derive(Clone, Debug)]
pub struct IntrinsicBundleState {
    pub intrinsics: Vec<Calibration>,
    pub poses: Vec<Pose>,
    pub points: Vec<V3>,
}

#[derive(Clone, Copy, Debug)]
pub struct IntrinsicBundleOptions {
    pub max_iterations: usize,
    pub max_trials: usize,
    pub initial_damping: f64,
    pub relative_cost_tolerance: f64,
    /// Huber threshold in pixels; `None` is plain least squares.
    pub huber_delta: Option<f64>,
    /// Bound on free intrinsics + pose parameters (dense reduced system).
    pub max_parameters: usize,
    pub max_observations: usize,
}

impl Default for IntrinsicBundleOptions {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            max_trials: 10,
            initial_damping: 1e-3,
            relative_cost_tolerance: 1e-12,
            huber_delta: Some(4.),
            max_parameters: 1200,
            max_observations: 400_000,
        }
    }
}

#[derive(Clone, Debug)]
pub struct IntrinsicBundleReport {
    /// Root mean square pixel residual over all observations (unweighted).
    pub initial_rms: f64,
    pub final_rms: f64,
    pub initial_cost: f64,
    pub final_cost: f64,
    pub iterations: usize,
    pub accepted_steps: usize,
    pub free_parameters: usize,
    pub termination: BundleTermination,
}

struct Layout {
    intrinsic_columns: Vec<[Option<usize>; INTRINSIC_PARAMETERS]>,
    pose_columns: Vec<Option<usize>>,
    dimension: usize,
    tracks: Vec<Vec<usize>>,
}

fn validate(
    state: &IntrinsicBundleState,
    setup: &IntrinsicBundleSetup,
    observations: &[Observation],
    options: &IntrinsicBundleOptions,
) -> Result<Layout> {
    let fail = |message: &str| Err(crate::error(message));
    if options.max_iterations == 0
        || options.max_trials == 0
        || !(options.initial_damping > 0. && options.initial_damping.is_finite())
        || !(options.relative_cost_tolerance >= 0.)
        || options
            .huber_delta
            .is_some_and(|d| !(d > 0. && d.is_finite()))
    {
        return fail("Invalid intrinsic bundle options");
    }
    let cameras = state.poses.len();
    if setup.masks.len() != state.intrinsics.len()
        || setup.camera_intrinsics.len() != cameras
        || setup.fixed_poses.len() != cameras
        || setup
            .camera_intrinsics
            .iter()
            .any(|&b| b >= state.intrinsics.len())
    {
        return fail("Intrinsic bundle setup does not match the state");
    }
    if observations.is_empty() || observations.len() > options.max_observations {
        return fail("Intrinsic bundle needs between one and max_observations observations");
    }
    if state.intrinsics.iter().any(|c| c.validate().is_err())
        || !state.poses.iter().all(Pose::finite)
        || !state.points.iter().flatten().all(|v| v.is_finite())
    {
        return fail("Intrinsic bundle state must be finite and valid");
    }
    if !setup.fixed_points && !setup.fixed_poses.iter().any(|&f| f) {
        return fail("Free points require at least one fixed pose to define the gauge");
    }
    if let Some([anchor, other]) = setup.scale_gauge
        && (anchor >= cameras
            || other >= cameras
            || anchor == other
            || !setup.fixed_poses[anchor]
            || setup.fixed_poses[other]
            || setup.fixed_points
            || norm(sub(
                state.poses[anchor].center(),
                state.poses[other].center(),
            )) <= 1e-10)
    {
        return fail("Scale gauge needs a fixed anchor, a free distinct camera and free points");
    }
    let mut tracks = vec![Vec::new(); state.points.len()];
    for (index, o) in observations.iter().enumerate() {
        if o.camera >= cameras
            || o.point >= state.points.len()
            || !o.xy.iter().all(|v| v.is_finite())
        {
            return fail("Observation references an unknown camera or point");
        }
        let calibration = &state.intrinsics[setup.camera_intrinsics[o.camera]];
        if project_calibrated(calibration, &state.poses[o.camera], state.points[o.point]).is_none()
        {
            return fail("Observation is not projectable in the initial state");
        }
        tracks[o.point].push(index);
    }
    let mut dimension = 0;
    let mut used_blocks = vec![false; state.intrinsics.len()];
    let mut used_cameras = vec![false; cameras];
    for o in observations {
        used_cameras[o.camera] = true;
        used_blocks[setup.camera_intrinsics[o.camera]] = true;
    }
    let intrinsic_columns = setup
        .masks
        .iter()
        .zip(&used_blocks)
        .map(|(mask, &used)| {
            std::array::from_fn(|i| {
                (used && mask.free(i)).then(|| {
                    dimension += 1;
                    dimension - 1
                })
            })
        })
        .collect();
    let pose_columns = (0..cameras)
        .map(|c| {
            (used_cameras[c] && !setup.fixed_poses[c]).then(|| {
                dimension += 6;
                dimension - 6
            })
        })
        .collect();
    if dimension > options.max_parameters {
        return fail("Intrinsic bundle exceeds max_parameters");
    }
    if dimension == 0 && setup.fixed_points {
        return fail("Intrinsic bundle has no free parameters");
    }
    tracks.retain(|t| !t.is_empty());
    Ok(Layout {
        intrinsic_columns,
        pose_columns,
        dimension,
        tracks,
    })
}

fn residual(
    state: &IntrinsicBundleState,
    setup: &IntrinsicBundleSetup,
    o: &Observation,
) -> Option<[f64; 2]> {
    let calibration = &state.intrinsics[setup.camera_intrinsics[o.camera]];
    let p = project_calibrated(calibration, &state.poses[o.camera], state.points[o.point])?;
    Some([p[0] - o.xy[0], p[1] - o.xy[1]])
}

/// Returns (weighted cost, squared residual sum).
fn cost(
    state: &IntrinsicBundleState,
    setup: &IntrinsicBundleSetup,
    observations: &[Observation],
    delta: Option<f64>,
) -> Option<(f64, f64)> {
    let mut robust = 0.;
    let mut squared = 0.;
    for o in observations {
        let r = residual(state, setup, o)?;
        let s = r[0] * r[0] + r[1] * r[1];
        squared += s;
        robust += match delta {
            Some(d) if s.sqrt() > d => 2. * d * s.sqrt() - d * d,
            _ => s,
        };
    }
    (robust.is_finite()).then_some((robust, squared))
}

fn weight(r: [f64; 2], delta: Option<f64>) -> f64 {
    let n = (r[0] * r[0] + r[1] * r[1]).sqrt();
    match delta {
        Some(d) if n > d => d / n,
        _ => 1.,
    }
}

struct PointSystem {
    point: usize,
    hessian: M3,
    rhs: V3,
    /// Sorted, merged `(column, d(column)ᵀ d(point))` cross blocks.
    cross: Vec<(usize, V3)>,
}

struct Linearization {
    hessian: Vec<f64>,
    rhs: Vec<f64>,
    points: Vec<PointSystem>,
}

/// Central-difference Jacobians of one observation residual.
fn observation_jacobian(
    state: &IntrinsicBundleState,
    setup: &IntrinsicBundleSetup,
    layout: &Layout,
    o: &Observation,
    columns: &mut Vec<(usize, [f64; 2])>,
) -> Option<[[f64; 2]; 3]> {
    columns.clear();
    let block = setup.camera_intrinsics[o.camera];
    let calibration = &state.intrinsics[block];
    let pose = &state.poses[o.camera];
    let point = state.points[o.point];
    let diff =
        |a: [f64; 2], b: [f64; 2], h: f64| [(a[0] - b[0]) / (2. * h), (a[1] - b[1]) / (2. * h)];
    for (index, column) in layout.intrinsic_columns[block].iter().enumerate() {
        let Some(column) = *column else { continue };
        let value = intrinsic(calibration, index);
        let h = if index < 4 {
            1e-6 * value.abs().max(1.)
        } else {
            1e-7
        };
        let mut plus = calibration.clone();
        set_intrinsic(&mut plus, index, value + h);
        let mut minus = calibration.clone();
        set_intrinsic(&mut minus, index, value - h);
        columns.push((
            column,
            diff(
                project_calibrated(&plus, pose, point)?,
                project_calibrated(&minus, pose, point)?,
                h,
            ),
        ));
    }
    if let Some(column) = layout.pose_columns[o.camera] {
        let size = norm(pose.translation).max(1.);
        for i in 0..6 {
            let h = if i < 3 { 1e-7 } else { 1e-7 * size };
            let mut delta = [0.; 6];
            delta[i] = h;
            let plus = project_calibrated(calibration, &pose.perturbed(&delta), point)?;
            delta[i] = -h;
            let minus = project_calibrated(calibration, &pose.perturbed(&delta), point)?;
            columns.push((column + i, diff(plus, minus, h)));
        }
    }
    let size = norm(point).max(1.);
    let mut jp = [[0.; 2]; 3];
    if !setup.fixed_points {
        for (i, column) in jp.iter_mut().enumerate() {
            let h = 1e-7 * size;
            let mut plus = point;
            plus[i] += h;
            let mut minus = point;
            minus[i] -= h;
            *column = diff(
                project_calibrated(calibration, pose, plus)?,
                project_calibrated(calibration, pose, minus)?,
                h,
            );
        }
    }
    Some(jp)
}

fn linearize(
    state: &IntrinsicBundleState,
    setup: &IntrinsicBundleSetup,
    observations: &[Observation],
    layout: &Layout,
    delta: Option<f64>,
) -> Option<Linearization> {
    let n = layout.dimension;
    let mut result = Linearization {
        hessian: vec![0.; n * n],
        rhs: vec![0.; n],
        points: Vec::new(),
    };
    let mut columns = Vec::with_capacity(15);
    for track in &layout.tracks {
        let mut system = PointSystem {
            point: observations[track[0]].point,
            hessian: [[0.; 3]; 3],
            rhs: [0.; 3],
            cross: Vec::new(),
        };
        for &index in track {
            let o = &observations[index];
            let r = residual(state, setup, o)?;
            let w = weight(r, delta);
            let jp = observation_jacobian(state, setup, layout, o, &mut columns)?;
            for &(a, ja) in &columns {
                result.rhs[a] -= w * (ja[0] * r[0] + ja[1] * r[1]);
                for &(b, jb) in &columns {
                    result.hessian[a * n + b] += w * (ja[0] * jb[0] + ja[1] * jb[1]);
                }
                if !setup.fixed_points {
                    let values = std::array::from_fn(|k| w * (ja[0] * jp[k][0] + ja[1] * jp[k][1]));
                    system.cross.push((a, values));
                }
            }
            if !setup.fixed_points {
                for i in 0..3 {
                    system.rhs[i] -= w * (jp[i][0] * r[0] + jp[i][1] * r[1]);
                    for j in 0..3 {
                        system.hessian[i][j] += w * (jp[i][0] * jp[j][0] + jp[i][1] * jp[j][1]);
                    }
                }
            }
        }
        if !setup.fixed_points {
            system.cross.sort_by_key(|c| c.0);
            let mut merged: Vec<(usize, V3)> = Vec::with_capacity(system.cross.len());
            for (column, values) in system.cross.drain(..) {
                match merged.last_mut() {
                    Some(last) if last.0 == column => last.1 = add(last.1, values),
                    _ => merged.push((column, values)),
                }
            }
            system.cross = merged;
            result.points.push(system);
        }
    }
    Some(result)
}

fn invert3(mut h: M3, damping: f64) -> Option<M3> {
    for (i, row) in h.iter_mut().enumerate() {
        row[i] += damping * row[i] + 1e-12;
    }
    let mut inverse = [[0.; 3]; 3];
    for k in 0..3 {
        let mut e = [0.; 3];
        e[k] = 1.;
        let column = crate::math::solve(h, e)?;
        for i in 0..3 {
            inverse[i][k] = column[i];
        }
    }
    Some(inverse)
}

/// In-place dense Cholesky solve of a symmetric positive definite system.
pub(crate) fn cholesky_solve(a: &mut [f64], b: &mut [f64]) -> Option<()> {
    let n = b.len();
    for j in 0..n {
        let mut d = a[j * n + j];
        for k in 0..j {
            d -= a[j * n + k] * a[j * n + k];
        }
        if !(d > 0.) || !d.is_finite() {
            return None;
        }
        let d = d.sqrt();
        a[j * n + j] = d;
        for i in j + 1..n {
            let mut s = a[i * n + j];
            for k in 0..j {
                s -= a[i * n + k] * a[j * n + k];
            }
            a[i * n + j] = s / d;
        }
    }
    for i in 0..n {
        let mut s = b[i];
        for k in 0..i {
            s -= a[i * n + k] * b[k];
        }
        b[i] = s / a[i * n + i];
    }
    for i in (0..n).rev() {
        let mut s = b[i];
        for k in i + 1..n {
            s -= a[k * n + i] * b[k];
        }
        b[i] = s / a[i * n + i];
    }
    b.iter().all(|v| v.is_finite()).then_some(())
}

fn step(linear: &Linearization, damping: f64) -> Option<(Vec<f64>, Vec<V3>)> {
    let n = linear.rhs.len();
    let mut hessian = linear.hessian.clone();
    let mut rhs = linear.rhs.clone();
    for i in 0..n {
        hessian[i * n + i] += damping * hessian[i * n + i] + 1e-12;
    }
    let mut inverses = Vec::with_capacity(linear.points.len());
    for point in &linear.points {
        let inverse = invert3(point.hessian, damping)?;
        for &(a, wa) in &point.cross {
            let wv = [0, 1, 2].map(|k| (0..3).map(|i| wa[i] * inverse[i][k]).sum::<f64>());
            rhs[a] -= (0..3).map(|k| wv[k] * point.rhs[k]).sum::<f64>();
            for &(b, wb) in &point.cross {
                hessian[a * n + b] -= (0..3).map(|k| wv[k] * wb[k]).sum::<f64>();
            }
        }
        inverses.push(inverse);
    }
    if n > 0 {
        cholesky_solve(&mut hessian, &mut rhs)?;
    }
    let points = linear
        .points
        .iter()
        .zip(&inverses)
        .map(|(point, inverse)| {
            let mut b = point.rhs;
            for &(a, wa) in &point.cross {
                for k in 0..3 {
                    b[k] -= wa[k] * rhs[a];
                }
            }
            mv(*inverse, b)
        })
        .collect();
    Some((rhs, points))
}

fn apply(
    state: &IntrinsicBundleState,
    setup: &IntrinsicBundleSetup,
    layout: &Layout,
    linear: &Linearization,
    delta: &(Vec<f64>, Vec<V3>),
) -> Option<IntrinsicBundleState> {
    let mut next = state.clone();
    for (block, columns) in layout.intrinsic_columns.iter().enumerate() {
        for (index, column) in columns.iter().enumerate() {
            if let Some(column) = *column {
                let value = intrinsic(&next.intrinsics[block], index) + delta.0[column];
                set_intrinsic(&mut next.intrinsics[block], index, value);
            }
        }
    }
    for (camera, column) in layout.pose_columns.iter().enumerate() {
        if let Some(column) = *column {
            next.poses[camera] = next.poses[camera].perturbed(&delta.0[column..column + 6]);
        }
    }
    for (point, d) in linear.points.iter().zip(&delta.1) {
        next.points[point.point] = add(next.points[point.point], *d);
    }
    if let Some([anchor, other]) = setup.scale_gauge {
        let origin = state.poses[anchor].center();
        let target = norm(sub(state.poses[other].center(), origin));
        let current = norm(sub(next.poses[other].center(), origin));
        if !(current > 1e-12) {
            return None;
        }
        let factor = target / current;
        for (camera, pose) in next.poses.iter_mut().enumerate() {
            if !setup.fixed_poses[camera] {
                let center = add(origin, scale(sub(pose.center(), origin), factor));
                pose.translation = scale(mv(pose.rotation, center), -1.);
            }
        }
        for point in &linear.points {
            let p = &mut next.points[point.point];
            *p = add(origin, scale(sub(*p, origin), factor));
        }
    }
    (next.poses.iter().all(Pose::finite)
        && next.points.iter().flatten().all(|v| v.is_finite())
        && next.intrinsics.iter().all(|c| {
            (0..INTRINSIC_PARAMETERS).all(|i| intrinsic(c, i).is_finite()) && c.fx > 0. && c.fy > 0.
        }))
    .then_some(next)
}

/// Levenberg–Marquardt over the free intrinsics, poses and points.
pub fn optimize_intrinsics(
    state: &mut IntrinsicBundleState,
    setup: &IntrinsicBundleSetup,
    observations: &[Observation],
    options: &IntrinsicBundleOptions,
) -> Result<IntrinsicBundleReport> {
    let layout = validate(state, setup, observations, options)?;
    let rms = |squared: f64| (squared / observations.len() as f64).sqrt();
    let (initial_cost, initial_squared) = cost(state, setup, observations, options.huber_delta)
        .ok_or_else(|| crate::error("Non-finite initial intrinsic bundle objective"))?;
    let mut report = IntrinsicBundleReport {
        initial_rms: rms(initial_squared),
        final_rms: rms(initial_squared),
        initial_cost,
        final_cost: initial_cost,
        iterations: 0,
        accepted_steps: 0,
        free_parameters: layout.dimension
            + if setup.fixed_points {
                0
            } else {
                3 * layout.tracks.len()
            },
        termination: BundleTermination::IterationLimit,
    };
    let mut current = state.clone();
    let mut damping = options.initial_damping;
    for iteration in 0..options.max_iterations {
        let linear = linearize(&current, setup, observations, &layout, options.huber_delta)
            .ok_or_else(|| crate::error("Intrinsic bundle linearization failed"))?;
        let previous = report.final_cost;
        let mut accepted = false;
        for _ in 0..options.max_trials {
            if let Some(delta) = step(&linear, damping)
                && let Some(next) = apply(&current, setup, &layout, &linear, &delta)
                && let Some((next_cost, squared)) =
                    cost(&next, setup, observations, options.huber_delta)
                && next_cost < previous
            {
                current = next;
                report.final_cost = next_cost;
                report.final_rms = rms(squared);
                report.accepted_steps += 1;
                damping = (damping / 3.).max(1e-12);
                accepted = true;
                break;
            }
            damping = (damping * 10.).min(1e12);
        }
        report.iterations = iteration + 1;
        if !accepted {
            report.termination = BundleTermination::NoImprovement;
            break;
        }
        if previous - report.final_cost <= options.relative_cost_tolerance * previous.max(1e-12) {
            report.termination = BundleTermination::Converged;
            break;
        }
    }
    *state = current;
    Ok(report)
}
