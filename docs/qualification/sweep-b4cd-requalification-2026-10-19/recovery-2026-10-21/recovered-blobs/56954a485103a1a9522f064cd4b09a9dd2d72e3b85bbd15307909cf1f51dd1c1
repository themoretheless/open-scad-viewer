//! Multipatch G1 continuity around a common vertex (vertex enclosure).
//!
//! A fan is an ordered ring of tensor-product patches whose corners coincide
//! at one vertex; patch `i` shares its `outgoing` boundary with the incoming
//! boundary of patch `(i + 1) % n`. The module offers
//!
//! - [`analyze`]: vertex-enclosure diagnostics — positional coincidence,
//!   tangent coplanarity, valence classification and a first-order twist
//!   compatibility test (the component of the twist sum orthogonal to the
//!   seam osculating plane `span{t, t''}` at the vertex),
//! - [`join_fan`]: seam degree/knot harmonization (exact elevation + union
//!   refinement through `Curve::elevate` / `Curve::refine`) followed by a
//!   global minimum-norm least-squares solve (RRQR from
//!   `numerics::robust_solvers`) for the first interior control rows, with a
//!   simplified polynomial G1 ansatz `crossB = -crossA + β(v)·tA + γ(v)·tB`,
//!   `α ≡ -1` fixed, `β, γ` linear in the seam parameter.
//!
//! Limitations (documented contract): patches must be polynomial (uniform
//! weights), non-periodic, clamped at the fan boundaries, and the fan must
//! already be G0 within the position tolerance. The collocation solve is
//! verified afterwards at a denser sampling; the residual is reported.
use crate::{
    Result, check, numeric, resource,
    curve::{Curve, basis},
    surface::Surface,
    surface_join::Boundary,
};
use math_core::{cross, dot, norm};
use std::collections::HashMap;

/// Which corner of a patch touches the common vertex.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corner {
    MinMin,
    MaxMin,
    MinMax,
    MaxMax,
}
impl Corner {
    fn signs(self) -> (f64, f64) {
        match self {
            Corner::MinMin => (1., 1.),
            Corner::MaxMin => (-1., 1.),
            Corner::MinMax => (1., -1.),
            Corner::MaxMax => (-1., -1.),
        }
    }
    fn boundaries(self) -> [Boundary; 2] {
        match self {
            Corner::MinMin => [Boundary::UMin, Boundary::VMin],
            Corner::MaxMin => [Boundary::UMax, Boundary::VMin],
            Corner::MinMax => [Boundary::UMin, Boundary::VMax],
            Corner::MaxMax => [Boundary::UMax, Boundary::VMax],
        }
    }
}

/// One patch of the vertex fan, ordered counterclockwise around the vertex.
#[derive(Clone, Debug)]
pub struct VertexPatch {
    pub surface: Surface,
    pub corner: Corner,
    /// Boundary of this patch shared with the NEXT fan patch (cyclic).
    pub outgoing: Boundary,
}

/// Valence parity at the vertex; odd valences restrict solvability of the
/// classical vertex enclosure problem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValenceClass {
    Even,
    Odd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConflictKind {
    /// Patch corners do not coincide at the vertex.
    PositionGap,
    /// Seam tangents at the vertex are not parallel / not coplanar.
    TangentMismatch,
    /// First-order twist compatibility fails along the seam at the vertex.
    TwistMismatch,
}

#[derive(Clone, Copy, Debug)]
pub struct VertexConflict {
    /// Seam index: between patch `seam` and patch `(seam + 1) % valence`.
    pub seam: usize,
    pub kind: ConflictKind,
    pub residual: f64,
}

#[derive(Clone, Debug)]
pub struct VertexG1Report {
    pub valence: usize,
    pub valence_class: ValenceClass,
    /// True when no twist incompatibility was detected at the vertex.
    pub twist_compatible: bool,
    pub conflicts: Vec<VertexConflict>,
}

/// Result of a global fan join.
#[derive(Clone, Debug)]
pub struct FanJoin {
    pub surfaces: Vec<Surface>,
    pub report: VertexG1Report,
    /// Largest control-point displacement applied by the solve.
    pub max_shift: f64,
    /// Largest G1 collocation residual at the verification sampling.
    pub max_residual: f64,
}

fn domains(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn scale(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn pt(p: &[f64]) -> [f64; 3] {
    [p[0], p[1], p[2]]
}
fn same_boundary(a: Boundary, b: Boundary) -> bool {
    std::mem::discriminant(&a) == std::mem::discriminant(&b)
}

/// Corner jet oriented away from the vertex: point, incoming-seam tangent,
/// outgoing-seam tangent, incoming second derivative, outgoing second
/// derivative and the away-away mixed derivative (twist).
struct CornerJet {
    point: [f64; 3],
    t_in: [f64; 3],
    t_out: [f64; 3],
    tt_in: [f64; 3],
    tt_out: [f64; 3],
    twist: [f64; 3],
}

fn corner_jet(patch: &VertexPatch) -> Result<CornerJet> {
    let s = &patch.surface;
    let d = domains(s);
    let (su, sv) = patch.corner.signs();
    let u = if su > 0. { d[0][0] } else { d[0][1] };
    let v = if sv > 0. { d[1][0] } else { d[1][1] };
    let e = s.evaluate(u, v)?;
    let (du, dv) = e.first_derivatives().unwrap_or(([0.; 3], [0.; 3]));
    let duu = e.second_derivatives().map(|x| x.0).unwrap_or([0.; 3]);
    let duv = e.second_derivatives().map(|x| x.1).unwrap_or([0.; 3]);
    let dvv = e.second_derivatives().map(|x| x.2).unwrap_or([0.; 3]);
    // Along a u-fixed boundary the free direction is v and vice versa.
    let along = |b: Boundary| -> ([f64; 3], [f64; 3]) {
        match b {
            Boundary::UMin | Boundary::UMax => (scale(dv, sv), dvv),
            Boundary::VMin | Boundary::VMax => (scale(du, su), duu),
        }
    };
    let [b0, b1] = patch.corner.boundaries();
    let incoming = if same_boundary(patch.outgoing, b0) {
        b1
    } else {
        b0
    };
    let (t_out, tt_out) = along(patch.outgoing);
    let (t_in, tt_in) = along(incoming);
    Ok(CornerJet {
        point: e.point,
        t_in,
        t_out,
        tt_in,
        tt_out,
        twist: scale(duv, su * sv),
    })
}

fn validate_fan(fan: &[VertexPatch]) -> Result<()> {
    check(
        (3..=16).contains(&fan.len()),
        "A vertex fan needs 3..16 patches",
    )?;
    for patch in fan {
        patch.surface.validate()?;
        check(
            !patch.surface.periodic_u && !patch.surface.periodic_v,
            "Vertex fan patches must be non-periodic",
        )?;
        let [b0, b1] = patch.corner.boundaries();
        check(
            same_boundary(patch.outgoing, b0) || same_boundary(patch.outgoing, b1),
            "The outgoing boundary must touch the declared corner",
        )?;
    }
    Ok(())
}

/// Vertex enclosure diagnostics for an ordered patch fan.
pub fn analyze(
    fan: &[VertexPatch],
    position_tolerance: f64,
    derivative_tolerance: f64,
) -> Result<VertexG1Report> {
    validate_fan(fan)?;
    check(
        position_tolerance.is_finite()
            && position_tolerance >= 0.
            && derivative_tolerance.is_finite()
            && derivative_tolerance >= 0.,
        "Vertex analysis tolerances must be finite and nonnegative",
    )?;
    let n = fan.len();
    let jets: Vec<CornerJet> = fan.iter().map(corner_jet).collect::<Result<_>>()?;
    let mut conflicts = Vec::new();
    let center = {
        let mut c = [0.; 3];
        for j in &jets {
            c = add(c, j.point);
        }
        scale(c, 1. / n as f64)
    };
    for (i, j) in jets.iter().enumerate() {
        let residual = norm(sub(j.point, center));
        if residual > position_tolerance {
            conflicts.push(VertexConflict {
                seam: i,
                kind: ConflictKind::PositionGap,
                residual,
            });
        }
    }
    // Seam tangent away from the vertex: average of both sides' tangents.
    let mut tangents = Vec::with_capacity(n);
    for i in 0..n {
        let a = jets[i].t_out;
        let b = jets[(i + 1) % n].t_in;
        let (na, nb) = (norm(a), norm(b));
        numeric(
            na > 0. && nb > 0.,
            "Vertex fan has a vanishing seam tangent at the vertex",
        )?;
        let residual = norm(cross(scale(a, 1. / na), scale(b, 1. / nb)));
        if residual > derivative_tolerance {
            conflicts.push(VertexConflict {
                seam: i,
                kind: ConflictKind::TangentMismatch,
                residual,
            });
        }
        let sum = add(scale(a, 1. / na), scale(b, 1. / nb));
        let t = if norm(sum) > 1e-14 {
            scale(sum, 1. / norm(sum))
        } else {
            scale(a, 1. / na)
        };
        tangents.push(t);
    }
    // Common tangent-plane normal from the cyclic tangent polygon.
    let mut normal = [0.; 3];
    for i in 0..n {
        normal = add(normal, cross(tangents[i], tangents[(i + 1) % n]));
    }
    if norm(normal) < 1e-12 {
        normal = cross(tangents[0], tangents[1]);
    }
    numeric(
        norm(normal) > 1e-14,
        "Vertex fan seam tangents are degenerate (all parallel)",
    )?;
    normal = scale(normal, 1. / norm(normal));
    for (i, t) in tangents.iter().enumerate() {
        let residual = dot(*t, normal).abs();
        if residual > derivative_tolerance {
            conflicts.push(VertexConflict {
                seam: i,
                kind: ConflictKind::TangentMismatch,
                residual,
            });
        }
    }
    // First-order twist compatibility per seam: with the ansatz
    // crossB = -crossA + β(v)·t, differentiating into the vertex gives
    // wA + wB ∈ span{t, t''}. The residual is the orthogonal component.
    let mut twist_compatible = true;
    for i in 0..n {
        let a = &jets[i];
        let b = &jets[(i + 1) % n];
        let sign = if dot(a.t_out, b.t_in) >= 0. { 1. } else { -1. };
        let v = add(a.twist, scale(b.twist, sign));
        let t = tangents[i];
        let tt = scale(add(a.tt_out, scale(b.tt_in, sign)), 0.5);
        let axial = dot(tt, t);
        let bend = sub(tt, scale(t, axial));
        let residual = if norm(bend) > 1e-12 {
            let axis = cross(t, bend);
            dot(v, axis).abs() / norm(axis)
        } else {
            norm(sub(v, scale(t, dot(v, t))))
        };
        if residual > derivative_tolerance {
            twist_compatible = false;
            conflicts.push(VertexConflict {
                seam: i,
                kind: ConflictKind::TwistMismatch,
                residual,
            });
        }
    }
    Ok(VertexG1Report {
        valence: n,
        valence_class: if n % 2 == 0 {
            ValenceClass::Even
        } else {
            ValenceClass::Odd
        },
        twist_compatible,
        conflicts,
    })
}

/// Boundary curve of a surface (free axis of the boundary, as stored).
fn boundary_curve(s: &Surface, b: Boundary) -> Curve {
    match b {
        Boundary::UMin => Curve {
            degree: s.degree_v,
            knots: s.knots_v.clone(),
            control_points: s.control_points[0].clone(),
            weights: s.weights[0].clone(),
            periodic: s.periodic_v,
        },
        Boundary::UMax => Curve {
            degree: s.degree_v,
            knots: s.knots_v.clone(),
            control_points: s.control_points.last().unwrap().clone(),
            weights: s.weights.last().unwrap().clone(),
            periodic: s.periodic_v,
        },
        Boundary::VMin => Curve {
            degree: s.degree_u,
            knots: s.knots_u.clone(),
            control_points: s.control_points.iter().map(|r| r[0].clone()).collect(),
            weights: s.weights.iter().map(|r| r[0]).collect(),
            periodic: s.periodic_u,
        },
        Boundary::VMax => Curve {
            degree: s.degree_u,
            knots: s.knots_u.clone(),
            control_points: s
                .control_points
                .iter()
                .map(|r| r.last().unwrap().clone())
                .collect(),
            weights: s.weights.iter().map(|r| *r.last().unwrap()).collect(),
            periodic: s.periodic_u,
        },
    }
}

/// Whether the vertex sits at the start (false) or end (true) of the
/// boundary's stored parameter.
fn vertex_at_end(corner: Corner, b: Boundary) -> bool {
    let (su, sv) = corner.signs();
    match b {
        Boundary::UMin | Boundary::UMax => sv < 0.,
        Boundary::VMin | Boundary::VMax => su < 0.,
    }
}

fn incoming(patch: &VertexPatch) -> Boundary {
    let [b0, b1] = patch.corner.boundaries();
    if same_boundary(patch.outgoing, b0) {
        b1
    } else {
        b0
    }
}

/// Free axis of a boundary: true when the boundary parameter runs along U.
fn free_axis_u(b: Boundary) -> bool {
    matches!(b, Boundary::VMin | Boundary::VMax)
}

/// Clamped-end check for a boundary (multiplicity degree + 1 at both ends of
/// the free axis is not required — only at the boundary itself the cross axis
/// must be clamped). Returns true when the CROSS axis is clamped at the end
/// that defines `b`.
fn clamped_at(s: &Surface, b: Boundary) -> bool {
    let (knots, degree, count) = match b {
        Boundary::UMin | Boundary::UMax => (&s.knots_u, s.degree_u, s.control_points.len()),
        Boundary::VMin | Boundary::VMax => (&s.knots_v, s.degree_v, s.control_points[0].len()),
    };
    let start = knots[..=degree].iter().all(|&k| k == knots[0]);
    let end = knots[count..].iter().all(|&k| k == knots[count]);
    match b {
        Boundary::UMin | Boundary::VMin => start,
        Boundary::UMax | Boundary::VMax => end,
    }
}

/// Affine-rescale both knot vectors of `axis` of every surface so the active
/// domain becomes [0, 1].
fn normalized_axis(s: &Surface, axis_u: bool) -> Result<Surface> {
    let mut out = s.clone();
    let (knots, degree, count) = if axis_u {
        (&mut out.knots_u, out.degree_u, out.control_points.len())
    } else {
        (
            &mut out.knots_v,
            out.degree_v,
            out.control_points[0].len(),
        )
    };
    let a = knots[degree];
    let b = knots[count];
    check(b > a, "Vertex fan patches need a nonzero axis domain")?;
    if a != 0. || b != 1. {
        for k in knots.iter_mut() {
            *k = (*k - a) / (b - a);
        }
    }
    out.validate()?;
    Ok(out)
}

/// Multiplicity-limited union of interior knots of two seam curves after
/// elevation to `degree`; returns the insertion list for `curve`.
fn insertions(curve: &Curve, other: &Curve, degree: usize) -> Result<Vec<f64>> {
    let mut target: Vec<(f64, usize)> = Vec::new();
    for c in [curve, other] {
        let [a, b] = c.domain();
        let mut values: Vec<f64> = c
            .knots
            .iter()
            .copied()
            .filter(|k| *k > a && *k < b)
            .collect();
        values.dedup();
        for v in values {
            let mult = c.knots.iter().filter(|&&k| k == v).count().min(degree);
            match target.iter_mut().find(|(x, _)| *x == v) {
                Some((_, m)) => *m = (*m).max(mult),
                None => target.push((v, mult)),
            }
        }
    }
    target.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out = Vec::new();
    for (v, m) in target {
        let have = curve.knots.iter().filter(|&&k| k == v).count().min(degree);
        out.extend(std::iter::repeat_n(v, m.saturating_sub(have)));
    }
    Ok(out)
}

/// Harmonize degree and knots along every seam of the fan (exact elevation
/// and union refinement applied to whole surface axes).
fn harmonize(fan: &[VertexPatch]) -> Result<Vec<Surface>> {
    let n = fan.len();
    // Normalize every axis that carries a fan boundary to the [0, 1] domain.
    let mut surfaces: Vec<Surface> = fan
        .iter()
        .map(|p| {
            let mut s = p.surface.clone();
            if free_axis_u(p.outgoing) || free_axis_u(incoming(p)) {
                s = normalized_axis(&s, true)?;
            }
            if !free_axis_u(p.outgoing) || !free_axis_u(incoming(p)) {
                s = normalized_axis(&s, false)?;
            }
            Ok(s)
        })
        .collect::<Result<_>>()?;
    // Pairwise per seam: both sides elevate to the common degree, then refine
    // to the multiplicity-limited union of interior knots.
    for seam in 0..n {
        let next = (seam + 1) % n;
        let b_a = fan[seam].outgoing;
        let b_b = incoming(&fan[next]);
        let curve_a = boundary_curve(&surfaces[seam], b_a);
        let curve_b = boundary_curve(&surfaces[next], b_b);
        let degree = curve_a.degree.max(curve_b.degree);
        let elevated_a = curve_a.elevate(degree)?;
        let elevated_b = curve_b.elevate(degree)?;
        let ins_a = insertions(&elevated_a, &elevated_b, degree)?;
        let ins_b = insertions(&elevated_b, &elevated_a, degree)?;
        let axis_a = if free_axis_u(b_a) {
            crate::surface::Axis::U
        } else {
            crate::surface::Axis::V
        };
        let axis_b = if free_axis_u(b_b) {
            crate::surface::Axis::U
        } else {
            crate::surface::Axis::V
        };
        let (sa, sb) = if seam < next {
            let (before, after) = surfaces.split_at_mut(next);
            (&mut before[seam], &mut after[0])
        } else {
            let (before, after) = surfaces.split_at_mut(seam);
            (&mut after[0], &mut before[next])
        };
        *sa = sa.edit_axis(axis_a, |c| c.elevate(degree)?.refine(&ins_a))?;
        *sb = sb.edit_axis(axis_b, |c| c.elevate(degree)?.refine(&ins_b))?;
    }
    Ok(surfaces)
}

/// Cross-derivative/tangent row data at one boundary station.
struct StationRow {
    /// (control flat index (i, j), coefficient) for variable interior points.
    variable: Vec<((usize, usize), f64)>,
    constant: [f64; 3],
}

/// Linear expansion of the INTERIOR-POINTING cross-boundary derivative at
/// boundary `b`, free-axis parameter `t`, in the control points of `s`.
/// Interior-pointing means the derivative along the cross axis directed into
/// the patch interior (+ for Min boundaries, - for Max boundaries); the G1
/// ansatz below uses opposing interior directions on the two seam sides.
fn cross_row(s: &Surface, b: Boundary, t: f64) -> Result<StationRow> {
    let d = domains(s);
    let (fixed_u, max) = match b {
        Boundary::UMin => (true, false),
        Boundary::UMax => (true, true),
        Boundary::VMin => (false, false),
        Boundary::VMax => (false, true),
    };
    let inward = if max { -1. } else { 1. };
    let (nu, nv) = (s.control_points.len(), s.control_points[0].len());
    let (bf, bg) = if fixed_u {
        (
            basis(s.degree_u, &s.knots_u, nu, d[0][usize::from(max)], false)?,
            basis(s.degree_v, &s.knots_v, nv, t, false)?,
        )
    } else {
        (
            basis(s.degree_v, &s.knots_v, nv, d[1][usize::from(max)], false)?,
            basis(s.degree_u, &s.knots_u, nu, t, false)?,
        )
    };
    let mut variable = Vec::new();
    let mut constant = [0.; 3];
    for i in 0..nu {
        for j in 0..nv {
            let coefficient = if fixed_u {
                bf.d1[i] * bg.basis[j]
            } else {
                bg.basis[i] * bf.d1[j]
            } * inward;
            if coefficient == 0. {
                continue;
            }
            let boundary_row = if fixed_u {
                i == usize::from(max) * (nu - 1)
            } else {
                j == usize::from(max) * (nv - 1)
            };
            if boundary_row {
                constant = add(constant, scale(pt(&s.control_points[i][j]), coefficient));
            } else {
                variable.push(((i, j), coefficient));
            }
        }
    }
    Ok(StationRow { variable, constant })
}

/// Tangent of the boundary curve (fixed control rows only) at parameter `t`.
fn boundary_tangent(s: &Surface, b: Boundary, t: f64) -> Result<[f64; 3]> {
    let curve = boundary_curve(s, b);
    let e = curve.evaluate(t)?;
    Ok(e.d1.as_deref().map(pt).unwrap_or([0.; 3]))
}

/// Harmonize, solve and apply a global minimum-norm G1 join around the fan
/// vertex. `stations_per_span` controls the collocation density per seam span
/// (clamped to 2..=8); the verification sampling is three times denser.
pub fn join_fan(
    fan: &[VertexPatch],
    position_tolerance: f64,
    stations_per_span: usize,
) -> Result<FanJoin> {
    validate_fan(fan)?;
    check(
        position_tolerance.is_finite() && position_tolerance >= 0.,
        "Fan join position tolerance must be finite and nonnegative",
    )?;
    for patch in fan {
        let (min, max) = patch
            .surface
            .weights
            .iter()
            .flatten()
            .fold((f64::INFINITY, f64::MIN), |(lo, hi), &w| (lo.min(w), hi.max(w)));
        check(
            max == min,
            "Fan join currently requires polynomial patches (uniform weights)",
        )?;
    }
    let n = fan.len();
    let reverse: Vec<bool> = (0..n)
        .map(|seam| {
            vertex_at_end(fan[seam].corner, fan[seam].outgoing)
                != vertex_at_end(fan[(seam + 1) % n].corner, incoming(&fan[(seam + 1) % n]))
        })
        .collect();
    let mut surfaces = harmonize(fan)?;
    // The cross-derivative collocation reads only the boundary row and the
    // first interior row; that requires clamped cross axes at every fan seam.
    for (index, patch) in fan.iter().enumerate() {
        for b in [patch.outgoing, incoming(patch)] {
            check(
                clamped_at(&surfaces[index], b),
                "Fan join requires clamped knot vectors at the fan boundaries",
            )?;
        }
    }
    // G0 gate on the harmonized boundary curves.
    for seam in 0..n {
        let next = (seam + 1) % n;
        let a = boundary_curve(&surfaces[seam], fan[seam].outgoing);
        let mut b = boundary_curve(&surfaces[next], incoming(&fan[next]));
        if reverse[seam] {
            b = b.reverse()?;
        }
        check(
            a.degree == b.degree && a.knots.len() == b.knots.len(),
            "Seam harmonization failed to align the bases",
        )?;
        let gap = a
            .control_points
            .iter()
            .zip(&b.control_points)
            .map(|(p, q)| {
                let d: [f64; 3] = [
                    p[0] - q[0],
                    p[1] - q[1],
                    p[2] - q[2],
                ];
                norm(d)
            })
            .fold(0_f64, f64::max);
        a.validate()?;
        check(
            gap <= position_tolerance,
            "Fan join requires a G0 fan; harmonized seam control points differ",
        )?;
    }
    // Variable control points: first interior row adjacent to each fan seam.
    let mut columns: HashMap<(usize, usize, usize), usize> = HashMap::new();
    for (index, patch) in fan.iter().enumerate() {
        let s = &surfaces[index];
        let (nu, nv) = (s.control_points.len(), s.control_points[0].len());
        for b in [patch.outgoing, incoming(patch)] {
            match b {
                Boundary::UMin if nu > 2 => {
                    for j in 0..nv {
                        let c = columns.len();
                        columns.entry((index, 1, j)).or_insert(c);
                    }
                }
                Boundary::UMax if nu > 2 => {
                    for j in 0..nv {
                        let c = columns.len();
                        columns.entry((index, nu - 2, j)).or_insert(c);
                    }
                }
                Boundary::VMin if nv > 2 => {
                    for i in 0..nu {
                        let c = columns.len();
                        columns.entry((index, i, 1)).or_insert(c);
                    }
                }
                Boundary::VMax if nv > 2 => {
                    for i in 0..nu {
                        let c = columns.len();
                        columns.entry((index, i, nv - 2)).or_insert(c);
                    }
                }
                _ => {}
            }
        }
    }
    // Preserve both authored fan boundary curves, including their intersection.
    columns.retain(|&(patch, i, j), _| {
        let s = &surfaces[patch];
        ![fan[patch].outgoing, incoming(&fan[patch])].iter().any(|b| match b {
            Boundary::UMin => i == 0,
            Boundary::UMax => i + 1 == s.control_points.len(),
            Boundary::VMin => j == 0,
            Boundary::VMax => j + 1 == s.control_points[0].len(),
        })
    });
    for (index, value) in columns.values_mut().enumerate() { *value = index; }
    let variable_points = columns.len();
    let total_columns = 3 * variable_points + 4 * n;
    // Collocation stations per seam.
    let per_span = stations_per_span.clamp(2, 8);
    let mut stations: Vec<Vec<f64>> = Vec::with_capacity(n);
    for seam in 0..n {
        let s = &surfaces[seam];
        let b = fan[seam].outgoing;
        let (knots, degree, count) = match b {
            Boundary::UMin | Boundary::UMax => (&s.knots_v, s.degree_v, s.control_points[0].len()),
            Boundary::VMin | Boundary::VMax => (&s.knots_u, s.degree_u, s.control_points.len()),
        };
        let mut breaks: Vec<f64> = knots
            .iter()
            .copied()
            .filter(|k| *k >= 0. && *k <= 1.)
            .collect();
        breaks.dedup();
        let _ = count;
        let mut ts = Vec::new();
        for w in breaks.array_windows() {
            let [lo, hi] = *w;
            if hi <= lo {
                continue;
            }
            for i in 0..per_span {
                let t = lo + (hi - lo) * (i as f64 + 0.5) / per_span as f64;
                ts.push(t);
            }
            let _ = degree;
        }
        ts.push(0.);
        ts.push(1.);
        ts.sort_by(f64::total_cmp);
        ts.dedup();
        stations.push(ts);
    }
    let total_rows: usize = stations.iter().map(|s| 3 * s.len()).sum();
    if total_columns > 4096 || total_rows > 16384 {
        return Err(resource("Fan join system exceeds 4096 unknowns / 16384 rows"));
    }
    let mut matrix = vec![vec![0.; total_columns]; total_rows];
    let mut rhs = vec![0.; total_rows];
    let mut row = 0;
    for seam in 0..n {
        let next = (seam + 1) % n;
        let b_a = fan[seam].outgoing;
        let b_b = incoming(&fan[next]);
        for &t in &stations[seam] {
            let tb = if reverse[seam] { 1. - t } else { t };
            let ra = cross_row(&surfaces[seam], b_a, t)?;
            let rb = cross_row(&surfaces[next], b_b, tb)?;
            let ta = boundary_tangent(&surfaces[seam], b_a, t)?;
            let tbv = boundary_tangent(&surfaces[next], b_b, tb)?;
            // Unknown point triples are displacements, so cancel the complete
            // current derivative, including existing interior poles.
            let mut constant = add(ra.constant, rb.constant);
            for ((i, j), coefficient) in &ra.variable {
                constant = add(constant, scale(pt(&surfaces[seam].control_points[*i][*j]), *coefficient));
            }
            for ((i, j), coefficient) in &rb.variable {
                constant = add(constant, scale(pt(&surfaces[next].control_points[*i][*j]), *coefficient));
            }
            for axis in 0..3 {
                let r = &mut matrix[row];
                for ((i, j), coefficient) in &ra.variable {
                    if let Some(point) = columns.get(&(seam, *i, *j)) {
                        r[3 * point + axis] += coefficient;
                    }
                }
                for ((i, j), coefficient) in &rb.variable {
                    if let Some(point) = columns.get(&(next, *i, *j)) {
                        r[3 * point + axis] += coefficient;
                    }
                }
                let base = 3 * variable_points + 4 * seam;
                r[base] = -ta[axis];
                r[base + 1] = -t * ta[axis];
                r[base + 2] = -tbv[axis];
                r[base + 3] = -t * tbv[axis];
                rhs[row] = -constant[axis];
                row += 1;
            }
        }
    }
    let factorization = numerics_rrqr(&matrix)?;
    let solution = factorization.solve_least_squares(&rhs)?;
    // Apply: point columns hold xyz triples; seam scalars are auxiliary.
    let mut max_shift = 0_f64;
    for (&(patch, i, j), &point) in columns.iter() {
        let shift = [
            solution[3 * point],
            solution[3 * point + 1],
            solution[3 * point + 2],
        ];
        max_shift = f64::max(max_shift, norm(shift));
        let target = &mut surfaces[patch].control_points[i][j];
        for axis in 0..3 {
            target[axis] += shift[axis];
        }
    }
    for s in &mut surfaces {
        s.validate()?;
    }
    // Verification at a denser sampling using the solved seam scalars.
    let mut max_residual = 0_f64;
    for seam in 0..n {
        let next = (seam + 1) % n;
        let base = 3 * variable_points + 4 * seam;
        let (b0, b1, c0, c1) = (
            solution[base],
            solution[base + 1],
            solution[base + 2],
            solution[base + 3],
        );
        for &t in &stations[seam] {
            for tt in [t, (t + 0.5).min(1.)] {
                let tb = if reverse[seam] { 1. - tt } else { tt };
                let ra = cross_row(&surfaces[seam], fan[seam].outgoing, tt)?;
                let rb = cross_row(&surfaces[next], incoming(&fan[next]), tb)?;
                let ta = boundary_tangent(&surfaces[seam], fan[seam].outgoing, tt)?;
                let tbv = boundary_tangent(&surfaces[next], incoming(&fan[next]), tb)?;
                let mut total = add(ra.constant, rb.constant);
                for ((i, j), coefficient) in &ra.variable {
                    total = add(
                        total,
                        scale(pt(&surfaces[seam].control_points[*i][*j]), *coefficient),
                    );
                }
                for ((i, j), coefficient) in &rb.variable {
                    total = add(
                        total,
                        scale(pt(&surfaces[next].control_points[*i][*j]), *coefficient),
                    );
                }
                let residual = norm(add(
                    sub(total, scale(ta, b0 + b1 * tt)),
                    scale(tbv, -(c0 + c1 * tt)),
                ));
                max_residual = f64::max(max_residual, residual);
            }
        }
    }
    let joined: Vec<VertexPatch> = fan
        .iter()
        .zip(surfaces.iter())
        .map(|(p, s)| VertexPatch {
            surface: s.clone(),
            corner: p.corner,
            outgoing: p.outgoing,
        })
        .collect();
    let report = analyze(
        &joined,
        position_tolerance,
        f64::max(1e-9, 10. * position_tolerance),
    )?;
    Ok(FanJoin {
        surfaces,
        report,
        max_shift,
        max_residual,
    })
}

fn numerics_rrqr(matrix: &[Vec<f64>]) -> Result<crate::numerics::robust_solvers::Rrqr> {
    crate::numerics::robust_solvers::rrqr(matrix, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bezier_surface(points: [[ [f64; 3]; 4 ]; 4]) -> Surface {
        Surface {
            degree_u: 3,
            degree_v: 3,
            knots_u: [vec![0.; 4], vec![2.; 4]].concat(),
            knots_v: [vec![0.; 4], vec![2.; 4]].concat(),
            control_points: points.iter().map(|r| r.iter().map(|p| p.to_vec()).collect()).collect(),
            weights: vec![vec![1.; 4]; 4],
            periodic_u: false,
            periodic_v: false,
        }
    }

    fn bump() -> Surface {
        let net = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                let x = 2. * i as f64 / 3.;
                let y = 2. * j as f64 / 3.;
                [x, y, 0.4 * x * y * (2. - x) * (2. - y) + 0.1 * x]
            })
        });
        bezier_surface(net)
    }

    fn quadrant(s: &Surface, u: [f64; 2], v: [f64; 2]) -> Surface {
        let s = s
            .edit_axis(crate::surface::Axis::U, |c| c.trim(u[0], u[1]))
            .unwrap()
            .edit_axis(crate::surface::Axis::V, |c| c.trim(v[0], v[1]))
            .unwrap();
        normalized_axis(&normalized_axis(&s, true).unwrap(), false).unwrap()
    }

    /// Fan of four quadrants of a smooth bump around its center (1, 1).
    fn fan() -> Vec<VertexPatch> {
        let s = bump();
        vec![
            VertexPatch {
                surface: quadrant(&s, [0., 1.], [0., 1.]),
                corner: Corner::MaxMax,
                outgoing: Boundary::UMax,
            },
            VertexPatch {
                surface: quadrant(&s, [1., 2.], [0., 1.]),
                corner: Corner::MinMax,
                outgoing: Boundary::VMax,
            },
            VertexPatch {
                surface: quadrant(&s, [1., 2.], [1., 2.]),
                corner: Corner::MinMin,
                outgoing: Boundary::UMin,
            },
            VertexPatch {
                surface: quadrant(&s, [0., 1.], [1., 2.]),
                corner: Corner::MaxMin,
                outgoing: Boundary::VMin,
            },
        ]
    }

    fn seam_normals_fan(a: &Surface, ea: Boundary, b: &Surface, eb: Boundary, samples: usize) -> f64 {
        let mut worst = 0_f64;
        for i in 0..=samples {
            let t = i as f64 / samples as f64;
            let pa = match ea {
                Boundary::UMin => a.evaluate(0., t).unwrap(),
                Boundary::UMax => a.evaluate(1., t).unwrap(),
                Boundary::VMin => a.evaluate(t, 0.).unwrap(),
                Boundary::VMax => a.evaluate(t, 1.).unwrap(),
            };
            let pb = match eb {
                Boundary::UMin => b.evaluate(0., t).unwrap(),
                Boundary::UMax => b.evaluate(1., t).unwrap(),
                Boundary::VMin => b.evaluate(t, 0.).unwrap(),
                Boundary::VMax => b.evaluate(t, 1.).unwrap(),
            };
            let na = pa.unit_normal().unwrap();
            let nb = pb.unit_normal().unwrap();
            worst = f64::max(worst, norm(cross(na, nb)));
        }
        worst
    }

    #[test]
    fn smooth_fan_analyzes_compatible_and_join_is_near_identity() {
        let f = fan();
        let report = analyze(&f, 1e-9, 1e-9).unwrap();
        assert_eq!(report.valence, 4);
        assert_eq!(report.valence_class, ValenceClass::Even);
        assert!(
            report.conflicts.is_empty(),
            "unexpected conflicts: {:?}",
            report.conflicts
        );
        assert!(report.twist_compatible);
        let joined = join_fan(&f, 1e-9, 4).unwrap();
        assert!(joined.max_shift < 1e-6, "shift {}", joined.max_shift);
        assert!(joined.max_residual < 1e-6, "residual {}", joined.max_residual);
    }

    #[test]
    fn perturbed_twist_is_diagnosed_and_join_restores_g1() {
        let mut f = fan();
        // Break the away-away twist at the shared corner of patch 0 (MaxMax):
        // the diagonally adjacent interior control point only affects duv.
        // The x shift keeps the defect visible: the seam osculating plane at
        // the vertex contains z, so a z twist jump is absorbable by beta(0).
        f[0].surface.control_points[2][2][0] += 0.3;
        let report = analyze(&f, 1e-9, 1e-6).unwrap();
        assert!(!report.twist_compatible);
        assert!(
            report
                .conflicts
                .iter()
                .any(|c| c.kind == ConflictKind::TwistMismatch),
            "expected a twist conflict: {:?}",
            report.conflicts
        );
        let joined = join_fan(&f, 1e-9, 4).unwrap();
        assert!(
            joined.max_residual < 1e-6,
            "residual {}",
            joined.max_residual
        );
        // The join may move interior poles, but both authored G0 boundaries
        // must remain identical after harmonization.
        let original = harmonize(&f).unwrap();
        for (index, patch) in f.iter().enumerate() {
            for boundary in [patch.outgoing, incoming(patch)] {
                assert_eq!(
                    boundary_curve(&original[index], boundary).control_points,
                    boundary_curve(&joined.surfaces[index], boundary).control_points,
                );
            }
        }
        // Verify normal agreement along every seam of the joined fan.
        for seam in 0..4 {
            let next = (seam + 1) % 4;
            let worst = seam_normals_fan(
                &joined.surfaces[seam],
                f[seam].outgoing,
                &joined.surfaces[next],
                incoming(&f[next]),
                16,
            );
            assert!(worst < 1e-4, "seam {seam} normal jump {worst}");
        }
    }

    #[test]
    fn polynomial_join_accepts_uniform_weight_scales_and_refuses_near_uniform_weights() {
        let mut f = fan();
        for (index, patch) in f.iter_mut().enumerate() {
            for row in &mut patch.surface.weights {
                row.fill((index + 1) as f64);
            }
        }
        let joined = join_fan(&f, 1e-9, 4).unwrap();
        assert!(joined.max_shift < 1e-6);
        assert!(joined.max_residual < 1e-6);
        // Even a small positive weight defect invalidates the polynomial
        // derivative rows; a modeling tolerance cannot make them rational.
        f[0].surface.weights[2][2] += 1e-13;
        assert!(join_fan(&f, 1e-9, 4).is_err());
    }

    #[test]
    fn cube_corner_reports_noncoplanar_tangents_and_odd_valence() {
        // Three unit-square faces of a cube around the origin.
        let face = |rows: [[f64; 3]; 4]| Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: rows
                .chunks(2)
                .map(|c| c.iter().map(|p| p.to_vec()).collect())
                .collect(),
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let xy = face([[0., 0., 0.], [0., 1., 0.], [1., 0., 0.], [1., 1., 0.]]);
        let yz = face([[0., 0., 0.], [0., 0., 1.], [0., 1., 0.], [0., 1., 1.]]);
        let zx = face([[0., 0., 0.], [1., 0., 0.], [0., 0., 1.], [1., 0., 1.]]);
        let fan = vec![
            VertexPatch {
                surface: xy,
                corner: Corner::MinMin,
                outgoing: Boundary::UMin,
            },
            VertexPatch {
                surface: yz,
                corner: Corner::MinMin,
                outgoing: Boundary::UMin,
            },
            VertexPatch {
                surface: zx,
                corner: Corner::MinMin,
                outgoing: Boundary::VMin,
            },
        ];
        let report = analyze(&fan, 1e-9, 1e-9).unwrap();
        assert_eq!(report.valence_class, ValenceClass::Odd);
        assert!(
            report
                .conflicts
                .iter()
                .any(|c| c.kind == ConflictKind::TangentMismatch),
            "cube corner must fail tangent coplanarity: {:?}",
            report.conflicts
        );
    }

    #[test]
    fn position_gap_is_reported() {
        let mut f = fan();
        f[2].surface.control_points[0][0][2] += 0.05;
        let report = analyze(&f, 1e-6, 1e-6).unwrap();
        assert!(
            report
                .conflicts
                .iter()
                .any(|c| c.kind == ConflictKind::PositionGap)
        );
    }
}
