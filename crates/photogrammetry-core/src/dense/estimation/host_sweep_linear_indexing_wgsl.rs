use super::*;

pub fn host_sweep_linear_indexing_wgsl() -> String {
    format!(
        "requires linear_indexing;\n{}",
        SWEEP_WGSL.replace(SWEEP_ENTRY_PREFIX, SWEEP_LINEAR_ENTRY_PREFIX)
    )
}

pub(in super::super) struct GrayImage {
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) values: Vec<f32>,
}
impl GrayImage {
    pub(in super::super) fn new(image: &Image) -> Self {
        Self {
            width: image.width,
            height: image.height,
            values: image.gray(),
        }
    }
    pub(in super::super) fn sample(&self, x: f64, y: f64) -> Option<f64> {
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
pub(in super::super) struct Patch {
    pub(super) centered: [f64; 25],
    pub(in super::super) len: usize,
    pub(super) energy: f64,
}
impl Patch {
    #[cfg(test)]
    pub(in super::super) fn new<const N: usize>(values: [f64; N]) -> Option<Self> {
        let mut buffer = [0.; 25];
        buffer[..N].copy_from_slice(&values);
        Self::from_values(buffer, N)
    }
    pub(in super::super) fn from_values(values: [f64; 25], len: usize) -> Option<Self> {
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
    pub(in super::super) fn correlate<const N: usize>(&self, values: [f64; N]) -> Option<f64> {
        let mut correlation = self.accumulator();
        for (i, &value) in values[..self.len].iter().enumerate() {
            correlation.sample(i, value);
        }
        correlation.finish()
    }

    pub(in super::super) fn accumulator(&self) -> Correlation<'_> {
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
pub(in super::super) struct Correlation<'a> {
    patch: &'a Patch,
    sum: f64,
    squares: f64,
    covariance: f64,
}
impl Correlation<'_> {
    pub(in super::super) fn sample(&mut self, index: usize, value: f64) {
        self.sum += value;
        self.squares += value * value;
        self.covariance += self.patch.centered[index] * value;
    }
    pub(in super::super) fn finish(self) -> Option<f64> {
        let energy = self.squares - self.sum * self.sum / self.patch.len as f64;
        if energy < 0.0001 {
            return None;
        }
        Some((self.covariance / (self.patch.energy * energy).sqrt()).clamp(-1., 1.))
    }
}

pub(in super::super) struct Source<'a> {
    /// Image index of `image` in the session (GPU gray atlas lookup).
    #[cfg_attr(not(feature = "gpu"), allow(dead_code))]
    pub(in super::super) index: usize,
    pub(in super::super) image: &'a GrayImage,
    pub(in super::super) camera: &'a Camera,
    pub(in super::super) rotation: M3,
    pub(in super::super) translation: V3,
    pub(in super::super) offsets: [V3; 25],
    pub(in super::super) rays: [V3; 25],
}
impl Source<'_> {
    /// Scoring reads only rays[..len] (and plane.rs reads the center ray below
    /// len); entries beyond len keep their zero initialization.
    pub(in super::super) fn prepare_rays(&mut self, center: V3, len: usize) {
        let center = mv(self.rotation, center);
        for (ray, &offset) in self.rays[..len].iter_mut().zip(&self.offsets[..len]) {
            *ray = add(center, offset);
        }
    }
    pub(in super::super) fn score(&self, patch: &Patch, z: f64, sampled: &mut u64) -> Option<f64> {
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
pub(super) fn select_views(sparse: &Reconstruction, image: usize, max_sources: usize) -> Vec<usize> {
    let reference = sparse.cameras[image].as_ref().unwrap();
    let center = reference.center();
    let observed: Vec<_> = sparse
        .points
        .iter()
        .filter(|point| point.observations.iter().any(|&(view, _)| view == image))
        .collect();
    let min_angle = 0.5f64.to_radians();
    let max_angle = 75f64.to_radians();
    // Loose cosine bounds with small margin for floating-point safety before exact atan2 check.
    let min_cos = max_angle.cos() - 1e-6;
    let max_cos = min_angle.cos() + 1e-6;
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
            let cos_ab = dot(a, b);
            if !(min_cos..=max_cos).contains(&cos_ab) {
                continue;
            }
            let angle = norm(cross(a, b)).atan2(cos_ab);
            // Accept 0.5 to 75 degrees; favor at least 10 degrees, penalize >50.
            if !(min_angle..=max_angle).contains(&angle) {
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
pub(in super::super) fn estimate(
    images: &[Image],
    sparse: &Reconstruction,
    options: &DenseOptions,
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<(Vec<DepthMap>, DenseDiagnostics)> {
    let grayscale = prepare_grayscale(images, sparse, options, progress)?;
    estimate_prepared(images, sparse, options, &grayscale, progress)
}

pub(in super::super) fn prepare_grayscale(
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

/// Shared per-view preamble: map geometry, source selection and the sparse
/// depth range. None marks the historical skip cases (small map, too few
/// source views or anchors, degenerate range).
pub(super) struct ViewPreamble<'a> {
    pub(super) reference: &'a Camera,
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) step: f64,
    pub(super) near: f64,
    pub(super) far: f64,
    pub(super) neighbors: Vec<usize>,
}
pub(super) fn view_preamble<'a>(
    images: &[Image],
    sparse: &'a Reconstruction,
    options: &DenseOptions,
    index: usize,
    reference: &'a Camera,
) -> Option<ViewPreamble<'a>> {
    let image = &images[index];
    // Never upsample a photo solely to satisfy requested preview resolution.
    let step = (image.width.max(image.height) as f64 / options.max_side as f64).max(1.);
    let width = (image.width as f64 / step).floor() as usize;
    let height = (image.height as f64 / step).floor() as usize;
    if width < 8 || height < 8 {
        return None;
    }
    let neighbors = select_views(sparse, index, options.max_source_views);
    if neighbors.len() < options.min_support_views {
        return None;
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
        return None;
    }
    let near = (depths[depths.len() / 20] * 0.85).max(0.0001);
    let far = depths[depths.len() * 19 / 20] * 1.15;
    if !far.is_finite() || far <= near {
        return None;
    }
    Some(ViewPreamble {
        reference,
        width,
        height,
        step,
        near,
        far,
        neighbors,
    })
}

/// Rotation/translation carrying a reference-frame point into this source
/// camera: R_source * R_reference^T and t_source - R * t_reference.
pub(super) fn source_transform(reference: &Camera, camera: &Camera) -> (M3, V3) {
    let rotation = mm(camera.rotation, tr(reference.rotation));
    let translation = sub(camera.translation, mv(rotation, reference.translation));
    (rotation, translation)
}

/// One source view ready to score patches: rotation/translation into the
/// reference frame and the per-tap ray offsets for this step size.
pub(super) fn build_view_sources<'a>(
    sparse: &'a Reconstruction,
    grayscale: &'a [Option<GrayImage>],
    preamble: &ViewPreamble<'a>,
    patch_radius: usize,
    source_step: f64,
) -> Vec<Source<'a>> {
    let reference = preamble.reference;
    let patch_side = patch_radius * 2 + 1;
    preamble
        .neighbors
        .iter()
        .map(|&j| {
            let camera = sparse.cameras[j].as_ref().unwrap();
            let (rotation, translation) = source_transform(reference, camera);
            Source {
                index: j,
                image: grayscale[j].as_ref().unwrap(),
                camera,
                rotation,
                translation,
                offsets: std::array::from_fn(|i| {
                    mv(
                        rotation,
                        [
                            ((i % patch_side) as f64 - patch_radius as f64) * source_step
                                / reference.focal,
                            ((i / patch_side) as f64 - patch_radius as f64) * source_step
                                / reference.focal,
                            0.,
                        ],
                    )
                }),
                rays: [[0.; 3]; 25],
            }
        })
        .collect()
}

/// Inverse-depth hypothesis grid shared by every sweep flavor.
pub(super) fn hypothesis_grid(near: f64, far: f64, count: usize) -> [f64; 128] {
    let mut hypotheses = [0.; 128];
    let last = count - 1;
    for (d, z) in hypotheses[..count].iter_mut().enumerate() {
        let f = d as f64 / last as f64;
        *z = 1. / ((1. - f) / near + f / far);
    }
    hypotheses
}
