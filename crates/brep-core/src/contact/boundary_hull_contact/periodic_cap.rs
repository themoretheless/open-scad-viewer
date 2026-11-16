//! An affine cap can meet a periodic wall only on a natural rim when all
//! other wall rows are strictly on the same side of the cap plane.
use super::{Certificate, Model};
use cad_predicates::Sign;

pub(super) fn certify(model: &Model, faces: [usize; 2]) -> Option<Certificate> {
    for [wall, cap] in [faces, [faces[1], faces[0]]] {
        let wall = &model.faces[wall];
        let cap = &model.faces[cap];
        let surface = &wall.surface;
        let periodic = match (surface.periodic_u, surface.periodic_v) {
            (true, false) => 0,
            (false, true) => 1,
            _ => continue,
        };
        let Some(plane) =
            nurbs_core::surface_injectivity::certify_affine_parameterization(&cap.surface)
        else {
            continue;
        };
        let transverse = 1 - periodic;
        let knots = [&surface.knots_u, &surface.knots_v];
        let degree = [surface.degree_u, surface.degree_v];
        let count = [
            surface.control_points.len(),
            surface.control_points[0].len(),
        ];
        let domain = [0, 1].map(|k| [knots[k][degree[k]], knots[k][count[k]]]);
        let corners = [
            plane.control_points[0][0].as_slice(),
            plane.control_points[1][0].as_slice(),
            plane.control_points[0][1].as_slice(),
        ];
        for rim in &model.loops[wall.outer].coedges {
            let pcurve = &rim.pcurve;
            if !super::full_use(model, rim)
                || model.edges[rim.edge].degenerate
                || pcurve.degree != 1
                || pcurve.periodic
                || pcurve.control_points.len() != 2
                || !std::iter::once(cap.outer)
                    .chain(cap.holes.iter().copied())
                    .flat_map(|wire| &model.loops[wire].coedges)
                    .any(|other| other.edge == rim.edge && super::full_use(model, other))
            {
                continue;
            }
            let [a, b] = [0, 1].map(|i| &pcurve.control_points[i]);
            if [a[periodic], b[periodic]] != domain[periodic]
                && [b[periodic], a[periodic]] != domain[periodic]
            {
                continue;
            }
            let Some(end) = domain[transverse]
                .iter()
                .position(|&value| a[transverse] == value && b[transverse] == value)
            else {
                continue;
            };
            let (boundary_knots, first_interior_knot) = if end == 0 {
                (
                    &knots[transverse][..=degree[transverse]],
                    knots[transverse][degree[transverse] + 1],
                )
            } else {
                (
                    &knots[transverse][count[transverse]..],
                    knots[transverse][count[transverse] - 1],
                )
            };
            if first_interior_knot == a[transverse]
                || boundary_knots.iter().any(|&knot| knot != a[transverse])
            {
                continue;
            }
            let row = if end == 0 { 0 } else { count[transverse] - 1 };
            let mut side = None;
            let mut contact_enclosure = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
            let separated = surface.control_points.iter().enumerate().all(|(u, poles)| {
                poles.iter().enumerate().all(|(v, pole)| {
                    let sign = crate::shared_boundary::orient(
                        &[corners[0], corners[1], corners[2], pole],
                        None,
                    );
                    if [u, v][transverse] == row {
                        for (bounds, &coordinate) in contact_enclosure.iter_mut().zip(pole) {
                            bounds[0] = bounds[0].min(coordinate);
                            bounds[1] = bounds[1].max(coordinate);
                        }
                        sign == Some(Sign::Zero)
                    } else if matches!(sign, Some(Sign::Positive | Sign::Negative)) {
                        sign == *side.get_or_insert(sign)
                    } else {
                        false
                    }
                })
            });
            if separated && side.is_some() {
                return Some(Certificate {
                    faces,
                    contact_enclosure,
                    edges: vec![rim.edge],
                    vertex: None,
                    joined_proof: None,
                    polar_join: None,
                    source_join: None,
                });
            }
        }
    }
    None
}

/// A trim entirely inside the transverse interval gives positive weight to
/// both rows. A cap plane on only one side of their hull cannot meet it.
pub(crate) fn disjoint(model: &Model, faces: [usize; 2]) -> bool {
    for [wall, cap] in [faces, [faces[1], faces[0]]] {
        let wall = &model.faces[wall];
        let surface = &wall.surface;
        let transverse = match (surface.periodic_u, surface.periodic_v) {
            (true, false) => 1,
            (false, true) => 0,
            _ => continue,
        };
        let knots = [&surface.knots_u, &surface.knots_v][transverse];
        if knots.len() != 4
            || [surface.degree_u, surface.degree_v][transverse] != 1
            || knots[0] != knots[1]
            || knots[2] != knots[3]
            || surface
                .control_points
                .len()
                .saturating_mul(surface.control_points[0].len())
                > 256
        {
            continue;
        }
        let wire = &model.loops[wall.outer].coedges;
        if wire.is_empty()
            || wire.len() > 256
            || !wire.iter().all(|c| {
                super::full_use(model, c)
                    && c.pcurve
                        .control_points
                        .iter()
                        .all(|p| knots[1] < p[transverse] && p[transverse] < knots[2])
            })
        {
            continue;
        }
        let Some(plane) = nurbs_core::surface_injectivity::certify_affine_parameterization(
            &model.faces[cap].surface,
        ) else {
            continue;
        };
        let anchors = [
            plane.control_points[0][0].as_slice(),
            plane.control_points[1][0].as_slice(),
            plane.control_points[0][1].as_slice(),
        ];
        let mut side = None;
        let mut strict = [true; 2];
        let separated = surface.control_points.iter().enumerate().all(|(u, poles)| {
            poles.iter().enumerate().all(|(v, pole)| {
                match crate::shared_boundary::orient(
                    &[anchors[0], anchors[1], anchors[2], pole],
                    None,
                ) {
                    Some(Sign::Zero) => {
                        strict[[u, v][transverse]] = false;
                        true
                    }
                    sign @ Some(Sign::Positive | Sign::Negative) => {
                        sign == *side.get_or_insert(sign)
                    }
                    _ => false,
                }
            })
        });
        if separated && strict.into_iter().any(|all_strict| all_strict) {
            return true;
        }
    }
    false
}
