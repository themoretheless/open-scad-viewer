//! Joint pose/point refinement with fixed intrinsics and no external numerical runtime.
//!
//! Huber-weighted LM eliminates independent 3x3 point blocks before solving the bounded
//! camera system. The anchor pose is fixed; a projection-preserving similarity fixes
//! the initial anchor/scale-camera baseline after each candidate, removing scale drift.
//! Inputs are committed only after successful completion, including the final callback.
use crate::camera::Camera;
use crate::math::{add, cross, det, dot, mm, mv, norm, rotation, scale, sub, tr, unit, M3, V3};
use std::ops::Range;

#[derive(Clone, Copy, Debug)]
pub struct Observation {
    pub camera: usize,
    pub point: usize,
    pub xy: [f64; 2],
}

/// Opt-in observation pruning applied by callers between optimization runs.
/// Huber loss only downweights outliers; this removes them from later runs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FilterOptions {
    /// Observations with a larger reprojection error (pixels) are dropped.
    pub max_reprojection_error: f64,
    /// Tracks whose widest viewing angle stays below this (radians) are dropped;
    /// zero disables the parallax test.
    pub min_parallax: f64,
}

#[derive(Clone, Debug)]
pub struct BundleOptions {
    pub max_iterations: usize,
    pub max_trials: usize,
    pub huber_delta: f64,
    pub initial_damping: f64,
    pub relative_cost_tolerance: f64,
    /// The browser defaults to 24. Native callers may explicitly raise this to 64.
    pub max_cameras: usize,
    pub max_points: usize,
    pub max_observations: usize,
    /// None keeps every observation forever (previous behavior).
    pub filter: Option<FilterOptions>,
}
impl Default for BundleOptions {
    fn default() -> Self {
        Self {
            max_iterations: 12,
            max_trials: 8,
            huber_delta: 3.,
            initial_damping: 1e-3,
            relative_cost_tolerance: 1e-7,
            max_cameras: 24,
            max_points: 50_000,
            max_observations: 200_000,
            filter: None,
        }
    }
}

impl BundleOptions {
    /// Checks the numerical/resource policy before a caller prepares image data.
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=100).contains(&self.max_iterations)
            || !(1..=16).contains(&self.max_trials)
            || !(2..=64).contains(&self.max_cameras)
            || !(6..=200_000).contains(&self.max_points)
            || !(12..=1_000_000).contains(&self.max_observations)
            || !self.huber_delta.is_finite()
            || self.huber_delta <= 0.
            || !self.initial_damping.is_finite()
            || !(1e-12..=1e12).contains(&self.initial_damping)
            || !self.relative_cost_tolerance.is_finite()
            || !(0. ..=0.01).contains(&self.relative_cost_tolerance)
        {
            return Err("Invalid bundle adjustment options".into());
        }
        if let Some(filter) = &self.filter {
            if !filter.max_reprojection_error.is_finite()
                || !(0. ..=100.).contains(&filter.max_reprojection_error)
                || filter.max_reprojection_error <= 0.
                || !filter.min_parallax.is_finite()
                || !(0. ..=1.).contains(&filter.min_parallax)
            {
                return Err("Invalid bundle adjustment filter options".into());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BundleTermination {
    Converged,
    IterationLimit,
    NoImprovement,
}

#[derive(Clone, Copy, Debug)]
pub struct BundleProgress {
    /// Number of completed outer iterations; damping retries use the same iteration.
    pub iteration: usize,
    /// Zero during preparation, then the latest accepted objective.
    pub cost: f64,
}

#[derive(Clone, Debug)]
pub struct BundleReport {
    /// Sum of Huber losses, in squared pixel units near zero (not an RMSE).
    pub initial_cost: f64,
    pub final_cost: f64,
    pub iterations: usize,
    pub accepted_steps: usize,
    pub observations: usize,
    /// Post-run pruning counters, filled by the caller when `BundleOptions::filter`
    /// is enabled; `optimize` itself never filters and leaves both at zero.
    pub filtered_observations: usize,
    pub filtered_tracks: usize,
    pub termination: BundleTermination,
}

/// Pruning totals of one `filter_observations` pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FilterReport {
    pub observations_removed: usize,
    pub tracks_removed: usize,
}

/// Per-observation keep mask for opt-in outlier pruning at the current state.
/// An observation is dropped when its reprojection error exceeds the threshold,
/// when its track is left with a single view, or when the track's widest viewing
/// angle stays below `min_parallax`. Deterministic: observations are visited in
/// storage order and per-track aggregates are indexed by point id.
pub fn outlier_mask(
    cameras: &[Option<Camera>],
    positions: &[V3],
    observations: &[Observation],
    filter: &FilterOptions,
) -> Vec<bool> {
    let limit = filter.max_reprojection_error * filter.max_reprojection_error;
    let mut keep: Vec<bool> = observations
        .iter()
        .map(|observation| {
            cameras
                .get(observation.camera)
                .and_then(Option::as_ref)
                .and_then(|camera| positions.get(observation.point).and_then(|p| camera.project(*p)))
                .is_some_and(|uv| {
                    (uv[0] - observation.xy[0]).powi(2) + (uv[1] - observation.xy[1]).powi(2)
                        <= limit
                })
        })
        .collect();
    let mut support = vec![0usize; positions.len()];
    for (observation, &kept) in observations.iter().zip(&keep) {
        if kept {
            support[observation.point] += 1;
        }
    }
    // cos decreases on [0, pi], so the widest angle stays below min_parallax
    // exactly when the smallest pairwise ray cosine exceeds cos(min_parallax).
    let mut min_cosine = vec![f64::INFINITY; positions.len()];
    if filter.min_parallax > 0. {
        let mut centers: Vec<Option<V3>> = vec![None; cameras.len()];
        for a in 0..observations.len() {
            if !keep[a] {
                continue;
            }
            let point = observations[a].point;
            if centers[observations[a].camera].is_none() {
                centers[observations[a].camera] =
                    Some(cameras[observations[a].camera].as_ref().unwrap().center());
            }
            let ray = unit(sub(
                positions[point],
                centers[observations[a].camera].unwrap(),
            ));
            for b in a + 1..observations.len() {
                if !keep[b] || observations[b].point != point {
                    continue;
                }
                if centers[observations[b].camera].is_none() {
                    centers[observations[b].camera] =
                        Some(cameras[observations[b].camera].as_ref().unwrap().center());
                }
                let other = unit(sub(
                    positions[point],
                    centers[observations[b].camera].unwrap(),
                ));
                let cosine = dot(ray, other);
                if cosine < min_cosine[point] {
                    min_cosine[point] = cosine;
                }
            }
        }
    }
    let cosine_limit = filter.min_parallax.cos();
    for (index, observation) in observations.iter().enumerate() {
        if keep[index]
            && (support[observation.point] < 2
                || (filter.min_parallax > 0. && min_cosine[observation.point] > cosine_limit))
        {
            keep[index] = false;
        }
    }
    keep
}

/// Applies `outlier_mask` in place, preserving the relative observation order.
pub fn filter_observations(
    cameras: &[Option<Camera>],
    positions: &[V3],
    observations: &mut Vec<Observation>,
    filter: &FilterOptions,
) -> FilterReport {
    let keep = outlier_mask(cameras, positions, observations, filter);
    let mut before = vec![0usize; positions.len()];
    for observation in observations.iter() {
        before[observation.point] += 1;
    }
    let mut index = 0;
    observations.retain(|_| {
        let kept = keep[index];
        index += 1;
        kept
    });
    let mut after = vec![0usize; positions.len()];
    for observation in observations.iter() {
        after[observation.point] += 1;
    }
    FilterReport {
        observations_removed: keep.len() - observations.len(),
        tracks_removed: before
            .iter()
            .zip(&after)
            .filter(|&(&b, &a)| b > 0 && a == 0)
            .count(),
    }
}

/// Cooperative cancellation is shared by every long stage, without publishing candidates.
struct Checkpoints<'a> {
    progress: &'a mut dyn FnMut(BundleProgress) -> bool,
    event: BundleProgress,
    cancelled: bool,
}
impl<'a> Checkpoints<'a> {
    fn new(progress: &'a mut dyn FnMut(BundleProgress) -> bool) -> Self {
        Self {
            progress,
            event: BundleProgress {
                iteration: 0,
                cost: 0.,
            },
            cancelled: false,
        }
    }
    fn check(&mut self) -> bool {
        if !self.cancelled {
            self.cancelled = !(self.progress)(self.event);
        }
        !self.cancelled
    }
    fn every(&mut self, index: usize, interval: usize) -> bool {
        index % interval != 0 || self.check()
    }
}

struct Layout {
    columns: Vec<Option<usize>>,
    active_cameras: Vec<usize>,
    tracks: Vec<Range<usize>>,
    order: Vec<usize>,
    dimension: usize,
    anchor_center: V3,
    baseline: f64,
}

/// Iterative union-find root with path halving; deterministic and float-free.
fn find_root(parent: &mut [usize], mut index: usize) -> usize {
    while parent[index] != index {
        parent[index] = parent[parent[index]];
        index = parent[index];
    }
    index
}

fn validate(
    cameras: &[Option<Camera>],
    positions: &[V3],
    observations: &[Observation],
    anchor: usize,
    scale_camera: usize,
    options: &BundleOptions,
    checkpoints: &mut Checkpoints,
) -> Result<Layout, String> {
    if !checkpoints.check() {
        return Err("Cancelled".into());
    }
    options.validate()?;
    if cameras.len() > 200
        || cameras.iter().flatten().count() > options.max_cameras
        || positions.len() > options.max_points
        || observations.len() > options.max_observations
        || positions.len() < 6
        || observations.len() < 12
    {
        return Err("Bundle adjustment input exceeds its bounds or has too little support".into());
    }
    let Some(anchor_pose) = cameras.get(anchor).and_then(Option::as_ref) else {
        return Err("Bundle anchor camera is not registered".into());
    };
    let Some(scale_pose) = cameras.get(scale_camera).and_then(Option::as_ref) else {
        return Err("Bundle scale camera is not registered".into());
    };
    let anchor_center = anchor_pose.center();
    let baseline = norm(sub(scale_pose.center(), anchor_center));
    if anchor == scale_camera || !baseline.is_finite() || baseline <= 1e-10 {
        return Err("Bundle adjustment needs a nonzero initial baseline".into());
    }
    for camera in cameras.iter().flatten() {
        if !camera.rotation.iter().flatten().all(|v| v.is_finite())
            || !camera.translation.iter().all(|v| v.is_finite())
            || !camera.focal.is_finite()
            || camera.focal <= 0.
            || !camera.cx.is_finite()
            || !camera.cy.is_finite()
            || (det(camera.rotation) - 1.).abs() > 1e-3
        {
            return Err("Invalid bundle camera".into());
        }
        let rrt = mm(camera.rotation, tr(camera.rotation));
        if (0..3).any(|i| (0..3).any(|j| (rrt[i][j] - f64::from(i == j)).abs() > 1e-3)) {
            return Err("Bundle camera rotation is not orthonormal".into());
        }
    }
    for (index, point) in positions.iter().enumerate() {
        if !checkpoints.every(index, 256) {
            return Err("Cancelled".into());
        }
        if !point.iter().all(|v| v.is_finite()) {
            return Err("Non-finite bundle point".into());
        }
    }
    let mut seen = vec![false; cameras.len()];
    for (index, observation) in observations.iter().enumerate() {
        if !checkpoints.every(index, 256) {
            return Err("Cancelled".into());
        }
        let Some(camera) = cameras.get(observation.camera).and_then(Option::as_ref) else {
            return Err("Bundle observation references an unregistered camera".into());
        };
        let Some(&position) = positions.get(observation.point) else {
            return Err("Bundle observation references a missing point".into());
        };
        if !observation.xy.iter().all(|v| v.is_finite()) || projection(camera, position).is_none() {
            return Err("Invalid bundle observation or non-positive initial depth".into());
        }
        seen[observation.camera] = true;
    }
    if !seen[anchor] || !seen[scale_camera] {
        return Err("Bundle gauge cameras must have observations".into());
    }
    // Bounded counting order replaces a non-interruptible O(n log n) sort.
    let mut offsets = vec![0; positions.len() + 1];
    for (index, observation) in observations.iter().enumerate() {
        if !checkpoints.every(index, 256) {
            return Err("Cancelled".into());
        }
        offsets[observation.point + 1] += 1;
    }
    for index in 1..offsets.len() {
        if !checkpoints.every(index, 256) {
            return Err("Cancelled".into());
        }
        offsets[index] += offsets[index - 1];
    }
    let mut order = vec![0; observations.len()];
    for (index, observation) in observations.iter().enumerate() {
        if !checkpoints.every(index, 256) {
            return Err("Cancelled".into());
        }
        order[offsets[observation.point]] = index;
        offsets[observation.point] += 1;
    }
    let mut last_point = vec![usize::MAX; cameras.len()];
    // Camera centers are pure functions of the validated poses; cache per camera.
    let mut centers: Vec<Option<V3>> = vec![None; cameras.len()];
    let mut tracks = Vec::new();
    let mut start = 0;
    while start < order.len() {
        if !checkpoints.every(tracks.len(), 128) {
            return Err("Cancelled".into());
        }
        let point = observations[order[start]].point;
        let mut end = start;
        while end < order.len() && observations[order[end]].point == point {
            let camera = observations[order[end]].camera;
            if last_point[camera] == point {
                return Err("Duplicate bundle observation for one point and camera".into());
            }
            last_point[camera] = point;
            end += 1;
        }
        if end - start < 2 {
            return Err("Bundle points need at least two distinct views".into());
        }
        let mut ray = |index: usize| {
            let camera = observations[order[index]].camera;
            let center = match centers[camera] {
                Some(center) => center,
                None => {
                    let center = cameras[camera].as_ref().unwrap().center();
                    centers[camera] = Some(center);
                    center
                }
            };
            unit(sub(positions[point], center))
        };
        let first_ray = ray(start);
        if !(start + 1..end).any(|index| norm(cross(first_ray, ray(index))) > 1e-6) {
            return Err("Bundle point has no triangulation parallax".into());
        }
        tracks.push(start..end);
        start = end;
    }
    if tracks.len() < 6 {
        return Err("Bundle adjustment needs at least six observed points".into());
    }
    // Reject disconnected components: every optimized pose must share a track path
    // with the anchor, otherwise a single gauge cannot constrain the whole problem.
    // One union-find pass over the camera/track hypergraph replaces repeated sweeps.
    let mut parent: Vec<usize> = (0..cameras.len()).collect();
    for (index, track) in tracks.iter().enumerate() {
        if !checkpoints.every(index, 128) {
            return Err("Cancelled".into());
        }
        let mut members = order[track.clone()].iter().map(|&i| observations[i].camera);
        let first = members.next().unwrap();
        for camera in members {
            let (a, b) = (find_root(&mut parent, first), find_root(&mut parent, camera));
            if a != b {
                parent[b] = a;
            }
        }
    }
    let anchor_root = find_root(&mut parent, anchor);
    if seen
        .iter()
        .enumerate()
        .any(|(camera, &seen)| seen && find_root(&mut parent, camera) != anchor_root)
    {
        return Err("Bundle observations contain disconnected camera groups".into());
    }
    let active_cameras: Vec<_> = (0..cameras.len()).filter(|&i| seen[i]).collect();
    let mut columns = vec![None; cameras.len()];
    let mut dimension = 0;
    for &camera in &active_cameras {
        if camera != anchor {
            columns[camera] = Some(dimension);
            dimension += 6;
        }
    }
    Ok(Layout {
        columns,
        active_cameras,
        tracks,
        order,
        dimension,
        anchor_center,
        baseline,
    })
}

fn projection(camera: &Camera, point: V3) -> Option<([f64; 2], V3)> {
    let transformed = camera.camera_point(point);
    camera
        .project_camera_point(transformed)
        .map(|pixel| (pixel, transformed))
}

/// Left SE(3) update: R' = exp(w) R, t' = exp(w) t + v.
fn perturb(camera: &Camera, delta: &[f64]) -> Camera {
    let dr = rotation([delta[0], delta[1], delta[2]]);
    let mut result = camera.clone();
    result.rotation = mm(dr, camera.rotation);
    result.translation = add(mv(dr, camera.translation), [delta[3], delta[4], delta[5]]);
    result
}

fn jacobians(camera: &Camera, transformed: V3) -> ([[f64; 6]; 2], [[f64; 3]; 2]) {
    let [x, y, z] = transformed;
    let fz = camera.focal / z;
    let projection = [[fz, 0., -fz * x / z], [0., fz, -fz * y / z]];
    let rotation = [[0., z, -y], [-z, 0., x], [y, -x, 0.]];
    let jc = std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            if column < 3 {
                (0..3)
                    .map(|i| projection[row][i] * rotation[i][column])
                    .sum()
            } else {
                projection[row][column - 3]
            }
        })
    });
    let jp = std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            (0..3)
                .map(|i| projection[row][i] * camera.rotation[i][column])
                .sum()
        })
    });
    (jc, jp)
}

fn huber(residual: [f64; 2], delta: f64) -> (f64, f64) {
    let length = residual[0].hypot(residual[1]);
    if length <= delta {
        (0.5 * length * length, 1.)
    } else {
        (delta * (length - 0.5 * delta), delta / length)
    }
}

fn cost(
    cameras: &[Option<Camera>],
    positions: &[V3],
    observations: &[Observation],
    delta: f64,
    checkpoints: &mut Checkpoints,
) -> Option<f64> {
    let mut result = 0.;
    for (index, observation) in observations.iter().enumerate() {
        if !checkpoints.every(index, 256) {
            return None;
        }
        let camera = cameras[observation.camera].as_ref()?;
        let (xy, _) = projection(camera, positions[observation.point])?;
        result += huber(
            [xy[0] - observation.xy[0], xy[1] - observation.xy[1]],
            delta,
        )
        .0;
    }
    result.is_finite().then_some(result)
}

struct CrossBlock {
    column: usize,
    values: [[f64; 3]; 6],
}
struct PointBlock {
    point: usize,
    hessian: M3,
    rhs: V3,
    cross: Range<usize>,
}
struct Linearization {
    camera_hessian: Vec<f64>,
    camera_rhs: Vec<f64>,
    points: Vec<PointBlock>,
    cross: Vec<CrossBlock>,
}

fn linearize(
    cameras: &[Option<Camera>],
    positions: &[V3],
    observations: &[Observation],
    layout: &Layout,
    delta: f64,
    checkpoints: &mut Checkpoints,
) -> Option<Linearization> {
    let n = layout.dimension;
    let mut result = Linearization {
        camera_hessian: vec![0.; n * n],
        camera_rhs: vec![0.; n],
        points: Vec::with_capacity(layout.tracks.len()),
        cross: Vec::with_capacity(observations.len()),
    };
    for (index, track) in layout.tracks.iter().enumerate() {
        if !checkpoints.every(index, 32) {
            return None;
        }
        let point = observations[layout.order[track.start]].point;
        let mut block = PointBlock {
            point,
            hessian: [[0.; 3]; 3],
            rhs: [0.; 3],
            cross: result.cross.len()..result.cross.len(),
        };
        for &index in &layout.order[track.clone()] {
            let observation = &observations[index];
            let camera = cameras[observation.camera].as_ref().unwrap();
            // The current state was validated by the objective before acceptance.
            let (xy, transformed) = projection(camera, positions[point]).unwrap();
            let residual = [xy[0] - observation.xy[0], xy[1] - observation.xy[1]];
            let weight = huber(residual, delta).1;
            let (jc, jp) = jacobians(camera, transformed);
            for i in 0..3 {
                block.rhs[i] -= weight * (jp[0][i] * residual[0] + jp[1][i] * residual[1]);
                // Symmetric: only the upper triangle is accumulated, then mirrored.
                for j in i..3 {
                    block.hessian[i][j] += weight * (jp[0][i] * jp[0][j] + jp[1][i] * jp[1][j]);
                }
            }
            if let Some(column) = layout.columns[observation.camera] {
                let mut cross = CrossBlock {
                    column,
                    values: [[0.; 3]; 6],
                };
                for i in 0..6 {
                    result.camera_rhs[column + i] -=
                        weight * (jc[0][i] * residual[0] + jc[1][i] * residual[1]);
                    // Symmetric: only the upper triangle is accumulated, then mirrored.
                    for j in i..6 {
                        result.camera_hessian[(column + i) * n + column + j] +=
                            weight * (jc[0][i] * jc[0][j] + jc[1][i] * jc[1][j]);
                    }
                    for j in 0..3 {
                        cross.values[i][j] = weight * (jc[0][i] * jp[0][j] + jc[1][i] * jp[1][j]);
                    }
                }
                result.cross.push(cross);
            }
        }
        // The symmetric lower triangle mirrors the accumulated upper one.
        for i in 0..3 {
            for j in i + 1..3 {
                block.hessian[j][i] = block.hessian[i][j];
            }
        }
        block.cross.end = result.cross.len();
        result.points.push(block);
    }
    // The symmetric lower triangle of each camera block mirrors the upper one.
    for &camera in &layout.active_cameras {
        if let Some(column) = layout.columns[camera] {
            for i in 0..6 {
                for j in i + 1..6 {
                    result.camera_hessian[(column + j) * n + column + i] =
                        result.camera_hessian[(column + i) * n + column + j];
                }
            }
        }
    }
    Some(result)
}

/// Scale before inverting the tiny SPD block to avoid world-unit overflow/underflow.
fn inverse_point(mut h: M3, damping: f64) -> Option<M3> {
    for i in 0..3 {
        h[i][i] += damping * h[i][i].max(1e-9);
    }
    let size = (0..3).map(|i| h[i][i]).fold(0., f64::max);
    if !size.is_finite() || size <= 0. {
        return None;
    }
    h = h.map(|r| r.map(|v| v / size));
    let d = det(h);
    if !d.is_finite() || d <= 1e-30 {
        return None;
    }
    let [a, b, c] = h[0];
    let e = h[1][1];
    let f = h[1][2];
    let i = h[2][2];
    let inverse = [
        [e * i - f * f, c * f - b * i, b * f - c * e],
        [c * f - b * i, a * i - c * c, b * c - a * f],
        [b * f - c * e, b * c - a * f, a * e - b * b],
    ]
    .map(|r| r.map(|v| v / d / size));
    inverse
        .iter()
        .flatten()
        .all(|v| v.is_finite())
        .then_some(inverse)
}

/// Diagonally normalized Cholesky. Storage is only quadratic in camera count.
/// Reuses caller-owned buffers so damping retries do not reallocate the system.
fn solve_reduced(
    h: &mut [f64],
    rhs: &mut [f64],
    checkpoints: &mut Checkpoints,
) -> Option<()> {
    let n = rhs.len();
    let mut scales = vec![0.; n];
    for i in 0..n {
        if !h[i * n + i].is_finite() || h[i * n + i] <= 0. {
            return None;
        }
        scales[i] = h[i * n + i].sqrt();
        rhs[i] /= scales[i];
    }
    for i in 0..n {
        for j in 0..=i {
            h[i * n + j] /= scales[i] * scales[j];
        }
    }
    for i in 0..n {
        if !checkpoints.every(i, 16) {
            return None;
        }
        for j in 0..=i {
            let mut value = h[i * n + j];
            for k in 0..j {
                value -= h[i * n + k] * h[j * n + k];
            }
            if i == j {
                if !value.is_finite() || value <= 1e-14 {
                    return None;
                }
                h[i * n + j] = value.sqrt();
            } else {
                h[i * n + j] = value / h[j * n + j];
            }
        }
        for j in 0..i {
            rhs[i] -= h[i * n + j] * rhs[j];
        }
        rhs[i] /= h[i * n + i];
    }
    for i in (0..n).rev() {
        for j in i + 1..n {
            rhs[i] -= h[j * n + i] * rhs[j];
        }
        rhs[i] /= h[i * n + i];
    }
    for i in 0..n {
        rhs[i] /= scales[i];
    }
    rhs.iter().all(|v| v.is_finite()).then_some(())
}

struct Step {
    cameras: Vec<f64>,
    points: Vec<V3>,
}
/// Reusable camera-system storage shared by every damping trial of one call.
#[derive(Default)]
struct SchurScratch {
    h: Vec<f64>,
    rhs: Vec<f64>,
}
fn schur_step(
    linear: &Linearization,
    damping: f64,
    scratch: &mut SchurScratch,
    checkpoints: &mut Checkpoints,
) -> Option<Step> {
    let n = linear.camera_rhs.len();
    scratch.h.clone_from(&linear.camera_hessian);
    scratch.rhs.clone_from(&linear.camera_rhs);
    let h = &mut scratch.h;
    let rhs = &mut scratch.rhs;
    for i in 0..n {
        h[i * n + i] += damping * h[i * n + i].max(1e-9);
    }
    let mut inverses = Vec::with_capacity(linear.points.len());
    let mut products: Vec<[[f64; 3]; 6]> = Vec::new();
    for (index, point) in linear.points.iter().enumerate() {
        if !checkpoints.every(index, 16) {
            return None;
        }
        let inverse = inverse_point(point.hessian, damping)?;
        let point_rhs = mv(inverse, point.rhs);
        // Store only the current track's W V^-1; no global camera-by-point matrix.
        let cross = &linear.cross[point.cross.clone()];
        products.clear();
        products.extend(
            cross
                .iter()
                .map(|w| std::array::from_fn(|i| mv(inverse, w.values[i]))),
        );
        for (a, wa) in cross.iter().enumerate() {
            for i in 0..6 {
                rhs[wa.column + i] -= (0..3).map(|k| wa.values[i][k] * point_rhs[k]).sum::<f64>();
            }
            for (b, wb) in cross.iter().enumerate().take(a + 1) {
                // Compute the 6x6 contribution contiguously, then subtract it into
                // the scattered direct and transposed addresses in the same order.
                let mut block = [[0.; 6]; 6];
                for i in 0..6 {
                    for j in 0..6 {
                        block[i][j] = (0..3)
                            .map(|k| products[a][i][k] * wb.values[j][k])
                            .sum::<f64>();
                    }
                }
                for i in 0..6 {
                    for j in 0..6 {
                        h[(wa.column + i) * n + wb.column + j] -= block[i][j];
                        if a != b {
                            h[(wb.column + j) * n + wa.column + i] -= block[i][j];
                        }
                    }
                }
            }
        }
        inverses.push(inverse);
    }
    solve_reduced(h, rhs, checkpoints)?;
    let cameras = rhs.clone();
    let mut points = Vec::with_capacity(linear.points.len());
    for (index, (point, inverse)) in linear.points.iter().zip(inverses).enumerate() {
        if !checkpoints.every(index, 128) {
            return None;
        }
        let mut rhs = point.rhs;
        for w in &linear.cross[point.cross.clone()] {
            for (i, value) in rhs.iter_mut().enumerate() {
                *value -= (0..6)
                    .map(|j| w.values[j][i] * cameras[w.column + j])
                    .sum::<f64>();
            }
        }
        let delta = mv(inverse, rhs);
        if !delta.iter().all(|v| v.is_finite()) {
            return None;
        }
        points.push(delta);
    }
    Some(Step { cameras, points })
}

#[allow(clippy::too_many_arguments)]
fn candidate(
    cameras: &[Option<Camera>],
    positions: &[V3],
    linear: &Linearization,
    layout: &Layout,
    scale_camera: usize,
    step: &Step,
    next_cameras: &mut Vec<Option<Camera>>,
    next_points: &mut Vec<V3>,
    checkpoints: &mut Checkpoints,
) -> Option<()> {
    next_cameras.clear();
    next_cameras.extend_from_slice(cameras);
    next_points.clear();
    next_points.extend_from_slice(positions);
    for &index in &layout.active_cameras {
        if let Some(column) = layout.columns[index] {
            next_cameras[index] = Some(perturb(
                cameras[index].as_ref()?,
                &step.cameras[column..column + 6],
            ));
        }
    }
    for (index, (point, delta)) in linear.points.iter().zip(&step.points).enumerate() {
        if !checkpoints.every(index, 256) {
            return None;
        }
        next_points[point.point] = add(next_points[point.point], *delta);
    }
    let baseline = norm(sub(
        next_cameras[scale_camera].as_ref()?.center(),
        layout.anchor_center,
    ));
    if !baseline.is_finite() || baseline <= 1e-10 {
        return None;
    }
    let factor = layout.baseline / baseline;
    for &index in &layout.active_cameras {
        // Never reassign the anchor, even to an algebraically equivalent pose.
        if layout.columns[index].is_some() {
            let camera = next_cameras[index].as_mut()?;
            let center = add(
                layout.anchor_center,
                scale(sub(camera.center(), layout.anchor_center), factor),
            );
            camera.translation = scale(mv(camera.rotation, center), -1.);
        }
    }
    for (index, point) in linear.points.iter().enumerate() {
        if !checkpoints.every(index, 256) {
            return None;
        }
        next_points[point.point] = add(
            layout.anchor_center,
            scale(sub(next_points[point.point], layout.anchor_center), factor),
        );
    }
    Some(())
}

/// Jointly optimize a connected reconstruction. Unobserved points/cameras are preserved.
/// Each observed point must have at least two distinct camera observations. Invalid
/// geometry, disconnected groups and resource-limit violations return an explicit error.
/// Returning false from `progress` cancels the entire call without modifying either input.
/// A successful report may contain zero accepted steps; this is never a quality guarantee.
pub fn optimize(
    cameras: &mut [Option<Camera>],
    positions: &mut [V3],
    observations: &[Observation],
    anchor_camera: usize,
    scale_camera: usize,
    options: &BundleOptions,
    mut progress: impl FnMut(BundleProgress) -> bool,
) -> Result<BundleReport, String> {
    let mut checkpoints = Checkpoints::new(&mut progress);
    let layout = validate(
        cameras,
        positions,
        observations,
        anchor_camera,
        scale_camera,
        options,
        &mut checkpoints,
    )?;
    let initial_cost = cost(
        cameras,
        positions,
        observations,
        options.huber_delta,
        &mut checkpoints,
    );
    if checkpoints.cancelled {
        return Err("Cancelled".into());
    }
    let initial_cost = initial_cost.ok_or("Non-finite initial bundle objective")?;
    let mut report = BundleReport {
        initial_cost,
        final_cost: initial_cost,
        iterations: 0,
        accepted_steps: 0,
        observations: observations.len(),
        filtered_observations: 0,
        filtered_tracks: 0,
        termination: BundleTermination::IterationLimit,
    };
    let mut current_cameras = cameras.to_vec();
    let mut current_points = positions.to_vec();
    let mut trial_cameras: Vec<Option<Camera>> = Vec::new();
    let mut trial_points: Vec<V3> = Vec::new();
    let mut schur_scratch = SchurScratch::default();
    let mut damping = options.initial_damping;
    for iteration in 0..options.max_iterations {
        checkpoints.event = BundleProgress {
            iteration,
            cost: report.final_cost,
        };
        if !checkpoints.check() {
            return Err("Cancelled".into());
        }
        let linear = linearize(
            &current_cameras,
            &current_points,
            observations,
            &layout,
            options.huber_delta,
            &mut checkpoints,
        )
        .ok_or("Cancelled")?;
        let previous = report.final_cost;
        let mut accepted = false;
        for _ in 0..options.max_trials {
            if !checkpoints.check() {
                return Err("Cancelled".into());
            }
            let proposed = schur_step(&linear, damping, &mut schur_scratch, &mut checkpoints)
                .and_then(|step| {
                    candidate(
                        &current_cameras,
                        &current_points,
                        &linear,
                        &layout,
                        scale_camera,
                        &step,
                        &mut trial_cameras,
                        &mut trial_points,
                        &mut checkpoints,
                    )
                })
                .is_some();
            if proposed {
                if let Some(next_cost) = cost(
                    &trial_cameras,
                    &trial_points,
                    observations,
                    options.huber_delta,
                    &mut checkpoints,
                ) {
                    if next_cost < previous {
                        std::mem::swap(&mut current_cameras, &mut trial_cameras);
                        std::mem::swap(&mut current_points, &mut trial_points);
                        report.final_cost = next_cost;
                        report.accepted_steps += 1;
                        damping = (damping / 3.).max(1e-12);
                        accepted = true;
                        break;
                    }
                }
            }
            if checkpoints.cancelled {
                return Err("Cancelled".into());
            }
            damping = (damping * 10.).min(1e12);
        }
        report.iterations = iteration + 1;
        if !accepted {
            report.termination = BundleTermination::NoImprovement;
            break;
        }
        if previous - report.final_cost <= options.relative_cost_tolerance * previous.max(1.) {
            report.termination = BundleTermination::Converged;
            break;
        }
    }
    checkpoints.event = BundleProgress {
        iteration: report.iterations,
        cost: report.final_cost,
    };
    if !checkpoints.check() {
        return Err("Cancelled".into());
    }
    cameras.clone_from_slice(&current_cameras);
    positions.copy_from_slice(&current_points);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::ID;

    fn scene(noise: bool, outliers: bool) -> (Vec<Option<Camera>>, Vec<V3>, Vec<Observation>) {
        let mut cameras = Vec::new();
        for i in 0..4 {
            let mut camera = Camera::identity(700. + 30. * i as f64, 480., 320.);
            camera.rotation = rotation([0.01 * i as f64, -0.045 * i as f64, 0.005 * i as f64]);
            let center = [0.65 * i as f64, 0.12 * (i as f64).sin(), 0.07 * i as f64];
            camera.translation = scale(mv(camera.rotation, center), -1.);
            cameras.push(Some(camera));
        }
        let points: Vec<_> = (0..120)
            .map(|i| {
                [
                    (i % 12) as f64 * 0.18 - 0.6,
                    (i / 12) as f64 * 0.16 - 0.7,
                    4. + ((i * 37) % 19) as f64 * 0.095,
                ]
            })
            .collect();
        let mut observations = Vec::new();
        for (point, &position) in points.iter().enumerate() {
            for (camera, pose) in cameras.iter().enumerate() {
                let mut xy = pose.as_ref().unwrap().project(position).unwrap();
                if noise {
                    xy[0] += 0.12 * ((point * 7 + camera * 3) as f64).sin();
                    xy[1] += 0.12 * ((point * 5 + camera * 11) as f64).cos();
                }
                if outliers && (point * 4 + camera) % 43 == 0 {
                    xy[0] += 35.;
                    xy[1] -= 24.;
                }
                observations.push(Observation { camera, point, xy });
            }
        }
        (cameras, points, observations)
    }

    fn distort(cameras: &mut [Option<Camera>], points: &mut [V3]) {
        for (i, camera) in cameras.iter_mut().enumerate().skip(1) {
            let original = camera.as_ref().unwrap();
            let mut updated = perturb(original, &[0.006, -0.009, 0.004, 0.025, -0.02, 0.04]);
            // Preserve seed baseline in the initialization so known-scale shape error
            // measures recovery, not the unavoidable gauge ambiguity of photographs.
            if i == 1 {
                let center = scale(
                    updated.center(),
                    norm(original.center()) / norm(updated.center()),
                );
                updated.translation = scale(mv(updated.rotation, center), -1.);
            }
            *camera = Some(updated);
        }
        for (i, point) in points.iter_mut().enumerate() {
            point[0] += 0.04 * (i as f64 * 1.7).sin();
            point[1] += 0.03 * (i as f64 * 2.3).cos();
            point[2] += 0.07 * (i as f64 * 0.8).sin();
        }
    }

    #[test]
    fn schur_elimination_matches_full_normal_equations() {
        let (mut cameras, mut points, mut observations) = scene(true, false);
        points.truncate(8);
        observations.retain(|observation| observation.point < points.len());
        distort(&mut cameras, &mut points);
        let options = BundleOptions::default();
        let mut keep_running = |_| true;
        let mut checkpoints = Checkpoints::new(&mut keep_running);
        let layout = validate(
            &cameras,
            &points,
            &observations,
            0,
            1,
            &options,
            &mut checkpoints,
        )
        .unwrap();
        let linear = linearize(
            &cameras,
            &points,
            &observations,
            &layout,
            options.huber_delta,
            &mut checkpoints,
        )
        .unwrap();
        let damping = 0.003;
        let step = schur_step(&linear, damping, &mut SchurScratch::default(), &mut checkpoints)
            .unwrap();
        let nc = layout.dimension;
        let n = nc + linear.points.len() * 3;
        let mut h = vec![0.; n * n];
        let mut rhs = vec![0.; n];
        for i in 0..nc {
            rhs[i] = linear.camera_rhs[i];
            for j in 0..nc {
                h[i * n + j] = linear.camera_hessian[i * nc + j];
            }
        }
        for (index, point) in linear.points.iter().enumerate() {
            let column = nc + index * 3;
            for i in 0..3 {
                rhs[column + i] = point.rhs[i];
                for j in 0..3 {
                    h[(column + i) * n + column + j] = point.hessian[i][j];
                }
            }
            for cross in &linear.cross[point.cross.clone()] {
                for i in 0..6 {
                    for j in 0..3 {
                        h[(cross.column + i) * n + column + j] = cross.values[i][j];
                        h[(column + j) * n + cross.column + i] = cross.values[i][j];
                    }
                }
            }
        }
        for i in 0..n {
            h[i * n + i] += damping * h[i * n + i].max(1e-9);
        }
        solve_reduced(&mut h, &mut rhs, &mut checkpoints).unwrap();
        let full = rhs;
        for (actual, expected) in step
            .cameras
            .iter()
            .chain(step.points.iter().flatten())
            .zip(full)
        {
            assert!(
                (actual - expected).abs() < 1e-8,
                "Schur {actual}, full {expected}"
            );
        }
    }

    #[test]
    fn disconnected_and_single_view_tracks_are_rejected() {
        let (mut cameras, mut points, observations) = scene(false, false);
        let disconnected: Vec<_> = observations
            .iter()
            .copied()
            .filter(|o| (o.point < 60) == (o.camera < 2))
            .collect();
        assert!(optimize(
            &mut cameras,
            &mut points,
            &disconnected,
            0,
            1,
            &BundleOptions::default(),
            |_| true
        )
        .unwrap_err()
        .contains("disconnected"));
        let single_view: Vec<_> = observations
            .iter()
            .copied()
            .filter(|o| o.point != 0 || o.camera == 0)
            .collect();
        assert!(optimize(
            &mut cameras,
            &mut points,
            &single_view,
            0,
            1,
            &BundleOptions::default(),
            |_| true
        )
        .unwrap_err()
        .contains("two distinct"));
    }

    #[test]
    fn pure_rotation_tracks_are_rejected_even_with_distinct_camera_ids() {
        let (mut cameras, mut points, mut observations) = scene(false, false);
        cameras[2] = cameras[0].clone();
        let camera = cameras[2].as_mut().unwrap();
        camera.rotation = rotation([0., 0.03, 0.]);
        // Other points still connect a valid-baseline camera; only point0 is depth-degenerate.
        observations.retain(|o| o.point != 0 || o.camera == 0 || o.camera == 2);
        for o in &mut observations {
            if o.camera == 2 {
                o.xy = camera.project(points[o.point]).unwrap();
            }
        }
        assert!(optimize(
            &mut cameras,
            &mut points,
            &observations,
            0,
            1,
            &BundleOptions::default(),
            |_| true
        )
        .unwrap_err()
        .contains("parallax"));
    }

    #[test]
    fn rejected_steps_leave_an_exact_reconstruction_unchanged() {
        let (mut cameras, mut points, observations) = scene(false, false);
        let original_points = points.clone();
        let original_cameras = format!("{cameras:?}");
        let report = optimize(
            &mut cameras,
            &mut points,
            &observations,
            0,
            1,
            &BundleOptions::default(),
            |_| true,
        )
        .unwrap();
        assert_eq!(report.initial_cost, 0.);
        assert_eq!(report.accepted_steps, 0);
        assert_eq!(points, original_points);
        assert_eq!(format!("{cameras:?}"), original_cameras);
    }

    #[test]
    fn analytic_jacobians_match_central_differences() {
        let camera = Camera {
            rotation: rotation([0.16, -0.08, 0.04]),
            translation: [-0.7, 0.2, 0.4],
            focal: 730.,
            cx: 480.,
            cy: 320.,
        };
        let point = [0.6, -0.4, 4.2];
        let (jc, jp) = jacobians(&camera, projection(&camera, point).unwrap().1);
        let epsilon = 1e-6;
        for column in 0..6 {
            let mut delta = [0.; 6];
            delta[column] = epsilon;
            let plus = perturb(&camera, &delta).project(point).unwrap();
            delta[column] = -epsilon;
            let minus = perturb(&camera, &delta).project(point).unwrap();
            for row in 0..2 {
                let numerical = (plus[row] - minus[row]) / (2. * epsilon);
                assert!(
                    (numerical - jc[row][column]).abs() < 1e-5,
                    "camera {row},{column}: {numerical} != {}",
                    jc[row][column]
                );
            }
        }
        for column in 0..3 {
            let mut plus = point;
            let mut minus = point;
            plus[column] += epsilon;
            minus[column] -= epsilon;
            let plus = camera.project(plus).unwrap();
            let minus = camera.project(minus).unwrap();
            for row in 0..2 {
                assert!(((plus[row] - minus[row]) / (2. * epsilon) - jp[row][column]).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn joint_refinement_recovers_nonplanar_shape_and_holds_gauge() {
        let (mut cameras, truth, observations) = scene(true, false);
        let mut points = truth.clone();
        distort(&mut cameras, &mut points);
        let anchor = cameras[0].as_ref().unwrap().clone();
        let baseline = norm(cameras[1].as_ref().unwrap().center());
        let initial_error: f64 = points
            .iter()
            .zip(&truth)
            .map(|(&a, &b)| norm(sub(a, b)))
            .sum::<f64>()
            / truth.len() as f64;
        let report = optimize(
            &mut cameras,
            &mut points,
            &observations,
            0,
            1,
            &BundleOptions::default(),
            |_| true,
        )
        .unwrap();
        let final_error = points
            .iter()
            .zip(&truth)
            .map(|(&a, &b)| norm(sub(a, b)))
            .sum::<f64>()
            / truth.len() as f64;
        assert!(report.final_cost < report.initial_cost * 0.01, "{report:?}");
        assert!(
            final_error < initial_error * 0.2 && final_error < 0.01,
            "{initial_error} -> {final_error}"
        );
        assert_eq!(cameras[0].as_ref().unwrap().rotation, anchor.rotation);
        assert_eq!(cameras[0].as_ref().unwrap().translation, anchor.translation);
        assert!((norm(cameras[1].as_ref().unwrap().center()) - baseline).abs() < 1e-12);
        eprintln!(
            "bundle synthetic: cost {} -> {}, mean shape error {} -> {}, accepted {}",
            report.initial_cost,
            report.final_cost,
            initial_error,
            final_error,
            report.accepted_steps
        );
    }

    #[test]
    fn robust_loss_reduces_outlier_damage() {
        let (mut cameras, truth, observations) = scene(true, true);
        let mut points = truth.clone();
        distort(&mut cameras, &mut points);
        let mut least_squares_cameras = cameras.clone();
        let mut least_squares_points = points.clone();
        let report = optimize(
            &mut cameras,
            &mut points,
            &observations,
            0,
            1,
            &BundleOptions {
                max_iterations: 20,
                ..Default::default()
            },
            |_| true,
        )
        .unwrap();
        optimize(
            &mut least_squares_cameras,
            &mut least_squares_points,
            &observations,
            0,
            1,
            &BundleOptions {
                max_iterations: 20,
                huber_delta: 1e6,
                ..Default::default()
            },
            |_| true,
        )
        .unwrap();
        let clean_error = |positions: &[V3]| {
            let mut errors: Vec<_> = positions
                .iter()
                .zip(&truth)
                .enumerate()
                .filter(|(i, _)| (0..4).all(|c| (i * 4 + c) % 43 != 0))
                .map(|(_, (&a, &b))| norm(sub(a, b)))
                .collect();
            errors.sort_by(f64::total_cmp);
            errors[errors.len() / 2]
        };
        let robust_error = clean_error(&points);
        let ordinary_error = clean_error(&least_squares_points);
        assert!(report.final_cost < report.initial_cost * 0.4, "{report:?}");
        assert!(
            robust_error < ordinary_error * 0.5,
            "Huber median shape error {robust_error}, ordinary least squares {ordinary_error}"
        );
        eprintln!("bundle outliers: Huber median shape error {robust_error}, ordinary least squares {ordinary_error}");
    }

    #[test]
    fn cancellation_precedes_preparation_and_preserves_inputs() {
        let (mut cameras, mut points, mut observations) = scene(false, false);
        observations[0].point = usize::MAX;
        let before_cameras = format!("{cameras:?}");
        let before_points = points.clone();
        let mut calls = 0;
        let error = optimize(
            &mut cameras,
            &mut points,
            &observations,
            0,
            1,
            &BundleOptions::default(),
            |event| {
                calls += 1;
                assert_eq!(event.iteration, 0);
                assert_eq!(event.cost, 0.);
                false
            },
        )
        .unwrap_err();
        assert_eq!(error, "Cancelled");
        assert_eq!(calls, 1);
        assert_eq!(points, before_points);
        assert_eq!(format!("{cameras:?}"), before_cameras);
    }

    #[test]
    fn cancellation_inside_the_first_linearization_is_transactional() {
        let (mut cameras, mut points, observations) = scene(false, false);
        distort(&mut cameras, &mut points);
        let before_cameras = format!("{cameras:?}");
        let before_points = points.clone();
        let mut numerical_checkpoints = 0;
        let error = optimize(
            &mut cameras,
            &mut points,
            &observations,
            0,
            1,
            &BundleOptions::default(),
            |event| {
                assert_eq!(event.iteration, 0);
                if event.cost > 0. {
                    numerical_checkpoints += 1;
                }
                numerical_checkpoints < 3
            },
        )
        .unwrap_err();
        assert_eq!(error, "Cancelled");
        assert_eq!(numerical_checkpoints, 3);
        assert_eq!(points, before_points);
        assert_eq!(format!("{cameras:?}"), before_cameras);
    }

    #[test]
    fn schur_elimination_checks_cancellation_between_point_blocks() {
        let (cameras, points, observations) = scene(true, false);
        let mut keep_running = |_| true;
        let mut checkpoints = Checkpoints::new(&mut keep_running);
        let options = BundleOptions::default();
        let layout = validate(
            &cameras,
            &points,
            &observations,
            0,
            1,
            &options,
            &mut checkpoints,
        )
        .unwrap();
        let linear = linearize(
            &cameras,
            &points,
            &observations,
            &layout,
            options.huber_delta,
            &mut checkpoints,
        )
        .unwrap();
        let mut calls = 0;
        let mut cancel_during_elimination = |_| {
            calls += 1;
            calls < 2
        };
        let mut checkpoints = Checkpoints::new(&mut cancel_during_elimination);
        assert!(
            schur_step(
                &linear,
                options.initial_damping,
                &mut SchurScratch::default(),
                &mut checkpoints
            )
            .is_none()
        );
        assert!(checkpoints.cancelled);
        assert_eq!(calls, 2);
    }

    #[test]
    fn cancellation_is_transactional_after_an_accepted_step() {
        let (mut cameras, mut points, observations) = scene(false, false);
        distort(&mut cameras, &mut points);
        let initial_cameras = format!("{cameras:?}");
        let initial_points = points.clone();
        let mut last_cost = f64::INFINITY;
        let mut saw_improvement = false;
        let error = optimize(
            &mut cameras,
            &mut points,
            &observations,
            0,
            1,
            &BundleOptions::default(),
            |p| {
                saw_improvement |= p.cost < last_cost && last_cost.is_finite();
                last_cost = p.cost;
                p.iteration < 1
            },
        )
        .unwrap_err();
        assert_eq!(error, "Cancelled");
        assert!(saw_improvement);
        assert_eq!(points, initial_points);
        assert_eq!(format!("{cameras:?}"), initial_cameras);
    }

    #[test]
    fn invalid_and_degenerate_inputs_do_not_mutate() {
        let (mut cameras, mut points, mut observations) = scene(false, false);
        let initial_points = points.clone();
        cameras[1] = cameras[0].clone();
        assert!(optimize(
            &mut cameras,
            &mut points,
            &observations,
            0,
            1,
            &BundleOptions::default(),
            |_| true
        )
        .unwrap_err()
        .contains("baseline"));
        let (mut cameras, _, _) = scene(false, false);
        observations.push(observations[0]);
        assert!(optimize(
            &mut cameras,
            &mut points,
            &observations,
            0,
            1,
            &BundleOptions::default(),
            |_| true
        )
        .unwrap_err()
        .contains("Duplicate"));
        observations.pop();
        points[0][2] = -1.;
        assert!(optimize(
            &mut cameras,
            &mut points,
            &observations,
            0,
            1,
            &BundleOptions::default(),
            |_| true
        )
        .unwrap_err()
        .contains("depth"));
        points = initial_points.clone();
        let initial_cameras = format!("{cameras:?}");
        let options = BundleOptions {
            max_cameras: 3,
            ..Default::default()
        };
        assert!(optimize(
            &mut cameras,
            &mut points,
            &observations,
            0,
            1,
            &options,
            |_| true
        )
        .is_err());
        assert_eq!(points, initial_points);
        assert_eq!(format!("{cameras:?}"), initial_cameras);
    }

    #[test]
    fn unobserved_data_and_nonidentity_anchor_are_preserved() {
        let (mut cameras, mut points, observations) = scene(false, false);
        let shift = [3., -2., 0.7];
        let world_rotation = rotation([0.07, 0.08, -0.13]);
        for camera in cameras.iter_mut().flatten() {
            camera.rotation = mm(camera.rotation, tr(world_rotation));
            camera.translation = sub(camera.translation, mv(camera.rotation, shift));
        }
        for point in &mut points {
            *point = add(mv(world_rotation, *point), shift);
        }
        let anchor = cameras[0].as_ref().unwrap().clone();
        let baseline = norm(sub(cameras[1].as_ref().unwrap().center(), anchor.center()));
        for point in &mut points {
            point[2] += 0.01;
        }
        points.push([100., 200., -300.]);
        cameras.push(Some(Camera {
            rotation: ID,
            translation: [8., 4., -9.],
            focal: 900.,
            cx: 0.,
            cy: 0.,
        }));
        let unused_camera = format!("{:?}", cameras.last());
        let report = optimize(
            &mut cameras,
            &mut points,
            &observations,
            0,
            1,
            &BundleOptions::default(),
            |_| true,
        )
        .unwrap();
        assert!(report.final_cost < report.initial_cost);
        assert_eq!(points.last().unwrap(), &[100., 200., -300.]);
        assert_eq!(format!("{:?}", cameras.last()), unused_camera);
        assert_eq!(cameras[0].as_ref().unwrap().rotation, anchor.rotation);
        assert_eq!(cameras[0].as_ref().unwrap().translation, anchor.translation);
        assert!(
            (norm(sub(cameras[1].as_ref().unwrap().center(), anchor.center())) - baseline).abs()
                < 1e-12
        );
    }

    #[test]
    fn filter_options_are_validated_only_when_enabled() {
        assert!(BundleOptions::default().validate().is_ok());
        let options = |max_reprojection_error: f64, min_parallax: f64| BundleOptions {
            filter: Some(FilterOptions {
                max_reprojection_error,
                min_parallax,
            }),
            ..Default::default()
        };
        assert!(options(4., 0.).validate().is_ok());
        assert!(options(2., 0.01).validate().is_ok());
        for bad in [0., -1., f64::NAN, f64::INFINITY, 100.5] {
            assert!(options(bad, 0.).validate().is_err(), "{bad}");
        }
        for bad in [-0.1, 1.1, f64::NAN] {
            assert!(options(4., bad).validate().is_err(), "{bad}");
        }
    }

    #[test]
    fn outlier_mask_drops_only_large_errors_deterministically() {
        let (cameras, points, mut observations) = scene(true, true);
        let filter = FilterOptions {
            max_reprojection_error: 4.,
            min_parallax: 0.,
        };
        let expected: Vec<bool> = observations
            .iter()
            .map(|o| (o.point * 4 + o.camera) % 43 != 0)
            .collect();
        let first = outlier_mask(&cameras, &points, &observations, &filter);
        assert_eq!(first, outlier_mask(&cameras, &points, &observations, &filter));
        assert_eq!(first, expected);
        let report = filter_observations(&cameras, &points, &mut observations, &filter);
        assert_eq!(report.observations_removed, expected.iter().filter(|&&k| !k).count());
        assert_eq!(report.tracks_removed, 0);
        assert_eq!(observations.len(), expected.iter().filter(|&&k| k).count());
        // Every surviving observation stays within the threshold.
        for observation in &observations {
            let uv = cameras[observation.camera]
                .as_ref()
                .unwrap()
                .project(points[observation.point])
                .unwrap();
            let error = (uv[0] - observation.xy[0]).hypot(uv[1] - observation.xy[1]);
            assert!(error <= 4., "{error}");
        }
    }

    #[test]
    fn filter_drops_tracks_reduced_below_two_views_or_parallax() {
        let (cameras, points, _) = scene(false, false);
        // Point 0 keeps only camera 0 within the threshold: a corrupted camera 1
        // observation is dropped, leaving a single view, so the track goes too.
        let mut observations = Vec::new();
        for (point, position) in points.iter().enumerate().take(8) {
            for (camera, pose) in cameras.iter().enumerate() {
                let mut xy = pose.as_ref().unwrap().project(*position).unwrap();
                if point == 0 && camera > 0 {
                    xy[0] += 30.;
                }
                observations.push(Observation { camera, point, xy });
            }
        }
        let filter = FilterOptions {
            max_reprojection_error: 4.,
            min_parallax: 0.,
        };
        let mut retained = observations.clone();
        let report = filter_observations(&cameras, &points, &mut retained, &filter);
        assert_eq!(report.observations_removed, 4);
        assert_eq!(report.tracks_removed, 1);
        assert!(retained.iter().all(|o| o.point != 0));
        // A parallax floor above the widest baseline angle drops every track.
        let mut retained = observations;
        let report = filter_observations(
            &cameras,
            &points,
            &mut retained,
            &FilterOptions {
                max_reprojection_error: 400.,
                min_parallax: 0.9,
            },
        );
        assert_eq!(report.tracks_removed, 8);
        assert!(retained.is_empty());
    }

    #[test]
    fn filtering_between_runs_improves_outlier_recovery() {
        let (mut cameras, truth, observations) = scene(true, true);
        let mut points = truth.clone();
        distort(&mut cameras, &mut points);
        let mut single_pass_cameras = cameras.clone();
        let mut single_pass_points = points.clone();
        optimize(
            &mut single_pass_cameras,
            &mut single_pass_points,
            &observations,
            0,
            1,
            &BundleOptions {
                max_iterations: 20,
                ..Default::default()
            },
            |_| true,
        )
        .unwrap();
        let filter = FilterOptions {
            max_reprojection_error: 4.,
            min_parallax: 0.,
        };
        optimize(
            &mut cameras,
            &mut points,
            &observations,
            0,
            1,
            &BundleOptions {
                max_iterations: 20,
                ..Default::default()
            },
            |_| true,
        )
        .unwrap();
        let mut retained = observations.clone();
        let removed = filter_observations(&cameras, &points, &mut retained, &filter);
        assert!(removed.observations_removed > 0);
        let second = optimize(
            &mut cameras,
            &mut points,
            &retained,
            0,
            1,
            &BundleOptions {
                max_iterations: 20,
                ..Default::default()
            },
            |_| true,
        )
        .unwrap();
        let clean_error = |positions: &[V3]| {
            let mut errors: Vec<_> = positions
                .iter()
                .zip(&truth)
                .enumerate()
                .filter(|(i, _)| (0..4).all(|c| (i * 4 + c) % 43 != 0))
                .map(|(_, (&a, &b))| norm(sub(a, b)))
                .collect();
            errors.sort_by(f64::total_cmp);
            errors[errors.len() / 2]
        };
        let filtered_error = clean_error(&points);
        let unfiltered_error = clean_error(&single_pass_points);
        assert!(
            filtered_error < unfiltered_error,
            "filtered median {filtered_error}, unfiltered {unfiltered_error}"
        );
        eprintln!(
            "bundle filter: removed {} observations / {} tracks, clean median shape error {unfiltered_error} -> {filtered_error}, second run cost {}",
            removed.observations_removed,
            removed.tracks_removed,
            second.final_cost,
        );
    }
}
