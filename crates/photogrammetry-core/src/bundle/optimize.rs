use super::*;

pub(super) fn linearize(
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
    for (i, row) in h.iter_mut().enumerate() {
        row[i] += damping * row[i].max(1e-9);
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
pub(super) fn solve_reduced(h: &mut [f64], rhs: &mut [f64], checkpoints: &mut Checkpoints) -> Option<()> {
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

pub(super) struct Step {
    pub(super) cameras: Vec<f64>,
    pub(super) points: Vec<V3>,
}
/// Reusable camera-system storage shared by every damping trial of one call.
#[derive(Default)]
pub(super) struct SchurScratch {
    h: Vec<f64>,
    rhs: Vec<f64>,
}
pub(super) fn schur_step(
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
) -> Result<BundleReport> {
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
        return Err(crate::error("Cancelled"));
    }
    let initial_cost =
        initial_cost.ok_or_else(|| crate::error("Non-finite initial bundle objective"))?;
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
            return Err(crate::error("Cancelled"));
        }
        let linear = linearize(
            &current_cameras,
            &current_points,
            observations,
            &layout,
            options.huber_delta,
            &mut checkpoints,
        )
        .ok_or_else(|| crate::error("Cancelled"))?;
        let previous = report.final_cost;
        let mut accepted = false;
        for _ in 0..options.max_trials {
            if !checkpoints.check() {
                return Err(crate::error("Cancelled"));
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
            if proposed
                && let Some(next_cost) = cost(
                    &trial_cameras,
                    &trial_points,
                    observations,
                    options.huber_delta,
                    &mut checkpoints,
                )
                && next_cost < previous
            {
                std::mem::swap(&mut current_cameras, &mut trial_cameras);
                std::mem::swap(&mut current_points, &mut trial_points);
                report.final_cost = next_cost;
                report.accepted_steps += 1;
                damping = (damping / 3.).max(1e-12);
                accepted = true;
                break;
            }
            if checkpoints.cancelled {
                return Err(crate::error("Cancelled"));
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
        return Err(crate::error("Cancelled"));
    }
    cameras.clone_from_slice(&current_cameras);
    positions.copy_from_slice(&current_points);
    Ok(report)
}
