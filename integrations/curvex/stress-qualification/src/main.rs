use planar_geometry::{
    path::BezierPath,
    stroke::{LineCap, LineJoin, StrokeOptions, tessellate_stroke},
    tessellation::FillMesh,
};
use serde_json::json;
use std::{hint::black_box, time::Instant};
fn cross(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
}
fn hits(m: &FillMesh, p: [f64; 2]) -> usize {
    m.indices
        .chunks_exact(3)
        .filter(|t| {
            let [a, b, c] = [
                m.positions[t[0] as usize],
                m.positions[t[1] as usize],
                m.positions[t[2] as usize],
            ];
            // Top-left ownership counts a shared edge once. Strict interior
            // tests falsely report gaps when translated probes round onto it.
            let owns = |a: [f64; 2], b: [f64; 2]| {
                let e = cross(a, b, p);
                e > 0. || (e == 0. && (b[1] < a[1] || (b[1] == a[1] && b[0] > a[0])))
            };
            cross(a, b, c) > 0. && owns(a, b) && owns(b, c) && owns(c, a)
        })
        .count()
}
fn distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let t =
        (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / (d[0] * d[0] + d[1] * d[1])).clamp(0., 1.);
    (p[0] - a[0] - t * d[0]).hypot(p[1] - a[1] - t * d[1])
}
fn random(s: &mut u64) -> f64 {
    *s = s
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*s >> 11) as f64 / ((1u64 << 53) as f64)
}
fn main() {
    let mut results = vec![];
    for (name, n, width, kind) in [
        ("crossing_80", 80, 3., 0),
        ("crossing_240_wide", 240, 24., 0),
        ("wide_wave_400", 400, 40., 1),
        ("consumed_square", 4, 160., 2),
        ("retraced_200", 200, 12., 3),
        ("dashed_wave", 120, 10., 1),
    ] {
        let pts: Vec<_> = (0..n)
            .map(|i| {
                let t = i as f64 / (n - 1) as f64;
                match kind {
                    0 => {
                        let a = t * std::f64::consts::TAU;
                        [100. * (3. * a).sin(), 100. * (2. * a).sin()]
                    }
                    1 => [t * 600., 30. * (t * 30.).sin()],
                    2 => [[0., 0.], [100., 0.], [100., 100.], [0., 100.]][i],
                    _ => [if i % 2 == 0 { 0. } else { 100. }, (i / 2) as f64],
                }
            })
            .collect();
        let path = BezierPath::from_polyline(&pts, kind == 2).unwrap();
        let opts = StrokeOptions {
            width,
            join: LineJoin::Round,
            cap: LineCap::Round,
            dash: if name == "dashed_wave" {
                Some(vec![10., 4.])
            } else {
                None
            },
            ..Default::default()
        };
        let mut times = vec![];
        let mut last = None;
        let mut error = None;
        for i in 0..18 {
            let now = Instant::now();
            match tessellate_stroke(&path, &opts, 0.05) {
                Ok(m) => {
                    if i >= 3 {
                        times.push(now.elapsed().as_secs_f64() * 1e6)
                    }
                    last = Some(black_box(m));
                }
                Err(e) => {
                    error = Some(e.to_string());
                    break;
                }
            }
        }
        times.sort_by(f64::total_cmp);
        results.push(if let Some(m)=last {let area:f64=m.indices.chunks_exact(3).map(|t|cross(m.positions[t[0] as usize],m.positions[t[1] as usize],m.positions[t[2] as usize])*0.5).sum();json!({"case":name,"median_us":times[times.len()/2],"samples_us":times,"vertices":m.positions.len(),"triangles":m.triangle_count(),"area":area})}else{json!({"case":name,"error":error})});
    }
    let mut failures = vec![];
    let mut probes = 0;
    let mut cases = 0;
    let oracle_cases = if std::env::args().any(|arg| arg == "--benchmark-only") { 0 } else { 1024 };
    for case in 0..oracle_cases {
        let mut seed = 42 + case as u64;
        let scale = [0.000001, 0.001, 1., 1000., 1000000.][case % 5];
        let origin = if case % 5 == 0 { 1e7 } else { 0. };
        let n = 3 + case % 10;
        let mut pts: Vec<_> = (0..n)
            .map(|_| {
                [
                    (random(&mut seed) * 80. - 40.) * scale + origin,
                    (random(&mut seed) * 80. - 40.) * scale + origin,
                ]
            })
            .collect();
        if case % 7 == 0 {
            pts[2] = pts[0];
        }
        let closed = case % 2 == 0;
        let width = (2. + random(&mut seed) * 70.) * scale;
        let tol = 0.01 * scale;
        let path = BezierPath::from_polyline(&pts, closed).unwrap();
        let opts = StrokeOptions {
            width,
            join: LineJoin::Round,
            cap: LineCap::Round,
            ..Default::default()
        };
        let mesh = match tessellate_stroke(&path, &opts, tol) {
            Ok(m) => m,
            Err(e) => {
                failures.push(json!({"case":case,"error":e.to_string()}));
                continue;
            }
        };
        cases += 1;
        if !mesh.positions.iter().flatten().all(|x| x.is_finite())
            || mesh
                .indices
                .iter()
                .any(|&i| i as usize >= mesh.positions.len())
        {
            failures.push(json!({"case":case,"error":"invalid mesh"}));
            continue;
        }
        for _ in 0..256 {
            let p = [
                (random(&mut seed) * 160. - 80.) * scale + origin,
                (random(&mut seed) * 160. - 80.) * scale + origin,
            ];
            let d = (0..if closed { n } else { n - 1 })
                .map(|i| distance(p, pts[i], pts[(i + 1) % n]))
                .fold(f64::INFINITY, f64::min);
            if (d - width * 0.5).abs() < tol * 2. {
                continue;
            }
            probes += 1;
            let count = hits(&mesh, p);
            if count != usize::from(d < width * 0.5) {
                failures
                    .push(json!({"case":case,"point":p,"distance":d,"width":width,"hits":count}));
                break;
            }
        }
    }
    println!("{}",serde_json::to_string_pretty(&json!({"benchmarks":results,"oracle":{"cases":cases,"probes":probes,"failures":failures}})).unwrap());
    if !failures.is_empty() {
        std::process::exit(1)
    }
}
