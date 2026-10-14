use super::*;

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
    pub fn validate(&self) -> Result<()> {
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
            return Err(crate::error("Invalid bundle adjustment options"));
        }
        if let Some(filter) = &self.filter
            && (!filter.max_reprojection_error.is_finite()
                || !(0. ..=100.).contains(&filter.max_reprojection_error)
                || filter.max_reprojection_error <= 0.
                || !filter.min_parallax.is_finite()
                || !(0. ..=1.).contains(&filter.min_parallax))
        {
            return Err(crate::error("Invalid bundle adjustment filter options"));
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
                .and_then(|camera| {
                    positions
                        .get(observation.point)
                        .and_then(|p| camera.project(*p))
                })
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
pub(super) struct Checkpoints<'a> {
    pub(super) progress: &'a mut dyn FnMut(BundleProgress) -> bool,
    pub(super) event: BundleProgress,
    pub(super) cancelled: bool,
}
impl<'a> Checkpoints<'a> {
    pub(super) fn new(progress: &'a mut dyn FnMut(BundleProgress) -> bool) -> Self {
        Self {
            progress,
            event: BundleProgress {
                iteration: 0,
                cost: 0.,
            },
            cancelled: false,
        }
    }
    pub(super) fn check(&mut self) -> bool {
        if !self.cancelled {
            self.cancelled = !(self.progress)(self.event);
        }
        !self.cancelled
    }
    pub(super) fn every(&mut self, index: usize, interval: usize) -> bool {
        !index.is_multiple_of(interval) || self.check()
    }
}

pub(super) struct Layout {
    pub(super) columns: Vec<Option<usize>>,
    pub(super) active_cameras: Vec<usize>,
    pub(super) tracks: Vec<Range<usize>>,
    pub(super) order: Vec<usize>,
    pub(super) dimension: usize,
    pub(super) anchor_center: V3,
    pub(super) baseline: f64,
}

/// Iterative union-find root with path halving; deterministic and float-free.
fn find_root(parent: &mut [usize], mut index: usize) -> usize {
    while parent[index] != index {
        parent[index] = parent[parent[index]];
        index = parent[index];
    }
    index
}

pub(super) fn validate(
    cameras: &[Option<Camera>],
    positions: &[V3],
    observations: &[Observation],
    anchor: usize,
    scale_camera: usize,
    options: &BundleOptions,
    checkpoints: &mut Checkpoints,
) -> Result<Layout> {
    if !checkpoints.check() {
        do yeet crate::error("Cancelled");
    }
    options.validate()?;
    if cameras.len() > 200
        || cameras.iter().flatten().count() > options.max_cameras
        || positions.len() > options.max_points
        || observations.len() > options.max_observations
        || positions.len() < 6
        || observations.len() < 12
    {
        do yeet crate::error(
            "Bundle adjustment input exceeds its bounds or has too little support",
        );
    }
    let Some(anchor_pose) = cameras.get(anchor).and_then(Option::as_ref) else {
        do yeet crate::error("Bundle anchor camera is not registered");
    };
    let Some(scale_pose) = cameras.get(scale_camera).and_then(Option::as_ref) else {
        do yeet crate::error("Bundle scale camera is not registered");
    };
    let anchor_center = anchor_pose.center();
    let baseline = norm(sub(scale_pose.center(), anchor_center));
    if anchor == scale_camera || !baseline.is_finite() || baseline <= 1e-10 {
        do yeet crate::error("Bundle adjustment needs a nonzero initial baseline");
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
            do yeet crate::error("Invalid bundle camera");
        }
        let rrt = mm(camera.rotation, tr(camera.rotation));
        if (0..3).any(|i| (0..3).any(|j| (rrt[i][j] - f64::from(i == j)).abs() > 1e-3)) {
            do yeet crate::error("Bundle camera rotation is not orthonormal");
        }
    }
    for (index, point) in positions.iter().enumerate() {
        if !checkpoints.every(index, 256) {
            do yeet crate::error("Cancelled");
        }
        if !point.iter().all(|v| v.is_finite()) {
            do yeet crate::error("Non-finite bundle point");
        }
    }
    let mut seen = vec![false; cameras.len()];
    for (index, observation) in observations.iter().enumerate() {
        if !checkpoints.every(index, 256) {
            do yeet crate::error("Cancelled");
        }
        let Some(camera) = cameras.get(observation.camera).and_then(Option::as_ref) else {
            do yeet crate::error("Bundle observation references an unregistered camera");
        };
        let Some(&position) = positions.get(observation.point) else {
            do yeet crate::error("Bundle observation references a missing point");
        };
        if !observation.xy.iter().all(|v| v.is_finite()) || projection(camera, position).is_none() {
            do yeet crate::error("Invalid bundle observation or non-positive initial depth");
        }
        seen[observation.camera] = true;
    }
    if !seen[anchor] || !seen[scale_camera] {
        return Err(crate::error("Bundle gauge cameras must have observations"));
    }
    // Bounded counting order replaces a non-interruptible O(n log n) sort.
    let mut offsets = vec![0; positions.len() + 1];
    for (index, observation) in observations.iter().enumerate() {
        if !checkpoints.every(index, 256) {
            return Err(crate::error("Cancelled"));
        }
        offsets[observation.point + 1] += 1;
    }
    for index in 1..offsets.len() {
        if !checkpoints.every(index, 256) {
            return Err(crate::error("Cancelled"));
        }
        offsets[index] += offsets[index - 1];
    }
    let mut order = vec![0; observations.len()];
    for (index, observation) in observations.iter().enumerate() {
        if !checkpoints.every(index, 256) {
            return Err(crate::error("Cancelled"));
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
            return Err(crate::error("Cancelled"));
        }
        let point = observations[order[start]].point;
        let mut end = start;
        while end < order.len() && observations[order[end]].point == point {
            let camera = observations[order[end]].camera;
            if last_point[camera] == point {
                return Err(crate::error(
                    "Duplicate bundle observation for one point and camera",
                ));
            }
            last_point[camera] = point;
            end += 1;
        }
        if end - start < 2 {
            return Err(crate::error(
                "Bundle points need at least two distinct views",
            ));
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
            return Err(crate::error("Bundle point has no triangulation parallax"));
        }
        tracks.push(start..end);
        start = end;
    }
    if tracks.len() < 6 {
        return Err(crate::error(
            "Bundle adjustment needs at least six observed points",
        ));
    }
    // Reject disconnected components: every optimized pose must share a track path
    // with the anchor, otherwise a single gauge cannot constrain the whole problem.
    // One union-find pass over the camera/track hypergraph replaces repeated sweeps.
    let mut parent: Vec<usize> = (0..cameras.len()).collect();
    for (index, track) in tracks.iter().enumerate() {
        if !checkpoints.every(index, 128) {
            return Err(crate::error("Cancelled"));
        }
        let mut members = order[track.clone()].iter().map(|&i| observations[i].camera);
        let first = members.next().unwrap();
        for camera in members {
            let (a, b) = (
                find_root(&mut parent, first),
                find_root(&mut parent, camera),
            );
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
        return Err(crate::error(
            "Bundle observations contain disconnected camera groups",
        ));
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

pub(super) fn projection(camera: &Camera, point: V3) -> Option<([f64; 2], V3)> {
    let transformed = camera.camera_point(point);
    camera
        .project_camera_point(transformed)
        .map(|pixel| (pixel, transformed))
}

/// Left SE(3) update: R' = exp(w) R, t' = exp(w) t + v.
pub(super) fn perturb(camera: &Camera, delta: &[f64]) -> Camera {
    let dr = rotation([delta[0], delta[1], delta[2]]);
    let mut result = camera.clone();
    result.rotation = mm(dr, camera.rotation);
    result.translation = add(mv(dr, camera.translation), [delta[3], delta[4], delta[5]]);
    result
}

pub(super) fn jacobians(camera: &Camera, transformed: V3) -> ([[f64; 6]; 2], [[f64; 3]; 2]) {
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

pub(super) fn huber(residual: [f64; 2], delta: f64) -> (f64, f64) {
    let length = residual[0].hypot(residual[1]);
    if length <= delta {
        (0.5 * length * length, 1.)
    } else {
        (delta * (length - 0.5 * delta), delta / length)
    }
}

pub(super) fn cost(
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

pub(super) struct CrossBlock {
    pub(super) column: usize,
    pub(super) values: [[f64; 3]; 6],
}
pub(super) struct PointBlock {
    pub(super) point: usize,
    pub(super) hessian: M3,
    pub(super) rhs: V3,
    pub(super) cross: Range<usize>,
}
pub(super) struct Linearization {
    pub(super) camera_hessian: Vec<f64>,
    pub(super) camera_rhs: Vec<f64>,
    pub(super) points: Vec<PointBlock>,
    pub(super) cross: Vec<CrossBlock>,
}
