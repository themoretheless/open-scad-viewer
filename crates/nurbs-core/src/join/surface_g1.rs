//! Continuous tangent-plane agreement on affine-corresponding boundaries.
use crate::{
    Result, check,
    distance_bounds::{Interval as I, box_distance},
    surface::Surface,
    surface_join::{Boundary, Decision, boundary_box},
    surface_measure::jets,
};
#[derive(Clone, Debug)]
pub struct Report {
    pub position_bounds: [f64; 2],
    pub normal_chord_lower: f64,
    /// Absent unless both boundary normals are regular over the entire seam.
    pub normal_chord_upper: Option<f64>,
    pub regularity_proven: bool,
    pub decision: Decision,
    pub cells: usize,
}
fn edge(e: Boundary) -> (usize, bool) {
    match e {
        Boundary::UMin => (0, false),
        Boundary::UMax => (0, true),
        Boundary::VMin => (1, false),
        Boundary::VMax => (1, true),
    }
}
fn domains(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
pub(crate) fn boundary_jets(
    s: &Surface,
    e: Boundary,
    t: [f64; 2],
    reverse: bool,
) -> Result<Vec<jets::Jets>> {
    let (fixed, max) = edge(e);
    let free = 1 - fixed;
    let d = domains(s);
    let mut t = I::new(t[0], t[1])?;
    if reverse {
        t = I::point(1.).sub(t)?.intersect(0., 1.)?;
    }
    let x = I::point(d[free][0])
        .add(I::point(d[free][1]).sub(I::point(d[free][0]))?.mul(t)?)?
        .intersect(d[free][0], d[free][1])?;
    let (p, k, n) = if fixed == 0 {
        (s.degree_u, &s.knots_u, s.control_points.len())
    } else {
        (s.degree_v, &s.knots_v, s.control_points[0].len())
    };
    let fixed_span = (p..n).filter(|&i| k[i] < k[i + 1]);
    let fs = if max {
        fixed_span.last()
    } else {
        fixed_span.into_iter().next()
    }
    .unwrap();
    let (p, k, n) = if free == 0 {
        (s.degree_u, &s.knots_u, s.control_points.len())
    } else {
        (s.degree_v, &s.knots_v, s.control_points[0].len())
    };
    let fixed_knots = if fixed == 0 { &s.knots_u } else { &s.knots_v };
    let mut out = Vec::new();
    for i in p..n {
        let lo = x.lo.max(k[i]);
        let hi = x.hi.min(k[i + 1]);
        if k[i] >= k[i + 1] || lo > hi {
            continue;
        }
        let mut domain = d;
        let mut span = [0; 2];
        span[fixed] = fs;
        span[free] = i;
        domain[fixed] = [fixed_knots[fs], fixed_knots[fs + 1]];
        let mut corner = [None; 2];
        corner[fixed] = Some(usize::from(max));
        if lo == hi {
            if lo - k[i] >= k[i + 1] - lo {
                domain[free] = [k[i], lo];
                corner[free] = Some(1);
            } else {
                domain[free] = [lo, k[i + 1]];
                corner[free] = Some(0);
            }
        } else {
            domain[free] = [lo, hi];
        }
        let j = jets::calculate_partial_stable(s, span, domain, corner)?;
        out.push(j);
    }
    Ok(out)
}
fn normal_image(s: &Surface, e: Boundary, t: [f64; 2], reverse: bool) -> Result<Option<Vec<I>>> {
    let mut out: Option<Vec<I>> = None;
    for j in boundary_jets(s, e, t, reverse)? {
        let a = j[1][0];
        let b = j[0][1];
        let mut normal = vec![I::point(0.); 3];
        for q in 0..3 {
            let r = (q + 1) % 3;
            let z = (q + 2) % 3;
            normal[q] = a[r].mul(b[z])?.sub(a[z].mul(b[r])?)?;
        }
        let (low, high) = box_distance(&normal, &[I::point(0.); 3])?;
        if low <= 0. {
            return Ok(None);
        }
        let length = I::new(low, high)?;
        for x in &mut normal {
            *x = x.div(length)?.intersect(-1., 1.)?;
        }
        if let Some(out) = &mut out {
            for (x, y) in out.iter_mut().zip(normal) {
                x.lo = x.lo.min(y.lo);
                x.hi = x.hi.max(y.hi);
            }
        } else {
            out = Some(normal);
        }
    }
    Ok(out)
}
fn plane_distance(a: &[I], b: &[I]) -> Result<(f64, f64)> {
    let opposite: Vec<_> = b
        .iter()
        .map(|x| I::new(-x.hi, -x.lo))
        .collect::<Result<_>>()?;
    let same = box_distance(a, b)?;
    let opposite = box_distance(a, &opposite)?;
    Ok((same.0.min(opposite.0), same.1.min(opposite.1).min(2.)))
}
struct Cell {
    t: [f64; 2],
    position: [f64; 2],
    normal_lower: f64,
    normal_upper: Option<f64>,
}
fn cell(
    a: &Surface,
    ea: Boundary,
    b: &Surface,
    eb: Boundary,
    reverse: bool,
    t: [f64; 2],
) -> Result<Cell> {
    let upper = box_distance(
        &boundary_box(a, ea, t, false)?,
        &boundary_box(b, eb, t, reverse)?,
    )?
    .1;
    let normal_upper = match (
        normal_image(a, ea, t, false)?,
        normal_image(b, eb, t, reverse)?,
    ) {
        (Some(x), Some(y)) => Some(plane_distance(&x, &y)?.1),
        _ => None,
    };
    let mut lower = 0_f64;
    let mut normal_lower = 0_f64;
    for p in [t[0], t[0] * 0.5 + t[1] * 0.5, t[1]] {
        lower = lower.max(
            box_distance(
                &boundary_box(a, ea, [p, p], false)?,
                &boundary_box(b, eb, [p, p], reverse)?,
            )?
            .0,
        );
        if let (Some(x), Some(y)) = (
            normal_image(a, ea, [p, p], false)?,
            normal_image(b, eb, [p, p], reverse)?,
        ) {
            normal_lower = normal_lower.max(plane_distance(&x, &y)?.0);
        }
    }
    Ok(Cell {
        t,
        position: [lower, upper],
        normal_lower,
        normal_upper,
    })
}
/// Sufficient G1: G0 plus regular tangent planes along the full seam. Normal
/// chord distance is min(|n_a-n_b|,|n_a+n_b|), so orientation is ignored.
/// This does not certify manifold adjacency, equal parameter derivatives, or
/// continuity across a periodic storage seam. C1 free-axis basis is required.
pub fn inspect(
    a: &Surface,
    ea: Boundary,
    b: &Surface,
    eb: Boundary,
    reverse: bool,
    position_tolerance: f64,
    normal_chord_tolerance: f64,
    max_cells: usize,
) -> Result<Report> {
    a.validate()?;
    b.validate()?;
    check(
        position_tolerance.is_finite()
            && position_tolerance >= 0.
            && normal_chord_tolerance.is_finite()
            && (0. ..=2.).contains(&normal_chord_tolerance),
        "G1 needs finite nonnegative position and 0..2 normal-chord tolerances",
    )?;
    check(
        (1..=100000).contains(&max_cells),
        "G1 needs 1..100000 cells",
    )?;
    for (s, e) in [(a, ea), (b, eb)] {
        let free = 1 - edge(e).0;
        let (p, k, n) = if free == 0 {
            (s.degree_u, &s.knots_u, s.control_points.len())
        } else {
            (s.degree_v, &s.knots_v, s.control_points[0].len())
        };
        for &t in k {
            if k[p] < t && t < k[n] {
                check(
                    k.iter().filter(|&&x| x == t).count() < p,
                    "G1 requires a C1 free-axis basis",
                )?;
            }
        }
    }
    let mut cells = vec![cell(a, ea, b, eb, reverse, [0., 1.])?];
    let mut work = 1;
    let mut lower = cells[0].position[0];
    let mut normal_lower = cells[0].normal_lower;
    loop {
        let upper = cells.iter().map(|c| c.position[1]).fold(lower, f64::max);
        let normal_upper = cells
            .iter()
            .try_fold(0_f64, |x, c| c.normal_upper.map(|y| x.max(y)));
        let decision = if lower > position_tolerance || normal_lower > normal_chord_tolerance {
            Some(Decision::CorrespondenceExceedsTolerance)
        } else if upper <= position_tolerance
            && normal_upper.is_some_and(|x| x <= normal_chord_tolerance)
        {
            Some(Decision::WithinTolerance)
        } else if work + 2 > max_cells {
            Some(Decision::Unresolved)
        } else {
            None
        };
        let report = |decision| Report {
            position_bounds: [lower, upper],
            normal_chord_lower: normal_lower,
            normal_chord_upper: normal_upper,
            regularity_proven: normal_upper.is_some(),
            decision,
            cells: work,
        };
        if let Some(decision) = decision {
            return Ok(report(decision));
        }
        let score = |c: &Cell| {
            c.normal_upper.map_or(f64::INFINITY, |n| {
                (n / normal_chord_tolerance.max(1e-12))
                    .max(c.position[1] / position_tolerance.max(1e-12))
            })
        };
        let index = cells
            .iter()
            .enumerate()
            .max_by(|a, b| {
                score(a.1)
                    .total_cmp(&score(b.1))
                    .then_with(|| (a.1.t[1] - a.1.t[0]).total_cmp(&(b.1.t[1] - b.1.t[0])))
            })
            .unwrap()
            .0;
        let t = cells[index].t;
        let mid = t[0] * 0.5 + t[1] * 0.5;
        if !(t[0] < mid && mid < t[1]) {
            return Ok(report(Decision::Unresolved));
        }
        cells.swap_remove(index);
        for t in [[t[0], mid], [mid, t[1]]] {
            let c = cell(a, ea, b, eb, reverse, t)?;
            lower = lower.max(c.position[0]);
            normal_lower = normal_lower.max(c.normal_lower);
            cells.push(c);
            work += 1;
        }
    }
}
