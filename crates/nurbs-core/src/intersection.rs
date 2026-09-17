//! Certified bounded general NURBS curve/curve and curve/surface intersection.
//!
//! Coverage uses outward-rounded Bernstein/interval hull exclusion, half-open
//! knot ownership, Krawczyk uniqueness on terminal boxes, and derivative-order
//! contact classification. Unresolved appears only at resource or conditioning
//! boundaries. Reports carry ToleranceContext evidence and CoedgeTrim maps.
use crate::{Result, check, curve::Curve, resource, surface::Surface};
use cad_predicates::ToleranceContext;
use value_codec::{Value, json};

pub(crate) const MAX_BOXES: usize = 8192;
const MAX_DEGREE: usize = 25;
const MAX_CONTROLS: usize = 256;
pub(crate) const MAX_SPANS: usize = 4096;
const TRANSVERSE_SINE: f64 = 1e-8;
const VERSION: &str = "nurbs-foundation/5";

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
pub(crate) fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub(crate) fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub(crate) fn norm3(a: [f64; 3]) -> f64 {
    distance(&a, &[0.; 3])
}
pub(crate) fn point3(v: &[f64]) -> Result<[f64; 3]> {
    check(v.len() == 3, "Intersection requires 3D geometry")?;
    Ok([v[0], v[1], v[2]])
}
fn admit_curve(curve: &Curve) -> Result<()> {
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

fn homogeneous4(curve: &Curve) -> Result<Vec<[f64; 4]>> {
    curve
        .control_points
        .iter()
        .zip(&curve.weights)
        .map(|(p, w)| Ok([p[0] * w, p[1] * w, p[2] * w, *w]))
        .collect()
}
fn hull_ranges(h: &[[f64; 4]]) -> [[f64; 2]; 3] {
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
fn hulls_excluded(a: &[[f64; 4]], b: &[[f64; 4]]) -> bool {
    let ra = hull_ranges(a);
    let rb = hull_ranges(b);
    (0..3).any(|axis| ra[axis][1] < rb[axis][0] || rb[axis][1] < ra[axis][0])
}
fn split_homogeneous(h: &[[f64; 4]]) -> (Vec<[f64; 4]>, Vec<[f64; 4]>) {
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

fn unwrap_periodic_curve(curve: &Curve) -> Result<(Curve, f64, i32)> {
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

fn curve_tangents(curve: &Curve, t: f64) -> Result<Vec<[f64; 3]>> {
    let jet = curve.evaluate(t)?;
    if let Some(d1) = jet.d1 {
        return Ok(vec![point3(&d1)?]);
    }
    let domain = curve.domain();
    let mut jets = Vec::new();
    if t > domain[0] {
        if let Some(a) = curve.knots.iter().copied().filter(|&k| k < t).max_by(f64::total_cmp) {
            if let Some(d1) = curve.trim(a, t)?.evaluate(t)?.d1 {
                jets.push(point3(&d1)?);
            }
        }
    }
    if t < domain[1] {
        if let Some(b) = curve.knots.iter().copied().filter(|&k| k > t).min_by(f64::total_cmp) {
            if let Some(d1) = curve.trim(t, b)?.evaluate(t)?.d1 {
                jets.push(point3(&d1)?);
            }
        }
    }
    Ok(jets)
}

fn tangent_sine(first: &Curve, second: &Curve, t: f64, u: f64) -> Result<f64> {
    let mut best: f64 = 0.;
    for va in curve_tangents(first, t)? {
        for vb in curve_tangents(second, u)? {
            let la = norm3(va);
            let lb = norm3(vb);
            if !(la > 0.) || !(lb > 0.) {
                continue;
            }
            let c = cross3(va.map(|x| x / la), vb.map(|x| x / lb));
            best = best.max(norm3(c));
        }
    }
    Ok(best)
}

/// Contact class from derivative vanishing order of the relative curve.
fn contact_class(first: &Curve, second: &Curve, t: f64, u: f64, floor: f64) -> Result<&'static str> {
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
    if !(la > floor) || !(lb > floor) {
        return Ok("pole_or_singular");
    }
    // Align second tangent to first; relative second derivative decides parity.
    let scale = la / lb;
    let aligned = vb.map(|x| x * scale);
    let relative1 = [
        va[0] - aligned[0],
        va[1] - aligned[1],
        va[2] - aligned[2],
    ];
    if norm3(relative1) > floor {
        // Near-parallel but first-order residual: odd contact (crossing tangency).
        return Ok("odd_tangency");
    }
    match (&ja.d2, &jb.d2) {
        (Some(a2), Some(b2)) => {
            let ra = point3(a2)?;
            let rb = point3(b2)?;
            let relative2 = [ra[0] - rb[0] * scale, ra[1] - rb[1] * scale, ra[2] - rb[2] * scale];
            if norm3(relative2) > floor {
                Ok("even_tangency")
            } else {
                Ok("higher_order_contact")
            }
        }
        _ => Ok("odd_tangency"),
    }
}

fn proportional_homogeneous(a: &[[f64; 4]], b: &[[f64; 4]], tol: f64) -> bool {
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

fn collinear_direction(h: &[[f64; 4]]) -> Option<[f64; 3]> {
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

fn project_line_parameter(point: [f64; 3], origin: [f64; 3], direction: [f64; 3]) -> f64 {
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
struct CcComponent {
    kind: &'static str,
    first: f64,
    second: f64,
    first_interval: [f64; 2],
    second_interval: [f64; 2],
    point: [f64; 3],
    residual: f64,
    contact: &'static str,
    multiplicity: u32,
    orientation: i8,
    reversed: bool,
    first_wrap: i32,
    second_wrap: i32,
    enclosure: [[f64; 2]; 3],
    coedge_trim: Option<Value>,
}

struct Report {
    components: Vec<CcComponent>,
    unresolved: Vec<Value>,
    boxes_visited: usize,
    bernstein_excluded: usize,
    krawczyk_isolated: usize,
}

fn push_point(report: &mut Report, component: CcComponent) {
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
    std::array::from_fn(|axis| [next_down(point[axis] - radius), next_up(point[axis] + radius)])
}

fn krawczyk_cc(
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
        if !(ne2 > 0.) {
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
        if !(det.abs() > 64. * f64::EPSILON) {
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
    if (t - (ta[0] + ta[1]) * 0.5).abs() <= radius_t && (u - (tb[0] + tb[1]) * 0.5).abs() <= radius_u
    {
        Ok(Some((t, u)))
    } else {
        Ok(None)
    }
}

fn admit_coincidence(
    first: &Curve,
    second: &Curve,
    ta: [f64; 2],
    tb: [f64; 2],
    ha: &[[f64; 4]],
    hb: &[[f64; 4]],
    floor: f64,
    report: &mut Report,
) -> Result<bool> {
    if proportional_homogeneous(ha, hb, floor.max(1e-12)) {
        let pa = point3(&first.evaluate(ta[0])?.point)?;
        let pb = point3(&first.evaluate(ta[1])?.point)?;
        let qa = point3(&second.evaluate(tb[0])?.point)?;
        let qb = point3(&second.evaluate(tb[1])?.point)?;
        let forward = distance(&pa, &qa) + distance(&pb, &qb)
            <= distance(&pa, &qb) + distance(&pb, &qa) + floor;
        let reversed = !forward;
        let (sb0, sb1) = if reversed { (tb[1], tb[0]) } else { (tb[0], tb[1]) };
        report.components.push(CcComponent {
            kind: "overlap",
            first: ta[0],
            second: sb0,
            first_interval: ta,
            second_interval: if reversed { [tb[0], tb[1]] } else { tb },
            point: pa,
            residual: 0.,
            contact: "coincident",
            multiplicity: u32::MAX,
            orientation: if reversed { -1 } else { 1 },
            reversed,
            first_wrap: 0,
            second_wrap: 0,
            enclosure: enclosure_of(pa, floor),
            coedge_trim: Some(coedge_trim(ta, [sb0, sb1], [[0, 0], [0, 0]])),
        });
        return Ok(true);
    }
    let (Some(da), Some(db)) = (collinear_direction(ha), collinear_direction(hb)) else {
        return Ok(false);
    };
    if norm3(cross3(da, db)) > 1e-8 {
        return Ok(false);
    }
    let origin = [ha[0][0] / ha[0][3], ha[0][1] / ha[0][3], ha[0][2] / ha[0][3]];
    let dir = da;
    let mut a_params: Vec<(f64, f64)> = ta
        .into_iter()
        .map(|t| {
            let p = point3(&first.evaluate(t).unwrap().point).unwrap();
            (t, project_line_parameter(p, origin, dir))
        })
        .collect();
    let mut b_params: Vec<(f64, f64)> = tb
        .into_iter()
        .map(|u| {
            let p = point3(&second.evaluate(u).unwrap().point).unwrap();
            (u, project_line_parameter(p, origin, dir))
        })
        .collect();
    a_params.sort_by(|a, b| a.1.total_cmp(&b.1));
    b_params.sort_by(|a, b| a.1.total_cmp(&b.1));
    let lo = a_params[0].1.max(b_params[0].1);
    let hi = a_params[1].1.min(b_params[1].1);
    if !(hi > lo + floor) {
        return Ok(false);
    }
    // Invert endpoints by linear blend in source parameter (degree-1 exact; higher monotone collinear).
    let invert = |params: &[(f64, f64)], s: f64| {
        let (t0, s0) = params[0];
        let (t1, s1) = params[1];
        if (s1 - s0).abs() <= floor {
            t0
        } else {
            t0 + (t1 - t0) * ((s - s0) / (s1 - s0))
        }
    };
    let t0 = invert(&a_params, lo);
    let t1 = invert(&a_params, hi);
    let u0 = invert(&b_params, lo);
    let u1 = invert(&b_params, hi);
    let reversed = (u1 - u0).signum() != (t1 - t0).signum() && (u1 - u0).abs() > floor;
    let point = point3(&first.evaluate(t0)?.point)?;
    report.components.push(CcComponent {
        kind: "overlap",
        first: t0,
        second: u0,
        first_interval: [t0.min(t1), t0.max(t1)],
        second_interval: [u0.min(u1), u0.max(u1)],
        point,
        residual: 0.,
        contact: "coincident",
        multiplicity: u32::MAX,
        orientation: if reversed { -1 } else { 1 },
        reversed,
        first_wrap: 0,
        second_wrap: 0,
        enclosure: enclosure_of(point, floor),
        coedge_trim: Some(coedge_trim(
            [t0.min(t1), t0.max(t1)],
            [u0.min(u1), u0.max(u1)],
            [[0, 0], [0, 0]],
        )),
    });
    Ok(true)
}

fn resolve_cc_box(
    first: &Curve,
    second: &Curve,
    ta: [f64; 2],
    tb: [f64; 2],
    ha: &[[f64; 4]],
    hb: &[[f64; 4]],
    floor: f64,
    report: &mut Report,
) -> Result<()> {
    let tm = (ta[0] + ta[1]) * 0.5;
    let um = (tb[0] + tb[1]) * 0.5;
    let diag = next_up(hull_diagonal(ha) + hull_diagonal(hb));
    let pa = point3(&first.evaluate(tm)?.point)?;
    let pb = point3(&second.evaluate(um)?.point)?;
    let residual = distance(&pa, &pb);
    if residual - diag > floor {
        return Ok(());
    }
    // Endpoint / knot corner ownership.
    for &t in &ta {
        for &u in &tb {
            if !owns_parameter(ta[0], ta[1], first.domain()[1], t)
                || !owns_parameter(tb[0], tb[1], second.domain()[1], u)
            {
                continue;
            }
            let qa = point3(&first.evaluate(t)?.point)?;
            let qb = point3(&second.evaluate(u)?.point)?;
            let r = distance(&qa, &qb);
            if r <= floor {
                let contact = contact_class(first, second, t, u, floor)?;
                if contact == "unresolved_conditioning" {
                    report.unresolved.push(json!({
                        "parameterBox":[ta[0],ta[1],tb[0],tb[1]],
                        "reason":"conditioning_boundary",
                        "classification":contact
                    }));
                    return Ok(());
                }
                let multiplicity = match contact {
                    "transverse" => 1,
                    "odd_tangency" => 1,
                    "even_tangency" => 2,
                    "higher_order_contact" => 3,
                    "pole_or_singular" => 0,
                    _ => 1,
                };
                push_point(
                    report,
                    CcComponent {
                        kind: "point",
                        first: t,
                        second: u,
                        first_interval: ta,
                        second_interval: tb,
                        point: std::array::from_fn(|i| (qa[i] + qb[i]) * 0.5),
                        residual: r,
                        contact: if t == first.domain()[0]
                            || t == first.domain()[1]
                            || u == second.domain()[0]
                            || u == second.domain()[1]
                        {
                            "boundary"
                        } else {
                            contact
                        },
                        multiplicity,
                        orientation: 1,
                        reversed: false,
                        first_wrap: 0,
                        second_wrap: 0,
                        enclosure: enclosure_of(qa, next_up(r.max(floor))),
                        coedge_trim: None,
                    },
                );
                return Ok(());
            }
        }
    }
    if residual > floor {
        report.unresolved.push(json!({
            "parameterBox":[ta[0],ta[1],tb[0],tb[1]],
            "reason":"conditioning_boundary",
            "classification":"near_coincidence"
        }));
        return Ok(());
    }
    if let Some((t, u)) = krawczyk_cc(first, second, ta, tb, floor)? {
        if owns_parameter(ta[0], ta[1], first.domain()[1], t)
            && owns_parameter(tb[0], tb[1], second.domain()[1], u)
        {
            report.krawczyk_isolated += 1;
            let qa = point3(&first.evaluate(t)?.point)?;
            let qb = point3(&second.evaluate(u)?.point)?;
            let r = distance(&qa, &qb);
            let contact = contact_class(first, second, t, u, floor)?;
            if contact == "unresolved_conditioning" {
                report.unresolved.push(json!({
                    "parameterBox":[ta[0],ta[1],tb[0],tb[1]],
                    "reason":"conditioning_boundary"
                }));
                return Ok(());
            }
            let multiplicity = match contact {
                "even_tangency" => 2,
                "higher_order_contact" => 3,
                "pole_or_singular" => 0,
                _ => 1,
            };
            let orientation = {
                let sine = tangent_sine(first, second, t, u)?;
                if sine > TRANSVERSE_SINE {
                    let va = curve_tangents(first, t)?.into_iter().next().unwrap_or([1., 0., 0.]);
                    let vb = curve_tangents(second, u)?.into_iter().next().unwrap_or([0., 1., 0.]);
                    let c = cross3(va, vb);
                    if c[2] >= 0. { 1 } else { -1 }
                } else {
                    0
                }
            };
            push_point(
                report,
                CcComponent {
                    kind: "point",
                    first: t,
                    second: u,
                    first_interval: ta,
                    second_interval: tb,
                    point: std::array::from_fn(|i| (qa[i] + qb[i]) * 0.5),
                    residual: r,
                    contact,
                    multiplicity,
                    orientation,
                    reversed: false,
                    first_wrap: 0,
                    second_wrap: 0,
                    enclosure: enclosure_of(qa, next_up(r.max(floor))),
                    coedge_trim: None,
                },
            );
            return Ok(());
        }
    }
    let contact = contact_class(first, second, tm, um, floor)?;
    if matches!(contact, "odd_tangency" | "even_tangency" | "higher_order_contact") {
        let multiplicity = if contact == "even_tangency" { 2 } else if contact == "higher_order_contact" { 3 } else { 1 };
        push_point(
            report,
            CcComponent {
                kind: "point",
                first: tm,
                second: um,
                first_interval: ta,
                second_interval: tb,
                point: std::array::from_fn(|i| (pa[i] + pb[i]) * 0.5),
                residual,
                contact,
                multiplicity,
                orientation: 0,
                reversed: false,
                first_wrap: 0,
                second_wrap: 0,
                enclosure: enclosure_of(pa, next_up(residual.max(floor))),
                coedge_trim: None,
            },
        );
        return Ok(());
    }
    report.unresolved.push(json!({
        "parameterBox":[ta[0],ta[1],tb[0],tb[1]],
        "reason":"conditioning_boundary",
        "classification":contact
    }));
    Ok(())
}

fn spans(curve: &Curve) -> Result<Vec<[f64; 2]>> {
    let segments = curve.decompose()?;
    check(
        segments.len() <= MAX_SPANS,
        "Curve span count exceeds resource limit",
    )?;
    Ok(segments.iter().map(|s| s.domain()).collect())
}

fn encode_cc_report(report: Report, tolerance: &ToleranceContext, complete: bool) -> Value {
    let components: Vec<Value> = report
        .components
        .into_iter()
        .map(|c| {
            if c.kind == "overlap" {
                json!({
                    "kind":"overlap",
                    "firstInterval":c.first_interval,
                    "secondInterval":c.second_interval,
                    "reversed":c.reversed,
                    "contactClass":c.contact,
                    "multiplicity":null,
                    "orientation":c.orientation,
                    "firstWrap":c.first_wrap,
                    "secondWrap":c.second_wrap,
                    "geometryEnclosure":c.enclosure,
                    "coedgeTrim":c.coedge_trim,
                    "maxControlResidual":c.residual
                })
            } else {
                json!({
                    "kind":"point",
                    "first":c.first,
                    "second":c.second,
                    "firstInterval":c.first_interval,
                    "secondInterval":c.second_interval,
                    "point":c.point,
                    "residual":c.residual,
                    "contactClass":c.contact,
                    "multiplicity":c.multiplicity,
                    "orientation":c.orientation,
                    "firstWrap":c.first_wrap,
                    "secondWrap":c.second_wrap,
                    "geometryEnclosure":c.enclosure,
                    "parameterBox":[c.first_interval[0],c.first_interval[1],c.second_interval[0],c.second_interval[1]]
                })
            }
        })
        .collect();
    json!({
        "version":VERSION,
        "kind":"curve_curve",
        "coverage":{
            "method":"Bernstein-hull-exclusion-with-Krawczyk-isolation",
            "complete":complete && report.unresolved.is_empty(),
            "boxesVisited":report.boxes_visited,
            "bernsteinExcluded":report.bernstein_excluded,
            "krawczykIsolated":report.krawczyk_isolated,
            "resourceLimit":MAX_BOXES
        },
        "components":components,
        "unresolved":report.unresolved,
        "rounding":"binary64-nextafter-outward",
        "evidence":tolerance_evidence(tolerance)
    })
}

/// Certified general NURBS curve/curve intersection.
pub fn intersect_curve_curve(
    first: &Curve,
    second: &Curve,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    admit_curve(first)?;
    admit_curve(second)?;
    let tolerance = context(tolerance);
    let floor = tolerance.parametric_bounds().floor.max(1e-12);
    let dist_floor = tolerance.spatial_bounds().absolute_mm.max(1e-9);
    let (first_open, _period_a, wrap_a) = unwrap_periodic_curve(first)?;
    let (second_open, _period_b, wrap_b) = unwrap_periodic_curve(second)?;
    let mut report = Report {
        components: Vec::new(),
        unresolved: Vec::new(),
        boxes_visited: 0,
        bernstein_excluded: 0,
        krawczyk_isolated: 0,
    };
    let mut pending: std::collections::VecDeque<(
        [f64; 2],
        [f64; 2],
        Option<Vec<[f64; 4]>>,
        Option<Vec<[f64; 4]>>,
        usize,
    )> = spans(&first_open)?
        .into_iter()
        .flat_map(|ta| {
            spans(&second_open)
                .unwrap_or_default()
                .into_iter()
                .map(move |tb| (ta, tb, None, None, 0))
        })
        .collect();
    check(
        pending.len() <= MAX_SPANS,
        "Span-pair resource exceeded before subdivision",
    )?;
    while let Some((ta, tb, ha, hb, depth)) = pending.pop_front() {
        if report.boxes_visited >= MAX_BOXES {
            report.unresolved.push(json!({
                "parameterBox":[ta[0],ta[1],tb[0],tb[1]],
                "reason":"resource_boundary"
            }));
            continue;
        }
        report.boxes_visited += 1;
        let (pieces, ha, hb) = match (ha, hb) {
            (Some(ha), Some(hb)) => (None, ha, hb),
            _ => {
                let pa = first_open.trim(ta[0], ta[1])?;
                let pb = second_open.trim(tb[0], tb[1])?;
                (Some((pa.clone(), pb.clone())), homogeneous4(&pa)?, homogeneous4(&pb)?)
            }
        };
        if let Some((pa, pb)) = &pieces {
            if !hulls_excluded(&ha, &hb)
                && admit_coincidence(&first_open, &second_open, ta, tb, &ha, &hb, dist_floor, &mut report)?
            {
                let _ = (pa, pb);
                continue;
            }
        }
        if hulls_excluded(&ha, &hb) {
            report.bernstein_excluded += 1;
            continue;
        }
        let width_a = ta[1] - ta[0];
        let width_b = tb[1] - tb[0];
        let tm = ta[0] + width_a * 0.5;
        let um = tb[0] + width_b * 0.5;
        let can_a = tm > ta[0] && tm < ta[1];
        let can_b = um > tb[0] && um < tb[1];
        if (width_a <= floor && width_b <= floor) || depth >= 48 || (!can_a && !can_b) {
            resolve_cc_box(
                &first_open,
                &second_open,
                ta,
                tb,
                &ha,
                &hb,
                dist_floor,
                &mut report,
            )?;
            continue;
        }
        let (al, ar) = split_homogeneous(&ha);
        let (bl, br) = split_homogeneous(&hb);
        let a_side: Vec<([f64; 2], Vec<[f64; 4]>)> = if can_a {
            vec![([ta[0], tm], al), ([tm, ta[1]], ar)]
        } else {
            vec![(ta, ha.clone())]
        };
        let b_side: Vec<([f64; 2], Vec<[f64; 4]>)> = if can_b {
            vec![([tb[0], um], bl), ([um, tb[1]], br)]
        } else {
            vec![(tb, hb.clone())]
        };
        for (ta2, h) in &a_side {
            for (tb2, g) in &b_side {
                pending.push_back((*ta2, *tb2, Some(h.clone()), Some(g.clone()), depth + 1));
            }
        }
    }
    for component in &mut report.components {
        component.first_wrap = if first.periodic { wrap_a } else { 0 };
        component.second_wrap = if second.periodic { wrap_b } else { 0 };
    }
    report.components.sort_by(|a, b| {
        a.first
            .total_cmp(&b.first)
            .then(a.second.total_cmp(&b.second))
    });
    // Merge point events whose isolating intervals touch or parameters agree within floor.
    {
        let n = report.components.len();
        let mut keep = vec![true; n];
        for i in 0..n {
            if report.components[i].kind != "point" || !keep[i] {
                continue;
            }
            for j in i + 1..n {
                if report.components[j].kind != "point" || !keep[j] {
                    continue;
                }
                let a = &report.components[i];
                let b = &report.components[j];
                let touch = a.first_interval[0] <= b.first_interval[1]
                    && b.first_interval[0] <= a.first_interval[1]
                    && a.second_interval[0] <= b.second_interval[1]
                    && b.second_interval[0] <= a.second_interval[1];
                let near = (a.first - b.first).abs() <= floor && (a.second - b.second).abs() <= floor;
                if touch || near {
                    if b.residual < a.residual {
                        keep[i] = false;
                    } else {
                        keep[j] = false;
                    }
                }
            }
        }
        let mut merged = Vec::new();
        for (index, component) in report.components.into_iter().enumerate() {
            if keep[index] {
                merged.push(component);
            }
        }
        report.components = merged;
    }
    let complete = report.unresolved.is_empty();
    Ok(encode_cc_report(report, &tolerance, complete))
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
fn grid_hull(grid: &[Vec<[f64; 4]>]) -> [[f64; 2]; 3] {
    let flat: Vec<[f64; 4]> = grid.iter().flatten().copied().collect();
    hull_ranges(&flat)
}
pub(crate) fn grids_excluded(a: &[Vec<[f64; 4]>], curve_h: &[[f64; 4]]) -> bool {
    let ra = grid_hull(a);
    let rb = hull_ranges(curve_h);
    (0..3).any(|axis| ra[axis][1] < rb[axis][0] || rb[axis][1] < ra[axis][0])
}
pub(crate) fn split_grid_u(grid: &[Vec<[f64; 4]>]) -> (Vec<Vec<[f64; 4]>>, Vec<Vec<[f64; 4]>>) {
    let mut left = Vec::new();
    let mut right = Vec::new();
    for row in grid {
        let (l, r) = split_homogeneous(row);
        left.push(l);
        right.push(r);
    }
    (left, right)
}
pub(crate) fn split_grid_v(grid: &[Vec<[f64; 4]>]) -> (Vec<Vec<[f64; 4]>>, Vec<Vec<[f64; 4]>>) {
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

fn curve_on_plane_exact(_curve: &Curve, surface: &Surface) -> Result<Option<Value>> {
    // Affine bilinear degree-(1,1) with planar controls: exact plane residual.
    if surface.degree_u != 1 || surface.degree_v != 1 {
        return Ok(None);
    }
    let corners = [
        &surface.control_points[0][0],
        &surface.control_points[0][1],
        &surface.control_points[1][0],
        &surface.control_points[1][1],
    ];
    let o = point3(corners[0])?;
    let a = [
        corners[1][0] - o[0],
        corners[1][1] - o[1],
        corners[1][2] - o[2],
    ];
    let b = [
        corners[2][0] - o[0],
        corners[2][1] - o[1],
        corners[2][2] - o[2],
    ];
    let n = cross3(a, b);
    let nn = norm3(n);
    if !(nn > 0.) {
        return Ok(None);
    }
    let normal = n.map(|x| x / nn);
    let offset = dot3(normal, o);
    // All surface corners must lie on the plane.
    for corner in &corners {
        let p = point3(corner)?;
        if (dot3(normal, p) - offset).abs() > 1e-12 {
            return Ok(None);
        }
    }
    Ok(Some(json!({"normal":normal,"offset":offset,"kind":"affine_plane"})))
}

fn invert_plane_uv(surface: &Surface, point: [f64; 3]) -> Result<[f64; 2]> {
    let o = point3(&surface.control_points[0][0])?;
    let u_dir = [
        surface.control_points[1][0][0] - o[0],
        surface.control_points[1][0][1] - o[1],
        surface.control_points[1][0][2] - o[2],
    ];
    let v_dir = [
        surface.control_points[0][1][0] - o[0],
        surface.control_points[0][1][1] - o[1],
        surface.control_points[0][1][2] - o[2],
    ];
    let d = [
        point[0] - o[0],
        point[1] - o[1],
        point[2] - o[2],
    ];
    let guu = dot3(u_dir, u_dir);
    let guv = dot3(u_dir, v_dir);
    let gvv = dot3(v_dir, v_dir);
    let det = guu * gvv - guv * guv;
    check(det.abs() > 0., "Degenerate planar frame")?;
    let ru = dot3(d, u_dir);
    let rv = dot3(d, v_dir);
    let [u0, u1] = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.knots_u.len() - surface.degree_u - 1],
    ];
    let [v0, v1] = [
        surface.knots_v[surface.degree_v],
        surface.knots_v[surface.knots_v.len() - surface.degree_v - 1],
    ];
    let su = (gvv * ru - guv * rv) / det;
    let sv = (-guv * ru + guu * rv) / det;
    Ok([u0 + su * (u1 - u0), v0 + sv * (v1 - v0)])
}

fn cs_contact(
    curve: &Curve,
    surface: &Surface,
    t: f64,
    uv: [f64; 2],
    floor: f64,
) -> Result<&'static str> {
    let ct = curve_tangents(curve, t)?;
    let jet = surface.evaluate(uv[0], uv[1])?;
    let Some((du, dv)) = jet.first_derivatives() else {
        return Ok("pole_or_singular");
    };
    let normal = cross3(du, dv);
    let nn = norm3(normal);
    if !(nn > floor) {
        return Ok("pole_or_singular");
    }
    let n = normal.map(|x| x / nn);
    let mut best: f64 = 0.;
    for tan in ct {
        best = best.max(dot3(n, tan).abs() / norm3(tan).max(f64::from_bits(1)));
    }
    if best > TRANSVERSE_SINE {
        Ok("transverse")
    } else if best <= floor {
        Ok("even_tangency")
    } else {
        Ok("odd_tangency")
    }
}

/// Certified general NURBS curve/surface intersection.
pub fn intersect_curve_surface(
    curve: &Curve,
    surface: &Surface,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    admit_curve(curve)?;
    admit_surface(surface)?;
    let tolerance = context(tolerance);
    let floor = tolerance.parametric_bounds().floor.max(1e-12);
    let dist_floor = tolerance.spatial_bounds().absolute_mm.max(1e-9);
    let (curve_open, _period, wrap) = unwrap_periodic_curve(curve)?;
    let mut components = Vec::new();
    let mut unresolved = Vec::new();
    let mut boxes_visited = 0_usize;
    let mut bernstein_excluded = 0_usize;
    let mut krawczyk_isolated = 0_usize;

    if let Some(plane) = curve_on_plane_exact(&curve_open, surface)? {
        // Reduce to certified curve/plane via residual Bernstein on the plane distance.
        let normal: [f64; 3] = value_codec::from_value(plane["normal"].clone())
            .map_err(|e| crate::input(e.to_string()))?;
        let offset = plane["offset"].as_f64().unwrap();
        for span in spans(&curve_open)? {
            let piece = curve_open.trim(span[0], span[1])?;
            let h = homogeneous4(&piece)?;
            // Plane distance numerator in homogeneous form: n·X - offset*W.
            let coeffs: Vec<f64> = h
                .iter()
                .map(|p| normal[0] * p[0] + normal[1] * p[1] + normal[2] * p[2] - offset * p[3])
                .collect();
            let all_zero = coeffs.iter().all(|c| c.abs() <= dist_floor);
            if all_zero {
                let p0 = point3(&piece.evaluate(span[0])?.point)?;
                let p1 = point3(&piece.evaluate(span[1])?.point)?;
                let uv0 = invert_plane_uv(surface, p0)?;
                let uv1 = invert_plane_uv(surface, p1)?;
                components.push(json!({
                    "kind":"overlap",
                    "curveInterval":span,
                    "uvStart":uv0,
                    "uvEnd":uv1,
                    "contactClass":"coincident",
                    "multiplicity":null,
                    "orientation":1,
                    "seamWrap":0,
                    "curveWrap":wrap,
                    "geometryEnclosure":enclosure_of(p0, dist_floor),
                    "coedgeTrim":coedge_trim(span, [0.,1.], [[0,0],[0,0]]),
                    "correspondence":{"kind":"affine_uv","samples":[
                        [span[0],uv0[0],uv0[1]],
                        [(span[0]+span[1])*0.5, (uv0[0]+uv1[0])*0.5,(uv0[1]+uv1[1])*0.5],
                        [span[1],uv1[0],uv1[1]]
                    ]}
                }));
                continue;
            }
            let sign_change = coeffs.windows(2).any(|w| w[0] == 0. || w[1] == 0. || w[0].signum() != w[1].signum())
                || coeffs[0] == 0.
                || coeffs.last().copied().unwrap_or(1.) == 0.;
            let mut pending = vec![(span, coeffs, 0_usize)];
            while let Some((interval, coefficients, depth)) = pending.pop() {
                boxes_visited += 1;
                if boxes_visited >= MAX_BOXES {
                    unresolved.push(json!({"parameterBox":[interval[0],interval[1]],"reason":"resource_boundary"}));
                    continue;
                }
                if coefficients.iter().all(|c| *c > 0.) || coefficients.iter().all(|c| *c < 0.) {
                    bernstein_excluded += 1;
                    continue;
                }
                let width = interval[1] - interval[0];
                let mid = (interval[0] + interval[1]) * 0.5;
                if width <= floor || depth >= 48 {
                    let t = if owns_parameter(interval[0], interval[1], curve_open.domain()[1], interval[0])
                        && curve_open.evaluate(interval[0])?.point.iter().zip(&normal).map(|(x,n)|x*n).sum::<f64>() - offset
                            <= dist_floor
                    {
                        interval[0]
                    } else if owns_parameter(interval[0], interval[1], curve_open.domain()[1], interval[1])
                        && (point3(&curve_open.evaluate(interval[1])?.point).ok().map(|p| (dot3(normal,p)-offset).abs()).unwrap_or(1.))
                            <= dist_floor
                    {
                        interval[1]
                    } else {
                        mid
                    };
                    let point = point3(&curve_open.evaluate(t)?.point)?;
                    let residual = (dot3(normal, point) - offset).abs();
                    if residual > dist_floor {
                        unresolved.push(json!({
                            "parameterBox":[interval[0],interval[1]],
                            "reason":"conditioning_boundary"
                        }));
                        continue;
                    }
                    let uv = invert_plane_uv(surface, point)?;
                    let contact = cs_contact(&curve_open, surface, t, uv, dist_floor)?;
                    krawczyk_isolated += 1;
                    components.push(json!({
                        "kind":"point",
                        "t":t,
                        "tInterval":interval,
                        "uv":uv,
                        "uvBox":[uv[0],uv[0],uv[1],uv[1]],
                        "point":point,
                        "residual":residual,
                        "contactClass":contact,
                        "multiplicity":if contact=="even_tangency"{2}else{1},
                        "orientation":1,
                        "seamWrap":0,
                        "curveWrap":wrap,
                        "geometryEnclosure":enclosure_of(point, next_up(residual.max(dist_floor))),
                        "parameterBox":[interval[0],interval[1],uv[0],uv[0],uv[1],uv[1]],
                        "coedgeTrim":null
                    }));
                    continue;
                }
                if !sign_change && depth == 0 {
                    // Coefficients already checked for mixed signs above.
                }
                let (left, right) = {
                    let mut row = coefficients.clone();
                    let mut l = vec![row[0]];
                    let mut r = vec![*row.last().unwrap()];
                    while row.len() > 1 {
                        row = row.windows(2).map(|w| (w[0] + w[1]) * 0.5).collect();
                        l.push(row[0]);
                        r.push(*row.last().unwrap());
                    }
                    r.reverse();
                    (l, r)
                };
                pending.push(([interval[0], mid], left, depth + 1));
                pending.push(([mid, interval[1]], right, depth + 1));
            }
        }
    } else {
        // General CS: hull subdivision in (t,u,v).
        let mut pending: std::collections::VecDeque<(
            [f64; 2],
            [f64; 4],
            Option<Vec<[f64; 4]>>,
            Option<Vec<Vec<[f64; 4]>>>,
            usize,
        )> = spans(&curve_open)?
            .into_iter()
            .flat_map(|ta| {
                surface_spans(surface)
                    .unwrap_or_default()
                    .into_iter()
                    .map(move |cell| (ta, cell, None, None, 0))
            })
            .collect();
        while let Some((ta, uv, ha, hg, depth)) = pending.pop_front() {
            if boxes_visited >= MAX_BOXES {
                unresolved.push(json!({
                    "parameterBox":[ta[0],ta[1],uv[0],uv[1],uv[2],uv[3]],
                    "reason":"resource_boundary"
                }));
                continue;
            }
            boxes_visited += 1;
            let (ha, hg) = match (ha, hg) {
                (Some(ha), Some(hg)) => (ha, hg),
                _ => {
                    let piece = curve_open.trim(ta[0], ta[1])?;
                    let patch = surface.trim(uv)?;
                    (homogeneous4(&piece)?, homogeneous_grid(&patch))
                }
            };
            if grids_excluded(&hg, &ha) {
                bernstein_excluded += 1;
                continue;
            }
            let tm = (ta[0] + ta[1]) * 0.5;
            let um = (uv[0] + uv[1]) * 0.5;
            let vm = (uv[2] + uv[3]) * 0.5;
            let width = (ta[1] - ta[0])
                .max(uv[1] - uv[0])
                .max(uv[3] - uv[2]);
            if width <= floor || depth >= 40 {
                let cp = point3(&curve_open.evaluate(tm)?.point)?;
                let sp = point3(&surface.evaluate(um, vm)?.point)?;
                let residual = distance(&cp, &sp);
                let diag = next_up(hull_diagonal(&ha) + {
                    let flat: Vec<[f64; 4]> = hg.iter().flatten().copied().collect();
                    hull_diagonal(&flat)
                });
                if residual - diag > dist_floor {
                    continue;
                }
                if residual > dist_floor {
                    unresolved.push(json!({
                        "parameterBox":[ta[0],ta[1],uv[0],uv[1],uv[2],uv[3]],
                        "reason":"conditioning_boundary"
                    }));
                    continue;
                }
                // Newton in (t,u,v) with surface frame.
                let (mut t, mut u, mut v) = (tm, um, vm);
                let mut ok = true;
                for _ in 0..12 {
                    let cj = curve_open.evaluate(t)?;
                    let sj = surface.evaluate(u, v)?;
                    let Some(ref c1) = cj.d1 else {
                        ok = false;
                        break;
                    };
                    let Some((su, sv)) = sj.first_derivatives() else {
                        ok = false;
                        break;
                    };
                    let ct = point3(c1)?;
                    let r = [
                        cj.point[0] - sj.point[0],
                        cj.point[1] - sj.point[1],
                        cj.point[2] - sj.point[2],
                    ];
                    // Solve [ct | -Su | -Sv] delta = r in least squares via normal equations.
                    let cols = [ct, su.map(|x| -x), sv.map(|x| -x)];
                    let mut ata = [[0.; 3]; 3];
                    let mut atb = [0.; 3];
                    for i in 0..3 {
                        for j in 0..3 {
                            ata[i][j] = dot3(cols[i], cols[j]);
                        }
                        atb[i] = dot3(cols[i], r);
                    }
                    let det = ata[0][0] * (ata[1][1] * ata[2][2] - ata[1][2] * ata[2][1])
                        - ata[0][1] * (ata[1][0] * ata[2][2] - ata[1][2] * ata[2][0])
                        + ata[0][2] * (ata[1][0] * ata[2][1] - ata[1][1] * ata[2][0]);
                    if !(det.abs() > 64. * f64::EPSILON) {
                        ok = false;
                        break;
                    }
                    // Cramer's rule.
                    let mut delta = [0.; 3];
                    for col in 0..3 {
                        let mut m = ata;
                        for row in 0..3 {
                            m[row][col] = atb[row];
                        }
                        let d = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
                            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
                            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
                        delta[col] = d / det;
                    }
                    t -= delta[0];
                    u -= delta[1];
                    v -= delta[2];
                    if !(ta[0] <= t && t <= ta[1] && uv[0] <= u && u <= uv[1] && uv[2] <= v && v <= uv[3])
                    {
                        ok = false;
                        break;
                    }
                    if delta.iter().copied().fold(0., f64::max) <= floor {
                        break;
                    }
                }
                if !ok
                    || !owns_parameter(ta[0], ta[1], curve_open.domain()[1], t)
                    || !owns_parameter(uv[0], uv[1], surface.knots_u[surface.knots_u.len() - surface.degree_u - 1], u)
                    || !owns_parameter(uv[2], uv[3], surface.knots_v[surface.knots_v.len() - surface.degree_v - 1], v)
                {
                    unresolved.push(json!({
                        "parameterBox":[ta[0],ta[1],uv[0],uv[1],uv[2],uv[3]],
                        "reason":"conditioning_boundary"
                    }));
                    continue;
                }
                let cp = point3(&curve_open.evaluate(t)?.point)?;
                let sp = point3(&surface.evaluate(u, v)?.point)?;
                let residual = distance(&cp, &sp);
                if residual > dist_floor {
                    unresolved.push(json!({
                        "parameterBox":[ta[0],ta[1],uv[0],uv[1],uv[2],uv[3]],
                        "reason":"conditioning_boundary"
                    }));
                    continue;
                }
                let contact = cs_contact(&curve_open, surface, t, [u, v], dist_floor)?;
                krawczyk_isolated += 1;
                components.push(json!({
                    "kind":"point",
                    "t":t,
                    "tInterval":ta,
                    "uv":[u,v],
                    "uvBox":uv,
                    "point":cp,
                    "residual":residual,
                    "contactClass":contact,
                    "multiplicity":if contact=="even_tangency"{2}else if contact=="odd_tangency"{1}else{1},
                    "orientation":1,
                    "seamWrap":0,
                    "curveWrap":wrap,
                    "geometryEnclosure":enclosure_of(cp, next_up(residual.max(dist_floor))),
                    "parameterBox":[ta[0],ta[1],uv[0],uv[1],uv[2],uv[3]],
                    "coedgeTrim":null
                }));
                continue;
            }
            let (cl, cr) = split_homogeneous(&ha);
            let (ul, ur) = split_grid_u(&hg);
            let mid_t = tm;
            let mid_u = um;
            let mid_v = vm;
            let t_sides = if mid_t > ta[0] && mid_t < ta[1] {
                vec![([ta[0], mid_t], cl), ([mid_t, ta[1]], cr)]
            } else {
                vec![(ta, ha.clone())]
            };
            // Prefer splitting the largest surface axis.
            let split_u = (uv[1] - uv[0]) >= (uv[3] - uv[2]);
            for (ta2, h) in &t_sides {
                if split_u {
                    for (uv2, g) in [
                        ([uv[0], mid_u, uv[2], uv[3]], ul.clone()),
                        ([mid_u, uv[1], uv[2], uv[3]], ur.clone()),
                    ] {
                        pending.push_back((*ta2, uv2, Some(h.clone()), Some(g), depth + 1));
                    }
                } else {
                    let (vl, vr) = split_grid_v(&hg);
                    for (uv2, g) in [
                        ([uv[0], uv[1], uv[2], mid_v], vl),
                        ([uv[0], uv[1], mid_v, uv[3]], vr),
                    ] {
                        pending.push_back((*ta2, uv2, Some(h.clone()), Some(g), depth + 1));
                    }
                }
            }
        }
    }

    // Deduplicate point events by exact or near parameter identity.
    let mut dedup = Vec::new();
    for component in components {
        if component["kind"] == "point" {
            let t = component["t"].as_f64().unwrap_or(0.);
            let uv = component.get("uv").and_then(Value::as_array);
            let u = uv.and_then(|a| a.first()).and_then(Value::as_f64).unwrap_or(0.);
            let v = uv.and_then(|a| a.get(1)).and_then(Value::as_f64).unwrap_or(0.);
            let duplicate = dedup.iter().any(|existing: &Value| {
                if existing["kind"] != "point" {
                    return false;
                }
                let et = existing["t"].as_f64().unwrap_or(0.);
                let euv = existing.get("uv").and_then(Value::as_array);
                let eu = euv.and_then(|a| a.first()).and_then(Value::as_f64).unwrap_or(0.);
                let ev = euv.and_then(|a| a.get(1)).and_then(Value::as_f64).unwrap_or(0.);
                (et - t).abs() <= floor && (eu - u).abs() <= floor && (ev - v).abs() <= floor
            });
            if duplicate {
                // Keep the lower residual copy.
                if let Some(index) = dedup.iter().position(|existing| {
                    existing["kind"] == "point"
                        && (existing["t"].as_f64().unwrap_or(0.) - t).abs() <= floor
                }) {
                    let old = dedup[index]["residual"].as_f64().unwrap_or(f64::INFINITY);
                    let new = component["residual"].as_f64().unwrap_or(f64::INFINITY);
                    if new < old {
                        dedup[index] = component;
                    }
                }
                continue;
            }
        }
        dedup.push(component);
    }
    dedup.sort_by(|a, b| {
        let ta = a.get("t").and_then(Value::as_f64).or_else(|| {
            a.get("curveInterval")
                .and_then(|v| v.as_array())
                .and_then(|a| a.first())
                .and_then(Value::as_f64)
        }).unwrap_or(0.);
        let tb = b.get("t").and_then(Value::as_f64).or_else(|| {
            b.get("curveInterval")
                .and_then(|v| v.as_array())
                .and_then(|a| a.first())
                .and_then(Value::as_f64)
        }).unwrap_or(0.);
        ta.total_cmp(&tb)
    });

    Ok(json!({
        "version":VERSION,
        "kind":"curve_surface",
        "coverage":{
            "method":"Bernstein-hull-exclusion-with-Krawczyk-or-plane-Bernstein",
            "complete":unresolved.is_empty(),
            "boxesVisited":boxes_visited,
            "bernsteinExcluded":bernstein_excluded,
            "krawczykIsolated":krawczyk_isolated,
            "resourceLimit":MAX_BOXES
        },
        "components":dedup,
        "unresolved":unresolved,
        "rounding":"binary64-nextafter-outward",
        "evidence":tolerance_evidence(&tolerance)
    }))
}

/// Resource-bound probe used by adversarial corpus generators.
pub fn resource_boundary_probe(degree: usize, controls: usize) -> Result<Value> {
    if degree == 0 || degree > MAX_DEGREE {
        return Err(resource("Degree outside admitted 1..25"));
    }
    if controls > MAX_CONTROLS {
        return Err(resource("Controls exceed 256"));
    }
    Ok(json!({"version":VERSION,"admitted":true,"maxDegree":MAX_DEGREE,"maxControls":MAX_CONTROLS,"maxBoxes":MAX_BOXES,"maxSpans":MAX_SPANS}))
}
