use super::bounds::{GpuPointBounds, GpuTransformedPointBounds};
use super::distance::{GpuDistancePairSum, GpuDistancePairs, GpuTransformedDistancePairSum};
use super::moments::{GpuPointCloudStats, GpuPointMoments};
use super::neighbors::{GpuChamfer, GpuNearestFour, GpuNearestNeighbor, GpuNearestTwo};
use super::{GpuArithmetic, GpuMathError, MathExecution};
use crate::{M3, V3};
use gpu_compute::{BackendReport, GpuContext};
use std::cell::OnceCell;

/// Owns lazy math kernels and their buffer caches on a caller-selected device.
/// A session is intended for one host thread (its caches use RefCell). Clone the
/// GpuContext to share a device/queue with compute or raster sessions. The caller
/// controls this session's lifetime and drops it outside thread-local teardown.
pub struct MathGpuSession {
    context: GpuContext,
    pub(super) point_kernels:
        OnceCell<Result<super::plans::PointKernels, compute_core::KernelError>>,
    nearest_neighbor: OnceCell<Result<GpuNearestNeighbor, compute_core::KernelError>>,
    squared_distance_pairs: OnceCell<Result<GpuDistancePairs, compute_core::KernelError>>,
    squared_distance_pair_sum: OnceCell<Result<GpuDistancePairSum, compute_core::KernelError>>,
    transformed_squared_distance_pair_sum:
        OnceCell<Result<GpuTransformedDistancePairSum, compute_core::KernelError>>,
    point_bounds: OnceCell<Result<GpuPointBounds, compute_core::KernelError>>,
    transformed_point_bounds:
        OnceCell<Result<GpuTransformedPointBounds, compute_core::KernelError>>,
    point_moments: OnceCell<Result<GpuPointMoments, compute_core::KernelError>>,
    point_cloud_stats: OnceCell<Result<GpuPointCloudStats, compute_core::KernelError>>,
    #[cfg(not(target_arch = "wasm32"))]
    stable_stats_runtime: OnceCell<compute_core::ComputeRuntime>,
    directed_chamfer: OnceCell<Result<GpuChamfer, compute_core::KernelError>>,
    nearest_two: OnceCell<Result<GpuNearestTwo, compute_core::KernelError>>,
    nearest_four: OnceCell<Result<GpuNearestFour, compute_core::KernelError>>,
}

impl MathGpuSession {
    pub fn new(context: &GpuContext) -> Self {
        Self {
            context: context.clone(),
            point_kernels: OnceCell::new(),
            nearest_neighbor: OnceCell::new(),
            squared_distance_pairs: OnceCell::new(),
            squared_distance_pair_sum: OnceCell::new(),
            transformed_squared_distance_pair_sum: OnceCell::new(),
            point_bounds: OnceCell::new(),
            transformed_point_bounds: OnceCell::new(),
            point_moments: OnceCell::new(),
            point_cloud_stats: OnceCell::new(),
            #[cfg(not(target_arch = "wasm32"))]
            stable_stats_runtime: OnceCell::new(),
            directed_chamfer: OnceCell::new(),
            nearest_two: OnceCell::new(),
            nearest_four: OnceCell::new(),
        }
    }
    pub fn context(&self) -> &GpuContext {
        &self.context
    }
    pub fn backend_report(&self) -> BackendReport {
        self.context.backend_report()
    }
    /// Shared resident tensor geometry on this session's existing device/queue.
    /// The returned recipes report backend errors and perform no CPU fallback.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn tensor(
        &self,
    ) -> Result<crate::tensor::TensorMath<'_, compute_core::ComputeRuntime>, GpuMathError> {
        Ok(crate::tensor::TensorMath::new(self.tensor_runtime()?))
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn tensor_runtime(&self) -> Result<&compute_core::ComputeRuntime, GpuMathError> {
        if self.stable_stats_runtime.get().is_none() {
            let runtime = compute_core::ComputeRuntime::new(&self.context)?;
            let _ = self.stable_stats_runtime.set(runtime);
        }
        Ok(self
            .stable_stats_runtime
            .get()
            .expect("runtime initialized above"))
    }
    pub fn nearest_neighbor(&self, queries: &[V3], targets: &[V3]) -> Option<Vec<(u32, f64)>> {
        self.try_nearest_neighbor(queries, targets)
            .ok()
            .map(|result| result.value)
    }

    pub fn try_nearest_neighbor(
        &self,
        queries: &[V3],
        targets: &[V3],
    ) -> Result<MathExecution<Vec<(u32, f64)>>, GpuMathError> {
        self.validate_points(queries)?;
        self.validate_points(targets)?;
        self.execute(|| {
            let value = self
                .nearest_neighbor
                .get_or_init(|| GpuNearestNeighbor::new(&self.context))
                .as_ref()
                .map_err(|e| GpuMathError::Kernel(e.clone()))?
                .run(queries, targets)?;
            Ok(value)
        })
    }
    pub fn squared_distance_pairs(&self, a: &[V3], b: &[V3]) -> Option<Vec<f64>> {
        self.try_squared_distance_pairs(a, b)
            .ok()
            .map(|result| result.value)
    }

    pub fn try_squared_distance_pairs(
        &self,
        a: &[V3],
        b: &[V3],
    ) -> Result<MathExecution<Vec<f64>>, GpuMathError> {
        if a.len() != b.len() {
            return Err(GpuMathError::InvalidInput("empty or incompatible inputs"));
        }
        self.validate_points(a)?;
        self.validate_points(b)?;
        self.execute(|| {
            let value = self
                .squared_distance_pairs
                .get_or_init(|| GpuDistancePairs::new(&self.context))
                .as_ref()
                .map_err(|e| GpuMathError::Kernel(e.clone()))?
                .run(a, b)?;
            Ok(value)
        })
    }
    pub fn squared_distance_pair_sum(&self, a: &[V3], b: &[V3]) -> Option<f64> {
        self.try_squared_distance_pair_sum(a, b)
            .ok()
            .map(|result| result.value)
    }

    pub fn try_squared_distance_pair_sum(
        &self,
        a: &[V3],
        b: &[V3],
    ) -> Result<MathExecution<f64>, GpuMathError> {
        if a.len() != b.len() {
            return Err(GpuMathError::InvalidInput("empty or incompatible inputs"));
        }
        self.validate_points(a)?;
        self.validate_points(b)?;
        self.execute(|| {
            let value = self
                .squared_distance_pair_sum
                .get_or_init(|| GpuDistancePairSum::new(&self.context))
                .as_ref()
                .map_err(|e| GpuMathError::Kernel(e.clone()))?
                .run(a, b)?;
            Ok(value)
        })
    }
    pub fn transformed_squared_distance_pair_sum(
        &self,
        source: &[V3],
        target: &[V3],
        m: M3,
        t: V3,
    ) -> Option<f64> {
        self.try_transformed_squared_distance_pair_sum(source, target, m, t)
            .ok()
            .map(|result| result.value)
    }

    pub fn try_transformed_squared_distance_pair_sum(
        &self,
        source: &[V3],
        target: &[V3],
        m: M3,
        t: V3,
    ) -> Result<MathExecution<f64>, GpuMathError> {
        if source.len() != target.len() {
            return Err(GpuMathError::InvalidInput("empty or incompatible inputs"));
        }
        self.validate_points(source)?;
        self.validate_points(target)?;
        self.validate_transform(m, t)?;
        self.execute(|| {
            let value = self
                .transformed_squared_distance_pair_sum
                .get_or_init(|| GpuTransformedDistancePairSum::new(&self.context))
                .as_ref()
                .map_err(|e| GpuMathError::Kernel(e.clone()))?
                .run(source, target, m, t)?;
            Ok(value)
        })
    }
    pub fn point_bounds(&self, points: &[V3]) -> Option<crate::PointBounds> {
        self.try_point_bounds(points)
            .ok()
            .map(|result| result.value)
    }

    pub fn try_point_bounds(
        &self,
        points: &[V3],
    ) -> Result<MathExecution<crate::PointBounds>, GpuMathError> {
        if points.is_empty() {
            return Err(GpuMathError::InvalidInput("empty or incompatible inputs"));
        }
        self.validate_points(points)?;
        self.execute(|| {
            let value = self
                .point_bounds
                .get_or_init(|| GpuPointBounds::new(&self.context))
                .as_ref()
                .map_err(|e| GpuMathError::Kernel(e.clone()))?
                .run(points)?;
            Ok(value)
        })
    }
    pub fn transformed_point_bounds(
        &self,
        points: &[V3],
        m: M3,
        t: V3,
    ) -> Option<crate::PointBounds> {
        self.try_transformed_point_bounds(points, m, t)
            .ok()
            .map(|result| result.value)
    }

    pub fn try_transformed_point_bounds(
        &self,
        points: &[V3],
        m: M3,
        t: V3,
    ) -> Result<MathExecution<crate::PointBounds>, GpuMathError> {
        if points.is_empty() {
            return Err(GpuMathError::InvalidInput("empty or incompatible inputs"));
        }
        self.validate_points(points)?;
        self.validate_transform(m, t)?;
        self.execute(|| {
            let value = self
                .transformed_point_bounds
                .get_or_init(|| GpuTransformedPointBounds::new(&self.context))
                .as_ref()
                .map_err(|e| GpuMathError::Kernel(e.clone()))?
                .run(points, m, t)?;
            Ok(value)
        })
    }
    pub fn point_moments(&self, points: &[V3]) -> Option<crate::PointMoments> {
        self.try_point_moments(points)
            .ok()
            .map(|result| result.value)
    }

    pub fn try_point_moments(
        &self,
        points: &[V3],
    ) -> Result<MathExecution<crate::PointMoments>, GpuMathError> {
        if points.is_empty() {
            return Err(GpuMathError::InvalidInput("empty or incompatible inputs"));
        }
        self.validate_points(points)?;
        self.execute(|| {
            let value = self
                .point_moments
                .get_or_init(|| GpuPointMoments::new(&self.context))
                .as_ref()
                .map_err(|e| GpuMathError::Kernel(e.clone()))?
                .run(points)?;
            Ok(value)
        })
    }
    pub fn point_cloud_stats(&self, points: &[V3]) -> Option<crate::PointCloudStats> {
        self.try_point_cloud_stats(points)
            .ok()
            .map(|result| result.value)
    }

    pub fn try_point_cloud_stats(
        &self,
        points: &[V3],
    ) -> Result<MathExecution<crate::PointCloudStats>, GpuMathError> {
        if points.is_empty() {
            return Err(GpuMathError::InvalidInput("empty or incompatible inputs"));
        }
        self.validate_points(points)?;
        self.execute(|| {
            let value = self
                .point_cloud_stats
                .get_or_init(|| GpuPointCloudStats::new(&self.context))
                .as_ref()
                .map_err(|e| GpuMathError::Kernel(e.clone()))?
                .run(points)?;
            Ok(value)
        })
    }

    /// Explicit f32 statistics with a second, centered covariance pass. Uses
    /// the recorded stable plan and reads back only the final 24 scalars.
    /// Kernels are cached; each call uploads the points and allocates its plan.
    /// For repeated device-resident work, use
    /// [`super::MathGpuProgram::point_cloud_stats_stable`] instead.
    ///
    /// Raw moments keep the original meaning `E[p*p^T]`. Centered arithmetic
    /// improves covariance at large offsets but cannot recover differences
    /// lost during f64-to-f32 input conversion, or handle overflowing f32 sums.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_point_cloud_stats_stable(
        &self,
        points: &[V3],
    ) -> Result<MathExecution<crate::PointCloudStats>, GpuMathError> {
        if points.is_empty() {
            return Err(GpuMathError::InvalidInput("empty or incompatible inputs"));
        }
        self.validate_points(points)?;
        self.execute(|| {
            let runtime = self.tensor_runtime()?;
            let flat: Vec<f32> = points.iter().flatten().map(|&v| v as f32).collect();
            let input = runtime.upload(&flat)?;
            let mut plan = self.program(runtime)?;
            let stats = plan.point_cloud_stats_stable(super::PointCloudView::new(&input)?)?;
            let mut encoder = self
                .context
                .device
                .create_command_encoder(&Default::default());
            plan.record(&mut encoder);
            let mut read = runtime.record_read(&mut encoder, &stats.values)?;
            read.submitted(self.context.queue.submit([encoder.finish()]));
            let packed = read.wait(std::time::Duration::from_secs(30))?;
            let get = |i| f64::from(packed[i]);
            let bounds = crate::PointBounds::new(
                points.len(),
                [get(0), get(1), get(2)],
                [get(3), get(4), get(5)],
            );
            let mut moments = crate::PointMoments::from_sums(
                points.len(),
                [get(6), get(7), get(8)],
                [get(9), get(10), get(11), get(12), get(13), get(14)],
            );
            moments.centroid = [get(15), get(16), get(17)];
            moments.covariance = [
                [get(18), get(19), get(20)],
                [get(19), get(21), get(22)],
                [get(20), get(22), get(23)],
            ];
            Ok(crate::PointCloudStats::from_parts(bounds, moments))
        })
    }
    pub fn directed_chamfer(
        &self,
        queries: &[V3],
        targets: &[V3],
    ) -> Option<crate::DirectedChamfer> {
        self.try_directed_chamfer(queries, targets)
            .ok()
            .map(|result| result.value)
    }

    pub fn try_directed_chamfer(
        &self,
        queries: &[V3],
        targets: &[V3],
    ) -> Result<MathExecution<crate::DirectedChamfer>, GpuMathError> {
        if queries.is_empty() || targets.is_empty() {
            return Err(GpuMathError::InvalidInput("empty or incompatible inputs"));
        }
        self.validate_points(queries)?;
        self.validate_points(targets)?;
        self.execute(|| {
            let value = self
                .directed_chamfer
                .get_or_init(|| GpuChamfer::new(&self.context))
                .as_ref()
                .map_err(|e| GpuMathError::Kernel(e.clone()))?
                .run(queries, targets)?;
            Ok(value)
        })
    }
    pub fn nearest_two(&self, queries: &[V3], targets: &[V3]) -> Option<Vec<crate::TwoNearest>> {
        self.try_nearest_two(queries, targets)
            .ok()
            .map(|result| result.value)
    }

    pub fn try_nearest_two(
        &self,
        queries: &[V3],
        targets: &[V3],
    ) -> Result<MathExecution<Vec<crate::TwoNearest>>, GpuMathError> {
        self.validate_points(queries)?;
        self.validate_points(targets)?;
        self.execute(|| {
            let value = self
                .nearest_two
                .get_or_init(|| GpuNearestTwo::new(&self.context))
                .as_ref()
                .map_err(|e| GpuMathError::Kernel(e.clone()))?
                .run(queries, targets)?;
            Ok(value)
        })
    }
    pub fn nearest_four(&self, queries: &[V3], targets: &[V3]) -> Option<Vec<crate::FourNearest>> {
        self.try_nearest_four(queries, targets)
            .ok()
            .map(|result| result.value)
    }

    pub fn try_nearest_four(
        &self,
        queries: &[V3],
        targets: &[V3],
    ) -> Result<MathExecution<Vec<crate::FourNearest>>, GpuMathError> {
        self.validate_points(queries)?;
        self.validate_points(targets)?;
        self.execute(|| {
            let value = self
                .nearest_four
                .get_or_init(|| GpuNearestFour::new(&self.context))
                .as_ref()
                .map_err(|e| GpuMathError::Kernel(e.clone()))?
                .run(queries, targets)?;
            Ok(value)
        })
    }
    fn execute<T>(
        &self,
        run: impl FnOnce() -> Result<T, GpuMathError>,
    ) -> Result<MathExecution<T>, GpuMathError> {
        let oom = self
            .context
            .device
            .push_error_scope(gpu_compute::wgpu::ErrorFilter::OutOfMemory);
        let internal = self
            .context
            .device
            .push_error_scope(gpu_compute::wgpu::ErrorFilter::Internal);
        let validation = self
            .context
            .device
            .push_error_scope(gpu_compute::wgpu::ErrorFilter::Validation);
        let result = run();
        let errors = [
            gpu_compute::block_on(validation.pop()),
            gpu_compute::block_on(internal.pop()),
            gpu_compute::block_on(oom.pop()),
        ];
        if let Some(error) = errors.into_iter().flatten().next() {
            return Err(GpuMathError::Device(error.to_string()));
        }
        Ok(MathExecution {
            value: result?,
            backend: self.backend_report(),
            arithmetic: GpuArithmetic::F32,
        })
    }

    fn validate_points(&self, points: &[V3]) -> Result<(), GpuMathError> {
        let limit = self.context.device.limits().max_storage_buffer_binding_size;
        if points.len() > 65535 * 256 || (points.len() as u64).saturating_mul(16) > limit {
            return Err(GpuMathError::InvalidInput(
                "point count exceeds GPU buffer/dispatch limits",
            ));
        }
        if points.iter().flatten().any(|v| !(*v as f32).is_finite()) {
            return Err(GpuMathError::InvalidInput(
                "coordinate is not representable as finite f32",
            ));
        }
        Ok(())
    }
    fn validate_transform(&self, m: M3, t: V3) -> Result<(), GpuMathError> {
        if m.iter()
            .flatten()
            .chain(t.iter())
            .any(|v| !(*v as f32).is_finite())
        {
            return Err(GpuMathError::InvalidInput(
                "transform is not representable as finite f32",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_session_shares_device_and_keeps_math_contracts() {
        let Some(context) = GpuContext::new() else {
            assert!(
                std::env::var_os("COMPUTE_REQUIRE_GPU").is_none(),
                "GPU adapter required"
            );
            return;
        };
        let session = MathGpuSession::new(&context);
        assert_eq!(session.context().device, context.device);
        assert_eq!(session.context().queue, context.queue);
        // Initialize independent kernel families, then reuse their capacities
        // with smaller inputs. Domain values still match CPU reference math.
        let points = [[0.0, 0.0, 0.0], [3.0, 4.0, 0.0], [-1.0, 2.0, 1.0]];
        let origin = [[0.0; 3]; 3];
        for n in [3, 1, 2] {
            let a = &points[..n];
            let b = &origin[..n];
            assert_eq!(
                session.squared_distance_pairs(a, b).unwrap(),
                crate::squared_distance_pairs(a, b).unwrap()
            );
            assert_eq!(
                session.squared_distance_pair_sum(a, b).unwrap(),
                crate::squared_distance_pair_sum(a, b).unwrap()
            );
            assert_eq!(
                session.point_bounds(a).unwrap(),
                crate::point_bounds(a).unwrap()
            );
            assert_eq!(
                session.nearest_neighbor(a, b).unwrap(),
                crate::nearest_neighbor(a, b)
            );
        }
        assert!(
            session
                .squared_distance_pairs(&points, &origin[..1])
                .is_none()
        );
        assert!(session.point_bounds(&[]).is_none());
        assert!(session.directed_chamfer(&points, &[]).is_none());
        let runtime = compute_core::ComputeRuntime::new(session.context()).unwrap();
        let input = runtime.upload(&[3.0f32, 4.0]).unwrap();
        let mut program = runtime.program();
        let result = program.dot(&input, &input).unwrap();
        assert_eq!(
            program
                .submit_read(&result)
                .unwrap()
                .wait(std::time::Duration::from_secs(10))
                .unwrap(),
            [25.0]
        );
        drop(session); // Explicit sessions release normally, outside TLS teardown.
        assert_eq!(
            runtime
                .read(&input)
                .unwrap()
                .wait(std::time::Duration::from_secs(10))
                .unwrap(),
            [3.0, 4.0]
        );
    }
}
