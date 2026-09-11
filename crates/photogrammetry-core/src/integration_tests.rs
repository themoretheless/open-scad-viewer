use crate::camera::Camera;
use crate::*;
#[test]
fn rejects_invalid_images_and_cancels_before_features() {
    let invalid = Image {
        width: 64,
        height: 64,
        rgb: vec![0; 12],
        focal: 100.,
    };
    assert!(reconstruct(&[invalid.clone(), invalid], |_, _, _| true).is_err());
    let image = Image {
        width: 64,
        height: 64,
        rgb: vec![0; 64 * 64 * 3],
        focal: 100.,
    };
    assert_eq!(
        reconstruct(&[image.clone(), image], |_, _, _| false)
            .unwrap_err()
            .message,
        "Cancelled"
    );
}
#[test]
fn translated_features_have_correct_correspondences() {
    let (w, h) = (160, 128);
    let mut rng = camera::Rng::new();
    let noise: Vec<u8> = (0..w * h).map(|_| rng.next(256) as u8).collect();
    let make = |shift: usize| {
        let mut rgb = vec![0; w * h * 3];
        for y in 0..h {
            for x in shift..w {
                for c in 0..3 {
                    rgb[(y * w + x) * 3 + c] = noise[y * w + x - shift];
                }
            }
        }
        Image {
            width: w,
            height: h,
            rgb,
            focal: 150.,
        }
    };
    let a = features::extract(&make(0), 300).unwrap();
    let b = features::extract(&make(12), 300).unwrap();
    let pairs = features::matches(&a, &b);
    assert!(pairs.len() > 40, "{}", pairs.len());
    let correct = pairs
        .iter()
        .filter(|m| (b[m.b].x - a[m.a].x - 12.).abs() < 1. && (b[m.b].y - a[m.a].y).abs() < 1.)
        .count();
    assert!(correct * 10 > pairs.len() * 9);
}
#[test]
fn dense_plane_recovers_known_depth() {
    let cameras: Vec<_> = [0., -0.3]
        .iter()
        .map(|&x| {
            let mut c = Camera::identity(90., 32., 32.);
            c.translation = [x, 0., 0.];
            Some(c)
        })
        .collect();
    let images: Vec<_> = cameras
        .iter()
        .map(|c| {
            let c = c.as_ref().unwrap();
            let mut rgb = Vec::new();
            for y in 0..64 {
                for x in 0..64 {
                    let wx = (x as f64 - 32.) / 90. * 4. - c.translation[0];
                    let wy = (y as f64 - 32.) / 90. * 4.;
                    let value = (128.
                        + 75.
                            * (wx * 43. + (wy * 17.).sin()).sin()
                            * (wy * 39. + (wx * 13.).sin()).cos())
                        as u8;
                    rgb.extend([value, value, value]);
                }
            }
            Image {
                width: 64,
                height: 64,
                rgb,
                focal: 90.,
            }
        })
        .collect();
    let points = (0..25)
        .map(|i| Point {
            position: [(i % 5) as f64 / 4. - 0.5, (i / 5) as f64 / 4. - 0.5, 4.],
            color: [128; 3],
            observations: vec![(0, i), (1, i)],
        })
        .collect();
    let sparse = Reconstruction {
        cameras,
        points,
        input_images: 2,
        reprojection_rmse: 0.,
    };
    let surface = dense::densify(&images, &sparse, 64, |_, _, _| true).unwrap();
    assert!(surface.triangles.len() > 50);
    let mut errors: Vec<_> = surface
        .positions
        .iter()
        .map(|p| (p[2] - 4.).abs())
        .collect();
    errors.sort_by(f64::total_cmp);
    assert!(
        errors[errors.len() / 2] < 0.06,
        "{}",
        errors[errors.len() / 2]
    );
    assert!(
        surface
            .triangles
            .iter()
            .flatten()
            .all(|&i| (i as usize) < surface.positions.len())
    );
}
