//! Two coplanar natural Jordan charts partition a Jordan region along one
//! original, exactly lifted edge. Only the joint exact-domain audit may call
//! this, after all coedge lifts and domains have been independently checked.
use crate::{Model, boundary_hull_contact::Certificate};
use cad_predicates::Sign;
use nurbs_core::{curve::Curve, surface_injectivity::certify_chart};

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

    // Limit the fallback to two carriers lying in the same coordinate plane.
    let omitted = (0..3).find(|&k| {
        let anchor = surfaces[0].control_points[0][0][k];
        surfaces
            .iter()
            .all(|s| s.control_points.iter().flatten().all(|p| p[k] == anchor))
    })?;
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
        if !proof.proven
            || !matches!(
                proof.reason,
                "global-rational-jordan-jacobian" | "global-rational-multispan-jordan-jacobian"
            )
        {
            return None;
        }
        let axes = proof.projection?;
        if axes.contains(&omitted) || projection.is_some_and(|p| p != axes) {
            return None;
        }
        projection = Some(axes);
        charge(cells, limit, 1)?;
        // The Jordan route proves a nonzero Jacobian. Clamping and positive
        // weights give its sign at the corner from these three poles.
        let sign = super::orient(
            &[
                &s.control_points[0][0],
                &s.control_points[1][0],
                &s.control_points[0][1],
            ],
            Some(axes),
        )?;
        if sign == Sign::Zero || orientation.is_some_and(|p| p != sign) {
            return None;
        }
        orientation = Some(sign);
    }

    let axes = projection?;
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
            let mut points = source
                .control_points
                .iter()
                .map(|p| axes.map(|k| p[k]).to_vec())
                .collect::<Vec<_>>();
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
