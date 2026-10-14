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

mod boundaries;
use boundaries::*;


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
#[path = "tests/multipatch_g1.rs"]
mod tests;
