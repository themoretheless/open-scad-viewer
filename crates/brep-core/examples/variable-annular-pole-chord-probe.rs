//! Floating candidates only: neither a wall certificate nor a certified witness.
use nurbs_core::surface::{Evaluation, SurfaceSampler};
fn normal(e: &Evaluation) -> Option<[f64; 3]> {
    let (a, b) = e.first_derivatives()?;
    let n: [f64; 3] =
        std::array::from_fn(|k| a[(k + 1) % 3] * b[(k + 2) % 3] - a[(k + 2) % 3] * b[(k + 1) % 3]);
    let scale = n.iter().map(|x| x.abs()).fold(0_f64, f64::max);
    if scale <= 0. || !scale.is_finite() {
        return None;
    }
    let n = n.map(|x| x / scale);
    let length = n.iter().map(|x| x * x).sum::<f64>().sqrt();
    Some(n.map(|x| x / length))
}
fn candidate(
    s: &SurfaceSampler,
    anchor: usize,
    u: f64,
    v: f64,
) -> Option<(f64, value_codec::Value)> {
    let v0 = 0.00018;
    let a = s.evaluate(u, v0).ok()?;
    let target = a.point[anchor];
    let (mut lo, mut hi) = (u * 0.5, u * 2.);
    let flo = s.evaluate(lo, v).ok()?.point[anchor];
    let fhi = s.evaluate(hi, v).ok()?.point[anchor];
    if target < flo.min(fhi) || target > flo.max(fhi) {
        return None;
    }
    for _ in 0..60 {
        let middle = lo + (hi - lo) * 0.5;
        if middle == lo || middle == hi {
            break;
        }
        let x = s.evaluate(middle, v).ok()?.point[anchor];
        if (x < target) == (flo < fhi) {
            lo = middle;
        } else {
            hi = middle;
        }
    }
    let other = lo + (hi - lo) * 0.5;
    let b = s.evaluate(other, v).ok()?;
    let delta: [f64; 3] = std::array::from_fn(|k| b.point[k] - a.point[k]);
    let length = delta.iter().map(|x| x * x).sum::<f64>().sqrt();
    if length <= 0. || !length.is_finite() {
        return None;
    }
    let chord = delta.map(|x| x / length);
    let normals = [normal(&a)?, normal(&b)?];
    let cosines = normals.map(|n| n.iter().zip(chord).map(|(x, y)| x * y).sum::<f64>().powi(2));
    let score = cosines[0].min(cosines[1]);
    Some((
        score,
        value_codec::json!({"parameters":[[u,v0],[other,v]],"points":[a.point,b.point],"normals":normals,"distanceMm":length,"cosineSquared":cosines,"minimumCosineSquared":score,"anchorResidualMm":b.point[anchor]-target}),
    ))
}
fn main() {
    let output = std::env::args().nth(1).expect("output JSON path");
    let mut cases = Vec::new();
    for radii in [[0.5, 1.25], [1.25, 0.5]] {
        for direction in [1., -1.] {
            let model = brep_core::circular_blend::partial_annular_variable_arc(
                20.,
                5.,
                6.,
                radii,
                [0.3, 0.7, 0.4].map(|x| x * direction),
                1e-7,
            )
            .unwrap();
            let before = value_codec::to_string(&model).unwrap();
            let surface = &model.faces[0].surface;
            let sampler = SurfaceSampler::new(surface).unwrap();
            let anchor = (0..3)
                .max_by(|a, b| {
                    (surface.control_points[1][0][*a] - surface.control_points[0][0][*a])
                        .abs()
                        .total_cmp(
                            &(surface.control_points[1][0][*b] - surface.control_points[0][0][*b])
                                .abs(),
                        )
                })
                .unwrap();
            let mut probes = Vec::new();
            for u in [0.01, 0.001, 0.0001] {
                let mut best: Option<(f64, f64, value_codec::Value)> = None;
                for i in 0..=200 {
                    let v = 0.35 + 0.1 * i as f64 / 200.;
                    if let Some((score, row)) = candidate(&sampler, anchor, u, v) {
                        if best.as_ref().is_none_or(|b| score > b.0) {
                            best = Some((score, v, row));
                        }
                    }
                }
                let mut width = 0.0005;
                for _ in 0..8 {
                    let Some(center) = best.as_ref().map(|b| b.1) else {
                        break;
                    };
                    for i in 0..=20 {
                        let v = (center - width + 2. * width * i as f64 / 20.).clamp(0., 1.);
                        if let Some((score, row)) = candidate(&sampler, anchor, u, v) {
                            if best.as_ref().is_none_or(|b| score > b.0) {
                                best = Some((score, v, row));
                            }
                        }
                    }
                    width *= 0.1;
                }
                eprintln!(
                    "radii={radii:?} direction={direction} u={u} best_min_cosine_squared={:?}",
                    best.as_ref().map(|b| b.0)
                );
                probes.push(value_codec::json!({"u":u,"candidate":best.map(|b|b.2)}));
            }
            assert_eq!(value_codec::to_string(&model).unwrap(), before);
            cases.push(value_codec::json!({"radii":radii,"direction":direction,"face":0,"anchor":anchor,"probes":probes}));
        }
    }
    std::fs::write(output,value_codec::to_string_pretty(&value_codec::json!({"schema":"cad-annular-pole-chord-probe/1","scope":"Floating candidates only; not a wall certificate or a certified thin witness. Failure to find a candidate is not proof of absence.","cases":cases})).unwrap()).unwrap();
}
