//! Explicit measured Brown–Conrady calibration, rectified once before SfM and MVS.
//!
//! Calibration uses integer pixel centres in the already EXIF-oriented original
//! raster. Resizing follows `(p + 0.5) * scale - 0.5`. The output is the kernel's
//! existing centred, square-pixel pinhole. No invalid/black border is manufactured.
use crate::{Image, Result};

#[derive(Clone, Debug, PartialEq)]
pub struct Calibration {
    pub width: usize,
    pub height: usize,
    pub fx: f64,
    pub fy: f64,
    pub cx: f64,
    pub cy: f64,
    pub k1: f64,
    pub k2: f64,
    pub k3: f64,
    pub p1: f64,
    pub p2: f64,
}

impl Calibration {
    pub fn validate(&self) -> Result<()> {
        if self.width < 48
            || self.height < 48
            || self
                .width
                .checked_mul(self.height)
                .is_none_or(|n| n > 50_000_000)
        {
            return Err(crate::error(
                "Calibration requires an original raster of at most 50 megapixels",
            ));
        }
        if ![
            self.fx, self.fy, self.cx, self.cy, self.k1, self.k2, self.k3, self.p1, self.p2,
        ]
        .iter()
        .all(|v| v.is_finite())
            || self.fx < 1.
            || self.fy < 1.
            || self.fx > 1e7
            || self.fy > 1e7
            || self.cx < 0.
            || self.cy < 0.
            || self.cx > (self.width - 1) as f64
            || self.cy > (self.height - 1) as f64
            || [self.k1, self.k2, self.k3, self.p1, self.p2]
                .iter()
                .any(|v| v.abs() > 10.)
        {
            return Err(crate::error(
                "Invalid measured calibration; check pixel units and Brown coefficients",
            ));
        }
        Ok(())
    }

    /// The decoded source dimensions must match exactly: equal aspect ratios
    /// alone cannot establish that a calibration belongs to this raster/crop.
    pub fn resized(&self, width: usize, height: usize, source_size: [usize; 2]) -> Result<Self> {
        self.validate()?;
        if source_size != [self.width, self.height] {
            return Err(crate::error(
                "Calibration dimensions do not match the oriented original photo",
            ));
        }
        if width < 48
            || height < 48
            || width > 2048
            || height > 2048
            || width > self.width
            || height > self.height
        {
            return Err(crate::error("Invalid resized calibration raster"));
        }
        let sx = width as f64 / self.width as f64;
        let sy = height as f64 / self.height as f64;
        // Browser resizing rounds each axis independently. Reject an unrecorded
        // anisotropic resize/crop, while allowing that one-pixel rounding.
        if (width as f64 - self.width as f64 * sy).abs() > 1.01
            || (height as f64 - self.height as f64 * sx).abs() > 1.01
        {
            return Err(crate::error(
                "Calibration requires an aspect-preserving photo resize",
            ));
        }
        Ok(Self {
            width,
            height,
            fx: self.fx * sx,
            fy: self.fy * sy,
            cx: (self.cx + 0.5) * sx - 0.5,
            cy: (self.cy + 0.5) * sy - 0.5,
            ..self.clone()
        })
    }

    /// Ideal normalized ray -> measured pixel. Also rejects locally folded
    /// Brown maps; coefficients from another lens model must not look valid.
    pub fn project_ray(&self, ray: [f64; 2]) -> Option<[f64; 2]> {
        let [x, y] = ray;
        // Coefficient-only products are hoisted; every remaining float op keeps
        // its exact operand order and rounding.
        let k2x2 = 2. * self.k2;
        let p1x2 = 2. * self.p1;
        let p1x6 = 6. * self.p1;
        let p2x2 = 2. * self.p2;
        let p2x6 = 6. * self.p2;
        let r2 = x * x + y * y;
        let radial = 1. + r2 * (self.k1 + r2 * (self.k2 + r2 * self.k3));
        let derivative = self.k1 + r2 * (k2x2 + 3. * r2 * self.k3);
        let cross = 2. * x * y * derivative + p1x2 * x + p2x2 * y;
        let dx = radial + 2. * x * x * derivative + p1x2 * y + p2x6 * x;
        let dy = radial + 2. * y * y * derivative + p1x6 * y + p2x2 * x;
        let determinant = dx * dy - cross * cross;
        let u = self.fx * (x * radial + p1x2 * x * y + self.p2 * (r2 + 2. * x * x)) + self.cx;
        let v = self.fy * (y * radial + self.p1 * (r2 + 2. * y * y) + p2x2 * x * y) + self.cy;
        (u.is_finite()
            && v.is_finite()
            && determinant.is_finite()
            && dx > 1e-6
            && dy > 1e-6
            && determinant > 1e-6)
            .then_some([u, v])
    }
}

#[derive(Clone, Debug)]
pub struct RectificationOptions {
    /// Maximum focal multiplier relative to sqrt(fx*fy); limits field-of-view loss.
    pub max_zoom: f64,
    pub max_output_bytes: usize,
    pub max_pixel_evaluations: usize,
    /// Placement for independent output-pixel mapping and sampling. `Cpu`
    /// remains the bit-exact reference. `Auto` uses a GPU only when the
    /// upload/readback cost is amortized by a sufficiently large raster.
    pub acceleration: crate::Acceleration,
}
impl Default for RectificationOptions {
    fn default() -> Self {
        Self {
            max_zoom: 4.,
            max_output_bytes: 3 * 2048 * 2048,
            max_pixel_evaluations: 40_000_000,
            acceleration: crate::Acceleration::Cpu,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RectificationReport {
    pub input_size: [usize; 2],
    pub output_size: [usize; 2],
    pub focal: f64,
    pub zoom: f64,
    pub output_fov_degrees: [f64; 2],
    pub resampled: bool,
}
pub struct RectifiedImage {
    pub image: Image,
    pub report: RectificationReport,
}

const GRID: usize = 17;
const RENDER_ATTEMPTS: usize = 8;

fn source_pixel(
    calibration: &Calibration,
    width: usize,
    height: usize,
    focal: f64,
    x: f64,
    y: f64,
) -> Option<[f64; 2]> {
    let p = calibration.project_ray([
        (x - width as f64 / 2.) / focal,
        (y - height as f64 / 2.) / focal,
    ])?;
    (p[0] >= 0. && p[1] >= 0. && p[0] <= (width - 1) as f64 && p[1] <= (height - 1) as f64)
        .then_some(p)
}

fn bilinear(image: &Image, p: [f64; 2], out: &mut [u8]) {
    let (x, y) = (p[0].floor() as usize, p[1].floor() as usize);
    let (nx, ny) = ((x + 1).min(image.width - 1), (y + 1).min(image.height - 1));
    let (a, b) = (p[0] - x as f64, p[1] - y as f64);
    let (i00, i10) = ((y * image.width + x) * 3, (y * image.width + nx) * 3);
    let (i01, i11) = ((ny * image.width + x) * 3, (ny * image.width + nx) * 3);
    let at = |i: usize, channel: usize| image.rgb[i + channel] as f64;
    let c00 = [at(i00, 0), at(i00, 1), at(i00, 2)];
    let c10 = [at(i10, 0), at(i10, 1), at(i10, 2)];
    let c01 = [at(i01, 0), at(i01, 1), at(i01, 2)];
    let c11 = [at(i11, 0), at(i11, 1), at(i11, 2)];
    for (channel, value) in out.iter_mut().enumerate() {
        *value = ((1. - b) * ((1. - a) * c00[channel] + a * c10[channel])
            + b * ((1. - a) * c01[channel] + a * c11[channel]))
            .round()
            .clamp(0., 255.) as u8;
    }
}

/// Transactional rectification: input is immutable; error/cancel publishes no image.
/// Progress counts bounded mapping work. The WASM host cancels by terminating its
/// disposable Worker; native callers can cancel at every row and search step.
pub fn rectify(
    image: &Image,
    calibration: &Calibration,
    source_size: [usize; 2],
    options: &RectificationOptions,
    mut progress: impl FnMut(usize, usize) -> bool,
) -> Result<RectifiedImage> {
    image.validate()?;
    if !options.max_zoom.is_finite()
        || !(1. ..=8.).contains(&options.max_zoom)
        || image.rgb.len() > options.max_output_bytes
        || options.max_pixel_evaluations == 0
    {
        return Err(crate::error(
            "Rectification exceeds configured image/work limits",
        ));
    }
    let cal = calibration.resized(image.width, image.height, source_size)?;
    let base_focal = (cal.fx * cal.fy).sqrt();
    if !(20. ..=20000.).contains(&base_focal) {
        return Err(crate::error(
            "Resized measured focal is outside the kernel's supported range",
        ));
    }
    let mut work = 0usize;
    let mut checkpoint = |amount: usize| -> Result<()> {
        work = work
            .checked_add(amount)
            .ok_or_else(|| crate::error("Rectification work overflow"))?;
        if work > options.max_pixel_evaluations {
            return Err(crate::error("Rectification mapping work limit exceeded"));
        }
        if !progress(work, options.max_pixel_evaluations) {
            return Err(crate::error("Cancelled"));
        }
        Ok(())
    };
    checkpoint(0)?;
    let report = |focal: f64, resampled: bool| RectificationReport {
        input_size: [image.width, image.height],
        output_size: [image.width, image.height],
        focal,
        zoom: focal / base_focal,
        output_fov_degrees: [image.width, image.height]
            .map(|side| (side as f64 / (2. * focal)).atan().to_degrees() * 2.),
        resampled,
    };
    if cal.fx == cal.fy
        && cal.cx == image.width as f64 / 2.
        && cal.cy == image.height as f64 / 2.
        && [cal.k1, cal.k2, cal.k3, cal.p1, cal.p2]
            .iter()
            .all(|&v| v == 0.)
    {
        checkpoint(1)?;
        let copy = Image {
            focal: cal.fx,
            ..image.clone()
        };
        checkpoint(0)?;
        return Ok(RectifiedImage {
            image: copy,
            report: report(cal.fx, false),
        });
    }
    let max_focal = (base_focal * options.max_zoom).min(20000.);
    let mut grid_valid = |focal: f64| -> Result<bool> {
        checkpoint(GRID * GRID)?;
        Ok((0..GRID).all(|gy| {
            (0..GRID).all(|gx| {
                source_pixel(
                    &cal,
                    image.width,
                    image.height,
                    focal,
                    (image.width - 1) as f64 * gx as f64 / (GRID - 1) as f64,
                    (image.height - 1) as f64 * gy as f64 / (GRID - 1) as f64,
                )
                .is_some()
            })
        }))
    };
    let mut low = base_focal;
    let mut high = base_focal;
    while !grid_valid(high)? {
        if high >= max_focal {
            return Err(crate::error(
                "Calibration has no fully valid view within the zoom limit",
            ));
        }
        low = high;
        high = (high * 1.2).min(max_focal);
    }
    if high > base_focal {
        for _ in 0..12 {
            let middle = (low + high) * 0.5;
            if grid_valid(middle)? {
                high = middle;
            } else {
                low = middle;
            }
        }
        high = (high * 1.00001).min(max_focal);
    }
    // The coarse grid only chooses a candidate. Every actual output pixel is
    // checked before sampling; hidden folds/invalid intervals cannot be filled.
    let mut rgb = vec![0; image.rgb.len()];
    let half_width = image.width as f64 / 2.;
    let half_height = image.height as f64 / 2.;
    let max_u = (image.width - 1) as f64;
    let max_v = (image.height - 1) as f64;
    for _ in 0..RENDER_ATTEMPTS {
        // Preserve row-level cancellation and accounting before a device
        // submission; search, validation, retries, and reporting stay CPU.
        for _ in 0..image.height {
            checkpoint(image.width)?;
        }
        if let Some(device_rgb) = accelerated_render(image, &cal, high, options.acceleration) {
            checkpoint(0)?;
            return Ok(RectifiedImage {
                image: Image {
                    width: image.width,
                    height: image.height,
                    focal: high,
                    rgb: device_rgb,
                },
                report: report(high, true),
            });
        }
        let mut valid = true;
        'rows: for y in 0..image.height {
            // Row-invariant normalized coordinate; same expression source_pixel
            // would compute, evaluated once per row instead of per pixel.
            let v = (y as f64 - half_height) / high;
            for x in 0..image.width {
                let Some(p) = cal
                    .project_ray([(x as f64 - half_width) / high, v])
                    .filter(|p| p[0] >= 0. && p[1] >= 0. && p[0] <= max_u && p[1] <= max_v)
                else {
                    valid = false;
                    break 'rows;
                };
                let i = (y * image.width + x) * 3;
                bilinear(image, p, &mut rgb[i..i + 3]);
            }
        }
        if valid {
            checkpoint(0)?;
            return Ok(RectifiedImage {
                image: Image {
                    width: image.width,
                    height: image.height,
                    focal: high,
                    rgb,
                },
                report: report(high, true),
            });
        }
        if high >= max_focal {
            break;
        }
        high = (high * 1.04).min(max_focal);
    }
    Err(crate::error(
        "Calibration contains invalid or folded pixels; no border-filled image was produced",
    ))
}

const GPU_RENDER_PIXELS: usize = 256 * 256;

fn accelerated_render(
    image: &Image,
    calibration: &Calibration,
    focal: f64,
    acceleration: crate::Acceleration,
) -> Option<Vec<u8>> {
    let resolved = match acceleration {
        crate::Acceleration::Auto
            if image.width.saturating_mul(image.height) < GPU_RENDER_PIXELS =>
        {
            crate::Acceleration::Cpu
        }
        crate::Acceleration::Auto => crate::Acceleration::Gpu,
        explicit => explicit,
    };
    if !resolved.is_gpu() {
        return None;
    }
    #[cfg(feature = "gpu")]
    {
        #[cfg(feature = "cuda")]
        if resolved == crate::Acceleration::Cuda
            && let Some(rgb) = crate::gpu::rectification::render_cuda(image, calibration, focal)
        {
            return Some(rgb);
        }
        crate::gpu::rectification::render(image, calibration, focal)
    }
    #[cfg(not(feature = "gpu"))]
    {
        let _ = (image, calibration, focal);
        None
    }
}

#[cfg(test)]
mod tests;
