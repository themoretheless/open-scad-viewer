use super::*;
use crate::camera::{triangulate, Camera};

fn calibration() -> Calibration {
    Calibration {
        width: 320,
        height: 240,
        fx: 245.,
        fy: 270.,
        cx: 157.,
        cy: 122.,
        k1: 0.42,
        k2: 0.04,
        k3: 0.002,
        p1: 0.015,
        p2: -0.01,
    }
}
fn image() -> Image {
    Image {
        width: 320,
        height: 240,
        focal: 250.,
        rgb: (0..320 * 240 * 3).map(|i| (i % 253) as u8).collect(),
    }
}

#[test]
fn identity_preserves_every_pixel_and_uses_the_measured_focal() {
    let input = image();
    let c = Calibration {
        fx: 300.,
        fy: 300.,
        cx: 160.,
        cy: 120.,
        k1: 0.,
        k2: 0.,
        k3: 0.,
        p1: 0.,
        p2: 0.,
        ..calibration()
    };
    let out = rectify(&input, &c, [320, 240], &Default::default(), |_, _| true).unwrap();
    assert_eq!(out.image.rgb, input.rgb);
    assert_eq!(out.image.focal, 300.);
    assert!(!out.report.resampled);
    assert_eq!(out.report.zoom, 1.);
}

#[test]
fn resize_uses_pixel_centres_and_rejects_wrong_crop_or_orientation() {
    let mut c = calibration();
    c.width *= 3;
    c.height *= 3;
    c.fx *= 3.;
    c.fy *= 3.;
    c.cx = (c.cx + 0.5) * 3. - 0.5;
    c.cy = (c.cy + 0.5) * 3. - 0.5;
    assert_eq!(c.resized(320, 240, [960, 720]).unwrap(), calibration());
    assert!(c
        .resized(320, 240, [720, 960])
        .unwrap_err()
        .contains("oriented"));
    assert!(c
        .resized(300, 240, [960, 720])
        .unwrap_err()
        .contains("aspect"));
}

#[test]
fn all_rectified_pixels_have_valid_unfolded_source_support() {
    let c = calibration();
    let out = rectify(&image(), &c, [320, 240], &Default::default(), |_, _| true).unwrap();
    assert!(out.report.zoom > 1.);
    for y in 0..240 {
        for x in 0..320 {
            assert!(source_pixel(&c, 320, 240, out.image.focal, x as f64, y as f64).is_some());
        }
    }
}

#[test]
fn cancellation_and_limits_never_change_the_input() {
    let input = image();
    let before = input.rgb.clone();
    let mut calls = 0;
    let cancelled = rectify(
        &input,
        &calibration(),
        [320, 240],
        &Default::default(),
        |_, _| {
            calls += 1;
            calls < 25
        },
    );
    assert_eq!(cancelled.err().unwrap(), "Cancelled");
    assert_eq!(input.rgb, before);
    let memory = RectificationOptions {
        max_output_bytes: 5,
        ..Default::default()
    };
    assert!(rectify(&input, &calibration(), [320, 240], &memory, |_, _| true).is_err());
    let work = RectificationOptions {
        max_pixel_evaluations: 300,
        ..Default::default()
    };
    assert!(
        rectify(&input, &calibration(), [320, 240], &work, |_, _| true)
            .err()
            .unwrap()
            .contains("work limit")
    );
}

#[test]
fn invalid_calibration_and_no_valid_rectangle_are_rejected() {
    let input = image();
    for bad in [f64::NAN, f64::INFINITY, -1.] {
        let c = Calibration {
            fx: bad,
            ..calibration()
        };
        assert!(rectify(&input, &c, [320, 240], &Default::default(), |_, _| true).is_err());
    }
    let c = Calibration {
        cx: 0.,
        k1: 0.,
        k2: 0.,
        k3: 0.,
        p1: 0.,
        p2: 0.,
        ..calibration()
    };
    assert!(
        rectify(&input, &c, [320, 240], &Default::default(), |_, _| true)
            .err()
            .unwrap()
            .contains("zoom limit")
    );
    assert!(Calibration {
        k1: -3.,
        ..calibration()
    }
    .project_ray([0.5, 0.])
    .is_none());
}

// Independent forward renderer: known non-coplanar 3D landmarks are rendered as
// Gaussian spots in measured Brown cameras. Tests recover centroids from actual
// RGB before/after rectification, then triangulate with known extrinsics. This
// measures the calibration/image path, not feature matching or blind SfM.
fn measured_pixel(c: &Calibration, centre: f64, p: [f64; 3]) -> [f64; 2] {
    let x = (p[0] - centre) / p[2];
    let y = p[1] / p[2];
    let r = x * x + y * y;
    let q = 1. + c.k1 * r + c.k2 * r * r + c.k3 * r * r * r;
    [
        c.fx * (x * q + 2. * c.p1 * x * y + c.p2 * (3. * x * x + y * y)) + c.cx,
        c.fy * (y * q + c.p1 * (x * x + 3. * y * y) + 2. * c.p2 * x * y) + c.cy,
    ]
}
fn render(c: &Calibration, centre: f64, points: &[[f64; 3]]) -> Image {
    let mut out = Image {
        width: c.width,
        height: c.height,
        focal: (c.fx * c.fy).sqrt(),
        rgb: vec![0; c.width * c.height * 3],
    };
    for &point in points {
        let p = measured_pixel(c, centre, point);
        for y in (p[1] as isize - 6)..=(p[1] as isize + 6) {
            for x in (p[0] as isize - 6)..=(p[0] as isize + 6) {
                if x < 0 || y < 0 || x >= c.width as isize || y >= c.height as isize {
                    continue;
                }
                let r2 = (x as f64 - p[0]).powi(2) + (y as f64 - p[1]).powi(2);
                let value = (240. * (-r2 / 2.88).exp()).round() as u8;
                let i = (y as usize * c.width + x as usize) * 3;
                for channel in &mut out.rgb[i..i + 3] {
                    *channel = (*channel).max(value);
                }
            }
        }
    }
    out
}
fn centroid(image: &Image, expected: [f64; 2]) -> [f64; 2] {
    let mut sum = [0.; 3];
    for y in expected[1].round() as isize - 6..=expected[1].round() as isize + 6 {
        for x in expected[0].round() as isize - 6..=expected[0].round() as isize + 6 {
            if x < 0 || y < 0 || x >= image.width as isize || y >= image.height as isize {
                continue;
            }
            let weight = image.rgb[(y as usize * image.width + x as usize) * 3] as f64;
            sum[0] += weight * x as f64;
            sum[1] += weight * y as f64;
            sum[2] += weight;
        }
    }
    assert!(sum[2] > 100.);
    [sum[0] / sum[2], sum[1] / sum[2]]
}

#[test]
fn measured_mixed_cameras_reduce_known_3d_error_in_rendered_images() {
    let calibrations = [
        calibration(),
        Calibration {
            fx: 270.,
            fy: 240.,
            cx: 162.,
            cy: 117.,
            k1: 0.3,
            p1: -0.012,
            p2: 0.017,
            ..calibration()
        },
    ];
    let centres = [-0.45, 0.45];
    let points: Vec<[f64; 3]> = (0..5)
        .flat_map(|y| {
            (0..5).map(move |x| {
                [
                    (x as f64 - 2.) * 0.5,
                    (y as f64 - 2.) * 0.35,
                    3.4 + ((x + y) % 3) as f64 * 0.4,
                ]
            })
        })
        .collect();
    let input: Vec<_> = calibrations
        .iter()
        .enumerate()
        .map(|(i, c)| render(c, centres[i], &points))
        .collect();
    let output: Vec<_> = input
        .iter()
        .zip(&calibrations)
        .map(|(image, c)| rectify(image, c, [320, 240], &Default::default(), |_, _| true).unwrap())
        .collect();
    let make_cameras = |focals: [f64; 2]| -> [Camera; 2] {
        std::array::from_fn(|i| {
            let mut c = Camera::identity(focals[i], 160., 120.);
            c.translation[0] = -centres[i];
            c
        })
    };
    let before = make_cameras([input[0].focal, input[1].focal]);
    let after = make_cameras([output[0].image.focal, output[1].image.focal]);
    let mut before_squared = 0.;
    let mut after_squared = 0.;
    for &point in &points {
        let raw: [([f64; 2], [f64; 2]); 2] = std::array::from_fn(|i| {
            (
                centroid(
                    &input[i],
                    measured_pixel(&calibrations[i], centres[i], point),
                ),
                centroid(&output[i].image, after[i].project(point).unwrap()),
            )
        });
        let a = triangulate(&[(&before[0], raw[0].0), (&before[1], raw[1].0)]).unwrap();
        let b = triangulate(&[(&after[0], raw[0].1), (&after[1], raw[1].1)]).unwrap();
        before_squared += (0..3).map(|j| (a[j] - point[j]).powi(2)).sum::<f64>();
        after_squared += (0..3).map(|j| (b[j] - point[j]).powi(2)).sum::<f64>();
    }
    let before_rmse = (before_squared / points.len() as f64).sqrt();
    let after_rmse = (after_squared / points.len() as f64).sqrt();
    println!(
        "CALIBRATION_MEASUREMENT {{\"scene\":\"brown_rgb_known_extrinsics\",\"views\":2,\"landmarks\":{},\"before_3d_rmse\":{},\"after_3d_rmse\":{},\"units\":\"synthetic scene units\",\"output_focals\":[{},{}],\"blind_sfm\":false}}",
        points.len(),
        before_rmse,
        after_rmse,
        after[0].focal,
        after[1].focal
    );
    assert!(
        after_rmse < before_rmse * 0.1,
        "before={before_rmse}, after={after_rmse}"
    );
    assert!(after_rmse < 0.01, "after={after_rmse}");
}
