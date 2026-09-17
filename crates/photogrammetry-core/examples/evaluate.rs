//! Compare ASCII XYZ or ASCII PLY vertices in an already established common frame.
use photogrammetry_core::{
    Acceleration,
    evaluation::{DistanceSummary, EvaluationOptions, evaluate_clouds},
};
use std::{
    fs,
    io::{BufRead, BufReader, Read},
};

fn read_points(path: &str) -> Result<Vec<[f64; 3]>, Box<dyn std::error::Error>> {
    if fs::metadata(path)?.len() > 256 * 1024 * 1024 {
        return Err("Input exceeds 256 MiB".into());
    }
    let file = fs::File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut ended = false;
    let mut lines = std::iter::from_fn(move || {
        if ended {
            return None;
        }
        let mut line = String::new();
        match reader.by_ref().take(16 * 1024 + 1).read_line(&mut line) {
            Ok(0) => {
                ended = true;
                None
            }
            Ok(n) if n > 16 * 1024 => {
                ended = true;
                Some(Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Point/header line exceeds 16 KiB",
                )))
            }
            Ok(_) => Some(Ok(line)),
            Err(error) => {
                ended = true;
                Some(Err(error))
            }
        }
    });
    let first = lines.next().ok_or("Empty point file")??;
    let mut points = Vec::new();
    let parse = |line: &str, columns: [usize; 3]| -> Result<[f64; 3], Box<dyn std::error::Error>> {
        let mut result = [None; 3];
        for (i, field) in line
            .split_whitespace()
            .take(1 + *columns.iter().max().unwrap())
            .enumerate()
        {
            for axis in 0..3 {
                if i == columns[axis] {
                    result[axis] = Some(field.parse::<f64>()?);
                }
            }
        }
        Ok([
            result[0].ok_or("Missing x")?,
            result[1].ok_or("Missing y")?,
            result[2].ok_or("Missing z")?,
        ])
    };
    if first.trim() == "ply" {
        let mut ascii = false;
        let mut vertex_count = None;
        let mut in_vertices = false;
        let mut properties = Vec::new();
        let mut ended = false;
        for line in lines.by_ref() {
            let line = line?;
            if line.trim() == "end_header" {
                ended = true;
                break;
            }
            let fields = line.split_whitespace().collect::<Vec<_>>();
            match fields.as_slice() {
                ["format", "ascii", "1.0"] => ascii = true,
                ["element", "vertex", count] => {
                    if vertex_count.is_some() {
                        return Err("Duplicate PLY vertex element".into());
                    }
                    vertex_count = Some(count.parse::<usize>()?);
                    in_vertices = true;
                }
                ["element", _, count] => {
                    if vertex_count.is_none() && count.parse::<usize>()? != 0 {
                        return Err("PLY vertices must precede other nonempty elements".into());
                    }
                    in_vertices = false;
                }
                ["property", "list", ..] if in_vertices => {
                    return Err("List vertex properties are unsupported".into());
                }
                ["property", _, name] if in_vertices => {
                    if properties.len() >= 64 {
                        return Err("PLY has more than 64 vertex properties".into());
                    }
                    properties.push(name.to_string());
                }
                _ => {}
            }
        }
        if !ascii || !ended {
            return Err("Expected complete ASCII PLY 1.0 header".into());
        }
        let count = vertex_count.ok_or("PLY vertex count absent")?;
        if count == 0 || count > 2_000_000 {
            return Err("Expected 1 to 2000000 PLY vertices".into());
        }
        let columns = [
            properties
                .iter()
                .position(|x| x == "x")
                .ok_or("PLY x absent")?,
            properties
                .iter()
                .position(|x| x == "y")
                .ok_or("PLY y absent")?,
            properties
                .iter()
                .position(|x| x == "z")
                .ok_or("PLY z absent")?,
        ];
        points.reserve(count);
        for _ in 0..count {
            points.push(parse(
                &lines.next().ok_or("Truncated PLY vertices")??,
                columns,
            )?);
        }
    } else {
        for line in std::iter::once(Ok(first)).chain(lines) {
            let line = line?;
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }
            if points.len() >= 2_000_000 {
                return Err("Too many XYZ samples".into());
            }
            points.push(parse(&line, [0, 1, 2])?);
        }
    }
    Ok(points)
}

fn summary(s: &DistanceSummary) -> String {
    format!(
        "{{\"samples\":{},\"mean\":{},\"median\":{},\"p95\":{},\"maximum\":{},\"within_tolerance\":{}}}",
        s.samples, s.mean, s.median, s.p95, s.maximum, s.within_tolerance
    )
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if !(3..=5).contains(&args.len()) {
        return Err("Usage: evaluate MODEL.xyz|ply REFERENCE.xyz|ply TOLERANCE [VOXEL_SIZE] [ACCELERATION=auto]. Coordinates must already share a frame and scale. Reference must describe the evaluated domain. No scale fit is performed.".into());
    }
    let model = read_points(&args[0])?;
    let reference = read_points(&args[1])?;
    let options = EvaluationOptions {
        tolerance: args[2].parse()?,
        voxel_size: args.get(3).map(|s| s.parse()).transpose()?,
        acceleration: args
            .get(4)
            .map(|s| Acceleration::parse(s).ok_or("Unknown acceleration"))
            .transpose()?
            .unwrap_or(Acceleration::Auto),
    };
    let started = std::time::Instant::now();
    let r = evaluate_clouds(&model, &reference, &options, |_, _, _| true)?;
    println!(
        "{{\"format\":\"open-scad-viewer/cloud-evaluation\",\"version\":1,\"metric\":\"point_to_point\",\"scale_fitted\":false,\"tolerance\":{},\"voxel_size\":{},\"input_model\":{},\"input_reference\":{},\"model_to_reference\":{},\"reference_to_model\":{},\"precision\":{},\"recall\":{},\"f1\":{},\"symmetric_mean\":{},\"evaluation_ms\":{}}}",
        options.tolerance,
        options.voxel_size.map_or("null".into(), |v| v.to_string()),
        r.input_reconstructed,
        r.input_reference,
        summary(&r.reconstructed_to_reference),
        summary(&r.reference_to_reconstructed),
        r.precision,
        r.recall,
        r.f1,
        r.symmetric_mean,
        started.elapsed().as_secs_f64() * 1000.
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    fn parse_fixture(text: &str) -> Result<Vec<[f64; 3]>, Box<dyn std::error::Error>> {
        let path = std::env::temp_dir().join(format!(
            "photo-evaluation-{}-{}.txt",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        std::io::Write::write_all(&mut file, text.as_bytes())?;
        drop(file);
        let result = read_points(path.to_str().unwrap());
        fs::remove_file(path)?;
        result
    }
    #[test]
    fn coordinates_follow_declared_ply_property_order() {
        let text = "ply\nformat ascii 1.0\nelement vertex 1\nproperty uchar red\nproperty float z\nproperty float x\nproperty float y\nend_header\n255 3 1 2\n";
        assert_eq!(parse_fixture(text).unwrap(), vec![[1., 2., 3.]]);
    }
    #[test]
    fn nonvertex_data_cannot_be_misread_as_coordinates() {
        let text = "ply\nformat ascii 1.0\nelement face 1\nproperty list uchar int vertex_indices\nelement vertex 1\nproperty float x\nproperty float y\nproperty float z\nend_header\n3 0 1 2\n1 2 3\n";
        assert!(
            parse_fixture(text)
                .unwrap_err()
                .to_string()
                .contains("vertices must precede")
        );
    }
    #[test]
    fn an_oversized_line_is_rejected_before_collecting_fields() {
        assert!(
            parse_fixture(&"1 ".repeat(20_000))
                .unwrap_err()
                .to_string()
                .contains("16 KiB")
        );
    }
}
