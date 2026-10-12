use super::*;

fn admit_coincidence(input: &CurveIntersectionBox<'_>, report: &mut Report) -> Result<bool> {
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

fn resolve_cc_box(input: &CurveIntersectionBox<'_>, report: &mut Report) -> Result<()> {
    let CurveIntersectionBox {
        first,
        second,
        ta,
        tb,
        ha,
        hb,
        floor,
    } = *input;
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
    if let Some((t, u)) = krawczyk_cc(first, second, ta, tb, floor)?
        && owns_parameter(ta[0], ta[1], first.domain()[1], t)
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
    let contact = contact_class(first, second, tm, um, floor)?;
    if matches!(
        contact,
        "odd_tangency" | "even_tangency" | "higher_order_contact"
    ) {
        let multiplicity = if contact == "even_tangency" {
            2
        } else if contact == "higher_order_contact" {
            3
        } else {
            1
        };
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

pub(super) fn spans(curve: &Curve) -> Result<Vec<[f64; 2]>> {
    let segments = curve.decompose()?;
    check(
        segments.len() <= MAX_SPANS,
        "Curve span count exceeds resource limit",
    )?;
    Ok(segments.iter().map(|s| s.domain()).collect())
}

fn encode_cc_report(report: Report, tolerance: &ToleranceContext, complete: bool, max_boxes:usize) -> Value {
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
            "resourceLimit":max_boxes
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
    intersect_curve_curve_bounded(first,second,tolerance,MAX_BOXES)
}
/// Same original-definition coverage, with a caller-owned work budget.
/// Unvisited boxes remain explicit unresolved coverage.
pub fn intersect_curve_curve_bounded(first:&Curve,second:&Curve,tolerance:Option<ToleranceContext>,max_boxes:usize)->Result<Value>{
    check((1..=MAX_BOXES).contains(&max_boxes),"Curve intersection needs 1..8192 boxes")?;
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
    while let Some((ta, tb, ha, hb, depth)) = pending.pop_front() {
        if report.boxes_visited >= max_boxes {
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
        if let Some((pa, pb)) = &pieces
            && !hulls_excluded(&ha, &hb)
            && admit_coincidence(&box_input, &mut report)?
        {
            let _ = (pa, pb);
            continue;
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
    let complete = report.unresolved.is_empty();
    Ok(encode_cc_report(report, &tolerance, complete,max_boxes))
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
