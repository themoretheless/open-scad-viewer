//! Exact source-chart gluing followed by whole-chart injectivity.
//! Admission additionally requires the joint exact-boundary/trim prerequisites.
use crate::{Model, Result};
use nurbs_core::{surface::Surface, surface_linear_monotonicity};

#[derive(Clone, Debug)]
pub struct Certificate {
    pub faces: [usize; 2],
    pub edge: usize,
    pub contact_enclosure: [[f64; 2]; 3],
    pub projection: [[i8; 3]; 2],
    pub row_weights: [f64; 2],
    pub profile_parameterization: Option<surface_linear_monotonicity::ProfileParameterization>,
    pub cells: usize,
}
pub struct Outcome {
    pub certificate: Option<Certificate>,
    pub cells: usize,
}
fn natural_edges(model: &Model, face: usize) -> Option<Vec<(usize, usize, bool)>> {
    let face = &model.faces[face];
    if !face.holes.is_empty() {
        return None;
    }
    let wire = &model.loops[face.outer];
    if wire.coedges.len() != 4 {
        return None;
    }
    let mut sides = Vec::new();
    for c in &wire.coedges {
        let p = &c.pcurve;
        if p.degree != 1 || p.control_points.len() != 2 || p.knots != [0., 0., 1., 1.] {
            return None;
        }
        let a = &p.control_points[0];
        let b = &p.control_points[1];
        let axis = (0..2).find(|&k| {
            a[k] == b[k]
                && (a[k] == 0. || a[k] == 1.)
                && ((a[1 - k] == 0. && b[1 - k] == 1.) || (a[1 - k] == 1. && b[1 - k] == 0.))
        })?;
        let max = a[axis] == 1.;
        if sides.iter().any(|&(_, k, m)| k == axis && m == max) {
            return None;
        }
        sides.push((c.edge, axis, max));
    }
    Some(sides)
}
fn clamped_unit(s: &Surface) -> bool {
    let p = [s.degree_u, s.degree_v];
    let n = [s.control_points.len(), s.control_points[0].len()];
    let knots = [&s.knots_u, &s.knots_v];
    !s.periodic_u
        && !s.periodic_v
        && (0..2).all(|k| {
            p[k] > 0
                && p[k] <= 8
                && n[k] == p[k] + 1
                && knots[k].len() == 2 * (p[k] + 1)
                && knots[k][..=p[k]].iter().all(|v| *v == 0.)
                && knots[k][p[k] + 1..].iter().all(|v| *v == 1.)
        })
        && n[0] * n[1] <= 64
}
fn reflect(s: &mut Surface, axis: usize) {
    if axis == 0 {
        s.control_points.reverse();
        s.weights.reverse();
    } else {
        for row in &mut s.control_points {
            row.reverse();
        }
        for row in &mut s.weights {
            row.reverse();
        }
    }
}
fn strip(s: &Surface, axis: usize, max: bool) -> Vec<(Vec<f64>, f64)> {
    if axis == 0 {
        let row = if max { s.control_points.len() - 1 } else { 0 };
        s.control_points[row]
            .iter()
            .cloned()
            .zip(s.weights[row].iter().copied())
            .collect()
    } else {
        let index = if max {
            s.control_points[0].len() - 1
        } else {
            0
        };
        s.control_points
            .iter()
            .zip(&s.weights)
            .map(|(row, w)| (row[index].clone(), w[index]))
            .collect()
    }
}
fn exact_product(a: f64, b: f64) -> Option<f64> {
    let p = a * b;
    // This range keeps every possible binary64 multiplication residual normal.
    if !p.is_finite() || !(1e-100..=1e100).contains(&p) || a.mul_add(b, -p) != 0. {
        None
    } else {
        Some(p)
    }
}
fn stitch(a: &Surface, b: &Surface, axis: usize, amax: bool, bmax: bool) -> Option<Surface> {
    if !clamped_unit(a) || !clamped_unit(b) || a.degree_u != b.degree_u || a.degree_v != b.degree_v
    {
        return None;
    }
    let mut a = a.clone();
    let mut b = b.clone();
    if !amax {
        reflect(&mut a, axis);
    }
    if bmax {
        reflect(&mut b, axis);
    }
    let left = strip(&a, axis, true);
    let mut right = strip(&b, axis, false);
    if left.iter().zip(&right).any(|(a, b)| a.0 != b.0) {
        reflect(&mut b, 1 - axis);
        right = strip(&b, axis, false);
    }
    if left.len() != right.len() || left.iter().zip(&right).any(|(a, b)| a.0 != b.0) {
        return None;
    }
    let scale = left[0].1 / right[0].1;
    if !scale.is_finite()
        || scale <= 0.
        || left
            .iter()
            .zip(&right)
            .any(|(a, b)| exact_product(scale, b.1) != Some(a.1))
    {
        return None;
    }
    for w in b.weights.iter_mut().flatten() {
        *w = exact_product(scale, *w)?;
    }
    let degree = if axis == 0 { a.degree_u } else { a.degree_v };
    let knots = [vec![0.; degree + 1], vec![1.; degree], vec![2.; degree + 1]].concat();
    if axis == 0 {
        a.control_points
            .extend(b.control_points.into_iter().skip(1));
        a.weights.extend(b.weights.into_iter().skip(1));
        a.knots_u = knots;
    } else {
        for (left, right) in a.control_points.iter_mut().zip(b.control_points) {
            left.extend(right.into_iter().skip(1));
        }
        for (left, right) in a.weights.iter_mut().zip(b.weights) {
            left.extend(right.into_iter().skip(1));
        }
        a.knots_v = knots;
    }
    a.validate().ok()?;
    Some(a)
}
pub(crate) fn certify(model: &Model, faces: [usize; 2], max_cells: usize) -> Result<Outcome> {
    let mut out = Outcome {
        certificate: None,
        cells: 0,
    };
    if max_cells == 0 {
        return Ok(out);
    }
    let Some(a) = natural_edges(model, faces[0]) else {
        return Ok(out);
    };
    let Some(b) = natural_edges(model, faces[1]) else {
        return Ok(out);
    };
    for &(edge, axis, amax) in &a {
        let Some(&(_, other, bmax)) = b.iter().find(|x| x.0 == edge) else {
            continue;
        };
        if axis != other {
            continue;
        }
        let Some(merged) = stitch(
            &model.faces[faces[0]].surface,
            &model.faces[faces[1]].surface,
            axis,
            amax,
            bmax,
        ) else {
            continue;
        };
        let r = surface_linear_monotonicity::inspect_candidate(&merged, max_cells - out.cells)?;
        out.cells += r.cells;
        if r.certified {
            let curve = &model.edges[edge].curve;
            let enclosure = std::array::from_fn(|k| {
                [
                    curve
                        .control_points
                        .iter()
                        .map(|p| p[k])
                        .fold(f64::INFINITY, f64::min),
                    curve
                        .control_points
                        .iter()
                        .map(|p| p[k])
                        .fold(f64::NEG_INFINITY, f64::max),
                ]
            });
            out.certificate = Some(Certificate {
                faces,
                edge,
                contact_enclosure: enclosure,
                projection: r.projection,
                row_weights: r.row_weights,
                profile_parameterization: r.profile_parameterization,
                cells: out.cells,
            });
            break;
        }
        if out.cells == max_cells {
            break;
        }
    }
    Ok(out)
}
