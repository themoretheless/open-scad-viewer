use super::*;
#[derive(Clone, Debug)]
pub enum CurveRuledSurfaceComponent {
    /// Newton-confirmed root of C(t) = S(u,v) inside its isolating box.
    Point {
        t: f64,
        t_interval: [f64; 2],
        uv: [f64; 2],
        /// Isolating surface box [u0,u1,v0,v1]; V never subdivides on a ruling.
        uv_box: [f64; 4],
        point: [f64; 3],
        residual: f64,
        contact: Contact,
    },
    /// Curve span lying on the ruled surface: a ruling (constant U) or an
    /// iso-V directrix blend. The lifted UV path is the UV segment from
    /// uv_start to uv_end; its endpoints correspond to the curve interval
    /// ends. `correspondence` carries three (t,u,v) samples at the curve
    /// interval fractions 0, 1/2 and 1, sufficient to reconstruct the exact
    /// per-parameter map: a Möbius t->v map along a ruling (unequal endpoint
    /// weights make V a cross-ratio function of t) or an affine t->u map
    /// along an iso-V directrix. It is None only where no single map covers
    /// the merged interval (seam-joined pieces). `seam_wrap` marks a
    /// component unified across an exactly closed U seam: the canonical
    /// representative sits at u = u_min and the u_max-side copy is folded
    /// in, or an iso-V path leaves the domain at u_max and re-enters at
    /// u_min.
    Overlap {
        curve_interval: [f64; 2],
        uv_start: [f64; 2],
        uv_end: [f64; 2],
        max_control_residual: f64,
        seam_wrap: bool,
        correspondence: Option<OverlapCorrespondence>,
    },
}

/// Per-parameter map samples along a curve/ruled-surface overlap. `samples`
/// are exact (t,u,v) triples at curve-interval fractions 0, 1/2, 1.
#[derive(Clone, Debug)]
pub struct OverlapCorrespondence {
    pub kind: OverlapCorrespondenceKind,
    pub samples: [[f64; 3]; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlapCorrespondenceKind {
    /// Ruling overlap: u is constant; v(t) is the unique Möbius map through
    /// the three (t,v) samples, reconstructed by the cross-ratio identity
    /// (v-v0)(v1-v2)/((v-v2)(v1-v0)) = (t-t0)(t1-t2)/((t-t2)(t1-t0)).
    MobiusV,
    /// Iso-V overlap: v is constant; u(t) is the affine map through the
    /// three (t,u) samples.
    AffineU,
}

/// Unit-column determinant of (C'(t), S_u, S_v); zero/invalid jets stay 0.
/// This vanishes exactly when the curve direction lies in the surface tangent
/// plane, so small values mark tangency rather than transverse crossings.
pub(crate) fn cs_transversality(curve: &Curve, surface: &Surface, t: f64, u: f64, v: f64) -> Result<f64> {
    let (us, vs) = surface_tangents(surface, u, v)?;
    let mut best: f64 = 0.;
    for a in curve_tangents(curve, t)? {
        for b in &us {
            for c in &vs {
                let (la, lb, lc) = (
                    distance(a, [0.; 3]),
                    distance(*b, [0.; 3]),
                    distance(*c, [0.; 3]),
                );
                if !la.is_finite()
                    || la <= 0.
                    || !lb.is_finite()
                    || lb <= 0.
                    || !lc.is_finite()
                    || lc <= 0.
                {
                    continue;
                }
                best = best.max(
                    dot(
                        a.map(|x| x / la),
                        cross(b.map(|x| x / lb), c.map(|x| x / lc)),
                    )
                    .abs(),
                );
            }
        }
    }
    Ok(best)
}
/// 3x3 solve with partial pivoting; ill-conditioned systems return None.
fn solve3(a: [[f64; 3]; 3], b: [f64; 3]) -> Option<[f64; 3]> {
    let scale = a.iter().flatten().fold(0_f64, |m, x| m.max(x.abs()));
    if !scale.is_finite() || scale <= 0. {
        return None;
    }
    let mut m = a;
    let mut r = b;
    for col in 0..3 {
        let pivot = (col..3).max_by(|&i, &j| m[i][col].abs().total_cmp(&m[j][col].abs()))?;
        if !m[pivot][col].is_finite() || m[pivot][col].abs() <= 1e-14 * scale {
            return None;
        }
        m.swap(col, pivot);
        r.swap(col, pivot);
        for row in col + 1..3 {
            let f = m[row][col] / m[col][col];
            for k in col..3 {
                m[row][k] -= f * m[col][k];
            }
            r[row] -= f * r[col];
        }
    }
    let mut x = [0.; 3];
    for i in (0..3).rev() {
        x[i] = (r[i] - (i + 1..3).map(|k| m[i][k] * x[k]).sum::<f64>()) / m[i][i];
    }
    if x.iter().all(|v| v.is_finite()) {
        Some(x)
    } else {
        None
    }
}

pub(crate) fn dot4(a: [f64; 4], b: [f64; 4]) -> f64 {
    (0..4).map(|i| a[i] * b[i]).sum()
}

enum CsRefinement {
    Root(f64, [f64; 2], [f64; 3], f64),
    Outside,
    Failed,
}
/// Newton refinement of C(t)-S(u,v)=0 inside [ta]x[ua]x[vd]. Only iterates
/// that stay inside the isolating box are reported by that box.
#[allow(clippy::too_many_arguments)]
fn refine_cs_root(
    curve: &Curve,
    surface: &Surface,
    ta: [f64; 2],
    ua: [f64; 2],
    vd: [f64; 2],
    tm: f64,
    um: f64,
    vm: f64,
    options: Options,
) -> Result<CsRefinement> {
    let (mut t, mut u, mut v) = (tm, um, vm);
    let (wt, wu, wv) = (ta[1] - ta[0], ua[1] - ua[0], vd[1] - vd[0]);
    let inside = |t: f64, u: f64, v: f64| {
        ta[0] <= t && t <= ta[1] && ua[0] <= u && u <= ua[1] && vd[0] <= v && v <= vd[1]
    };
    for _ in 0..16 {
        if !inside(t, u, v) {
            return Ok(CsRefinement::Outside);
        }
        let jc = curve.evaluate(t)?;
        let js = surface.evaluate(u, v)?;
        let r = sub(point3(&jc.point), js.point);
        if distance(r, [0.; 3]) <= options.distance_tolerance * 1e-3 {
            break;
        }
        // C0 knots have no two-sided jets: step with one-sided span jets,
        // preferring the sides the iterate came from; any confirming side
        // combination is accepted.
        let mut c_sides = curve_tangents(curve, t)?;
        if c_sides.len() == 2 && tm > t {
            c_sides.swap(0, 1);
        }
        let (mut u_sides, mut v_sides) = surface_tangents(surface, u, v)?;
        if u_sides.len() == 2 && um > u {
            u_sides.swap(0, 1);
        }
        if v_sides.len() == 2 && vm > v {
            v_sides.swap(0, 1);
        }
        let mut step = None;
        'sides: for dc in &c_sides {
            for du in &u_sides {
                for dv in &v_sides {
                    let rows = [
                        [dc[0], -du[0], -dv[0]],
                        [dc[1], -du[1], -dv[1]],
                        [dc[2], -du[2], -dv[2]],
                    ];
                    if let Some(s) = solve3(rows, [-r[0], -r[1], -r[2]]) {
                        step = Some(s);
                        break 'sides;
                    }
                }
            }
        }
        let Some(step) = step else {
            return Ok(CsRefinement::Failed);
        };
        t += step[0];
        u += step[1];
        v += step[2];
        if step[0].abs() <= wt * 1e-6 && step[1].abs() <= wu * 1e-6 && step[2].abs() <= wv * 1e-6 {
            break;
        }
    }
    // Half-open ownership in t and u; V never subdivides, so both V faces
    // remain owned by the only V band.
    let t = snap_to_face(snap_to_face(t, ta[0]), ta[1]);
    let u = snap_to_face(snap_to_face(u, ua[0]), ua[1]);
    let v = snap_to_face(snap_to_face(v, vd[0]), vd[1]);
    let sd = surface_domain(surface);
    if !owns_parameter(ta[0], ta[1], curve.domain()[1], t)
        || !owns_parameter(ua[0], ua[1], sd[1], u)
        || !(vd[0] <= v && v <= vd[1])
    {
        return Ok(CsRefinement::Outside);
    }
    let jc = curve.evaluate(t)?;
    let js = surface.evaluate(u, v)?;
    let pc = point3(&jc.point);
    let ps = js.point;
    let residual = distance(pc, ps);
    if residual > options.distance_tolerance {
        return Ok(CsRefinement::Failed);
    }
    // One-sided jets screen a converged C0-knot root: it resolves when any
    // side combination is transverse and stays unresolved when none is.
    if cs_transversality(curve, surface, t, u, v)? <= TRANSVERSE_SINE {
        return Ok(CsRefinement::Failed);
    }
    Ok(CsRefinement::Root(
        t,
        [u, v],
        std::array::from_fn(|i| (pc[i] + ps[i]) * 0.5),
        residual,
    ))
}
/// Terminal curve/ruled-surface box resolution, mirroring the curve/curve
/// rule: provable separation by hull diagonals, ambiguous bands stay
/// unresolved, tangencies are never collapsed to guessed points.
#[allow(clippy::too_many_arguments)]
fn resolve_cs_box(
    curve: &Curve,
    surface: &Surface,
    ta: [f64; 2],
    ua: [f64; 2],
    vd: [f64; 2],
    hc: &[[f64; 4]],
    hs: &HomogeneousGrid,
    options: Options,
    report: &mut Report<CurveRuledSurfaceComponent>,
) -> Result<()> {
    let box6 = || vec![ta[0], ta[1], ua[0], ua[1], vd[0], vd[1]];
    let diagonal = |ranges: [[f64; 2]; 3]| {
        ranges
            .iter()
            .map(|r| {
                let w = r[1] - r[0];
                w * w
            })
            .sum::<f64>()
            .sqrt()
    };
    let diag = diagonal(hull_ranges(hc)) + diagonal(grid_ranges(hs));
    let tm = ta[0] + (ta[1] - ta[0]) * 0.5;
    let um = ua[0] + (ua[1] - ua[0]) * 0.5;
    let vm = vd[0] + (vd[1] - vd[0]) * 0.5;
    let pc = point3(&curve.evaluate(tm)?.point);
    let ps = surface.evaluate(um, vm)?.point;
    let residual = distance(pc, ps);
    // |C(t)-S(u,v)| >= residual - diag everywhere in the box (triangle bound).
    if residual - diag > options.distance_tolerance {
        return Ok(());
    }
    // V never subdivides on a ruling, so the midpoint's V coordinate is not
    // converged: a large midpoint residual does not disprove an in-box root.
    // Newton runs either way; only a converged, in-box, low-residual,
    // transverse iterate is reported by this box.
    if residual <= options.distance_tolerance
        && cs_transversality(curve, surface, tm, um, vm)? <= TRANSVERSE_SINE
    {
        report.unresolved(box6(), UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(());
    }
    let near_midpoint = residual <= options.distance_tolerance;
    match refine_cs_root(curve, surface, ta, ua, vd, tm, um, vm, options)? {
        CsRefinement::Root(t, uv, point, residual) => {
            let td = curve.domain();
            let sd = surface_domain(surface);
            let boundary = t == td[0]
                || t == td[1]
                || curve.knots.contains(&t)
                || uv[0] == sd[0]
                || uv[0] == sd[1]
                || surface.knots_u.contains(&uv[0])
                || uv[1] == sd[2]
                || uv[1] == sd[3];
            push_cs_point(
                report,
                CurveSurfacePointEvent {
                    t,
                    t_interval: ta,
                    uv,
                    uv_box: [ua[0], ua[1], vd[0], vd[1]],
                    point,
                    residual,
                    contact: if boundary {
                        Contact::Boundary
                    } else {
                        Contact::Transverse
                    },
                },
            );
        }
        CsRefinement::Outside => (),
        CsRefinement::Failed => {
            // A box whose midpoint was already a contact candidate fails as a
            // tangency; a far midpoint that Newton could not bring in-box is a
            // near-surface band. Both stay explicit.
            report.unresolved(
                box6(),
                if near_midpoint {
                    UnresolvedReason::TangencyOrMultipleRoot
                } else {
                    UnresolvedReason::NearCoincidence
                },
            )
        }
    }
    Ok(())
}

/// Intersect a retained positive-weight 3D NURBS curve with a ruled NURBS
/// surface: rational in one direction (any degree, multiple knot spans) and
/// linear with positive, possibly unequal endpoint weights in the other.
/// Curve spans and surface U-spans decompose into homogeneous rational Bezier
/// pieces traversed by one FIFO queue; outward-rounded control hulls exclude
/// separated boxes. Terminal boxes confirm C(t)=S(u,v) by 3x3 Newton with the
/// root required inside its own box; exact parameter corners own boundary
/// contacts (dedup by exact (t,u,v) only). Ruling and iso-V coincidences lift
/// to explicit UV path endpoints with sampled per-parameter correspondence
/// (Möbius in V along a ruling, affine in U along an iso-V); exactly closed
/// U seams unify seam-side contacts and wrap overlaps canonically at u_min,
/// while near-closed seams stay duplicate behind explicit near_coincidence
/// bands; tangencies, near-surface bands, partial ruling trims and exhausted
/// budgets stay unresolved. Reports never authorize topology changes.
pub fn curve_ruled_surface(
    curve: &Curve,
    surface: &Surface,
    options: Options,
) -> Result<Report<CurveRuledSurfaceComponent>> {
    curve.validate()?;
    if curve.control_points[0].len() != 3 {
        return Err(invalid("Curve/ruled-surface requires a 3D curve"));
    }
    surface.validate()?;
    let options = options.validate()?;
    let mut report = Report::default();
    let unsupported = |report: &mut Report<CurveRuledSurfaceComponent>| {
        let d = surface_domain(surface);
        report.unresolved(
            vec![curve.domain()[0], curve.domain()[1], d[0], d[1], d[2], d[3]],
            UnresolvedReason::UnsupportedSurface,
        );
    };
    // Canonical orientation is ruled in V: linear V with exactly two rows.
    let swapped =
        if surface.degree_v == 1 && surface.control_points[0].len() == 2 && !surface.periodic_v {
            false
        } else if surface.degree_u == 1 && surface.control_points.len() == 2 && !surface.periodic_u
        {
            true
        } else {
            unsupported(&mut report);
            return Ok(report);
        };
    let canonical = if swapped {
        transpose_surface(surface)
    } else {
        surface.clone()
    };
    let domain = surface_domain(&canonical);
    let vd = [domain[2], domain[3]];
    // Geometric U-seam closure of the canonical orientation: an exactly
    // proportional seam unifies contacts across u_min/u_max; a near-closed
    // seam stays duplicate and both seam bands are explicit unresolved
    // regions (merging is never by tolerance).
    let seam = ruled_seam_state(&canonical);
    let seam_u = (seam == RuledSeam::Closed).then_some([domain[0], domain[1]]);
    if seam == RuledSeam::Uncertain {
        let td = curve.domain();
        for &u in &domain[..2] {
            report.unresolved(
                vec![td[0], td[1], u, u, vd[0], vd[1]],
                UnresolvedReason::NearCoincidence,
            );
        }
    }
    type CurveSurfacePending = (
        [f64; 2],
        [f64; 2],
        Option<Vec<[f64; 4]>>,
        Option<HomogeneousGrid>,
        usize,
    );
    let mut pending: std::collections::VecDeque<CurveSurfacePending> =
        spans(&curve.knots, curve.degree, curve.control_points.len())
            .into_iter()
            .flat_map(|ta| {
                spans(
                    &canonical.knots_u,
                    canonical.degree_u,
                    canonical.control_points.len(),
                )
                .into_iter()
                .map(move |ua| (ta, ua, None, None, 0))
            })
            .collect();
    while let Some((ta, ua, hc, hs, depth)) = pending.pop_front() {
        if report.boxes_visited >= options.max_boxes {
            report.unresolved(
                vec![ta[0], ta[1], ua[0], ua[1], vd[0], vd[1]],
                UnresolvedReason::BudgetExceeded,
            );
            continue;
        }
        report.boxes_visited += 1;
        let (pieces, hc, hs) = match (hc, hs) {
            (Some(hc), Some(hs)) => (None, hc, hs),
            _ => {
                let pc = curve.trim(ta[0], ta[1])?;
                let ps = canonical.trim([ua[0], ua[1], vd[0], vd[1]])?;
                let (hc, hs) = (homogeneous4(&pc), homogeneous_grid(&ps));
                (Some((pc, ps)), hc, hs)
            }
        };
        if let Some((pc, ps)) = &pieces {
            for &t in &ta {
                for &u in &ua {
                    for &v in &vd {
                        admit_cs_corner(curve, &canonical, t, u, v, options, &mut report)?;
                    }
                }
            }
            if !grids_excluded(&hc, &hs)
                && ruled_coincidence(pc, ps, ta, ua, vd, seam_u, options, &mut report)?
            {
                continue;
            }
        }
        if grids_excluded(&hc, &hs) {
            report.bernstein_excluded += 1;
            continue;
        }
        let width_t = ta[1] - ta[0];
        let width_u = ua[1] - ua[0];
        let tm = ta[0] + width_t * 0.5;
        let um = ua[0] + width_u * 0.5;
        let can_t = tm > ta[0] && tm < ta[1];
        let can_u = um > ua[0] && um < ua[1];
        if (width_t <= options.parameter_tolerance && width_u <= options.parameter_tolerance)
            || depth == options.max_depth
            || (!can_t && !can_u)
        {
            resolve_cs_box(
                curve,
                &canonical,
                ta,
                ua,
                vd,
                &hc,
                &hs,
                options,
                &mut report,
            )?;
            continue;
        }
        let c_side: Vec<([f64; 2], Vec<[f64; 4]>)> = if can_t {
            let (l, r) = split_homogeneous(&hc);
            vec![([ta[0], tm], l), ([tm, ta[1]], r)]
        } else {
            vec![(ta, hc.clone())]
        };
        let s_side: Vec<([f64; 2], HomogeneousGrid)> = if can_u {
            let (l, r) = split_grid_u(&hs);
            vec![([ua[0], um], l), ([um, ua[1]], r)]
        } else {
            vec![(ua, hs.clone())]
        };
        for (ta2, h) in &c_side {
            for (ua2, g) in &s_side {
                pending.push_back((*ta2, *ua2, Some(h.clone()), Some(g.clone()), depth + 1));
            }
        }
    }
    merge_cs_points(&mut report);
    if let Some(su) = seam_u {
        unify_cs_seam(&mut report, su);
    }
    report.components.sort_by(|a, b| {
        let key = |c: &CurveRuledSurfaceComponent| match c {
            CurveRuledSurfaceComponent::Point { t, uv, .. } => (*t, uv[0], uv[1]),
            CurveRuledSurfaceComponent::Overlap {
                curve_interval,
                uv_start,
                ..
            } => (curve_interval[0], uv_start[0], uv_start[1]),
        };
        let (x, y) = (key(a), key(b));
        x.0.total_cmp(&y.0)
            .then(x.1.total_cmp(&y.1))
            .then(x.2.total_cmp(&y.2))
    });
    if swapped {
        for pending in &mut report.unresolved {
            pending.parameter_box.swap(2, 4);
            pending.parameter_box.swap(3, 5);
        }
        for component in &mut report.components {
            match component {
                CurveRuledSurfaceComponent::Point { uv, uv_box, .. } => {
                    uv.swap(0, 1);
                    uv_box.swap(0, 2);
                    uv_box.swap(1, 3);
                }
                CurveRuledSurfaceComponent::Overlap {
                    uv_start, uv_end, ..
                } => {
                    uv_start.swap(0, 1);
                    uv_end.swap(0, 1);
                }
            }
        }
    }
    Ok(report)
}
