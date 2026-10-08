//! Curve/curve intersection core: coincidence admission, Krawczyk isolation,
//! contact classification, and the subdivision driver (split from the hub).
use super::*;
use crate::foundation::guards::{Budget, require_finite_point};

fn tangent_sine(first: &Curve, second: &Curve, t: f64, u: f64) -> Result<f64> {
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
fn contact_class(
    first: &Curve,
    second: &Curve,
    t: f64,
    u: f64,
    floor: f64,
) -> Result<ContactClass> {
    let sine = tangent_sine(first, second, t, u)?;
    if sine > TRANSVERSE_SINE {
        return Ok(ContactClass::Transverse);
    }
    let ja = first.evaluate(t)?;
    let jb = second.evaluate(u)?;
    let Some(ref a1) = ja.d1 else {
        return Ok(ContactClass::UnresolvedConditioning);
    };
    let Some(ref b1) = jb.d1 else {
        return Ok(ContactClass::UnresolvedConditioning);
    };
    let va = point3(a1)?;
    let vb = point3(b1)?;
    let la = norm3(va);
    let lb = norm3(vb);
    if !la.is_finite() || !lb.is_finite() || la <= floor || lb <= floor {
        return Ok(ContactClass::PoleOrSingular);
    }
    // Align second tangent to first; relative second derivative decides parity.
    let scale = la / lb;
    let aligned = vb.map(|x| x * scale);
    let relative1 = [va[0] - aligned[0], va[1] - aligned[1], va[2] - aligned[2]];
    if norm3(relative1) > floor {
        // Near-parallel but first-order residual: odd contact (crossing tangency).
        return Ok(ContactClass::OddTangency);
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
                Ok(ContactClass::EvenTangency)
            } else {
                Ok(ContactClass::HigherOrderContact)
            }
        }
        _ => Ok(ContactClass::OddTangency),
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

#[derive(Clone, Copy, Debug)]
pub enum UnresolvedReason {
    ConditioningBoundary,
    ResourceBoundary,
}
#[derive(Clone, Debug)]
pub struct UnresolvedCurveIntersection {
    pub parameter_box: [f64; 4],
    pub reason: UnresolvedReason,
    pub classification: Option<ContactClass>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContactClass {
    Transverse,
    OddTangency,
    EvenTangency,
    HigherOrderContact,
    UnresolvedConditioning,
    Coincident,
    PoleOrSingular,
    Boundary,
    NearCoincidence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurveCurveComponentKind {
    Point,
    Overlap,
}

#[derive(Clone)]
pub struct CcComponent {
    pub kind: CurveCurveComponentKind,
    pub first: f64,
    pub second: f64,
    pub first_interval: [f64; 2],
    pub second_interval: [f64; 2],
    pub point: [f64; 3],
    pub residual: f64,
    pub contact: ContactClass,
    pub multiplicity: u32,
    pub orientation: i8,
    pub reversed: bool,
    pub first_wrap: i32,
    pub second_wrap: i32,
    pub enclosure: [[f64; 2]; 3],
    pub coedge_trim: Option<brep_topology::CoedgeTrim>,
}

pub struct CurveCurveReport {
    pub components: Vec<CcComponent>,
    pub unresolved: Vec<UnresolvedCurveIntersection>,
    pub boxes_visited: usize,
    pub bernstein_excluded: usize,
    pub krawczyk_isolated: usize,
    /// Boxes excluded or shrunk by the fat-line Bézier clipping pre-filter.
    pub bezier_clipped: usize,
}

type CurveSpanPending = (
    [f64; 2],
    [f64; 2],
    Option<Vec<[f64; 4]>>,
    Option<Vec<[f64; 4]>>,
    usize,
);

struct CurveIntersectionBox<'a> {
    first: &'a Curve,
    second: &'a Curve,
    ta: [f64; 2],
    tb: [f64; 2],
    ha: &'a [[f64; 4]],
    hb: &'a [[f64; 4]],
    floor: f64,
}

fn push_point(report: &mut CurveCurveReport, component: CcComponent) {
    let duplicate = report.components.iter().any(|c| {
        c.kind == CurveCurveComponentKind::Point
            && c.first == component.first
            && c.second == component.second
            && c.first_wrap == component.first_wrap
            && c.second_wrap == component.second_wrap
    });
    if !duplicate {
        report.components.push(component);
    }
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
    // Unified guard backing the fixed 12-step Newton polish (item 1065).
    let mut guard = Budget::with_iterations(12)?.guard("cc_krawczyk_newton");
    for _ in 0..12 {
        guard.tick()?;
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
        // Isolated parameters must be finite before they enter a report (1093).
        require_finite_point(&[t, u], "krawczyk parameters")?;
        Ok(Some((t, u)))
    } else {
        Ok(None)
    }
}

fn admit_coincidence(
    input: &CurveIntersectionBox<'_>,
    report: &mut CurveCurveReport,
) -> Result<bool> {
    let CurveIntersectionBox {
        first,
        second,
        ta,
        tb,
        ha,
        hb,
        floor,
    } = *input;
    if proportional_homogeneous(ha, hb, floor.max(1e-12)) {
        let pa = point3(&first.evaluate(ta[0])?.point)?;
        let pb = point3(&first.evaluate(ta[1])?.point)?;
        let qa = point3(&second.evaluate(tb[0])?.point)?;
        let qb = point3(&second.evaluate(tb[1])?.point)?;
        let forward = distance(&pa, &qa) + distance(&pb, &qb)
            <= distance(&pa, &qb) + distance(&pb, &qa) + floor;
        let reversed = !forward;
        let (sb0, sb1) = if reversed {
            (tb[1], tb[0])
        } else {
            (tb[0], tb[1])
        };
        report.components.push(CcComponent {
            kind: CurveCurveComponentKind::Overlap,
            first: ta[0],
            second: sb0,
            first_interval: ta,
            second_interval: if reversed { [tb[0], tb[1]] } else { tb },
            point: pa,
            residual: 0.,
            contact: ContactClass::Coincident,
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
    let origin = [
        ha[0][0] / ha[0][3],
        ha[0][1] / ha[0][3],
        ha[0][2] / ha[0][3],
    ];
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
    if !hi.is_finite() || hi <= lo + floor {
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
        kind: CurveCurveComponentKind::Overlap,
        first: t0,
        second: u0,
        first_interval: [t0.min(t1), t0.max(t1)],
        second_interval: [u0.min(u1), u0.max(u1)],
        point,
        residual: 0.,
        contact: ContactClass::Coincident,
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

fn resolve_cc_box(input: &CurveIntersectionBox<'_>, report: &mut CurveCurveReport) -> Result<()> {
    let CurveIntersectionBox {
        first,
        second,
        ta,
        tb,
        ha: _,
        hb: _,
        floor,
    } = *input;
    let tm = (ta[0] + ta[1]) * 0.5;
    let um = (tb[0] + tb[1]) * 0.5;
    let a_box = curve_box(first, ta)?;
    let b_box = curve_box(second, tb)?;
    if crate::distance_bounds::box_distance(&a_box, &b_box)?.0 > floor {
        return Ok(());
    }
    let pa = point3(&first.evaluate(tm)?.point)?;
    let pb = point3(&second.evaluate(um)?.point)?;
    let residual = distance(&pa, &pb);
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
                if contact == ContactClass::UnresolvedConditioning {
                    report.unresolved.push(UnresolvedCurveIntersection {
                        parameter_box: [ta[0], ta[1], tb[0], tb[1]],
                        reason: UnresolvedReason::ConditioningBoundary,
                        classification: Some(contact),
                    });
                    return Ok(());
                }
                let multiplicity = match contact {
                    ContactClass::Transverse => 1,
                    ContactClass::OddTangency => 1,
                    ContactClass::EvenTangency => 2,
                    ContactClass::HigherOrderContact => 3,
                    ContactClass::PoleOrSingular => 0,
                    _ => 1,
                };
                push_point(
                    report,
                    CcComponent {
                        kind: CurveCurveComponentKind::Point,
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
                            ContactClass::Boundary
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
        report.unresolved.push(UnresolvedCurveIntersection {
            parameter_box: [ta[0], ta[1], tb[0], tb[1]],
            reason: UnresolvedReason::ConditioningBoundary,
            classification: Some(ContactClass::NearCoincidence),
        });
        return Ok(());
    }
    if let Some((t, u)) = krawczyk_cc(first, second, ta, tb, floor)?
        && owns_parameter(ta[0], ta[1], first.domain()[1], t)
        && owns_parameter(tb[0], tb[1], second.domain()[1], u)
    {
        report.krawczyk_isolated += 1;
        let qa = point3(&first.evaluate(t)?.point)?;
        let qb = point3(&second.evaluate(u)?.point)?;
        let r = distance(&qa, &qb);
        let contact = contact_class(first, second, t, u, floor)?;
        if contact == ContactClass::UnresolvedConditioning {
            report.unresolved.push(UnresolvedCurveIntersection {
                parameter_box: [ta[0], ta[1], tb[0], tb[1]],
                reason: UnresolvedReason::ConditioningBoundary,
                classification: None,
            });
            return Ok(());
        }
        let multiplicity = match contact {
            ContactClass::EvenTangency => 2,
            ContactClass::HigherOrderContact => 3,
            ContactClass::PoleOrSingular => 0,
            _ => 1,
        };
        let orientation = {
            let sine = tangent_sine(first, second, t, u)?;
            if sine > TRANSVERSE_SINE {
                let va = curve_tangents(first, t)?
                    .into_iter()
                    .next()
                    .unwrap_or([1., 0., 0.]);
                let vb = curve_tangents(second, u)?
                    .into_iter()
                    .next()
                    .unwrap_or([0., 1., 0.]);
                let c = cross3(va, vb);
                if c[2] >= 0. { 1 } else { -1 }
            } else {
                0
            }
        };
        push_point(
            report,
            CcComponent {
                kind: CurveCurveComponentKind::Point,
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
    let contact = contact_class(first, second, tm, um, floor)?;
    if matches!(
        contact,
        ContactClass::OddTangency | ContactClass::EvenTangency | ContactClass::HigherOrderContact
    ) {
        let multiplicity = if contact == ContactClass::EvenTangency {
            2
        } else if contact == ContactClass::HigherOrderContact {
            3
        } else {
            1
        };
        push_point(
            report,
            CcComponent {
                kind: CurveCurveComponentKind::Point,
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
    report.unresolved.push(UnresolvedCurveIntersection {
        parameter_box: [ta[0], ta[1], tb[0], tb[1]],
        reason: UnresolvedReason::ConditioningBoundary,
        classification: Some(contact),
    });
    Ok(())
}

/// Certified general NURBS curve/curve intersection.
pub struct CurveCurveIntersection {
    pub report: CurveCurveReport,
    pub tolerance: ToleranceContext,
}

pub fn intersect_curve_curve_report(
    first: &Curve,
    second: &Curve,
    tolerance: Option<ToleranceContext>,
) -> Result<CurveCurveIntersection> {
    intersect_curve_curve_bounded_report(first,second,tolerance,MAX_BOXES)
}

pub fn intersect_curve_curve_bounded_report(
    first: &Curve,
    second: &Curve,
    tolerance: Option<ToleranceContext>,
    max_boxes: usize,
) -> Result<CurveCurveIntersection> {
    check((1..=MAX_BOXES).contains(&max_boxes),"Curve intersection needs 1..8192 boxes")?;

    intersect_curve_curve_report_with_clip_bounded(first, second, tolerance, CLIP_ROUNDS_PER_BOX, max_boxes)
}

/// Per-box fat-line clip round budget for the production entry point.
const CLIP_ROUNDS_PER_BOX: usize = 16;

/// Test/diagnostic entry: `clip_budget == 0` disables the fat-line pre-filter
/// so the unclipped subdivision path can be A/B compared against the fast one.
pub(crate) fn intersect_curve_curve_report_with_clip(
    first: &Curve,
    second: &Curve,
    tolerance: Option<ToleranceContext>,
    clip_budget: usize,
) -> Result<CurveCurveIntersection> {
    intersect_curve_curve_report_with_clip_bounded(first,second,tolerance,clip_budget,MAX_BOXES)
}

pub(crate) fn intersect_curve_curve_report_with_clip_bounded(
    first: &Curve,
    second: &Curve,
    tolerance: Option<ToleranceContext>,
    clip_budget: usize,
    max_boxes: usize,
) -> Result<CurveCurveIntersection> {
    admit_curve(first)?;
    admit_curve(second)?;
    let tolerance = context(tolerance);
    let floor = tolerance.parametric_bounds().floor.max(1e-12);
    let dist_floor = tolerance.spatial_bounds().absolute_mm.max(1e-9);
    let (first_open, _period_a, wrap_a) = unwrap_periodic_curve(first)?;
    let (second_open, _period_b, wrap_b) = unwrap_periodic_curve(second)?;
    let mut report = CurveCurveReport {
        components: Vec::new(),
        unresolved: Vec::new(),
        boxes_visited: 0,
        bernstein_excluded: 0,
        krawczyk_isolated: 0,
        bezier_clipped: 0,
    };
    let mut pending: std::collections::VecDeque<CurveSpanPending> = spans(&first_open)?
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
    // Unified runaway insurance (item 1065): pops are bounded by the initial
    // span pairs plus at most four split children and one clip requeue per
    // counted box. max_boxes keeps its unresolved-entry semantics.
    let mut guard = Budget::new(MAX_SPANS + 5 * max_boxes + 1, 49, u64::MAX)?
        .guard("cc_subdivision");
    while let Some((ta, tb, ha, hb, depth)) = pending.pop_front() {
        guard.tick()?;
        guard.check()?;
        if report.boxes_visited >= max_boxes {
            report.unresolved.push(UnresolvedCurveIntersection {
                parameter_box: [ta[0], ta[1], tb[0], tb[1]],
                reason: UnresolvedReason::ResourceBoundary,
                classification: None,
            });
            continue;
        }
        report.boxes_visited += 1;
        let (pieces, ha, hb) = match (ha, hb) {
            (Some(ha), Some(hb)) => (None, ha, hb),
            _ => {
                let pa = first_open.trim(ta[0], ta[1])?;
                let pb = second_open.trim(tb[0], tb[1])?;
                (
                    Some((pa.clone(), pb.clone())),
                    homogeneous4(&pa)?,
                    homogeneous4(&pb)?,
                )
            }
        };
        let box_input = CurveIntersectionBox {
            first: &first_open,
            second: &second_open,
            ta,
            tb,
            ha: &ha,
            hb: &hb,
            floor: dist_floor,
        };
        let excluded = curves_excluded(&first_open, &second_open, ta, tb)?;
        // Coincidence admission only on untouched original span pairs: both
        // bisected children and clip-shrunk boxes carry depth >= 1, and their
        // re-trimmed tiny hulls can collapse into false proportional matches.
        if let Some((pa, pb)) = &pieces
            && depth == 0
            && !excluded
            && admit_coincidence(&box_input, &mut report)?
        {
            let _ = (pa, pb);
            continue;
        }
        if excluded {
            report.bernstein_excluded += 1;
            continue;
        }
        // Fat-line fast pre-filter (Sederberg–Nishita): exact convex-hull
        // bounds with outward rounding, so clip exclusion is as sound as the
        // Bernstein exclusion above; a shrunk box keeps every intersection.
        if clip_budget > 0 {
            let mut budget = clip_budget;
            match bezier_clip::clip_pair(&first_open, &second_open, ta, tb, &mut budget)? {
                None => {
                    report.bezier_clipped += 1;
                    continue;
                }
                Some([na0, na1, nb0, nb1]) => {
                    let shrink =
                        ((na1 - na0) / (ta[1] - ta[0])).max((nb1 - nb0) / (tb[1] - tb[0]));
                    // Only accept a real shrink (geometric decay preserves the
                    // termination argument); otherwise fall through to bisect.
                    if shrink < 0.8
                        && ta[0] <= na0
                        && na0 < na1
                        && na1 <= ta[1]
                        && tb[0] <= nb0
                        && nb0 < nb1
                        && nb1 <= tb[1]
                    {
                        report.bezier_clipped += 1;
                        // depth + 1: clip-shrunk boxes are re-trimmed on the
                        // next visit, and coincidence admission is restricted
                        // to untouched original span pairs (depth == 0) so the
                        // collapsed hulls of a tiny clip box can never fake a
                        // Coincident overlap.
                        pending.push_back(([na0, na1], [nb0, nb1], None, None, depth + 1));
                        continue;
                    }
                }
            }
        }
        let width_a = ta[1] - ta[0];
        let width_b = tb[1] - tb[0];
        let tm = ta[0] + width_a * 0.5;
        let um = tb[0] + width_b * 0.5;
        let can_a = tm > ta[0] && tm < ta[1];
        let can_b = um > tb[0] && um < tb[1];
        if (width_a <= floor && width_b <= floor) || depth >= 48 || (!can_a && !can_b) {
            resolve_cc_box(&box_input, &mut report)?;
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
            if report.components[i].kind != CurveCurveComponentKind::Point || !keep[i] {
                continue;
            }
            for j in i + 1..n {
                if report.components[j].kind != CurveCurveComponentKind::Point || !keep[j] {
                    continue;
                }
                let a = &report.components[i];
                let b = &report.components[j];
                let touch = a.first_interval[0] <= b.first_interval[1]
                    && b.first_interval[0] <= a.first_interval[1]
                    && a.second_interval[0] <= b.second_interval[1]
                    && b.second_interval[0] <= a.second_interval[1];
                let near =
                    (a.first - b.first).abs() <= floor && (a.second - b.second).abs() <= floor;
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
    Ok(CurveCurveIntersection { report, tolerance })
}

#[cfg(test)]
mod guard_tests {
    use super::*;

    fn bezier(points: &[[f64; 3]]) -> Curve {
        let n = points.len();
        Curve {
            degree: n - 1,
            knots: [vec![0.; n], vec![1.; n]].concat(),
            control_points: points.iter().map(|p| p.to_vec()).collect(),
            weights: vec![1.; n],
            periodic: false,
        }
    }

    #[test]
    fn nan_control_point_is_rejected_at_the_boundary() {
        let a = bezier(&[[0., 0., 0.], [1., 1., 0.]]);
        let mut b = bezier(&[[0., 1., 0.], [1., 0., 0.]]);
        b.control_points[1][0] = f64::NAN;
        let err = intersect_curve_curve_report(&a, &b, None).err().expect("invalid intersection input must fail");
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    }

    #[test]
    fn crossing_pair_resolves_with_guards_active() {
        // The unified guards must not perturb the certified result.
        let a = bezier(&[[0., 0., 0.], [2., 2., 0.]]);
        let b = bezier(&[[0., 2., 0.], [2., 0., 0.]]);
        let r = intersect_curve_curve_report(&a, &b, None).unwrap();
        assert_eq!(r.report.components.len(), 1);
        assert!(r.report.unresolved.is_empty());
        let c = &r.report.components[0];
        assert!((c.first - 0.5).abs() < 1e-9 && (c.second - 0.5).abs() < 1e-9);
        assert!(c.point.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn krawczyk_newton_guard_matches_iteration_cap() {
        // Guard sized exactly at the Newton cap: 12 ticks pass, the 13th fails.
        let mut guard = Budget::with_iterations(12).unwrap().guard("cc_krawczyk_newton");
        for _ in 0..12 {
            guard.tick().unwrap();
        }
        let err = guard.tick().unwrap_err();
        assert_eq!(err.code, crate::RESOURCE_LIMIT, "{err:?}");
        assert!(err.contains("cc_krawczyk_newton"));
    }
}
