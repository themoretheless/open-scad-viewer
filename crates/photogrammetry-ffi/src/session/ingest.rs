//! Photo ingestion and calibration: measured inputs, rectification, pixel alignment.
use super::*;
pub fn add(width: usize, height: usize, focal: f64, rgb: Box<[u8]>) -> Result<usize> {
    add_image(width, height, focal, rgb, None)
}

struct MeasuredInput {
    id: String,
    group: Value,
    calibration: Calibration,
    source_size: [usize; 2],
}
impl MeasuredInput {
    fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > 16 * 1024 {
            return Err(input("Calibration metadata exceeds 16 KiB"));
        }
        let value = value_codec::decode_binary(bytes).map_err(|e| e.to_string())?;
        let group = value["group"].clone();
        let id: String = field(&group, "id")?;
        let label: String = field(&group, "label")?;
        let source: String = field(&group, "source")?;
        if id.is_empty()
            || id.len() > 128
            || label.is_empty()
            || label.len() > 512
            || source.is_empty()
            || source.len() > 8192
        {
            return Err(input(
                "Calibration group requires a bounded id, label and measurement source",
            ));
        }
        let distortion = &group["distortion"];
        if distortion["model"].as_str() != Some("brown-conrady") {
            return Err(input(
                "Only measured Brown-Conrady calibration is supported",
            ));
        }
        let calibration = Calibration {
            width: field(&group, "imageWidth")?,
            height: field(&group, "imageHeight")?,
            fx: field(&group, "fx")?,
            fy: field(&group, "fy")?,
            cx: field(&group, "cx")?,
            cy: field(&group, "cy")?,
            k1: field(distortion, "k1")?,
            k2: field(distortion, "k2")?,
            k3: field(distortion, "k3")?,
            p1: field(distortion, "p1")?,
            p2: field(distortion, "p2")?,
        };
        calibration.validate().map_err(input)?;
        let source_size = [
            field(&value, "sourceWidth")?,
            field(&value, "sourceHeight")?,
        ];
        if source_size != [calibration.width, calibration.height] {
            return Err(input(
                "Calibration dimensions do not match the oriented original photo",
            ));
        }
        Ok(Self {
            id,
            group,
            calibration,
            source_size,
        })
    }
}

pub fn add_calibrated(
    width: usize,
    height: usize,
    focal: f64,
    rgb: Box<[u8]>,
    metadata: &[u8],
) -> Result<usize> {
    add_image(
        width,
        height,
        focal,
        rgb,
        Some(MeasuredInput::parse(metadata)?),
    )
}

fn calibration_provenance(
    index: usize,
    image: &Image,
    measured: Option<(&MeasuredInput, &RectificationReport)>,
) -> Value {
    let output = json!({"width": image.width, "height": image.height, "focal": image.focal,
        "cx": image.width as f64 / 2., "cy": image.height as f64 / 2.});
    match measured {
        Some((input, report)) => json!({
            "image": index, "mode": "measured-brown", "group": input.group,
            "sourceSize": input.source_size, "inputSize": report.input_size,
            "output": output, "zoom": report.zoom,
            "outputFovDegrees": report.output_fov_degrees,
            "resampled": report.resampled, "borderPolicy": "fully-valid",
        }),
        None => {
            json!({"image": index, "mode": "focal-hint", "inputSize": [image.width, image.height],
            "output": output, "resampled": false})
        }
    }
}

// Session-local rectification mirroring calibration::rectify, so the grid
// search focal — a function of (calibration, width, height) only, never of the
// pixels — can be reused across the photos of one calibration group. Every
// float expression is verbatim from the kernel; the cached-focal test below
// compares both paths bit for bit and fails if they ever drift apart.
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

/// Rectify like the kernel, skipping the grid search when the group cache
/// already holds its deterministic result. Returns the searched focal so the
/// caller can cache it; None on the undistorted copy path and on cache hits.
fn rectify_searched(
    image: &Image,
    calibration: &Calibration,
    source_size: [usize; 2],
    cached_high: Option<f64>,
) -> Result<(RectifiedImage, Option<f64>)> {
    let options = RectificationOptions::default();
    image.validate()?;
    if !options.max_zoom.is_finite()
        || !(1. ..=8.).contains(&options.max_zoom)
        || image.rgb.len() > options.max_output_bytes
        || options.max_pixel_evaluations == 0
    {
        return Err("Rectification exceeds configured image/work limits".into());
    }
    let cal = calibration.resized(image.width, image.height, source_size)?;
    let base_focal = (cal.fx * cal.fy).sqrt();
    if !(20. ..=20000.).contains(&base_focal) {
        return Err("Resized measured focal is outside the kernel's supported range".into());
    }
    let mut work = 0usize;
    let mut checkpoint = |amount: usize| -> Result<()> {
        work = work
            .checked_add(amount)
            .ok_or("Rectification work overflow")?;
        if work > options.max_pixel_evaluations {
            return Err("Rectification mapping work limit exceeded".into());
        }
        // The kernel's progress callback is always `true` here; the WASM host
        // cancels by terminating its disposable Worker instead.
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
        return Ok((
            RectifiedImage {
                image: copy,
                report: report(cal.fx, false),
            },
            None,
        ));
    }
    let max_focal = (base_focal * options.max_zoom).min(20000.);
    let mut searched_high = None;
    let mut high = match cached_high {
        Some(high) => high,
        None => {
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
                    return Err("Calibration has no fully valid view within the zoom limit".into());
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
            searched_high = Some(high);
            high
        }
    };
    // The coarse grid only chooses a candidate. Every actual output pixel is
    // checked before sampling; hidden folds/invalid intervals cannot be filled.
    let mut rgb = vec![0; image.rgb.len()];
    let half_width = image.width as f64 / 2.;
    let half_height = image.height as f64 / 2.;
    let max_u = (image.width - 1) as f64;
    let max_v = (image.height - 1) as f64;
    for _ in 0..RENDER_ATTEMPTS {
        let mut valid = true;
        'rows: for y in 0..image.height {
            checkpoint(image.width)?;
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
            return Ok((
                RectifiedImage {
                    image: Image {
                        width: image.width,
                        height: image.height,
                        focal: high,
                        rgb,
                    },
                    report: report(high, true),
                },
                searched_high,
            ));
        }
        if high >= max_focal {
            break;
        }
        high = (high * 1.04).min(max_focal);
    }
    Err("Calibration contains invalid or folded pixels; no border-filled image was produced".into())
}

fn add_image(
    width: usize,
    height: usize,
    focal: f64,
    rgb: Box<[u8]>,
    measured: Option<MeasuredInput>,
) -> Result<usize> {
    if width.checked_mul(height).and_then(|n| n.checked_mul(3)) != Some(rgb.len())
        || rgb.len() > 3 * 2048 * 2048
    {
        return Err(input("Invalid RGB image dimensions"));
    }
    PHOTO.with(|session| {
        let mut s = session.borrow_mut();
        if s.images.len() >= 24
            || s.images.iter().map(|i| i.rgb.len()).sum::<usize>() + rgb.len() > 96 * 1024 * 1024
        {
            return Err(input("Photo session exceeds 24 images or 96 MiB"));
        }
        if let Some(input) = &measured {
            if s.groups
                .get(&input.id)
                .is_some_and(|group| group != &input.group)
            {
                return Err("The same calibration group id has conflicting measurements".into());
            }
        }
        let raw = Image {
            width,
            height,
            focal,
            // The WASM host's buffer is adopted instead of copied.
            rgb: rgb.into_vec(),
        };
        raw.validate().map_err(input)?;
        let (image, provenance) = if let Some(input) = &measured {
            // Rectify exactly once. Both sparse and dense read these same stored pixels.
            // Worker termination owns browser cancellation during synchronous WASM.
            // The same group id carries identical measurements (checked above), so
            // its grid-search focal is deterministic and reusable across photos.
            let key = (input.id.clone(), width, height);
            let cached_high = s.rectify_high.get(&key).copied();
            let (rectified, searched_high) =
                rectify_searched(&raw, &input.calibration, input.source_size, cached_high)?;
            if let Some(high) = searched_high {
                s.rectify_high.insert(key, high);
            }
            let provenance = calibration_provenance(
                s.images.len(),
                &rectified.image,
                Some((input, &rectified.report)),
            );
            (rectified.image, provenance)
        } else {
            let provenance = calibration_provenance(s.images.len(), &raw, None);
            (raw, provenance)
        };
        // Publish pixels and metadata together only after complete rectification.
        if let Some(input) = measured {
            s.groups.insert(input.id, input.group);
        }
        s.images.push(image);
        s.calibrations.push(provenance);
        s.sparse = None;
        s.dense = None;
        s.diagnostics = None;
        Ok(s.images.len())
    })
}
