use super::*;

pub(crate) fn context(value: Option<ToleranceContext>) -> ToleranceContext {
    value.unwrap_or_else(ToleranceContext::default_valid)
}
pub(crate) fn distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

pub(crate) fn norm3(a: [f64; 3]) -> f64 {
    distance(&a, &[0.; 3])
}
pub(crate) fn point3(v: &[f64]) -> Result<[f64; 3]> {
    check(v.len() == 3, "Intersection requires 3D geometry")?;
    Ok([v[0], v[1], v[2]])
}
pub(super) fn admit_curve(curve: &Curve) -> Result<()> {
    curve.validate()?;
    check(
        (1..=MAX_DEGREE).contains(&curve.degree),
        "Admitted intersection degree is 1..25",
    )?;
    check(
        curve.control_points.len() <= MAX_CONTROLS,
        "Curve exceeds 256 controls",
    )?;
    check(
        curve.weights.iter().all(|w| *w > 0. && *w <= 1e12),
        "Intersection requires positive weights in (0,1e12]",
    )?;
    check(
        curve.control_points[0].len() == 3,
        "Intersection requires 3D curves",
    )?;
    Ok(())
}
pub(crate) fn admit_surface(surface: &Surface) -> Result<()> {
    surface.validate()?;
    check(
        (1..=MAX_DEGREE).contains(&surface.degree_u)
            && (1..=MAX_DEGREE).contains(&surface.degree_v),
        "Admitted surface degrees are 1..25",
    )?;
    let controls = surface.control_points.len() * surface.control_points[0].len();
    check(controls <= MAX_CONTROLS, "Surface exceeds 256 controls")?;
    check(
        surface
            .weights
            .iter()
            .flatten()
            .all(|w| *w > 0. && *w <= 1e12),
        "Intersection requires positive surface weights",
    )?;
    Ok(())
}

/// Half-open ownership: [lo, hi) owns interior faces; the active domain end owns hi.
pub(crate) fn owns_parameter(lo: f64, hi: f64, domain_hi: f64, x: f64) -> bool {
    if x == domain_hi && hi == domain_hi {
        return true;
    }
    lo <= x && x < hi || (x == hi && hi == domain_hi)
}

pub(super) fn homogeneous4(curve: &Curve) -> Result<Vec<[f64; 4]>> {
    curve
        .control_points
        .iter()
        .zip(&curve.weights)
        .map(|(p, w)| Ok([p[0] * w, p[1] * w, p[2] * w, *w]))
        .collect()
}
pub(super) fn hull_ranges(h: &[[f64; 4]]) -> [[f64; 2]; 3] {
    std::array::from_fn(|axis| {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for p in h {
            let x = p[axis] / p[3];
            lo = lo.min(next_down(x));
            hi = hi.max(next_up(x));
        }
        [lo, hi]
    })
}
// Exclusion must enclose original authored curves, not rounded trim controls.
pub(super) fn curve_box(curve: &Curve, range: [f64; 2]) -> Result<Vec<crate::distance_bounds::Interval>> {
    let span = (curve.degree..curve.control_points.len())
        .find(|&i| {
            curve.knots[i] < curve.knots[i + 1]
                && curve.knots[i] <= range[0]
                && range[1] <= curve.knots[i + 1]
        })
        .ok_or_else(|| crate::input("Intersection box must lie inside one original knot span"))?;
    crate::curve_distance::enclosure(
        curve,
        span,
        crate::distance_bounds::Interval::new(range[0], range[1])?,
    )
}
pub(super) fn curves_excluded(first: &Curve, second: &Curve, ta: [f64; 2], tb: [f64; 2]) -> Result<bool> {
    let a = curve_box(first, ta)?;
    let b = curve_box(second, tb)?;
    Ok(a.iter().zip(&b).any(|(a, b)| a.hi < b.lo || b.hi < a.lo))
}
pub(super) fn split_homogeneous(h: &[[f64; 4]]) -> (Vec<[f64; 4]>, Vec<[f64; 4]>) {
    let mut row = h.to_vec();
    let mut left = vec![row[0]];
    let mut right = vec![*row.last().unwrap()];
    while row.len() > 1 {
        row = row
            .windows(2)
            .map(|p| std::array::from_fn(|i| (p[0][i] + p[1][i]) * 0.5))
            .collect();
        left.push(row[0]);
        right.push(*row.last().unwrap());
    }
    right.reverse();
    (left, right)
}
pub(crate) fn hull_diagonal(h: &[[f64; 4]]) -> f64 {
    let r = hull_ranges(h);
    next_up(
        (0..3)
            .map(|axis| {
                let w = r[axis][1] - r[axis][0];
                w * w
            })
            .sum::<f64>()
            .sqrt(),
    )
}

pub(super) fn unwrap_periodic_curve(curve: &Curve) -> Result<(Curve, f64, i32)> {
    if !curve.periodic {
        return Ok((curve.clone(), 0., 0));
    }
    let [a, b] = curve.domain();
    let period = b - a;
    check(period > 0., "Periodic curve needs positive period")?;
    // Materialize one fundamental period as a non-periodic open cover.
    let open = curve.trim(a, b)?;
    let mut open = open;
    open.periodic = false;
    Ok((open, period, 1))
}

pub(super) fn curve_tangents(curve: &Curve, t: f64) -> Result<Vec<[f64; 3]>> {
    let jet = curve.evaluate(t)?;
    if let Some(d1) = jet.d1 {
        return Ok(vec![point3(&d1)?]);
    }
    let domain = curve.domain();
    let mut jets = Vec::new();
    if t > domain[0]
        && let Some(a) = curve
            .knots
            .iter()
            .copied()
            .filter(|&k| k < t)
            .max_by(f64::total_cmp)
        && let Some(d1) = curve.trim(a, t)?.evaluate(t)?.d1
    {
        jets.push(point3(&d1)?);
    }
    if t < domain[1]
        && let Some(b) = curve
            .knots
            .iter()
            .copied()
            .filter(|&k| k > t)
            .min_by(f64::total_cmp)
        && let Some(d1) = curve.trim(t, b)?.evaluate(t)?.d1
    {
        jets.push(point3(&d1)?);
    }
    Ok(jets)
}


pub(crate) fn coedge_trim(
    curve: [f64; 2],
    pcurve: [f64; 2],
    lifts: [[i32; 2]; 2],
) -> brep_topology::CoedgeTrim {
    brep_topology::CoedgeTrim {
        curve_parameter: curve,
        pcurve_parameter: pcurve,
        periodic_lift: lifts,
    }
}


pub(super) type HomogeneousGrid = Vec<Vec<[f64; 4]>>;
pub(super) type HomogeneousGridPair = (HomogeneousGrid, HomogeneousGrid);
pub(super) type CurveSurfacePending = (
    [f64; 2],
    [f64; 4],
    Option<Vec<[f64; 4]>>,
    Option<HomogeneousGrid>,
    usize,
);


pub(crate) fn enclosure_of(point: [f64; 3], radius: f64) -> [[f64; 2]; 3] {
    std::array::from_fn(|axis| {
        [
            next_down(point[axis] - radius),
            next_up(point[axis] + radius),
        ]
    })
}


pub(super) fn spans(curve: &Curve) -> Result<Vec<[f64; 2]>> {
    let segments = curve.decompose()?;
    check(
        segments.len() <= MAX_SPANS,
        "Curve span count exceeds resource limit",
    )?;
    Ok(segments.iter().map(|s| s.domain()).collect())
}


pub(crate) fn surface_spans(surface: &Surface) -> Result<Vec<[f64; 4]>> {
    let [u0, u1] = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.knots_u.len() - surface.degree_u - 1],
    ];
    let [v0, v1] = [
        surface.knots_v[surface.degree_v],
        surface.knots_v[surface.knots_v.len() - surface.degree_v - 1],
    ];
    let mut us = vec![u0];
    for &k in &surface.knots_u {
        if k > *us.last().unwrap() && k < u1 {
            us.push(k);
        }
    }
    us.push(u1);
    let mut vs = vec![v0];
    for &k in &surface.knots_v {
        if k > *vs.last().unwrap() && k < v1 {
            vs.push(k);
        }
    }
    vs.push(v1);
    let mut cells = Vec::new();
    for window_u in us.windows(2) {
        for window_v in vs.windows(2) {
            if window_u[1] > window_u[0] && window_v[1] > window_v[0] {
                cells.push([window_u[0], window_u[1], window_v[0], window_v[1]]);
            }
        }
    }
    check(cells.len() <= MAX_SPANS, "Surface cell resource exceeded")?;
    Ok(cells)
}

pub(crate) fn homogeneous_grid(surface: &Surface) -> Vec<Vec<[f64; 4]>> {
    surface
        .control_points
        .iter()
        .zip(&surface.weights)
        .map(|(points, weights)| {
            points
                .iter()
                .zip(weights)
                .map(|(p, w)| [p[0] * w, p[1] * w, p[2] * w, *w])
                .collect()
        })
        .collect()
}
pub(super) fn grid_hull(grid: &[Vec<[f64; 4]>]) -> [[f64; 2]; 3] {
    let flat: Vec<[f64; 4]> = grid.iter().flatten().copied().collect();
    hull_ranges(&flat)
}
pub(crate) fn grids_excluded(a: &[Vec<[f64; 4]>], curve_h: &[[f64; 4]]) -> bool {
    let ra = grid_hull(a);
    let rb = hull_ranges(curve_h);
    (0..3).any(|axis| ra[axis][1] < rb[axis][0] || rb[axis][1] < ra[axis][0])
}
pub(crate) fn split_grid_u(grid: &[Vec<[f64; 4]>]) -> HomogeneousGridPair {
    let mut left = Vec::new();
    let mut right = Vec::new();
    for row in grid {
        let (l, r) = split_homogeneous(row);
        left.push(l);
        right.push(r);
    }
    (left, right)
}
pub(crate) fn split_grid_v(grid: &[Vec<[f64; 4]>]) -> HomogeneousGridPair {
    let cols = grid[0].len();
    let columns: Vec<Vec<[f64; 4]>> = (0..cols)
        .map(|j| grid.iter().map(|row| row[j]).collect())
        .collect();
    let mut left_cols = Vec::new();
    let mut right_cols = Vec::new();
    for col in &columns {
        let (l, r) = split_homogeneous(col);
        left_cols.push(l);
        right_cols.push(r);
    }
    let rows = grid.len();
    let left = (0..rows)
        .map(|i| left_cols.iter().map(|col| col[i]).collect())
        .collect();
    let right = (0..rows)
        .map(|i| right_cols.iter().map(|col| col[i]).collect())
        .collect();
    (left, right)
}

pub(super) fn cs_contact(
    curve: &Curve,
    surface: &Surface,
    t: f64,
    uv: [f64; 2],
    floor: f64,
) -> Result<ContactClass> {
    let ct = curve_tangents(curve, t)?;
    let jet = surface.evaluate(uv[0], uv[1])?;
    let Some((du, dv)) = jet.first_derivatives() else {
        return Ok(ContactClass::PoleOrSingular);
    };
    let normal = cross3(du, dv);
    let nn = norm3(normal);
    if !nn.is_finite() || nn <= floor {
        return Ok(ContactClass::PoleOrSingular);
    }
    let n = normal.map(|x| x / nn);
    let mut best: f64 = 0.;
    for tan in ct {
        best = best.max(dot3(n, tan).abs() / norm3(tan).max(f64::from_bits(1)));
    }
    if best > TRANSVERSE_SINE {
        Ok(ContactClass::Transverse)
    } else if best <= floor {
        Ok(ContactClass::EvenTangency)
    } else {
        Ok(ContactClass::OddTangency)
    }
}
