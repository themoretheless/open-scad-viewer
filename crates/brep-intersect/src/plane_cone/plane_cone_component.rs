use super::*;

#[derive(Clone, Debug)]
pub enum PlaneConeComponent {
    /// Plane perpendicular to the axis: the exact circle (full, or clipped
    /// to the finite plane domain) with the radius linearly interpolated
    /// between the rings at the section height, with lifts on both
    /// surfaces.
    Circle {
        /// Exact rational quadratic sweep (full circle: four 90-degree
        /// arcs, weights cos(pi/4), knots 0..=4).
        curve: Curve,
        center: [f64; 3],
        radius: f64,
        /// Unit plane normal.
        normal: [f64; 3],
        /// True when the circle lies fully inside the plane patch domain.
        full: bool,
        plane_uv: Vec<PlanePatchCurve>,
        /// Iso-v circle arcs on the side patches.
        cone_uv: Vec<CylinderPatchCurve>,
        max_sample_residual: f64,
    },
    /// Plane through the axis: one exact straight ruling through the apex,
    /// clipped by the finite height and the patch rectangle.
    Line {
        /// Exact degree-1 line, knots [0,0,1,1], unit weights, start -> end.
        curve: Curve,
        start: [f64; 3],
        end: [f64; 3],
        /// Unit ruling direction (apex toward the rings).
        direction: [f64; 3],
        /// Clip endpoints sit on ring planes or the patch boundary.
        contact: Contact,
        plane_uv: Vec<PlanePatchCurve>,
        /// Iso-u segments on the side patches (two entries when the ruling
        /// sits on a quadrant seam).
        cone_uv: Vec<CylinderPatchCurve>,
        max_sample_residual: f64,
    },
    /// Oblique plane steeper than the side (|axis.n| > sin(alpha)): the
    /// exact ellipse (full, or clipped to the finite height and patch
    /// domain) with the exact plane-UV lift. The cone side lift is null:
    /// no exact low-degree rational UV representation exists on the ruled
    /// side patches (module header); the 3D ellipse is exact regardless.
    Ellipse {
        /// Exact rational quadratic sweep over the eccentric-angle interval.
        curve: Curve,
        center: [f64; 3],
        semi_major: f64,
        semi_minor: f64,
        /// Unit in-plane semi-major direction.
        major: [f64; 3],
        /// Unit in-plane semi-minor direction.
        minor: [f64; 3],
        /// Unit plane normal.
        normal: [f64; 3],
        /// True when the full ellipse survives every clip.
        full: bool,
        plane_uv: Vec<PlanePatchCurve>,
        cone_uv: Option<Vec<CylinderPatchCurve>>,
        max_sample_residual: f64,
    },
    /// Plane exactly parallel to a ruling (|axis.n| == sin(alpha) at pure
    /// rounding): the exact rational parabola (polynomial quadratic arcs,
    /// all weights 1) clipped by the finite height and the patch domain.
    Parabola {
        /// Exact degree-2 Bezier arcs over the clipped t-interval, weights 1.
        curve: Curve,
        /// Parabola vertex (3D).
        vertex: [f64; 3],
        /// Unit in-plane direction the parabola opens toward.
        direction: [f64; 3],
        /// Focal length |L| / 4 of y^2 = L (x - x0).
        focal_length: f64,
        /// Unit plane normal.
        normal: [f64; 3],
        plane_uv: Vec<PlanePatchCurve>,
        cone_uv: Option<Vec<CylinderPatchCurve>>,
        max_sample_residual: f64,
    },
    /// Plane shallower than the side (|axis.n| < sin(alpha)), including
    /// off-axis parallel planes: one exact rational quadratic hyperbola arc
    /// clipped by the height rings and the patch domain.
    Hyperbola {
        /// Exact rational quadratic arc over the clipped t-interval.
        curve: Curve,
        /// Midpoint between the two branch vertices (3D).
        center: [f64; 3],
        /// Transverse semi-axis a (vertex distance from the center).
        semi_transverse: f64,
        /// Conjugate semi-axis b (asymptote slope b / a).
        semi_conjugate: f64,
        /// Unit in-plane transverse direction toward this branch.
        transverse: [f64; 3],
        /// Unit in-plane conjugate direction.
        conjugate: [f64; 3],
        /// Unit plane normal.
        normal: [f64; 3],
        plane_uv: Vec<PlanePatchCurve>,
        cone_uv: Option<Vec<CylinderPatchCurve>>,
        max_sample_residual: f64,
    },
}

/// Plane UV coordinates of a 3D point under the rectangular patch frame.
pub(super) fn plane_uv(plane: &CanonicalPlane, point: [f64; 3]) -> [f64; 2] {
    let rel = sub(point, plane.origin);
    [
        dot(rel, plane.u) / (plane.u_len * plane.u_len),
        dot(rel, plane.v) / (plane.v_len * plane.v_len),
    ]
}


/// Worst residual over `count` curve samples against the cone side equation
/// (radius linearly interpolated in the axial coordinate) and the plane
/// equation.
pub(super) fn side_plane_residuals(
    curve: &Curve,
    cone: &CanonicalCone,
    plane: &CanonicalPlane,
    count: usize,
) -> Result<f64> {
    let [lo, hi] = curve.domain();
    let mut worst = 0_f64;
    for i in 0..count {
        let t = lo + (hi - lo) * i as f64 / (count - 1) as f64;
        let p = point3(&curve.evaluate(t)?.point);
        let rel = sub(p, cone.bottom);
        let a = dot(rel, cone.axis);
        let perp = sub(rel, cone.axis.map(|x| x * a));
        let side = (norm(perp) - (cone.r_bottom + cone.slope * a)).abs();
        let planar = dot(plane.normal, sub(p, plane.origin)).abs();
        worst = worst.max(side).max(planar);
    }
    Ok(worst)
}

/// Clipped iso-v circle lifts on the side patches for a section circle whose
/// swept parameter intervals are `arcs`; per-quadrant trims use the exact
/// rational-arc angle parameter of `cylinder_cylinder::arc_parameter`.
pub(super) fn lift_circle_side(
    cone: &CanonicalCone,
    center: [f64; 3],
    v0: f64,
    arcs: &[(f64, f64)],
    point_at: impl Fn(f64) -> [f64; 3],
) -> Vec<CylinderPatchCurve> {
    let mut lifted = Vec::new();
    for &(a, b) in arcs {
        let sweep = b - a;
        if !sweep.is_finite() || sweep <= 0. {
            continue;
        }
        let theta = |phi: f64| {
            let rel = sub(point_at(phi), center);
            dot(rel, cone.frame[1])
                .atan2(dot(rel, cone.frame[0]))
                .rem_euclid(TAU)
        };
        let (ta, tb, tm) = (theta(a), theta(b), theta(a + sweep / 2.));
        let forward = (tb - ta).rem_euclid(TAU);
        let image = if (tm - ta).rem_euclid(TAU) <= forward {
            (ta, forward)
        } else {
            (tb, (ta - tb).rem_euclid(TAU))
        };
        for (quadrant, patch) in cone.sides.iter().enumerate() {
            let span = (quadrant as f64 * QUARTER, QUARTER);
            for (s0, sw) in ccw_intersect(span, image) {
                let rel0 = s0 - quadrant as f64 * QUARTER;
                let u0 = arc_parameter(rel0);
                let u1 = arc_parameter(rel0 + sw);
                if u1 - u0 <= 1e-12 {
                    continue;
                }
                lifted.push(CylinderPatchCurve {
                    patch: *patch,
                    arcs: vec![Curve {
                        degree: 1,
                        knots: vec![0., 0., 1., 1.],
                        control_points: vec![[u0, v0].to_vec(), [u1, v0].to_vec()],
                        weights: vec![1., 1.],
                        periodic: false,
                    }],
                });
            }
        }
    }
    lifted
}

/// Iso-v lines on all four side patches (full circle at height v0).
pub(super) fn lift_circle_side_full(cone: &CanonicalCone, v0: f64) -> Vec<CylinderPatchCurve> {
    cone.sides
        .iter()
        .map(|&patch| CylinderPatchCurve {
            patch,
            arcs: vec![Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: vec![[0., v0].to_vec(), [1., v0].to_vec()],
                weights: vec![1., 1.],
                periodic: false,
            }],
        })
        .collect()
}

/// Per-patch iso-u lift of a ruling: the line apex + t*w over t in [lo, hi]
/// sits at one fixed quadrant angle on the side wall (the angle comes from
/// the ruling direction, since the apex itself has no radial part); v is the
/// height fraction (t + t_apex) / h. Seam rulings are emitted on both
/// adjacent patches.
pub(super) fn lift_ruling(cone: &CanonicalCone, w: [f64; 3], lo: f64, hi: f64) -> Vec<CylinderPatchCurve> {
    const SEAM: f64 = 1e-12;
    let t_apex = -cone.r_bottom / cone.slope;
    let axial = dot(w, cone.axis);
    let perp = sub(w, cone.axis.map(|x| x * axial));
    let angle = dot(perp, cone.frame[1])
        .atan2(dot(perp, cone.frame[0]))
        .rem_euclid(TAU);
    let u_global = angle / QUARTER;
    let quadrant_f = u_global.floor();
    let u_linear = u_global - quadrant_f;
    let quadrant = quadrant_f as usize % 4;
    let u = arc_parameter(u_linear * QUARTER);
    let v_lo = (lo + t_apex) / cone.height;
    let v_hi = (hi + t_apex) / cone.height;
    let segment = |u: f64| Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![[u, v_lo].to_vec(), [u, v_hi].to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    };
    if u_linear <= SEAM {
        vec![
            CylinderPatchCurve {
                patch: cone.sides[quadrant],
                arcs: vec![segment(0.)],
            },
            CylinderPatchCurve {
                patch: cone.sides[(quadrant + 3) % 4],
                arcs: vec![segment(1.)],
            },
        ]
    } else if u_linear >= 1. - SEAM {
        vec![
            CylinderPatchCurve {
                patch: cone.sides[(quadrant + 1) % 4],
                arcs: vec![segment(0.)],
            },
            CylinderPatchCurve {
                patch: cone.sides[quadrant],
                arcs: vec![segment(1.)],
            },
        ]
    } else {
        vec![CylinderPatchCurve {
            patch: cone.sides[quadrant],
            arcs: vec![segment(u)],
        }]
    }
}

/// Clip the line `foot_uv + t * dir_uv` to `[lo0, hi0]` and the unit square.
/// Returns the surviving interval, or the reason it fails: misses are
/// silent, boundary-coincident lines, band-thin clips and patch-corner
/// touches are tangencies.
pub(super) enum LineClip {
    Miss,
    Tangent,
    Span(f64, f64),
}

pub(super) fn clip_line_rect(
    foot_uv: [f64; 2],
    dir_uv: [f64; 2],
    lo0: f64,
    hi0: f64,
    band: f64,
    band_uv: [f64; 2],
) -> LineClip {
    let mut lo = lo0;
    let mut hi = hi0;
    for k in 0..2 {
        let f = foot_uv[k];
        let d = dir_uv[k];
        if d.abs() <= 1e-30 {
            if f < -band_uv[k] || f > 1. + band_uv[k] {
                return LineClip::Miss;
            }
            if f.abs() <= band_uv[k] || (f - 1.).abs() <= band_uv[k] {
                return LineClip::Tangent;
            }
            continue;
        }
        let t0 = -f / d;
        let t1 = (1. - f) / d;
        lo = lo.max(t0.min(t1));
        hi = hi.min(t0.max(t1));
    }
    if hi < lo - band {
        return LineClip::Miss;
    }
    if hi - lo <= band {
        return LineClip::Tangent;
    }
    for t in [lo, hi] {
        let q = [foot_uv[0] + t * dir_uv[0], foot_uv[1] + t * dir_uv[1]];
        let near_u = q[0].abs() <= band_uv[0] || (q[0] - 1.).abs() <= band_uv[0];
        let near_v = q[1].abs() <= band_uv[1] || (q[1] - 1.).abs() <= band_uv[1];
        if near_u && near_v {
            return LineClip::Tangent;
        }
    }
    LineClip::Span(lo, hi)
}

/// Polar form (blossom) of the quadratic c0 + c1 t + c2 t^2 at (u, v).
fn polar(c: [f64; 3], u: f64, v: f64) -> f64 {
    c[0] + c[1] * (u + v) / 2. + c[2] * u * v
}

/// Exact rational quadratic Bezier (single span, knots [0,0,0,1,1,1]) of the
/// 3D rational quadratic p(t) = num(t)/den(t) over [t0, t1], via the polar
/// form of the homogeneous numerator/denominator — never a fit.
pub(super) fn bezier_rational_3d(num: [[f64; 3]; 3], den: [f64; 3], t0: f64, t1: f64) -> Curve {
    let ws = [polar(den, t0, t0), polar(den, t0, t1), polar(den, t1, t1)];
    let ts = [(t0, t0), (t0, t1), (t1, t1)];
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|k| polar(num[k], ts[i].0, ts[i].1) / ws[i])
                    .collect()
            })
            .collect(),
        weights: ws.to_vec(),
        periodic: false,
    }
}

/// Exact rational quadratic Bezier of the 2D p(t) = num(t)/den(t) over
/// [t0, t1].
pub(super) fn bezier_rational_2d(num: [[f64; 3]; 2], den: [f64; 3], t0: f64, t1: f64) -> Curve {
    let ws = [polar(den, t0, t0), polar(den, t0, t1), polar(den, t1, t1)];
    let ts = [(t0, t0), (t0, t1), (t1, t1)];
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..2)
                    .map(|k| polar(num[k], ts[i].0, ts[i].1) / ws[i])
                    .collect()
            })
            .collect(),
        weights: ws.to_vec(),
        periodic: false,
    }
}

/// Clip the rational quadratic p(t) = num(t)/den(t) (2D coefficient arrays
/// c0 + c1 t + c2 t^2) on t in (lo, hi) — where den > 0 — by the half-planes
/// n.p <= offset. Each half-plane boundary crossing solves the exact
/// quadratic f(t) = n.num(t) - offset den(t) = 0; an extremum of a
/// constraint function within the half-plane's band of the boundary is a
/// tangency reported through the flag and never clipped through; surviving
/// spans are decided by exact midpoint evaluation; band-thin spans are
/// tangencies; nothing is merged across real gaps.
pub(super) fn clip_conic_t(
    num: [[f64; 3]; 2],
    den: [f64; 3],
    lo: f64,
    hi: f64,
    halfplanes: &[Halfplane2],
    t_band: f64,
) -> (Vec<(f64, f64)>, bool) {
    let d = |t: f64| (den[2] * t + den[1]) * t + den[0];
    let p = |t: f64, k: usize| (num[k][2] * t + num[k][1]) * t + num[k][0];
    let coeffs = |hp: &Halfplane2| {
        (
            hp.normal[0] * num[0][2] + hp.normal[1] * num[1][2] - hp.offset * den[2],
            hp.normal[0] * num[0][1] + hp.normal[1] * num[1][1] - hp.offset * den[1],
            hp.normal[0] * num[0][0] + hp.normal[1] * num[1][0] - hp.offset * den[0],
        )
    };
    let mut cuts: Vec<f64> = Vec::new();
    let mut tangent = false;
    for hp in halfplanes {
        let (a, b, c) = coeffs(hp);
        if !a.is_finite() || !b.is_finite() || !c.is_finite() {
            return (Vec::new(), true);
        }
        let f = |t: f64| (a * t + b) * t + c;
        let band_f = |t: f64| hp.band * d(t).abs();
        let mid = (lo + hi) / 2.;
        if a == 0. {
            if b == 0. {
                if c.abs() <= band_f(mid) {
                    tangent = true;
                } else if c > 0. {
                    return (Vec::new(), tangent);
                }
                continue;
            }
            cuts.push(-c / b);
            continue;
        }
        let t_ext = -b / (2. * a);
        let f_ext = f(t_ext);
        // An extremum within the band of the boundary: a tangency, never
        // clipped through.
        if f_ext.abs() <= band_f(t_ext) {
            tangent = true;
            continue;
        }
        if a > 0. && f_ext > 0. {
            // Violated everywhere: the conic misses this region.
            return (Vec::new(), tangent);
        }
        if a < 0. && f_ext < 0. {
            // Satisfied everywhere: the constraint never binds.
            continue;
        }
        let disc = b * b - 4. * a * c;
        if !disc.is_finite() || disc <= 0. {
            if f_ext > 0. {
                return (Vec::new(), tangent);
            }
            continue;
        }
        let root = disc.sqrt();
        let q = if b >= 0. {
            -0.5 * (b + root)
        } else {
            -0.5 * (b - root)
        };
        cuts.push(q / a);
        if q != 0. {
            cuts.push(c / q);
        } else {
            cuts.push(0.);
        }
    }
    let inside = |t: f64| {
        halfplanes.iter().all(|hp| {
            let (a, b, c) = coeffs(hp);
            let dv = d(t);
            let f = (a * t + b) * t + c;
            let scale = (hp.normal[0].abs() * p(t, 0).abs()
                + hp.normal[1].abs() * p(t, 1).abs()
                + hp.offset.abs() * dv.abs())
                / dv.abs().max(1e-300)
                + 1.;
            f / dv <= 1e-12 * scale
        })
    };
    if cuts.is_empty() {
        if hi - lo > t_band {
            return (vec![(lo, hi)], tangent);
        }
        return (Vec::new(), true);
    }
    cuts.retain(|t| *t > lo && *t < hi && t.is_finite());
    cuts.sort_by(f64::total_cmp);
    cuts.dedup_by(|a, b| (*a - *b).abs() <= 1e-12 * (1. + a.abs()));
    let mut arcs: Vec<(f64, f64)> = Vec::new();
    let mut start = lo;
    for &cut in cuts.iter().chain(std::iter::once(&hi)) {
        let end = cut;
        if end - start > 1e-14 * (1. + start.abs()) && inside((start + end) / 2.) {
            arcs.push((start, end));
        }
        start = end;
    }
    // Merge spans split only by a phantom (sub-rounding) cut.
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for arc in arcs {
        if let Some(last) = merged.last_mut()
            && arc.0 - last.1 <= 1e-12 * (1. + last.1.abs())
        {
            last.1 = arc.1;
            continue;
        }
        merged.push(arc);
    }
    let mut out = Vec::new();
    for (a, b) in merged {
        if b - a <= t_band {
            // Band-thin clip: a boundary touch, never a degenerate arc.
            tangent = true;
            continue;
        }
        out.push((a, b));
    }
    (out, tangent)
}
