use super::*;

pub(crate) fn invert_uv(surface: &Surface, point: [f64; 3]) -> Result<[f64; 2]> {
    let mut best = [0.5, 0.5];
    let mut best_d = f64::INFINITY;
    for i in 0..=48 {
        for j in 0..=48 {
            let u = i as f64 / 48.;
            let v = j as f64 / 48.;
            let p = surface.evaluate(u, v)?.point;
            let d =
                (p[0] - point[0]).powi(2) + (p[1] - point[1]).powi(2) + (p[2] - point[2]).powi(2);
            if d < best_d {
                best_d = d;
                best = [u, v];
            }
        }
    }
    let mut u = best[0];
    let mut v = best[1];
    let mut step = 0.04;
    for _ in 0..16 {
        let mut improved = false;
        for (du, dv) in [
            (-step, 0.),
            (step, 0.),
            (0., -step),
            (0., step),
            (-step, -step),
            (step, step),
            (-step, step),
            (step, -step),
        ] {
            let nu = (u + du).clamp(0., 1.);
            let nv = (v + dv).clamp(0., 1.);
            let p = surface.evaluate(nu, nv)?.point;
            let d =
                (p[0] - point[0]).powi(2) + (p[1] - point[1]).powi(2) + (p[2] - point[2]).powi(2);
            if d + 1e-18 < best_d {
                best_d = d;
                u = nu;
                v = nv;
                improved = true;
            }
        }
        if !improved {
            step *= 0.5;
        }
    }
    if best_d > 1e-2 {
        return Err(refuse("Failed to invert surface UV for hole vertex"));
    }
    Ok([u, v])
}

pub(crate) fn surface_aabb(surface: &Surface) -> ([f64; 3], [f64; 3]) {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for p in surface.control_points.iter().flatten() {
        for i in 0..3 {
            min[i] = min[i].min(p[i]);
            max[i] = max[i].max(p[i]);
        }
    }
    (min, max)
}

pub(crate) fn step_header(filename: &str, description: &str) -> Vec<String> {
    vec![
        "ISO-10303-21;".into(),
        "HEADER;".into(),
        format!("FILE_DESCRIPTION(('{description}'),'2;1');"),
        format!(
            "FILE_NAME('{filename}','2026-09-16',('open-scad-viewer'),(''),'nurbs-step','','');"
        ),
        "FILE_SCHEMA(('AUTOMOTIVE_DESIGN','AP242_MANAGED_MODEL_BASED_3D_ENGINEERING'));".into(),
        "ENDSEC;".into(),
        "DATA;".into(),
    ]
}
