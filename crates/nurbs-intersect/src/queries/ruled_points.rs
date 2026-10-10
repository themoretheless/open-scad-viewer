use super::*;
/// The overlap UV path is a UV segment; coverage is bounding-box containment
/// of the event parameters, never spatial proximity.
fn cs_overlap_covers(report: &Report<CurveRuledSurfaceComponent>, t: f64, uv: [f64; 2]) -> bool {
    report.components.iter().any(|c| {
        matches!(c, CurveRuledSurfaceComponent::Overlap { curve_interval, uv_start, uv_end, .. }
            if curve_interval[0] <= t && t <= curve_interval[1]
                && uv_start[0].min(uv_end[0]) <= uv[0] && uv[0] <= uv_start[0].max(uv_end[0])
                && uv_start[1].min(uv_end[1]) <= uv[1] && uv[1] <= uv_start[1].max(uv_end[1]))
    })
}
pub(crate) struct CurveSurfacePointEvent {
    pub(crate) t: f64,
    pub(crate) t_interval: [f64; 2],
    pub(crate) uv: [f64; 2],
    pub(crate) uv_box: [f64; 4],
    pub(crate) point: [f64; 3],
    pub(crate) residual: f64,
    pub(crate) contact: Contact,
}

pub(crate) fn push_cs_point(report: &mut Report<CurveRuledSurfaceComponent>, event: CurveSurfacePointEvent) {
    let CurveSurfacePointEvent {
        t,
        t_interval,
        uv,
        uv_box,
        point,
        residual,
        contact,
    } = event;
    let duplicate = report.components.iter().any(|c| {
        matches!(c, CurveRuledSurfaceComponent::Point { t: et, uv: euv, .. }
            if *et == t && *euv == uv)
    });
    if duplicate || cs_overlap_covers(report, t, uv) {
        return;
    }
    report.components.push(CurveRuledSurfaceComponent::Point {
        t,
        t_interval,
        uv,
        uv_box,
        point,
        residual,
        contact,
    });
}
pub(crate) fn push_cs_overlap(
    curve_interval: [f64; 2],
    uv_start: [f64; 2],
    uv_end: [f64; 2],
    max_control_residual: f64,
    seam_wrap: bool,
    correspondence: Option<OverlapCorrespondence>,
    report: &mut Report<CurveRuledSurfaceComponent>,
) {
    report.components.push(CurveRuledSurfaceComponent::Overlap {
        curve_interval,
        uv_start,
        uv_end,
        max_control_residual,
        seam_wrap,
        correspondence,
    });
    let (umin, umax) = (uv_start[0].min(uv_end[0]), uv_start[0].max(uv_end[0]));
    let (vmin, vmax) = (uv_start[1].min(uv_end[1]), uv_start[1].max(uv_end[1]));
    report.components.retain(|c| {
        !matches!(c, CurveRuledSurfaceComponent::Point { t, uv, .. }
            if curve_interval[0] <= *t && *t <= curve_interval[1]
                && umin <= uv[0] && uv[0] <= umax
                && vmin <= uv[1] && uv[1] <= vmax)
    });
}
/// Exact parameter-corner contact admission for curve/ruled-surface boxes.
/// Curve-domain endpoints are admitted on distance alone; interior contacts
/// need a transverse crossing so grazing corner touches stay unresolved.
#[allow(clippy::too_many_arguments)]
pub(crate) fn admit_cs_corner(
    curve: &Curve,
    surface: &Surface,
    t: f64,
    u: f64,
    v: f64,
    options: Options,
    report: &mut Report<CurveRuledSurfaceComponent>,
) -> Result<bool> {
    if cs_overlap_covers(report, t, [u, v]) {
        return Ok(true);
    }
    let pc = point3(&curve.evaluate(t)?.point);
    let ps = surface.evaluate(u, v)?.point;
    let residual = distance(pc, ps);
    if residual > options.distance_tolerance {
        return Ok(false);
    }
    let curve_end = t == curve.domain()[0] || t == curve.domain()[1];
    if !curve_end && cs_transversality(curve, surface, t, u, v)? <= TRANSVERSE_SINE {
        return Ok(false);
    }
    push_cs_point(
        report,
        CurveSurfacePointEvent {
            t,
            t_interval: [t, t],
            uv: [u, v],
            uv_box: [u, u, v, v],
            point: std::array::from_fn(|i| (pc[i] + ps[i]) * 0.5),
            residual,
            contact: Contact::Boundary,
        },
    );
    Ok(true)
}
/// Evaluated boundary weight of a positive-weight curve at a parameter.
pub(crate) fn curve_weight_at(curve: &Curve, u: f64) -> Result<f64> {
    let basis = nurbs_core::curve::basis(
        curve.degree,
        &curve.knots,
        curve.control_points.len(),
        u,
        curve.periodic,
    )?;
    Ok(basis
        .basis
        .iter()
        .zip(&curve.weights)
        .map(|(b, w)| b * w)
        .sum())
}

/// Merge point events whose isolating intervals touch in the (t,u) plane.
/// Subdivision tiles exactly, so adjacency uses interval endpoints only.
pub(crate) fn merge_cs_points(report: &mut Report<CurveRuledSurfaceComponent>) {
    let intervals = |c: &CurveRuledSurfaceComponent| match c {
        CurveRuledSurfaceComponent::Point {
            t_interval, uv_box, ..
        } => Some((*t_interval, [uv_box[0], uv_box[1]])),
        _ => None,
    };
    let n = report.components.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    for i in 0..n {
        for j in i + 1..n {
            let (Some((at, au)), Some((bt, bu))) = (
                intervals(&report.components[i]),
                intervals(&report.components[j]),
            ) else {
                continue;
            };
            if at[0] <= bt[1] && bt[0] <= at[1] && au[0] <= bu[1] && bu[0] <= au[1] {
                let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                if ri != rj {
                    parent[ri] = rj;
                }
            }
        }
    }
    let mut merged: Vec<Option<CurveRuledSurfaceComponent>> = (0..n).map(|_| None).collect();
    let mut order: Vec<usize> = Vec::new();
    for (i, component) in report.components.iter().take(n).enumerate() {
        let Some((ti, ui)) = intervals(component) else {
            continue;
        };
        let root = find(&mut parent, i);
        if merged[root].is_none() {
            order.push(root);
        }
        let CurveRuledSurfaceComponent::Point {
            t,
            uv,
            uv_box,
            point,
            residual,
            contact,
            ..
        } = component
        else {
            continue;
        };
        let member = (*t, *uv, *point, *residual, *contact, (ti, ui), *uv_box);
        match &mut merged[root] {
            None => {
                merged[root] = Some(CurveRuledSurfaceComponent::Point {
                    t: member.0,
                    t_interval: ti,
                    uv: member.1,
                    uv_box: member.6,
                    point: member.2,
                    residual: member.3,
                    contact: member.4,
                });
            }
            Some(CurveRuledSurfaceComponent::Point {
                t,
                t_interval,
                uv,
                uv_box,
                point,
                residual,
                contact,
            }) => {
                if member.3 < *residual {
                    *t = member.0;
                    *uv = member.1;
                    *point = member.2;
                    *residual = member.3;
                }
                if member.4 == Contact::Boundary {
                    *contact = Contact::Boundary;
                }
                t_interval[0] = t_interval[0].min(member.5.0[0]);
                t_interval[1] = t_interval[1].max(member.5.0[1]);
                uv_box[0] = uv_box[0].min(member.6[0]);
                uv_box[1] = uv_box[1].max(member.6[1]);
            }
            _ => unreachable!(),
        }
    }
    let mut components = Vec::new();
    let mut index = 0;
    for root in order {
        while index < root {
            if intervals(&report.components[index]).is_none() {
                components.push(report.components[index].clone());
            }
            index += 1;
        }
        if let Some(c) = merged[root].take() {
            components.push(c);
        }
        index = root + 1;
    }
    while index < n {
        if intervals(&report.components[index]).is_none() {
            components.push(report.components[index].clone());
        }
        index += 1;
    }
    report.components = components;
}

/// Seam unification on an exactly closed ruled U direction. Point events at
/// the two seam sides merge only when their curve-parameter isolating
/// intervals touch and their V boxes overlap — parameter-based, never a
/// spatial tolerance. The canonical representative sits at u = u_min and
/// keeps the better-residual witness. Overlaps unify when their curve
/// intervals touch and their UV paths meet exactly at the seam: a ruling
/// pair at u_min/u_max folds to the u_min copy (identical lifted V
/// endpoints), and an iso-V chain leaving at u_max re-enters at u_min with
/// the wrap marked; merged pieces keep one correspondence map only when a
/// single map spans the whole merged interval.
pub(crate) fn unify_cs_seam(report: &mut Report<CurveRuledSurfaceComponent>, su: [f64; 2]) {
    let point_key = |c: &CurveRuledSurfaceComponent| match c {
        CurveRuledSurfaceComponent::Point {
            t_interval, uv_box, ..
        } => Some((*t_interval, *uv_box)),
        _ => None,
    };
    let mut dropped: Vec<usize> = Vec::new();
    let n = report.components.len();
    for i in 0..n {
        if dropped.contains(&i) {
            continue;
        }
        let Some((at, abox)) = point_key(&report.components[i]) else {
            continue;
        };
        if abox[0] != su[0] {
            continue;
        }
        for j in i + 1..n {
            if dropped.contains(&j) {
                continue;
            }
            let Some((bt, bbox)) = point_key(&report.components[j]) else {
                continue;
            };
            if bbox[1] != su[1] {
                continue;
            }
            let touch = at[0] <= bt[1] && bt[0] <= at[1];
            let v_overlap = abox[2] <= bbox[3] && bbox[2] <= abox[3];
            if !touch || !v_overlap {
                continue;
            }
            let (a, b) = {
                let (head, tail) = report.components.split_at_mut(j);
                (&mut head[i], &tail[0])
            };
            let (
                CurveRuledSurfaceComponent::Point {
                    t,
                    t_interval,
                    uv,
                    uv_box,
                    point,
                    residual,
                    contact,
                },
                CurveRuledSurfaceComponent::Point {
                    t: bt2,
                    t_interval: bt_interval,
                    uv: buv,
                    uv_box: buv_box,
                    point: bpoint,
                    residual: bresidual,
                    contact: bcontact,
                },
            ) = (a, b)
            else {
                unreachable!()
            };
            if *bresidual < *residual {
                *t = *bt2;
                uv[1] = buv[1];
                *point = *bpoint;
                *residual = *bresidual;
            }
            if *bcontact == Contact::Boundary {
                *contact = Contact::Boundary;
            }
            uv[0] = su[0];
            t_interval[0] = t_interval[0].min(bt_interval[0]);
            t_interval[1] = t_interval[1].max(bt_interval[1]);
            uv_box[0] = su[0];
            uv_box[2] = uv_box[2].min(buv_box[2]);
            uv_box[3] = uv_box[3].max(buv_box[3]);
            dropped.push(j);
            break;
        }
    }
    // Overlap unification across the seam.
    let overlap_key = |c: &CurveRuledSurfaceComponent| match c {
        CurveRuledSurfaceComponent::Overlap {
            curve_interval,
            uv_start,
            uv_end,
            ..
        } => Some((*curve_interval, *uv_start, *uv_end)),
        _ => None,
    };
    loop {
        let mut merged = false;
        'outer: for i in 0..report.components.len() {
            if dropped.contains(&i) {
                continue;
            }
            for j in 0..report.components.len() {
                if j == i || dropped.contains(&j) {
                    continue;
                }
                let (Some((aci, aus, aue)), Some((bci, bus, bue))) = (
                    overlap_key(&report.components[i]),
                    overlap_key(&report.components[j]),
                ) else {
                    continue;
                };
                // Ruling pair: same lifted V endpoints, touching intervals.
                let ruling_pair = aus[0] == aue[0]
                    && aus[0] == su[0]
                    && bus[0] == bue[0]
                    && bus[0] == su[1]
                    && aus[1] == bus[1]
                    && aue[1] == bue[1]
                    && aci[0] <= bci[1]
                    && bci[0] <= aci[1];
                // Iso-V chain crossing the seam: A ends at u_max exactly where
                // B begins at u_min, with touching curve intervals.
                let iso_cross = aue[0] == su[1]
                    && bus[0] == su[0]
                    && aue[1] == bus[1]
                    && aus[1] == aue[1]
                    && bus[1] == bue[1]
                    && aci[1] == bci[0];
                if !ruling_pair && !iso_cross {
                    continue;
                }
                let same_interval = aci == bci;
                let (lo, hi) = if i < j { (i, j) } else { (j, i) };
                let (head, tail) = report.components.split_at_mut(hi);
                let (first, second) = (&mut head[lo], &mut tail[0]);
                let (a, b) = if i < j {
                    (first, second)
                } else {
                    (second, first)
                };
                let CurveRuledSurfaceComponent::Overlap {
                    curve_interval,
                    uv_end,
                    max_control_residual,
                    seam_wrap,
                    correspondence,
                    ..
                } = a
                else {
                    unreachable!()
                };
                let CurveRuledSurfaceComponent::Overlap {
                    max_control_residual: bres,
                    correspondence: bcorr,
                    ..
                } = b
                else {
                    unreachable!()
                };
                let bres = *bres;
                let bcorr = bcorr.clone();
                if iso_cross {
                    *uv_end = bue;
                    *correspondence = None;
                } else if !same_interval {
                    *correspondence = None;
                } else if correspondence.is_none() {
                    *correspondence = bcorr;
                }
                curve_interval[0] = curve_interval[0].min(bci[0]);
                curve_interval[1] = curve_interval[1].max(bci[1]);
                *max_control_residual = max_control_residual.max(bres);
                *seam_wrap = true;
                dropped.push(j);
                merged = true;
                break 'outer;
            }
        }
        if !merged {
            break;
        }
    }
    if !dropped.is_empty() {
        let mut index = 0;
        report.components.retain(|_| {
            let keep = !dropped.contains(&index);
            index += 1;
            keep
        });
    }
}
