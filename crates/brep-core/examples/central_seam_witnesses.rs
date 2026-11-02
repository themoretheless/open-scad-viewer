//! Rejection-only interval witnesses. Point checks never grant whole-seam G2.
use nurbs_core::{
    curve::Curve,
    interval_eval::{self, Interval as I},
    surface::Surface,
    surface_shape_operator,
};
fn lower(x: I) -> f64 {
    if x.lo <= 0. && x.hi >= 0. {
        0.
    } else {
        x.lo.abs().min(x.hi.abs())
    }
}
fn difference(a: &[[f64; 2]], b: &[[f64; 2]]) -> f64 {
    [1., -1.]
        .into_iter()
        .map(|sign| {
            a.iter()
                .zip(b)
                .map(|(x, y)| {
                    lower(
                        I::new(x[0], x[1])
                            .unwrap()
                            .sub(I::new(y[0], y[1]).unwrap().mul(I::point(sign)).unwrap())
                            .unwrap(),
                    )
                })
                .fold(0f64, f64::max)
        })
        .fold(f64::INFINITY, f64::min)
}
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    assert_eq!(args.len(), 3, "candidate.json output.json");
    let data: value_codec::Value =
        value_codec::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
    let candidate: Surface = value_codec::from_value(data["surface"].clone()).unwrap();
    let mut sites = (1..32).map(|i| i as f64 / 32.).collect::<Vec<_>>();
    for t in [1e-7, 1e-6, 1e-5, 1e-4, 1e-3, 0.01] {
        sites.push(t);
        sites.push(1. - t);
    }
    sites.sort_by(f64::total_cmp);
    let mut records = Vec::new();
    let mut used = 0usize;
    for (seam, b) in data["boundaries"].as_array().unwrap().iter().enumerate() {
        let original: Surface = value_codec::from_value(b["originalSurface"].clone()).unwrap();
        let pc: [Curve; 2] = ["originalPcurve", "candidatePcurve"]
            .map(|k| value_codec::from_value(b[k].clone()).unwrap());
        let mut rejected = false;
        let mut resolved = 0usize;
        for &t in &sites {
            let mut shapes = Vec::new();
            for k in 0..2 {
                let d = pc[k].domain();
                let u = I::point(d[0])
                    .add(
                        I::point(d[1])
                            .sub(I::point(d[0]))
                            .unwrap()
                            .mul(I::point(t))
                            .unwrap(),
                    )
                    .unwrap();
                let uv = interval_eval::evaluate_interval(&pc[k], u).unwrap();
                let rect = [[uv[0].lo, uv[0].hi], [uv[1].lo, uv[1].hi]];
                let s = if k == 0 { &original } else { &candidate };
                let query = surface_shape_operator::rectangle_bounds(
                    s,
                    rect,
                    (100000usize.saturating_sub(used)).min(10000),
                );
                if let Ok(q) = query {
                    used += q.cells;
                    shapes.push(q.shape);
                } else {
                    break;
                }
            }
            if shapes.len() != 2 {
                continue;
            }
            let (Some(n0), Some(n1), Some(a0), Some(a1)) = (
                shapes[0].normal,
                shapes[1].normal,
                shapes[0].tensor,
                shapes[1].tensor,
            ) else {
                continue;
            };
            resolved += 1;
            let normal = difference(&n0, &n1);
            let aa = a0.into_iter().flatten().collect::<Vec<_>>();
            let bb = a1.into_iter().flatten().collect::<Vec<_>>();
            let tensor = difference(&aa, &bb);
            if normal > 1e-5 || tensor > 1e-4 {
                records.push(value_codec::json!({"seam":seam,"decision":"ExceedsAtAuthoredParameters","parameterFraction":t,"normalComponentLower":normal,"tensorComponentLowerPerMm":tensor,"wholeSeamG2Qualified":false}));
                rejected = true;
                break;
            }
        }
        if !rejected {
            records.push(value_codec::json!({"seam":seam,"decision":"NoExcessWitness","resolvedPoints":resolved,"wholeSeamG2Qualified":false}));
        }
    }
    std::fs::write(&args[2],value_codec::to_string(&value_codec::json!({"seams":records,"shapeCells":used,"scope":"Fresh interval boxes at authored corresponding pcurve parameters. Rejection only; no position or whole-seam admission."})).unwrap()).unwrap();
}
