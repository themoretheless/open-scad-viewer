//! Global nonintersection proof or exact interior fold witness; no sampled verdict.
use crate::{Result, surface::Surface, surface_injectivity};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Absent,
    Present,
    Unresolved,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub status: Status,
    pub injectivity: surface_injectivity::Report,
    /// Distinct interior normalized parameters, interpreted by the exact affine
    /// active-domain map. Exact reflected homogeneous controls prove coincidence.
    pub witness: Option<[[f64; 2]; 2]>,
    pub reflected_axis: Option<usize>,
}
fn bezier(k: &[f64], p: usize, n: usize) -> bool {
    n == p + 1 && k[..=p].iter().all(|x| *x == k[p]) && k[n..].iter().all(|x| *x == k[n])
}
/// Absence follows a single global contraction bound across every source span.
/// Presence uses exact homogeneous reflection of a complete Bezier axis: a
/// mathematical interior two-to-one fold, not a tolerance/sample coincidence.
/// All other cases (including unsupported periodic charts and insufficient
/// budgets) stay Unresolved. Isolated local regularity never proves absence.
pub fn inspect(s: &Surface, max_spans: usize) -> Result<Report> {
    let injectivity = surface_injectivity::certify(s, max_spans)?;
    let mut out = Report {
        status: if injectivity.proven {
            Status::Absent
        } else {
            Status::Unresolved
        },
        injectivity,
        witness: None,
        reflected_axis: None,
    };
    if out.status == Status::Absent {
        return Ok(out);
    }
    let (nu, nv) = (s.control_points.len(), s.control_points[0].len());
    if s.periodic_u || s.periodic_v {
        return Ok(out);
    }
    for axis in 0..2 {
        let eligible = if axis == 0 {
            bezier(&s.knots_u, s.degree_u, nu)
        } else {
            bezier(&s.knots_v, s.degree_v, nv)
        };
        if !eligible {
            continue;
        }
        let reflected = (0..nu).all(|i| {
            (0..nv).all(|j| {
                let (a, b) = if axis == 0 {
                    (nu - 1 - i, j)
                } else {
                    (i, nv - 1 - j)
                };
                s.control_points[i][j] == s.control_points[a][b]
                    && s.weights[i][j] == s.weights[a][b]
            })
        });
        if reflected {
            out.status = Status::Present;
            out.reflected_axis = Some(axis);
            out.witness = Some(if axis == 0 {
                [[0.25, 0.5], [0.75, 0.5]]
            } else {
                [[0.5, 0.25], [0.5, 0.75]]
            });
            return Ok(out);
        }
    }
    Ok(out)
}
