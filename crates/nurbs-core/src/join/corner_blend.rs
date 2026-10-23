//! Four-sided rational C2 corner proposals with original-support G2 admission.
//! Supports may have independent curvature. This does not solve radius laws,
//! infer trim loops, replace a B-rep corner, or certify global embedding.
use crate::{
    Result, check,
    curve::Curve,
    surface::Surface,
    surface_blend::{self, Boundary},
};
#[path = "corner_blend/mapped.rs"]
pub mod mapped;
#[path = "corner_blend/proof.rs"]
mod proof;

#[derive(Clone, Copy)]
pub struct Options {
    /// Supports corresponding to patch UMin, UMax, VMin, VMax.
    pub boundaries: [Boundary; 4],
    pub reverse_along: [bool; 4],
    /// Source transverse parameter units per patch parameter unit.
    pub gains: [f64; 4],
    /// Proposal-only compatibility gate on normalized homogeneous controls.
    pub compatibility_tolerance: f64,
    pub max_controls: usize,
}
/// An isoparametric trace on the immutable original support. The selected
/// boundary orientation in Options defines the retained side of `at`.
#[derive(Clone, Copy)]
pub struct Trace {
    pub at: f64,
    pub along: [f64; 2],
}
pub(super) fn natural_trace(s: &Surface, b: Boundary) -> Trace {
    let d = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let cross = usize::from(matches!(b, Boundary::VStart | Boundary::VEnd));
    Trace {
        at: d[cross][usize::from(end(b))],
        along: d[1 - cross],
    }
}
pub struct Report {
    pub surface: Surface,
    pub seams: Vec<crate::surface_g2::Report>,
    pub cells: usize,
    pub coefficient_work: usize,
    pub exact_work: u64,
    pub complete: bool,
    pub within_tolerance: bool,
    pub interior: crate::surface_linear_monotonicity::Report,
    /// All seams qualify and the whole candidate is a regular injective chart.
    /// Other faces, thickness and global body embedding are still separate.
    pub admitted: bool,
}
fn end(b: Boundary) -> bool {
    matches!(b, Boundary::UEnd | Boundary::VEnd)
}
fn packed(f: &[Curve], jet: usize, i: usize) -> [f64; 4] {
    let xyw = &f[2 * jet].control_points[i];
    [xyw[0], xyw[1], f[2 * jet + 1].control_points[i][0], xyw[2]]
}
fn layers(
    f: &[Curve],
    i: usize,
    knots: &[f64],
    degree: usize,
    count: usize,
    reverse: bool,
) -> [[f64; 4]; 3] {
    let h1 = if reverse {
        knots[count] - knots[count - 1]
    } else {
        knots[degree + 1] - knots[degree]
    };
    let h2 = if reverse {
        knots[count] - knots[count - 2]
    } else {
        knots[degree + 2] - knots[degree]
    };
    let h = packed(f, 0, i);
    let d = packed(f, 1, i);
    let dd = packed(f, 2, i);
    let one = std::array::from_fn(|k| h[k] + d[k] * h1 / degree as f64);
    let two = std::array::from_fn(|k| {
        one[k] + h2 / degree as f64 * (d[k] + dd[k] * h1 / (degree - 1) as f64)
    });
    [h, one, two]
}
/// Construct a tensor-product rational corner by imposing complete homogeneous
/// C0/C1/C2 boundary fields from all four original supports. Shared corner blocks
/// must agree; incompatible jets refuse rather than being silently averaged.
/// Rounded basis edits and all retained controls require fresh whole-boundary
/// qualification against ORIGINAL supports. Only that qualification sets flags.
pub fn g2(
    supports: [&Surface; 4],
    o: Options,
    tolerances: [f64; 3],
    max_cells: usize,
) -> Result<Report> {
    g2_with_exact_limit(supports, o, tolerances, max_cells, 1000000)
}
/// As `g2`, with an explicit shared exact-predicate budget for knot ownership.
pub fn g2_with_exact_limit(
    supports: [&Surface; 4],
    o: Options,
    tolerances: [f64; 3],
    max_cells: usize,
    max_exact: u64,
) -> Result<Report> {
    for s in supports {
        s.validate()?;
    }
    let traces = std::array::from_fn(|i| natural_trace(supports[i], o.boundaries[i]));
    g2_traces(supports, traces, o, tolerances, max_cells, max_exact)
}
/// Construct from complete or partial isoparametric traces, including cuts
/// inside the original surface. Trimmed numerical jet fields only propose
/// geometry; all qualification evaluates the immutable original supports.
pub fn g2_traces(
    supports: [&Surface; 4],
    traces: [Trace; 4],
    o: Options,
    tolerances: [f64; 3],
    max_cells: usize,
    max_exact: u64,
) -> Result<Report> {
    g2_traces_bounded(
        supports, traces, o, tolerances, max_cells, max_exact, 2000000,
    )
}
fn g2_traces_bounded(
    supports: [&Surface; 4],
    traces: [Trace; 4],
    o: Options,
    tolerances: [f64; 3],
    max_cells: usize,
    max_exact: u64,
    max_coefficients: usize,
) -> Result<Report> {
    check(
        max_coefficients <= 2000000,
        "Corner coefficient budget exceeds limit",
    )?;
    check(
        o.gains.iter().all(|g| g.is_finite() && *g > 0.)
            && o.compatibility_tolerance.is_finite()
            && o.compatibility_tolerance >= 0.
            && (36..=4096).contains(&o.max_controls)
            && max_cells <= 100000
            && max_exact <= 1000000,
        "Corner blend requires finite gains and bounded controls/work",
    )?;
    check(
        tolerances.iter().all(|t| t.is_finite() && *t >= 0.) && tolerances[1] <= 2.,
        "Invalid corner qualification tolerances",
    )?;
    let mut fields = Vec::with_capacity(4);
    for side in 0..4 {
        let s = surface_blend::canonical(supports[side], o.boundaries[side])?;
        let along = [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]];
        let cross = [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]];
        let t = traces[side];
        check(
            t.at.is_finite()
                && t.along.iter().all(|x| x.is_finite())
                && along[0] <= t.along[0]
                && t.along[0] < t.along[1]
                && t.along[1] <= along[1]
                && cross[0] <= t.at
                && t.at <= cross[1]
                && if end(o.boundaries[side]) {
                    cross[0] < t.at
                } else {
                    t.at < cross[1]
                },
            "Corner trace must lie within the original support with a nonempty retained side",
        )?;
        check(
            !s.periodic_u && !s.periodic_v,
            "Periodic corner supports need lifted charts",
        )?;
        check(
            s.control_points
                .len()
                .checked_mul(s.control_points[0].len())
                .is_some_and(|n| n <= 4096),
            "Corner source control budget exceeded",
        )?;
        let sign = if end(o.boundaries[side]) { 1. } else { -1. };
        let mut f = surface_blend::fields_at(&s, t.at, sign * o.gains[side], false, 2)?;
        for c in &mut f {
            *c = c.trim(t.along[0], t.along[1])?;
            if o.reverse_along[side] {
                *c = c.reverse()?;
            }
            if c.degree < 5 {
                *c = c.elevate(5)?;
            }
        }
        fields.push(f);
    }
    for pair in [[0, 1], [2, 3]] {
        let all = fields[pair[0]]
            .iter()
            .chain(&fields[pair[1]])
            .cloned()
            .collect::<Vec<_>>();
        let aligned = crate::sections::compatible(&all)?;
        // Elevation currently stores separate Bezier spans. Restore a C2
        // proposal basis; refitting does NOT supply proof. All final jets are
        // independently compared to the immutable originals below.
        let mut knots = aligned[0].knots.clone();
        let degree = aligned[0].degree;
        let mut i = degree + 1;
        while i < knots.len() - degree - 1 {
            let k = knots[i];
            let count = knots.iter().filter(|&&v| v == k).count();
            for _ in degree - 2..count {
                knots.remove(i);
            }
            i += count.min(degree - 2);
        }
        let aligned = aligned
            .into_iter()
            .map(|c| {
                if c.knots == knots {
                    Ok(c)
                } else {
                    crate::foundation::refit_curve(&c, degree, knots.clone())
                }
            })
            .collect::<Result<Vec<_>>>()?;
        fields[pair[0]] = aligned[..6].to_vec();
        fields[pair[1]] = aligned[6..].to_vec();
    }
    // A constant projective gauge per support leaves every Cartesian jet intact.
    let w = |side: usize, last: bool| {
        let c = &fields[side][0];
        c.control_points[if last { c.control_points.len() - 1 } else { 0 }][2]
    };
    let f2 = w(0, false) / w(2, false);
    let f1 = f2 * w(2, true) / w(1, false);
    let f3 = w(0, true) / w(3, false);
    for (side, factor) in [1., f1, f2, f3].into_iter().enumerate() {
        check(
            factor.is_finite() && factor > 0.,
            "Corner projective gauge is invalid",
        )?;
        for c in &mut fields[side] {
            for p in &mut c.control_points {
                for x in p {
                    *x *= factor;
                }
            }
        }
    }
    let u = &fields[2][0];
    let v = &fields[0][0];
    let nu = u.control_points.len();
    let nv = v.control_points.len();
    check(
        nu.checked_mul(nv).is_some_and(|n| n <= o.max_controls),
        "Corner tensor control budget exceeded",
    )?;
    // A simple tensor patch needs C2-compatible internal knots; broken source
    // spans cannot be hidden by a successful boundary report.
    for c in [u, v] {
        let mut i = c.degree + 1;
        while i < c.control_points.len() {
            let mut j = i + 1;
            while j < c.knots.len() && c.knots[j] == c.knots[i] {
                j += 1;
            }
            check(
                j - i <= c.degree - 2,
                "Corner fields contain a non-C2 interior knot",
            )?;
            i = j;
        }
    }
    let mut net = vec![vec![None::<[f64; 4]>; nv]; nu];
    for side in 0..4 {
        let along = if side < 2 { nv } else { nu };
        let transverse = if side < 2 { u } else { v };
        let count = transverse.control_points.len();
        let reverse = side == 1 || side == 3;
        for i in 0..along {
            for (layer, h) in layers(
                &fields[side],
                i,
                &transverse.knots,
                transverse.degree,
                count,
                reverse,
            )
            .into_iter()
            .enumerate()
            {
                check(
                    h.iter().all(|x| x.is_finite()) && h[3] > 0.,
                    "Corner layer has invalid homogeneous weights",
                )?;
                let t = if reverse { count - 1 - layer } else { layer };
                let (a, b) = if side < 2 { (t, i) } else { (i, t) };
                if let Some(old) = net[a][b] {
                    check(
                        old.iter()
                            .zip(h)
                            .all(|(x, y)| (x - y).abs() <= o.compatibility_tolerance),
                        "Corner boundary jet blocks are incompatible",
                    )?;
                } else {
                    net[a][b] = Some(h);
                }
            }
        }
    }
    let corners = [
        net[0][0].unwrap(),
        net[nu - 1][0].unwrap(),
        net[0][nv - 1].unwrap(),
        net[nu - 1][nv - 1].unwrap(),
    ];
    let mut points = vec![vec![vec![0.; 3]; nv]; nu];
    let mut weights = vec![vec![1.; nv]; nu];
    for i in 0..nu {
        for j in 0..nv {
            let h = net[i][j].unwrap_or_else(|| {
                let a = i as f64 / (nu - 1) as f64;
                let b = j as f64 / (nv - 1) as f64;
                std::array::from_fn(|k| {
                    (1. - a) * (1. - b) * corners[0][k]
                        + a * (1. - b) * corners[1][k]
                        + (1. - a) * b * corners[2][k]
                        + a * b * corners[3][k]
                })
            });
            weights[i][j] = h[3];
            for k in 0..3 {
                points[i][j][k] = h[k] / h[3];
            }
        }
    }
    let surface = Surface {
        degree_u: u.degree,
        degree_v: v.degree,
        knots_u: u.knots.clone(),
        knots_v: v.knots.clone(),
        control_points: points,
        weights,
        periodic_u: false,
        periodic_v: false,
    };
    inspect_candidate_bounded(
        &surface,
        supports,
        traces,
        o,
        tolerances,
        max_cells,
        max_exact,
        max_coefficients,
    )
}
/// Fresh admission of an actual candidate: all four ORIGINAL-support seams and
/// the complete candidate interior share the caller's work ledger. Supplied
/// construction reports never authorize this result.
pub fn inspect_candidate(
    surface: &Surface,
    supports: [&Surface; 4],
    traces: [Trace; 4],
    o: Options,
    tolerances: [f64; 3],
    max_cells: usize,
    max_exact: u64,
) -> Result<Report> {
    inspect_candidate_bounded(
        surface, supports, traces, o, tolerances, max_cells, max_exact, 2000000,
    )
}
fn inspect_candidate_bounded(
    surface: &Surface,
    supports: [&Surface; 4],
    traces: [Trace; 4],
    o: Options,
    tolerances: [f64; 3],
    max_cells: usize,
    max_exact: u64,
    max_coefficients: usize,
) -> Result<Report> {
    surface.validate()?;
    check(
        max_cells <= 100000
            && max_exact <= 1000000
            && (36..=4096).contains(&o.max_controls)
            && o.gains.iter().all(|g| g.is_finite() && *g > 0.)
            && tolerances.iter().all(|t| t.is_finite() && *t >= 0.)
            && tolerances[1] <= 2.
            && !surface.periodic_u
            && !surface.periodic_v
            && surface
                .control_points
                .len()
                .checked_mul(surface.control_points[0].len())
                .is_some_and(|n| n <= o.max_controls),
        "Candidate corner admission needs bounded finite inputs",
    )?;
    for i in 0..4 {
        let s = surface_blend::canonical(supports[i], o.boundaries[i])?;
        let t = traces[i];
        let along = [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]];
        let cross = [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]];
        check(
            !s.periodic_u
                && !s.periodic_v
                && s.control_points
                    .len()
                    .checked_mul(s.control_points[0].len())
                    .is_some_and(|n| n <= 4096)
                && t.at.is_finite()
                && t.along.iter().all(|x| x.is_finite())
                && along[0] <= t.along[0]
                && t.along[0] < t.along[1]
                && t.along[1] <= along[1]
                && cross[0] <= t.at
                && t.at <= cross[1]
                && if end(o.boundaries[i]) {
                    cross[0] < t.at
                } else {
                    t.at < cross[1]
                },
            "Candidate seam must use a bounded immutable original trace",
        )?;
    }
    let mut report = Report {
        surface: surface.clone(),
        seams: Vec::with_capacity(4),
        cells: 0,
        coefficient_work: 0,
        exact_work: 0,
        complete: true,
        within_tolerance: true,
        interior: crate::surface_linear_monotonicity::Report {
            certified: false,
            cells: 0,
            projection: [[1, 0, 0], [0, 1, 0]],
            row_weights: [1.,1.],
            profile_parameterization: None,
            reason: Some("corner-interior-uninspected"),
        },
        admitted: false,
    };
    use crate::surface_join::Decision;
    let mut work = proof::Work {
        cells: 0,
        coefficients: 0,
        max_coefficients,
        max_cells,
        exact_work: 0,
        max_exact,
    };
    for side in 0..4 {
        if report.cells == max_cells {
            report.complete = false;
            report.within_tolerance = false;
            continue;
        }
        let proof = proof::inspect(
            &report.surface,
            [
                Boundary::UStart,
                Boundary::UEnd,
                Boundary::VStart,
                Boundary::VEnd,
            ][side],
            supports[side],
            o.boundaries[side],
            traces[side],
            o.reverse_along[side],
            o.gains[side],
            tolerances,
            &mut work,
        )?;
        report.cells += proof.cells;
        report.complete &= proof.decision != Decision::Unresolved;
        report.within_tolerance &= proof.decision == Decision::WithinTolerance;
        report.seams.push(proof);
    }
    report.coefficient_work = work.coefficients;
    report.exact_work = work.exact_work;
    let mut c2 = true;
    for (knots, p, n) in [
        (
            &surface.knots_u,
            surface.degree_u,
            surface.control_points.len(),
        ),
        (
            &surface.knots_v,
            surface.degree_v,
            surface.control_points[0].len(),
        ),
    ] {
        c2 &= !knots.iter().any(|&k| {
            knots[p] < k
                && k < knots[n]
                && knots.iter().filter(|&&x| x == k).count() > p.saturating_sub(2)
        });
    }
    if c2 {
        report.interior = crate::surface_linear_monotonicity::inspect_candidate(
            surface,
            max_cells - report.cells,
        )?;
        report.cells += report.interior.cells;
    } else {
        report.interior.reason = Some("corner-C2-interior-basis-unproven");
    }
    report.admitted = report.complete && report.within_tolerance && report.interior.certified;
    Ok(report)
}
