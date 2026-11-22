use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GradientMode {
    None,
    Vertical,
    Radial,
}

#[derive(Clone, Debug)]
pub struct VoronoiLatticeOptions {
    pub min: [f64; 2],
    pub max: [f64; 2],
    pub pattern: String,
    pub cell: f64,
    pub rib: f64,
    pub seed: f64,
    pub jitter: f64,
    pub overhang_angle: Option<f64>,
    pub stress_gradient: f64,
    pub gradient_mode: GradientMode,
    pub corner_radius: f64,
    pub slope: f64,
    pub frame: f64,
    pub keep_out: Vec<[f64; 4]>,
    pub max_sites: usize,
}

pub(super) fn clip_polygon(poly: Vec<[f64; 2]>, n: [f64; 2], d: f64) -> Vec<[f64; 2]> {
    let mut result = Vec::new();
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        let da = a[0] * n[0] + a[1] * n[1] - d;
        let db = b[0] * n[0] + b[1] * n[1] - d;
        if da <= 1e-9 {
            result.push(a);
        }
        if (da < 0.) != (db < 0.) {
            let t = da / (da - db);
            result.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]);
        }
    }
    result
}

pub(super) fn inset_polygon(poly: Vec<[f64; 2]>, distance: f64) -> Vec<[f64; 2]> {
    let mut result = poly.clone();
    // Clipping may remove vertices; offset the original edges, not the result.
    for i in 0..poly.len() {
        if result.is_empty() {
            break;
        }
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let len = dx.hypot(dy);
        if len < 1e-8 {
            continue;
        }
        let n = [dy / len, -dx / len];
        let boundary = a[0] * n[0] + a[1] * n[1] - distance;
        result = clip_polygon(result, n, boundary);
    }
    result
}

pub(super) fn polygon_area(poly: &[[f64; 2]]) -> f64 {
    signed_area(poly).abs()
}

fn signed_area(poly: &[[f64; 2]]) -> f64 {
    if poly.len() < 3 {
        return 0.0;
    }
    let s = poly.iter().enumerate().fold(0.0, |acc, (i, a)| {
        let b = poly[(i + 1) % poly.len()];
        acc + a[0] * b[1] - a[1] * b[0]
    });
    s * 0.5
}

fn sanitize_polygon(poly: Vec<[f64; 2]>, eps: f64) -> Vec<[f64; 2]> {
    let mut out: Vec<[f64; 2]> = Vec::with_capacity(poly.len());
    for p in poly {
        if !p[0].is_finite() || !p[1].is_finite() {
            return Vec::new();
        }
        if let Some(last) = out.last()
            && (p[0] - last[0]).hypot(p[1] - last[1]) < eps
        {
            continue;
        }
        out.push(p);
    }
    while out.len() >= 2 {
        let first = out[0];
        let last = out[out.len() - 1];
        if (first[0] - last[0]).hypot(first[1] - last[1]) < eps {
            out.pop();
        } else {
            break;
        }
    }
    if out.len() >= 3 && signed_area(&out) < 0.0 {
        out.reverse();
    }
    out
}

fn domain_planes(min: [f64; 2], max: [f64; 2], slope: f64, frame: f64) -> [([f64; 2], f64); 4] {
    let inv = 1.0 / 1.0_f64.hypot(slope);
    [
        ([0.0, -1.0], -(min[1] + frame)),
        ([0.0, 1.0], max[1] - frame),
        ([-1.0, 0.0], -(min[0] + frame)),
        ([inv, -slope * inv], (max[0] - slope * min[1]) * inv - frame),
    ]
}

fn framed_boundary(min: [f64; 2], max: [f64; 2], slope: f64, frame: f64) -> Vec<[f64; 2]> {
    if frame == 0.0 && slope == 0.0 {
        return vec![
            [min[0], min[1]],
            [max[0], min[1]],
            [max[0], max[1]],
            [min[0], max[1]],
        ];
    }
    let h = max[1] - min[1];
    let pad = frame.abs() + h * slope.abs() + 1.0;
    let x0 = min[0] - pad;
    let y0 = min[1] - pad;
    let x1 = max[0].max(max[0] + h * slope) + pad;
    let y1 = max[1] + pad;
    let mut poly = vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]];
    for (n, d) in domain_planes(min, max, slope, frame) {
        poly = clip_polygon(poly, n, d);
        if poly.len() < 3 {
            return Vec::new();
        }
    }
    sanitize_polygon(poly, 1e-8)
}

fn bounds_of(poly: &[[f64; 2]]) -> ([f64; 2], [f64; 2]) {
    let mut min = [f64::INFINITY, f64::INFINITY];
    let mut max = [f64::NEG_INFINITY, f64::NEG_INFINITY];
    for p in poly {
        min[0] = min[0].min(p[0]);
        min[1] = min[1].min(p[1]);
        max[0] = max[0].max(p[0]);
        max[1] = max[1].max(p[1]);
    }
    (min, max)
}

fn centroid_of(poly: &[[f64; 2]]) -> [f64; 2] {
    let n = poly.len() as f64;
    let (sx, sy) = poly.iter().fold((0.0, 0.0), |acc, p| (acc.0 + p[0], acc.1 + p[1]));
    [sx / n, sy / n]
}

fn local_rib_width(p: [f64; 2], b_min: [f64; 2], b_max: [f64; 2], rib: f64, grad: f64, mode: GradientMode) -> f64 {
    if grad == 0.0 || mode == GradientMode::None {
        return rib;
    }
    let w = (b_max[0] - b_min[0]).max(1e-6);
    let h = (b_max[1] - b_min[1]).max(1e-6);
    let factor = match mode {
        GradientMode::None => 1.0,
        GradientMode::Vertical => {
            let t = ((p[1] - b_min[1]) / h).clamp(0.0, 1.0);
            (1.0 + grad * (0.12 - 0.28 * t)).max(0.45)
        }
        GradientMode::Radial => {
            let cx = 0.5 * (b_min[0] + b_max[0]);
            let cy = 0.5 * (b_min[1] + b_max[1]);
            let rx = ((p[0] - cx).abs() / (0.5 * w)).clamp(0.0, 1.0);
            let ry = ((p[1] - cy).abs() / (0.5 * h)).clamp(0.0, 1.0);
            (1.0 + grad * (-0.18 + 0.32 * rx.max(ry))).max(0.45)
        }
    };
    rib * factor
}

fn point_in_convex_polygon(poly: &[[f64; 2]], pt: [f64; 2]) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let mut pos = false;
    let mut neg = false;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        let c = (b[0] - a[0]) * (pt[1] - a[1]) - (b[1] - a[1]) * (pt[0] - a[0]);
        if c > 1e-6 {
            pos = true;
        } else if c < -1e-6 {
            neg = true;
        }
        if pos && neg {
            return false;
        }
    }
    true
}

fn segment_intersects_aabb_interior(a: [f64; 2], b: [f64; 2], r: [f64; 4]) -> bool {
    let mut t0 = 0.0_f64;
    let mut t1 = 1.0_f64;
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    for (p, q) in [(-dx, a[0] - r[0]), (dx, r[2] - a[0]), (-dy, a[1] - r[1]), (dy, r[3] - a[1])] {
        if p.abs() <= 1e-12 {
            if q < 0.0 {
                return false;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
            if t0 > t1 {
                return false;
            }
        }
    }
    if t1 - t0 <= 1e-6 {
        return false;
    }
    let tm = 0.5 * (t0 + t1);
    let mx = a[0] + tm * dx;
    let my = a[1] + tm * dy;
    mx > r[0] + 1e-6 && mx < r[2] - 1e-6 && my > r[1] + 1e-6 && my < r[3] - 1e-6
}

fn polygon_overlaps_aabb(poly: &[[f64; 2]], rect: [f64; 4]) -> bool {
    let r = [
        rect[0].min(rect[2]),
        rect[1].min(rect[3]),
        rect[0].max(rect[2]),
        rect[1].max(rect[3]),
    ];
    if r[2] - r[0] <= 1e-6 || r[3] - r[1] <= 1e-6 {
        return false;
    }
    let (p_min, p_max) = bounds_of(poly);
    if p_max[0] <= r[0] + 1e-6
        || p_min[0] >= r[2] - 1e-6
        || p_max[1] <= r[1] + 1e-6
        || p_min[1] >= r[3] - 1e-6
    {
        return false;
    }
    if poly
        .iter()
        .any(|p| p[0] > r[0] + 1e-6 && p[0] < r[2] - 1e-6 && p[1] > r[1] + 1e-6 && p[1] < r[3] - 1e-6)
    {
        return true;
    }
    let corners = [
        [r[0], r[1]],
        [r[2], r[1]],
        [r[2], r[3]],
        [r[0], r[3]],
        [0.5 * (r[0] + r[2]), 0.5 * (r[1] + r[3])],
    ];
    if corners.iter().any(|&c| point_in_convex_polygon(poly, c)) {
        return true;
    }
    (0..poly.len()).any(|i| segment_intersects_aabb_interior(poly[i], poly[(i + 1) % poly.len()], r))
}

fn clip_overhang_roof(poly: Vec<[f64; 2]>, angle_deg: f64) -> Vec<[f64; 2]> {
    if poly.len() < 3 {
        return poly;
    }
    let alpha = (angle_deg.to_radians().tan() * 0.92).atan();
    let y_top = poly.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max);
    let mut x_min_top = f64::INFINITY;
    let mut x_max_top = f64::NEG_INFINITY;
    for p in &poly {
        if p[1] >= y_top - 1e-6 {
            x_min_top = x_min_top.min(p[0]);
            x_max_top = x_max_top.max(p[0]);
        }
    }
    let x_apex = 0.5 * (x_min_top + x_max_top);
    let n_l = [-alpha.cos(), alpha.sin()];
    let d_l = n_l[0] * x_apex + n_l[1] * y_top;
    let n_r = [alpha.cos(), alpha.sin()];
    let d_r = n_r[0] * x_apex + n_r[1] * y_top;
    let clipped = clip_polygon(clip_polygon(poly, n_l, d_l), n_r, d_r);
    sanitize_polygon(clipped, 1e-6)
}

fn round_convex_polygon(poly: Vec<[f64; 2]>, radius: f64) -> Vec<[f64; 2]> {
    let poly = sanitize_polygon(poly, 1e-5);
    let n = poly.len();
    if n < 3 || radius <= 1e-4 {
        return poly;
    }
    let mut out = Vec::with_capacity(n * 4);
    for i in 0..n {
        let prev = poly[(i + n - 1) % n];
        let curr = poly[i];
        let next = poly[(i + 1) % n];
        let u = [prev[0] - curr[0], prev[1] - curr[1]];
        let v = [next[0] - curr[0], next[1] - curr[1]];
        let lu = u[0].hypot(u[1]);
        let lv = v[0].hypot(v[1]);
        if lu < 1e-5 || lv < 1e-5 {
            out.push(curr);
            continue;
        }
        let u_dir = [u[0] / lu, u[1] / lu];
        let v_dir = [v[0] / lv, v[1] / lv];
        let cos_a = (u_dir[0] * v_dir[0] + u_dir[1] * v_dir[1]).clamp(-0.99999, 0.99999);
        let angle = cos_a.acos();
        let half = 0.5 * angle;
        let tan_half = half.tan();
        if tan_half <= 1e-4 || angle > std::f64::consts::PI - 1e-3 {
            out.push(curr);
            continue;
        }
        let t = (radius / tan_half).min(0.45 * lu.min(lv));
        let eff_r = t * tan_half;
        if eff_r <= 1e-4 {
            out.push(curr);
            continue;
        }
        let p_start = [curr[0] + u_dir[0] * t, curr[1] + u_dir[1] * t];
        let p_end = [curr[0] + v_dir[0] * t, curr[1] + v_dir[1] * t];
        let bis = [u_dir[0] + v_dir[0], u_dir[1] + v_dir[1]];
        let bl = bis[0].hypot(bis[1]);
        if bl <= 1e-6 {
            out.push(curr);
            continue;
        }
        let dist = eff_r / half.sin();
        let center = [curr[0] + bis[0] / bl * dist, curr[1] + bis[1] / bl * dist];
        let a0 = (p_start[1] - center[1]).atan2(p_start[0] - center[0]);
        let a1 = (p_end[1] - center[1]).atan2(p_end[0] - center[0]);
        let mut da = a1 - a0;
        while da <= -std::f64::consts::PI {
            da += std::f64::consts::TAU;
        }
        while da > std::f64::consts::PI {
            da -= std::f64::consts::TAU;
        }
        let segs = ((eff_r * 1.6).ceil() as usize).clamp(2, 8);
        for s in 0..=segs {
            let theta = a0 + da * (s as f64 / segs as f64);
            out.push([center[0] + eff_r * theta.cos(), center[1] + eff_r * theta.sin()]);
        }
    }
    sanitize_polygon(out, 1e-5)
}

fn validate_voronoi_options(o: &VoronoiLatticeOptions) -> Result<()> {
    if !o.min.iter().chain(o.max.iter()).all(|v| v.is_finite())
        || !o.cell.is_finite()
        || o.cell <= 0.0
        || !o.rib.is_finite()
        || o.rib <= 0.0
        || !o.seed.is_finite()
        || o.seed.fract() != 0.0
        || !o.stress_gradient.is_finite()
        || !(0.0..=2.0).contains(&o.stress_gradient)
        || !o.corner_radius.is_finite()
        || o.corner_radius < 0.0
        || !o.slope.is_finite()
        || !o.frame.is_finite()
        || o.frame < 0.0
    {
        return Err(input("Lightening cells must use positive finite parameters."));
    }
    if !o.jitter.is_finite() || !(0.0..=1.0).contains(&o.jitter) {
        return Err(input("Lightening jitter must be between 0 and 1."));
    }
    if let Some(angle) = o.overhang_angle
        && (!angle.is_finite() || !(10.0..=80.0).contains(&angle))
    {
        return Err(input("Overhang angle must be between 10 and 80 degrees."));
    }
    if o.max_sites == 0 || o.keep_out.iter().flatten().any(|v| !v.is_finite()) {
        return Err(input("Voronoi lattice limits or keep-out bounds are invalid."));
    }
    Ok(())
}

fn generate_raw_cells(
    pattern: &str,
    boundary: &[ [f64; 2] ],
    b_min: [f64; 2],
    b_max: [f64; 2],
    o: &VoronoiLatticeOptions,
) -> Result<Vec<Vec<[f64; 2]>>> {
    let width = b_max[0] - b_min[0];
    let height = b_max[1] - b_min[1];
    let nx_f = (width / o.cell).ceil().max(1.0);
    let ny_f = (height / o.cell).ceil().max(1.0);
    if !nx_f.is_finite() || !ny_f.is_finite() {
        return Err(input("Lightening domain has non-finite size."));
    }
    let nx = nx_f as usize;
    let ny = ny_f as usize;
    let dx = width / nx as f64;
    let dy = height / ny as f64;
    let rect = |x: f64, y: f64, w: f64, h: f64| -> Vec<[f64; 2]> {
        vec![[x, y], [x + w, y], [x + w, y + h], [x, y + h]]
    };
    let planes = domain_planes(o.min, o.max, o.slope, o.frame);
    let clip_to_domain = |mut poly: Vec<[f64; 2]>| -> Vec<[f64; 2]> {
        for (n, d) in planes {
            poly = clip_polygon(poly, n, d);
            if poly.len() < 3 {
                return Vec::new();
            }
        }
        poly
    };
    let mut cells = Vec::new();
    match pattern {
        "grid" | "triangles" => {
            if nx.saturating_mul(ny) > o.max_sites {
                return Err(input(format!("More than {} cells. Increase cell size.", o.max_sites)));
            }
            for y in 0..ny {
                for x in 0..nx {
                    let p = rect(b_min[0] + x as f64 * dx, b_min[1] + y as f64 * dy, dx, dy);
                    let candidates = if pattern == "grid" {
                        vec![p]
                    } else {
                        vec![vec![p[0], p[1], p[2]], vec![p[0], p[2], p[3]]]
                    };
                    for tri in candidates {
                        let clipped = if o.frame > 0.0 || o.slope != 0.0 {
                            clip_to_domain(tri)
                        } else {
                            tri
                        };
                        if clipped.len() >= 3 {
                            cells.push(clipped);
                        }
                    }
                }
            }
        }
        "isogrid" => {
            let h = o.cell * 3_f64.sqrt() / 2.0;
            if !h.is_finite() || h <= 0.0 {
                return Err(input("Isogrid requires finite increasing bounds."));
            }
            let cols = (width / o.cell).ceil() + 2.0;
            let rows = (height / h).ceil() + 2.0;
            if !cols.is_finite() || !rows.is_finite() || cols * rows * 2.0 > o.max_sites as f64 {
                return Err(input(format!("More than {} cells. Increase cell size.", o.max_sites)));
            }
            let at = |i: i32, j: i32| {
                [
                    b_min[0] + (i as f64 + j as f64 * 0.5) * o.cell,
                    b_min[1] + j as f64 * h,
                ]
            };
            for j in -1_i32..rows as i32 {
                for i in -j.div_euclid(2) - 2..cols as i32 {
                    for tri in [
                        vec![at(i, j), at(i + 1, j), at(i, j + 1)],
                        vec![at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)],
                    ] {
                        if tri.iter().flatten().any(|v| !v.is_finite()) {
                            return Err(input("Isogrid exceeds finite numeric range."));
                        }
                        let polygon = clip_to_domain(tri);
                        if polygon.len() >= 3 && polygon_area(&polygon) > o.rib * o.rib / 4.0 {
                            cells.push(polygon);
                        }
                    }
                }
            }
        }
        "honeycomb" | "web" | "voronoi" => {
            let mut seed = o.seed.rem_euclid(4_294_967_296.0) as u32;
            let mut random = || {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                seed as f64 / 4_294_967_296.0
            };
            let rows = (height / (o.cell * 3_f64.sqrt() / 2.0)).ceil().max(1.0) as usize;
            if nx.saturating_mul(rows) > o.max_sites {
                return Err(input(format!("More than {} sites. Increase cell size.", o.max_sites)));
            }
            let mut sites = Vec::with_capacity(nx * rows);
            let jit = if pattern == "honeycomb" { 0.0 } else { o.jitter };
            for y in 0..rows {
                for x in 0..nx {
                    let xx = b_min[0]
                        + (x as f64 + 0.25 + (y % 2) as f64 * 0.5 + (random() - 0.5) * jit) * dx;
                    let yy = b_min[1]
                        + (y as f64 + 0.5 + (random() - 0.5) * jit) * (height / rows as f64);
                    sites.push([xx, yy]);
                }
            }
            for a in &sites {
                let mut polygon = boundary.to_vec();
                for b in &sites {
                    if a == b {
                        continue;
                    }
                    let n = [b[0] - a[0], b[1] - a[1]];
                    let d = (b[0] * b[0] + b[1] * b[1] - a[0] * a[0] - a[1] * a[1]) / 2.0;
                    polygon = clip_polygon(polygon, n, d);
                    if polygon.len() < 3 {
                        break;
                    }
                }
                if polygon.len() >= 3 {
                    cells.push(polygon);
                }
            }
        }
        _ => return Err(input("Unknown lightening pattern.")),
    }
    Ok(cells)
}

pub fn generate_voronoi_cells(options: &VoronoiLatticeOptions) -> Result<Vec<Vec<[f64; 2]>>> {
    validate_voronoi_options(options)?;
    if options.pattern == "isogrid"
        && (options.max[0] <= options.min[0] || options.max[1] <= options.min[1])
    {
        return Err(input("Isogrid requires finite increasing bounds."));
    }
    let boundary = framed_boundary(options.min, options.max, options.slope, options.frame);
    if boundary.len() < 3 {
        return Ok(Vec::new());
    }
    let (b_min, b_max) = bounds_of(&boundary);
    if b_max[0] - b_min[0] <= options.rib || b_max[1] - b_min[1] <= options.rib {
        return Ok(Vec::new());
    }
    let raw_cells = generate_raw_cells(&options.pattern, &boundary, b_min, b_max, options)?;
    let mut out = Vec::with_capacity(raw_cells.len());
    for cell in raw_cells {
        let c = centroid_of(&cell);
        let local_rib = local_rib_width(
            c,
            b_min,
            b_max,
            options.rib,
            options.stress_gradient,
            options.gradient_mode,
        );
        let mut poly = inset_polygon(cell, local_rib * 0.5);
        if poly.len() < 3 {
            continue;
        }
        if !options.keep_out.is_empty()
            && options
                .keep_out
                .iter()
                .any(|&rect| polygon_overlaps_aabb(&poly, rect))
        {
            continue;
        }
        if let Some(angle) = options.overhang_angle {
            poly = clip_overhang_roof(poly, angle);
        } else if options.corner_radius > 1e-4 {
            poly = round_convex_polygon(poly, options.corner_radius);
        } else {
            poly = sanitize_polygon(poly, 1e-6);
        }
        if poly.len() >= 3 && polygon_area(&poly) > local_rib * local_rib / 8.0 {
            out.push(poly);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_opts(pattern: &str) -> VoronoiLatticeOptions {
        VoronoiLatticeOptions {
            min: [0.0, 0.0],
            max: [100.0, 68.0],
            pattern: pattern.into(),
            cell: 16.0,
            rib: 2.4,
            seed: 1.0,
            jitter: 0.7,
            overhang_angle: None,
            stress_gradient: 0.0,
            gradient_mode: GradientMode::None,
            corner_radius: 0.0,
            slope: 0.0,
            frame: 0.0,
            keep_out: Vec::new(),
            max_sites: 400,
        }
    }

    #[test]
    fn supports_all_patterns_and_voronoi_alias() {
        let v = generate_voronoi_cells(&base_opts("voronoi")).unwrap();
        let w = generate_voronoi_cells(&base_opts("web")).unwrap();
        assert_eq!(v, w);
        assert!(v.len() >= 12);
        for pat in ["honeycomb", "isogrid", "triangles", "grid"] {
            assert!(!generate_voronoi_cells(&base_opts(pat)).unwrap().is_empty());
        }
    }

    #[test]
    fn enforces_overhang_roof_limit_across_all_edges() {
        for angle in [35.0, 45.0] {
            let mut o = base_opts("voronoi");
            o.frame = 3.0;
            o.overhang_angle = Some(angle);
            let cells = generate_voronoi_cells(&o).unwrap();
            assert!(!cells.is_empty());
            for poly in &cells {
                for i in 0..poly.len() {
                    let a = poly[i];
                    let b = poly[(i + 1) % poly.len()];
                    let dx = b[0] - a[0];
                    let dy = b[1] - a[1];
                    let len = dx.hypot(dy);
                    if len < 1e-6 {
                        continue;
                    }
                    // CCW winding: outward normal is [dy/len, -dx/len].
                    let ny = -dx / len;
                    if ny > 1e-5 {
                        let edge_angle = ny.min(1.0).asin().to_degrees();
                        assert!(edge_angle <= angle + 1e-4, "angle {edge_angle} > {angle}");
                    }
                }
            }
        }
    }

    #[test]
    fn applies_stress_gradients_keep_out_slope_and_rounding() {
        let mut vert = base_opts("voronoi");
        vert.stress_gradient = 1.5;
        vert.gradient_mode = GradientMode::Vertical;
        vert.frame = 3.0;
        let vert_cells = generate_voronoi_cells(&vert).unwrap();
        assert!(!vert_cells.is_empty());

        let mut rad = base_opts("voronoi");
        rad.stress_gradient = 1.0;
        rad.gradient_mode = GradientMode::Radial;
        rad.corner_radius = 2.0;
        rad.slope = 0.2;
        rad.keep_out = vec![[15.0, 20.0, 40.0, 68.0]];
        let rad_cells = generate_voronoi_cells(&rad).unwrap();
        assert!(!rad_cells.is_empty());
        for poly in &rad_cells {
            assert!(!polygon_overlaps_aabb(poly, [15.0, 20.0, 40.0, 68.0]));
        }
    }

    #[test]
    fn returns_empty_when_framed_domain_cannot_fit_cells() {
        let mut o = base_opts("voronoi");
        o.max = [8.0, 68.0];
        o.frame = 3.5;
        assert!(generate_voronoi_cells(&o).unwrap().is_empty());
    }
}
