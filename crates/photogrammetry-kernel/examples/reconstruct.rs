use photogrammetry_kernel::{reconstruct_detailed, Image, ReconstructionOptions};
use std::{fs, io::Write};
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() < 4 {
        return Err("Usage: reconstruct OUTPUT.ply FOCAL_PIXELS INPUT.ppm INPUT.ppm ...".into());
    }
    let focal: f64 = args[1].parse()?;
    let mut images = Vec::new();
    for path in &args[2..] {
        let bytes = fs::read(path)?;
        let mut i = 0;
        let mut tokens = Vec::new();
        while tokens.len() < 4 {
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'#' {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            if i == bytes.len() {
                return Err("Truncated PPM header".into());
            }
            let start = i;
            while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            tokens.push(std::str::from_utf8(&bytes[start..i])?.to_string());
        }
        if tokens[0] != "P6" || tokens[3] != "255" {
            return Err("Expected 8-bit P6 PPM".into());
        }
        if bytes.get(i) == Some(&b'\r') && bytes.get(i + 1) == Some(&b'\n') {
            i += 2;
        } else {
            i += 1;
        }
        let width = tokens[1].parse()?;
        let height = tokens[2].parse()?;
        images.push(Image {
            width,
            height,
            rgb: bytes[i..].to_vec(),
            focal,
        });
    }
    let started = std::time::Instant::now();
    let mut options = ReconstructionOptions::default();
    options.feature_options = match std::env::var("PHOTO_FEATURE_PROFILE").as_deref() {
        Ok("baseline") => photogrammetry_kernel::features::FeatureOptions::BASELINE,
        Ok("subpixel") => photogrammetry_kernel::features::FeatureOptions {
            subpixel: true,
            ..photogrammetry_kernel::features::FeatureOptions::BASELINE
        },
        Ok("refined") => photogrammetry_kernel::features::FeatureOptions::REFINED,
        Ok("root") => photogrammetry_kernel::features::FeatureOptions::ROOT,
        Ok(other) => return Err(format!("Unknown PHOTO_FEATURE_PROFILE: {other}").into()),
        Err(_) => options.feature_options,
    };
    options.geometry_options = match std::env::var("PHOTO_GEOMETRY_PROFILE").as_deref() {
        Ok("baseline") => photogrammetry_kernel::camera::GeometryOptions::BASELINE,
        Ok("robust") => photogrammetry_kernel::camera::GeometryOptions::ROBUST,
        Ok("safe") => photogrammetry_kernel::camera::GeometryOptions::SAFE,
        Ok("adaptive") => photogrammetry_kernel::camera::GeometryOptions::ADAPTIVE,
        Ok("consensus") => photogrammetry_kernel::camera::GeometryOptions::CONSENSUS,
        Ok("physical") => photogrammetry_kernel::camera::GeometryOptions::PHYSICAL,
        Ok(other) => return Err(format!("Unknown PHOTO_GEOMETRY_PROFILE: {other}").into()),
        Err(_) => options.geometry_options,
    };
    let outcome = reconstruct_detailed(&images, &options, |stage, n, total| {
        eprintln!("{stage}: {n}/{total}");
        true
    });
    if std::env::var_os("PHOTO_REPORT").is_some() {
        eprintln!("{:#?}", outcome.report);
    }
    let result = outcome.reconstruction?;
    eprintln!(
        "sparse_elapsed_ms={:.3}",
        started.elapsed().as_secs_f64() * 1000.
    );
    if std::env::var_os("PHOTO_DENSE").is_some() {
        let mut last = String::new();
        let mut dense_options = photogrammetry_kernel::dense::DenseOptions::default();
        match std::env::var("PHOTO_DENSE_PROFILE").as_deref() {
            Ok("slanted") => {
                dense_options.estimator =
                    photogrammetry_kernel::dense::DenseEstimator::SlantedPlane;
                dense_options.patch_radius = 2;
            }
            Ok("baseline") | Err(_) => (),
            Ok(other) => return Err(format!("Unknown PHOTO_DENSE_PROFILE: {other}").into()),
        }
        let dense = photogrammetry_kernel::dense::densify_with_options(
            &images,
            &result,
            &dense_options,
            |stage, n, total| {
                let key = format!("{stage} {}%", n * 100 / total.max(1) / 10 * 10);
                if key != last {
                    eprintln!("{key}");
                    last = key;
                }
                true
            },
        )?;
        eprintln!(
            "dense_hypotheses={} source_patches={} sampled_source_pixels={}",
            dense.diagnostics.evaluated_hypotheses,
            dense.diagnostics.evaluated_source_patches,
            dense.diagnostics.sampled_source_pixels
        );
        let dense = dense.surface;
        let mut f = fs::File::create(format!("{}.surface.ply", args[0]))?;
        writeln!(f,"ply\nformat ascii 1.0\nelement vertex {}\nproperty float x\nproperty float y\nproperty float z\nproperty uchar red\nproperty uchar green\nproperty uchar blue\nelement face {}\nproperty list uchar int vertex_indices\nend_header",dense.positions.len(),dense.triangles.len())?;
        for (p, c) in dense.positions.iter().zip(&dense.colors) {
            writeln!(f, "{} {} {} {} {} {}", p[0], p[1], p[2], c[0], c[1], c[2])?;
        }
        for t in &dense.triangles {
            writeln!(f, "3 {} {} {}", t[0], t[1], t[2])?;
        }
        eprintln!(
            "surface vertices={} faces={}",
            dense.positions.len(),
            dense.triangles.len()
        );
    }
    let mut file = fs::File::create(&args[0])?;
    writeln!(file,"ply\nformat ascii 1.0\nelement vertex {}\nproperty float x\nproperty float y\nproperty float z\nproperty uchar red\nproperty uchar green\nproperty uchar blue\nend_header",result.points.len())?;
    for p in &result.points {
        writeln!(
            file,
            "{} {} {} {} {} {}",
            p.position[0], p.position[1], p.position[2], p.color[0], p.color[1], p.color[2]
        )?;
    }
    eprintln!(
        "registered={}/{} points={} reprojection_rmse={} elapsed={:?}",
        result.cameras.iter().filter(|c| c.is_some()).count(),
        images.len(),
        result.points.len(),
        result.reprojection_rmse,
        started.elapsed()
    );
    Ok(())
}
