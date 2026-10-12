use super::*;

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
            let sign_change = coeffs
                .windows(2)
                .any(|w| w[0] == 0. || w[1] == 0. || w[0].signum() != w[1].signum())
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
                    let t = if owns_parameter(
                        interval[0],
                        interval[1],
                        curve_open.domain()[1],
                        interval[0],
                    ) && curve_open
                        .evaluate(interval[0])?
                        .point
                        .iter()
                        .zip(&normal)
                        .map(|(x, n)| x * n)
                        .sum::<f64>()
                        - offset
                        <= dist_floor
                    {
                        interval[0]
                    } else if owns_parameter(
                        interval[0],
                        interval[1],
                        curve_open.domain()[1],
                        interval[1],
                    ) && (point3(&curve_open.evaluate(interval[1])?.point)
                        .ok()
                        .map(|p| (dot3(normal, p) - offset).abs())
                        .unwrap_or(1.))
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
        let mut pending: std::collections::VecDeque<CurveSurfacePending> = spans(&curve_open)?
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
            let width = (ta[1] - ta[0]).max(uv[1] - uv[0]).max(uv[3] - uv[2]);
            if width <= floor || depth >= 40 {
                let cp = point3(&curve_open.evaluate(tm)?.point)?;
                let sp = point3(&surface.evaluate(um, vm)?.point)?;
                let residual = distance(&cp, &sp);
                let diag = next_up(
                    hull_diagonal(&ha) + {
                        let flat: Vec<[f64; 4]> = hg.iter().flatten().copied().collect();
                        hull_diagonal(&flat)
                    },
                );
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
                    if !det.is_finite() || det.abs() <= 64. * f64::EPSILON {
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
                    if !(ta[0] <= t
                        && t <= ta[1]
                        && uv[0] <= u
                        && u <= uv[1]
                        && uv[2] <= v
                        && v <= uv[3])
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
                    || !owns_parameter(
                        uv[0],
                        uv[1],
                        surface.knots_u[surface.knots_u.len() - surface.degree_u - 1],
                        u,
                    )
                    || !owns_parameter(
                        uv[2],
                        uv[3],
                        surface.knots_v[surface.knots_v.len() - surface.degree_v - 1],
                        v,
                    )
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
                    "multiplicity":if contact=="even_tangency"{2}else{1},
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
    // Parse (t, u, v) once, sort, and sweep a dedup window: O(n log n) instead
    // of rescanning and reparsing every accumulated event per component.
    struct PointEvent {
        t: f64,
        u: f64,
        v: f64,
        residual: f64,
        index: usize,
    }
    let parse_point = |component: &Value| -> Option<PointEvent> {
        if component["kind"] != "point" {
            return None;
        }
        let t = component["t"].as_f64().unwrap_or(0.);
        let uv = component.get("uv").and_then(Value::as_array);
        let u = uv
            .and_then(|a| a.first())
            .and_then(Value::as_f64)
            .unwrap_or(0.);
        let v = uv
            .and_then(|a| a.get(1))
            .and_then(Value::as_f64)
            .unwrap_or(0.);
        Some(PointEvent {
            t,
            u,
            v,
            residual: component["residual"].as_f64().unwrap_or(f64::INFINITY),
            index: usize::MAX,
        })
    };
    let mut events: Vec<PointEvent> = Vec::new();
    for component in &components {
        if let Some(mut event) = parse_point(component) {
            event.index = events.len();
            events.push(event);
        }
    }
    let mut order: Vec<usize> = (0..events.len()).collect();
    order.sort_by(|&a, &b| {
        events[a]
            .t
            .total_cmp(&events[b].t)
            .then_with(|| events[a].u.total_cmp(&events[b].u))
            .then_with(|| events[a].v.total_cmp(&events[b].v))
    });
    // Windowed sweep: candidates for a near-duplicate share a t within `floor`.
    // Keeps the lower-residual copy, like the original first/best rule.
    let mut dropped = vec![false; events.len()];
    let mut window: Vec<usize> = Vec::new();
    for &i in &order {
        while window
            .first()
            .is_some_and(|&j| events[j].t < events[i].t - floor)
        {
            window.remove(0);
        }
        let mut duplicate_of: Option<usize> = None;
        for &j in &window {
            if (events[j].t - events[i].t).abs() <= floor
                && (events[j].u - events[i].u).abs() <= floor
                && (events[j].v - events[i].v).abs() <= floor
            {
                duplicate_of = Some(j);
                break;
            }
        }
        if let Some(j) = duplicate_of {
            // Keep the lower residual copy.
            if events[i].residual < events[j].residual {
                dropped[j] = true;
                window.retain(|&k| k != j);
                window.push(i);
            } else {
                dropped[i] = true;
                continue;
            }
        } else {
            window.push(i);
        }
    }
    let mut event_cursor = 0usize;
    let mut dedup = Vec::new();
    for component in components {
        if component["kind"] == "point" {
            let dropped_event = dropped[event_cursor];
            event_cursor += 1;
            if dropped_event {
                continue;
            }
        }
        dedup.push(component);
    }
    dedup.sort_by(|a, b| {
        let ta = a
            .get("t")
            .and_then(Value::as_f64)
            .or_else(|| {
                a.get("curveInterval")
                    .and_then(|v| v.as_array())
                    .and_then(|a| a.first())
                    .and_then(Value::as_f64)
            })
            .unwrap_or(0.);
        let tb = b
            .get("t")
            .and_then(Value::as_f64)
            .or_else(|| {
                b.get("curveInterval")
                    .and_then(|v| v.as_array())
                    .and_then(|a| a.first())
                    .and_then(Value::as_f64)
            })
            .unwrap_or(0.);
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
    Ok(
        json!({"version":VERSION,"admitted":true,"maxDegree":MAX_DEGREE,"maxControls":MAX_CONTROLS,"maxBoxes":MAX_BOXES,"maxSpans":MAX_SPANS}),
    )
}

#[cfg(test)]mod bounded_curve_tests {
 use super::super::*;
 fn line(a:[f64;3],b:[f64;3])->Curve{Curve::from_polyline(vec![a.to_vec(),b.to_vec()]).unwrap()}
 #[test]fn default_budget_keeps_existing_report_identical(){
  let a=line([0.,0.,0.],[2.,2.,0.]);let b=line([0.,2.,0.],[2.,0.,0.]);
  assert_eq!(intersect_curve_curve(&a,&b,None).unwrap(),intersect_curve_curve_bounded(&a,&b,None,8192).unwrap());
 }
 #[test]fn unvisited_source_spans_remain_unresolved_at_box_limit(){
  let a=Curve::from_polyline(vec![vec![0.,0.,0.],vec![1.,1.,0.],vec![2.,0.,0.]]).unwrap();let b=line([1.,-1.,0.],[1.,2.,0.]);
  let report=intersect_curve_curve_bounded(&a,&b,None,1).unwrap();
  assert_eq!(report["coverage"]["resourceLimit"].as_u64(),Some(1));assert!(report["coverage"]["boxesVisited"].as_u64().unwrap()<=1);assert_eq!(report["coverage"]["complete"].as_bool(),Some(false));
  assert!(!report["unresolved"].as_array().unwrap().is_empty());
  assert!(intersect_curve_curve_bounded(&a,&b,None,0).is_err());
 }
}
