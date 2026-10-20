//! Tolerance-qualified contact locality at an actual shared rectangle corner.
//! Exact vertex contact is exposed only after exact lifts and zero hull intrusion.
//! Global body embedding remains separate.
use crate::{Model, face_domain::FaceDomain};
use nurbs_core::{
    Error, Result, curve::Curve, interval_eval::Interval as I, surface::Surface,
    surface_coordinate_flow, surface_g2_pcurves,
};
#[derive(Clone,Debug)]
pub struct Certificate {
    pub faces: [usize; 2],
    pub vertex: usize,
    pub trimmed_face: usize,
    pub coordinates: [usize; 2],
    pub distance_upper: f64,
    pub hull_penetrations: [f64; 2],
    /// Both faces contain the owned vertex and every possible pair contact
    /// equals it. This never follows from a small metric error bound alone.
    pub exact_vertex_contact: bool,
}
pub struct Attempt {
    pub certificate: Option<Certificate>,
    pub cells: usize,
    pub exact_work: u64,
}
fn corners(model: &Model, face: usize) -> Vec<(usize, [f64; 2])> {
    model.loops[model.faces[face].outer]
        .coedges
        .iter()
        .filter_map(|c| {
            let p = &c.pcurve;
            let d = p.domain();
            let n = p.control_points.len();
            if p.periodic
                || p.knots[..=p.degree].iter().any(|&k| k != d[0])
                || p.knots[n..].iter().any(|&k| k != d[1])
            {
                return None;
            }
            Some((
                model.edges[c.edge].vertices[usize::from(c.reversed)],
                [p.control_points[0][0], p.control_points[0][1]],
            ))
        })
        .collect()
}
// Exact literal separability, not a near-zero sampled cross derivative.
fn independent(s: &Surface, axis: usize, coordinate: usize) -> bool {
    let nu = s.control_points.len();
    let nv = s.control_points[0].len();
    (0..nu).all(|u| {
        (0..nv).all(|v| {
            let (a, b) = if axis == 0 { (u, 0) } else { (0, v) };
            s.control_points[u][v][coordinate] == s.control_points[a][b][coordinate]
                && s.weights[u][v] == s.weights[a][b]
        })
    })
}
pub fn inspect(
    model: &Model,
    faces: [usize; 2],
    max_cells: usize,
    max_exact: u64,
) -> Result<Attempt> {
    model.validate_boundary_diagnostic_inputs()?;
    if faces[0] == faces[1]
        || faces.iter().any(|&f| f >= model.faces.len())
        || max_cells > 100000
        || max_exact > 1000000
    {
        return Err(Error::new(
            "BREP_VERTEX_CONTACT_LIMITS",
            "Invalid pair or shared vertex-contact work",
        ));
    }
    if faces.iter().any(|&f| {
        let s = &model.faces[f].surface;
        s.control_points
            .len()
            .checked_mul(s.control_points[0].len())
            .is_none_or(|n| n > 4096)
    }) {
        return Err(Error::new(
            "BREP_VERTEX_CONTACT_LIMITS",
            "Original surface control budget exceeded",
        ));
    }
    let mut out = Attempt {
        certificate: None,
        cells: 0,
        exact_work: 0,
    };
    for [a, b] in [faces, [faces[1], faces[0]]] {
        if out.cells == max_cells {
            break;
        }
        let Some(rect) = FaceDomain::new(model, a, 1e-8)?.literal_rectangle() else {
            continue;
        };
        let ca = corners(model, a);
        let cb = corners(model, b);
        let shared = ca
            .iter()
            .filter(|(v, _)| cb.iter().any(|(w, _)| v == w))
            .collect::<Vec<_>>();
        if shared.len() != 1 {
            continue;
        }
        let (vertex, uv) = *shared[0];
        let other = cb.iter().filter(|(v, _)| *v == vertex).collect::<Vec<_>>();
        if other.len() != 1 {
            continue;
        }
        let other_uv = other[0].1;
        let mut ends = [0usize; 2];
        let mut corner = true;
        for k in 0..2 {
            if uv[k] == rect[k][0] {
                ends[k] = 0;
            } else if uv[k] == rect[k][1] {
                ends[k] = 1;
            } else {
                corner = false;
            }
        }
        if !corner {
            continue;
        }
        let s = &model.faces[a].surface;
        let t = &model.faces[b].surface;
        let natural = [
            [t.knots_u[t.degree_u], t.knots_u[t.control_points.len()]],
            [t.knots_v[t.degree_v], t.knots_v[t.control_points[0].len()]],
        ];
        if std::iter::once(model.faces[b].outer)
            .chain(model.faces[b].holes.iter().copied())
            .flat_map(|w| &model.loops[w].coedges)
            .any(|c| {
                c.pcurve
                    .control_points
                    .iter()
                    .any(|p| (0..2).any(|k| p[k] < natural[k][0] || p[k] > natural[k][1]))
            })
        {
            continue;
        }
        for x in 0..3 {
            for y in 0..3 {
                if x == y || !independent(s, 0, x) || !independent(s, 1, y) {
                    continue;
                }
                let coordinates = [x, y];
                let mut factors = [0.; 2];
                let mut penetration = [0.; 2];
                let mut usable = true;
                for axis in 0..2 {
                    if out.cells == max_cells {
                        return Ok(out);
                    }
                    out.cells += 1;
                    let flow = surface_coordinate_flow::inspect(
                        s,
                        axis,
                        coordinates[axis],
                        max_cells - out.cells,
                    )?;
                    out.cells += flow.cells;
                    let (Some(direction), Some(factor)) =
                        (flow.direction, flow.inverse_distance_factor)
                    else {
                        usable = false;
                        break;
                    };
                    if !flow.complete {
                        usable = false;
                        break;
                    }
                    factors[axis] = factor;
                    let k = coordinates[axis];
                    let value = model.vertices[vertex].point[k];
                    let low = t
                        .control_points
                        .iter()
                        .flatten()
                        .map(|p| p[k])
                        .fold(f64::INFINITY, f64::min);
                    let high = t
                        .control_points
                        .iter()
                        .flatten()
                        .map(|p| p[k])
                        .fold(f64::NEG_INFINITY, f64::max);
                    let positive = (direction > 0) == (ends[axis] == 1);
                    penetration[axis] = if positive && low < value {
                        I::point(value).sub(I::point(low))?.hi
                    } else if !positive && high > value {
                        I::point(high).sub(I::point(value))?.hi
                    } else {
                        0.
                    };
                    if I::point(factor).mul(I::point(penetration[axis]))?.lo > model.tolerance_mm {
                        usable = false;
                        break;
                    }
                }
                if !usable {
                    continue;
                }
                if out.cells == max_cells {
                    return Ok(out);
                }
                let c = Curve::from_polyline(vec![model.vertices[vertex].point.to_vec(); 2])?;
                let pc = Curve::from_polyline(vec![uv.to_vec(); 2])?;
                let other_pc = Curve::from_polyline(vec![other_uv.to_vec(); 2])?;
                let position = surface_g2_pcurves::verify_positions(
                    &c,
                    [&pc, &other_pc],
                    [s, t],
                    [false, false],
                    model.tolerance_mm / 16.,
                    max_cells - out.cells,
                    max_exact - out.exact_work,
                )?;
                out.cells += position.cells;
                out.exact_work += position.exact_work;
                if !position.proven {
                    continue;
                }
                // Follow U to the corner's U, then V to the corner. Literal
                // separability keeps each projected displacement independent of
                // the other parameter. The original flow factors bound both legs.
                let delta = I::point(position.upper.unwrap());
                let mut upper = delta;
                for axis in 0..2 {
                    upper = upper.add(
                        delta
                            .add(I::point(penetration[axis]))?
                            .mul(I::point(factors[axis]))?,
                    )?;
                }
                if upper.hi > model.tolerance_mm {
                    continue;
                }
                let mut valid = true;
                for face in [a, b] {
                    if max_cells - out.cells < 2 {
                        valid = false;
                        break;
                    }
                    let f = &model.faces[face];
                    let loops = std::iter::once(f.outer)
                        .chain(f.holes.iter().copied())
                        .map(|w| {
                            model.loops[w]
                                .coedges
                                .iter()
                                .map(|c| c.pcurve.clone())
                                .collect()
                        })
                        .collect::<Vec<Vec<_>>>();
                    let stage = (max_cells - out.cells) / 2;
                    let r =
                        nurbs_core::trim_region_audit::inspect(&loops, 1e-8, 10000, stage, stage)?;
                    out.cells += r.cells + r.domain_cells;
                    if r.valid != Some(true) {
                        valid = false;
                        break;
                    }
                }
                if !valid {
                    continue;
                }
                // Zero coordinate gaps, strict independent coordinate flow
                // and exact corner lifts force both source parameters to the
                // actual corner. Exact lifts also put that vertex on BOTH
                // actual outer boundaries. No tolerance threshold is used.
                let exact_vertex_contact = penetration == [0., 0.]
                    && position.upper == Some(0.)
                    && position.sides.iter().all(|side| {
                        side.exact
                            .as_ref()
                            .is_some_and(|p| p.outcome == cad_predicates::BezierIdentity::Equal)
                    });
                out.certificate = Some(Certificate {
                    faces,
                    vertex,
                    trimmed_face: a,
                    coordinates,
                    distance_upper: if exact_vertex_contact { 0. } else { upper.hi },
                    hull_penetrations: penetration,
                    exact_vertex_contact,
                });
                return Ok(out);
            }
        }
    }
    Ok(out)
}
