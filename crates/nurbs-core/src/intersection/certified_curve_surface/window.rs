//! Natural-parameter windows on ORIGINAL curve/surface coefficients.
use super::*;
#[derive(Debug)]
pub struct WindowRoot {
    pub spans: [usize; 3],
    /// Exact binary64 search domain on which strict Krawczyk admission occurred.
    pub existence_domain: Box3,
    pub parameter_box: Box3,
    pub position_bounds: [[f64; 2]; 3],
    pub crossing_sign: i8,
}
#[derive(Debug)]
pub struct WindowReport {
    pub roots: Vec<WindowRoot>,
    pub unresolved: Vec<Box3>,
    pub cells: usize,
    pub complete: bool,
}
struct Query {
    ids: [usize; 3],
    domain: Box3,
    depth: usize,
}
/// Returns the first certified root; queued windows remain unresolved. Complete
/// means only this caller-specified window, never surface/solid completeness.
pub fn first_in_window(
    c: &Curve,
    s: &Surface,
    window: Box3,
    max_cells: usize,
) -> Result<WindowReport> {
    c.validate()?;
    search(Source::Curve(c), s, window, max_cells)
}
use super::window_source::Source;
pub(super) fn search(
    c: Source<'_>,
    s: &Surface,
    window: Box3,
    max_cells: usize,
) -> Result<WindowReport> {
    s.validate()?;
    check(
        c.valid() && !s.periodic_u && !s.periodic_v && (1..=100000).contains(&max_cells),
        "Window roots need bounded nonperiodic 3D sources",
    )?;
    let domains = [
        c.domain(),
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    check(
        (0..3).all(|i| {
            window[i][0].is_finite()
                && window[i][1].is_finite()
                && window[i][0] < window[i][1]
                && window[i][0] >= domains[i][0]
                && window[i][1] <= domains[i][1]
        }),
        "Invalid original root window",
    )?;
    check(
        (c.degree() + 1)
            .saturating_mul(s.degree_u + 1)
            .saturating_mul(s.degree_v + 1)
            <= 4096,
        "Window root degree product exceeds 4096",
    )?;
    let spans = |k: &[f64], p: usize, n: usize| {
        (p..n)
            .filter(|&i| k[i] < k[i + 1])
            .map(|i| (i, [k[i], k[i + 1]]))
            .collect::<Vec<_>>()
    };
    let cs = spans(c.knots(), c.degree(), c.count());
    let us = spans(&s.knots_u, s.degree_u, s.control_points.len());
    let vs = spans(&s.knots_v, s.degree_v, s.control_points[0].len());
    let mut stack = Vec::new();
    for &(ct, t) in &cs {
        for &(su, u) in &us {
            for &(sv, v) in &vs {
                let full = [t, u, v];
                let d: Box3 = std::array::from_fn(|i| {
                    [full[i][0].max(window[i][0]), full[i][1].min(window[i][1])]
                });
                if d.iter().any(|x| x[0] >= x[1]) {
                    continue;
                }
                if stack.len() == 1024 {
                    return Err(resource("Window root span product exceeds 1024"));
                }
                stack.push(Query {
                    ids: [ct, su, sv],
                    domain: d,
                    depth: 0,
                });
            }
        }
    }
    let mut out = WindowReport {
        roots: vec![],
        unresolved: vec![],
        cells: 0,
        complete: false,
    };
    while let Some(cell) = stack.pop() {
        if out.cells == max_cells {
            out.unresolved.push(cell.domain);
            out.unresolved.extend(stack.into_iter().map(|q| q.domain));
            return Ok(out);
        }
        out.cells += 1;
        match krawczyk(&c.field(s, cell.ids, cell.domain)?)? {
            Verdict::Excluded => {}
            Verdict::Unique(root, sign) => {
                let mut domain = map_box(cell.domain, [[0., 1.]; 3], root)?;
                for _ in 0..8 {
                    if out.cells == max_cells {
                        break;
                    }
                    out.cells += 1;
                    let search = refinement_box(domain, cell.domain);
                    match krawczyk(&c.field(s, cell.ids, search)?)? {
                        Verdict::Unique(next, next_sign) => {
                            if next_sign != sign {
                                return Err(crate::Error::new(
                                    "NURBS_ROOT_PROOF_CONTRADICTION",
                                    "Window root orientation changed during refinement",
                                ));
                            }
                            let next = map_box(search, [[0., 1.]; 3], next)?;
                            if next == domain {
                                break;
                            }
                            domain = next;
                        }
                        Verdict::Unresolved => break,
                        Verdict::Excluded => {
                            return Err(crate::Error::new(
                                "NURBS_ROOT_PROOF_CONTRADICTION",
                                "Certified window root excluded during refinement",
                            ));
                        }
                    }
                }
                let p = c.position(cell.ids[0], domain[0])?;
                out.roots.push(WindowRoot {
                    spans: cell.ids,
                    existence_domain: cell.domain,
                    parameter_box: domain,
                    position_bounds: std::array::from_fn(|i| [p[i].lo, p[i].hi]),
                    crossing_sign: sign,
                });
                out.unresolved.extend(stack.into_iter().map(|q| q.domain));
                out.complete = out.unresolved.is_empty();
                return Ok(out);
            }
            Verdict::Unresolved => {
                if cell.depth == 48 {
                    out.unresolved.push(cell.domain);
                    continue;
                }
                let axis = cell.depth % 3;
                let [a, b] = cell.domain[axis];
                let mid = a + (b - a) * 0.4375;
                if mid <= a || mid >= b {
                    out.unresolved.push(cell.domain);
                    continue;
                }
                let mut left = cell.domain;
                let mut right = cell.domain;
                left[axis][1] = mid;
                right[axis][0] = mid;
                stack.push(Query {
                    ids: cell.ids,
                    domain: right,
                    depth: cell.depth + 1,
                });
                stack.push(Query {
                    ids: cell.ids,
                    domain: left,
                    depth: cell.depth + 1,
                });
            }
        }
    }
    out.complete = out.unresolved.is_empty();
    Ok(out)
}
