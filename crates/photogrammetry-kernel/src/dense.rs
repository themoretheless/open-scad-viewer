//! Bounded dense reconstruction: plane sweep, reciprocal depth consistency and
//! normal-aware fusion of observed samples. Experimental shared-volume extraction
//! allows bounded continuation within pixel footprints, without global hole filling.
mod consistency;
mod limits;
mod estimation;
mod selection;
mod fusion;
mod mesh;
mod plane;
mod volume;
mod simplify;
pub use simplify::{simplify, simplify_with_progress};
#[cfg(test)]
mod quality_tests;

use crate::{math::*, Image, Reconstruction, Result};
pub use mesh::compact;

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
}
impl Default for DenseOptions {
    fn default() -> Self {
        Self {
            shared_volume: false,
            dual_scale: false,
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
        }
    }
}
impl DenseOptions {
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
        {
            return Err("Invalid dense reconstruction options".into());
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
        let hypotheses = self.evaluated_hypotheses.checked_add(other.evaluated_hypotheses)
            .ok_or("Depth hypothesis counter overflow")?;
        let patches = self.evaluated_source_patches.checked_add(other.evaluated_source_patches)
            .ok_or("Depth patch counter overflow")?;
        let pixels = self.sampled_source_pixels.checked_add(other.sampled_source_pixels)
            .ok_or("Depth pixel counter overflow")?;
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
impl DepthMap {
    fn point(&self, camera: &crate::camera::Camera, index: usize) -> V3 {
        let x = (index % self.width) as f64 * self.step;
        let y = (index / self.width) as f64 * self.step;
        world(camera, x, y, self.depth[index])
    }
}
fn world(c: &crate::camera::Camera, x: f64, y: f64, z: f64) -> V3 {
    mv(tr(c.rotation), sub(scale(c.ray([x, y]), z), c.translation))
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
        Err("Cancelled".into())
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
    let mut bytes = if options.shared_volume { volume::WORKING_BYTES } else { 0usize };
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
        return Err("Expected 2 to 200 matching images and camera slots".into());
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
            return Err("Invalid camera for dense reconstruction".into());
        }
    }
    if estimated_working_bytes(images, sparse, options)
        .is_none_or(|bytes| bytes > MAX_WORKING_BYTES)
    {
        return Err("Dense working set exceeds 512 MiB planning budget; reduce image count or depth resolution".into());
    }
    cancelled(&mut progress, "depth", 0, images.len())?;
    let grayscale = estimation::prepare_grayscale(images,sparse,options,&mut progress)?;
    let (mut maps, mut diagnostics) = estimation::estimate_prepared(images, sparse, options, &grayscale, &mut progress)?;
    if options.dual_scale {
        let mut secondary_options=options.clone();
        secondary_options.patch_radius=1;
        secondary_options.dual_scale=false;
        let retained=selection::retained_map_bytes(&maps,maps.capacity())?;
        selection::check_secondary_budget(estimated_working_bytes(images,sparse,&secondary_options),retained,MAX_WORKING_BYTES)?;
        let (secondary,work)=estimation::estimate_prepared(images,sparse,&secondary_options,&grayscale,
            &mut |_,done,total| progress("depth-secondary",done,total))?;
        diagnostics.add_estimation_work(&work)?;
        for map in &mut maps {
            let other=secondary.iter().find(|m|m.image==map.image)
                .ok_or("Missing secondary depth map")?;
            selection::select_depth(map,other,options.relative_depth_tolerance,&mut progress)?;
        }
    }
    drop(grayscale);
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
        volume::reconstruct(&patches, &maps, sparse, options, &mut diagnostics, &mut progress)?
    } else {
        fusion::fuse(&patches, options.fuse, &mut diagnostics, &mut progress)?
    };
    if surface.positions.len() < 20 {
        return Err(
            "Insufficient multi-view depth agreement; only sparse reconstruction is available"
                .into(),
        );
    }
    diagnostics.vertices = surface.positions.len();
    diagnostics.triangles = surface.triangles.len();
    Ok(DenseReconstruction {
        surface,
        diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
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
            densify(&images, &sparse, 64, |_, _, _| false).unwrap_err(),
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
        assert_eq!(error, "Invalid dense reconstruction options");
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
        assert!(error.contains("512 MiB"), "{error}");
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
