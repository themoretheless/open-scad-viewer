//! Photometric hypotheses only. Geometry checks and fusion do not mutate these maps.
use super::{cancelled, DenseDiagnostics, DenseEstimator, DenseOptions, DepthMap};
use crate::{camera::Camera, math::*, Image, Reconstruction, Result};

/// The NCC sweep compute shader (WGSL), shared by the native `gpu` feature and
/// the browser WebGPU host path; both must execute the identical text.
pub const SWEEP_WGSL: &str = r#"
struct Params {
    width: u32,
    height: u32,
    n_hyp: u32,
    n_src: u32,
    plen: u32,
    radius: u32,
    needed: u32,
    ref_w: u32,
    ref_h: u32,
    ref_goff: u32,
    _p1: u32,
    _p2: u32,
    step: f32,
    ref_f: f32,
    ref_cx: f32,
    ref_cy: f32,
}
@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> hyps: array<f32>;
@group(0) @binding(2) var<storage, read> srcf: array<f32>;
@group(0) @binding(3) var<storage, read> srcm: array<u32>;
@group(0) @binding(4) var<storage, read> grays: array<f32>;
@group(0) @binding(5) var<storage, read_write> scores: array<f32>;

// Gray levels live in [0, 1]; -1 marks an invalid (out-of-frame) sample,
// matching the CPU's Option-returning bilinear sampler.
fn sample_gray(offset: u32, w: u32, h: u32, x: f32, y: f32) -> f32 {
    if (!(x >= 0.0) || !(y >= 0.0) || x >= f32(w) - 1.0 || y >= f32(h) - 1.0) {
        return -1.0;
    }
    let ix = u32(x);
    let iy = u32(y);
    let a = x - f32(ix);
    let b = y - f32(iy);
    let i = offset + iy * w + ix;
    let p00 = grays[i];
    let p10 = grays[i + 1u];
    let p01 = grays[i + w];
    let p11 = grays[i + w + 1u];
    return (1.0 - b) * ((1.0 - a) * p00 + a * p10) + b * ((1.0 - a) * p01 + a * p11);
}

@compute @workgroup_size(16, 16)
fn sweep(@builtin(global_invocation_id) id: vec3<u32>) {
    let x = id.x;
    let y = id.y;
    if (x >= params.width || y >= params.height) {
        return;
    }
    let base = (y * params.width + x) * params.n_hyp;
    if (x < 3u || y < 3u || x + 3u >= params.width || y + 3u >= params.height) {
        for (var d = 0u; d < params.n_hyp; d++) {
            scores[base + d] = -1.0;
        }
        return;
    }
    let px = f32(x) * params.step;
    let py = f32(y) * params.step;
    let ps = params.radius * 2u + 1u;
    var refv: array<f32, 25>;
    for (var i = 0u; i < params.plen; i++) {
        let ox = f32(i % ps) - f32(params.radius);
        let oy = f32(i / ps) - f32(params.radius);
        // Interior pixels keep the reference patch fully in frame (CPU invariant).
        refv[i] = sample_gray(params.ref_goff, params.ref_w, params.ref_h, px + ox * params.step, py + oy * params.step);
    }
    var mean = 0.0;
    for (var i = 0u; i < params.plen; i++) {
        mean += refv[i];
    }
    mean = mean / f32(params.plen);
    var centered: array<f32, 25>;
    var energy = 0.0;
    for (var i = 0u; i < params.plen; i++) {
        centered[i] = refv[i] - mean;
        energy += centered[i] * centered[i];
    }
    if (energy < 0.0001) {
        for (var d = 0u; d < params.n_hyp; d++) {
            scores[base + d] = -1.0;
        }
        return;
    }
    for (var d = 0u; d < params.n_hyp; d++) {
        let z = hyps[d];
        var sum = 0.0;
        var count = 0u;
        for (var s = 0u; s < params.n_src; s++) {
            let fb = s * 16u;
            let mb = s * 4u;
            let goff = srcm[mb];
            let gw = srcm[mb + 1u];
            let gh = srcm[mb + 2u];
            let focal = srcf[fb + 12u];
            let cx = srcf[fb + 13u];
            let cy = srcf[fb + 14u];
            var ssum = 0.0;
            var sq = 0.0;
            var cov = 0.0;
            var ok = true;
            for (var i = 0u; i < params.plen; i++) {
                let ox = (f32(i % ps) - f32(params.radius)) * params.step;
                let oy = (f32(i / ps) - f32(params.radius)) * params.step;
                let rx = (px - params.ref_cx + ox) / params.ref_f;
                let ry = (py - params.ref_cy + oy) / params.ref_f;
                let vx = srcf[fb] * rx + srcf[fb + 1u] * ry + srcf[fb + 2u];
                let vy = srcf[fb + 3u] * rx + srcf[fb + 4u] * ry + srcf[fb + 5u];
                let vz = srcf[fb + 6u] * rx + srcf[fb + 7u] * ry + srcf[fb + 8u];
                let wx = vx * z + srcf[fb + 9u];
                let wy = vy * z + srcf[fb + 10u];
                let wz = vz * z + srcf[fb + 11u];
                if (wz <= 0.0) {
                    ok = false;
                    break;
                }
                let value = sample_gray(goff, gw, gh, focal * wx / wz + cx, focal * wy / wz + cy);
                if (value < 0.0) {
                    ok = false;
                    break;
                }
                ssum += value;
                sq += value * value;
                cov += centered[i] * value;
            }
            if (!ok) {
                continue;
            }
            let energy_s = sq - ssum * ssum / f32(params.plen);
            if (energy_s < 0.0001) {
                continue;
            }
            let ncc = clamp(cov / sqrt(energy * energy_s), -1.0, 1.0);
            if (ncc > 0.4) {
                sum += ncc;
                count++;
            }
        }
        scores[base + d] = select(-1.0, sum / f32(count), count >= params.needed);
    }
}
"#;

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
        self.prepare_rays(center, 25);
    }
    /// Scoring reads only rays[..len] (and plane.rs reads the center ray below
    /// len); entries beyond len keep their zero initialization.
    pub(super) fn prepare_rays(&mut self, center: V3, len: usize) {
        let center = mv(self.rotation, center);
        for (ray, &offset) in self.rays[..len].iter_mut().zip(&self.offsets[..len]) {
            *ray = add(center, offset);
        }
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


/// Shared per-view preamble: map geometry, source selection and the sparse
/// depth range. None marks the historical skip cases (small map, too few
/// source views or anchors, degenerate range).
struct ViewPreamble<'a> {
    reference: &'a Camera,
    width: usize,
    height: usize,
    step: f64,
    near: f64,
    far: f64,
    neighbors: Vec<usize>,
}
fn view_preamble<'a>(
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
    Some(ViewPreamble { reference, width, height, step, near, far, neighbors })
}

/// One source view ready to score patches: rotation/translation into the
/// reference frame and the per-tap ray offsets for this step size.
fn build_view_sources<'a>(
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
                            ((i % patch_side) as f64 - patch_radius as f64)
                                * source_step
                                / reference.focal,
                            ((i / patch_side) as f64 - patch_radius as f64)
                                * source_step
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
fn hypothesis_grid(near: f64, far: f64, count: usize) -> [f64; 128] {
    let mut hypotheses = [0.; 128];
    let last = count - 1;
    for (d, z) in hypotheses[..count].iter_mut().enumerate() {
        let f = d as f64 / last as f64;
        *z = 1. / ((1. - f) / near + f / far);
    }
    hypotheses
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
        let Some(preamble) = view_preamble(images, sparse, options, index, reference) else {
            continue;
        };
        let gray = grayscale[index].as_ref().unwrap();
        let (width, height, step, near, far) =
            (preamble.width, preamble.height, preamble.step, preamble.near, preamble.far);
        let neighbors = &preamble.neighbors;
        let build_sources =
            |source_step: f64| build_view_sources(sparse, grayscale, &preamble, options.patch_radius, source_step);
        let mut sources = build_sources(step);
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
                neighbors: neighbors.clone(),
            });
            continue;
        }
        // Opt-in per-pixel sweep ranges: a coarse half-resolution pass fills
        // the map, then sparse anchors override the pixels they cover.
        let mut ranges: Option<(Vec<f64>, Vec<f64>)> = None;
        if options.coarse_to_fine && width >= 16 && height >= 16 {
            let coarse_width = width / 2;
            let coarse_height = height / 2;
            let coarse_step = step * 2.;
            let coarse_prior = if options.sparse_depth_prior {
                sparse_intervals(
                    sparse,
                    index,
                    reference,
                    coarse_width,
                    coarse_height,
                    coarse_step,
                    near,
                    far,
                )
            } else {
                None
            };
            let mut coarse_sources = build_sources(coarse_step);
            let (coarse_depth, _) = sweep_depth(
                gray,
                reference,
                &mut coarse_sources,
                coarse_width,
                coarse_height,
                coarse_step,
                near,
                far,
                coarse_prior.as_ref(),
                options,
                &mut diagnostics,
                &mut |row| {
                    cancelled(
                        progress,
                        "depth",
                        view_number * options.max_side + row,
                        active.len() * options.max_side,
                    )
                },
            )?;
            let (lo, hi) = coarse_intervals(
                &coarse_depth,
                coarse_width,
                width,
                height,
                near,
                far,
                options.depth_hypotheses,
            );
            ranges = Some((lo, hi));
        }
        if options.sparse_depth_prior {
            let sparse_ranges =
                sparse_intervals(sparse, index, reference, width, height, step, near, far);
            match (&mut ranges, sparse_ranges) {
                (Some((lo, hi)), Some((sparse_lo, sparse_hi))) => {
                    for i in 0..lo.len() {
                        if sparse_lo[i] > 0. {
                            lo[i] = sparse_lo[i];
                            hi[i] = sparse_hi[i];
                        }
                    }
                }
                (None, sparse) => ranges = sparse,
                (Some(_), None) => {}
            }
        }
        let (depth, confidence) = sweep_depth(
            gray,
            reference,
            &mut sources,
            width,
            height,
            step,
            near,
            far,
            ranges.as_ref(),
            options,
            &mut diagnostics,
            &mut |row| {
                cancelled(
                    progress,
                    "depth",
                    view_number * options.max_side + row,
                    active.len() * options.max_side,
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
            neighbors: neighbors.clone(),
        });
    }
    Ok((maps, diagnostics))
}

/// Plain-data sweep payload of one view for host execution (browser WebGPU).
/// Grayscale rasters are not duplicated here; the host reads them per image
/// through `HostSweepView::ref_image` / `HostSweepSource::image`.
pub struct HostSweepView {
    pub patch_radius: usize,
    pub ref_image: usize,
    pub map_width: usize,
    pub map_height: usize,
    pub step: f64,
    pub needed: usize,
    pub ref_focal: f64,
    pub ref_cx: f64,
    pub ref_cy: f64,
    pub hypotheses: Vec<f64>,
    pub sources: Vec<HostSweepSource>,
}
pub struct HostSweepSource {
    pub image: usize,
    pub rotation: [[f64; 3]; 3],
    pub translation: [f64; 3],
    pub focal: f64,
    pub cx: f64,
    pub cy: f64,
}

/// Kernel-side bookkeeping retained between `prepare_host_views` and
/// `finish_host_views`; opaque to callers, which only store and return it.
pub struct PreparedView {
    width: usize,
    height: usize,
    step: f64,
    near: f64,
    far: f64,
    ranges: Option<SweepRanges>,
    neighbors: Vec<usize>,
}

impl PreparedView {
    /// Score-buffer length per hypothesis count: map pixels (all, including
    /// boundary rows that the shader fills with -1).
    pub fn map_area(&self) -> usize {
        self.width * self.height
    }
}

/// Per-view payloads for the host NCC sweep, indexed by image. Frontoparallel
/// only; `dense::prepare_host_sweep` rejects ineligible option sets first.
pub fn prepare_host_views(
    images: &[Image],
    sparse: &Reconstruction,
    options: &DenseOptions,
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<(Vec<Option<HostSweepView>>, Vec<Option<PreparedView>>)> {
    let active: Vec<_> = sparse
        .cameras
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.as_ref().map(|c| (i, c)))
        .collect();
    let mut views: Vec<Option<HostSweepView>> = (0..images.len()).map(|_| None).collect();
    let mut prepared: Vec<Option<PreparedView>> = (0..images.len()).map(|_| None).collect();
    for (view_number, &(index, reference)) in active.iter().enumerate() {
        cancelled(progress, "depth", view_number * options.max_side, active.len() * options.max_side)?;
        let Some(preamble) = view_preamble(images, sparse, options, index, reference) else {
            continue;
        };
        let hypotheses = hypothesis_grid(preamble.near, preamble.far, options.depth_hypotheses);
        let needed = preamble.neighbors.len().min(2).max(options.min_support_views);
        let ranges = if options.sparse_depth_prior {
            sparse_intervals(
                sparse,
                index,
                reference,
                preamble.width,
                preamble.height,
                preamble.step,
                preamble.near,
                preamble.far,
            )
        } else {
            None
        };
        let sources = preamble
            .neighbors
            .iter()
            .map(|&j| {
                let camera = sparse.cameras[j].as_ref().unwrap();
                let rotation = mm(camera.rotation, tr(reference.rotation));
                let translation = sub(camera.translation, mv(rotation, reference.translation));
                HostSweepSource {
                    image: j,
                    rotation,
                    translation,
                    focal: camera.focal,
                    cx: camera.cx,
                    cy: camera.cy,
                }
            })
            .collect();
        views[index] = Some(HostSweepView {
            patch_radius: options.patch_radius,
            ref_image: index,
            map_width: preamble.width,
            map_height: preamble.height,
            step: preamble.step,
            needed,
            ref_focal: reference.focal,
            ref_cx: reference.cx,
            ref_cy: reference.cy,
            hypotheses: hypotheses[..options.depth_hypotheses].to_vec(),
            sources,
        });
        prepared[index] = Some(PreparedView {
            width: preamble.width,
            height: preamble.height,
            step: preamble.step,
            near: preamble.near,
            far: preamble.far,
            ranges,
            neighbors: preamble.neighbors,
        });
    }
    Ok((views, prepared))
}

/// Builds depth maps from host-computed score rows (indexed by image, aligned
/// with `prepare_host_views` output). Scores are f32 from the GPU shader and
/// widened exactly to f64 for the shared CPU selection.
pub fn finish_host_views(
    images: &[Image],
    sparse: &Reconstruction,
    options: &DenseOptions,
    grayscale: &[Option<GrayImage>],
    prepared: &[Option<PreparedView>],
    scores: &[Option<Vec<f32>>],
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<(Vec<DepthMap>, DenseDiagnostics)> {
    if prepared.len() != images.len() || scores.len() != images.len() {
        return Err("Host sweep state does not match the session images".into());
    }
    let active: Vec<_> = sparse
        .cameras
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.as_ref().map(|c| (i, c)))
        .collect();
    let mut maps = Vec::new();
    let mut diagnostics = DenseDiagnostics::default();
    let patch_side = options.patch_radius * 2 + 1;
    for (view_number, &(index, reference)) in active.iter().enumerate() {
        cancelled(progress, "depth", view_number * options.max_side, active.len() * options.max_side)?;
        let (Some(prep), Some(view_scores)) = (&prepared[index], &scores[index]) else {
            continue;
        };
        let expected = prep.width * prep.height * options.depth_hypotheses;
        if view_scores.len() != expected {
            return Err(format!(
                "Host sweep scores for image {index} have {}, expected {expected} values",
                view_scores.len()
            ));
        }
        let sources = build_view_sources(sparse, grayscale, &ViewPreamble {
            reference,
            width: prep.width,
            height: prep.height,
            step: prep.step,
            near: prep.near,
            far: prep.far,
            neighbors: prep.neighbors.clone(),
        }, options.patch_radius, prep.step);
        // The host evaluated every hypothesis/source/tap without early exits;
        // counters report that full work, as in the native GPU mode.
        let pixels = (prep.width - 6) * (prep.height - 6);
        diagnostics.evaluated_hypotheses += pixels * options.depth_hypotheses;
        diagnostics.evaluated_source_patches += pixels * options.depth_hypotheses * sources.len();
        diagnostics.sampled_source_pixels +=
            (pixels * options.depth_hypotheses * sources.len() * patch_side * patch_side) as u64;
        let inverse_span = 1. / prep.near - 1. / prep.far;
        let (depth, confidence) = select_from_scores(
            view_scores,
            prep.width,
            prep.height,
            prep.ranges.as_ref(),
            prep.near,
            prep.far,
            inverse_span,
            options,
            &mut |row| {
                cancelled(
                    progress,
                    "depth",
                    view_number * options.max_side + row,
                    active.len() * options.max_side,
                )
            },
        )?;
        let _ = sources;
        maps.push(DepthMap {
            image: index,
            width: prep.width,
            height: prep.height,
            step: prep.step,
            depth,
            confidence,
            neighbors: prep.neighbors.clone(),
        });
    }
    Ok((maps, diagnostics))
}

/// Per-pixel sweep range as `(near, far)` map arrays; zeroed entries mean the
/// pixel keeps the global range.
type SweepRanges = (Vec<f64>, Vec<f64>);

/// Window radius in map pixels for sparse-anchor aggregation; six covers the
/// typical sparse grid spacing without blending distant surfaces.
const PRIOR_WINDOW: usize = 6;

/// Deterministic per-pixel depth intervals from sparse anchors observed in the
/// reference view (COLMAP-style geometric prior). Anchors are splatted to the
/// map grid, then each pixel takes the padded min/max depth of anchors within
/// a fixed square window. Returns None when the view has no usable anchors.
fn sparse_intervals(
    sparse: &Reconstruction,
    image: usize,
    reference: &Camera,
    width: usize,
    height: usize,
    step: f64,
    near: f64,
    far: f64,
) -> Option<SweepRanges> {
    let mut cell_lo = vec![f64::INFINITY; width * height];
    let mut cell_hi = vec![0f64; width * height];
    let mut any = false;
    for point in &sparse.points {
        if !point.observations.iter().any(|&(view, _)| view == image) {
            continue;
        }
        let z = add(mv(reference.rotation, point.position), reference.translation)[2];
        if !z.is_finite() || z <= 0. {
            continue;
        }
        let Some(uv) = reference.project(point.position) else {
            continue;
        };
        if uv[0] < 0. || uv[1] < 0. {
            continue;
        }
        let (gx, gy) = ((uv[0] / step) as usize, (uv[1] / step) as usize);
        if gx >= width || gy >= height {
            continue;
        }
        let i = gy * width + gx;
        cell_lo[i] = cell_lo[i].min(z);
        cell_hi[i] = cell_hi[i].max(z);
        any = true;
    }
    if !any {
        return None;
    }
    let mut lo = vec![0.; width * height];
    let mut hi = vec![0.; width * height];
    for y in 0..height {
        for x in 0..width {
            let mut l = f64::INFINITY;
            let mut h = 0f64;
            for wy in y.saturating_sub(PRIOR_WINDOW)..=(y + PRIOR_WINDOW).min(height - 1) {
                for wx in x.saturating_sub(PRIOR_WINDOW)..=(x + PRIOR_WINDOW).min(width - 1) {
                    let i = wy * width + wx;
                    if cell_hi[i] > 0. {
                        l = l.min(cell_lo[i]);
                        h = h.max(cell_hi[i]);
                    }
                }
            }
            if h > 0. {
                let a = (l * 0.95).max(near);
                let b = (h * 1.05).min(far);
                if b > a {
                    lo[y * width + x] = a;
                    hi[y * width + x] = b;
                }
            }
        }
    }
    Some((lo, hi))
}

/// Per-pixel refinement intervals around a half-resolution depth map: four
/// coarse hypothesis steps of inverse depth on either side of the lifted
/// depth, clamped to the global range.
fn coarse_intervals(
    coarse_depth: &[f64],
    coarse_width: usize,
    width: usize,
    height: usize,
    near: f64,
    far: f64,
    hypotheses: usize,
) -> SweepRanges {
    let spacing = (1. / near - 1. / far) / (hypotheses - 1) as f64;
    let mut lo = vec![0.; width * height];
    let mut hi = vec![0.; width * height];
    for y in 0..height {
        for x in 0..width {
            let z = coarse_depth[(y / 2) * coarse_width + x / 2];
            if z <= 0. {
                continue;
            }
            let w = 1. / z;
            let w_lo = (w - 4. * spacing).max(1. / far);
            let w_hi = (w + 4. * spacing).min(1. / near);
            lo[y * width + x] = 1. / w_hi;
            hi[y * width + x] = 1. / w_lo;
        }
    }
    (lo, hi)
}


/// Per-pixel selection over a host/GPU-computed score map (f32 widened to f64,
/// exact widening), shared by the native GPU sweep and the browser host path.
#[allow(clippy::too_many_arguments)]
fn select_from_scores(
    scores_map: &[f32],
    width: usize,
    height: usize,
    ranges: Option<&SweepRanges>,
    near: f64,
    far: f64,
    inverse_span: f64,
    options: &DenseOptions,
    row_progress: &mut impl FnMut(usize) -> Result<()>,
) -> Result<(Vec<f64>, Vec<f32>)> {
    let mut depth = vec![0.; width * height];
    let mut confidence = vec![0.; width * height];
    for y in 3..height - 3 {
        row_progress(y)?;
        for x in 3..width - 3 {
            let pixel = y * width + x;
            let start = pixel * options.depth_hypotheses;
            let row = &scores_map[start..start + options.depth_hypotheses];
            let mut scores = [0.; 128];
            for (d, &v) in row.iter().enumerate() {
                scores[d] = v as f64;
            }
            if let Some((d, c)) = pick_depth(
                &scores[..options.depth_hypotheses],
                ranges,
                pixel,
                near,
                far,
                inverse_span,
                options,
            ) {
                depth[pixel] = d;
                confidence[pixel] = c;
            }
        }
    }
    Ok((depth, confidence))
}

/// Frontoparallel NCC sweep over inverse-depth hypotheses. Optional per-pixel
/// ranges restrict peak selection to the geometrically plausible bins and
/// exclude those bins from the uniqueness alternatives; pixels without a range
/// keep the historical full-range behavior bit-exactly.
///
/// Hypothesis selection shared by the CPU and GPU sweeps; takes the per-pixel
/// score row and applies the range clamp, uniqueness margin and the parabolic
/// subpixel offset. Returns (depth, confidence).
#[allow(clippy::too_many_arguments)]
fn pick_depth(
    scores: &[f64],
    ranges: Option<&SweepRanges>,
    pixel: usize,
    near: f64,
    far: f64,
    inverse_span: f64,
    options: &DenseOptions,
) -> Option<(f64, f32)> {
    let last = options.depth_hypotheses - 1;
    // Map the depth interval onto the global hypothesis grid. Intervals
    // wider than 15% relative depth can straddle a depth edge, so they
    // are not trusted and the pixel keeps full-range behavior.
    let (bin_lo, bin_hi) = match ranges {
        Some((lo, hi)) if lo[pixel] > 0. => {
            let (range_lo, range_hi) = (lo[pixel], hi[pixel]);
            if range_hi > range_lo * 1.15 {
                (0, last)
            } else {
                let bin_of = |z: f64| (1. / near - 1. / z) / inverse_span * last as f64;
                let lo_bin = bin_of(range_lo).floor().max(0.) as usize;
                let hi_bin = (bin_of(range_hi).ceil() as usize).min(last);
                (lo_bin, hi_bin.max(lo_bin))
            }
        }
        _ => (0, last),
    };
    // With a per-pixel range the prior vouches for everything inside
    // it; ambiguity only matters at competing depths outside. If the
    // in-range peak is implausible, the range was wrong for this pixel:
    // fall back to the historical full-range selection.
    let ranged = bin_lo > 0 || bin_hi < last;
    let best_ranged = (bin_lo..=bin_hi).max_by(|&a, &b| scores[a].total_cmp(&scores[b])).unwrap();
    let (best, alternative) = if ranged && scores[best_ranged] >= options.min_correlation {
        let alternative = (0..options.depth_hypotheses)
            .filter(|&d| d + 3 < bin_lo || d > bin_hi + 3)
            .map(|d| scores[d])
            .fold(-1., f64::max);
        (best_ranged, alternative)
    } else {
        let best = (0..options.depth_hypotheses)
            .max_by(|&a, &b| scores[a].total_cmp(&scores[b]))
            .unwrap();
        let alternative = (0..options.depth_hypotheses)
            .filter(|&d| d.abs_diff(best) > 3)
            .map(|d| scores[d])
            .fold(-1., f64::max);
        (best, alternative)
    };
    if scores[best] < options.min_correlation {
        return None;
    }
    if scores[best] - alternative < options.uniqueness_margin {
        return None;
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
    Some((1. / ((1. - f) / near + f / far), scores[best] as f32))
}

/// Packs the prepared CPU-side view for the GPU sweep and returns the per-pixel
/// hypothesis scores; None without a GPU adapter (the caller falls back).
#[cfg(feature = "gpu")]
#[allow(clippy::too_many_arguments)]
fn gpu_sweep_scores(
    gray: &GrayImage,
    reference: &Camera,
    sources: &[Source],
    width: usize,
    height: usize,
    step: f64,
    hypotheses: &[f64],
    options: &DenseOptions,
) -> Option<Vec<f32>> {
    let needed = sources.len().min(2).max(options.min_support_views);
    let mut grays = Vec::new();
    let mut payloads = Vec::with_capacity(sources.len());
    let ref_len = gray.values.len();
    for source in sources {
        // Offset 0 of the shared gray buffer is the reference image.
        let gray_offset = (ref_len + grays.len()) as u32;
        grays.extend_from_slice(&source.image.values);
        payloads.push(crate::gpu::sweep::SourcePayload {
            rotation: source.rotation.map(|row| row.map(|v| v as f32)),
            translation: source.translation.map(|v| v as f32),
            focal: source.camera.focal as f32,
            cx: source.camera.cx as f32,
            cy: source.camera.cy as f32,
            gray_offset,
            gray_width: source.image.width as u32,
            gray_height: source.image.height as u32,
        });
    }
    let hypotheses32: Vec<f32> = hypotheses.iter().map(|&z| z as f32).collect();
    crate::gpu::sweep::sweep_view(
        &gray.values,
        gray.width,
        gray.height,
        reference.focal,
        reference.cx,
        reference.cy,
        &payloads,
        &grays,
        &hypotheses32,
        width,
        height,
        options.patch_radius,
        step,
        needed,
    )
}

#[allow(clippy::too_many_arguments)]
fn sweep_depth(
    gray: &GrayImage,
    reference: &Camera,
    sources: &mut [Source],
    width: usize,
    height: usize,
    step: f64,
    near: f64,
    far: f64,
    ranges: Option<&SweepRanges>,
    options: &DenseOptions,
    diagnostics: &mut DenseDiagnostics,
    row_progress: &mut impl FnMut(usize) -> Result<()>,
) -> Result<(Vec<f64>, Vec<f32>)> {
    let hypotheses = hypothesis_grid(near, far, options.depth_hypotheses);
    let inverse_span = 1. / near - 1. / far;
    let patch_side = options.patch_radius * 2 + 1;
    let mut depth = vec![0.; width * height];
    let mut confidence = vec![0.; width * height];
    #[cfg(feature = "gpu")]
    if options.acceleration == crate::Acceleration::Gpu {
        if let Some(scores_map) = gpu_sweep_scores(
            gray,
            reference,
            sources,
            width,
            height,
            step,
            &hypotheses[..options.depth_hypotheses],
            options,
        ) {
            // The GPU evaluates every hypothesis/source/tap without early
            // exits; counters report that full work (f32 arithmetic, see
            // DenseOptions::acceleration).
            let pixels = (width - 6) * (height - 6);
            let taps = patch_side * patch_side;
            diagnostics.evaluated_hypotheses += pixels * options.depth_hypotheses;
            diagnostics.evaluated_source_patches +=
                pixels * options.depth_hypotheses * sources.len();
            diagnostics.sampled_source_pixels +=
                (pixels * options.depth_hypotheses * sources.len() * taps) as u64;
            return select_from_scores(
                &scores_map,
                width,
                height,
                ranges,
                near,
                far,
                inverse_span,
                options,
                row_progress,
            );
        }
    }
    for y in 3..height - 3 {
        row_progress(y)?;
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
            for source in sources.iter_mut() {
                source.prepare_rays(ray, patch.len);
            }
            let mut scores = [-1.; 128];
            let needed = sources.len().min(2).max(options.min_support_views);
            for (d, score) in scores[..options.depth_hypotheses].iter_mut().enumerate() {
                diagnostics.evaluated_hypotheses += 1;
                let mut sum = 0.;
                let mut count = 0;
                for (s, source) in sources.iter().enumerate() {
                    // The remaining sources cannot lift count to needed;
                    // the hypothesis score stays -1, so stop scoring early.
                    if count + (sources.len() - s) < needed {
                        break;
                    }
                    diagnostics.evaluated_source_patches += 1;
                    if let Some(ncc) =
                        source.score(&patch, hypotheses[d], &mut diagnostics.sampled_source_pixels)
                    {
                        if ncc > 0.4 {
                            sum += ncc;
                            count += 1;
                        }
                    }
                }
                if count >= needed {
                    *score = sum / count as f64;
                }
            }
            if let Some((d, c)) = pick_depth(
                &scores[..options.depth_hypotheses],
                ranges,
                y * width + x,
                near,
                far,
                inverse_span,
                options,
            ) {
                depth[y * width + x] = d;
                confidence[y * width + x] = c;
            }
        }
    }
    Ok((depth, confidence))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "gpu")]
    #[test]
    fn gpu_sweep_matches_cpu_on_shifted_plane() {
        // Reference texture 80x80; a source camera translated -0.5 along x sees
        // the plane z=4 shifted left by f*B/z = 80*0.5/4 = 10 px.
        let mut rng = 0xABCDEFu64;
        let mut tex = vec![0f32; 80 * 80];
        for v in &mut tex {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            *v = (rng >> 33) as f32 / 2147483648.0;
        }
        let mut src_values = vec![0f32; 80 * 80];
        for y in 0..80 {
            for x in 0..70 {
                src_values[y * 80 + x] = tex[y * 80 + x + 10];
            }
        }
        let ref_image = Image { width: 80, height: 80, rgb: vec![], focal: 80. };
        let _ = &ref_image;
        let reference = Camera::identity(80., 40., 40.);
        let source_camera = Camera {
            rotation: ID,
            translation: [-0.5, 0., 0.],
            focal: 80.,
            cx: 40.,
            cy: 40.,
        };
        let ref_gray = GrayImage { width: 80, height: 80, values: tex };
        let src_gray = GrayImage { width: 80, height: 80, values: src_values };
        let build = |step: f64| {
            vec![Source {
                image: &src_gray,
                camera: &source_camera,
                rotation: ID,
                translation: [-0.5, 0., 0.],
                offsets: std::array::from_fn(|i| {
                    [
                        ((i % 3) as f64 - 1.) * step / 80.,
                        ((i / 3) as f64 - 1.) * step / 80.,
                        0.,
                    ]
                }),
                rays: [[0.; 3]; 25],
            }]
        };
        let options = |acceleration| DenseOptions {
            acceleration,
            depth_hypotheses: 16,
            patch_radius: 1,
            min_support_views: 1,
            ..Default::default()
        };
        let run = |acceleration| {
            let mut sources = build(2.);
            let mut diagnostics = DenseDiagnostics::default();
            sweep_depth(
                &ref_gray,
                &reference,
                &mut sources,
                40,
                40,
                2.,
                2.,
                8.,
                None,
                &options(acceleration),
                &mut diagnostics,
                &mut |_| Ok(()),
            )
            .unwrap()
            .0
        };
        let cpu = run(crate::Acceleration::Cpu);
        let Some(gpu_available) = (crate::gpu::GpuContext::new().map(|_| ())) else {
            eprintln!("no GPU adapter; skipping");
            return;
        };
        let _ = gpu_available;
        let gpu = run(crate::Acceleration::Gpu);
        let mut valid = 0;
        let mut agree = 0;
        for i in 0..cpu.len() {
            if cpu[i] > 0. {
                valid += 1;
                if (gpu[i] - cpu[i]).abs() <= 0.15 {
                    agree += 1;
                }
            }
        }
        eprintln!("valid={valid} agree={agree} cpu[820]={} gpu[820]={}", cpu[820], gpu[820]);
        assert!(valid > 400, "plane should be mostly valid: {valid}");
        assert!(agree as f64 >= 0.95 * valid as f64, "GPU depth rows disagree: {agree}/{valid}");
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn host_sweep_split_matches_cpu_sweep() {
        // Same shifted-plane scene as the GPU sweep test: prepare the host
        // payloads, score them natively with the same shader, finish, and the
        // depth map must match the CPU sweep.
        let mut rng = 0xABCDEFu64;
        let mut tex = vec![0f32; 80 * 80];
        for v in &mut tex {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            *v = (rng >> 33) as f32 / 2147483648.0;
        }
        let mut src_values = vec![0f32; 80 * 80];
        for y in 0..80 {
            for x in 0..70 {
                src_values[y * 80 + x] = tex[y * 80 + x + 10];
            }
        }
        let reference = Camera::identity(80., 40., 40.);
        let source_camera = Camera { rotation: ID, translation: [-0.5, 0., 0.], focal: 80., cx: 40., cy: 40. };
        let gray = [
            Some(GrayImage { width: 80, height: 80, values: tex }),
            Some(GrayImage { width: 80, height: 80, values: src_values }),
        ];
        let point = |x, y, z| crate::Point {
            position: [x, y, z],
            color: [0; 3],
            observations: vec![(0, 0), (1, 0)],
        };
        let mut points = Vec::new();
        for i in 0..12 {
            let x = (i % 4) as f64 - 1.5;
            let y = (i / 4) as f64 - 1.;
            points.push(point(x * 0.4, y * 0.4, 3.5 + i as f64 * 0.05));
        }
        let sparse = Reconstruction {
            cameras: vec![Some(reference.clone()), Some(source_camera)],
            points,
            input_images: 2,
            reprojection_rmse: 0.,
        };
        let images = [
            Image { width: 80, height: 80, rgb: vec![], focal: 80. },
            Image { width: 80, height: 80, rgb: vec![], focal: 80. },
        ];
        let options = DenseOptions {
            depth_hypotheses: 16,
            patch_radius: 1,
            min_support_views: 1,
            max_side: 64,
            ..Default::default()
        };
        let (views, prepared) =
            prepare_host_views(&images, &sparse, &options, &mut |_, _, _| true).unwrap();
        let view = views[0].as_ref().expect("view 0 prepared");
        let run_scores = |view: &HostSweepView| -> Option<Vec<f32>> {
            let mut flat: Vec<f32> = Vec::new();
            let mut payloads = Vec::new();
            for s in &view.sources {
                let gray_offset = (gray[view.ref_image].as_ref().unwrap().values.len() + flat.len()) as u32;
                flat.extend_from_slice(&gray[s.image].as_ref().unwrap().values);
                payloads.push(crate::gpu::sweep::SourcePayload {
                    rotation: s.rotation.map(|row| row.map(|x| x as f32)),
                    translation: s.translation.map(|x| x as f32),
                    focal: s.focal as f32,
                    cx: s.cx as f32,
                    cy: s.cy as f32,
                    gray_offset,
                    gray_width: 80,
                    gray_height: 80,
                });
            }
            let hyps: Vec<f32> = view.hypotheses.iter().map(|&z| z as f32).collect();
            crate::gpu::sweep::sweep_view(
                &gray[view.ref_image].as_ref().unwrap().values,
                80,
                80,
                view.ref_focal,
                view.ref_cx,
                view.ref_cy,
                &payloads,
                &flat,
                &hyps,
                view.map_width,
                view.map_height,
                1,
                view.step,
                view.needed,
            )
        };
        let Some(_) = run_scores(view) else {
            eprintln!("no GPU adapter; skipping");
            return;
        };
        let scores: Vec<Option<Vec<f32>>> = views.iter().map(|v| v.as_ref().and_then(run_scores)).collect();
        let (maps, _) = finish_host_views(
            &images,
            &sparse,
            &options,
            &gray,
            &prepared,
            &scores,
            &mut |_, _, _| true,
        )
        .unwrap();
        // CPU reference sweep for the same view.
        let mut sources = vec![Source {
            image: gray[1].as_ref().unwrap(),
            camera: &sparse.cameras[1].as_ref().unwrap(),
            rotation: ID,
            translation: [-0.5, 0., 0.],
            offsets: std::array::from_fn(|i| {
                [((i % 3) as f64 - 1.) * 1.25 / 80., ((i / 3) as f64 - 1.) * 1.25 / 80., 0.]
            }),
            rays: [[0.; 3]; 25],
        }];
        let mut diagnostics = DenseDiagnostics::default();
        let (cpu_depth, _) = sweep_depth(
            gray[0].as_ref().unwrap(),
            &reference,
            &mut sources,
            64,
            64,
            1.25,
            3.5 * 0.85,
            4.05 * 1.15,
            None,
            &options,
            &mut diagnostics,
            &mut |_| Ok(()),
        )
        .unwrap();
        let host_depth = &maps[0].depth;
        let mut valid = 0;
        let mut agree = 0;
        for i in 0..cpu_depth.len() {
            if cpu_depth[i] > 0. {
                valid += 1;
                if (host_depth[i] - cpu_depth[i]).abs() <= 0.15 {
                    agree += 1;
                }
            }
        }
        assert!(valid > 400, "plane should be mostly valid: {valid}");
        assert!(agree as f64 >= 0.95 * valid as f64, "host split disagrees: {agree}/{valid}");
    }

    #[test]
    fn sparse_intervals_cover_anchors_deterministically() {
        let camera = Camera::identity(100., 32., 32.);
        let point = |x, y, z, views: Vec<(usize, usize)>| crate::Point {
            position: [x, y, z],
            color: [0; 3],
            observations: views,
        };
        let sparse = Reconstruction {
            cameras: vec![Some(camera.clone())],
            points: vec![
                point(-0.5, -0.5, 4., vec![(0, 0)]),
                point(0.5, 0.5, 5., vec![(0, 1)]),
                // Not observed by the reference view: never splatted.
                point(0., 0., 2., vec![(1, 0)]),
            ],
            input_images: 1,
            reprojection_rmse: 0.,
        };
        assert!(sparse_intervals(&sparse, 2, &camera, 64, 64, 1., 3., 6.).is_none());
        let a = sparse_intervals(&sparse, 0, &camera, 64, 64, 1., 3., 6.).unwrap();
        let b = sparse_intervals(&sparse, 0, &camera, 64, 64, 1., 3., 6.).unwrap();
        assert_eq!(a, b);
        // The window around each anchor brackets its padded depth interval.
        let pixel = camera.project([-0.5, -0.5, 4.]).unwrap();
        let i = pixel[1] as usize * 64 + pixel[0] as usize;
        assert!(a.0[i] <= 4. && a.1[i] >= 4., "{:?}", (a.0[i], a.1[i]));
        // Far-away pixels without anchors keep the zero (global range) marker.
        assert_eq!(a.0[0], 0.);
        assert_eq!(a.1[0], 0.);
    }
    #[test]
    fn coarse_intervals_bracket_lifted_depth() {
        let coarse = [0., 4., 5., 0.];
        let (lo, hi) = coarse_intervals(&coarse, 2, 4, 4, 3., 6., 64);
        // Pixel (1, 1) lifts coarse cell (0, 0): empty, keeps the zero marker.
        assert_eq!(lo[5], 0.);
        // Pixel (2, 0) lifts coarse cell (1, 0) with depth 4.
        assert!(lo[2] < 4. && hi[2] > 4., "{:?}", (lo[2], hi[2]));
        assert!(lo[2] >= 3. && hi[2] <= 6.);
    }
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
