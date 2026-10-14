use super::*;

pub(crate) fn next_down(x: f64) -> f64 {
    if x == f64::NEG_INFINITY || x.is_nan() {
        x
    } else if x == 0. {
        -f64::from_bits(1)
    } else {
        f64::from_bits(x.to_bits().wrapping_add(if x < 0. { 1 } else { u64::MAX }))
    }
}
pub(crate) fn next_up(x: f64) -> f64 {
    if x == f64::INFINITY || x.is_nan() {
        x
    } else if x == 0. {
        f64::from_bits(1)
    } else {
        f64::from_bits(x.to_bits().wrapping_add(if x < 0. { u64::MAX } else { 1 }))
    }
}
pub(crate) fn tolerance_evidence(context: &ToleranceContext) -> Value {
    let spatial = context.spatial_bounds();
    json!({
        "toleranceIdentity": context.spec_identity(),
        "linearAbsoluteMm": spatial.absolute_mm,
        "linearRelative": spatial.relative,
        "parametricFloor": context.parametric_bounds().floor,
        "maxEntityErrorMm": context.entity_error_bounds().maximum_mm
    })
}
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
pub(crate) use math_core::{cross as cross3, dot as dot3};
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
pub(super) fn hulls_excluded(a: &[[f64; 4]], b: &[[f64; 4]]) -> bool {
    let ra = hull_ranges(a);
    let rb = hull_ranges(b);
    (0..3).any(|axis| ra[axis][1] < rb[axis][0] || rb[axis][1] < ra[axis][0])
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

pub(super) fn tangent_sine(first: &Curve, second: &Curve, t: f64, u: f64) -> Result<f64> {
    let mut best: f64 = 0.;
    for va in curve_tangents(first, t)? {
        for vb in curve_tangents(second, u)? {
            let la = norm3(va);
            let lb = norm3(vb);
            if !la.is_finite() || !lb.is_finite() || la <= 0. || lb <= 0. {
                continue;
            }
            let c = cross3(va.map(|x| x / la), vb.map(|x| x / lb));
            best = best.max(norm3(c));
        }
    }
    Ok(best)
}

/// Contact class from derivative vanishing order of the relative curve.
pub(super) fn contact_class(
    first: &Curve,
    second: &Curve,
    t: f64,
    u: f64,
    floor: f64,
) -> Result<&'static str> {
    let sine = tangent_sine(first, second, t, u)?;
    if sine > TRANSVERSE_SINE {
        return Ok("transverse");
    }
    let ja = first.evaluate(t)?;
    let jb = second.evaluate(u)?;
    let Some(ref a1) = ja.d1 else {
        return Ok("unresolved_conditioning");
    };
    let Some(ref b1) = jb.d1 else {
        return Ok("unresolved_conditioning");
    };
    let va = point3(a1)?;
    let vb = point3(b1)?;
    let la = norm3(va);
    let lb = norm3(vb);
    if !la.is_finite() || !lb.is_finite() || la <= floor || lb <= floor {
        return Ok("pole_or_singular");
    }
    // Align second tangent to first; relative second derivative decides parity.
    let scale = la / lb;
    let aligned = vb.map(|x| x * scale);
    let relative1 = [va[0] - aligned[0], va[1] - aligned[1], va[2] - aligned[2]];
    if norm3(relative1) > floor {
        // Near-parallel but first-order residual: odd contact (crossing tangency).
        return Ok("odd_tangency");
    }
    match (&ja.d2, &jb.d2) {
        (Some(a2), Some(b2)) => {
            let ra = point3(a2)?;
            let rb = point3(b2)?;
            let relative2 = [
                ra[0] - rb[0] * scale,
                ra[1] - rb[1] * scale,
                ra[2] - rb[2] * scale,
            ];
            if norm3(relative2) > floor {
                Ok("even_tangency")
            } else {
                Ok("higher_order_contact")
            }
        }
        _ => Ok("odd_tangency"),
    }
}

pub(super) fn proportional_homogeneous(a: &[[f64; 4]], b: &[[f64; 4]], tol: f64) -> bool {
    if a.len() != b.len() || a.is_empty() {
        return false;
    }
    let mut scale = None;
    for (pa, pb) in a.iter().zip(b) {
        for axis in 0..4 {
            if pa[axis].abs() <= tol && pb[axis].abs() <= tol {
                continue;
            }
            if pa[axis].abs() <= tol || pb[axis].abs() <= tol {
                return false;
            }
            let ratio = pa[axis] / pb[axis];
            match scale {
                None => scale = Some(ratio),
                Some(s) if (ratio - s).abs() > tol.max(s.abs() * 1e-9) => return false,
                _ => {}
            }
        }
    }
    scale.is_some()
}

pub(super) fn collinear_direction(h: &[[f64; 4]]) -> Option<[f64; 3]> {
    let points: Vec<[f64; 3]> = h
        .iter()
        .map(|p| [p[0] / p[3], p[1] / p[3], p[2] / p[3]])
        .collect();
    let origin = points[0];
    let mut direction = None;
    for point in &points[1..] {
        let d = [
            point[0] - origin[0],
            point[1] - origin[1],
            point[2] - origin[2],
        ];
        if norm3(d) <= 1e-14 {
            continue;
        }
        match direction {
            None => direction = Some(d),
            Some(dir) => {
                if norm3(cross3(dir, d)) > 1e-9 * norm3(dir) * norm3(d) {
                    return None;
                }
            }
        }
    }
    direction.map(|d| {
        let n = norm3(d).max(f64::from_bits(1));
        d.map(|x| x / n)
    })
}

pub(super) fn project_line_parameter(point: [f64; 3], origin: [f64; 3], direction: [f64; 3]) -> f64 {
    dot3(
        [
            point[0] - origin[0],
            point[1] - origin[1],
            point[2] - origin[2],
        ],
        direction,
    )
}

pub(crate) fn coedge_trim(curve: [f64; 2], pcurve: [f64; 2], lifts: [[i32; 2]; 2]) -> Value {
    json!({
        "curveParameter": curve,
        "pcurveParameter": pcurve,
        "periodicLift": lifts
    })
}

#[derive(Clone)]
pub(super) struct CcComponent {
    pub(super) kind: &'static str,
    pub(super) first: f64,
    pub(super) second: f64,
    pub(super) first_interval: [f64; 2],
    pub(super) second_interval: [f64; 2],
    pub(super) point: [f64; 3],
    pub(super) residual: f64,
    pub(super) contact: &'static str,
    pub(super) multiplicity: u32,
    pub(super) orientation: i8,
    pub(super) reversed: bool,
    pub(super) first_wrap: i32,
    pub(super) second_wrap: i32,
    pub(super) enclosure: [[f64; 2]; 3],
    pub(super) coedge_trim: Option<Value>,
}

pub(super) struct Report {
    pub(super) components: Vec<CcComponent>,
    pub(super) unresolved: Vec<Value>,
    pub(super) boxes_visited: usize,
    pub(super) bernstein_excluded: usize,
    pub(super) krawczyk_isolated: usize,
}

type HomogeneousGrid = Vec<Vec<[f64; 4]>>;
pub(super) type HomogeneousGridPair = (HomogeneousGrid, HomogeneousGrid);
pub(super) type CurveSpanPending = (
    [f64; 2],
    [f64; 2],
    Option<Vec<[f64; 4]>>,
    Option<Vec<[f64; 4]>>,
    usize,
);
pub(super) type CurveSurfacePending = (
    [f64; 2],
    [f64; 4],
    Option<Vec<[f64; 4]>>,
    Option<HomogeneousGrid>,
    usize,
);

pub(super) struct CurveIntersectionBox<'a> {
    pub(super) first: &'a Curve,
    pub(super) second: &'a Curve,
    pub(super) ta: [f64; 2],
    pub(super) tb: [f64; 2],
    pub(super) ha: &'a [[f64; 4]],
    pub(super) hb: &'a [[f64; 4]],
    pub(super) floor: f64,
}

pub(super) fn push_point(report: &mut Report, component: CcComponent) {
    let duplicate = report.components.iter().any(|c| {
        c.kind == "point"
            && c.first == component.first
            && c.second == component.second
            && c.first_wrap == component.first_wrap
            && c.second_wrap == component.second_wrap
    });
    if !duplicate {
        report.components.push(component);
    }
}

pub(crate) fn enclosure_of(point: [f64; 3], radius: f64) -> [[f64; 2]; 3] {
    std::array::from_fn(|axis| {
        [
            next_down(point[axis] - radius),
            next_up(point[axis] + radius),
        ]
    })
}

pub(super) fn krawczyk_cc(
    first: &Curve,
    second: &Curve,
    ta: [f64; 2],
    tb: [f64; 2],
    floor: f64,
) -> Result<Option<(f64, f64)>> {
    let width_t = ta[1] - ta[0];
    let width_u = tb[1] - tb[0];
    if width_t.max(width_u) > floor.max(2_f64.powi(-24)) {
        return Ok(None);
    }
    let (mut t, mut u) = ((ta[0] + ta[1]) * 0.5, (tb[0] + tb[1]) * 0.5);
    for _ in 0..12 {
        let ja = first.evaluate(t)?;
        let jb = second.evaluate(u)?;
        let Some(ref a1) = ja.d1 else {
            return Ok(None);
        };
        let Some(ref b1) = jb.d1 else {
            return Ok(None);
        };
        let va = point3(a1)?;
        let vb = point3(b1)?;
        let r = [
            ja.point[0] - jb.point[0],
            ja.point[1] - jb.point[1],
            ja.point[2] - jb.point[2],
        ];
        // Project residual onto the two dominant axes of va×vb plane.
        let n = cross3(va, vb);
        let nn = norm3(n);
        if nn <= TRANSVERSE_SINE * norm3(va) * norm3(vb) {
            return Ok(None);
        }
        let e1 = va;
        let e2 = cross3(n, va);
        let ne2 = norm3(e2);
        if !ne2.is_finite() || ne2 <= 0. {
            return Ok(None);
        }
        let e2 = e2.map(|x| x / ne2);
        let e1n = norm3(e1).max(f64::from_bits(1));
        let e1 = e1.map(|x| x / e1n);
        let ft = [dot3(r, e1), dot3(r, e2)];
        let j00 = dot3(va, e1);
        let j01 = -dot3(vb, e1);
        let j10 = dot3(va, e2);
        let j11 = -dot3(vb, e2);
        let det = j00 * j11 - j01 * j10;
        if !det.is_finite() || det.abs() <= 64. * f64::EPSILON {
            return Ok(None);
        }
        let dt = (j11 * ft[0] - j01 * ft[1]) / det;
        let du = (-j10 * ft[0] + j00 * ft[1]) / det;
        t -= dt;
        u -= du;
        if !(ta[0] <= t && t <= ta[1] && tb[0] <= u && u <= tb[1]) {
            return Ok(None);
        }
        if dt.abs().max(du.abs()) <= floor {
            break;
        }
    }
    // Krawczyk contraction: image of the box under Newton stays inside.
    let radius_t = width_t * 0.45;
    let radius_u = width_u * 0.45;
    if (t - (ta[0] + ta[1]) * 0.5).abs() <= radius_t
        && (u - (tb[0] + tb[1]) * 0.5).abs() <= radius_u
    {
        Ok(Some((t, u)))
    } else {
        Ok(None)
    }
}

pub(crate) fn coedge_trim_report(curve:[f64;2],pcurve:[f64;2],lifts:[[i32;2];2])->brep_topology::CoedgeTrim { brep_topology::CoedgeTrim {curve_parameter:curve,pcurve_parameter:pcurve,periodic_lift:lifts} }
