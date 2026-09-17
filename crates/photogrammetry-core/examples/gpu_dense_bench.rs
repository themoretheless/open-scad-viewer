//! GPU dense-sweep benchmark: analytic scenes with ground truth (accuracy and
//! whole-stage timing) plus a synthetic raw sweep-kernel timing. Emits JSON.
//!
//! DENSE_ACCELERATION=auto|gpu|metal|cuda|cpu (default gpu; requires --features gpu for device modes)
//! DENSE_ACCURATE=1 starts from `DenseOptions::accurate()` instead of the defaults
//! DENSE_REPEAT=N            median of N timed runs after one warmup (default 5)
//! DENSE_PATCH_RADIUS=1|2    (default 1, the product default)
//! DENSE_KERNEL_SIDE=N       synthetic kernel map side (default 256; 0 skips)
#[allow(unused_imports)]
use math_core::*;
use photogrammetry_core::{
    Acceleration, Image, Point, Reconstruction,
    camera::Camera,
    dense::{DenseOptions, densify_with_options},
};
use std::time::Instant;
#[path = "support/dense_fixture.rs"]
mod fixture;
use fixture::*;

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .map_or(default, |s| s.parse().unwrap())
}

/// Wall time per progress stage (first-seen order), for one densify call.
fn profile(
    images: &[Image],
    sparse: &Reconstruction,
    options: &DenseOptions,
) -> Vec<(String, f64)> {
    let mut stages: Vec<(String, Instant)> = Vec::new();
    let start = Instant::now();
    let _ = densify_with_options(images, sparse, options, |stage, _, _| {
        if stages.last().is_none_or(|(s, _)| s != stage) {
            stages.push((stage.to_string(), Instant::now()));
        }
        true
    })
    .unwrap();
    let end = Instant::now();
    let mut out = Vec::new();
    for i in 0..stages.len() {
        let next = stages.get(i + 1).map_or(end, |(_, t)| *t);
        out.push((
            stages[i].0.clone(),
            (next - stages[i].1).as_secs_f64() * 1000.,
        ));
    }
    let _ = start;
    out
}
fn stage_json(stages: &[(String, f64)]) -> String {
    stages
        .iter()
        .map(|(s, t)| format!("\"{s}\":{t:.3}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn median(times: &mut [f64]) -> f64 {
    times.sort_by(f64::total_cmp);
    times[times.len() / 2]
}

fn main() {
    let acceleration = match std::env::var("DENSE_ACCELERATION") {
        Ok(value) => Acceleration::parse(&value).unwrap_or(Acceleration::Gpu),
        Err(_) => Acceleration::Gpu,
    };
    let repeats = env_usize("DENSE_REPEAT", 5).max(1);
    let accurate = std::env::var_os("DENSE_ACCURATE").is_some();
    let base = if accurate {
        DenseOptions::accurate()
    } else {
        DenseOptions::default()
    };
    let radius = env_usize("DENSE_PATCH_RADIUS", base.patch_radius);
    let kernel_side = env_usize("DENSE_KERNEL_SIDE", 256);
    let hypotheses = env_usize("DENSE_HYPOTHESES", 64);
    let prior = base.sparse_depth_prior || std::env::var_os("DENSE_SPARSE_PRIOR").is_some();
    let backend = {
        #[cfg(feature = "gpu")]
        {
            photogrammetry_core::gpu::backend_label().unwrap_or("none")
        }
        #[cfg(not(feature = "gpu"))]
        {
            "none"
        }
    };
    println!(
        "{{\"schema\":1,\"acceleration\":\"{acceleration:?}\",\"gpu_backend\":\"{backend}\",\"accurate\":{accurate},\"patch_radius\":{radius},\"repeats\":{repeats},\"tolerance\":0.04,\"scenes\":["
    );
    let mut first = true;
    let mut f1_sum = 0.;
    let mut mean_error_sum = 0.;
    let mut time_sum = 0.;
    let scenes = [
        Scene {
            angle: 0.,
            thin: false,
        },
        Scene {
            angle: 15.,
            thin: false,
        },
        Scene {
            angle: 30.,
            thin: false,
        },
        Scene {
            angle: 60.,
            thin: false,
        },
        Scene {
            angle: 30.,
            thin: true,
        },
    ];
    for scene in scenes {
        let (images, sparse, ground) = fixture(scene);
        let options = DenseOptions {
            max_side: SIDE,
            patch_radius: radius,
            depth_hypotheses: hypotheses,
            sparse_depth_prior: prior,
            fuse: std::env::var_os("DENSE_NO_FUSE").is_none(),
            dual_scale: base.dual_scale || std::env::var_os("DENSE_DUAL").is_some(),
            fusion_merge_pass: std::env::var_os("DENSE_MERGE").is_some(),
            selected_sources_consistency: std::env::var_os("DENSE_SELCONS").is_some(),
            max_source_views: env_usize("DENSE_SOURCES", 3),
            acceleration,
            ..base.clone()
        };
        let _ = densify_with_options(&images, &sparse, &options, |_, _, _| true).unwrap();
        let mut times = Vec::with_capacity(repeats);
        let mut result = None;
        for _ in 0..repeats {
            let start = Instant::now();
            let r = densify_with_options(&images, &sparse, &options, |_, _, _| true).unwrap();
            times.push(start.elapsed().as_secs_f64() * 1000.);
            result = Some(r);
        }
        let result = result.unwrap();
        let stages = profile(&images, &sparse, &options);
        let points = &result.surface.positions;
        let mut errors: Vec<_> = points.iter().map(|&p| scene.distance(p)).collect();
        errors.sort_by(f64::total_cmp);
        let precision = errors.iter().filter(|&&d| d <= 0.04).count() as f64 / errors.len() as f64;
        let mut covered = 0;
        for &(p, _) in &ground {
            if points.iter().any(|&q| norm(sub(p, q)) <= 0.04) {
                covered += 1;
            }
        }
        let recall = covered as f64 / ground.len() as f64;
        let f1 = 2. * precision * recall / (precision + recall).max(1e-12);
        let mean_error = errors.iter().sum::<f64>() / errors.len() as f64;
        let time = median(&mut times);
        f1_sum += f1;
        mean_error_sum += mean_error;
        time_sum += time;
        if !first {
            println!(",");
        }
        first = false;
        print!(
            "{{\"angle_deg\":{},\"occlusion_thin\":{},\"dense_median_ms\":{:.3},\"stages\":{{{}}},\"vertices\":{},\"triangles\":{},\"mean_surface_error\":{:.6},\"median_surface_error\":{:.6},\"p90_surface_error\":{:.6},\"precision\":{:.6},\"completeness\":{:.6},\"f1\":{:.6}}}",
            scene.angle,
            scene.thin,
            time,
            stage_json(&stages),
            points.len(),
            result.surface.triangles.len(),
            mean_error,
            errors[errors.len() / 2],
            errors[errors.len() * 9 / 10],
            precision,
            recall,
            f1,
        );
    }
    let n = scenes.len() as f64;
    println!(
        "\n],\"summary\":{{\"dense_total_median_ms\":{:.3},\"mean_f1\":{:.6},\"mean_surface_error\":{:.6}}}",
        time_sum,
        f1_sum / n,
        mean_error_sum / n
    );
    if kernel_side > 0 {
        kernel_bench(acceleration, kernel_side, radius, hypotheses, repeats);
    }
    println!("}}");
}

/// Synthetic textured plane at the requested map side: three source cameras,
/// 64 hypotheses, product-default sweep options. Times densify's estimation
/// stage proxy (whole densify, fusion is small relative to the sweep here).
fn kernel_bench(
    acceleration: Acceleration,
    side: usize,
    radius: usize,
    hypotheses: usize,
    repeats: usize,
) {
    let focal = side as f64 * 1.4;
    let texture = |x: f64, y: f64| -> u8 {
        let v = 128.
            + 44. * (x * 19. + (y * 7.).sin()).sin()
            + 35. * (y * 23. + (x * 11.).cos()).cos()
            + 17. * (x * 37. + y * 29.).sin();
        v.clamp(0., 255.) as u8
    };
    let centers = [[0., 0., 0.], [-0.3, 0., 0.], [0.3, 0., 0.], [0., 0.25, 0.]];
    let cameras: Vec<Option<Camera>> = centers
        .iter()
        .map(|&c| {
            let mut cam = Camera::identity(focal, side as f64 / 2., side as f64 / 2.);
            cam.translation = scale(c, -1.);
            Some(cam)
        })
        .collect();
    let hit = |camera: &Camera, uv: [f64; 2]| -> V3 {
        let center = camera.center();
        let ray = mv(tr(camera.rotation), camera.ray(uv));
        let t = (4. - center[2]) / ray[2];
        add(center, scale(ray, t))
    };
    let images: Vec<Image> = cameras
        .iter()
        .map(|c| {
            let c = c.as_ref().unwrap();
            let mut rgb = Vec::with_capacity(side * side * 3);
            for y in 0..side {
                for x in 0..side {
                    let p = hit(c, [x as f64, y as f64]);
                    let v = texture(p[0], p[1]);
                    rgb.extend([v, v, v]);
                }
            }
            Image {
                width: side,
                height: side,
                rgb,
                focal,
            }
        })
        .collect();
    let mut points = Vec::new();
    let stride = (side / 12).max(4);
    for y in (8..side - 8).step_by(stride) {
        for x in (8..side - 8).step_by(stride) {
            let position = hit(cameras[0].as_ref().unwrap(), [x as f64, y as f64]);
            let observations = cameras
                .iter()
                .enumerate()
                .filter_map(|(i, c)| {
                    let uv = c.as_ref().unwrap().project(position)?;
                    (uv[0] > 3.
                        && uv[1] > 3.
                        && uv[0] < side as f64 - 4.
                        && uv[1] < side as f64 - 4.)
                        .then_some((i, points.len()))
                })
                .collect();
            points.push(Point {
                position,
                color: [128; 3],
                observations,
            });
        }
    }
    let sparse = Reconstruction {
        cameras,
        points,
        input_images: 4,
        reprojection_rmse: 0.,
    };
    let options = DenseOptions {
        max_side: side.clamp(64, 384),
        patch_radius: radius,
        depth_hypotheses: hypotheses,
        acceleration,
        ..Default::default()
    };
    let _ = densify_with_options(&images, &sparse, &options, |_, _, _| true).unwrap();
    let mut times = Vec::with_capacity(repeats);
    let mut result = None;
    for _ in 0..repeats {
        let start = Instant::now();
        let r = densify_with_options(&images, &sparse, &options, |_, _, _| true).unwrap();
        times.push(start.elapsed().as_secs_f64() * 1000.);
        result = Some(r);
    }
    let result = result.unwrap();
    let mut errors: Vec<_> = result
        .surface
        .positions
        .iter()
        .map(|p| (p[2] - 4.).abs())
        .collect();
    errors.sort_by(f64::total_cmp);
    let stages = profile(&images, &sparse, &options);
    println!(
        ",\"kernel\":{{\"side\":{side},\"dense_median_ms\":{:.3},\"stages\":{{{}}},\"vertices\":{},\"mean_depth_error\":{:.6},\"p90_depth_error\":{:.6}}}",
        median(&mut times),
        stage_json(&stages),
        result.surface.positions.len(),
        errors.iter().sum::<f64>() / errors.len() as f64,
        errors[errors.len() * 9 / 10]
    );
}
