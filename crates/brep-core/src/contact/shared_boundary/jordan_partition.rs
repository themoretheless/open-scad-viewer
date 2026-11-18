//! Two natural charts with a common projection partition a Jordan region along one
//! original, exactly lifted edge. Only the joint exact-domain audit may call
//! this, after all coedge lifts and domains have been independently checked.
use crate::{Model, boundary_hull_contact::Certificate};
use cad_predicates::Sign;
use num_rational::BigRational;
use num_traits::ToPrimitive;
use nurbs_core::{
    curve::Curve,
    surface_injectivity::{certify_chart, certify_jordan_projection},
};

#[derive(Clone, Copy, PartialEq)]
enum Projection {
    Coordinate([usize; 2]),
    Linear([[f64; 3]; 2]),
}

impl Projection {
    fn rows(self) -> [[i8; 3]; 2] {
        match self {
            Self::Coordinate(axes) => axes.map(|axis| {
                let mut row = [0; 3];
                row[axis] = 1;
                row
            }),
            // The chart proof proposes integer rows in -64..=64.
            Self::Linear(rows) => rows.map(|row| row.map(|value| value as i8)),
        }
    }

    fn point(self, point: &[f64]) -> Option<[f64; 2]> {
        match self {
            Self::Coordinate(axes) => Some(axes.map(|axis| point[axis])),
            Self::Linear(rows) => {
                let dot = |row: [f64; 3]| {
                    let mut exact = BigRational::from_integer(0.into());
                    for (coefficient, &value) in row.into_iter().zip(point) {
                        exact +=
                            BigRational::from_float(coefficient)? * BigRational::from_float(value)?;
                    }
                    let stored = exact.to_f64()?;
                    (BigRational::from_float(stored).as_ref() == Some(&exact)).then_some(stored)
                };
                Some([dot(rows[0])?, dot(rows[1])?])
            }
        }
    }
}

pub(crate) fn certify(
    model: &Model,
    faces: [usize; 2],
    limit: usize,
) -> (Option<Certificate>, usize) {
    let mut cells = 0;
    let result = prove(model, faces, limit, &mut cells);
    (result, cells)
}

fn charge(cells: &mut usize, limit: usize, cost: usize) -> Option<()> {
    if cost > limit - *cells {
        return None;
    }
    *cells += cost;
    Some(())
}

fn prove(model: &Model, faces: [usize; 2], limit: usize, cells: &mut usize) -> Option<Certificate> {
    let patches = faces.map(|i| &model.faces[i]);
    let surfaces = patches.map(|f| &f.surface);
    if !surfaces.iter().any(|s| {
        s.control_points.len() > s.degree_u + 1 || s.control_points[0].len() > s.degree_v + 1
    }) || patches.iter().any(|f| !f.holes.is_empty())
    {
        return None;
    }
    let wires = patches.map(|f| model.loops[f.outer].coedges.as_slice());
    if wires
        .iter()
        .any(|w| w.len() != 4 || w.iter().any(|c| c.trim().is_some()))
    {
        return None;
    }
    charge(cells, limit, 4)?;
    let shared = wires[0]
        .iter()
        .enumerate()
        .flat_map(|(i, a)| {
            wires[1]
                .iter()
                .enumerate()
                .filter_map(move |(j, b)| (a.edge == b.edge).then_some([i, j]))
        })
        .collect::<Vec<_>>();
    let [common] = shared.as_slice() else {
        return None;
    };
    let edge = wires[0][common[0]].edge;
    if wires[0][common[0]].reversed == wires[1][common[1]].reversed {
        return None;
    }

    let mut projection = None;
    let mut orientation = None;
    for (s, wire) in surfaces.into_iter().zip(wires) {
        let [u0, u1] = [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]];
        let [v0, v1] = [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]];
        let corners = [[u0, v0], [u1, v0], [u1, v1], [u0, v1]];
        let mut seen = [false; 4];
        for coedge in wire {
            charge(cells, limit, 1)?;
            let p = &coedge.pcurve;
            if p.periodic
                || p.degree != 1
                || p.control_points.len() != 2
                || p.weights[0] != p.weights[1]
                || p.knots != [p.knots[0], p.knots[0], p.knots[3], p.knots[3]]
            {
                return None;
            }
            let side = (0..4).find(|&i| {
                p.control_points[0] == corners[i] && p.control_points[1] == corners[(i + 1) % 4]
            })?;
            if std::mem::replace(&mut seen[side], true) {
                return None;
            }
        }

        let remaining = limit - *cells;
        if remaining == 0 {
            return None;
        }
        let proof = certify_chart(s, remaining).ok()?;
        charge(cells, limit, proof.spans)?;
        let own_projection = match proof.reason {
            "global-rational-jordan-jacobian" | "global-rational-multispan-jordan-jacobian" => {
                proof.projection.map(Projection::Coordinate)
            }
            "global-exact-oblique-jordan-jacobian" => {
                proof.linear_projection.map(Projection::Linear)
            }
            _ => None,
        };
        let chart_projection = if let Some(shared) = projection {
            let shared: Projection = shared;
            if own_projection != Some(shared) {
                let (certified, used) =
                    certify_jordan_projection(s, shared.rows(), limit - *cells).ok()?;
                charge(cells, limit, used)?;
                if !certified {
                    return None;
                }
            }
            shared
        } else {
            own_projection?
        };
        projection = Some(chart_projection);
        charge(cells, limit, 1)?;
        // The Jordan route proves a nonzero Jacobian. Clamping and positive
        // weights give its sign at the corner from these three poles.
        let points = [
            &s.control_points[0][0],
            &s.control_points[1][0],
            &s.control_points[0][1],
        ];
        let [a, b, c] = points.map(|p| chart_projection.point(p));
        let [[ax, ay], [bx, by], [cx, cy]] = [a?, b?, c?];
        let sign = super::orient(&[&[ax, ay, 0.], &[bx, by, 0.], &[cx, cy, 0.]], Some([0, 1]))?;
        if sign == Sign::Zero || orientation.is_some_and(|p| p != sign) {
            return None;
        }
        orientation = Some(sign);
    }

    let projection = projection?;
    let mut exterior = Vec::with_capacity(6);
    for (wire, start) in wires.into_iter().zip(*common) {
        for offset in 1..4 {
            charge(cells, limit, 1)?;
            let c = &wire[(start + offset) % 4];
            let source = &model.edges[c.edge].curve;
            let d = source.degree;
            if source.periodic
                || !(1..=5).contains(&d)
                || source.control_points.len() != d + 1
                || source.knots[..=d].iter().any(|k| *k != source.knots[d])
                || source.knots[d + 1..]
                    .iter()
                    .any(|k| *k != source.knots[d + 1])
            {
                return None;
            }
            if matches!(projection, Projection::Linear(_)) {
                charge(cells, limit, source.control_points.len())?;
            }
            let mut points = source
                .control_points
                .iter()
                .map(|p| projection.point(p).map(|point| point.to_vec()))
                .collect::<Option<Vec<_>>>()?;
            let mut weights = source.weights.clone();
            if c.reversed {
                points.reverse();
                weights.reverse();
            }
            exterior.push(Curve {
                degree: d,
                knots: [vec![0.; d + 1], vec![1.; d + 1]].concat(),
                control_points: points,
                weights,
                periodic: false,
            });
        }
    }

    charge(cells, limit, 15)?;
    let remaining = (limit - *cells).min(64);
    if remaining == 0 {
        return None;
    }
    let boundary = nurbs_core::trim_simplicity::inspect(&exterior, 1e-8, 15, remaining).ok()?;
    charge(cells, limit, boundary.cells)?;
    if !boundary.proven_simple {
        return None;
    }
    // Off the boundaries, the projected winding numbers have the same sign.
    // Their sum is the winding of the simple exterior (the shared edge cancels),
    // so the two interiors cannot overlap. Nor can either interior contain the
    // other's boundary by openness and density. Simplicity of the exterior and
    // the two original loops leaves only the authored seam as boundary contact.
    let mut enclosure = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for pole in &model.edges[edge].curve.control_points {
        charge(cells, limit, 1)?;
        for (interval, &value) in enclosure.iter_mut().zip(pole) {
            interval[0] = interval[0].min(value);
            interval[1] = interval[1].max(value);
        }
    }
    Some(Certificate {
        faces,
        contact_enclosure: enclosure,
        edges: vec![edge],
        vertex: None,
        joined_proof: None,
        polar_join: None,
        source_join: None,
    })
}
