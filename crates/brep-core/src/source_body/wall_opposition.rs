//! Conservative original-surface normal opposition inventory in one shell.
//! Unknowns remain candidates. The angular rule is not material-wall pairing.
//! First-jet bounds include all one-sided original knot limits; they do not
//! certify storage continuity or normal cones at sharp edges.
use crate::{Error, Model, Result, face_domain::FaceDomain};
use nurbs_core::{interval_eval::Interval as I, surface_g1_pcurves, surface_shape_operator};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Opposing,
    NonOpposing,
    Unresolved,
}
pub struct Face {
    pub face: usize,
    pub decision: Decision,
    pub dot_bounds: Option<[f64; 2]>,
    pub cells: usize,
}
pub struct Report {
    pub shell: usize,
    pub source_face: usize,
    pub cutoff: f64,
    pub faces: Vec<Face>,
    pub opposing_faces: Vec<usize>,
    pub candidate_faces: Vec<usize>,
    pub excluded_faces: Vec<usize>,
    pub selection_complete: bool,
    pub cells: usize,
}
fn domain(model: &Model, face: usize, uv_tol: f64) -> Result<[[f64; 2]; 2]> {
    let f = &model.faces[face];
    Ok(FaceDomain::new(model, face, uv_tol)?
        .literal_rectangle()
        .unwrap_or([
            [
                f.surface.knots_u[f.surface.degree_u],
                f.surface.knots_u[f.surface.control_points.len()],
            ],
            [
                f.surface.knots_v[f.surface.degree_v],
                f.surface.knots_v[f.surface.control_points[0].len()],
            ],
        ]))
}
fn pair(
    model: &Model,
    faces: [usize; 2],
    reverse: [bool; 2],
    domains: [[[f64; 2]; 2]; 2],
    cutoff: f64,
    max: usize,
) -> Result<Face> {
    let mut out = Face {
        face: faces[1],
        decision: Decision::Unresolved,
        dot_bounds: None,
        cells: 0,
    };
    if faces.iter().any(|&f| {
        let s = &model.faces[f].surface;
        s.periodic_u || s.periodic_v
    }) {
        return Ok(out);
    }
    let mut pending = vec![(domains, 0usize)];
    let mut opposing = false;
    let mut non = false;
    let mut bound = [f64::INFINITY, f64::NEG_INFINITY];
    while let Some((query, depth)) = pending.pop() {
        if out.cells == max {
            return Ok(out);
        }
        let mut normals = Vec::new();
        for side in 0..2 {
            let original = surface_shape_operator::rectangle_bounds(
                &model.faces[faces[side]].surface,
                query[side],
                max - out.cells,
            )?;
            out.cells += original.cells;
            let normal = if out.cells < max {
                let first = surface_g1_pcurves::rectangle_normal_bounds(
                    &model.faces[faces[side]].surface,
                    query[side],
                    max - out.cells,
                )?;
                out.cells += first.cells;
                match (original.shape.normal, first.normal) {
                    (Some(a), Some(b)) => {
                        let mut n = a;
                        for k in 0..3 {
                            let bound = I::new(a[k][0], a[k][1])?.intersect(b[k][0], b[k][1])?;
                            n[k] = [bound.lo, bound.hi];
                        }
                        Some(n)
                    }
                    (Some(n), None) | (None, Some(n)) => Some(n),
                    (None, None) => None,
                }
            } else {
                original.shape.normal
            };
            let Some(n) = normal else {
                return Ok(out);
            };
            normals.push(n);
        }
        let mut dot = I::point(0.);
        for k in 0..3 {
            dot = dot.add(
                I::new(normals[0][k][0], normals[0][k][1])?
                    .mul(I::new(normals[1][k][0], normals[1][k][1])?)?,
            )?;
        }
        if reverse[0] != reverse[1] {
            dot = dot.mul(I::point(-1.))?;
        }
        let lo = dot.lo.max(-1.);
        let hi = dot.hi.min(1.);
        if lo > hi {
            return Ok(out);
        }
        if hi <= cutoff {
            opposing = true;
            bound[0] = bound[0].min(lo);
            bound[1] = bound[1].max(hi);
        } else if lo > cutoff {
            non = true;
            bound[0] = bound[0].min(lo);
            bound[1] = bound[1].max(hi);
        } else {
            if depth == 12 {
                return Ok(out);
            }
            let mut choice = None;
            let mut largest = 0.;
            for side in 0..2 {
                for axis in 0..2 {
                    let size = (query[side][axis][1] - query[side][axis][0])
                        / (domains[side][axis][1] - domains[side][axis][0]);
                    if size > largest {
                        largest = size;
                        choice = Some((side, axis));
                    }
                }
            }
            let Some((side, axis)) = choice else {
                return Ok(out);
            };
            let [a, b] = query[side][axis];
            let mid = a * 0.5 + b * 0.5;
            if !(a < mid && mid < b) {
                return Ok(out);
            }
            let mut left = query;
            let mut right = query;
            left[side][axis][1] = mid;
            right[side][axis][0] = mid;
            pending.push((right, depth + 1));
            pending.push((left, depth + 1));
        }
        if opposing && non {
            return Ok(out);
        }
    }
    out.decision = if opposing {
        Decision::Opposing
    } else {
        Decision::NonOpposing
    };
    out.dot_bounds = Some(bound);
    Ok(out)
}
/// Every classified result covers the complete source/target chart cover.
/// Nonrectangular trims use a conservative natural-domain cover. Shell face
/// orientation is applied before the dot-product proof. No sample excludes a
/// target. This shell-scoped angular filter is not a whole-solid thickness proof.
pub fn inspect(
    model: &Model,
    shell: usize,
    source_face: usize,
    cutoff: f64,
    uv_tol: f64,
    max_cells: usize,
) -> Result<Report> {
    model.validate()?;
    if shell >= model.shells.len()
        || !model.shells[shell].closed
        || source_face >= model.faces.len()
        || !cutoff.is_finite()
        || !(-1. ..0.).contains(&cutoff)
        || !uv_tol.is_finite()
        || uv_tol <= 0.
        || max_cells > 100000
    {
        return Err(Error::new(
            "BREP_WALL_OPPOSITION_INPUT",
            "Choose a closed shell, finite negative angular cutoff and bounded work",
        ));
    }
    let uses = &model.shells[shell].faces;
    if uses.len() > 1024 {
        return Err(Error::new(
            "BREP_WALL_OPPOSITION_INPUT",
            "Shell face budget exceeded",
        ));
    }
    if uses.iter().any(|u| {
        let s = &model.faces[u.face].surface;
        s.control_points
            .len()
            .checked_mul(s.control_points[0].len())
            .is_none_or(|n| n > 4096)
    }) {
        return Err(Error::new(
            "BREP_WALL_OPPOSITION_INPUT",
            "Original surface control budget exceeded",
        ));
    }
    let source = uses.iter().find(|u| u.face == source_face).ok_or_else(|| {
        Error::new(
            "BREP_WALL_OPPOSITION_INPUT",
            "Source face must belong to the selected shell",
        )
    })?;
    let source_domain = domain(model, source_face, uv_tol)?;
    let targets = uses
        .iter()
        .filter(|u| u.face != source_face)
        .collect::<Vec<_>>();
    let mut report = Report {
        shell,
        source_face,
        cutoff,
        faces: Vec::new(),
        opposing_faces: Vec::new(),
        candidate_faces: Vec::new(),
        excluded_faces: Vec::new(),
        selection_complete: true,
        cells: 0,
    };
    for (index, target) in targets.iter().enumerate() {
        let remaining = max_cells - report.cells;
        let allowance = if remaining == 0 {
            0
        } else {
            (remaining / (targets.len() - index)).max(1)
        };
        let r = pair(
            model,
            [source_face, target.face],
            [source.reversed, target.reversed],
            [source_domain, domain(model, target.face, uv_tol)?],
            cutoff,
            allowance,
        )?;
        report.cells += r.cells;
        match r.decision {
            Decision::Opposing => {
                report.opposing_faces.push(r.face);
                report.candidate_faces.push(r.face);
            }
            Decision::NonOpposing => report.excluded_faces.push(r.face),
            Decision::Unresolved => {
                report.candidate_faces.push(r.face);
                report.selection_complete = false;
            }
        }
        report.faces.push(r);
    }
    Ok(report)
}
