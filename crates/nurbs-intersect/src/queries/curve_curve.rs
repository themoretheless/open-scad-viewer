use super::*;
/// Numerical curve/curve correspondence for positive-weight 3D NURBS pairs.
/// Points retain both source parameters and their isolating boxes; overlaps
/// retain ascending source intervals on both curves plus the run direction.
#[derive(Clone, Debug)]
pub enum CurveCurveComponent {
    Point {
        first: f64,
        first_interval: [f64; 2],
        second: f64,
        second_interval: [f64; 2],
        point: [f64; 3],
        residual: f64,
        contact: Contact,
    },
    /// Whole coincident span pair or clipped collinear coincidence. Intervals
    /// ascend in each source knot domain; `reversed` marks opposite runs.
    Overlap {
        first_interval: [f64; 2],
        second_interval: [f64; 2],
        reversed: bool,
        max_control_residual: f64,
    },
}

/// Outcome of an exact evaluation at a box face shared with the adjacent box.
pub(crate) enum FaceRoot {
    /// No confirmed root at the face.
    Miss,
    /// A confirmed transverse root at the face.
    Root(CurvePoint),
    /// A confirmed contact without a transverse one-sided jet.
    Ambiguous,
}
/// Exact curve/plane evaluation at a box face. Transversality is screened
/// with the one-sided jet interior to this box (the right side at the lower
/// face, the left side at the upper face), so a C0 knot root resolves when
/// its one-sided geometry is transverse. Ownership of shared faces is the
/// caller's decision (see `owns_parameter`).
pub(crate) fn plane_face_root(
    curve: &Curve,
    plane: Plane,
    interval: [f64; 2],
    face: f64,
    options: Options,
) -> Result<FaceRoot> {
    let domain = curve.domain();
    let point = point3(&curve.evaluate(face)?.point);
    let residual = plane.distance(point).abs();
    if residual > options.distance_tolerance {
        return Ok(FaceRoot::Miss);
    }
    let at_end = face == domain[0] || face == domain[1];
    let tangents = curve_tangents(curve, face)?;
    let side = if face == interval[0] {
        tangents.last()
    } else {
        tangents.first()
    };
    let transverse = side.is_some_and(|d| dot(plane.normal, *d).abs() > options.distance_tolerance);
    if !at_end && !transverse {
        return Ok(FaceRoot::Ambiguous);
    }
    Ok(FaceRoot::Root(CurvePoint {
        parameter: face,
        parameter_interval: [face, face],
        point,
        plane_residual: residual,
        contact: if at_end || curve.knots.contains(&face) {
            Contact::Boundary
        } else {
            Contact::Transverse
        },
    }))
}
/// Push a point event, deduplicating by exact parameter pair only — never by
/// spatial proximity, so repeated visits to one location stay distinct.
struct CurvePointEvent {
    first: f64,
    first_interval: [f64; 2],
    second: f64,
    second_interval: [f64; 2],
    point: [f64; 3],
    residual: f64,
    contact: Contact,
}

fn push_curve_point(report: &mut Report<CurveCurveComponent>, event: CurvePointEvent) {
    let CurvePointEvent {
        first,
        first_interval,
        second,
        second_interval,
        point,
        residual,
        contact,
    } = event;
    let duplicate = report.components.iter().any(|c| {
        matches!(c, CurveCurveComponent::Point { first: f, second: s, .. }
            if *f == first && *s == second)
    });
    if duplicate {
        return;
    }
    report.components.push(CurveCurveComponent::Point {
        first,
        first_interval,
        second,
        second_interval,
        point,
        residual,
        contact,
    });
}
/// Exact knot-corner contact admission. Interior corners need a transverse
/// crossing; domain-boundary corners are admitted on distance alone, matching
/// the curve/plane endpoint rule. Corners covered by an existing overlap are
/// owned by its interval, not reported again.
pub(crate) fn admit_curve_corner(
    first: &Curve,
    second: &Curve,
    t: f64,
    u: f64,
    options: Options,
    report: &mut Report<CurveCurveComponent>,
) -> Result<bool> {
    let covered = report.components.iter().any(|c| {
        matches!(c, CurveCurveComponent::Overlap { first_interval, second_interval, .. }
            if first_interval[0] <= t && t <= first_interval[1]
                && second_interval[0] <= u && u <= second_interval[1])
    });
    if covered {
        return Ok(true);
    }
    let pa = point3(&first.evaluate(t)?.point);
    let pb = point3(&second.evaluate(u)?.point);
    let residual = distance(pa, pb);
    if residual > options.distance_tolerance {
        return Ok(false);
    }
    let at_boundary = t == first.domain()[0]
        || t == first.domain()[1]
        || u == second.domain()[0]
        || u == second.domain()[1];
    if !at_boundary && tangent_sine(first, second, t, u)? <= TRANSVERSE_SINE {
        return Ok(false);
    }
    let contact = if at_boundary || first.knots.contains(&t) || second.knots.contains(&u) {
        Contact::Boundary
    } else {
        Contact::Transverse
    };
    push_curve_point(
        report,
        CurvePointEvent {
            first: t,
            first_interval: [t, t],
            second: u,
            second_interval: [u, u],
            point: std::array::from_fn(|i| (pa[i] + pb[i]) * 0.5),
            residual,
            contact,
        },
    );
    Ok(true)
}

#[derive(Debug)]
enum Refinement {
    /// Converged inside the isolating box: parameters, point, residual.
    Root(f64, f64, [f64; 3], f64),
    /// The Newton image left the box; the root's own box reports it.
    Outside,
    /// Degenerate tangent basis, nonfinite step, or stalled residual.
    Failed,
}
/// Newton refinement of a transverse midpoint candidate on the two dominant
/// cross-product axes. Only iterates that stay inside the isolating box are
/// reported by that box; halo boxes whose image lands elsewhere stay silent
/// instead of duplicating the root with shifted parameters.
fn refine_curve_root(
    first: &Curve,
    second: &Curve,
    ta: [f64; 2],
    tb: [f64; 2],
    tm: f64,
    um: f64,
    options: Options,
) -> Result<Refinement> {
    let (mut t, mut u) = (tm, um);
    let width_t = ta[1] - ta[0];
    let width_u = tb[1] - tb[0];
    for _ in 0..16 {
        if !(ta[0] <= t && t <= ta[1] && tb[0] <= u && u <= tb[1]) {
            return Ok(Refinement::Outside);
        }
        let ja = first.evaluate(t)?;
        let jb = second.evaluate(u)?;
        let r = sub(point3(&ja.point), point3(&jb.point));
        if distance(r, [0.; 3]) <= options.distance_tolerance * 1e-3 {
            // Converged; knots with discontinuous derivatives may have no
            // d1 here, and the box midpoint already screened transversality.
            break;
        }
        // At a C0 knot the two-sided jet does not exist: step with the
        // one-sided span jets, preferring the side the iterate came from,
        // and accept either side that yields a valid step.
        let mut a_sides = curve_tangents(first, t)?;
        if a_sides.len() == 2 && tm > t {
            a_sides.swap(0, 1);
        }
        let mut b_sides = curve_tangents(second, u)?;
        if b_sides.len() == 2 && um > u {
            b_sides.swap(0, 1);
        }
        let mut step = None;
        'sides: for va in &a_sides {
            for vb in &b_sides {
                let la = distance(*va, [0.; 3]);
                let lb = distance(*vb, [0.; 3]);
                let c = cross(*va, *vb);
                let dominant = c.map(|x| x.abs()).into_iter().fold(0., f64::max);
                if !dominant.is_finite() || dominant <= TRANSVERSE_SINE * la * lb {
                    continue;
                }
                // Solve A(t)-B(u)=0 on the two axes with the best-conditioned minor.
                let drop = if c[0].abs() == dominant {
                    0
                } else if c[1].abs() == dominant {
                    1
                } else {
                    2
                };
                let keep: [usize; 2] = match drop {
                    0 => [1, 2],
                    1 => [0, 2],
                    _ => [0, 1],
                };
                // va[k]*dt - vb[k]*du = -r[k] on both kept axes.
                let (a, b, e0) = (va[keep[0]], -vb[keep[0]], -r[keep[0]]);
                let (c2, d2, e1) = (va[keep[1]], -vb[keep[1]], -r[keep[1]]);
                let det = a * d2 - b * c2;
                if !det.is_finite() || det.abs() <= 0. {
                    continue;
                }
                let dt = (e0 * d2 - b * e1) / det;
                let du = (a * e1 - e0 * c2) / det;
                if !dt.is_finite() || !du.is_finite() {
                    continue;
                }
                step = Some((dt, du));
                break 'sides;
            }
        }
        let Some((dt, du)) = step else {
            return Ok(Refinement::Failed);
        };
        t += dt;
        u += du;
        if dt.abs() <= width_t * 1e-6 && du.abs() <= width_u * 1e-6 {
            break;
        }
    }
    // Half-open ownership: reconcile the converged image with the bitwise
    // shared faces, then exactly one box — interior and lower face, plus the
    // domain's upper end for the last box — reports a boundary-sitting root.
    let t = snap_to_face(snap_to_face(t, ta[0]), ta[1]);
    let u = snap_to_face(snap_to_face(u, tb[0]), tb[1]);
    if !owns_parameter(ta[0], ta[1], first.domain()[1], t)
        || !owns_parameter(tb[0], tb[1], second.domain()[1], u)
    {
        return Ok(Refinement::Outside);
    }
    let ja = first.evaluate(t)?;
    let jb = second.evaluate(u)?;
    let pa = point3(&ja.point);
    let pb = point3(&jb.point);
    let residual = distance(pa, pb);
    if residual > options.distance_tolerance {
        return Ok(Refinement::Failed);
    }
    // One-sided jets screen a converged C0-knot root: it resolves when either
    // side is transverse, and stays unresolved when both sides are tangent.
    if tangent_sine(first, second, t, u)? <= TRANSVERSE_SINE {
        return Ok(Refinement::Failed);
    }
    Ok(Refinement::Root(
        t,
        u,
        std::array::from_fn(|i| (pa[i] + pb[i]) * 0.5),
        residual,
    ))
}
/// Terminal box resolution. The hull-diagonal lower bound separates provably
/// disjoint pieces; ambiguous residual bands stay unresolved; transverse
/// midpoint candidates must refine to a certified in-box root. Tangent or
/// multiple-root regions are never collapsed to a guessed point.
#[allow(clippy::too_many_arguments)]
fn resolve_curve_box(
    first: &Curve,
    second: &Curve,
    ta: [f64; 2],
    tb: [f64; 2],
    tm: f64,
    um: f64,
    ha: &[[f64; 4]],
    hb: &[[f64; 4]],
    options: Options,
    report: &mut Report<CurveCurveComponent>,
) -> Result<()> {
    let box4 = || vec![ta[0], ta[1], tb[0], tb[1]];
    let diagonal = |h: &[[f64; 4]]| {
        hull_ranges(h)
            .iter()
            .map(|r| {
                let w = r[1] - r[0];
                w * w
            })
            .sum::<f64>()
            .sqrt()
    };
    let diag = diagonal(ha) + diagonal(hb);
    let pa = point3(&first.evaluate(tm)?.point);
    let pb = point3(&second.evaluate(um)?.point);
    let residual = distance(pa, pb);
    // |A(t)-B(u)| >= residual - diag everywhere in the box (triangle bound).
    if residual - diag > options.distance_tolerance {
        return Ok(());
    }
    if residual > options.distance_tolerance {
        report.unresolved(box4(), UnresolvedReason::NearCoincidence);
        return Ok(());
    }
    if tangent_sine(first, second, tm, um)? <= TRANSVERSE_SINE {
        report.unresolved(box4(), UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(());
    }
    match refine_curve_root(first, second, ta, tb, tm, um, options)? {
        Refinement::Root(t, u, point, residual) => push_curve_point(
            report,
            CurvePointEvent {
                first: t,
                first_interval: ta,
                second: u,
                second_interval: tb,
                point,
                residual,
                contact: Contact::Transverse,
            },
        ),
        Refinement::Outside => (),
        Refinement::Failed => report.unresolved(box4(), UnresolvedReason::TangencyOrMultipleRoot),
    }
    Ok(())
}
/// Merge point events whose isolating intervals touch in parameter space.
/// Subdivision tiles parameter space with exactly shared binary64 boundaries,
/// so adjacency is decided on interval endpoints — never on spatial
/// proximity. The merged event keeps the lowest-residual parameter pair.
fn merge_curve_points(report: &mut Report<CurveCurveComponent>) {
    let intervals = |c: &CurveCurveComponent| match c {
        CurveCurveComponent::Point {
            first_interval,
            second_interval,
            ..
        } => Some((*first_interval, *second_interval)),
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
            let (Some((af, as_)), Some((bf, bs))) = (
                intervals(&report.components[i]),
                intervals(&report.components[j]),
            ) else {
                continue;
            };
            if af[0] <= bf[1] && bf[0] <= af[1] && as_[0] <= bs[1] && bs[0] <= as_[1] {
                let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                if ri != rj {
                    parent[ri] = rj;
                }
            }
        }
    }
    let mut merged: Vec<Option<CurveCurveComponent>> = (0..n).map(|_| None).collect();
    let mut order: Vec<usize> = Vec::new();
    for (i, component) in report.components.iter().take(n).enumerate() {
        let Some((fi, si)) = intervals(component) else {
            continue;
        };
        let root = find(&mut parent, i);
        if merged[root].is_none() {
            order.push(root);
        }
        let CurveCurveComponent::Point {
            first,
            second,
            point,
            residual,
            contact,
            ..
        } = component
        else {
            continue;
        };
        let member = (*first, *second, *point, *residual, *contact, (fi, si));
        match &mut merged[root] {
            None => {
                merged[root] = Some(CurveCurveComponent::Point {
                    first: member.0,
                    first_interval: fi,
                    second: member.1,
                    second_interval: si,
                    point: member.2,
                    residual: member.3,
                    contact: member.4,
                });
            }
            Some(CurveCurveComponent::Point {
                first,
                first_interval,
                second,
                second_interval,
                point,
                residual,
                contact,
            }) => {
                if member.3 < *residual {
                    *first = member.0;
                    *second = member.1;
                    *point = member.2;
                    *residual = member.3;
                }
                if member.4 == Contact::Boundary {
                    *contact = Contact::Boundary;
                }
                first_interval[0] = first_interval[0].min(member.5.0[0]);
                first_interval[1] = first_interval[1].max(member.5.0[1]);
                second_interval[0] = second_interval[0].min(member.5.1[0]);
                second_interval[1] = second_interval[1].max(member.5.1[1]);
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

/// Intersect two retained positive-weight 3D NURBS curves over their entire
/// active knot domains. Knot spans decompose into homogeneous rational Bezier
/// pieces traversed by one FIFO queue across all span pairs and subdivisions;
/// positive-weight control hulls exclude disjoint boxes with outward rounding.
/// Transverse points carry both source parameters and their isolating boxes,
/// exact knot corners own boundary contacts (dedup by parameter pair only),
/// and coincident spans retain explicit trim intervals on both curves.
/// Tangencies, ambiguous coincidence clipping and exhausted budgets stay
/// unresolved; results never authorize topology changes.
pub fn curve_curve(
    first: &Curve,
    second: &Curve,
    options: Options,
) -> Result<Report<CurveCurveComponent>> {
    first.validate()?;
    second.validate()?;
    if first.control_points[0].len() != 3 || second.control_points[0].len() != 3 {
        return Err(invalid("Curve/curve requires two 3D curves"));
    }
    let options = options.validate()?;
    let mut report = Report::default();
    type CurveCurvePending = (
        [f64; 2],
        [f64; 2],
        Option<Vec<[f64; 4]>>,
        Option<Vec<[f64; 4]>>,
        usize,
    );
    let mut pending: std::collections::VecDeque<CurveCurvePending> =
        spans(&first.knots, first.degree, first.control_points.len())
            .into_iter()
            .flat_map(|ta| {
                spans(&second.knots, second.degree, second.control_points.len())
                    .into_iter()
                    .map(move |tb| (ta, tb, None, None, 0))
            })
            .collect();
    while let Some((ta, tb, ha, hb, depth)) = pending.pop_front() {
        if report.boxes_visited >= options.max_boxes {
            report.unresolved(
                vec![ta[0], ta[1], tb[0], tb[1]],
                UnresolvedReason::BudgetExceeded,
            );
            continue;
        }
        report.boxes_visited += 1;
        let (pieces, ha, hb) = match (ha, hb) {
            (Some(ha), Some(hb)) => (None, ha, hb),
            _ => {
                let pa = first.trim(ta[0], ta[1])?;
                let pb = second.trim(tb[0], tb[1])?;
                let (ha, hb) = (homogeneous4(&pa), homogeneous4(&pb));
                (Some((pa, pb)), ha, hb)
            }
        };
        if let Some((pa, pb)) = &pieces {
            for &t in &ta {
                for &u in &tb {
                    admit_curve_corner(first, second, t, u, options, &mut report)?;
                }
            }
            if !hulls_excluded(&ha, &hb) && curve_coincidence(pa, pb, ta, tb, options, &mut report)?
            {
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
        if (width_a <= options.parameter_tolerance && width_b <= options.parameter_tolerance)
            || depth == options.max_depth
            || (!can_a && !can_b)
        {
            resolve_curve_box(
                first,
                second,
                ta,
                tb,
                tm,
                um,
                &ha,
                &hb,
                options,
                &mut report,
            )?;
            continue;
        }
        // Bisect both sides per depth level so depth 48 bounds each width by
        // 2^-48 of the source span; four children keep the FIFO fair order.
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
    merge_curve_points(&mut report);
    report.components.sort_by(|a, b| {
        let parameters = |c: &CurveCurveComponent| match c {
            CurveCurveComponent::Point { first, second, .. } => (*first, *second),
            CurveCurveComponent::Overlap {
                first_interval,
                second_interval,
                ..
            } => (first_interval[0], second_interval[0]),
        };
        let (af, as_) = parameters(a);
        let (bf, bs) = parameters(b);
        af.total_cmp(&bf).then(as_.total_cmp(&bs))
    });
    Ok(report)
}
