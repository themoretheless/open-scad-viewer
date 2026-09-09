//! Photometric hypotheses only. Geometry checks and fusion do not mutate these maps.
use super::{cancelled, DenseDiagnostics, DenseEstimator, DenseOptions, DepthMap};
use crate::{camera::Camera, math::*, Image, Reconstruction, Result};

pub(super) struct GrayImage {
    width: usize,
    height: usize,
    values: Vec<f32>,
}
impl GrayImage {
    pub(super) fn new(image: &Image) -> Self {
        Self {
            width: image.width,
            height: image.height,
            values: image.gray(),
        }
    }
    pub(super) fn sample(&self, x: f64, y: f64) -> Option<f64> {
        if !x.is_finite()
            || !y.is_finite()
            || x < 0.
            || y < 0.
            || x >= (self.width - 1) as f64
            || y >= (self.height - 1) as f64
        {
            return None;
        }
        let (ix, iy) = (x as usize, y as usize);
        let (a, b) = (x - ix as f64, y - iy as f64);
        let i = iy * self.width + ix;
        let p = |i| self.values[i] as f64;
        Some(
            (1. - b) * ((1. - a) * p(i) + a * p(i + 1))
                + b * ((1. - a) * p(i + self.width) + a * p(i + self.width + 1)),
        )
    }
}

/// Centered patch is computed once, not again for every view and depth hypothesis.
pub(super) struct Patch {
    centered: [f64; 25],
    pub(super) len: usize,
    energy: f64,
}
impl Patch {
    #[cfg(test)]
    pub(super) fn new<const N: usize>(values: [f64; N]) -> Option<Self> {
        let mut buffer = [0.; 25];
        buffer[..N].copy_from_slice(&values);
        Self::from_values(buffer, N)
    }
    pub(super) fn from_values(values: [f64; 25], len: usize) -> Option<Self> {
        let mean = values[..len].iter().sum::<f64>() / len as f64;
        let centered = values.map(|x| x - mean);
        let energy = centered[..len].iter().map(|x| x * x).sum::<f64>();
        (energy >= 0.0001).then_some(Self {
            centered,
            energy,
            len,
        })
    }
    #[cfg(test)]
    pub(super) fn correlate<const N: usize>(&self, values: [f64; N]) -> Option<f64> {
        let mut correlation = self.accumulator();
        for (i, &value) in values[..self.len].iter().enumerate() {
            correlation.sample(i, value);
        }
        correlation.finish()
    }

    pub(super) fn accumulator(&self) -> Correlation<'_> {
        Correlation {
            patch: self,
            // Match Iterator::sum::<f64>() including its signed-zero identity.
            sum: -0.,
            squares: -0.,
            covariance: -0.,
        }
    }
}

/// Sampling already visits the patch in row order. Accumulate each original NCC
/// reduction in that same order while visiting the sampled value.
/// This removes the source scratch array and three passes over it without
/// reassociating floating-point additions or changing partial-sample counters.
pub(super) struct Correlation<'a> {
    patch: &'a Patch,
    sum: f64,
    squares: f64,
    covariance: f64,
}
impl Correlation<'_> {
    pub(super) fn sample(&mut self, index: usize, value: f64) {
        self.sum += value;
        self.squares += value * value;
        self.covariance += self.patch.centered[index] * value;
    }
    pub(super) fn finish(self) -> Option<f64> {
        let energy = self.squares - self.sum * self.sum / self.patch.len as f64;
        if energy < 0.0001 {
            return None;
        }
        Some((self.covariance / (self.patch.energy * energy).sqrt()).clamp(-1., 1.))
    }
}

pub(super) struct Source<'a> {
    pub(super) image: &'a GrayImage,
    pub(super) camera: &'a Camera,
    pub(super) rotation: M3,
    pub(super) translation: V3,
    pub(super) offsets: [V3; 25],
    pub(super) rays: [V3; 25],
}
impl Source<'_> {
    pub(super) fn prepare_pixel(&mut self, center: V3) {
        let center = mv(self.rotation, center);
        self.rays = self.offsets.map(|offset| add(center, offset));
    }
    pub(super) fn score(&self, patch: &Patch, z: f64, sampled: &mut u64) -> Option<f64> {
        let mut correlation = patch.accumulator();
        for (i, ray) in self.rays[..patch.len].iter().enumerate() {
            let p = add(scale(*ray, z), self.translation);
            if p[2] <= 0. {
                return None;
            }
            let value = self.image.sample(
                self.camera.focal * p[0] / p[2] + self.camera.cx,
                self.camera.focal * p[1] / p[2] + self.camera.cy,
            )?;
            *sampled += 1;
            correlation.sample(i, value);
        }
        correlation.finish()
    }
}

/// Co-visible points provide overlap and a useful triangulation angle. Nearly
/// coincident cameras/pure rotation are not selected just because they are close.
fn select_views(sparse: &Reconstruction, image: usize, max_sources: usize) -> Vec<usize> {
    let reference = sparse.cameras[image].as_ref().unwrap();
    let center = reference.center();
    let observed: Vec<_> = sparse
        .points
        .iter()
        .filter(|point| point.observations.iter().any(|&(view, _)| view == image))
        .collect();
    let mut candidates = Vec::new();
    for (j, camera) in sparse.cameras.iter().enumerate() {
        let Some(camera) = camera else {
            continue;
        };
        if j == image {
            continue;
        }
        let other_center = camera.center();
        let mut score = 0.;
        let mut common = 0;
        for point in &observed {
            if !point.observations.iter().any(|&(view, _)| view == j) {
                continue;
            }
            if reference.project(point.position).is_none()
                || camera.project(point.position).is_none()
            {
                continue;
            }
            let a = unit(sub(point.position, center));
            let b = unit(sub(point.position, other_center));
            let angle = norm(cross(a, b)).atan2(dot(a, b));
            // Accept 0.5 to 75 degrees; favor at least 10 degrees, penalize >50.
            if !(0.5f64.to_radians()..=75f64.to_radians()).contains(&angle) {
                continue;
            }
            score += (angle / 10f64.to_radians()).min(1.) * (50f64.to_radians() / angle).min(1.);
            common += 1;
        }
        if common >= 4 {
            candidates.push((score, common, j));
        }
    }
    candidates.sort_by(|a, b| b.0.total_cmp(&a.0).then(b.1.cmp(&a.1)).then(a.2.cmp(&b.2)));
    candidates
        .into_iter()
        .take(max_sources)
        .map(|(_, _, j)| j)
        .collect()
}

#[cfg(test)]
pub(super) fn estimate(
    images: &[Image],
    sparse: &Reconstruction,
    options: &DenseOptions,
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<(Vec<DepthMap>, DenseDiagnostics)> {
    let grayscale = prepare_grayscale(images,sparse,options,progress)?;
    estimate_prepared(images,sparse,options,&grayscale,progress)
}

pub(super) fn prepare_grayscale(
    images: &[Image],
    sparse: &Reconstruction,
    options: &DenseOptions,
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<Vec<Option<GrayImage>>> {
    let active: Vec<_> = sparse
        .cameras
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.as_ref().map(|c| (i, c)))
        .collect();
    let passes = if options.estimator == DenseEstimator::SlantedPlane {
        3
    } else {
        1
    };
    let mut grayscale = Vec::with_capacity(images.len());
    for (image, camera) in images.iter().zip(&sparse.cameras) {
        cancelled(
            progress,
            "depth",
            0,
            active.len() * options.max_side * passes,
        )?;
        grayscale.push(camera.as_ref().map(|_| GrayImage::new(image)));
    }
    Ok(grayscale)
}

pub(super) fn estimate_prepared(
    images: &[Image],
    sparse: &Reconstruction,
    options: &DenseOptions,
    grayscale: &[Option<GrayImage>],
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<(Vec<DepthMap>, DenseDiagnostics)> {
    let active: Vec<_> = sparse
        .cameras
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.as_ref().map(|c| (i, c)))
        .collect();
    let passes = if options.estimator == DenseEstimator::SlantedPlane {
        3
    } else {
        1
    };
    let mut maps = Vec::new();
    let mut diagnostics = DenseDiagnostics::default();
    for (view_number, &(index, reference)) in active.iter().enumerate() {
        cancelled(
            progress,
            "depth",
            view_number * options.max_side * passes,
            active.len() * options.max_side * passes,
        )?;
        let image = &images[index];
        let gray = grayscale[index].as_ref().unwrap();
        // Never upsample a photo solely to satisfy requested preview resolution.
        let step = (image.width.max(image.height) as f64 / options.max_side as f64).max(1.);
        let width = (image.width as f64 / step).floor() as usize;
        let height = (image.height as f64 / step).floor() as usize;
        if width < 8 || height < 8 {
            continue;
        }
        let neighbors = select_views(sparse, index, options.max_source_views);
        if neighbors.len() < options.min_support_views {
            continue;
        }
        let mut depths: Vec<_> = sparse
            .points
            .iter()
            .filter(|p| p.observations.iter().any(|&(i, _)| i == index))
            .map(|p| add(mv(reference.rotation, p.position), reference.translation)[2])
            .filter(|z| z.is_finite() && *z > 0.)
            .collect();
        depths.sort_by(f64::total_cmp);
        if depths.len() < 8 {
            continue;
        }
        let near = (depths[depths.len() / 20] * 0.85).max(0.0001);
        let far = depths[depths.len() * 19 / 20] * 1.15;
        if !far.is_finite() || far <= near {
            continue;
        }
        let mut hypotheses = [0.; 128];
        let last = options.depth_hypotheses - 1;
        for (d, z) in hypotheses[..options.depth_hypotheses]
            .iter_mut()
            .enumerate()
        {
            let f = d as f64 / last as f64;
            *z = 1. / ((1. - f) / near + f / far);
        }
        let patch_side = options.patch_radius * 2 + 1;
        let mut sources: Vec<_> = neighbors
            .iter()
            .map(|&j| {
                let camera = sparse.cameras[j].as_ref().unwrap();
                let rotation = mm(camera.rotation, tr(reference.rotation));
                let translation = sub(camera.translation, mv(rotation, reference.translation));
                Source {
                    image: grayscale[j].as_ref().unwrap(),
                    camera,
                    rotation,
                    translation,
                    offsets: std::array::from_fn(|i| {
                        mv(
                            rotation,
                            [
                                ((i % patch_side) as f64 - options.patch_radius as f64) * step
                                    / reference.focal,
                                ((i / patch_side) as f64 - options.patch_radius as f64) * step
                                    / reference.focal,
                                0.,
                            ],
                        )
                    }),
                    rays: [[0.; 3]; 25],
                }
            })
            .collect();
        if options.estimator == DenseEstimator::SlantedPlane {
            let (depth, confidence) = super::plane::estimate(
                gray,
                reference,
                &mut sources,
                width,
                height,
                step,
                near,
                far,
                options,
                &mut diagnostics,
                &mut |_, row, _| {
                    progress(
                        "depth",
                        view_number * options.max_side * 3 + row,
                        active.len() * options.max_side * 3,
                    )
                },
            )?;
            maps.push(DepthMap {
                image: index,
                width,
                height,
                step,
                depth,
                confidence,
                neighbors,
            });
            continue;
        }
        let mut depth = vec![0.; width * height];
        let mut confidence = vec![0.; width * height];
        for y in 3..height - 3 {
            cancelled(
                progress,
                "depth",
                view_number * options.max_side + y,
                active.len() * options.max_side,
            )?;
            for x in 3..width - 3 {
                let (px, py) = (x as f64 * step, y as f64 * step);
                let mut values = [0.; 25];
                for (i, value) in values[..patch_side * patch_side].iter_mut().enumerate() {
                    *value = gray
                        .sample(
                            px + ((i % patch_side) as f64 - options.patch_radius as f64) * step,
                            py + ((i / patch_side) as f64 - options.patch_radius as f64) * step,
                        )
                        .unwrap();
                }
                let Some(patch) = Patch::from_values(values, patch_side * patch_side) else {
                    continue;
                };
                let ray = reference.ray([px, py]);
                for source in &mut sources {
                    source.prepare_pixel(ray);
                }
                let mut scores = [-1.; 128];
                for (d, score) in scores[..options.depth_hypotheses].iter_mut().enumerate() {
                    diagnostics.evaluated_hypotheses += 1;
                    let mut sum = 0.;
                    let mut count = 0;
                    for source in &sources {
                        diagnostics.evaluated_source_patches += 1;
                        if let Some(ncc) = source.score(
                            &patch,
                            hypotheses[d],
                            &mut diagnostics.sampled_source_pixels,
                        ) {
                            if ncc > 0.4 {
                                sum += ncc;
                                count += 1;
                            }
                        }
                    }
                    let needed = sources.len().min(2).max(options.min_support_views);
                    if count >= needed {
                        *score = sum / count as f64;
                    }
                }
                let best = (0..options.depth_hypotheses)
                    .max_by(|&a, &b| scores[a].total_cmp(&scores[b]))
                    .unwrap();
                if scores[best] < options.min_correlation {
                    continue;
                }
                let alternative = (0..options.depth_hypotheses)
                    .filter(|&d| d.abs_diff(best) > 3)
                    .map(|d| scores[d])
                    .fold(-1., f64::max);
                if scores[best] - alternative < options.uniqueness_margin {
                    continue;
                }
                let mut offset = 0.;
                if best > 0 && best < last {
                    let (a, b, c) = (scores[best - 1], scores[best], scores[best + 1]);
                    let denom = a - 2. * b + c;
                    if a > 0. && c > 0. && denom.abs() > 1e-8 {
                        offset = (0.5 * (a - c) / denom).clamp(-0.5, 0.5);
                    }
                }
                let f = (best as f64 + offset) / last as f64;
                depth[y * width + x] = 1. / ((1. - f) / near + f / far);
                confidence[y * width + x] = scores[best] as f32;
            }
        }
        maps.push(DepthMap {
            image: index,
            width,
            height,
            step,
            depth,
            confidence,
            neighbors,
        });
    }
    Ok((maps, diagnostics))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn centered_correlation_rejects_blank_and_preserves_exposure() {
        assert!(Patch::new([1.; 9]).is_none());
        let a = [0., 0.2, 0.7, 0.3, 0.9, 0.1, 0.8, 0.5, 0.2];
        assert!(
            (Patch::new(a)
                .unwrap()
                .correlate(a.map(|x| x * 0.6 + 0.2))
                .unwrap()
                - 1.)
                .abs()
                < 1e-10
        );
    }
    #[test]
    fn sampled_correlation_is_bitwise_equal_to_separate_reductions() {
        let mut state = 0x573a_691d_eb4f_25c1u64;
        let mut random = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 11) as f64 / ((1u64 << 53) as f64)
        };
        for len in [9, 25] {
            for trial in 0..1000 {
                let reference = std::array::from_fn(|_| random());
                let mut values = std::array::from_fn::<_, 25, _>(|_| random());
                if trial % 4 == 0 {
                    // Include low-variance rejection near the numerical threshold.
                    values = values.map(|v| 0.5 + v * 0.001);
                }
                let patch = Patch::from_values(reference, len).unwrap();
                let values_slice = &values[..len];
                let sum = values_slice.iter().sum::<f64>();
                let energy =
                    values_slice.iter().map(|x| x * x).sum::<f64>() - sum * sum / len as f64;
                let expected = if energy < 0.0001 {
                    None
                } else {
                    let covariance = patch.centered[..len]
                        .iter()
                        .zip(values_slice)
                        .map(|(a, b)| a * b)
                        .sum::<f64>();
                    Some((covariance / (patch.energy * energy).sqrt()).clamp(-1., 1.))
                };
                assert_eq!(
                    patch.correlate(values).map(f64::to_bits),
                    expected.map(f64::to_bits),
                    "length={len} trial={trial}"
                );
            }
        }
    }

    #[test]
    fn source_rejection_keeps_partial_sample_count_and_u64_range() {
        let image = GrayImage::new(&Image {
            width: 16,
            height: 16,
            rgb: (0..16 * 16).flat_map(|i| [(i % 256) as u8; 3]).collect(),
            focal: 1.,
        });
        let camera = Camera::identity(1., 0., 0.);
        let patch = Patch::new([0., 0.2, 0.7, 0.3, 0.9, 0.1, 0.8, 0.5, 0.2]).unwrap();
        let mut source = Source {
            image: &image,
            camera: &camera,
            rotation: ID,
            translation: [0.; 3],
            offsets: [[0.; 3]; 25],
            rays: [[1., 1., 1.]; 25],
        };
        for invalid in [[-1., 1., 1.], [1., 1., -1.], [f64::NAN, 1., 1.]] {
            source.rays[4] = invalid;
            let mut count = u32::MAX as u64 - 2;
            assert!(source.score(&patch, 1., &mut count).is_none());
            assert_eq!(count, u32::MAX as u64 + 2);
        }
        source.rays[4] = [1., 1., 1.];
        let mut count = 0;
        assert!(source.score(&patch, 1., &mut count).is_none()); // Uniform patch.
        assert_eq!(count, 9);
    }

    #[test]
    fn source_selection_rejects_pure_rotation_and_prefers_supported_baseline() {
        let camera = Camera::identity(100., 32., 32.);
        let mut close = camera.clone();
        close.translation = [-0.0001, 0., 0.];
        let mut useful = camera.clone();
        useful.translation = [-0.5, 0., 0.];
        let sparse = Reconstruction {
            cameras: vec![
                Some(camera.clone()),
                Some(close),
                Some(useful),
                Some(camera),
            ],
            points: (0..20)
                .map(|i| crate::Point {
                    position: [i as f64 * 0.01, 0., 4.],
                    color: [0; 3],
                    observations: vec![(0, i), (1, i), (2, i), (3, i)],
                })
                .collect(),
            input_images: 4,
            reprojection_rmse: 0.,
        };
        assert_eq!(select_views(&sparse, 0, 3), vec![2]);
    }
}
