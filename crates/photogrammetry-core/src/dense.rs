//! Bounded dense reconstruction: plane sweep, reciprocal depth consistency and
//! normal-aware fusion of observed samples. Experimental shared-volume extraction
//! allows bounded continuation within pixel footprints, without global hole filling.
mod consistency;
mod estimation;
mod fusion;
mod limits;
mod mesh;
mod plane;
mod selection;
mod simplify;
mod volume;
pub use estimation::{
    HOST_SWEEP_LINEAR_INDEXING_VARIANT_LABEL, HostSweepSource, HostSweepView, PreparedView,
    SWEEP_WGSL, host_sweep_linear_indexing_wgsl,
};
pub use simplify::{simplify, simplify_with_progress};
#[cfg(test)]
mod quality_tests;

use crate::{Image, Reconstruction, Result, math::*};
pub use mesh::{compact, filter_small_components};

#[cfg(test)]
use crate::{Point, camera::Camera};
#[cfg(test)]
#[path = "../examples/support/dense_fixture.rs"]
mod fixture;

#[derive(Clone, Debug, Default)]
pub struct Surface {
    pub positions: Vec<V3>,
    pub colors: Vec<[u8; 3]>,
    pub triangles: Vec<[u32; 3]>,
}

/// The original sweep is retained for reproducible comparisons and rollback.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DenseEstimator {
    #[default]
    FrontoparallelSweep,
    /// Global depth initialization followed by bounded depth/normal plane refinement.
    SlantedPlane,
}

/// Parameters are expressed in source-normalized depth-map pixels and relative
/// scene units. `min_support_views` excludes the reference image itself.
#[derive(Clone, Debug)]
pub struct DenseOptions {
    /// Qualification-only shared signed-distance surface candidate.
    pub shared_volume: bool,
    /// Experimental two-footprint selection and curvature support; requires radius 2.
    pub dual_scale: bool,
    /// Opt-in checkerboard (red-black) hypothesis propagation for SlantedPlane;
    /// false keeps the historical forward/backward scanline order bit-identical.
    pub red_black_propagation: bool,
    /// Opt-in early stop when a SlantedPlane refinement pass or descent cycle
    /// no longer improves any hypothesis.
    pub adaptive_refine_budget: bool,
    /// Opt-in reciprocal checks against the selected source views only;
    /// false keeps polling every depth map.
    pub selected_sources_consistency: bool,
    /// Opt-in per-pixel depth intervals from sparse anchors narrow the
    /// frontoparallel sweep; uncovered pixels keep the global range.
    pub sparse_depth_prior: bool,
    /// Opt-in half-resolution sweep, then a full-resolution pass refined
    /// around the lifted depth. Sparse anchors take precedence where both
    /// modes cover a pixel. Frontoparallel estimator only.
    pub coarse_to_fine: bool,
    pub estimator: DenseEstimator,
    pub max_side: usize,
    pub depth_hypotheses: usize,
    /// Radius 1 retains the original 3x3 patch; radius 2 uses 5x5 in either mode.
    pub patch_radius: usize,
    pub max_source_views: usize,
    pub min_support_views: usize,
    pub min_correlation: f64,
    pub uniqueness_margin: f64,
    pub relative_depth_tolerance: f64,
    pub reprojection_tolerance: f64,
    /// Fuse compatible observations; false retains independent observed patches
    /// and is useful for an identical-depth baseline comparison.
    pub fuse: bool,
    /// Opt-in deterministic second pass merging compatible surfels by their
    /// accumulated averages; reduces overlapping fragments whose fixed anchors
    /// rejected each other. Inert when `fuse` is false.
    pub fusion_merge_pass: bool,
    /// Opt-in removal of connected surface components with fewer triangles
    /// than this threshold; 0 (default) keeps every component.
    pub min_component_triangles: usize,
    /// Opt-in GPU evaluation of the frontoparallel NCC sweep (requires the
    /// `gpu` crate feature; falls back to the CPU sweep without an adapter).
    /// GPU arithmetic is f32 and scores are not bit-identical to the CPU
    /// reference; the mode is qualified separately. Selection logic and
    /// geometry checks stay on the CPU.
    pub acceleration: crate::Acceleration,
}
impl Default for DenseOptions {
    fn default() -> Self {
        Self {
            shared_volume: false,
            dual_scale: false,
            red_black_propagation: false,
            adaptive_refine_budget: false,
            selected_sources_consistency: false,
            sparse_depth_prior: false,
            coarse_to_fine: false,
            estimator: DenseEstimator::FrontoparallelSweep,
            max_side: 128,
            depth_hypotheses: 64,
            patch_radius: 1,
            max_source_views: 3,
            min_support_views: 1,
            min_correlation: 0.78,
            uniqueness_margin: 0.015,
            relative_depth_tolerance: 0.025,
            reprojection_tolerance: 1.5,
            fuse: true,
            fusion_merge_pass: false,
            min_component_triangles: 0,
            acceleration: crate::Acceleration::Cpu,
        }
    }
}
impl DenseOptions {
    /// Qualified accuracy bundle for the frontoparallel sweep: 5x5 patches,
    /// the dual-scale secondary pass and sparse depth intervals. On the
    /// analytic scenes this lowers mean surface error by ~27% and raises
    /// mean F1 by ~3% versus the defaults; the extra sweep work is what the
    /// batched GPU path (`acceleration: Gpu`) absorbs.
    pub fn accurate() -> Self {
        Self {
            dual_scale: true,
            sparse_depth_prior: true,
            patch_radius: 2,
            ..Self::default()
        }
    }

    fn validate(&self) -> Result<()> {
        if (self.dual_scale && self.patch_radius != 2)
            || !(64..=384).contains(&self.max_side)
            || !(16..=128).contains(&self.depth_hypotheses)
            || !(1..=2).contains(&self.patch_radius)
            || !(1..=6).contains(&self.max_source_views)
            || !(1..=6).contains(&self.min_support_views)
            || self.min_support_views > self.max_source_views
            || !self.min_correlation.is_finite()
            || !(0.4..=1.).contains(&self.min_correlation)
            || !self.uniqueness_margin.is_finite()
            || !(0.0..=0.5).contains(&self.uniqueness_margin)
            || !self.relative_depth_tolerance.is_finite()
            || !(0.0001..=0.1).contains(&self.relative_depth_tolerance)
            || !self.reprojection_tolerance.is_finite()
            || !(0.1..=4.).contains(&self.reprojection_tolerance)
            || self.min_component_triangles > limits::MAX_SURFACE_TRIANGLES
        {
            return Err(crate::error("Invalid dense reconstruction options"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
pub struct DenseViewReport {
    pub image: usize,
    pub source_images: Vec<usize>,
    pub photometric_samples: usize,
    pub consistent_samples: usize,
}
#[derive(Clone, Debug, Default)]
pub struct DenseDiagnostics {
    /// Zero when volume reconstruction is not selected.
    pub volume_attempts: usize,
    /// Relative voxel spacing selected by bounded volume retries.
    pub volume_spacing_scale: Option<f64>,
    /// Hypothesis slots considered during initialization/refinement, including
    /// exact duplicate proposals whose photometric work is reused.
    pub evaluated_hypotheses: usize,
    pub evaluated_source_patches: usize,
    /// Successful bilinear source-image samples, excluding reference patch reads.
    /// u64 because valid large jobs can exceed 2^32 samples on wasm32.
    pub sampled_source_pixels: u64,
    pub estimated_maps: usize,
    pub selected_source_pairs: usize,
    pub photometric_samples: usize,
    pub consistent_samples: usize,
    pub rejected_inconsistent_samples: usize,
    /// Number of accepted observations merged into existing surface vertices.
    pub fused_samples: usize,
    pub vertices: usize,
    pub triangles: usize,
    pub view_reports: Vec<DenseViewReport>,
}
impl DenseDiagnostics {
    /// Add work from another estimator pass, without duplicating output-map or fusion counts.
    /// Validate all sums before publishing any new counters.
    pub(super) fn add_estimation_work(&mut self, other: &Self) -> Result<()> {
        let hypotheses = self
            .evaluated_hypotheses
            .checked_add(other.evaluated_hypotheses)
            .ok_or_else(|| crate::error("Depth hypothesis counter overflow"))?;
        let patches = self
            .evaluated_source_patches
            .checked_add(other.evaluated_source_patches)
            .ok_or_else(|| crate::error("Depth patch counter overflow"))?;
        let pixels = self
            .sampled_source_pixels
            .checked_add(other.sampled_source_pixels)
            .ok_or_else(|| crate::error("Depth pixel counter overflow"))?;
        self.evaluated_hypotheses = hypotheses;
        self.evaluated_source_patches = patches;
        self.sampled_source_pixels = pixels;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct DenseReconstruction {
    pub surface: Surface,
    pub diagnostics: DenseDiagnostics,
}

struct DepthMap {
    image: usize,
    width: usize,
    height: usize,
    step: f64,
    depth: Vec<f64>,
    confidence: Vec<f32>,
    neighbors: Vec<usize>,
}
// World points per pixel, computed once per map with the rotation transpose hoisted;
// each value is bit-identical to evaluating mv(tr(r), sub(scale(ray, z), t)) per point.
fn world_points(map: &DepthMap, camera: &crate::camera::Camera) -> Vec<V3> {
    let transpose = tr(camera.rotation);
    (0..map.depth.len())
        .map(|i| {
            let x = (i % map.width) as f64 * map.step;
            let y = (i / map.width) as f64 * map.step;
            mv(
                transpose,
                sub(scale(camera.ray([x, y]), map.depth[i]), camera.translation),
            )
        })
        .collect()
}
fn cancelled(
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
    stage: &str,
    n: usize,
    total: usize,
) -> Result<()> {
    if progress(stage, n, total) {
        Ok(())
    } else {
        Err(crate::error("Cancelled"))
    }
}

/// Compatibility entry point, using the default policy at the given resolution.
pub fn densify(
    images: &[Image],
    sparse: &Reconstruction,
    max_side: usize,
    progress: impl FnMut(&str, usize, usize) -> bool,
) -> Result<Surface> {
    densify_with_options(
        images,
        sparse,
        &DenseOptions {
            max_side,
            ..Default::default()
        },
        progress,
    )
    .map(|result| result.surface)
}

/// Conservative working-set planning limit, excluding caller-owned RGB images.
/// It is checked before allocation; it is not an allocator-level memory sandbox.
const MAX_WORKING_BYTES: usize = 512 * 1024 * 1024;
fn estimated_working_bytes(
    images: &[Image],
    sparse: &Reconstruction,
    options: &DenseOptions,
) -> Option<usize> {
    let mut bytes = if options.shared_volume {
        volume::WORKING_BYTES
    } else {
        0usize
    };
    for (image, camera) in images.iter().zip(&sparse.cameras) {
        if camera.is_none() {
            continue;
        }
        let gray_bytes = image.width.checked_mul(image.height)?.checked_mul(4)?;
        let step = (image.width.max(image.height) as f64 / options.max_side as f64).max(1.);
        let width = (image.width as f64 / step).floor() as usize;
        let height = (image.height as f64 / step).floor() as usize;
        // Maps, normals, patches, fusion accumulators, spatial index and up to
        // two observed triangles per pixel, with margin for collection capacity.
        let bytes_per_pixel = if options.estimator == DenseEstimator::SlantedPlane {
            640
        } else {
            576
        };
        // Per-pixel range maps and the coarse half-resolution pass.
        let bytes_per_pixel = bytes_per_pixel
            + usize::from(options.sparse_depth_prior || options.coarse_to_fine) * 96;
        let dense_bytes = width.checked_mul(height)?.checked_mul(bytes_per_pixel)?;
        bytes = bytes.checked_add(gray_bytes)?.checked_add(dense_bytes)?;
    }
    Some(bytes)
}

pub fn densify_with_options(
    images: &[Image],
    sparse: &Reconstruction,
    options: &DenseOptions,
    mut progress: impl FnMut(&str, usize, usize) -> bool,
) -> Result<DenseReconstruction> {
    options.validate()?;
    if images.len() != sparse.cameras.len() || !(2..=200).contains(&images.len()) {
        return Err(crate::error(
            "Expected 2 to 200 matching images and camera slots",
        ));
    }
    for image in images {
        image.validate()?;
    }
    for camera in sparse.cameras.iter().flatten() {
        if !camera.focal.is_finite()
            || camera.focal <= 0.
            || !camera.cx.is_finite()
            || !camera.cy.is_finite()
            || camera
                .rotation
                .iter()
                .flatten()
                .chain(camera.translation.iter())
                .any(|x| !x.is_finite())
        {
            return Err(crate::error("Invalid camera for dense reconstruction"));
        }
    }
    if estimated_working_bytes(images, sparse, options)
        .is_none_or(|bytes| bytes > MAX_WORKING_BYTES)
    {
        return Err(crate::error(
            "Dense working set exceeds 512 MiB planning budget; reduce image count or depth resolution",
        ));
    }
    cancelled(&mut progress, "depth", 0, images.len())?;
    let grayscale = estimation::prepare_grayscale(images, sparse, options, &mut progress)?;
    let (mut maps, mut diagnostics) =
        estimation::estimate_prepared(images, sparse, options, &grayscale, &mut progress)?;
    if options.dual_scale {
        let mut secondary_options = options.clone();
        secondary_options.patch_radius = 1;
        secondary_options.dual_scale = false;
        let retained = selection::retained_map_bytes(&maps, maps.capacity())?;
        selection::check_secondary_budget(
            estimated_working_bytes(images, sparse, &secondary_options),
            retained,
            MAX_WORKING_BYTES,
        )?;
        let (secondary, work) = estimation::estimate_prepared(
            images,
            sparse,
            &secondary_options,
            &grayscale,
            &mut |_, done, total| progress("depth-secondary", done, total),
        )?;
        diagnostics.add_estimation_work(&work)?;
        for map in &mut maps {
            let other = secondary
                .iter()
                .find(|m| m.image == map.image)
                .ok_or_else(|| crate::error("Missing secondary depth map"))?;
            selection::select_depth(map, other, options.relative_depth_tolerance, &mut progress)?;
        }
    }
    drop(grayscale);
    finish_densify(images, sparse, options, maps, diagnostics, progress)
}
/// Shared post-estimation tail: consistency checks, fusion and meshing.
fn finish_densify(
    images: &[Image],
    sparse: &Reconstruction,
    options: &DenseOptions,
    maps: Vec<DepthMap>,
    mut diagnostics: DenseDiagnostics,
    mut progress: impl FnMut(&str, usize, usize) -> bool,
) -> Result<DenseReconstruction> {
    diagnostics.estimated_maps = maps.len();
    diagnostics.selected_source_pairs = maps.iter().map(|m| m.neighbors.len()).sum();
    diagnostics.view_reports = maps
        .iter()
        .map(|map| DenseViewReport {
            image: map.image,
            source_images: map.neighbors.clone(),
            photometric_samples: map.depth.iter().filter(|&&z| z > 0.).count(),
            consistent_samples: 0,
        })
        .collect();
    diagnostics.photometric_samples = diagnostics
        .view_reports
        .iter()
        .map(|r| r.photometric_samples)
        .sum();
    let patches = consistency::filter(
        images,
        sparse,
        &maps,
        options,
        &mut diagnostics,
        &mut progress,
    )?;
    let surface = if options.shared_volume {
        volume::reconstruct(
            &patches,
            &maps,
            sparse,
            options,
            &mut diagnostics,
            &mut progress,
        )?
    } else {
        fusion::fuse_consolidating(
            &patches,
            options.fuse,
            options.fusion_merge_pass,
            &mut diagnostics,
            &mut progress,
        )?
    };
    let surface = if options.min_component_triangles > 0 {
        mesh::filter_small_components(&surface, options.min_component_triangles)?
    } else {
        surface
    };
    if surface.positions.len() < 20 {
        return Err(crate::error(
            "Insufficient multi-view depth agreement; only sparse reconstruction is available",
        ));
    }
    diagnostics.vertices = surface.positions.len();
    diagnostics.triangles = surface.triangles.len();
    Ok(DenseReconstruction {
        surface,
        diagnostics,
    })
}

/// Browser WebGPU sweep support, stage 1: per-view plain-data payloads for the
/// host-run NCC shader. Returns None when the options are ineligible for the
/// host sweep (slanted/volume estimators, coarse-to-fine, dual scale); callers
/// then use the regular in-kernel `densify_with_options`. The sparse
/// depth-prior ranges are computed here and stay kernel-side.
pub fn prepare_host_sweep(
    images: &[Image],
    sparse: &Reconstruction,
    options: &DenseOptions,
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<
    Option<estimation::HostSweepPreparation>,
> {
    options.validate()?;
    if options.estimator != DenseEstimator::FrontoparallelSweep
        || options.coarse_to_fine
        || options.dual_scale
        || options.shared_volume
    {
        return Ok(None);
    }
    if images.len() != sparse.cameras.len() {
        return Err(crate::error("Expected matching images and camera slots"));
    }
    estimation::prepare_host_views(images, sparse, options, progress).map(Some)
}

/// Browser WebGPU sweep support, stage 2: builds depth maps from host-computed
/// scores and runs the shared consistency/fusion/meshing tail.
pub fn densify_with_host_scores(
    images: &[Image],
    sparse: &Reconstruction,
    options: &DenseOptions,
    prepared: &[Option<estimation::PreparedView>],
    scores: &[Option<Vec<f32>>],
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<DenseReconstruction> {
    options.validate()?;
    let grayscale = estimation::prepare_grayscale(images, sparse, options, progress)?;
    let (maps, diagnostics) = estimation::finish_host_views(
        images, sparse, options, &grayscale, prepared, scores, progress,
    )?;
    finish_densify(images, sparse, options, maps, diagnostics, progress)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn depth_priors_are_opt_in_and_deterministic() {
        use fixture::{Scene, fixture};
        let (images, sparse, _) = fixture(Scene {
            angle: 30.,
            thin: true,
        });
        let options = DenseOptions {
            max_side: 64,
            patch_radius: 2,
            ..Default::default()
        };
        assert!(!options.sparse_depth_prior && !options.coarse_to_fine);
        let reference = densify_with_options(&images, &sparse, &options, |_, _, _| true).unwrap();
        // Explicitly disabled modes keep the historical output bit-identical.
        let disabled = densify_with_options(
            &images,
            &sparse,
            &DenseOptions {
                sparse_depth_prior: false,
                coarse_to_fine: false,
                ..options.clone()
            },
            |_, _, _| true,
        )
        .unwrap();
        assert_eq!(reference.surface.positions, disabled.surface.positions);
        assert_eq!(reference.surface.triangles, disabled.surface.triangles);
        // Enabled modes are deterministic across repeated runs.
        let enabled = DenseOptions {
            sparse_depth_prior: true,
            coarse_to_fine: true,
            ..options
        };
        let a = densify_with_options(&images, &sparse, &enabled, |_, _, _| true).unwrap();
        let b = densify_with_options(&images, &sparse, &enabled, |_, _, _| true).unwrap();
        assert_eq!(a.surface.positions, b.surface.positions);
        assert_eq!(a.surface.triangles, b.surface.triangles);
        // Exact anchors on this analytic fixture must not reduce map support.
        assert!(
            a.diagnostics.photometric_samples >= reference.diagnostics.photometric_samples,
            "enabled={} reference={}",
            a.diagnostics.photometric_samples,
            reference.diagnostics.photometric_samples
        );
    }
    #[test]
    fn cancellation_and_invalid_options_are_explicit() {
        let image = Image {
            width: 64,
            height: 64,
            rgb: vec![0; 64 * 64 * 3],
            focal: 100.,
        };
        let sparse = Reconstruction {
            cameras: vec![Some(image.camera()); 2],
            points: vec![],
            input_images: 2,
            reprojection_rmse: 0.,
        };
        let images = [image.clone(), image];
        assert_eq!(
            densify(&images, &sparse, 64, |_, _, _| false)
                .unwrap_err()
                .message,
            "Cancelled"
        );
        let options = DenseOptions {
            relative_depth_tolerance: f64::NAN,
            ..Default::default()
        };
        assert!(densify_with_options(&images, &sparse, &options, |_, _, _| true).is_err());
    }
    #[test]
    fn impossible_support_request_is_rejected_before_processing() {
        let options = DenseOptions {
            min_support_views: 2,
            max_source_views: 1,
            ..Default::default()
        };
        let sparse = Reconstruction {
            cameras: vec![],
            points: vec![],
            input_images: 0,
            reprojection_rmse: 0.,
        };
        let error = densify_with_options(&[], &sparse, &options, |_, _, _| {
            panic!("Invalid options must fail before work")
        })
        .unwrap_err();
        assert_eq!(error.message, "Invalid dense reconstruction options");
    }
    #[test]
    fn working_set_budget_is_checked_before_starting_depth_estimation() {
        let image = Image {
            width: 512,
            height: 512,
            rgb: vec![0; 512 * 512 * 3],
            focal: 1000.,
        };
        let sparse = Reconstruction {
            cameras: vec![Some(image.camera()); 7],
            points: vec![],
            input_images: 7,
            reprojection_rmse: 0.,
        };
        let images = vec![image; 7];
        let options = DenseOptions {
            max_side: 384,
            ..Default::default()
        };
        let error = densify_with_options(&images, &sparse, &options, |_, _, _| {
            panic!("Depth computation must not start above its planning budget")
        })
        .unwrap_err();
        assert!(error.message.contains("512 MiB"), "{error}");
    }
    #[test]
    fn narrow_images_fail_without_unsigned_underflow() {
        let image = Image {
            width: 2048,
            height: 48,
            rgb: vec![0; 2048 * 48 * 3],
            focal: 1000.,
        };
        let sparse = Reconstruction {
            cameras: vec![Some(image.camera()); 2],
            points: vec![],
            input_images: 2,
            reprojection_rmse: 0.,
        };
        assert!(densify(&[image.clone(), image], &sparse, 64, |_, _, _| true).is_err());
    }
}
